# Consolidation campaign metrics

**Baseline snapshot:** `662a9b56096a304ce9fcfe3ebcf1177403f2452d`
**Instrument:** `python3 scripts/architecture_census.py`

> **No target value is implied.**
>
> These values show campaign progress and repository shape. They are not quality budgets.

| Metric | Baseline | Counting rule |
| --- | --- | --- |
| Workspace packages | 80 | Root `Cargo.toml` `[workspace].members`. Exact manifest count. |
| Workspace Rust files | 1,882 | `*.rs` under workspace member directories. Exact filesystem count for this snapshot. |
| Repository tracked Rust files | 1,908 | `git ls-files` entries ending in `.rs`. Exact tracked-file count. |
| Workspace raw Rust LOC | 825,759 | Physical Rust source lines under workspace member directories. |
| Workspace nonblank Rust LOC | 777,646 | Same files, excluding whitespace-only lines. |
| Test Rust LOC (heuristic) | 265,099 | Nonblank lines in `tests/`, `tests.rs`, or `*_tests.rs`. Inline `cfg(test)` modules elsewhere are not counted. |
| Large Rust modules (discovery only) | 182 | Tracked Rust files with at least 1000 nonblank lines. The threshold is a navigation filter, not a quality limit. |
| Major mechanical authority families in manual ledger | 20 | One stable `AUTH-*` ledger row per reviewed high-impact authority family. Not all 525 generated ECS resources. |
| Suspected/review-needed duplicate-authority families | 4 | Manual duplicate-family rows tagged because independent values may answer the same current fact or a supported compatibility road creates a second source. |
| Explicit App resources with narrower session/generation semantics | 32 | Unique types named by `SessionScopedResources` and `SessionOwnedCheckpointState`, plus `SessionMechanics`. Source-owned explicit list, not regex inference. |
| Construction entry roads | 9 | One `ROAD-*` ledger row per architecturally distinct construction/reconstruction entry road reviewed. |
| Publication/admission roads | 8 | One `PUB-*` ledger row per high-impact authority publication/admission boundary reviewed. |
| Mechanical editor domains | 6 | Production `MechanicalDomain::of::<T>` domains after fixture/test markers are excluded, then manually checked for protocol stages. |
| Mixed local/canonical identity types | 2 | Manual identity rows where one type currently combines local lifetime/correlation with canonical/peer-visible responsibility. |
| Local identity types feeding canonical provenance | 3 | Manual identity rows for local tokens known from source/planning to enter current peer-visible/canonical provenance paths. |
| Known transitional compatibility/architecture roads | 8 | Rows whose primary category is `transitional`. Related authority/road rows are not counted again. |
| Correctness-sensitive ordering relationships | 7 | Manual `ORDER-*` rows. Raw `.before/.after` edges are excluded unless a semantic invariant was identified. |
| Optional Res/ResMut occurrences (heuristic) | 732 | Source-text pattern outside obvious tests. Discovery only; no defect meaning. |
| Optional Res/ResMut unique type spellings (heuristic) | 196 | Unique type tail from the optional-resource source-text pattern. Discovery only. |

## Generated `.agent` navigation inventory

The commit-matched generated inventory is useful for breadth. It is not the semantic authority.
At this snapshot it reports:

- **33315** indexed Rust symbols;
- **8262** tests;
- **525** ECS resources;
- **1098** registered systems;
- **189** plugins.

These values come from `counts.ecs` in `.agent/index/catalog.json`. Use the generated catalog or `.agent` query tooling instead of hand-maintaining a second inventory.

## How to compare a later campaign

1. Run the same helper at the later commit.
2. Record the new commit and the same counting rules.
3. For manual ledger metrics, update the existing stable IDs. Do not append a second row for the same authority.
4. Explain changes by ownership outcome. Do not claim improvement only because a number decreased.
5. A valid new capability can increase counts. A consolidation can also keep counts stable while removing an independent truth.

Examples of meaningful progress are:

- `DUP-ROOM-PUBLICATION` changes from suspected duplicate authority to one owner plus projections;
- the narrower-lifetime resource count falls because state moved under an actual session owner and reset code disappeared;
- a transitional road is deleted after its replacement becomes the normal path;
- a mixed identity type is split into local correlation and canonical provenance even if the total number of types increases.
