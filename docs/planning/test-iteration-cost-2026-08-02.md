# Test iteration cost — the suite is the slowest thing an agent does

**Armed:** 2026-08-02, on Jon's instruction, as a MAJOR item for the next long
run. Surveyed against `scripts/run_tests.py`, `Cargo.toml`, and a measured run
of `./run_tests.sh` on the dev VM.

**The objective, in Jon's words:**

> Testing iteration is wasting too much agent time, it is unacceptable.

---

## The thesis

The suite's wall time is dominated by **compiling the same dependencies over and
over**, not by running tests. The runner asks cargo for 33 different feature
resolutions, one after another, on a machine where dependencies are built at
`opt-level = 3` with incremental off. Each distinct resolution is a distinct
build graph, so `bevy`, `wgpu`, `parry2d` and friends are compiled again for
every group that does not share a graph — and then the tests themselves, the
part that answers the question, run single-file on one core.

Nothing here is a bug in the runner. Every piece of it was a reasonable local
decision (§4 records why each exists). The cost is the *product* of those
decisions, and the product is what an agent pays every time it wants to know
whether it broke something.

---

## What is true today, with evidence

### 1. The jobs run strictly one at a time

`scripts/run_tests.py:569-573`:

```python
for j in jobs:
    ...
    rc = subprocess.run(j.argv, cwd=j.cwd or REPO, env=env).returncode
```

A plain sequential loop over 33 jobs. The dev VM has **8 cores** (`nproc`).
During the run phase — the part that is actually testing — the suite uses about
one of them, because `cargo test` also runs its test binaries one after the
other by default.

### 2. Every job is its own build graph, on purpose

`./run_tests.sh --list` prints 33 jobs, of which 28 are
`cargo test -p <crate> --features <that crate's safe set>`. The runner's own
docstring says why, and the reason is sound:

> cargo unifies features per build graph, and there is no safe workspace-wide
> `--all-features` here (that would pull in android/web/wasm targets). So to
> actually COMPILE AND RUN a crate's `#[cfg(feature = "...")]` tests, we enable
> that crate's headless-safe features in its own `cargo test -p` invocation.

The consequence is not in the docstring. Distinct feature sets mean distinct
fingerprints, so a dependency shared by two jobs with different resolutions is
compiled twice.

**Why it is not already one compile, precisely.** Cargo resolves features per
invocation, over only the packages that invocation selects. `cargo test -p
ambition_render --features input,portal_render` computes one union across that
subgraph; `cargo test -p ambition_input --features input` computes a different
one. Any shared dependency whose resolved features differ is a different
fingerprint, so it is a different artifact — and everything downstream of it
rebuilds too. Twenty-eight per-crate jobs therefore admit up to twenty-eight
variants of the mid-layer crates, which is exactly what §3 measures.

**The one-compile job already exists, and it is affordable.** `workspace (default
features)` IS `cargo test --workspace` — one graph, everything in it — and it
costs **607s of the 3776s**. The remaining ~53 minutes buys one thing: compiling
and running the tests behind `#[cfg(feature = "...")]`, which the default graph
never turns on. That is the entire trade, and front 1 is how to keep the second
half without paying for it twenty-eight times.

And the count grows on its own. `queue-72h-2026-07-31.md` records the suite
green at **24/24 jobs** on 2026-07-31; `--list` prints **33** on 2026-08-02.
Nine jobs in two days, because the job plan is computed from the manifests and a
new crate or a new feature adds one. That is the right design for *coverage* —
it cannot drift — and it means the cost curve is attached to the thing the
project does constantly, which is add crates.

### 3. Measured: 7% of the suite executes tests, and the monolith is built 16 times

One complete run, 2026-08-02 20:43–21:46 on the dev VM. Numbers are from the
runner's own timing report and from libtest's and cargo's own output — nothing
here is a wrapper's estimate.

| | |
|---|---|
| wall clock | **3776s — 63 minutes** for 33 jobs |
| libtest execution, all 72 test binaries | **239.3s** |
| pytest execution | **26.1s** |
| **fraction of the run that executed tests** | **≈7%** |
| `Compiling` events | **1858**, over 436 distinct crates |
| `Checking` events | 403, over 295 distinct crates |
| crates built more than once | **400 of 454** |
| `ambition_platformer2d_actor_monolith` built | **16 times** |
| `ambition_sprite_sheet` built | **18 times** |
| `bevy_render` built | 6 times |
| disk consumed by the run | **+77 GB** (267 GB free → 190 GB) |

