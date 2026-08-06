# GPT 5.6 review through `32eb27a` — verbatim, then triage

*Kept verbatim per the standing rule that a review's own text is the record;
agent commentary lives in the triage section below and can be edited freely.*

> The previous review's concrete regressions were mostly repaired. The new work
> adds meaningful architecture, but the cutscene rollback implementation has two
> correctness holes, and several authority seams remain unresolved.
>
> This review covers the 81 commits after `d46a0f7`.
>
> ## Previous findings
>
> The following issues were corrected:
> * `BonkOnly` now has consistent initial collision semantics in the
>   controlled-body and generic kinematic paths.
> * Loose Mary-O form pickups can no longer downgrade a stronger form.
> * The stale Mary-O LDtk block documentation was updated.
> * The test that depended on ignored generated gauntlet PNGs was deleted.
> * The recent range is `git diff --check` clean.
> * Optical sources now obtain their history key from the entity's
>   `WorldlineTracked2d` component rather than independently repeating that string.
>
> The optical identity fix is only partial, as described below.
>
> ## 1. Mid-beat rollback destroys cutscene presentation state
>
> `ActiveCutscene` contains `runtime`, `current_dialogue`, `current_banner`,
> `camera_target`, `fade_alpha`. Its rollback snapshot encodes only `runtime`
> (`script`, `beat_index`, `elapsed`, `finished`). The other fields are reset to
> defaults during decoding. The source describes them as derived values that the
> next simulation tick republishes.
>
> That republication does not happen. `CutsceneRuntime::tick()` emits
> `BeatEntered` only when `self.elapsed == 0.0`. If rollback restores the middle
> of a timed beat, `elapsed` is nonzero. The beat-entry event is never re-emitted.
>
> Consequently, after rollback into the middle of: a banner, `current_banner` is
> absent for the rest of the beat; a camera pan, `camera_target` is absent; a
> fade, `fade_alpha` is reset; any future timed presentation beat, its projected
> state can disappear.
>
> The snapshot test verifies only that `runtime` round-trips. It does not prove
> the claimed derivation.
>
> ### Required correction
> Use one real authority. Either snapshot the complete running cutscene state, or
> implement a deterministic projection from `script + beat index + elapsed` that
> runs every simulation tick and immediately after restoration. Do not call fields
> derived unless a projector actually reconstructs them from the authoritative
> state. For banners, remaining duration is directly derivable from authored
> duration minus elapsed. Camera and fade transitions may need explicit
> beat-runtime state if their starting values cannot be reconstructed from the
> script.
>
> ## 2. Cutscene presentation also remains stale during ordinary forward playback
>
> This is visible without rollback. When dialogue advances into a camera, fade,
> wait, or flag beat, the system does not generally clear `current_dialogue`.
>
> For example: a dialogue beat sets `current_dialogue`; the participant dismisses
> it; a `CameraPan` beat begins; the handler sets `camera_target`; it does not
> clear `current_dialogue`; the dialogue overlay remains visible during the camera
> beat.
>
> The field documentation says the dialogue is cleared when the beat advances, but
> the implementation clears it only when entering a banner or when the complete
> cutscene ends. The same structure allows presentation fields from different
> beats to accumulate rather than representing the current beat.
>
> Additionally, `camera_target` and `fade_alpha` have no production consumers in
> the current tree. The cutscene overlay reads only dialogue and banner state.
> Therefore `CameraPan` and `Fade` currently advance their timers without
> producing the promised presentation.
>
> ### Required correction
> On every beat transition, derive or replace the entire presentation projection
> for the new beat rather than mutating whichever individual field that beat
> happens to use. The resulting state should make impossible combinations
> unrepresentable, such as dialogue and an unrelated camera beat both appearing
> current. Either connect camera and fade state to real presentation consumers or
> mark those beat types as unfinished rather than presenting them as operational.
>
> ## 3. Room-entry cutscene triggers are not rollback-safe
>
> `auto_trigger_room_cutscenes()` detects room changes through
> `Local<Option<String>>` and writes to `CutsceneTriggerQueue`. Neither the
> system-local room memory nor the queue is rollback state.
>
> The rollback coverage waiver says that save-game seen flags safely deduplicate
> re-fired triggers. That explanation assumes the trigger is actually re-fired.
>
> Consider: enter room B; the `Local` changes from A to B; the room cutscene is
> queued and begins; rollback restores a frame in room A; the system-local value
> remains B because Bevy system locals are not rewound; resimulation enters room B
> again; the trigger system sees `last_room == B` and emits nothing.
>
> If `ActiveCutscene` was restored to its pre-trigger state, the cutscene is now
> skipped entirely. The seen flag cannot deduplicate a re-fire that never happens.
>
> The unregistered queue has the related general problem that requests can survive
> from an abandoned future or disappear across restoration unless every producer
> is guaranteed to regenerate them deterministically.
>
> ### Required correction
> Room-transition detection that causes simulation behavior must be rollback state
> or must be derived from rollback state. Good endpoints include: a
> rollback-registered previous-room identity; an explicit rollback-registered
> room-entry sequence or generation; a deterministic room-entered simulation
> message reconstructed during resimulation. The trigger queue can remain
> transient only when every request is regenerated from authoritative state on the
> same restored timeline. Correct the coverage waiver after fixing the actual
> causal path.
>
> ## 4. Hidden blocks remain pass-through after discovery
>
> The initial invisible-block semantics are now correct: rising head from below →
> collision and bonk; falling from above → pass through; horizontal movement →
> pass through; resting support → none.
>
> After a hidden block is struck: `SpentPowerBlocks` records the hit; the renderer
> shows the spent block tile; the underlying room geometry remains
> `BlockKind::BonkOnly`. The source and LDtk documentation now explicitly
> acknowledge this.
>
> Therefore the newly visible used block still cannot support Mary-O, support
> enemies, block horizontal movement, or behave as an ordinary solid block. That
> is inconsistent with the classic hidden-block mechanic: discovery turns the
> block into a visible solid.
>
> ### Required correction
> Make discovery change the effective collision state as well as presentation. The
> Mary-O crate should own the gameplay transition. The reusable engine seam can be
> narrow: a rollback-registered block-kind override keyed by `GeoId`; or a
> rollback-owned dynamic solid placed at the authored block AABB. The same
> rollback authority that records discovery must determine both art and collision.
> Do not leave a visible solid image over permanently pass-through geometry.
>
> ## 5. Optical history still uses a display string as its actual identity
>
> The latest fix removes one independently repeated label: `OpticalSource2d` now
> reads the `WorldlineTracked2d` component attached to the same entity. That
> prevents a source label typo from selecting some unrelated history.
>
> However, the underlying authority remains `WorldlineTracked2d(pub String)` and
> `BTreeMap<String, VecDeque<WorldlineSample2d>>`. Duplicate strings are resolved
> by sorting claimants and refusing all but the first.
>
> The warning correctly says "A label is a display name, not an identity." but the
> label remains the map identity.
>
> This means: two entities cannot legitimately share a display label; one
> duplicate source silently receives no telemetry; renaming a display label
> changes its authoritative history address; ownership is selected using an
> allocator-local entity tie-break.
>
> ### Required correction
> Introduce a typed stable track identity, such as `WorldlineTrackId`, `SimId`, or
> another stable simulation identity. Key the history by that value. Retain a
> separate label solely for presentation. Requiring `WorldlineTracked2d` on an
> optical source was a useful partial repair, but it should not be described as
> completing the identity seam while the component itself remains a freeform
> display string.
>
> ## 6. Flying enemies have two authorities for whether gravity applies
>
> The new plane enemies state aerial behavior in two places: character catalog
> `body_kind = Floating`, enemy archetype `is_aerial = true`. The source
> explicitly notes that these are separate authorities over the same question.
>
> Different spawn paths consult different values: peaceful/catalog-backed
> construction derives aerial status from `CharacterBodyKind::Floating`; hostile
> `EnemySpawn` construction uses `ArchetypeSpec::is_aerial`.
>
> The same character can therefore float on one construction path and fall on
> another if the two records diverge. Repeating `true` in both places makes the
> current planes work, but it institutionalizes the disagreement rather than
> resolving it.
>
> ### Required correction
> Resolve one final body-motion classification during actor-definition assembly.
> For example: catalog body kind supplies the character default; an archetype may
> provide an explicit optional override; assembly rejects contradictory explicit
> values; every spawn path consumes the same resolved result. Do not require every
> flying character to remember two independent booleans so it behaves consistently
> across peaceful and hostile construction.
>
> ## 7. Roster topology can still arrive after the rollback session starts
>
> The session owner now runs after `InputSet::Collect`, which removes one concrete
> scheduling race. The source correctly acknowledges the remaining gap: a route
> can publish its proposed roster a frame later. In that case the local session
> still freezes topology from connected devices before the decided participant
> roster exists.
>
> Connected devices are not equivalent to participants: a keyboard seat may have
> no controller entity; a spare controller may not be participating; a CPU seat
> has no device.
>
> The attempted restart-on-mismatch repair was correctly removed because it
> destroyed seat-to-handle binding. The underlying initialization problem remains.
>
> ### Required correction
> The local session needs an explicit readiness condition: roster decided → seat
> topology frozen from that roster → session installed from that topology. A host
> with no roster can explicitly declare device-derived seating. A host that
> intends to publish a roster must prevent session installation until that roster
> is ready. Do not rely on "usually published in the same Update" as an
> initialization contract.
>
> ## 8. The new editor-art tool contains obvious duplicate code
>
> `editor_art.py` currently contains duplicated statements including
> `args = parser.parse_args(argv)` twice, and a duplicated unreachable
> `raise SystemExit` in sheet lookup. There is also duplicated comment text in the
> Mary-O block dresser.
>
> These do not appear to change current behavior, and the focused editor-art tests
> pass, but they show that the new tooling and comment-heavy edits need a basic
> cleanup pass before more behavior is layered onto them.
>
> Keep that cleanup narrow. Do not add another census or ratchet for duplicated
> lines.
>
> ## Priority
> 1. Fix the cutscene snapshot/projection model.
> 2. Make room-entry cutscene triggering rollback-safe.
> 3. Turn discovered hidden blocks into actual solids.
> 4. Replace string-keyed worldline identity.
> 5. Unify aerial body authority.
> 6. Complete roster-before-session activation.
> 7. Remove the small duplicate-code residue.
>
> Use focused validation around these seams. None of the findings requires
> repeatedly running the complete workspace suite.
>
> I could not run Rust or Cargo in this environment. The recent diff is
> whitespace-clean, and 11 focused LDtk editor-tool tests passed.

