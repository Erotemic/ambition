# Compile telemetry: the record schema — 2026-08-08

Jon, 2026-08-08: *"I also want to see a graph over time that looks at modules,
maybe what modules they were split out of if there is a lineage, time per module,
lines of code in the module at time of compile. things like that. debug vs
release, optimization mode. we may want to quantify test time like this too.
basically we should start recording this so we can build statistics and gain more
insights into how to optimize compile time in maybe non obvious ways."*

⚠ **the schema is the deliverable, and it is the part that is expensive to get
wrong.** Rows recorded without a dimension cannot be back-filled: a year of
measurements with no opt-level column simply cannot answer an opt-level question.
So the columns land BEFORE the collector, including columns nothing populates
yet. This file is the contract.

---

## 1. One envelope, four kinds, four files

Every row in every ledger below carries the same envelope. `kind` is the
discriminator that lets the four files be read as one table.

| field | type | meaning |
|---|---|---|
| `schema` | int | this document's version. **1** today. |
| `kind` | str | `graph` · `unit` · `scenario` · `job` · `carve` |
| `recorded_at` | str | ISO-8601 with offset, when the ROW was written |
| `commit` | str | `git rev-parse --short=12 HEAD` at write time |
| `dirty` | bool | working tree had uncommitted changes |
| `run_id` | str | 12 hex chars; joins rows produced by one invocation |
| `label` | str | free text, e.g. `incremental`, `backfill: …` |

### ⛔ Why four files and not one

A `run_tests` row is **one invocation of the suite** carrying an array of
*commands*. A compile row is **one rustc invocation on one crate**. One suite job
contains hundreds of compile units, so nesting units inside `per_job` would grow
that file roughly 700x and would still be the wrong shape for every build that is
not a suite run. Forcing one row shape over both grains means every row is half
nulls — a union type with no discriminator, not a schema.

What must be shared is the **envelope**, and it is shared. The grains stay apart.

### ⚠ Why the envelope is duplicated by hand rather than imported

`scripts/run_tests.py` is the suite's own entry point. Giving it an import of a
module that itself imports a 1,500-line checker is a coupling that can take the
suite down for a reason unrelated to any test. Seven keys copied is cheaper. If a
third writer appears, revisit — not before.

---

## 2. `dev/compile_graph.jsonl` — `kind: "graph"`

**Deterministic. No build required. Populated today.** Written by
`scripts/compile_ratchet.py --update`; the same object is frozen as
`dev/compile_ratchet_baseline.json`, which is what the gate compares against.

| field | source | populated |
|---|---|---|
| `consumer` | constant — `ambition_app`, the AGENTS.md gate | ✅ |
| `line_unit` | constant; states the units so the ledger is readable in a year | ✅ |
| `first_party_crates` / `first_party_lines` | `cargo tree` ∩ `cargo metadata` | ✅ |
| `largest_unit` | `{crate, lines}` — the biggest recompilation unit | ✅ |
| `worst_edit_cost` | `{crate, lines, crates}` — the most an edit can force | ✅ |
| `watched_edit_cost` | same, for crates named in `WATCHED` | ✅ |
| `critical_path_crates` | longest serial chain of first-party crates | ✅ |
| `crate_lines` | `{crate: lines}` for every first-party crate | ✅ |
| `crates` *(baseline only)* | the full per-crate table, incl. `direct_dependents` | ✅ |
| `headroom_fraction` *(baseline only)* | the budget the gate applies | ✅ |

**Units, stated on purpose:** `lines` is *physical* lines of `<crate>/src/**/*.rs`
— blanks, comments and inline `#[cfg(test)]` modules included. Not statements,
not tokens. `test_file_lines` is the share of those in `tests.rs` / `tests/` /
`test_*.rs`, a **proxy** — this repo also writes `#[cfg(test)] mod` inline, which
no path rule can see.

⚠ **lines are a proxy for codegen cost that is wrong by 17x BETWEEN crates**
(2026-08-07: 0.45 ms/line for the monolith, 7.80 ms/line for `relativity2d`) and
reliable for one crate against *itself* over time. Use it for the second thing.

---

## 3. `dev/compile_units.jsonl` — `kind: "unit"`

**Per-module wall time. Needs a real build.** 19 real rows today, back-filled from
the 2026-08-07 report that produced the journal's findings.

Written by `scripts/compile_ratchet.py --ingest-timings <cargo-timing.html>`.

