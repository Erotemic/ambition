# Compile time, disk, and three wrong suspects — 2026-08-07

Jon: *"Currently compile time is too long even after a warm recompile."* Then:
*"We are wasting so much time on compiles and links, especially when agents run
tests."*

Everything below was measured on this box — **8 cores, mold, warm tree** — with
`scripts/compile_cost.py`, which was written during this session and records to
`dev/ambition_dev_measurements/compile_cost.jsonl`. Numbers do not transfer across machines; the method
does.

## The headline

The agent loop — edit one function in `ambition_platformer2d_actor_monolith`,
then build the app's test binary, which is what gets paid before a single test
runs:

| config | seconds |
|---|---|
| repo default (incremental off) | **104.9** |
| `CARGO_INCREMENTAL=1` | **21.3** |
| `opt-level = 0` instead of `1` | 74.3 |

Incremental was turned on in `.cargo/config.toml` as a result. The suite keeps
forcing it off for itself; see "the two configs" below.

## ⛔ Three suspects that were wrong, in the order they were eliminated

**1. The link.** The obvious one: `app_it` is a **769 MB** binary and there are
121 link products over 10 MB in the target dir totalling 33 GB. Relinking one
after touching only its own source: **9.3s**. Not the problem, and it would have
been easy to spend a day on `lld` vs `mold` or `split-debuginfo` believing it
was.

**2. The frontend.** `cargo check` of the monolith is 8.5s; *building* it is
50.3s. ~~Across the whole agent-loop build, **255 of 313 unit-seconds are
codegen**. The build is LLVM-bound. Anything that speeds parsing, resolution or
macro expansion is aimed at 18% of the problem.~~
⛔ **BOTH SENTENCES ARE WRONG and the correction is 160 lines below, which is
too far — a reader who stops at the summary carries the error away.** "255 of
313" reproduces from no reading; the real split is **72–80% codegen** across four
fresh builds (§ "the codegen/frontend split", 2026-08-08). And "aimed at 18% of
the problem" is wrong in a way that matters more than the arithmetic: **on a
REBUILD, halving the frontend is worth 5.2x halving codegen**, because a rebuild
is dependency-bound rather than core-bound. The rebuild is what an agent pays
before one test runs. See § "two regimes".

**3. Crate size.** This is the one worth remembering:

```
  0.45 ms/line   ambition_platformer2d_actor_monolith   110,911 lines   50.3s
  1.50 ms/line   ambition_content                        20,708 lines   31.1s
  1.73 ms/line   ambition_demo_mary_o                    15,110 lines   26.2s
  4.38 ms/line   ambition_platformer2d_runtime           14,649 lines   64.2s
  5.45 ms/line   ambition_platformer2d_host               2,973 lines   16.2s
  7.80 ms/line   ambition_relativity2d                    2,705 lines   21.1s
```

⭐⭐ **The monolith is the CHEAPEST crate per line in the workspace.** Its 110k
lines codegen faster than four crates a fraction of its size. `relativity2d`
costs 17x more per line; `runtime` checks in 2.5s and builds in 64.2s — a **26x**
codegen ratio, `relativity2d` **31x**.

That does not argue against carving the monolith. An edit anywhere in it still
rebuilds all of it, and it sits at the head of a 6-deep serial chain
(monolith → sim_view → runtime/render → provider/host → content/demos → app)
where `user/real` was 1.3 — seven of eight cores idle. But **a carve sold on
compile time alone would disappoint**, and the crates worth profiling for
codegen are the small generic-heavy ones, not the big one.

Next lead, unexplored: `runtime` is 61.7 of its 64.2 seconds in codegen and
holds the rollback registry (82 encoded types, each monomorphising the snapshot
machinery). That is a fix inside one crate rather than a carve.

## The two configs, and why they differ

`.cargo/config.toml` now sets `incremental = true`; `scripts/run_tests.py` keeps
`CARGO_INCREMENTAL=0` for suite jobs. This is deliberate and the reasons are
opposite:

* **the dev loop** re-edits the same crates repeatedly — maximum cache reuse,
  measured 4.9x, and the cache stayed around 7 GB across a day of use.
* **the suite** compiles a dozen feature variants that share nothing, so a job
  either has no cache to hit or needs no compile at all. On 2026-07-31
  `target/debug/incremental` alone reached **110 GB** and builds died with
  ENOSPC mid-suite.