## Triage

Worked in the reviewer's own priority order, because that order is right: the
two cutscene findings are correctness, and the rest are authority seams that get
cheaper once the first two stop moving.

Verdicts are recorded here as each is reached.

- ✔ **P1 cutscene snapshot/projection — ACCEPTED and FIXED** (`91016a22e`). Both
  findings 1 and 2 were exactly as described, and one change answers both:
  `CutsceneRuntime::presentation()` is a pure function of
  `(script, beat_index, elapsed)` — the state the snapshot already carries — and
  the tick replaces the WHOLE picture from it. Dialogue surviving into a camera
  beat stopped being representable rather than stopping by convention, and the
  banner countdown is `authored − elapsed` instead of a second timer on the same
  clock.
  ⛔⛔ **writing the test the review asked for found a shipped codec bug the
  review did not.** `CutsceneScript::decode` read its optional seen flag as
  `reader.bool()?.then(|| reader.str().map(str::to_owned))?` — `bool::then`
  yields `None` when the bool is false and the trailing `?` returns it from the
  function, so *"this script has no seen flag"* decoded as *"this snapshot is
  corrupt"* and the whole cutscene was dropped on restore. **Every existing
  fixture called `.with_seen_flag(..)`**, so the false branch of the only branch
  in that codec had never once been decoded. ⭐ that is the shape to look for
  elsewhere: a suite that exercises one side of a two-sided decision.
  ✔ **the unfinished-beat half accepted too** (`78c60b0f9`): nothing in the tree
  reads `camera_target` or `fade_alpha`, so `CameraPan` and `Fade` are marked
  UNFINISHED on both the fields and the beat variants rather than removed — they
  are authored in scripts, and deleting them would make the projection lie the
  other way.