Sixty-three minutes to run four and a half minutes of tests. The largest crate
in the workspace was compiled sixteen times, and 88% of the dependency graph was
compiled more than once.

The slowest single job, `workspace (default features)`, took **607s**; the
median job is under a minute. The distribution matters for Front 2: the suite is
not one long pole, it is a queue of medium poles being drained one at a time.

⚠ **Measurement conditions, stated so the numbers are not over-read.** This ran
on a shared 8-core VM while a second agent session was working in the same
checkout, and the cache was warm but not complete. That inflates the absolute
wall time. It does NOT explain the repeat compiles, which are structural — they
follow from the feature resolutions, not from the cache state. Front 0 exists so
the next measurement is not another log read by hand.

### 3b. At 63 minutes, the suite outlives the tree it is testing

This run reported **4 failed jobs**, and not one of them can be attributed.

Two of the failures were `include_str!` compile errors on generated sprite
manifests — `assets/sprites_0_5x/*.ron`, which is untracked, generated content.
Those files exist now and existed before the run. They were rewritten at
**21:06–21:08**, twenty-three minutes into a run that started at 20:43, because
the other session regenerated sprites while the suite was compiling against
them. The suite was reading a tree that changed underneath it.

That is not a freak event, it is a function of duration: a 63-minute job on a
shared checkout will overlap with whatever else is happening, and AGENTS.md's
own rule ("don't run the suite while editing — every job reads the LIVE tree")
is unenforceable when the suite runs for an hour and the repo has more than one
agent in it.

So the cost of the suite is not only the hour. It is that **after the hour, the
result may still not answer the question**, and the honest response is to run it
again — which is the loop Jon is calling unacceptable.

### 4. Dependencies are compiled the expensive way, for a good reason

`Cargo.toml:121-130` builds every non-workspace dependency at `opt-level = 3`
with `debug = false`. That is right for a game — a Bevy app at `opt-level = 0`
is not playable, and the tests here run real app compositions. It is also the
most expensive possible thing to redo, and §2 redoes it.

`.cargo/config.toml:27` sets `incremental = false`, which is not negotiable:
mold plus incremental produced undefined-symbol link failures three times on
2026-07-31, and incremental caches filled the disk (queue rows S14, S33).

Those two are load-bearing. The lever is not to weaken them — it is to stop
paying them repeatedly.

### 5. An agent waiting on the suite cannot see what it is waiting for

The live status file (`scripts/run_tests.py:562-566, 579`) carries
`{pid, started, jobs, free_gb_at_start, state, finished_jobs}` — a count, and
nothing about WHICH job is running or how long the ones before it took. Per-job
timings do exist (`timing_report`, `:220-231`) but only print at the END, and
the machine-readable form is opt-in (`if timings_json:`, `:598`).

So the only way to learn that job 14 of 33 has been stuck for eleven minutes is
to read the raw log, which is the thing the status file was built to avoid. An
agent that wants a progress estimate has no source for one.

### 6. The narrow escape hatches are the ones AGENTS.md warns against

`AGENTS.md:93` is explicit: **`cargo check -p <one_crate>` is not the gate —
`cargo check -p ambition_app` is**, because a per-crate check has been observed
green on a crate that fails in the app build. `./run_tests.sh -p <crate>` and
`--fast` exist, but there is no written statement of *which* cheap command is
sufficient for *which* kind of change. So an agent either runs the whole suite
(≈an hour) or runs something it has been told is not the gate. That gap is why
the expensive option keeps getting chosen.

---

## ⭐ RESULT (2026-08-03): 63 minutes → 25.5, and the disk cost is gone

Fronts 0 and 1 landed and the suite measured itself. Every number below comes
from `dev/run_tests_cost.jsonl`, which Front 0 exists so that nobody has to read
another log by hand:

| | before (08-02, hand-read) | after (08-03, from the ledger) |
|---|---|---|
| wall clock | 3776s / 63 min | **1528s / 25.5 min** |
| executing tests | ~7% | **17%** (266s) |
| disk consumed | +77 GB | **+3 GB** |

