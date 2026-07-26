# The binding resolution boundary

**Status:** PARTIAL, landed 2026-07-25, corrected 2026-07-25 and 2026-07-26 after
two external reviews. §§1–7 describe the code as it is; §8 is the record of what
this document used to claim falsely, kept so the overstatements are visible
rather than quietly edited away. §9 is what is genuinely still open.
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
invisible Sanic rings came from, and where `neil_ongras_turfson` shipping a fully
transparent sprite sheet came from.

The spark blossom that was never drawn is **not** an instance of this class,
though this document claimed it twice. Its id was registered correctly, pointing
at a PNG no generator target produced. The id resolved; the FILE did not exist.
That is a sibling defect class — a reference into the ASSET namespace rather than
a content one — and it needs its own check, which `report_unloadable_item_art`
now is (§3).

## 2. The rule

> A reference resolves through the authority that knows, into a typed handle.
> What does not resolve becomes a value someone holds — never an early `return`
> nobody sees.

Resolution happens once where the answer is stable (room construction, startup
manifests) and per frame where the question changes per frame (which items are on
screen). Both go through the same types; the difference is whether the diagnostic
is carried on a plan or gated behind `ReportedOnce`.

Three types in `ambition_platformer_primitives::binding` carry it:

- **`Ref<N>`** — an authored id in namespace `N`. Deliberately inert: it has no
  lookup method, because a reference that can look itself up is one that can
  silently fail to. Most authored structs still hold a `String` and the sweep
  mints the `Ref` at the boundary; the lookup became typed, the data model did
  not.
- **`Resolver<N>`** — built from the ids that exist. The only thing that mints a
  **`Bound<N>`**, which has no public constructor: holding one is proof that SOME
  resolver of that namespace had the id — not that this authority did, since the
  marker names a family and two sheets are two authorities in it.
  `Bound::slot()` is the DECLARATION position — the sheet's row, the manifest's
  entry — so a resolved reference indexes the authored data with no second lookup
  to get wrong. `bind` is the allocation-free half; `explain` is the expensive
  half, called once per distinct failure.
- **`BindingLedger` / `BindingReport`** — resolution keeps going past a failure,
  so one pass reports every bad reference across every namespace that pass
  touched. One report per PASS, not one per run: construction has one,
  presentation consumers have their own, audio keeps its own vocabulary.
- **`AmbiguousRef`** — an id declared twice. It resolves (first declaration
  wins) and the second is unreachable, which is a silence of its own. Reported as
  a warning; it does not fail a binding.

The report names the namespace, the id, WHO declared it, every id that WAS
available, and a did-you-mean. Suggestions cover both ways a reference goes
stale: a typo (`dead` -> `death`, by edit distance) and a rename (`stair` ->
`stair_top`, by unambiguous affix).

### What the rule is not

It is not "unresolved references are fatal". Content has typos; that is normal.
The placeholder art, the passive fallback, and the row-tinted quad all stay,
because **a blind run must never go black** — it just no longer gets to be quiet
about why it is magenta. A non-empty report does not stop a room being published.

Where a stronger rule already exists it stays the sole authority: an unknown
`held_item` REFUSES construction (`UnknownHeldItem`), and a hard refusal beats a
report. That refusal now CARRIES an `UnresolvedRef`, so the one authority is also
the one with the did-you-mean — and the room sweep no longer checks held items at
all. Two authorities for one defect is how the softer one ends up describing the
behaviour wrongly.

## 3. Where it landed

| Namespace | Owner | Resolver source |
|---|---|---|
| `AnimRow` | `ambition_sprite_sheet::binding` | the sheet's own rows |
| `WorldItemSprite` / `HeldItemSprite` | `ambition_platformer_primitives` | the unioned provider manifests |
| `KinematicPathId`, `CharacterId` | `ambition_actors::world::rooms::binding` | the room + the catalogs |
| `HeldItemId` | `ambition_actors::construction` | the held-item registry |

Wired into the real paths, not offered beside them:

- `RoomFeatureConstructionPlan::prepare` sweeps **both** channels — the room's
  authored families and the spawn requests content staging hands it — and carries
  the report on the plan. The second channel is where Mary-O and Sanic keep their
  enemies, so without it the demos were the consumers the sweep could not see.
- The item visuals resolve per frame behind a `Local<ReportedOnce>`, which gates
  BEFORE the diagnostic is built, so a permanently missing id costs one binary
  search per frame rather than a rebuilt report. They clear the memory when the
  art resource changes, because "we already said that" about replaced content is
  a lie.
- The slash and shrine visuals resolve their rows ONCE, into a `Local` source
  cache, and log the report at fill time.
- `report_unloadable_item_art` watches the load state of every bound art handle
  and names any whose FILE failed to arrive. This is the spark-blossom class, and
  no resolver can see it: a resolver proves that content agrees with content.