⚠ **switching between them costs a full rebuild (~100s, measured).** The
incremental flag is part of every unit's fingerprint, so `cargo check` and a
`./run_tests.sh` run do not share artifacts. The cost is paid on entering and
leaving the suite rather than per edit, which is why the trade still wins — but
a 100s "warm" build after a suite run is expected, not a symptom.

⛔ **if links start failing, delete the cache first.** Three times on 2026-07-31
`mold: error: undefined symbol: anon.<hash>.llvm.<hash>` came from a stale rlib,
cured only by discarding `debug/incremental`. That is why this was off for a
week. `rm -rf "$CARGO_TARGET_DIR/debug/incremental"` is the whole cure.

## Disk: incremental is not what fills it

| | |
|---|---|
| `debug/deps/` | **51 GB** |
| — duplicate fingerprint variants inside it | **15.2 GB** |
| `debug/incremental/` | 6.8 GB |

**`app_it` alone exists in four variants — 3.1 GB for one test target.** Every
distinct flag combination (`--features`, a profile override, incremental on/off)
leaves a complete artifact set, and cargo never garbage-collects the previous
one. `ambition_game_bin`, `ladder_rig`, `stage_diagram` and others are also at
four copies.

One session of compile experiments — varying only `CARGO_INCREMENTAL` and
`opt-level` — grew the target dir from 51 GB to 63 GB. An agent trying a feature
flag does the same thing at 769 MB a binary. **That, not incremental, is why the
disk fills quickly.**

`scripts/sweep_cargo_target.sh` is the remedy and it already handles both
(`--deep` also discards incremental state). It needs `cargo-mark-sweep`, which
was not installed until this session — so the tool you reach for when the disk
fills was itself unavailable, which is part of why it kept filling.

## ⛔ Two measurement hazards that produce a plausible wrong number

Both are recorded in `scripts/compile_cost.py` because neither raises an error;
each just reports a believable time that is not the answer.

**A cold cache measures what the tree owed, not what the edit cost.** The same
scenario read **4m52s** and then **9.3s** minutes apart, and only the second was
the answer — the first was rebuilding upstream commits that had landed while the
session ran. Every scenario now warms with the identical command first.

**Two cargo builds whose flags differ rebuild each other's work in a shared
target dir.** A warm no-op that should be under a second read **222s** because an
incremental-on build was running beside an incremental-off one. It looks like a
slow machine, not like a mistake. (This repo already knows the worktree version:
never share a target dir across worktrees.)

## Instrumentation that existed and was not being used

ADR 0013 prescribes `cargo build --timings` "quarterly" and it is plumbed
through `run_game.sh --timings` and `scripts/profile_desktop.sh --timings`.
Nothing ran it, and it answers a different question anyway: it profiles ONE
build, not the edit→feedback loop that is actually being paid for.

`scripts/compile_cost.py` measures an EDIT — appends a real function to a real
source file, runs a real cargo command, reverts, appends a row. It refuses to
run on a dirty target file and restores saved bytes rather than calling
`git checkout`, which would delete uncommitted work.

---

# Addendum, 2026-08-08 — the deterministic half, and two numbers that did not
# reproduce

Jon: *"what will be valuable is tracking compile times and measuring if we get
any wins from making crate carves. I want to quantify those compile wins as we do
those. And to guard against compile time regressions."*

The gate is `scripts/compile_ratchet.py`; the schema is
`dev/compile_telemetry_schema.md`. **It never builds anything** — every number
comes from `cargo tree`'s resolved graph and from line counts — because a
wall-clock threshold on a shared box fails randomly, gets waived, and then gets
ignored. Wall clock stays in the ledgers and is never a gate.

## ⛔ The `conversation` carve buys 0.87%, and 0.00% for the thing it was for

C4e concluded the `conversation` carve is "a COMPILE-ISOLATION win, not a
footprint win", and nobody could say how big. Simulated from the graph
(`--carve crates/ambition_platformer2d_actor_monolith/src/conversation`):

| | before | after | |
|---|---|---|---|
| largest recompilation unit | 111,579 | 109,412 | **−1.94%** |
| edit cost, rest of the monolith | 248,672 | 246,505 | **−0.87%** |
| **edit cost, `conversation` itself** | 248,672 | 248,672 | **±0.00%** |
| critical path (crates) | 12 | 12 | 0 |