What did it: the 23 per-crate feature jobs were doing two things at once, and only
one of them needed its own build graph. They are now 23 `cargo check` proofs
(468s total, 20.3s mean) plus ONE `cargo test --workspace --no-fail-fast` over
the union of every headless-safe feature (339.5s wall, 126.8s executing) — about
what the plain default-feature job costs, while running strictly more.

⭐ **the disk result matters more than the minutes.** The volume filled three
times across two runs (S14, S33) because every feature job left a full variant of
the graph behind; a check lane produces no codegen to leave.

⚠ **three findings the union surfaced that no per-crate job could**, and they are
the argument for the shape rather than a side effect: three `causal` message
channels sitting outside both rollback oracles because the default job never
compiled them; a rollback schema dump that is feature-dependent; and (from the
same run) an `Empowered` component registered by two demos, green in each and a
panic in the app that composes both.

⚠ **2 of 33 jobs report FAIL** — both workspace jobs, on three pre-existing
`app_it` failures that predate this work. A faithful red, not a false green.

---

## The fronts

Ordered by dependency. Front 0 is not optional — every other front's claim is
judged by it, and this campaign should not repeat the mistake of quoting
percentages from an instrument nobody checked.

### Front 0 — make the suite report its own cost, always

- Write the per-job timings on EVERY run, not on request: default
  `--timings-json` to a path under `dev/` so runs accumulate a record.
- Put the live facts in the status file: current job name, its start time, and
  the completed jobs with their seconds. A waiter should be able to answer "how
  far along, and is it stuck" from that file alone.
- Record, per job, the split between compiling and running. Cargo can be asked
  directly (`--timings` emits an HTML/JSON build profile); libtest already
  reports its own execution time per binary. Both halves exist; nothing collects
  them.

**Done when:** `./run_tests.sh` leaves behind a machine-readable record of where
its wall clock went, and a second run can be compared to the first without
anyone re-reading a log by hand.

### Front 1 — compile the graph once, then run everything in it

This is the whole campaign in one sentence, and §2 is the obstacle.

The per-crate feature jobs are doing **two different things at once**: proving
each feature combination COMPILES, and RUNNING the tests those features gate.
Only the first needs a separate build graph.

- Split them. Keep a per-feature-set `cargo check` for the compile-combination
  guarantee — metadata-only, no codegen, no linking, and the part §4 makes
  expensive is largely skipped.
- Run the tests in as few graphs as possible: one workspace-wide invocation with
  the union of the headless-safe feature sets (`cargo test --workspace -F
  pkg/feat ...`), falling back to a small number of groups if the union is not
  safe.
- ⚠ **Probe the union before believing in it.** Feature unification changes what
  each crate is compiled with, so a test can pass in the union and fail in the
  crate's own resolution. The check-per-combination above is what makes that
  acceptable, but the campaign must SHOW a case where union and per-crate agree
  and a case where they would have differed, rather than asserting it.

**Done when:** one suite run compiles each dependency at most twice (once for
the check lane, once for the test lane), measured by Front 0's record.

### Front 2 — use the cores

- Run the independent jobs concurrently, bounded by cores and by disk. Note the
  constraint before writing the loop: cargo takes a **build lock per target
  directory**, so concurrent cargo processes sharing one target dir serialize
  their build phases. Parallelism therefore has to come either after Front 1
  (one build, many test processes) or from separate target dirs, which multiply
  a 31 GB directory and are how the disk filled three times already.
  Front 1 first is the cheap order.
- Inside a job, run test binaries in parallel. `cargo-nextest` does exactly this
  and is not installed here; adopting it is a decision, not a detail, so it goes
  to Jon with a measured before/after rather than being slipped in.

**Done when:** the run phase saturates more than one core, and the suite's wall
time is within a small factor of its longest single job.

### ⭐ Front 3 — `./run_tests.sh` is too ALLURING, and that is the real defect

**Jon, 2026-08-02, and this is the headline:**

> the bigger problem is that run_tests looks so alluring to an agent, and it
> prevents it from running the focused test that actually matters — instead it
> just runs all the junk that comes with regular run_tests.

