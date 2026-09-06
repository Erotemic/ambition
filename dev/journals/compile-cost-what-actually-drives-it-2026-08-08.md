# What actually drives compile cost — 2026-08-08

Companion to `compile-time-and-disk-2026-08-07.md`, which established that the
cost is codegen rather than link or frontend, and that a cold build and a rebuild
want opposite fixes. This one asks the next question: **given that, which crates
are expensive and why?**

Short answer, held loosely: **lines do not predict cost, the backend does, and a
large part of the backend's cost looks per-CRATE rather than per-line.** That
last clause is the one with consequences — if it holds, splitting a crate
multiplies a fixed toll.

Machine: 8 cores, mold. Numbers do not transfer; the method does.

---

## 0. Qualitative assessment — how much to believe any of this

⛔ **Read this before acting on anything below.** The quantitative results are
checked in and reproducible. What follows is my *judgement*, which has a bad day
behind it: **I was wrong six times on this exact topic today**, each time
confidently, and each correction was found by a check rather than by foresight.
That is a reason to discount my newest position too, not to trust it because it
is newest.

| claim | confidence | why |
|---|---|---|
| Lines do not predict cost per line | **high** | 52 crates, negative in every config separately, three independent framings agree |
| The cost is the backend, not the frontend | **high** | 4.4% frontend measured directly; consistent with the 2026-08-07 codegen finding at much larger n |
| ThinLTO specifically is the largest single pass | **medium-high** | measured twice on one crate, including against its own confound (50.7% / 58.7%) |
| Backend cost is largely **per-crate** | **medium-low** | ⚠ **n = 3, and two of the three are warm-cache recompiles.** The monotone trend is suggestive, not established |
| Therefore **carving multiplies the toll** | **low-medium** | this is an inference from the row above, and it is the claim with the biggest consequence. It is the one I would bet against myself on |
| Incremental makes *release* 2.2x faster | **low** | one sample, two target dirs, plausible mechanism, nothing else |

⭐ **The asymmetry that matters**: the claims I am confident about are the
*negative* ones — what does **not** explain the cost (lines, own generics, system
density, dependency size). Those were each killed by a specific check. The
positive story is one crate deep and three crates wide.

⚠ **What would change my mind on the carving corollary**, which is the only claim
here that should move a decision:

* `-Ztime-passes` on 5–10 more crates spanning sizes, all **cold, incremental
  off, uncontended**, so the trend is not three points with two contaminated;
* an actual before/after — carve one module out and measure the full build both
  ways. **Nothing here measures a carve. It measures crates that already exist**,
  and infers what a carve would do. That inference is the weakest link.
* the parallelism half, which I have not measured at all: more crates means more
  units in flight, and a real build is core-bound during its cold phase. That
  effect points the *other* way and could plausibly dominate.

⭐ **So the honest position on lane C is not "do not carve".** It is: *the
compile-time argument FOR carving is not supported, and there is some evidence
against it, so carve on architectural grounds and stop citing build time in
either direction until someone measures an actual carve.* That is a weaker claim
than §4 reads, and §4 should be read through it.

---

## 1. Lines do not predict cost

Across all 52 first-party crates ≥300 lines, using measured release seconds from
`dev/ambition_dev_measurements/compile_units.jsonl`:

| predictor | corr with seconds |
|---|---|
| own lines | +0.576 |
| transitive dependency lines | +0.497 |
| number of dependencies | +0.514 |

and for cost **per line** against size, `corr(ms/line, lines) = −0.23` — slightly
*negative*. It stays negative inside every build configuration separately:
−0.11 (release/opt-3), −0.27 (test/opt-1), −0.78 (test/opt-0).

**Bigger crates are cheaper per line.** That matters because line count is the
unit every carve discussion in `docs/planning` has used, and it is the unit three
of `compile_ratchet.py`'s four guarded numbers are denominated in.

## 2. Three explanations, tested and killed

Each was plausible, and each died to a cheap check before it could become a
finding.

1. **The crate's own generics.** `ambition_relativity2d` is the density outlier
   (2,840 lines, the highest ms/line in the workspace). It contains **no
   `impl<`, no `macro_rules!`, and 21 generic fns.** Dead.