- ✔ **P2 room-entry triggers — ACCEPTED and FIXED** (`43bd33638`). The trigger is
  on the SIM schedule and remembered its room in a `Local`, which is not rewound;
  the waiver's "seen flags dedup re-fires" assumed a re-fire that could not
  happen. `ambition_cutscene::LastCutsceneRoom`, registered as
  `cutscene.last_room` (schema v14). The waiver's wrong reason is corrected IN
  PLACE rather than deleted, because the wrong version is the part worth reading.
  ⭐ that also makes the transient trigger queue legitimately transient — its
  contents are regenerated from rollback state on the restored timeline, which is
  the condition the old waiver assumed rather than met.

- ✔ **P3 discovered hidden blocks — ACCEPTED and FIXED** (`7197522ac`). No engine
  change was needed: `FeatureEcsWorldOverlay` already carries
  `removed_block_names` AND `blocks`, so the reviewer's own narrow endpoint (*"a
  rollback-owned dynamic solid placed at the authored block AABB"*) already
  existed and runs in the slot the brick removal uses. `SpentPowerBlocks` is the
  authority for art and collision both.
  ⚠ the BEFORE case is asserted too — solidifying every hidden block on sight
  passes *"it is solid after"* while deleting the mechanic.
- ✔ **P4 worldline identity — ACCEPTED and FIXED** (`a301a79a0`).
  `WorldlineTrackId` keys the history; `WorldlineTracked2d` carries it beside a
  label nothing keys on. Renaming a caption no longer moves a body's history and
  two bodies may share one.
  ⚠ **kept a `String`, against the review's `SimId` suggestion, for a measured
  reason**: TwinTrack's traveler and passband are demo-spawned WITHOUT a `SimId`,
  so keying on it would silently drop those tracks. What the finding actually
  requires is a separate value nothing draws, and that is met.
