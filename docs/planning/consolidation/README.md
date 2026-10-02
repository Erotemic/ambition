# Architecture consolidation

This directory is the architecture consolidation control plane. It answers one
question: which independent truths exist now, and which of them can become one
owner plus projections?

Read the pages in this order:

1. [`architecture-census.md`](architecture-census.md) — the current authority
   and lifetime map. It is the human owner of every row's evidence class.
2. [`consolidation-plan.md`](consolidation-plan.md) — the ranked campaigns, which
   of them are open, and the regression rules of the closed ones.
3. [`campaign-metrics.md`](campaign-metrics.md) — the current static measurements
   and the counting rules behind them.
4. [`consolidation-ledger.json`](consolidation-ledger.json) — the stable-ID machine
   ledger. Update an item instead of adding a second prose account of the same
   fact.

## Current shape in one paragraph

The duplicate-authority families that the census named are closed: 20 are
resolved and 4 are deliberate separations (the census states the derived split).
Room and session replacement is one candidate publication. Live construction
reads only the activated generation. The open consolidation work is C03
(session-owned state that is still stored as App resources), C07 (optional
authorities that a production profile requires), C08 and C09 (facade and crate
boundaries), and the AUTHORITY-POLISH lane in the queue. The plan's priority
table is the authority for campaign state.

## Evidence words

- `SOURCE_CONFIRMED`: explicit source establishes the claim.
- `SOURCE_INFERRED`: source shape supports the claim, but static inspection does
  not prove the full execution path.
- `DOC_CLAIM`: a planning or architecture document states the claim, and the
  census did not prove it from source.
- `NEEDS_COMPILED_VERIFICATION`: type checking, macro expansion, trait resolution
  or resolved features are needed.
- `NEEDS_RUNTIME_VERIFICATION`: schedule execution, rollback behaviour, two-App
  behaviour or another runtime fact is needed.

The complexity classes are `ESSENTIAL_COMPLEXITY`, `ACCIDENTAL_COMPLEXITY`,
`TRANSITIONAL_COMPLEXITY` and `UNCERTAIN`. They describe the reason for a
mechanism. They are not scores.

## What is checked mechanically

These scripts read this directory. Keep the parts they parse (row IDs, the State
column, the derived split sentence, the HTML-comment markers, the backtick name
lists).

| script | what it holds |
| --- | --- |
| `scripts/check_consolidation_ledger_still_resolves.py` | every cited path exists, every CamelCase name in a `current_truth` resolves, workspace crates match cargo, storage-kind claims match source |
| `scripts/check_consolidation_ledger_states_are_live.py` | every ledger ID is named on the census page; the page's evidence class equals the ledger's; the DUP State column, the `suspect_duplicate_authority` tag and the split sentence agree; every `blocked_by` names a live queue row or `Q` question |
| `scripts/check_session_owner_census_matches_source.py` | the `session-owner-census` marker in the plan, the census name lists and every restated count match `teardown.rs` and `checkpoint.rs` |
| `scripts/check_alias_census_agrees_with_source.py` | the `alias-census` marker in the census and the `alias-split` marker in the plan match a live count |
| `scripts/check_collapsed_authorities_stay_collapsed.py` | deleted second owners stay deleted in code |
| `scripts/check_separated_authorities_stay_separated.py` | the `LEGITIMATE_SEPARATION` families keep their separating shape |
| `scripts/check_discharged_holds_are_rewritten.py` | no `**DO NOT START BEFORE:**` line names a finished campaign |
| `scripts/check_planning_docs_survive.py` | line floors and required headings for these pages |

⚠ A green run of these checks does not say that a `current_truth` sentence is
still true. An owner can change while every path and name still resolves. Re-read
the source behind a row before you cost work from it.

## Helper instruments

- `python3 scripts/architecture_census.py` measures packages, LOC, optional
  resources, editor domains and ledger tags. Use it instead of a hand-written
  scan. It strips `#[cfg(test)]` items and comments with the shared helpers in
  `scripts/lib/`.
- `python3 scripts/multi_writer_resource_census.py` lists resources written from
  more than one production file. It is a shortlist, not a finding list.
- `python3 scripts/check_multi_writer_resources_are_adjudicated.py` ratchets that
  population and prints the types that still have no written verdict. Quote its
  output, not a copy of it.
- `python3 scripts/measure_session_scoped_resource_readers.py` counts the readers
  of each `SessionScopedResources` member. Its counts are a floor.

## Refresh rule

```bash
python3 scripts/architecture_census.py
python3 scripts/architecture_census.py --crate-table
python3 scripts/architecture_census.py --json > /tmp/architecture-census.json
```

Then re-read the source behind any semantic ledger item that changed. Do not
replace a source conclusion with a text-pattern count. A number on these pages
is a measurement with a date. Re-derive a row before you cost work from it.

## Standing rules

- No count on these pages is a limit. A count can rise when a valid capability
  is added.
- Keep one owner for each number. When a page needs a number another page owns,
  link to the owner or quote the command that produces it.
- Keep current state here. Investigation history belongs in Git.
