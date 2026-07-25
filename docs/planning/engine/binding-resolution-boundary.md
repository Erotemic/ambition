# The binding resolution boundary

**Status:** PARTIAL, landed 2026-07-25 and corrected the same day after an
external review. Read §8 before trusting any coverage claim here.
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

Wired into the real paths, not offered beside them:

- `RoomFeatureConstructionPlan::prepare` sweeps **both** channels — the room's
  authored families and the spawn requests content staging hands it — and carries
  the report on the plan. The second channel is where Mary-O and Sanic keep their
  enemies, so without it the demos were the consumers the sweep could not see.
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

## 8. What this is NOT — corrections from the 2026-07-25 review

An external review read the series and was right about most of it. The claims
below were in this document and are false; they are recorded rather than quietly
edited, because a plan that overstates its own coverage is the exact failure the
feature exists to prevent.

- **"Loading-zone links had no validation."** False. `RoomSet::layout_warnings()`
  already warned on missing source and target zones, and session setup and LDtk
  reload both run it. The room-link sweep this document described has been
  **deleted** — it re-derived an existing authority beside it, which is worse
  than a plainer message. Unknown ROOM ids in links keep their diagnostic in
  `from_parts`.
- **"The SFX diagnostic is production behavior."** It was not.
  `AudioLibrary::refresh_sfx_from_bank` has no caller at all, despite its own doc
  naming one. The real fix now lives in `audio_play_sfx_messages`, where a cue is
  actually requested and resolves to nothing; it records `missing_source_ids` and
  warns once per id instead of only bumping a counter.
- **"References resolve ONCE."** Not universally. Construction-time namespaces do.
  Item art and effect rows resolve during per-frame presentation sync, minting a
  `Bound` that is immediately discarded. The honest description of those paths is
  *checked lookup with structured diagnostics*, not one-time binding.
- **"`Bound<N>` is proof."** It proves the id existed in SOME resolver of that
  namespace, never that it came from THIS authority — the marker names the
  family, not the sheet. `SheetRecord::row` now checks the agreement with a real
  `assert!` (it was a `debug_assert!`, i.e. absent exactly where a wrong row is
  least visible) and a test pins it. The general fix — an authority tag on
  `Bound` — is NOT done; AnimRow is currently the only namespace whose slot-bearing
  `Bound` escapes its resolver.
- **"ONE unified report."** The type can hold several namespaces and construction
  merges two channels into one. It is not a single production result across every
  check: room construction, the per-visual `ReportedOnce` locals, and
  `layout_warnings` remain separate. An empty report means the namespaces in
  THAT sweep were clean, nothing more.
- **"Authored content declares `Ref<N>`."** Mostly it does not. Authored structs
  still hold `String`, and call sites mint a `Ref` immediately before resolving.
  The authored data model did not become typed; the lookup and its diagnostics
  did.

Also open, from the same review and not yet addressed:

- The miss path rebuilds a full diagnostic (clone available ids, compute a
  suggestion, build and sort a report) on EVERY frame for a permanently missing
  item-art id. `ReportedOnce` suppresses the log, not the work.
- `ReportedOnce` keys on (namespace, declarer, id) and not on content epoch or
  provider composition, so a defect fixed and reintroduced across a content
  reload is suppressed by the stale entry.
- `Resolver` deduplication is silently first-wins for every namespace. That
  disagrees with the room map (last-wins on duplicate ids) and hides genuinely
  ambiguous content — duplicate path aliases and duplicate sheet rows are
  probably malformed data, not something to silently pick a winner for.
- Held items are swept AND hard-refused by construction; only the refusal is
  authoritative, and the sweep's entry is redundant.