2. **Bevy system density.** `corr(ms/line, add_systems+add_observer per kloc) =
   +0.42` — real but moderate, with two counterexamples that kill the simple
   story: `ambition_app` has 4.6 systems/kloc and is the third *cheapest* per
   line; `ambition_demo_twintrack` has 1.8 and is third most expensive.
3. **Instantiation in the consumer** — the idea that small crates pay codegen for
   generics defined in their big dependencies. Tested by inverting
   `direct_dependents` in the ratchet baseline to get transitive dependencies:
   dependency size (+0.497) does **not** beat own size (+0.576). The case that
   kills it: `ambition_content`, `ambition_demo_mary_o` and `ambition_demo_sanic`
   have *identical* transitive dependency lines (364,978 — the demos depend on
   everything) and cost 124.0 / 93.8 / 58.5 s. Dependency size cannot order them;
   their own lines order them exactly.

⚠ the third test's weakness, stated: `deplines` saturates, so it has little
discriminating power and a null from it is weak evidence.

At that point **no structural variable available in the repo explained the
outlier**, which is what justified spending a profile on it.

## 3. `-Z time-passes` — it is the backend, and mostly ThinLTO

```
cargo +nightly rustc -p ambition_relativity2d --release -- -Ztime-passes
```

| pass | s | share |
|---|---|---|
| `LLVM_thinlto` | 6.47 | **50.7%** |
| `codegen_crate` | 4.28 | 33.5% |
| `LLVM_passes` | 4.20 | 32.9% |
| `monomorphization_collector_graph_walk` | 1.41 | 11.0% |
| frontend (typeck + borrowck + coherence + expand) | **0.56** | **4.4%** |

(passes overlap — `codegen_crate` spawns the LLVM work — so they do not sum.)
**Type-checking the whole crate is 0.196 s.** Over half the compile is one LLVM
pass.

### The confound, checked

`.cargo/config.toml` sets `incremental = true` workspace-wide, and incremental
uses many small codegen units — so the ThinLTO share might have been manufactured
by the repo's own setting rather than by the release profile. Re-run identically
with `CARGO_INCREMENTAL=0` in a fresh target dir:

| config | total | ThinLTO | share |
|---|---|---|---|
| `incremental = true` | 12.77 s | 6.47 s | 50.7% |
| `CARGO_INCREMENTAL=0` | 28.40 s | 16.68 s | **58.7%** |

**ThinLTO dominates either way.** The finding survives.

⛔ **and the prediction had the wrong sign.** The reasoning was "incremental
manufactures ThinLTO work, so removing it shrinks the share". It grew. A
plausible mechanism is not a direction — and this only avoided becoming a
published finding because the discriminating run was queued *before* the
explanation was written down. **Queue the experiment before you write the
story.**

⚠ banked, one sample, not acted on: incremental OFF made this *release* build
2.2x slower. Probable mechanism is CGU count — many small units parallelise
better on 8 cores even though each optimises less.

## 4. The shape across three sizes — and the consequence

| crate | lines | total | ms/line | ThinLTO | share | ThinLTO ms/line |
|---|---|---|---|---|---|---|
| `relativity2d` | 2,840 | 12.77 | 4.50 | 6.47 | **50.7%** | **2.28** |
| `platformer2d_runtime` | 14,747 | 65.99 | 4.48 | 24.47 | 37.1% | 1.66 |
| `actor_monolith` | 111,790 | 76.07 | **0.68** | 15.46 | **20.3%** | **0.14** |

⚠ the latter two are warm-cache recompiles (built once as dependencies, then
rebuilt with `-Ztime-passes`). Their **backend** figures are the usable half;
their frontend ones are not — see §6.

* ThinLTO's share falls monotonically with crate size, **50.7% → 37.1% →
  20.3%**, and its per-line cost falls **16x**. The monolith has **39x** the
  lines of `relativity2d` and **2.4x** the ThinLTO.
* The monolith at 111,790 lines costs 76.07 s against the runtime's 65.99 s at
  14,747 — **7.6x the code for 15% more time**, machine to itself.

⭐ **the reading this suggests** — and it is a reading of three points, not a
result — is that a substantial part of backend cost is **per-CRATE rather than
per-line**: every crate pays a fixed toll and a big one amortizes it. That would
explain §1's negative correlation directly, which is the main thing recommending
it.