- ◐ **P5 aerial authority — ACCEPTED, ATTEMPTED, REVERTED, and the review is
  right about the defect.** The two paths do disagree exactly as described:
  `new_peaceful_npc_in` reads the catalog's `body_kind: Floating`, `new_in` (the
  hostile `EnemySpawn` path) reads `ArchetypeSpec::is_aerial`.
  ⭐ **and the attempt found WHICH character it affects, which the review did
  not: `perfect_cellular_automaton`.** Its catalog row says `Floating`, so it
  FLOATS placed as an NPC and WALKS placed as an enemy — and the duel arena
  places it as a fighter. Resolving the two with an OR made it fly on both paths
  and turned `actor_phase_split`'s two tests red, because they spawn a PCA and
  assert grounded `locomotion.x` intent. Those tests encoded the bug.
  ⛔ **so the fix is not a resolution rule, it is a DECISION**: does the Perfect
  Cellular Automaton fly when it fights? Its catalog says yes and the shipped
  duel says no, and a blind draw here changes a shipped fighter's movement.
  ▢ **the review's own stronger endpoint is what makes it decidable** —
  `ArchetypeSpec::is_aerial` must become `Option<bool>` so an archetype can say
  *grounded* rather than being unable to distinguish that from *silent*, and
  assembly can then reject a contradiction instead of picking a winner. Eight
  authored rows and the spec type. The OR alone cannot express the PCA's answer
  either way, which is why it went back.