⭐ **the isolation runs ONE direction only, and that is the whole finding.** Six
files in the monolith name `crate::conversation`, so the new crate lands BELOW
the monolith and an edit to it still rebuilds the monolith and all 16 crates
above. What the carve buys is the other direction: edits to the remaining 109k
lines stop rebuilding these 2,167.

⚠ so the compile-time argument for this carve is **worth about 1%**, and the
architectural argument (zero inward edges, all outward edges already below the
monolith, the carve is a `Cargo.toml`) is the entire case. That is a decision
somebody can now make. The journal above already predicted this shape — *"a carve
sold on compile time alone would disappoint"* — and the simulator now prices it.

⛔ **and the shape that WOULD pay is a SIBLING carve**: a module nothing in the
owner names, which lands beside the crate instead of under it, compiles in
parallel with it, and skips it entirely on an edit. The simulator derives which
of the two you get from the coupling rather than letting a proposal assert it.

## ⚠ Two numbers above did not reproduce, and one of them is this file's headline

`cargo build --timings` writes an HTML report that **embeds the identical
per-unit JSON** as `const UNIT_DATA`, including the frontend/codegen split. Two
of those reports were still on disk from 2026-08-07 and are now ingested into
`dev/ambition_dev_measurements/compile_units.jsonl` (19 rows, real durations). Re-derived from the
artifact:

```
total 313.6s   frontend 94.7s (30%)   codegen 197.6s (63%)   unattributed 21.4s (7%)
```

* **"255 of 313 unit-seconds are codegen" does not reproduce** — from either
  report. The reproducible figure is **197.6s (63%)**, or 218.9s (70%) if you
  define codegen as `duration − frontend`. Neither is 255.
* **"aimed at 18% of the problem" does not reproduce** — the frontend is **30%**.

⭐ **the direction survives, the magnitude does not.** The build is still
codegen-dominant and the three eliminated suspects are still eliminated. But 63%
is not 82%, and the difference matters to anything that gets prioritised on it.
The 21.4s unattributed is the three `ambition_app` units, which carry no codegen
section at all — that is the link, and it is 7%, consistent with the 9.3s relink
measured above.

The lesson is the ledger, not the arithmetic: these numbers were read off a
report by hand and could not be re-checked until the report was parsed into rows.
They can be now.

## ⚠ `[profile.dev.package."*"]` does not apply to workspace members

The first draft of the per-unit `opt_level` column reported the monolith at
opt-level **3**. It builds at **1** — the glob applies to dependencies only, and
`Cargo.toml` says so in prose two lines above the table. A ledger that
contradicts its own manifest is worse than one with the column missing.

---

# Addendum 2, 2026-08-08 — the COLLECTOR ran, and the build has two regimes

`scripts/compile_collect.py` runs real builds under named configurations, each
in its own `CARGO_TARGET_DIR`, and writes one row per compile unit. Four dev
runs landed (a cold build, two identical 55-crate rebuilds, one manifest probe)
plus a release configuration. `--analyze` reads them back and builds nothing.

⚠ **every number below was measured on a CONTENDED box and says so.** A parallel
agent was running `cargo test`/`cargo check` against the default target dir
throughout, at load 14–18 on 8 cores. Different target dirs, so nothing was
invalidated — but the durations are inflated by the overlap, which is why the
collector now samples `getloadavg` and counts foreign `cargo` processes onto
every row. **Ratios and rank orders survive contention; absolute seconds do
not.**

## ⭐⭐ The finding: a COLD build and a REBUILD want opposite optimisations

Two bounds decide a build's wall clock, and whichever is larger is the one you
are paying:

* **perfect packing** = total unit-seconds ÷ cores. No scheduler beats it.
* **the dependency floor** = the longest chain, with rustc's *pipelined*
  semantics: a dependent is released when the predecessor's **metadata** lands,
  not when it finishes. Infinite cores do not beat it.

```
                          work/8    dep floor   actual   binding constraint
  dev cold, 583 units     767.6s      418.9s    833.9s   CORES
  dev rebuild, 57 units   123.9s      168.4s    210.5s   THE DEPENDENCY CHAIN
  release rebuild, 57     163.4s      262.6s    360.1s   THE DEPENDENCY CHAIN
```

