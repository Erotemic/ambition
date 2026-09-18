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

The value **8** is intentionally retained as the `662a9b...` baseline. The later documentation cleanup at `2dbd81abc50f` closed `TRANS-PLANNING-HISTORY`; a refreshed current-snapshot metric should therefore recompute the ledger instead of editing this baseline in place.

## A SECOND READING, `531f02e22`, 2026-09-16

⭐ **THIS IS THE "later campaign" COMPARISON THIS PAGE ASKS FOR AT THE BOTTOM,
not an edit of the baseline.** Same helper, same counting rules, a later commit —
`python3 scripts/architecture_census.py`.

| Metric | Baseline `662a9b5` | This reading | Δ |
| --- | ---: | ---: | ---: |
| Workspace packages | 80 | 80 | — |
| Workspace Rust files | 1,882 | 1,889 | +7 |
| Repository tracked Rust files | 1,908 | 1,915 | +7 |
| Workspace raw Rust LOC | 825,759 | 840,947 | +15,188 |
| Workspace nonblank Rust LOC | 777,646 | 792,058 | +14,412 |
| Test Rust LOC (heuristic) | 265,099 | 272,507 | +7,408 |
| Large Rust modules (≥1000 nonblank) | 182 | 185 | +3 |
| Optional Res/ResMut occurrences | 732 | **726** | −6 |
| Optional Res/ResMut unique spellings | 196 | 196 | — |
| Mechanical editor domains | 6 | 6 | — |
| Explicit App resources with session/generation semantics | 32 | **36** | **+4** |

⛔⛤ **ONE ROW MOVED FOR A REASON AND THE REST MOVED WITH THE TREE.** The +4 is
`SessionScopedResources` going 25 → 29: `StocksMatchSettled`,
`SuddenDeathEntered`, `LiveMatchTicks` and `SessionMatchOrdinal`, which the
peer-identity campaign made MEMBERS of that grouping rather than moving them.
That is C03's starting population, and it is now guarded —
`scripts/check_session_owner_census_matches_source.py` compares a marker in
`consolidation-plan.md` against `teardown.rs` on every `--maintenance` run.

⚠ **AND THE OPTIONAL-RESOURCE ROW IS THE ONE TO READ CAREFULLY**, because it is
where a hand-written scan and this instrument disagree by 16%. 726 is FLAT
against the baseline. A scan of mine that swept all tracked `.rs` and kept inline
`#[cfg(test)]` modules reported 850 and read as growth; the census strips each
file from its first `#[cfg(test)]` and scans only `crates/`, `game/`, `tools/`.
⇒ Both numbers are real; only one answers this row. Use the helper.

⚠ Note ~51% of the added nonblank LOC since the baseline is TEST code
(+7,408 of +14,412), which is worth knowing before reading the growth as
production sprawl.

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

## 2026-09-16 static re-measurement (`09629b060f65`)

⚠ **COUNTS ONLY.** The semantic ledger items were not re-read; see the ledger's
`static_measurement_refresh`. Deltas are against the `662a9b56096a` baseline.

| metric | baseline | 2026-09-16 | delta |
| --- | ---: | ---: | ---: |
| workspace nonblank Rust LOC | 777,646 | 787,934 | +10,288 |
| workspace Rust files | 1,882 | 1,883 | +1 |
| test Rust LOC (heuristic) | 265,099 | 269,454 | +4,355 |
| large modules (≥1000 nonblank lines) | 182 | 184 | +2 |
| `Option<Res<_>>` occurrences | 732 | 726 | −6 |
| `Option<Res<_>>` unique types | 196 | 196 | 0 |
| raw `.before`/`.after` edges | 473 | 471 | −2 |
| explicit narrow-lifetime resources | 32 | 32 | 0 |
| inventory: ECS resources | 525 | 493 | −32 |
| inventory: registrations | 1,818 | 1,763 | −55 |
| inventory: unique registration identifiers | 3,245 | 3,132 | −113 |
| inventory: components | 650 | 638 | −12 |
| inventory: non-ECS items | 2,123 | 2,048 | −75 |
| inventory: plugins | 189 | 187 | −2 |
| inventory: registered systems | 1,098 | 1,108 | +10 |

⭐ The inventory bucket fell across the board while source LOC ROSE. REASONED,
not measured: fewer distinct registered truths against more code is the shape a
demolition makes. ⚠ **Post-A10 demolition CLOSED on 2026-09-16 with a mostly
NEGATIVE result** — every A10 symbol had live production callers and exactly one
`pub` item across two files had no outside caller — so this movement is NOT that
campaign's output, and reading it as such would credit a deletion that did not
happen. ⛔ It is not proof. The counts are
text-pattern heuristics over a separately generated index, and a crate moving
between directories moves its rows too. ⇒ Do not cite the −113 as a consolidation
result without naming which identifiers went.

⛔ The inventory numbers were 50 commits stale when this was measured, and
regenerating `.agent/` changed `files`/`symbols`/`tests` while leaving every
`ecs.*` count identical. So the ECS deltas above are drift since the baseline,
not an artefact of the regeneration — that was checked, not assumed.