⚠ **but three crates is not a trend, and two of them are contaminated.** A
monotone sequence of length three happens by chance often. Before this is used
for anything, it wants 5–10 crates spanning sizes, cold, incremental off,
uncontended. See §0.

**The consequence people will want to draw** — that splitting one crate into N
multiplies the toll by N — follows from the reading, not from the measurement.
⛔ **nothing here measures a carve.** It measures crates that already exist and
infers what carving would do, and that inference is the weakest link in the
document.

⚠ **Two things point the other way** and neither has been measured: more crates
means more units in flight, and a cold build is core-bound — so parallelism could
plausibly dominate the per-crate toll. And pipelined compilation releases a
dependent at its predecessor's `rmeta`, so a carve can shorten the serial path
even when it adds total work.

⭐ **So the defensible position is narrower than "carving costs compile time":**
the compile-time argument *for* carving is not supported, there is some evidence
against it, and neither side should be cited until someone measures an actual
carve both ways. Carve for the serial chain, dependency direction and
architecture — the grounds that were always the real ones.

## 5. Two numbers, two questions

`ambition_relativity2d` costs **68.1 s** in `dev/ambition_dev_measurements/compile_units.jsonl` and
**12.77 s** under `-Ztime-passes`. Both are correct and they answer different
questions:

* a unit's duration in `cargo --timings` is wall time **inside a real build**,
  sharing 8 cores with sibling rustc processes that are each internally parallel
  → the input for **prioritising** ("what does this cost the build I run");
* `-Ztime-passes` on `cargo rustc -p X` gives the crate **the whole machine** →
  the input for **diagnosing** ("what does it intrinsically cost, and where").

⛔ Do not compare one to the other. Same error family as treating `cargo check`
as a build's frontend phase. The ratio has been consistent (5.3x, 4.1x), about
what 8-core sharing predicts.

## 6. Method errors made here, and what caught each

Kept because the failure modes are more reusable than the numbers.

| error | what caught it |
|---|---|
| **Pooled debug and release** in a per-crate average, then compared two crates that did not share an opt level | checking CV before reporting the mean — 51–73% is not noise, it is a mixed population |
| **Pooled cache states** within a profile after fixing the first one | someone else deriving cache state from `build_fresh_units` instead of the label |
| **Quoted a superlative** ("cheapest crate per line") that held in 1 of 8 builds — the one with 669 of 688 units cached, ranking 17 crates not 55 | reading the per-build view instead of the aggregate |
| **Called the ledger "inflated"** against the profile | asking what each instrument actually measures |
| **Read `type_check_crate` = 0.019 s** for a 14,747-line crate | plausibility — less than a 2,840-line crate is impossible; it was incremental reuse |
| **Predicted the confound's direction** and got the sign backwards | having queued the run before writing the hypothesis |

⭐ The one that generalises: **a build measurement's atomic unit is a BUILD,
identified by its own counters. A configuration label can lie about what a build
did** — one row here is labelled `dev/first-party` with `build_fresh_units: 0`,
having recompiled all 688 units in 540 s against two honest rebuilds at 188 s and
210 s. Grouping by that label corrupts every average it touches.

## 7. What is not answered

* **The `incremental` axis of the telemetry.** All 2,145 collector rows read
  `false`; the column exists and has only ever been set one way. One collector
  run with `CARGO_INCREMENTAL=1` closes it.
* **Whether the 2.2x release/incremental result reproduces** on a second crate.
* **Third-party cost** — 5,133 s of the dev cold build, `bevy_pbr` alone 334 s,
  `avian2d` 210 s. Visible in the data, never surfaced.
* **Whether tuning `codegen-units` / `lto` for release is worth it.** There is
  **no `[profile.release]` section** in `Cargo.toml` at all — the `lto = "thin"`
  and `codegen-units = 1` near it belong to `[profile.android-size]`. Dev carries
  hand-picked `opt-level = 0` overrides for the three worst crates; release has
  had no attention. Given §3, that is where the lever is.

---

# Addendum — the REAL loop, measured (2026-08-08, later)

Everything above measures builds staged for measurement. Jon's response was that
the useful measurement is *"the effect of real workflows on compile and test
time … inform how often we should be running some of these commands versus
adding code in batches, or working in the background while compiles and tests
run."*