⭐ **the cold build is core-bound and the rebuild is dependency-bound, and the
two want OPPOSITE optimisations.** Halving one half of the work moves both
bounds, so the achievable saving is `max(new floor, new work ÷ cores)` — quoting
the floor alone overstates it. Done honestly:

```
                        halve CODEGEN   halve the FRONTEND
  cold, 583 units          −282.7s            −101.1s      codegen wins 2.8x
  rebuild, 57 units         −11.8s             −61.6s      frontend wins 5.2x
```

⭐ **the same lever is worth 2.8x in one build and a fifth as much in the
other.** In the cold build cores are the limit, so removing work removes clock
and codegen is 73% of the work — the original journal's conclusion, and it holds
*for a cold build*. In the rebuild the chain is the limit, and on a pipelined
edge **only the frontend is serial**, so a quarter of the work carries almost
all of the wait. Reproduced across all four rebuild-shaped runs (frontend
leverage 3.6x, 4.0x, 5.2x, 4.1x). **The rebuild is what an agent pays before a
single test runs, and it is the one the repo has been prioritising with the cold
build's number.**

⚠ **and the naive serial chain overstates by 2.2x.** `critical_path_crates`
models the chain as a sum of full unit durations: 377.9s for a build that took
210.5s. Summing durations along a chain of a build that finished sooner is not a
paradox, it is the proof that the chain is not serial.

## Do the ratchet's four guarded numbers predict seconds? Mostly yes, one no

Nobody had tested this. Both halves now exist, so:

| guarded number | tested against | verdict |
|---|---|---|
| `largest_unit_lines` | per-crate seconds, n=55 | ✅ **rho +0.83 … +0.86** across four runs |
| `worst_edit_cost_lines` | seconds over the same dependent closure | ✅ **rho +0.99** — but see below |
| `watched_edit_cost_lines` | same | ✅ same |
| `critical_path_crates` | the measured chain | ⚠️ **right in hops, wrong by 2.2x in seconds** |

⛔ **the LINE WEIGHTING in `edit_cost_lines` contributes nothing.** Replacing
lines with a bare count of crates in the closure — `edit_cost_crates`, no
weighting at all — predicts the measured seconds *equally well* (rho +0.988 vs
+0.991 cold; **+0.984 vs +0.977 by Pearson in the rebuild, where the null model
wins**). Both quantities are sums over nested closures, so the rank order is
carried by closure size and the LOC is decoration. The guard is sound; what it
is really guarding is **how many crates an edit reaches**, and it should be read
that way.

⚠ `seconds vs lines` is **rho +0.86, r +0.66-0.84** — strong in rank, weak in
magnitude, because ms/line spans **29x to 39x between crates**. Lines rank
crates well and price them badly, which is exactly what the schema doc already
warned.

⭐ **and the monolith-is-cheapest finding reproduces at n=55, not n=6.**
0.67 ms/line, **3rd cheapest of 55 crates**, against a median of 3.18 and
`ambition_inventory_ui` at 13.99. Codegen share tracks it (rho +0.49): the
cheap-per-line crates are 0–60% codegen, the expensive ones 79–88%.

## The frontend/codegen split, measured fresh — and which prior figure was right

| source | frontend | codegen | unattributed |
|---|---|---|---|
| journal headline (2026-08-07, by hand) | 18% | **81%** ("255 of 313") | — |
| addendum 1, re-derived from that artifact | 30% | **63%** (197.6 / 313.6) | 7% |
| **fresh: dev cold, 688 units, 6,346 unit-s** | **18%** | **72%** | 10% |
| **fresh: dev rebuild, 57 units, 932 unit-s** | **25%** | **73%** | 2% |
| **fresh: release cold, 541 units, 7,164 unit-s** | 19% | **80%** | 1% |
| **fresh: release rebuild, 57 units, 1,307 unit-s** | 17% | **78%** | 5% |

**Addendum 1 is right about the artifact** — 197.6/313.6 is exactly 63.0% and
the original's "255" does not reproduce from that report by any reading. But
**63% is not the repo's split**; it is one 19-unit partial rebuild. Four fresh
builds, the largest an order of magnitude bigger, say **72–80% codegen and
17–25% frontend**. So the correct statement is: the original arithmetic was
wrong, addendum 1's arithmetic was right *about its artifact*, and neither
percentage is the general figure — **codegen is 72–80% depending on profile, and
that is the number to prioritise on.**