⭐ **the HTML is the source, and that is a finding rather than a fallback.**
`cargo build --timings=json` is `-Z unstable-options` on stable, which is part of
why ADR 0013's "quarterly" prescription never ran. The **stable** HTML report
embeds the identical per-unit JSON as `const UNIT_DATA`, including the per-unit
`sections` split into `frontend` and `codegen`. No nightly toolchain is needed.

| field | source | populated |
|---|---|---|
| `unit` / `version` | `UNIT_DATA[].name` / `.version` | ✅ |
| `target` | `UNIT_DATA[].target` — `""`, `"build-script"`, `… "test"` | ✅ |
| `mode` | `UNIT_DATA[].mode` | ⚠ `"todo"` in cargo 1.95 — a cargo-side placeholder, recorded verbatim anyway |
| `first_party` | membership in `cargo metadata --no-deps` | ✅ |
| `lines` | LOC of that crate **at ingest** | ✅ |
| `opt_level` | `[profile.dev]` + per-package overrides in `Cargo.toml` | ✅ |
| `seconds` | `UNIT_DATA[].duration` | ✅ |
| `start_seconds` | offset into the build; with `build_max_concurrency`, the parallelism story | ✅ |
| `frontend_seconds` / `codegen_seconds` | `UNIT_DATA[].sections` | ✅ |
| `features` | the feature set this unit compiled with | ✅ |
| `backfilled` | derived: report predates HEAD | ✅ |
| `build_profile` | HTML header `Profile:` — `dev` / `test` / `release` | ✅ |
| `build_fresh_units` / `build_dirty_units` / `build_total_units` | HTML header | ✅ |
| `build_total_seconds` / `build_max_concurrency` / `build_rustc` / `build_targets` / `build_started_at` | HTML header | ✅ |
| `build_source` | path of the ingested report | ✅ |
| `incremental` | the env the build ran under — **the collector sets it, never inherits it** | ✅ from 2026-08-08 |
| `opt_level_source` | `rustc-argv` · `rustc-argv-ambiguous` · `manifest` | ✅ from 2026-08-08 |
| `codegen_units` | `-C codegen-units=` on the rustc line | ✅ from 2026-08-08 |
| `config` / `phase` | the named configuration, and `cold` or `first-party` | ✅ from 2026-08-08 |
| `build_wall_seconds` / `build_target_dir` | wall clock of the whole build, and where it wrote | ✅ from 2026-08-08 |
| `build_load_mean` / `build_load_max` / `build_cores` / `build_foreign_cargo_peak` | contention during the build | ✅ from 2026-08-08 |
| `unblocks_at_rmeta` / `unblocks_at_completion` | successor unit names, split by WHEN cargo released them | ✅ from 2026-08-08 (four `dev` runs predate them) |

### ⭐ `unblocks_at_rmeta` is the column that makes a critical path computable

rustc's **pipelined compilation** releases a dependent when the predecessor's
*metadata* lands, not when the predecessor finishes. So on a pipelined edge only
the FRONTEND is serial and the predecessor's codegen overlaps everything
downstream. Without these two columns a ledger of durations cannot tell the
difference, and the naive reading — sum the durations along the longest chain —
produced **377.9s for a build that took 210.5s**. Recorded as unit *names*
rather than the report's local indices, which mean nothing outside their file.

⚠ **a unit with `unblocks_at_rmeta: []` is the same unit whose
`frontend_seconds`/`codegen_seconds` are null**, and for the same reason: it
emitted no metadata. Proc-macro crates, build scripts, bins, tests — and
`ambition_app`, the one lib in this workspace declaring a `cdylib`.

⚠ **the names are not unique in a COLD build.** A package compiled twice — host
against target, or two feature sets — yields two units with the same
`name` + first target token, and they collapse onto one node. 105 of a cold
build's 688 units did on 2026-08-08, so a cold DAG covers ~97% of the
unit-seconds and is approximate; `--analyze` prints the shortfall. A
`first-party` phase has no collisions and is exact, which is the phase the
conclusions were drawn from.

### ⚠ contention is RECORDED, not assumed away

⛔ two builds in ONE target dir invalidate each other — the 222s warm no-op. Two
builds in DIFFERENT target dirs corrupt nothing and simply share eight cores,
which inflates every duration and also *looks like a slow machine rather than a
mistake*. On 2026-08-08 a parallel agent held `cargo test`/`cargo check` on the
default target dir through an entire collection at load 14–18. So the collector
samples `getloadavg` every 10s and counts foreign `cargo` processes by
`/proc/<pid>/comm` (⛔ never `pgrep -f`, which matches its own shell). **Compare
ratios and rank orders across load levels; compare absolute seconds only within
one.**