Nothing was collecting that. The goal guard runs `cargo check -p ambition_app`
and the whole `app_it` suite every time a turn ends — `.goal/state.json` recorded
**114 such runs between 01:48 and 13:05** with no duration kept for any of them.
`goal_guard.py` now times each check and appends a row to
`.goal/check_cost.jsonl` (under `.goal/`, which the uncommitted-tree check
excludes, so the recorder cannot fail the run it measures).

## First five rows

| # | total | `cargo check` | `app_it` | contracts | load before→after | cargo procs |
|---|---|---|---|---|---|---|
| 1 | 161.1 | 0.5 | 158.4 | 2.0 | 2.32 → 5.90 | 0 |
| 2 | 134.3 | 0.5 | 131.9 | 1.8 | 2.03 → 5.33 | 0 |
| 3 | 212.0 | 0.7 | 208.8 | 2.4 | 1.56 → 13.92 | 0 |
| 4 | 208.8 | **20.9** | 185.2 | 2.5 | 4.84 → 9.82 | 2 |
| 5 | 235.3 | **16.7** | 216.0 | 2.5 | 7.37 → 14.02 | 0 |

**`app_it` is 94.6% of all check time so far.** Every question about cadence is
a question about that one command.

## Two things visible at n=5

⭐ **`cargo check` spreads 41x — 0.5 s to 20.9 s.** Warm it is free; after a Rust
edit it is 17–21 s. So "should I check more often, or batch edits?" has an easy
half: **checking is not what costs.** Batching edits to avoid `cargo check` saves
nothing worth having.

⚠ **the suite spreads 1.64x — 131.9 s to 216.0 s — and it orders PERFECTLY by
load.** Sorted by `load_after`: 5.33 → 131.9, 5.90 → 158.4, 9.82 → 185.2,
13.92 → 208.8, 14.02 → 216.0. Five points in exact order is a 1-in-120
coincidence if load did nothing.

⛔ **but do not take that as measured yet, for a specific reason**: `load_after`
is sampled *after* the suite ran, so it is not independent of the thing being
measured — the suite contributes to its own reading. `load_before` does NOT order
the same way (row 3 has the lowest load_before and the second-highest suite
time), which is exactly the kind of disagreement that means the causal story is
not settled. What is needed is more rows, and a `load` sample taken *during* the
suite rather than around it.

## What it already says about working in the background

If the ordering holds, **running a subagent while the Stop check runs costs
roughly 45% more suite time** (rows 1–2 average 145 s at low load; rows 3 and 5
average 212 s at high load). That is a direct answer to Jon's third question, and
an uncomfortable one for how this session has been operating: the supervisor
dispatches an agent, the agent compiles, and the guard's suite — which gates
every turn — runs into it.

⭐ **the honest framing is a trade, not a mistake.** Background work is why more
gets done per hour; it is not free, and the tax lands on the one command that is
94.6% of the checking budget. The fix is probably not "stop working in the
background" but **make the gate cheaper for turns that cannot have broken it** —
a docs-only turn running the full combat suite is pure tax.

⚠ n=5. This section exists to say what is being collected and what it hints at,
not to conclude. Re-read it at n=50.

## Addendum update at n=9 — the load story weakened, and a better one appeared

⛔ **the "perfect ordering by load" from n=5 did not survive.** At n=9,
`corr(suite, load_after)` is **+0.640** — real, but nothing like the exact
ordering that looked like a 1-in-120 coincidence four rows earlier.
`corr(suite, load_before)` is **+0.382**. This is exactly why that section said
*"re-read it at n=50"*, and it is a cheap reminder that **a perfect pattern in
five points is a property of five points.**

⭐ **what replaced it is more useful — the cost of a turn is set by WHICH CRATE
you edited**, which the `cargo check` time reveals for free:

| turn class | n | mean `cargo check` | mean `app_it` |
|---|---|---|---|
| prose only | 6 | 0.5 s | **156.2 s** |
| app-level edit | 2 | 18.8 s | 200.6 s |
| floor-crate edit | 1 | **64.5 s** | **396.7 s** |

The single floor-crate row is a `crates/ambition_platformer2d_core` edit
(`movement/tuning.rs`) — a crate with **44 dependents**. It cost 64.5 s to check
and 396.7 s to test: **3.3x the cheapest turn end to end.**