## 2026-09-17: the duplicate-authority row, re-derived

⛔⛤ **THE BASELINE'S `4` WAS CORRECT WHEN IT WAS WRITTEN AND WAS BEING READ AS
CURRENT.** `python3 scripts/architecture_census.py` prints the manual ledger
tags, so it printed `suspect_duplicate_authority: 4` while
[`architecture-census.md`](architecture-census.md#2-duplicate-truth-families)
declared three of those four families closed — one of them on 2026-09-14. The
instrument this page names was the copy that rotted, because the tag was hand-
maintained and the prose was the only place the adjudication was written.

| metric | baseline `662a9b5` | 2026-09-17 | delta |
| --- | ---: | ---: | ---: |
| duplicate-authority families, **open** | 4 | **1** | −3 |
| duplicate-authority families, **resolved** | — | **3** | new state |
| duplicate-authority families, **legitimate separation** | 4 | 4 | 0 |
| explicit App resources with session/generation semantics | 32 | **37** | +5 |
| `Option<Res<_>>` occurrences | 732 | 731 | −1 |
| `Option<Res<_>>` unique types | 196 | 197 | +1 |
| large modules (≥1000 nonblank lines) | 182 | 187 | +5 |
| workspace nonblank Rust LOC | 777,646 | 800,844 | +23,198 |

⚠ **COUNTING RULE CHANGED, AND THAT IS HALF THE DELTA.** The open count is no
longer "how many rows carry the tag"; it is derived from one field per ledger
item, `duplicate_authority_state`, with the tag carried by exactly the
`OPEN_PRESSURE` ones and
`scripts/check_consolidation_ledger_states_are_live.py` failing the
`--maintenance` lane if the tag, the field, this page's State column or its
headline split disagree.

⛔ **THE −3 IS NOT THREE CONSOLIDATIONS LANDING TODAY, and this page's own rule 4
says not to read it as one.** `DUP-ROOM-PUBLICATION` closed with A10,
`DUP-CONTENT-CURRENT` closed when the hot reload's generation advance moved
behind `publication_succeeded` on 2026-09-14, and `DUP-SESSION-CURRENT` closed on
2026-09-16 with one deletion. What happened on 2026-09-17 is that the count
caught up with the work and grew a guard, which is worth exactly that and no
more.

⭐ The one that is left, `DUP-GENERATION-MECHANICS`, is open for a reason the
row did not state until today: the shipped composition has no second
construction source — `GenerationMechanics::for_live_session` REFUSES a
shell-routed session with no generation — and what stays reachable is the
App-registry road for compositions that have none by design. That is a product
decision, asked as `Q144`, not an unowned cleanup.

## 2026-09-17: a COUNTING-RULE change, not a tree change

⛔⛤ **`scripts/architecture_census.py` WAS CUTTING EACH FILE FROM ITS FIRST
`#[cfg(test)]` TO THE END**, and in this tree a module declares its tests near
the TOP — `#[cfg(test)] mod tests;` at `platformer2d_runtime/src/lib.rs:25` — so
every pattern metric below that line was invisible. The helper's own docstring
called the cut *"a conservative heuristic"* and labelled the counts heuristic,
which was honest and was not the same as knowing the size.

⇒ Both test-boundary questions now come from `scripts/lib/test_paths.py`: the
file rule (which also knows about `test.rs`, `test_support.rs` and the four files
whose first attribute is an inner `#![cfg(test)]`) and a per-ITEM strip that
removes inline `#[cfg(test)] mod … { }` blocks and keeps the rest of the file.

| metric | same tree, tail cut | same tree, per-item strip | Δ |
| --- | ---: | ---: | ---: |
| `Option<Res<_>>` occurrences | 731 | **820** | +89 (+12%) |
| `Option<Res<_>>` unique types | 197 | **206** | +9 |
| `#[derive(Resource)]` declarations | 499 | **538** | +39 |
| `#[derive(Component)]` declarations | 635 | **659** | +24 |
| raw `.before`/`.after` edges | 475 | **588** | +113 (+24%) |
| test Rust LOC (heuristic) | 278,198 | **279,727** | +1,529 |
| "fallback" term hits | 728 | **776** | +48 |

⚠ **NOTHING IN THE TREE MOVED BETWEEN THOSE TWO COLUMNS — both were measured at
`49bfcf4ef` within a minute of each other.** Read every earlier reading on this
page as the left-hand column's rule, and do not subtract across the change: this
page's rule 2 asks for the same counting rules, and this is the case it exists
for.

⭐ **THE ONE THAT MATTERS MOST IS NOT THE BIGGEST.** `raw .before/.after edges`
moved 24%, and it is the population C07's *"correctness-sensitive ordering
relationships"* row samples from; the optional-resource row C07 is COSTED from
moved 12%. A campaign that priced itself off either number priced itself off a
corpus with a quarter of the ordering edges missing.

⛔ The `662a9b5` baseline table above is NOT edited. It records what was measured
under the old rule, and correcting it in place would destroy the only evidence of
what the numbers meant.
