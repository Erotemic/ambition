# The binding resolution boundary

**Status:** LANDED 2026-07-25.
**Scope:** cross-layer references — how content names something, and what happens
when the thing is not there.
**Authority:** implements the capability the master plan
([`competitive-2d-platformer-engine-roadmap.md`](competitive-2d-platformer-engine-roadmap.md))
scattered across Task 7 (readiness and failure policy), Task 9 (provider-owned
animation), and Task 12 (inspectable causality). It is a prerequisite for those,
not a part of them.

---

## 1. The defect class

Ambition is full of cross-layer references authored as strings: an anim row, a
world-item sprite id, a held-item id, a brain key, a patrol path, a loading zone,
an sfx cue. Each was resolved at its USE site, by its consumer, through a
fallible lookup — and every consumer spelled the miss as a shrug.

| Reference | Old resolver | What a miss did |
|---|---|---|
| anim row | `SheetRecord::row_index_of -> Option<usize>` | `unwrap_or(0)` — drew frame 0 of row 0 |
| world item art | `HashMap<String, _>::get` | placeholder quad |
| held item art | `HashMap<String, _>::get` | placeholder quad |
| brain key | `CharacterRoster::spec_for_brain` | generic `combatant` fallback |
| patrol path | `KinematicPathSpec::matches_id` | enemy goes passive |
| loading-zone link | `zone_by_id -> Option` | door does nothing |
| sfx cue | `SfxProvider::provide_clip -> Option` | silent stub for the session |

None of these panic. None log. Each produces a game that is *not right* rather
than one that is *obviously broken*, which is much quieter and survives playtests.

This is not a hypothetical class. It is where Mary-O's unreachable death
animation came from (`death` in the sheet, `dead` in the policy), where the
invisible Sanic rings came from, where the spark blossom that was never drawn
came from, and where `neil_ongras_turfson` shipping a fully transparent sprite
sheet came from.

## 2. The rule

> A reference resolves once, at construction, into a typed handle. What does not
> resolve becomes a value someone holds — never an early `return` nobody sees.

Three types in `ambition_platformer_primitives::binding` carry it:

- **`Ref<N>`** — an authored id in namespace `N`. Deliberately inert: it has no
  lookup method, because a reference that can look itself up is one that can
  silently fail to.
- **`Resolver<N>`** — built once from the ids that exist. The only thing that
  mints a **`Bound<N>`**, which has no public constructor: holding one is proof
  that resolution happened. `Bound::slot()` is the DECLARATION position — the
  sheet's row, the manifest's entry — so a resolved reference indexes the
  authored data with no second lookup to get wrong.
- **`BindingLedger` / `BindingReport`** — resolution keeps going past a failure,
  so one pass reports every bad reference. The report is cross-namespace by
  design: a room touches rows, sprites, and cues, and a reader chasing "why is
  this room wrong" should get one list, not four warnings from four crates with
  four error types.

The report names the namespace, the id, WHO declared it, every id that WAS
available, and a did-you-mean. Suggestions cover both ways a reference goes
stale: a typo (`dead` -> `death`, by edit distance) and a rename (`stair` ->
`stair_top`, by unambiguous affix).

### What the rule is not

It is not "unresolved references are fatal". Content has typos; that is normal.
The placeholder art, the passive fallback, and the row-tinted quad all stay,
because **a blind run must never go black** — it just no longer gets to be quiet
about why it is magenta. Where a stronger rule already exists it stays the
authority: an unknown `held_item` REFUSES construction (`UnknownHeldItem`), and a
hard refusal beats a report.

## 3. Where it landed

| Namespace | Owner | Resolver source |
|---|---|---|
| `AnimRow` | `ambition_sprite_sheet::binding` | the sheet's own rows |
| `WorldItemSprite` / `HeldItemSprite` | `ambition_platformer_primitives` | the unioned provider manifests |
| `KinematicPathId`, `CharacterId`, `HeldItemId` | `ambition_actors::world::rooms::binding` | the room + the catalogs |
| `RoomId`, `LoadingZoneId` | `ambition_world::rooms::binding` | the room set |

Wired into the real paths, not offered beside them:

- `RoomFeatureConstructionPlan::prepare` sweeps **both** channels — the room's
  authored families and the spawn requests content staging hands it — and carries
  the report on the plan. The second channel is where Mary-O and Sanic keep their
  enemies, so without it the demos were the consumers the sweep could not see.
- `RoomSet::from_parts` sweeps every link endpoint before building the graph.
- The item and slash/shrine visuals resolve per frame through a
  `Local<ReportedOnce>`, so a missing id is said once rather than sixty times a
  second.

Deliberately NOT using the generic machinery: **sfx cues**. The cue set is a
compile-time constant and the resolver is `SfxProvider` itself, so `Resolver`
would buy nothing except a parry2d-transitive dependency in the leanest audio
crate in the workspace. `refresh_sfx_from_bank` returns and names its unbound
cues instead. The rule that matters does not require the type that usually
carries it.

## 4. Why this is an ENGINE capability

`game/ambition_content/src/content_validation.rs` has had this thesis written at
the top for a while — catch typos "instead of letting string ids silently fall
back or never fire". But it reads raw LDtk JSON, so it only ever served the one
game with an `.ldtk` file. Mary-O builds `RoomSpec`s in Rust, Sanic builds a
course, Outlander authors a ridge from outside the workspace: none of them got
any of it, and `push_warning` sat marked dead-code with a note that the checks
"haven't been wired into startup yet".

Sweeping the **world IR** instead of the authoring file is the whole difference.
Every provider gets it, whatever backend produced the room (ADR 0021), including
backends that do not exist yet. That is the design oracle's question answered in
the affirmative for a capability rather than a feature.

## 5. Strangled

- `SheetRecord::row_index_of` — **deleted**. There is no name→row lookup on a
  sheet any more.
- `HeldItemArt` / `WorldItemArt` as `HashMap<String, _>` — **deleted**, replaced
  by `ArtBindings` whose `get` returns `Result<_, UnresolvedRef>`.
- The two `eprintln!` room-graph warnings — **deleted**, replaced by the link
  sweep.

Enforced by two grep gates in the run's goal file, both counting to zero.

## 6. Found on the way

- `GroundItemSpec::held_item` documented itself as "skipped at spawn rather than
  erroring". False since the planned-construction campaign: it is a hard
  `UnknownHeldItem` refusal. Comment corrected.
- `cargo test -- --exact <name>` exits **0** when the filter matches zero tests,
  so a CI check naming a not-yet-written test is green from minute zero. Any goal
  file naming tests must assert a positive pass count.

## 7. Open

- `content_validation.rs` still checks dialogue ids, quest conditions, and
  encounter/boss ids at the LDtk level. The patrol-path half is now subsumed by
  the IR sweep except for its bare-`Patrol:` warning. Logged in
  `dev/journals/code_smells.md` rather than deleted — the LDtk checks catch
  things the IR does not carry, and picking that apart is its own slice.
- Namespaces not yet migrated: moveset clip bindings, music tracks, recipe ids,
  dialogue ids. Each is the same shape and should be a small slice, not a
  campaign.