Deliberately NOT using the generic machinery: **sfx cues**. The cue set is a
compile-time constant and the resolver is `SfxProvider` itself, so `Resolver`
would buy nothing except a parry2d-transitive dependency in the leanest audio
crate in the workspace. The rule is carried in that crate's own vocabulary
instead: `ProviderSfxHandleCache::handle_for` returns `Result<_, SfxSourceMiss>`
distinguishing "no bank yet" from "not in the bank" from "would not decode", and
`audio_play_sfx_messages` warns once per `(provider, cue, reason)` — naming the
cue through `ids::name_of` and the loaded banks' name sections, since an `SfxId`
is a one-way hash. The rule that matters does not require the type that usually
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
  by `ArtBindings`, whose `get` cannot return art for an id nobody registered and
  whose `explain` says why.
- `AudioLibrary::refresh_sfx_from_bank` — **deleted**. No caller, and two
  paragraphs of its own documentation disagreeing about whether it had one.
- `RoomBindings::with_held_items` — **deleted**. The construction refusal is the
  authority; see §2.

The room-graph `eprintln!` warnings are NOT deleted, and the link sweep that was
supposed to replace them never should have existed: `RoomSet::layout_warnings()`
already covered it (§8).

Enforced by two grep gates in the run's goal file, both counting to zero.

## 6. Found on the way

- `GroundItemSpec::held_item` documented itself as "skipped at spawn rather than
  erroring". False since the planned-construction campaign: it is a hard
  `UnknownHeldItem` refusal. Comment corrected.
- `cargo test -- --exact <name>` exits **0** when the filter matches zero tests,
  so a CI check naming a not-yet-written test is green from minute zero. Any goal
  file naming tests must assert a positive pass count.

## 7. Left standing on purpose

- `content_validation.rs` still checks dialogue ids, quest conditions, and
  encounter/boss ids at the LDtk level. The patrol-path half is now subsumed by
  the IR sweep except for its bare-`Patrol:` warning. Logged in
  `dev/journals/code_smells.md` rather than deleted — the LDtk checks catch
  things the IR does not carry, and picking that apart is its own slice.

## 8. What this document used to claim falsely

Two external reviews read the series and were right about most of it. The claims
below WERE in this document. They are recorded rather than quietly deleted,
because a plan that overstates its own coverage is the exact failure the feature
exists to prevent — and because the second review's sharpest point was that
leaving corrections in a footer while the body still asserted the originals makes
the body historical.

§§1–7 above have been rewritten to say what the code does. This section says what
they used to say.

- **"Loading-zone links had no validation."** False. `RoomSet::layout_warnings()`
  already warned on missing source and target zones, and session setup and LDtk
  reload both run it. The room-link sweep this document described has been
  **deleted** — it re-derived an existing authority beside it, which is worse
  than a plainer message. Unknown ROOM ids in links keep their diagnostic in
  `from_parts`.
- **"The SFX diagnostic is production behavior."** It was not.
  `AudioLibrary::refresh_sfx_from_bank` had no caller at all, despite its own doc
  naming one. It is deleted. The real fix lives in `audio_play_sfx_messages`,
  where a cue is actually requested and resolves to nothing.
- **"The SFX warning names the cue."** It printed `SfxId(0x…)`, a one-way FNV-1a
  hash — the same non-answer as the counter it replaced. `ids.rs` now declares
  through a `sfx_ids!` macro that emits the constants and a name table from one
  declaration, and open provider-local ids are named from any loaded bank's name
  section.
- **"A missing cue stays silent for this session."** Asserted for four different
  failures, one of which (a request that beat its bank's load) was routinely
  false. The miss now carries its reason.
- **"Held items are not part of the room sweep."** The test said so; production
  passed `with_held_items` anyway. Now true: the builder method is gone and the
  construction refusal carries the diagnostic.
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

- **"The spark blossom is an instance of this defect class."** It is not; see §1.
  Its id was registered and its file did not exist.

## 9. Genuinely open

- **`Bound<N>` carries no authority tag.** `sheet_b.row(&sheet_a_bound)`
  type-checks and is caught by a runtime `assert!` in `SheetRecord::row`, not by
  construction. Making it unrepresentable needs a brand on the resolver
  (a generation id stamped into `Bound`, compared on use) — cheap to add, and
  worth doing when a second namespace starts letting a slot-bearing `Bound`
  escape its resolver. AnimRow is currently the only one.
- **`ReportedOnce` is cleared on resource change, not on content epoch.** That
  covers a replaced art manifest. It does not distinguish two providers using the
  same generic declarer (`"ground item"`), so a Sanic miss can suppress an
  identical Mary-O miss within one process. The fix is a richer declarer, not a
  richer key.
- **Namespaces not yet migrated:** moveset clip bindings, music tracks, recipe
  ids, dialogue ids. Each is the same shape and should be a small slice, not a
  campaign.
- **Asset-materialization checking covers item art only.** Character sheets,
  props, and projectile art load handles the same way and get no equivalent
  report. `report_unloadable_item_art` is the pattern; generalizing it is a
  slice.