⛔ **and the "unattributed 7% is the link" guess in addendum 1 is wrong.** The
unattributed bucket is **every unit rustc compiles without emitting rmeta** —
in the cold build that is 89 units, overwhelmingly proc-macro crates
(`derive_more` 53.4s, `serde_derive` 34.2s, `bevy_reflect_derive` 29.0s). Same
cause as their missing frontend/codegen split, and the same cause as the next
section.

## ⛔ `ambition_app` is the only lib in the workspace that cannot pipeline

It is the only crate declaring `crate-type = ["rlib", "cdylib"]` — the Android
`.so`. A unit emitting a cdylib emits no metadata, so cargo cannot pipeline it
**in either direction**: `ambition_app` waits for `ambition_content`'s full
codegen instead of its rmeta, and its own test and bin targets wait for the
whole thing. It is the one unit in every report with `sections: null`, which is
how it was found.

Probed by flipping the manifest to `["rlib"]` and rebuilding — `--config
dev-app-rlib-only`, which shares the `dev` target dir so the comparison is warm:

* ✅ **the mechanism is confirmed by measurement**: `ambition_app` gains a
  frontend/codegen split, and `ambition_content` stops being a
  wait-for-completion edge.
* **the dependency floor drops 168.4s → 151.4s (−10%)** — and the rlib-only run
  carried *more* total work (1102 vs 991 unit-seconds, it was more contended),
  so the drop is if anything understated.
* ⚠ **the wall clock did not move: 210.5s → 211.2s.** The second run ran at 12%
  higher load with three foreign cargo processes against one. A 17s effect
  cannot be resolved against that. **Unresolved, not disproved** — and it will
  stay unresolved on an 8-core box that is already saturated.

The fix, if it is taken, is not deleting the cdylib — Android needs it. It is
moving it to its own thin crate so the app's lib stays pipelineable.

## debug vs release, with the opt-level READ OFF THE RUSTC LINE

⛔ every `opt_level` here came from `cargo -v`'s rustc invocation, not from the
manifest. The manifest model has been wrong once already (`package."*"` does not
apply to workspace members), and the release profile is not in `Cargo.toml` at
all — a model would have had to guess cargo's defaults. All 55 first-party libs
read **3** in release; in dev, **52 read 1 and 3 read 0** (`runtime`, `render`,
`app` — exactly the three the manifest pins, confirming the model *and* the
measurement agree once the model is right).

| | dev (`test` profile) | release | |
|---|---|---|---|
| wall, 55-crate rebuild | 210.5s | **360.1s** | 1.71x |
| first-party lib seconds | 982.0s | 1,281.0s | 1.30x |
| **frontend** | 253.7s | **216.8s** | **0.85x — it goes DOWN** |
| codegen | 717.6s | 1,023.9s | 1.43x |
| dependency floor | 168.4s | 262.6s | 1.56x |

⭐ **opt-level is a pure codegen tax; the frontend gets cheaper.** So release
moves the build back toward the codegen-bound regime, and the two levers even
out: halving codegen saves 44.4s, halving the frontend 43.6s. The per-crate
sensitivity is 1.5x–4.9x and it is not uniform —
`ambition_platformer2d_provider` **4.88x**, `ambition_app` 3.74x,
`ambition_demo_mary_o` 3.24x, against `ambition_render` 1.48x. A crate's
opt-level sensitivity is a property worth knowing before pinning one, and it is
now a column.

## What a cold build actually spends on

**81% of a cold build is dependencies** (5,133 of 6,346 unit-seconds, 627 units)
and every one of the 525 measured is at **opt-level 3**, from
`[profile.dev.package."*"]`. `bevy_pbr` alone is 334.3s — 1.7x the monolith.
That is the price of the fast-dependencies trade, it is paid once per target
dir, and it is why a new feature variant is expensive.


---

## 2026-08-08 — a hypothesis about `rollback/domains/`, and its refutation

Recorded because the refutation is the useful part and it took one hour.