This is a behavioural defect, not a performance one, and it explains why the
other fronts are worth less than they look. `./run_tests.sh` is the one
documented front door. It is the thing AGENTS.md points at, it is the thing that
returns a single trustworthy green, and so an agent that wants to be careful
reaches for it — and then spends an hour not running the three tests that would
have answered its question.

Making the suite faster does not fix that. A 15-minute front door is still the
front door, and the focused test still does not get run.

**Two changes, and the second is Jon's:**

1. **The default must BE the focused set.** `./run_tests.sh` with no arguments
   should run what matters — the jobs that catch real regressions — and the long
   tail should be behind a flag, the way `--heavy` already works for `#[ignore]`.
   The bar for a job being in the default set is that it has CAUGHT something, or
   that it guards a documented recurring failure. Everything else is opt-in.
2. **Make the focused route obvious and blessed**, so choosing it is not felt as
   cutting a corner. Today the narrow options (`-p`, `-k`, `--fast`) are real but
   undocumented as a *policy*, and AGENTS.md's warning that `cargo check -p
   <crate>` is not the gate reads — correctly — as "do not trust narrow things".
   An agent facing "narrow is not trusted" and "wide costs an hour" picks the
   hour. That is the trap, and it is written into the docs today.

⚠ **The counter-argument, stated because it is the thing that will be raised:**
the job plan is computed from the manifests precisely so coverage cannot drift,
and a curated default set is exactly the drift that design prevented. The answer
is that the curated set does not replace the full plan; it changes who pays for
it and when.

**⭐ Who runs the full plan — Jon, 2026-08-02, and this is a design input, not a
detail:**

> there isn't any CI, but I do periodically run the entire suite. It's usually
> enough to let things drift for a day or so, and then it's not so bad to go back
> and fix all the bugs.

So the full sweep already has an owner and a cadence, and they are not the
agent's inner loop. **A day of drift is an accepted cost**, stated by the person
who pays it. That settles the counter-argument above rather than balancing it:
coverage is not lost by curating the default, because the exhaustive run still
happens on Jon's schedule — and an agent running it every edit is not adding
safety, it is duplicating a sweep that is already scheduled, an hour at a time.

⚠ Do not design the default set as though a missed regression is unrecoverable.
It is recoverable, in a batch, by the person who said so. Design it for the
question an agent actually has in the next thirty seconds.

What changes is which one an agent pays
for by default, and the honest framing is that the current default optimises for
never missing anything at the cost of the thing being missed most: the test that
should have been run in the first thirty seconds.

### Front 3b — write down the cheapest sufficient command

A table, in `AGENTS.md`, mapping the kind of change to the command that settles
it: a pure-Rust change inside one crate, a change that crosses a crate seam, a
change to authored content, a change to generated assets, a change to the
runner itself. Each row names a command AND what it does not cover.

This is the front that actually returns agent time, because it is the one that
stops the full suite from being run out of caution.

**Done when:** an agent can pick its verification from the table without
guessing, and the full suite is what Jon's periodic sweep runs rather than what
every edit runs.

### Front 3c — compile frequently-changing crates without optimizations

**Jon, 2026-08-02:** *"Does it make sense to compile tests for frequently
changing modules without any optimizations? That might also help the build
time."* Yes, and the mechanism is already in the tree — this is an extension of
an existing decision, not a new lever.

`Cargo.toml:139-143` already pins two workspace crates to `opt-level = 0`:

```toml
[profile.dev.package.ambition_platformer2d_runtime]
opt-level = 0
[profile.dev.package.ambition_render]
opt-level = 0
```

with the stated rule — "codegen-heavy workspace crates that are NOT hot loops
under test". Everything else in the workspace gets `opt-level = 1` and the
dependencies get 3.

So the front is: **extend that list by measurement, crate by crate.** The
candidates are the crates an agent edits most and §3 rebuilds most —
`ambition_platformer2d_actor_monolith` first.

⚠ **The counter-force, which is why this is measured and not just applied.**
These tests boot real app compositions: the rollback oracle alone runs 2980
sim advances, and `app_it` is 190s of the run. Dropping the monolith to
`opt-level = 0` moves cost from compile into that runtime. At today's split —
93% compiling, 7% running — the trade is heavily favourable and might still be
favourable at 50/50, but it inverts at some point and the existing comment's
"NOT hot loops under test" is exactly the right test to apply.

Measure per crate: compile seconds saved against test seconds added, on the same
run. Front 0's record is what makes that a subtraction rather than an argument.

### Front 4 — the scheduled crate carves are also a compile-time program

**Jon, 2026-08-02:** *"working on scheduled crate carves could help improve
compile iteration time."* They can, and the reason is sharper here than in a
normal Rust project.

**A crate is the rebuild unit, and this repo has incremental OFF.** With
`incremental = false` (`.cargo/config.toml:27`, forced by the mold link failures
and the disk), editing one line rebuilds the *entire crate* — there is no
sub-crate reuse to fall back on. So crate size is not merely correlated with
iteration cost, it IS the granularity of it. `ambition_platformer2d_actor_monolith`
is the largest thing an agent routinely edits, and every edit to it pays for all
of it. Eight cores can only help if there is more than one unit to build.

And §3 measured the multiplier: the monolith was compiled **16 times in one
suite run**, `ambition_sprite_sheet` 18. Whatever a carve saves on one build of
those crates, the suite currently pays for sixteen times over. That cuts both
ways and it is worth stating plainly: it means a carve's value is larger than a
single-build measurement suggests, AND it means fronts 0–1 could remove most of
that multiplier without moving a single line of code between crates. Do not let
either front be used as an argument against the other.

**This evidence now contributes directly to the active actor-monolith carve.**
The later API consumer-footprint measurement independently demonstrated that the
monolith leaks unrelated capabilities into a movement-only consumer. Together,
those results retire the July "no further carve owed" ruling. Compile cost does
not dictate boundaries: a carve that shortens builds by recreating divergent
player/enemy/boss paths is still a bad trade. It does mean the decomposition is
required, and compile isolation is one axis for ordering otherwise-sound slices.
See
[`engine/actor-monolith-decomposition.md`](engine/actor-monolith-decomposition.md).

What this front asks for is about ORDER and EVIDENCE:

- **Add compile cost to what a carve candidate is measured by.** The census
  (`scripts/core_import_census.py --cuts`) already ranks candidate cuts by
  import structure. It does not report what a cut would do to rebuild time.
  Cargo can answer that (`cargo build --timings` gives per-unit durations and
  the critical path); pairing the two turns "which cut is worth it" into a
  question with two axes instead of one.
- **When two carves are equally justified architecturally, do the one that cuts
  the rebuild fan-out first.** That is not always the biggest crate — a large
  leaf that little depends on costs less per edit than a medium crate that half
  the workspace sits downstream of.
- **Expect the win to be on the EDIT loop, not on the suite.** A carve does not
  reduce the repeated dependency builds of §2 at all; those are feature
  resolutions, and fronts 0–1 own them. What a carve improves is the inner loop
  an agent runs dozens of times between suite runs, which by volume may be the
  larger share of the wasted time. Both are worth having; they are separate
  mechanisms and should be measured separately rather than credited to each
  other.

**Done when:** the carve program's candidate ranking carries a measured rebuild
cost alongside its architectural argument, and a landed carve can point at a
before/after on the edit loop.

---

## Non-goals

- Weakening `opt-level = 3` on dependencies or turning incremental back on.
  §4 explains why both are load-bearing; this campaign makes the expensive build
  happen fewer times, it does not make it cheaper per time.
- Deleting tests to make the number go down. The suite's coverage is not the
  problem being solved here.
- A remote/distributed build. Out of scope for a solo dev VM.

---

## How we will know it worked

Today's baseline, all from §3 and to be re-measured the same way:

| measure | today | target |
|---|---|---|
| wall clock, warm cache | 3776s | a small fraction of it |
| fraction of the run executing tests | ≈7% | the majority |
| crates compiled more than once | 400 of 454 | near zero outside the check lane |
| `ambition_platformer2d_actor_monolith` builds per run | 16 | ≤2 |
| disk consumed per run | 77 GB | bounded, and stated by the runner |

1. The run is short enough that the tree does not change underneath it (§3b) —
   the point at which a failure means something again.
2. An agent waiting on a run can say which job is running and how much is left,
   from the status file alone.
3. `AGENTS.md` names the cheap sufficient command for each kind of change, and
   the full suite stops being the reflex for an ordinary edit.