### ⛔ `opt_level` is READ OFF THE RUSTC COMMAND LINE, not modelled

`scripts/compile_collect.py` passes `-v`, so cargo prints every rustc
invocation, and the collector takes `-C opt-level=`, `-C codegen-units=` and the
presence of `-C incremental=` from the line cargo actually issued. That is the
answer to a defect this repo has already shipped once —
`[profile.dev.package."*"]` does not apply to workspace members, and the first
draft of `package_opt_levels` reported the monolith at 3 when it builds at 1.
The modelled value survives as the fallback for a hand ingest of an old report,
and `opt_level_source` is the column that says which one a row got. ⚠ a
dependency's rustc line is keyed only by crate name, so `build_script_build`
collides across packages and those rows read `rustc-argv-ambiguous`.

⚠ **`incremental` is SET, not inherited.** `scripts/run_tests.py` copies the
environment and then `setdefault`s `CARGO_INCREMENTAL=0` for its children, so a
collector reading its own `os.environ` reports `true` for exactly the runs that
are off. The collector exports the variable for every build and records what it
exported, then cross-checks it against `-C incremental=` per unit.

⚠ **only units that did WORK are recorded.** The 2026-08-07 report lists 688
units of which 669 were cached at duration 0. "How long did nothing take" is not
a statistic, and 669 zero rows per build would bury the 19 that matter. The cache
state survives as `build_fresh_units` / `build_dirty_units`.

⚠ **`backfilled: true` means `lines` and `commit` describe the tree at INGEST,
not at build.** Drop those rows before regressing seconds against lines.

### The collector — `scripts/compile_collect.py`, landed 2026-08-08

```
python3 scripts/compile_collect.py --config dev --config release
python3 scripts/compile_collect.py --analyze      # builds nothing
```

It does the three things this section used to ask for. Each named configuration
gets **its own `CARGO_TARGET_DIR`** under `~/ambition-telemetry-target/<config>`
and the phases run strictly in sequence — ⛔ `compile_cost.py`'s docstring
records a warm no-op reading **222s** because two builds shared a target dir,
and that reading looks like a slow machine rather than a mistake. The report is
ingested in the commit that produced it, so `backfilled` stays false.

Two phases per configuration, because they answer different questions:

* **`cold`** — a fresh target dir, so every unit including third-party compiles.
  The only phase that can price a dependency.
* **`first-party`** — a real one-line edit appended to **every** first-party
  crate's `src/lib.rs`, then rebuilt, then the original bytes written back
  (⛔ never `git checkout --`). This is the recompilation the repo pays, and the
  phase to regress against `dev/compile_graph.jsonl`.

⭐ **the two phases are in different REGIMES and want opposite optimisations.**
Compare *total unit-seconds ÷ cores* against the *dependency floor*: whichever
is larger is what the build is paying. Measured 2026-08-08 — cold: 767.6s of
packing against a 418.9s floor, so **core-bound**; the 55-crate rebuild: 123.9s
of packing against a 168.4s floor, so **dependency-bound**. Codegen is ~73% of
the work in both, and moves the rebuild's floor by a sixth of what the frontend
does. `--analyze` prints both bounds for every run; the write-up is
`dev/journals/compile-time-and-disk-2026-08-07.md`, addendum 2.

A configuration may also carry `manifest_edits` — exact-text substitutions
applied for the run and reverted by writing the original bytes back. That is how
`dev-app-rlib-only` prices the app's `cdylib`: a manifest knob is a dimension no
cargo flag can express, and it shares the base configuration's target dir so the
comparison is warm.

---

## 4. `dev/compile_cost.jsonl` — `kind: "scenario"`

**The edit→rebuild stopwatch.** Written by `scripts/compile_cost.py`. 4 rows
today; the grain is one *scenario* (warm, edit, rebuild, revert), not one unit.

| field | source | populated |
|---|---|---|
| `scenario` / `why` / `edited_file` / `command` | the scenario table in that script | ✅ |
| `warm_noop_seconds` / `after_edit_seconds` / `restore_seconds` | measured | ✅ |
| `profile` / `opt_level` / `incremental` | **NEW in schema 1**, explicit columns | ✅ from 2026-08-08 |
| `machine_cores` / `machine_linker` / `machine_platform` / `machine_cargo` | measured | ✅ |

