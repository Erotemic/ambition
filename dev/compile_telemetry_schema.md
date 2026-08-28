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

## 2. `dev/ambition_dev_measurements/compile_graph.jsonl` — `kind: "graph"`

**Deterministic. No build required. Populated today.** Written by
`scripts/compile_ratchet.py --update`; the same object is frozen as
`dev/compile_ratchet_baseline.json`, which is what the gate compares against.

| field | source | populated |
|---|---|---|
| `consumer` | constant — `ambition_app`, the AGENTS.md gate | ✅ |
| `line_unit` | constant; states the units so the ledger is readable in a year | ✅ |
| `first_party_crates` / `first_party_lines` / `first_party_seconds` | `cargo tree` ∩ `cargo metadata`, priced by `unit_weights` | ✅ |
| `largest_unit` | `{crate, lines}` — the biggest recompilation unit | ✅ |
| `largest_unit_seconds` | `{crate, seconds}` — the most EXPENSIVE one, which is a different crate | ✅ |
| `worst_edit_cost` | `{crate, lines, crates}` — the most an edit can force | ✅ |
| `worst_edit_cost_seconds` | `{crate, seconds, crates}` — the same closure in measured seconds | ✅ |
| `watched_edit_cost` | both, for crates named in `WATCHED` | ✅ |
| `critical_path_crates` | longest serial chain of first-party crates | ✅ |
| `unit_weights` | the frozen ms/line table + the config and builds it came from | ✅ |
| `unpriced_crates` | crates in the graph with no measurement, priced at the median | ✅ |
| `crate_lines` | `{crate: lines}` for every first-party crate | ✅ |
| `crates` *(baseline only)* | the full per-crate table, incl. `direct_dependents`, `ms_per_line`, `seconds`, `edit_cost_seconds` | ✅ |
| `headroom_fraction` *(baseline only)* | the budget the gate applies, both directions | ✅ |

⭐ **`unit_weights` is the join back to §3.** It is `ms/line` per crate, taken
from `dev/ambition_dev_measurements/compile_units.jsonl` filtered to ONE profile and ONE cache class, and
FROZEN here rather than recomputed — so appending a build's rows cannot move a
guarded number without an explicit `--update`. A crate with no measurement is
priced at the population median and named in `unpriced_crates`, which raises an
`UNPRICED` finding: a least-squares fit of `seconds ~ a + b·lines` over the 55
first-party crates reads **R² = 0.12**, so no arithmetic over a crate's size can
substitute for measuring it.

⛔ **`build_label` is recorded in `unit_weights.builds[].untrusted_label` and
never selected on.** The cache class comes from `build_fresh_units` /
`build_dirty_units`; see §3's note on the build that says `first-party` and
recompiled everything.

**Units, stated on purpose:** `lines` is *physical* lines of `<crate>/src/**/*.rs`
— blanks, comments and inline `#[cfg(test)]` modules included. Not statements,
not tokens. `test_file_lines` is the share of those in `tests.rs` / `tests/` /
`test_*.rs`, a **proxy** — this repo also writes `#[cfg(test)] mod` inline, which
no path rule can see.

⚠ **lines are a proxy for codegen cost that is wrong by 17x BETWEEN crates**
(2026-08-07: 0.45 ms/line for the monolith, 7.80 ms/line for `relativity2d`) and
reliable for one crate against *itself* over time. Use it for the second thing.

---

## 3. `dev/ambition_dev_measurements/compile_units.jsonl` — `kind: "unit"`

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
| `frontend_seconds` / `codegen_seconds` | `UNIT_DATA[].sections` | ✅ (⛔ read the warning below before using `frontend_seconds`) |
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

### ⛔ `frontend_seconds` is TIME-TO-RMETA, and that is not rustc's frontend

The name is cargo's (`UNIT_DATA[].sections`) and is kept for provenance, but it
means *"when did this unit's metadata unblock its dependents"* — **not** "how long
did parsing, macro expansion and type checking take". The gap is not small,
because metadata encoding needs `exported_symbols`, which forces the
**monomorphization collector** to walk the whole instantiation graph first. So on
a registration-heavy unit `frontend_seconds` is mostly a *middle-end* number.

Measured on `ambition_platformer2d_runtime` (2026-08-08, D34 — cold, own target
dir, `CARGO_INCREMENTAL=0`, one unit compiled, idle machine):

| | metadata-only build (`cargo check`) | link build |
|---|---|---|
| `type_check_crate` | 1.124 s | 1.118 s |
| `generate_crate_metadata` | **0.008 s** | **12.873 s** |
| `monomorphization_collector_graph_walk` | **absent** | **11.294 s** |

That unit's ledger row reads `frontend_seconds: 23.32`. Its actual rustc frontend
is **1.8 s**; the rest is monomorphization collection.