⭐⭐ **that is a bridge between the two halves of this journal.**
`compile_ratchet.py` measures *edit blast radius* from the dependency graph;
`.goal/check_cost.jsonl` now measures *what a turn actually cost*. They are
measuring the same thing from opposite ends, and the floor-crate row is the first
point where the graph's prediction and the wall clock can be compared. ⚠ **one
point** — but it is the point that says the ratchet is guarding something real.

⚠ and it sharpens the cheaper-gate proposal in
`awaiting-maintainer-decision.md`: the win is **not** uniform. A prose turn pays
156 s it cannot possibly need. A floor-crate turn pays 397 s it absolutely does.
**Any subset rule must be driven by what changed, never by a fixed budget.**

## ⛔ Correction — the "35% contention swing" was never established

Repeated several times on 2026-08-08, including into `goal_guard.py`'s own
docstring: *"the same 688-unit build measured 833.9 s and 540.0 s, and the
biggest contender is the goal guard."*

`dev/ambition_dev_measurements/compile_units.jsonl` stamps contention per build — `build_load_mean`,
`build_load_max`, **`build_foreign_cargo_peak`**, `build_cores`. Reading them:

| build | dirty | wall | load mean | foreign cargo peak |
|---|---|---|---|---|
| `dev/cold` | 688 | 833.9 s | — | — (predates stamping) |
| `dev/first-party` | 688 | **540.0 s** | 9.22 | **0** |
| `release/cold` | 541 | 987.1 s | 16.26 | 1 |
| `release/first-party` | 57 | 360.1 s | 14.65 | 1 |

⭐ **the 540 s run was CLEAN — zero foreign cargo — and the 833.9 s run has no
contention data at all.** So the two cannot be compared on contention, and the
cause of the difference is **not established**. Attributing it to the goal guard
was a story that fit the numbers I had in front of me.

⭐ **and the load figures mean something different from what I assumed.** Load
14–16 on 8 cores with a foreign peak of 1 is the build's OWN parallelism — cargo
running ~8 rustc processes, each internally threaded for codegen. That is normal
for a real build, not interference. It does still explain why a unit's in-build
duration exceeds its alone-on-the-machine duration (§5), which was the part that
was right.

⚠ **the lesson is the cheap one**: the data that settles this was in the ledger
the whole time, under field names I had already read. I speculated about
contention for hours while `build_foreign_cargo_peak` sat in every row.

---

# `ambition_platformer2d_runtime` — it is not the frontend, and the rollback hypothesis was never actually tested (2026-08-08, D34)

Settles the question §3 could not: this crate is the most expensive first-party
unit in 3 of 3 release builds on 1/8 the monolith's lines, and an external review
attributed it to *"a surprisingly expensive frontend phase"* — 23.32 s of
frontend on 14.7k lines, read off `dev/ambition_dev_measurements/compile_units.jsonl`.

⛔ **Two things everyone believed about this crate are wrong, and the second one
is the expensive one.**

## 0. Method — the cache state, stated first

`rustc 1.99.0-nightly (84b36a78a)`, its own `CARGO_TARGET_DIR`
(`…/ambition-target/_nightly_probe`, deleted afterwards), `CARGO_INCREMENTAL=0`,
`cargo clean -p <crate>` before **every** run, and **exactly one unit compiled
per measurement** — verified by counting `Compiling` lines in each run's stderr,
not assumed. Machine idle, no other cargo. Features `portal`, which is what all
nine ledger rows for this unit recorded.

⚠ two instrument failures happened here and both were caught by a control:
extracting the rustc command line from `cargo build -v` and re-running it
produced **202 type errors on the unmodified command**, so every number from that
route was discarded; and a probe counter read 14 substitutions where 12 were
expected because the file already contained `// PROBED`. Neither reached a
finding. ⭐ **run the control before you interpret the treatment.**

## 1. The frontend claim is off by 14x

Dev profile, so the crate carries its `[profile.dev.package] opt-level = 0`
override. The monolith is the calibration — same instrument, same cache state,
same session:

| pass | runtime (14,819 ln) | monolith (112,794 ln) | runtime per-line |
|---|---|---|---|
| rustc `total` | **28.13 s** | **28.02 s** | 7.6x |
| frontend (parse→expand→resolve→typeck→borrowck) | **1.83 s (6.5%)** | 6.96 s (24.8%) | 2.0x |
| `type_check_crate` | 1.118 | 3.350 | 2.6x |
| **`monomorphization_collector_graph_walk`** | **11.29 s (40.1%)** | 3.86 s (13.8%) | **22.3x** |
| `generate_crate_metadata` | 12.87 | 4.77 | — |
| `codegen_to_LLVM_IR` | 11.81 | 6.66 | — |
| `LLVM_passes` | 11.91 | 7.53 | — |
| `LLVM_thinlto` | **0.00** (opt-0) | 9.34 (opt-1) | — |

⭐⭐ **The two crates cost the SAME 28 s, and the runtime's frontend is 1.8 s of
it.** Whatever is expensive here, it is not type checking — and per line the
runtime's frontend is only 2x the monolith's, against a 22x gap in the collector.

⭐ the `opt-level = 0` override is earning its keep and needs nothing added: it
removes ThinLTO from this crate outright, which is 9.3 s of the monolith's build.

## 2. Why the ledger says 23 s: `frontend_seconds` is time-to-RMETA, not the frontend

Same crate, same conditions, `cargo check` instead of a link build:

```text
  rustc total                        1.619 s
  type_check_crate                   1.124 s     (link build: 1.118 — identical work)
  generate_crate_metadata            0.008 s     (link build: 12.873)
  monomorphization_collector_...     ABSENT      (link build: 11.294)
```

⭐⭐ **That is the whole explanation.** A metadata-only build encodes metadata in
8 ms. A link build spends 12.87 s there, because metadata encoding needs
`exported_symbols`, which forces the monomorphization collector to walk the
entire instantiation graph first. Cargo's `--timings` calls everything before the
`.rmeta` lands "frontend" — so for this unit **the ledger's `frontend_seconds` is
~85–90% monomorphization collection.**

⛔ **This column has now produced two wrong conclusions**: the review's
"expensive frontend phase", and this journal's own §"a hypothesis about
`rollback/domains/`", whose ⚠ note already spotted the 2.5 s / 24.8 s gap and
guessed contention. It was not contention. `dev/compile_telemetry_schema.md` now
says so at the column.

⚠ it also means this crate's rmeta lands ~52% of the way through its own compile
against the monolith's ~37%, so it blocks its dependents for longer than its size
suggests. Same cause, and it moves with the same lever.

## 3. Where the time goes: 150,261 monomorphized instantiations

`-Zdump-mono-stats` (built into nightly — nothing was installed):

| | runtime | monolith |
|---|---|---|
| distinct generic definitions instantiated | 4,020 | 7,114 |
| **monomorphized instantiations** | **150,261** | 65,310 |
| per 1000 source lines | **10,140** | 579 |
| collector µs per instantiation | 75.2 | 59.1 |
| distinct SYSTEM types (`FunctionSystem::run_unsafe`) | **1,205** | **0** |
| distinct QUERY shapes (`QueryState::new_archetype`) | 866 | 183 |

⭐ **the per-instantiation cost is ordinary (75 vs 59 µs); the count is not.** The
collector is not slow on this crate — it has 17.5x more per line to walk.

⭐⭐ **the monolith instantiates ZERO system wrappers.** It *defines* most of the
engine's systems; the runtime *registers* them, and registration is where Bevy's
ECS generics get monomorphized. Codegen volume by origin: bevy ECS queries 30.5%,
bevy ECS systems 21.3%, std/hashbrown 18.8%, this crate's own code 12.1%, other
bevy 8.8%, `bevy_ggrs` 6.4%, schedule config 2.2%.

## 4. ⛔ The rollback subtraction, re-run with an instrument that can see it

The §"hypothesis about `rollback/domains/`" experiment commented out the same 12
lines and compared `cargo check`: 2.52/2.58/2.52 s against 2.59/2.42/2.44 s, and
concluded **"Refuted."** Re-run identically, but measuring a **build**:

| | baseline | domains commented out | Δ |
|---|---|---|---|
| source lines compiled | 14,819 | ~12,689 | −14% |
| **rustc `total` (dev)** | **28.13 / 26.32 s** | **8.66 s** | **−67 to −69%** |
| `monomorphization_collector_graph_walk` | 11.29 s | 3.23 s | −71% |
| `generate_crate_metadata` | 12.87 s | 3.64 s | −72% |
| `LLVM_passes` | 11.91 s | 2.88 s | −76% |
| **`type_check_crate`** | **1.118 s** | **1.165 s** | **+4%** |
| **monomorphized instantiations** | 150,261 | 42,725 | **−71.6%** |
| distinct system types | 1,205 | 295 | −75% |
| distinct query shapes | 866 | 239 | −72% |

⭐⭐⭐ **`−71.6%` instantiations predicts `−69%` time.** Two independent
instruments, one mechanism, and the numbers agree quantitatively rather than
directionally.

⛔ **so "the rollback registrations are not the cost" was never tested.** The old
experiment is *reproduced exactly* by the `type_check_crate` row — +4%, i.e.
nothing — and that row is **4% of the crate's build**. `cargo check` cannot
monomorphize, so it is structurally blind to 94% of what this crate costs.
⭐ **the finding generalises past this crate: `cargo check` is not a cheap proxy
for build cost on any registration-heavy unit**, and this repo has twice reasoned
as though it were.

## 5. Release: same cause, different pass

Cold, isolated, `CARGO_INCREMENTAL=0`, one unit:

```text
  total 73.68 s | LLVM_passes 35.58 (48%) | LLVM_thinlto 21.60 (29%)
                | mono collector 12.32 (17%) | type_check_crate 1.066 (1.4%)
```

Replicates §4's warm-cache release run closely (65.99 / 37.45 / 24.47), which
retroactively validates that run's backend half. Dev → release is 28 s → 74 s and
**the entire increase is LLVM** (11.9 → 35.6, plus 21.6 of ThinLTO from nothing);
the collector barely moves and the frontend does not move at all. Same 150k
instantiations — walked in dev, optimised in release.

⚠ **and release is not in anyone's loop**: neither `run_tests.py` nor
`goal_guard.py` passes `--release`. The 3.2x release gap that motivated D34 is
real and is paid by `compile_collect.py --config release` and by shipping. **In
the profile the edit loop actually pays, this crate costs the same as the
monolith.** There is still no `[profile.release]` table; this says a lever exists
there, not that it should be pulled.

## 6. Confidence, graded

| claim | confidence | why |
|---|---|---|
| The frontend is ~6% (dev) / ~2% (release), not 23 s | **very high** | `type_check_crate` measured 4x across 3 conditions (1.118/1.076/1.165/1.066); whole-crate `cargo check` 1.62 s; monolith calibration |
| `frontend_seconds` = time-to-rmeta, which in a link build subsumes mono collection | **high** | check-vs-link contrast is direct: metadata 0.008 s → 12.87 s, collector absent → 11.29 s |
| The cost is 150,261 instantiations at an ordinary per-item rate | **very high** | direct census, plus per-instantiation cost within 27% of the monolith's |
| The rollback domain registrations are ~69% of this crate's build | **high** | subtraction measured; instantiation drop predicts time drop; baseline reproduced twice (28.13 / 26.32) |
| Relocating those registrations would make the WHOLE build cheaper | ⛔ **not measured** | see below |

## 7. ⛔ What this does NOT license

* **It does not say to delete or move the rollback domains.** It says what they
  cost. Removing them removes the schema; the probe was reverted and the tree
  verified clean (`git status` shows only the pre-existing music submodule).
* **It does not price a relocation.** `rollback/domains/mod.rs` explains why they
  live here — the registration vocabulary is here and the domain crates must not
  gain a dependency on the runtime, which needs R1's schema-vocabulary extraction
  first. Moving them **moves** ~107k instantiations rather than deleting them, and
  whether ~11 crates each paying a share beats one crate paying all of it is the
  per-crate-toll question §4 of this journal explicitly says is unsettled. ⭐ the
  falsifier is cheap and specific: **relocate ONE domain and measure the whole
  workspace build both ways.** If the total does not fall, the recommendation is
  wrong; my number cannot tell you.
* **It says nothing about carving the monolith.** The monolith's profile is
  unremarkable — its cost is spread across a frontend proportional to its size
  and a ThinLTO its opt-level buys.
* ⚠ **the probed build is n=1.** The baseline is n=2 (28.13 / 26.32, 6% apart);
  8.66 s was measured once. A repeat would cost 40 s and nobody has run it.
