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

### A THIRD counting-rule change, the same day — the instrument was reading prose

⛔⛤ **`ResMut<T>` IN A COMMENT IS NOT A WRITER, AND THE MULTI-WRITER CENSUS WAS
COUNTING IT.** Found while citing that census's own narrowing table to adjudicate
`SlotInteractionState`. Three of its 85 multi-writer resources were ENTIRELY an
artefact of prose — `AcceptedCheckpointRestore`, `PortalTuning`, `PortalViewer`
each had one real writer plus a paragraph saying *"it was `ResMut<PortalTuning>`
registered"* — nine more carried a phantom writer file, and the type population
held a `R` and a `_`, harvested from `Res<R>/ResMut<R>` in a doc comment and
`Option<ResMut<_>>` in a line comment. **A census parsing prose does not fail; it
invents.** The worst of it landed in the class the guard sends people to first:
`AcceptedCheckpointRestore` is rollback-registered, so the highest-priority
shortlist was pointing at a duplication that did not exist.

⚠ **AND THE RULE ALREADY EXISTED SIX TIMES.** `rules/source_reference.rs` has
stripped comments since it was written; `check_capability_ships.py`'s docstring
records that *"the Python guards did not inherit that, and one of them read its
own documentation as evidence before this was noticed"* — and then five more grew
their own copy, two of which never strip a `/* */` block at all. There is now one
owner, `scripts/lib/rust_source.py`, which the census and the
system-registration guard both import; the remaining four are routed as each
consumer's counts are measured, the same staging `test_paths.py` was collapsed
under.

| metric | prose counted | prose stripped | Δ |
| --- | ---: | ---: | ---: |
| `ResMut<T>` types | 333 | **329** | −4 |
| multi-writer resources | 85 | **82** | −3 |
| `OwnedItems` writer files | 10 | **9** | −1 |
| `PendingLifecycleCommit` writer files | 4 | **3** | −1 |
| `DeveloperRuntimeState` writer files | 7 | **5** | −2 |
| `CausalRecording` writer files | 7 | **5** | −2 |

⚠ The two right-hand columns above are *prose stripped* only; `CausalRecording`
and the `ResMut<T>` type count move again under the test-module rules below, and
the figures shown are after both.

### The census knew ONE of the two ways Bevy hands out a mutable resource

⛔⛔ **82 → 83 → 102 THE SAME DAY, AND BOTH MOVES WERE THE INSTRUMENT.** Found
while adjudicating `PendingLifecycleCommit`, whose consumers were not in its
writer list:

| what the census could not see | sites | population |
| --- | ---: | --- |
| `ResMut<\n    long::path::T,\n>` — a trailing comma before `>`, so `\s*>` cannot reach it | 10 | 82 → 83 |
| `world.resource_mut::<T>()` / `get_resource_mut::<T>()` | 90 types' worth | 83 → **102** |

⭐ **THE MISSING FILES WERE NOT A RANDOM SAMPLE IN EITHER CASE, which is why
these mattered more than their counts.** The wrapped sites are the ones a
formatter chose to wrap; wrapping follows PATH LENGTH; a long path means the type
came from another crate — which is exactly where a second authority lives. And an
exclusive-world system is what a COMMIT EXECUTOR is, so the second blind spot hid
the destructive road: `rollback_ggrs/lifecycle_commit.rs` clears
`PendingLifecycleCommit` and `RoomTransitionLoadState` and touches
`LoadCoordinator`; `session/reset/mod.rs` reaches `AmbitionGameSave`,
`AuthoredOccurrences`, `QuestRegistry`, `GameplayBanner` and
`RoomTransitionCooldown`. **The census was blind to the systems that SPEND the
state it was auditing** — 19 types had never been on the shortlist at all, 21
gained writers.

⚠ **A THIRD SHAPE IS MEASURED AND DELIBERATELY NOT COUNTED.** A `&mut T`
PARAMETER would take the shortlist to **109** and add writers to 20 more types,
but unlike the other two it is ambiguous: `fn grant(items: &mut OwnedItems, ..)`
may be a second authority or the one owner's helper, and only the call sites say
which. Counting it would put every extracted helper on a duplicate-authority
shortlist. `insert_resource` / `init_resource` are also out, and for a firmer
reason — they INSTALL rather than mutate, which is a different question with a
different right answer.

### And the lesson was already written down, in the guard next door

⛔⛤ **`check_rollback_mutators_run_in_sim.py`'S DOCSTRING CARRIED BOTH THINGS THE
CENSUS HAD TO LEARN THE HARD WAY.** On `SessionWorldMut<T>`: *"a guard keyed on
how a write is SPELLED goes blind when a refactor respells it, and the direction
is the dangerous one — it reports no offenders."* On the lifetime: *"a system
signature elides it (`ResMut<T>`), but a `#[derive(SystemParam)]` FIELD cannot
(`ResMut<'w, T>`) … and the struct bodies are where 13 rollback-registered types
were hiding."* Both were recorded on 2026-09-15. The multi-writer census
inherited neither, and paid 82 → 121 for the second of them two days later.

