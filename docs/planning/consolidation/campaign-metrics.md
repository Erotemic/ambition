# Consolidation campaign metrics

**Instrument:** `python3 scripts/architecture_census.py` (add `--json` for the
full record, `--crate-table` for the package graph).
**Current reading:** 2026-10-02, at `eff03ab7ad27`.

> **No target value is implied.** These values show repository shape and
> campaign progress. They are not quality budgets.

## Current reading

| Metric | Value | Counting rule |
| --- | ---: | --- |
| Workspace packages | 88 | Root `Cargo.toml` `[workspace].members`. |
| Workspace Rust files | 2,032 | `*.rs` under workspace member directories. |
| Repository tracked Rust files | 2,058 | `git ls-files` entries ending in `.rs`. |
| Workspace raw Rust LOC | 894,419 | Physical lines under workspace member directories. |
| Workspace nonblank Rust LOC | 841,106 | The same files, without whitespace-only lines. |
| Test Rust LOC (heuristic) | 316,134 | Nonblank lines in `tests/` paths, `tests.rs` and `*_tests.rs`. Inline `#[cfg(test)]` modules elsewhere are not counted. |
| Large Rust modules (discovery only) | 191 | Tracked files with at least 1000 nonblank lines. A navigation filter, not a limit. |
| Optional `Res`/`ResMut` occurrences | 745 | Source-text pattern over production files, test items and comments stripped. Discovery only. |
| Optional `Res`/`ResMut` unique types | 200 | The type tails of the same pattern. |
| Mechanical editor domains | 7 | Production `MechanicalDomain::of::<T>` sites. The ledger has rows for six; `ExtensionModuleCode` has none yet. |
| Explicit session/generation-owned App resources | 36 | Every `ResMut` field of `SessionScopedResources` and `SessionOwnedCheckpointState`, optional fields included, plus `SessionMechanics`. |
| Duplicate-authority families | 0 open, 20 resolved, 4 separations | One `duplicate_authority_state` field per ledger item. The census page states the split. |

⚠ Two instruments count the session-owned set. `architecture_census.py` reads
36 because it includes the two `Option<ResMut<..>>` members of
`SessionScopedResources` (`BossDefeatsSinceCheckpoint`,
`BreakableRespawnSchedule`). `scripts/check_session_owner_census_matches_source.py`
counts only the required `ResMut` fields, and the plan's marker follows it.

## Manual ledger metric tags

The helper also prints the ledger's `metric_tags` counts (authority families,
construction roads, publication roads, ordering edges, transitional rows). Those
are counts of reviewed rows, not of the tree. Read them from the run.

## Generated navigation inventory

The generated `.agent` inventory is useful for breadth. It is not the semantic
authority, and it is regenerated separately from the source tree. The helper's
`agent_inventory` block names the commit it was generated from. Compare
inventory counts only with counts from the same commit.

## How to compare a later reading

1. Run the same helper at the later commit.
2. Record the commit and the date beside the new reading.
3. Do not subtract across a counting-rule change. The helper changed its rules on
   2026-09-17 and 2026-09-18 (per-item `#[cfg(test)]` stripping, then comment
   stripping), so readings before those dates are not comparable for the
   optional-resource, derive and ordering-edge rows.
4. For manual ledger metrics, update the existing stable IDs. Do not add a second
   row for the same authority.
5. Explain a change by its ownership outcome. A valid new capability can raise a
   count, and a consolidation can keep a count flat while it removes an
   independent truth.

Examples of meaningful progress:

- a narrower-lifetime resource leaves the App because its state moved under a
  real session owner and its reset was deleted;
- a transitional road is deleted after its replacement becomes the normal path;
- a mixed identity type splits into local correlation and canonical provenance,
  even if the number of types rises.