⛔ **This column has produced two wrong conclusions already** — an external
review's *"surprisingly expensive frontend phase"*, and this repo's own
`rollback/domains/` hypothesis, which was declared "Refuted" on a `cargo check`
A/B that could not see 94% of the cost. **`cargo check` is not a cheap proxy for
build cost on a unit that registers a lot of generics.** Full working:
`dev/journals/compile-cost-what-actually-drives-it-2026-08-08.md`, final section.

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

⛔ **the cache state is in the COUNTERS, and `build_label` contradicts them.**
The build at `2026-08-08T11:17` is labelled `collector: dev/first-party` and has
`build_fresh_units: 0` — it recompiled all 688 units and took 539.9s, against two
honest first-party rebuilds at 187.7s and 210.4s. Any selection that trusts the
label admits a cold build's durations into a rebuild's statistics and inflates
them 2–4x with nothing in the output saying so. ⚠ **and `run_id` is not the
build**: the collector reuses one `run_id` across its cold and warm passes, so
four of the eight recorded builds share a `run_id` with another. Group by
`build_source`.

⚠ **`backfilled: true` means `lines` and `commit` describe the tree at INGEST,
not at build.** Drop those rows before regressing seconds against lines.

⚠ **`seconds` is a unit's WALL duration inside a real parallel build**, sharing 8
cores with sibling rustc processes — nightly `-Ztime-passes` reads 12.77s for
`ambition_relativity2d` where this ledger reads 68.1s. Both are real measurements
of different questions: this one prioritises ("what does it cost the build I
run"), that one diagnoses ("what does it intrinsically cost"). The ratchet weights
use this one on purpose. Do not compare them.

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
  phase to regress against `dev/ambition_dev_measurements/compile_graph.jsonl`.

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

## 4. `dev/ambition_dev_measurements/compile_cost.jsonl` — `kind: "scenario"`

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

## 5. `dev/ambition_dev_measurements/run_tests_cost.jsonl` — `kind: "job"`

**Test time.** Written by `scripts/run_tests.py`; 75 rows predate this schema.
Grain: one suite invocation, with a `per_job` array of commands.

Its vocabulary is **reused, not replaced** — it got there first and it is right:

| field | meaning |
|---|---|
| `seconds` | total wall time |
| `executed_seconds` | the part of `seconds` spent RUNNING tests, summed over the jobs whose runner REPORTED one |
| `unclassified_seconds` | wall time in jobs whose runner reported nothing — neither build nor run |
| `build_seconds` | `seconds - unclassified_seconds - executed_seconds`; the build graph of the MEASURED jobs only |
| `per_job[]` | `{job, command, ok, seconds, executed_seconds}` — `executed_seconds` is `null` for a job whose runner reported none |
| `jobs` / `passed` / `exhaustive` / `filtered` | plan and outcome |
| `finished` | float epoch — **superseded** by the envelope's `recorded_at`, kept for the 75 existing rows |

⚠ **`executed_seconds` is `null` when the runner did not report one**, and that
is different from `0`. libtest prints `finished in Xs` on stdout, which the
runner pipes; **nextest prints `Summary [ Xs ]` on stderr, which is deliberately
left attached** so cargo's progress bar keeps rendering — so a nextest job has no
duration to read. Writing `0.0` there made every such job claim it spent no time
running tests, and the derived build-vs-execution split then attributed the whole
wall clock to the build. A reader must treat `null` as unmeasured and exclude it
from that split rather than summing it as a zero. Pytest jobs are `null` for the
same reason: nothing parsed a duration out of them.

⛔⛔ **AND THE PER-JOB NULL WAS NOT ENOUGH ON ITS OWN — the aggregates undid it,
2026-08-28.** Three roads summed `executed_seconds or 0.0` (the cost-ledger row,
the status payload, the human report) and `compile_report.py` derived
`build_seconds = seconds - executed_seconds`, so an unmeasured job's ENTIRE wall
clock landed in the build column and was stated as a measurement. ⇒ the split is
three numbers now, and `build_seconds` is derived only from the jobs that
reported. ⚠ **rows written before this date carry no `unclassified_seconds` and
an `executed_seconds` that may be a zero standing in for "unknown"** — those
zeros are read as-is rather than reinterpreted, because guessing which historical
zeros were real would rewrite the series the report exists to show.

The `seconds` / `executed_seconds` split is the same idea as the unit ledger's
`frontend_seconds` / `codegen_seconds`: a total, and the split that matters. They
are **named after their own sources** rather than unified, because renaming
cargo's `frontend`/`codegen` sections away from cargo would make the ingester's
provenance unreadable.

**Added in schema 1:** the full envelope, plus `profile`, `opt_level` and
`incremental` — the suite forces `CARGO_INCREMENTAL=0` for itself while the dev
loop runs with it on, and until now nothing recorded which of the two a row was.

---

## 6. `dev/ambition_dev_measurements/carve_lineage.jsonl` — `kind: "carve"`

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
