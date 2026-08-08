# Compile time, disk, and three wrong suspects — 2026-08-07

Jon: *"Currently compile time is too long even after a warm recompile."* Then:
*"We are wasting so much time on compiles and links, especially when agents run
tests."*

Everything below was measured on this box — **8 cores, mold, warm tree** — with
`scripts/compile_cost.py`, which was written during this session and records to
`dev/compile_cost.jsonl`. Numbers do not transfer across machines; the method
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
50.3s. Across the whole agent-loop build, **255 of 313 unit-seconds are
codegen**. The build is LLVM-bound. Anything that speeds parsing, resolution or
macro expansion is aimed at 18% of the problem.

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
`dev/compile_units.jsonl` (19 rows, real durations). Re-derived from the
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