**The hypothesis.** Three independent findings converged on
`ambition_platformer2d_runtime`'s `rollback/domains/` — 12 files, 2,130 lines:
it is the sole in-crate reference for `ambition_cutscene` and `ambition_items`
(so two leaked capability crates arrive through ~9 lines); it holds 74 of the
crate's 79 generic functions and 211 of ~222 `rollback_component_*<T>` call
sites; and it is why C7 concluded the declare/install split *"cannot be done by
data alone — installation is generic per type."* The runtime costs 24.8s of
frontend against the monolith's 23.9s from 7.6x fewer lines, so the generic
surface looked like the cause.

**The falsifier.** Comment out the 11 `pub(super) mod` lines and the 11
`domains::*::register(app)` calls, leaving a compilable crate, and compare full
rechecks (`cargo clean -p` then `cargo check -p`) on a quiet machine:

```text
  baseline (domains present)   2.52  2.58  2.52 s
  subtracted (domains gone)    2.59  2.42  2.44 s
```

**Refuted.** Removing 2,130 lines and 94% of the crate's generic surface changes
type-check time by nothing measurable.

⚠ **bounded**: `cargo check` of this crate is **2.5s** while a full build's
`--timings` attributes it **24.8s** of frontend. A 10x gap — `check` is not the
build's frontend phase, and the 24.8s was also taken under load 14–18. What is
refuted is *the generics dominate type-checking*. Whether they dominate a build's
frontend phase is unmeasured and wants nightly `-Z self-profile`.

⛔ **and it took four attempts to measure, three invalid.** `touch` + `cargo
check` read **0.64s** for a 14,747-line generic-heavy crate — and a second
experiment was run on that number before anyone noticed it was impossible. A real
content edit read 0.77s, still fresh, because **incremental is ON**, so it timed
the cost of adding one function. Only `cargo clean -p` forces the full recheck.
⭐ **an implausible number is a broken instrument, not a result**; ask what the
smallest plausible value is before interpreting any timing.

---

## 2026-08-08 — the ratchet's first reading on a real day's work

Landed at 00:47 and re-run after ~60 commits of ordinary work (seven bug fixes,
a fork closure, two guards). It **passes**, and every guarded number moved:

| | baseline | after a day | Δ |
|---|---:|---:|---:|
| `largest_unit_lines` | 111,579 | 111,805 | **+226** |
| `worst_edit_cost_lines` | 427,218 | 427,635 | **+417** |
| `edit_cost_lines` (monolith) | 248,672 | 248,965 | **+293** |
| `critical_path_crates` | 12 | 12 | 0 |

⭐ **this is the answer to "quantify the wins as we do those", pointed the other
way**: a day of fixes cost **226 lines** on the largest recompilation unit, all
of it inside the 2% headroom, and **the serial chain did not lengthen** — which
is the number that would have made the day quietly expensive.

⚠ **and it is a live instrument rather than a green tick.** Every guarded figure
drifted on the first ordinary day it was exposed to, which is what distinguishes
a ratchet that is measuring from one that cannot fail. The headroom is two-sided,
so the same run reports a CARVE as stale slack rather than passing silently.

## 2026-08-08 — the contention cost, measured: 833.9s → 540.0s for the same 688 units

A second collection run, taken deliberately on a QUIET machine after the last
worker finished. It is not the 57-unit rebuild that was wanted — the telemetry
target dir was cold, so it built 688 units — **which by accident makes it directly
comparable to the original cold build.**

```text
  688 units  833.9s   load 14–18, goal-guard cargo running   (2026-08-08 early)
  688 units  540.0s   load 9.22, foreign cargo peak 0        (2026-08-08 late)
```

⭐ **the same work, 35% faster, with nothing changed but the machine.** That is
the price of the goal guard's own Stop-hook checks (`cargo check -p ambition_app`
plus the 318-test `app_it`) running on the default target dir every time a turn
ended — and it is the first direct measurement of it rather than an inference
from two identical rebuilds differing 12%.

⛔ **so every absolute second in the earlier session is inflated by roughly a
third**, and the ratios that survived contention are the only figures from it
worth quoting. The regime split, the codegen share and the rank orders all hold;
the seconds do not.

⚠ **and the tree moved ~70 commits between the two runs**, so this is not a clean
controlled comparison — a same-tree A/B would need the goal disarmed for one run.
The direction and rough magnitude are what it supports, not the exact 35%.