### ⚠ The four schema-0 rows disagree with each other, and here is the mapping

The incremental setting was recorded as a stringly-typed side effect of how the
run was invoked: `machine_cargo_incremental` is `"1"` in two rows and
`"(config default)"` in two, with `env` `{"CARGO_INCREMENTAL": "1"}` vs `{}`.
They are **not rewritten** — this is an append-only ledger and rewriting it is
the failure `check_absence_contracts.py`'s own baselines rail against. Normalise
at read time:

| schema-0 row | `incremental` | `profile` | `opt_level` |
|---|---|---|---|
| `env == {"CARGO_INCREMENTAL": "1"}` | `true` | `test` | `1` |
| `env == {}`, label `baseline-config-default`, commit `ae624289` | `false` | `test` | `1` |
| `env == {}`, label `config-incremental-on`, commit `a6bd0b2f` | `true` | `dev` | `1` |

`.cargo/config.toml` set `incremental = true` on 2026-08-07; a row with
`"(config default)"` therefore means **off** before that commit and **on** after,
which is precisely why the field could not stay stringly typed.

---

## 5. `dev/run_tests_cost.jsonl` — `kind: "job"`

**Test time.** Written by `scripts/run_tests.py`; 75 rows predate this schema.
Grain: one suite invocation, with a `per_job` array of commands.

Its vocabulary is **reused, not replaced** — it got there first and it is right:

| field | meaning |
|---|---|
| `seconds` | total wall time |
| `executed_seconds` | the part of `seconds` spent RUNNING tests (libtest's own "finished in Xs") — so `seconds - executed_seconds` is the build graph |
| `per_job[]` | `{job, command, ok, seconds, executed_seconds}` |
| `jobs` / `passed` / `exhaustive` / `filtered` | plan and outcome |
| `finished` | float epoch — **superseded** by the envelope's `recorded_at`, kept for the 75 existing rows |

⚠ **`executed_seconds` counts LIBTEST only**, so the pytest jobs read 0.0.

The `seconds` / `executed_seconds` split is the same idea as the unit ledger's
`frontend_seconds` / `codegen_seconds`: a total, and the split that matters. They
are **named after their own sources** rather than unified, because renaming
cargo's `frontend`/`codegen` sections away from cargo would make the ingester's
provenance unreadable.

**Added in schema 1:** the full envelope, plus `profile`, `opt_level` and
`incremental` — the suite forces `CARGO_INCREMENTAL=0` for itself while the dev
loop runs with it on, and until now nothing recorded which of the two a row was.

---

## 6. `dev/carve_lineage.jsonl` — `kind: "carve"`

⛔ **the only dimension with NO other source.** `git log --follow` approximates a
file move, gives up on a module split across two homes, and records nothing about
why. A carve knows what it split from at the moment it splits and never again.

Appended by the carve's own commit:

```
python3 scripts/compile_ratchet.py --record-carve \
    --from <path> --to <path> --from-crate <c> --to-crate <c> --why '<one sentence>'
```

| field | meaning |
|---|---|
| `from_path` / `to_path` | repo-relative, before and after |
| `from_crate` / `to_crate` | equal for an intra-crate module move; different for a crate carve |
| `lines_at_split` | LOC that landed at `to_path`, measured when the row is written |
| `why` | one sentence. The column git cannot hold. |
| `recorded_from` | `live` = the carve's own commit wrote this. Anything else NAMES the record it was transcribed from. |
| `happened_in` | the carve's commit, when the row is written later |

⚠ **not back-filled for carves that already happened.** A reconstructed lineage
that reads like a recorded one is worse than a gap, because the next reader
cannot tell which is which. The one seeded row is *transcribed* from a written
record (`conversation/mod.rs`'s own docstring) and says so in `recorded_from`.

---

## 7. What no column here can answer

* **Whether a carve made the wall clock faster.** Only a timed build says that,
  and only against its own machine and linker. The graph ledger says what a carve
  *changed about the build's shape*; `compile_units.jsonl` says what that cost in
  seconds, once somebody runs a build in a quiet target dir.
* **Per-MODULE time inside one crate.** Cargo's unit is a crate. Sub-crate
  attribution needs `-Z self-profile`, which is nightly. `crate_lines` plus
  lineage is the deterministic stand-in until then.
* **`incremental` for a unit row.** The timing report does not carry it. It is a
  reserved column, and a collector that owns its own environment can fill it.
