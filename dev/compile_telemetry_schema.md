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
| `incremental` | **RESERVED** — the report does not carry it; the collector must record the env it built under | ❌ |

⚠ **only units that did WORK are recorded.** The 2026-08-07 report lists 688
units of which 669 were cached at duration 0. "How long did nothing take" is not
a statistic, and 669 zero rows per build would bury the 19 that matter. The cache
state survives as `build_fresh_units` / `build_dirty_units`.

⚠ **`backfilled: true` means `lines` and `commit` describe the tree at INGEST,
not at build.** Drop those rows before regressing seconds against lines.

### What the collector still has to do

1. Run `cargo build --timings` (or `cargo test --no-run --timings`) with **its own
   `CARGO_TARGET_DIR`**, or strictly sequenced against every other cargo process.
   ⛔ `compile_cost.py`'s docstring records a warm no-op reading **222s** because
   two builds shared a target dir; that reading looks like a slow machine, not
   like a mistake.
2. Ingest the report **in the commit that produced it**, so `lines` and `commit`
   are true and `backfilled` stays false.
3. Pass `--label` naming the configuration, and set `incremental` once the
   collector owns the env rather than inheriting it.

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