⇒ **THAT IS THE SECOND INSTANCE OF THE SAME FAILURE MODE IN ONE DAY.** The
comment-stripping rule was likewise recorded in `check_capability_ships.py`'s
docstring — *"the Python guards did not inherit that, and one of them read its
own documentation as evidence before this was noticed"* — while five more guards
grew their own copy. A lesson written down in one guard while its neighbour
repeats the defect is not a documentation problem; it is a missing owner. Both
rules now live in `scripts/lib/`.

### A population A10 created and the instrument did not follow

⛔⛔ **THE WRITER-SIDE INSTRUMENT WAS LOSING COVERAGE EXACTLY WHERE THE
ARCHITECTURE WAS MOVING.** A10 moved session state out of resources and onto the
session root, reached through `SessionWorldMut<T>`. None of the eight types
carrying that accessor has `#[derive(Resource)]`, so the multi-writer census was
silent about all of them by construction — while four have more than one
production writer:

| session world component | writer files |
| --- | ---: |
| `EncounterMusicRequest` | **8** |
| `RoomSet` | 2 |
| `RoomGeometry` | 2 |
| `LdtkRuntimeIndex` | 2 |

⚠ They are ratcheted as a SEPARATE population with their own baseline and their
own floor, not folded in: a resource's lifetime is the App's and a session world
component's is the SESSION's, so "two writers" answers a different question and a
session boundary reclaims the second. Folding them would make one number mean two
things.

⛔ **SO FOUR OF THIS PAGE'S RULE-2 CASES LANDED IN ONE DAY ON ONE INSTRUMENT**:
85 → 82 (prose), → 83 (trailing comma), → 102 (exclusive world), with the
test-module rules folded in. Every earlier reading of "multi-writer resources" on
this page belongs to whichever rule was live when it was taken. Do not subtract
across them.

⭐ **THE SECOND HALF: THE SHARED STRIPPER WAS MATCHING ONE SPELLING OF A TEST
MODULE OUT OF FOUR.** It looked for `#[cfg(test)]` and then `\s*mod NAME {`, and
each of the three things that can sit in that gap hid a whole test module:

| what sits between the attribute and the `mod` | sites | example |
| --- | ---: | --- |
| a comment (`\s*` cannot cross a `///`) | 2 | `abilities/src/ranged/sentry.rs:681` |
| a visibility | 1 | `persistence/src/store.rs:222` — `pub(crate) mod tests` |
| a cfg PREDICATE instead of the bare attribute | 16 | 8× `all(test, not(target_arch = "wasm32"))`, 5× `all(test, feature = "input")` |

⇒ 18 test-only modules read as production. That is how `Captured` — a type the
census's own docstring lists as multi-writer ONLY because a fixture wrote it —
acquired a second writer, and why `CausalRecording` was ratcheted at 7 writers
when two of them were fixtures (now 5). So the attribute is now EVALUATED rather
than matched, by `test_paths.cfg_requires_test`.

⛔⛔ **AND ONE OF THE NINETEEN IS DELIBERATELY STILL NOT STRIPPED.**
`#[cfg(any(test, feature = "test-support"))] pub mod test_support` at
`boss_encounter/src/clusters.rs:376` COMPILES INTO THE SHIPPED CRATE when that
feature is on — this repository has a live arm asserting `test-support` stays out
of `[dependencies]` for exactly that reason. `any` and `all` answer oppositely,
no pattern can tell them apart, and over-cutting hides shipping code from a
census whose whole job is to find shipping code. That is the one direction a test
filter must never err in.

| metric | before | after all three | Δ |
| --- | ---: | ---: | ---: |
| `#[derive(Resource)]` declarations | 538 | **535** | −3 |
| raw `.before`/`.after` edges | 588 | **579** | −9 |

⭐ **THE TWO PASSES ARE EXACTLY ADDITIVE, WHICH IS THE ONLY REASON TO TRUST
EITHER.** The comment rule alone gives 537 / 587; the cfg-and-visibility rules
alone give 536 / 580; together 535 / 579 — 1+2 and 1+8. The presence audit
(104/83/20/1 at the time), the rollback-mutator guard (518/8) and the test-static
census are byte-identical under all three, so this is not a measurement that
moves whatever it touches.

⚠ **THE PRESENCE AUDIT MOVED LATER THE SAME DAY FOR A REASON OF ITS OWN**, so do
not read `104/83/20/1` as a live figure: its `filter_sites` prefiltered with a
`git grep` whose path class `[A-Za-z_:]*` HAS NO DIGITS, so every filter written
through a qualified path containing `platformer2d` or `portal2d` was discarded
before the real parser saw it — `Has<ambition_platformer2d::characters::actor::
BodyWalletShield>` is exactly that shape. It also read one LINE at a time and
never stripped test regions. One grammar over the whole stripped source gives
**114/91/22/1**, and the two subjects it exposed are now waived with their
measurements (`PresentationOf` holds a live `Entity`; `BodyWalletShield` is
rebuilt `.before(PlayerHitResolutionSet)` while both readers run in later
`.chain()`ed phases).

⛔ So the two-column table above still reads 538 and 588, and stays that way: it
was measured at `49bfcf4ef` under that day's rule, and this page's rule 2 is that
a number carries its method and reference point. Do not subtract across these
three changes — four of the seven rows moved for a reason that has nothing to do
with the tree.
