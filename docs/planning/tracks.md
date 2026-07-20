# Tracks — current executable queue

This file is the live queue, not a completion ledger. The completed July 15–16
architecture campaign is summarized in [`status.md`](status.md); the 2026-07-19
deep-review evidence behind the newer tracks is
[`../archive/reviews/deep-review-2026-07-19.md`](../archive/reviews/deep-review-2026-07-19.md).

**Executor grades** (vision §7, restored): **[fable]** genuinely hard
design/kernel work; **[opus, fable-specced]** the spec in the named doc IS the
design — execute it verbatim and STOP at the first factual mismatch;
**[opus]** well-bounded engineering with shape + exit criteria;
**[sonnet]** mechanical with exact file lists. An agent may not deviate because
a step is hard or looks unnecessary; deviation is legitimate only when the code
contradicts the plan's factual assumptions — surface the mismatch.

**Standing fable-hard list:** the FB6 rollout redesign (track 6); the
falling-sand solver correctness pass (pooling/termination — Jon: "getting
falling sand to work right is part of the engine"); boss-fight *quality*
grammar beyond validation (boss-design.md's open iteration loop).

## ★ EXECUTION WAVE 1 (GPT-dialog keystones, 2026-07-19) — runs FIRST

The GPT 5.6 dialog converged on six keystone initiatives; framing and full
cards live in [`../reviews/fable-reply-2026-07-19-c.md`](../reviews/fable-reply-2026-07-19-c.md) §2
with mechanism corrections in [`-d`](../reviews/fable-reply-2026-07-19-d.md) —
this section is the bounded first wave, not a restatement. Vocabulary note
(Jon): feature bundles are "app configurations", never "personas".

**Immediate correctness** — [opus, fable-specced]
- ✅ Damage-multiplier semantics (`5148b4820`): incoming = difficulty ×
  assist only; slider scales outgoing. OPEN follow-up: apply the outgoing
  scale to melee through ONE attacker seam (investigate the misnamed
  `offense.damage_multiplier: i32` cluster field first — it holds melee
  base damage, not a multiplier).
- ✅ Keyboard-preset authority (`b10e45fbb`): `UserSettings.controls
  .keyboard_preset_index` is the one source; `SandboxDevState.preset_index`
  DELETED (it had no writer — the picker was a no-op).
- ✅ Portal composition + gate (`e4edd4acb`): host `portal` forwards
  `ambition_runtime/portal`; `demo_shell_smoke` 6/6 under `portal_render`;
  host un-skipped in the runner.
- ✅ Assist semantics — Jon decided 2026-07-19: **honest rename**. The
  halving stays; the UI now says "Damage assist — take half damage".
  Aim/traversal assists, if ever built, get their own settings.
- ◐ Audio replay-echo suppression + writer seam (`010c84369`) — **NOT
  confirmed-frame quarantine; corrected after review.** What landed: a guard
  at `SfxWriter` (the sole `OwnedSfxMessage` producer, so it covers every
  present and future emitter), `ambition_sfx` staying sim-blind with the host
  publishing `SfxEmissionGate` into it, and `SimulationReplayState` made
  frame-precise via a per-advance high-water mark — the old flag stayed
  raised through the new frame at the end of a rollback, so gating audio on
  it would have silenced the frame the player just caused (that also fixed a
  latent `gameplay_trace` hole). Both halves poison-tested.
  **The gate answers "this frame ran before", which is NOT "this frame is
  confirmed."** Under predicted remote input they diverge: the predicted pass
  emits sound A and it reaches the speakers, the correction rolls back, and
  the gate then suppresses the corrected sound B — phantom kept, correction
  lost. So it fixes today's local rollback echo and must NOT be copied as the
  final shape for VFX or anything else.
  **Track 1 stays OPEN and needs a different mechanism**: frame-stamped
  effect intents buffered to the host's confirmed boundary with abandoned
  predictions discarded; plus VFX, the autosave writer, and the end-to-end
  "sync-test rewind emits each effect exactly once" oracle.
  Related open question: audio and the forensic trace should NOT be forced to
  share one policy. A trace may legitimately want to keep predicted history,
  or key rows by GGRS frame so corrected state replaces them — it has no
  frame key today, so it cannot.
- ✅ **FS2/FS3 sand slice** (`574550a6d`, Jon-directed 2026-07-20; ruling +
  full card in `falling-sand.md` §4). The one-CA-step experiment resolved
  REPLACE from `bevy_falling_sand`'s source (private PostUpdate systems, a
  step signal that fires twice, DirtyAdvance starvation, parallel+RNG core).
  Sand now runs on a bespoke deterministic grid in
  `ambition_content::falling_sand_sim` (UNGATED; proofs run in every content
  test): one solver step per ordinary sim tick, conservation
  `loose + settled == emitted` asserted every tick, fixed-point settling
  proved, FS3 atomic transfer into a persistent `SettledSandLedger` that owns
  collision (kills the transient flicker), authored-room regression green in
  2.9s. ⛔ The falling-sand room is **not a netcode acceptance surface**:
  water/oil are SHELVED on the frame-driven external crate, and the bespoke
  sand grid/ledger are not rollback snapshots (the authoritative-pass gate
  stops duplicate stepping; it does not reconstruct historical material
  state). Per Jon's 2026-07-20 hard blocker in `falling-sand.md`, the unblock
  is an explicit rewrite/fork decision — no further correctness work on the
  bfs path until Jon calls it. The vestigial bfs-side sand plumbing dies with
  that rewrite.
- ✅ **PARTICIPANT-CENTERED INPUT, startup/launcher vertical slice**
  (GPT 5.6-directed 2026-07-20, fable; live doc =
  `docs/planning/engine/participant-input.md`). Four commits on main:
  `e7cc2be14` the persistent `InputParticipant` owns ActionState/InputMap
  (never on actors; `attach_player_input_components` deleted) + explicit
  `ContextClaim`/`ActiveInputContext` contexts declared by their owning
  surfaces + the `InputSet` pipeline
  Collect→ResolveActions→ResolveContext→Route→PublishCues→Consume;
  `2296ce6f6` the shell reads NO raw devices (semantic `MenuControlFrame`
  only, always live from the participant), vanity cards are tap-anywhere
  through the same semantic command as confirm, and the open
  `UiCue`/`ActiveUiCues` vocabulary replaces `MenuConfirmPrompt` (launcher
  cues "Play"/exit label, cards cue "Continue", inventory cues Equip/Use —
  one `ControlPrompt` writer per frame, decided by the resolved context);
  `fc37545b2` touch is a VIRTUAL DEVICE (leafwing input kinds over
  `MobileTouchState`, bound in the participant's InputMap; both folds and
  every GameMode routing branch in touch deleted; declared double-bindings
  replace the secret Jump-as-confirm); plus the assembled
  `app_it::participant_input` acceptance (no-actor startup/launcher, source
  ownership across sessions, held-edge transition safety, three-device raw
  screen-axis parity — it caught a real Update-schedule cycle before launch).
  Reference-frame seam untouched by construction (axes stay raw ScreenAxes
  until `AccelerationFrame::resolve_control`). NOT in slice: rebinding UX
  (P1/P5 stand), dialogue/pause/vehicle contexts, multi-participant frames,
  loading-context migration (its retry keeps a local raw read). The complete
  forward architecture and executable PA1–PA7 migration now live in
  [`engine/participant-action-system.md`](engine/participant-action-system.md).

**Keystone slices**
- ✅ **K1a movement tuning** — exit criterion MET. `ae::ActiveMovementTuning`
  is the neutral authority every sim system reads (damage, actor update,
  gravity resolve, player tick, room flow, session setup/reset, the provider
  session builder, both demos, the host smoke fixture);
  `EditableMovementTuning` is now only an inspector mirror pushed through
  `apply_editable_movement_tuning` in `DevEditApplySet`. `ambition_engine_core`
  promoted dev-dep → dep in `ambition_platformer_provider` (no new graph edge;
  actors already pulled it in). Remaining `EditableMovementTuning` references
  are editor paths only (inspector registration, settings/kaleidoscope
  writers, seeding, test fixtures) — verify with
  `rg EditableMovementTuning -g '*.rs' | grep -v ambition_dev_tools`.
  A live-edit test pins that F3 still reaches the sim; an adapter bug it
  caught (Bevy counts insertion as a change, so the mirror's defaults
  stomped authored tuning on frame one) is fixed and poison-tested.
  **LATER K1 completion** (unchanged, NOT done): deleting the
  `ambition_dev_tools` dep from actors/runtime still needs `SandboxDevState`,
  `EditableAbilitySet`, the schedule sets, and profiling hooks evicted.
- ▢ **K2a world-manifest parameterization** [opus]: preparation owns a
  `WorldManifest`; the ~13 reader sites (incl. plugin-build + pre-App
  paths — a singleton `Res` canNOT serve them) take `&WorldManifest`;
  DELETE the `OnceLock`, installer, and baked-in test fixture. Oracle: two
  providers prepare different manifests in one process.
- ▢ **K2b direct-entry activation** [opus]: route direct entry through the
  EXISTING `activate_prepared_platformer_sessions` /
  `PlatformerSessionBuilder` (no neighboring API). Oracle: the hand-built
  `SessionRoot` at `app/resources.rs:301` is deleted.

**Bounded hygiene** — [sonnet unless noted]
- ▢ Sequester the rollback inventory smoke → `tests/ambition_agent_guardrails/`
  (shape: fable-reply-2026-07-19-b.md §4; widen population by the static
  `BodyKinematics` filter, rename `rollback_inventory_smoke`, honest
  docstring). Runs only in the full gate by construction.
- ▢ Kill the vacuous projectile-anchor `.all()` (`desync_canary.rs:142`)
  + ONE strong mutable-state rewind canary. No scenario matrix — the old
  Track-0 exit list is opportunity, not contract.
- ▢ Base-SHA/overlap landing rule into existing agent instructions (doc
  only; a script waits for a second incident).
- ▢ The ONE deletion-heavy docs pass (tracks→open cards; smells refiled
  both directions; AGENTS ONE-BODY map extracted; archive provenance
  writer+validator; `run_source_analysis.sh`; reviews README) — then STOP.

**Design-before-code**
- ▢ Cutscene authority (model 1): write the semantic-playback state shape
  (beat index, deterministic elapsed, advance/skip edges through
  participant input) vs derived presentation FIRST; hold-to-skip stays a
  local accumulator, only the completed edge crosses. Frame-mode transport:
  Option A (modes ride `ControlFrame`). Then implement.

## 0. Pay down the GGRS correctness debt — **LARGELY LANDED 2026-07-19**

Spec: deep-review §2. Landed:

- registered the unregistered sim state: `WornEquipment`, `SwitchOn`,
  `SwitchFeature`, `SwitchActivationQueue`, breakable/hazard/respawn/stand
  timers, portal-gun runtime (`PortalTransitCooldown`/`PortalEmission`/
  `PortalShot`/`PortalGun`), and `RoomVisual`;
- `MovePlayback.live_boxes` is now VALIDATED against the world every tick, so a
  cloned cache slot naming a dead entity is dropped and the window respawns its
  volume. Mechanism-agnostic: it fixes the GGRS clone case and any future path
  that despawns a volume out from under a playback;
- `possession_trigger_system`'s `Local` hold/edge state moved onto the
  registered `PossessionState`;
- target selection no longer compares raw `Entity` (candidates are sorted by
  `SimId`, ties go to canonical order); slot requests are sorted by `actor_id`
  before `assign_slots`;
- **the coverage forcing function exists**:
  `game/ambition_app/tests/rollback_coverage.rs` boots the real sim and asserts
  every component ON a simulated entity is registered, derived, or waived with a
  reason. It is computed, not a checked-in ledger, and it found the last two
  gaps (`SwitchFeature`, `RoomVisual`) on its first run. NOTE it checks entity
  COMPOSITION, not system access — Bevy 0.18 does not expose per-system
  `FilteredAccessSet` publicly — so **resources still need review by hand**.

Remaining:

- the demo-content state composed into the app shell (`BallDash`,
  `BallDashInput`, `SanicActState`, `MaryOLevelState`, `FlagSequence`) — these
  live in `game/` crates and need the content-side registration seam;
- `FactionRelations`/`FriendlyFire` are unregistered and latent-safe (only
  `Default` writes today); register them when anything mutates them in-session;
- the exit oracle below.

**Exit:** a sync-test run that lands a melee hit, spends armor, flips a switch,
and breaks a brick across a forced rollback window stays checksum-identical.

## 1. Quarantine external effects to confirmed GGRS frames — [opus, fable-specced]

The exposure map is precise (deep-review §3): ~20 sim-side SFX emit sites,
all five VFX request families, and `autosave_sandbox_save` can observe predicted
or replayed state. The landed `SfxEmissionGate` and
`simulation_pass_is_authoritative` answer only **whether this frame number ran
before**. They suppress duplicate append/playback during historical replay, but
they are not a confirmed-frame boundary: a predicted effect may escape and the
corrected effect may then be suppressed.

Do not copy the trace/audio high-water pattern as the final mechanism.
`gameplay_trace` needs its own explicit policy: either record only confirmed
frames, or key rows by GGRS frame and replace predicted rows with corrected
state. Audio/VFX/persistence need a frame-stamped pending-intent journal that
releases only through the host-confirmed boundary.

- classify audio, VFX, save writes, trace rows, and host I/O by required policy;
- buffer external-effect intents by GGRS frame and session/context identity;
- release accepted intents exactly once through the confirmed boundary;
- discard abandoned predictions and invalidate pending intents on session
  replacement;
- gate autosave/settings persistence on confirmed state, not raw change
  detection;
- prove a real predicted-A/corrected-B rewind never emits A and emits B once;
- separately prove the chosen forensic-trace replacement/confirmation policy.

**Exit:** repeated rollback cannot duplicate an external effect, and a Matchbox
transport can be attached without changing simulation systems.

## 2. Build-graph hygiene (compile-time wins) — **LANDED 2026-07-19**

Deep-review §6. Landed:

- `ambition_menu` and `ambition_menu_kaleidoscope` now declare
  `default-features = false` with minimal feature sets (`bevy_ui` +
  `bevy_picking`; `bevy_pbr` + `bevy_ui`). **Measured result:** the
  `ambition_actors` build graph dropped `bevy_pbr`, `bevy_gltf`,
  `gltf_animation`, `bevy_audio`+`vorbis`, `mesh_picking`, `smaa_luts`,
  `tonemapping_luts`, `ktx2`, `sysinfo_plugin`, and `bevy_light` — the whole 3D
  stack that plain `bevy = "0.18.1"` was pushing into every build via feature
  unification, headless and CI included;
- `ambition_menu_kaleidoscope` is optional on the app, wired to the existing
  `kaleidoscope_menu` feature (its module is now cfg-gated to match), so
  bevy_lunex + bevy_rich_text3d leave non-cube builds entirely;
- `[workspace.dependencies]` adopted for serde/ron/thiserror across 30
  manifests, ending the ron 0.11-vs-0.12 split in our own tree. **Honest
  result:** the duplicate COMPILES remain, because ron 0.12 comes from
  `bevy_animation` and thiserror 1 from `bevy_ecs_ldtk` — transitive, not ours;
- deleted the dead `ambition_world → ambition_time` edge and made actors→ui_nav
  an optional feature-conduit. **Correction:** `sprite_sheet → interaction` is
  NOT dead (8 real path references) — the deep review was wrong there;
- deleted the vestigial `rl_sim` feature chain (actors' and the facade's copies
  gated nothing once the RL surface moved to `ambition_sim_harness`; the app's
  switch is the real one). `headless` stays: it is an intentional empty
  composite whose value is what it leaves off;
- **the trim exposed three crates that only ever compiled by accident**, because
  the untrimmed dep was donating features workspace-wide:
  `ambition_platformer_primitives` (needs `bevy_input_focus` for `KeyCode`),
  `ambition_game_shell` and `ambition_load_presentation` (need a windowing
  backend for the winit that `ui_api` pulls). All three now declare what they
  use — see `dev/journals/lessons_learned.md` (2026-07-19) for the pattern to
  expect on the next trim.

Remaining (own pass): `ui_api` is a bundle that pulls `bevy_animation` (unused
here) into ~6 crates; replacing it with explicit feature lists would drop that
plus ron 0.12. And `bevy` itself is still pinned in ~46 manifests — a
workspace-dependency conversion is mechanical but wants its own review, since
the per-crate feature sets legitimately differ.

## 3. Close Super Mary-O level 1 — [opus]

Engine seams proven (pickups/equip, grown form, ranged powerup, bricks, crony
stomp, flag, clock, tally, cyclic restart). The remaining customer work is the
list in [`demos/super-mary-o.md`](demos/super-mary-o.md) — **that doc is the
single source; do not copy the list back here** (it had already drifted: this
queue had dropped "enters the secret" from the scripted-run gate and omitted the
post-acceptance levels bullet).

**Exit:** visible and headless customers use the same provider, body, item, and
level state with no Mary-O-only engine path.

## 4. Close one complete Sanic act — [opus]

Corrected 2026-07-19: the ring economy (35 authored rings, collect SFX) and the
badnik enemy loop (stomp-with-bounce AND roll-through defeat through shared
contact/combat) are **landed** — the old "bits" and "one enemy loop" bullets
are done. The remaining work is the list in [`demos/sanic.md`](demos/sanic.md),
which already declares itself the single source — **refer, never copy** (these
are the copies that "drifted independently — again").

Do not absorb movement/contact work owned by another active campaign.

## 5. Build the provenance + three-origin `ConstructionPlan` vertical slice — [opus, fable-specced]

Unchanged (verified genuinely open: no `SpawnOrigin`/`RecipeId` at HEAD). Note
the room-lifecycle planner half already landed (`RoomConstructionPlan` is one
artifact for startup/reset/transition/reload/reconstruction) — build on it, do
not re-plan it. Spec: `engine/immutable-content-and-transactional-construction.md`
Phase 3:

- add explicit `SpawnOrigin` and internal stable `RecipeId`;
- plan one authored placement, one provider-staged actor, and one
  runtime-dynamic family through a common pure `ConstructionPlan`;
- validate identities and relationships before mutation;
- use the same recipes for ordinary spawn and reconstruction;
- remove `SimId` parsing as provenance authority for the selected family;
- prove deterministic plan dumps and planned-versus-committed roster parity.

**Exit:** failed planning leaves the active world untouched; all three origins
share one inspectable planner/executor; the runtime-dynamic family reconstructs
without inferring its recipe from an id string.

## 6. Correct the fighter-rollout design before FB6 — [fable]

Unchanged, and now urgent-if-touched: FB6 as written depends on the DELETED
snapshot engine (`snapshot.take/restore`) and is unimplementable at HEAD.

- prefer a fixed work budget over a wall-clock cutoff (or make decisions
  recorded external inputs);
- rollouts need a hypothetical state reconstructed solely from allowed
  `Perceived` facts, or a deliberately limited perceived-state forward model —
  now necessarily built on GGRS-era machinery.

**Exit:** the determinism and no-cheat contracts are explicit enough that an L3
implementation cannot accidentally violate either one.

## 7. Role evictions from the sim heart — [opus]

Deep-review §6 (all carve-doctrine-safe; the settled "no size-driven carve"
ruling stands — these are ROLE moves, each with a named destination):

- move `ambition_actors/src/menu/` (product Map-tab/settings-IR content) to the
  game side; drop actors' menu/settings_menu edges;
- invert `affordances/` behind a sim_view-style read model (the `ControlPrompt`
  precedent — decomposition.md names it the preferred direction); this removes
  touch_input's largest reason to name actors;
- migrate the `character_sprites/` anim pick-ladder toward
  sim_view/sprite_sheet (its own doc says lower authorities live there);
- ~~delete the compat facades~~ **PARTIALLY DONE 2026-07-19**: `effects/mod.rs`
  (never even declared in `lib.rs`) and `debug_label.rs` (zero consumers) are
  gone. `host/` is NOT dead — actors' own settings model consumes
  `crate::host::windowing`; the deep review was wrong there. The ~73
  `pub use ambition_*` lines remain [sonnet-able, needs consumer repointing];
- execute the already-ruled content evictions still outstanding (M23 / recon
  accepted #1): `ambition_items::Item` closed enum → provider-registered
  catalog; `deep_dream_strength` → content-owned presentation knob;
  `puppy_slug_gun.rs` → parameterized summon-ally ability + content data.

**Exit:** actors' out-degree drops (no menu/settings_menu/ui_nav edges); the
oracle "add a character/item without editing core" holds for items.

## 8. Combat unification batch — [opus] (first item LANDED 2026-07-19)

Deep-review §5; BIFURCATION entries in code_smells.md 2026-07-19:

- ~~collapse the projectile player/actor victim loops~~ **DONE**: one victim
  loop over every body, `Has<PlayerEntity>` picking only payload policy. Killed
  three drifts with the fork — actors now receive knockback, the player side
  gained the grudge term, and vulnerability became feedback-only for both (§A2).
  The vulnerability cluster is deliberately `Option` so simple feature bodies
  are not silently dropped from the query;
- build ONE victim-side hit/death feedback seam keyed on the attack/volume
  spec + the victim's feel profile; delete the two `is_player` attacker-side
  emit blocks — this is also where Jon's "each attack binds its own VFX/SFX"
  lands (authored effect identity on the volume/move spec). **Surveyed
  2026-07-19, still OPEN; the design is now PRE-SOLVED — execute
  [`engine/combat-model.md`](engine/combat-model.md) §8 CM8, do not re-derive
  it.** Headlines: the gap is wider than "two emit blocks" (no effect identity
  crosses `Hitbox`→`HitEvent` at all, so ALL move audio/visual fires at a
  TIMESTAMP, never on CONTACT); there are THREE payload forks, not two, one of
  which is a live bug (enemy-vs-enemy contact plays `PLAYER_DAMAGE` + the red
  "player got hurt" burst); and a plain delete of the attacker-side blocks
  would REGRESS player feel, because the rich payload exists only there;
- ~~fix the portal gun-visuals `BodyKinematics` read-model leak~~ **DONE
  2026-07-19**, and it was hiding a real bug. `ambition_portal_presentation`
  now reads a host-published `PortalBodyView` (pos/size/facing) on two host
  seams — `PortalSceneBody` (whose sprite decomposes) and the new
  `PortalAffordanceBody` (who operates the portals). The crate no longer names
  `BodyKinematics`, `PlayerEntity`, or `PrimaryPlayer` **at all**.
  **The bug:** the affordance body is tagged from `ControlledSubject`, not
  `PrimaryPlayer` — so while possessing, the held gun and the disorientation
  indicator followed the HOME AVATAR while the fire adapter already resolved
  the shot from the controlled body holding the gun (its own test:
  `portal_fire_origin_comes_from_the_holding_controlled_body`, explicitly "no
  fallback" to the primary). The visual and the mechanic disagreed; they now
  agree. Pinned by two host tests, the untag half poison-verified.
  **Deviation surfaced** (per the executor rule): the deep review said "pose
  views exist" — use them. They do, but `BodyPoseView` lives in
  `ambition_sim_view`, which depends on `ambition_actors`; consuming it would
  add an upward edge the presentation crate's manifest explicitly forbids
  ("never a host crate"). Fixed with the crate's OWN host-seam idiom instead
  (the same shape as `PortalCameraContinuityHostView`).

**Exit:** no `is_player` branch selects an effect payload anywhere in combat;
a moveset volume can author its own strike/hit effects and the goblin swipe
sounds different from the sword.

## 9. Player-facing repairs (Jon's fix list) — [opus]

From `untracked/jonnotes-FIXES.md`, verified state in deep-review §8:

- ~~room reset must consult `RespawnPolicy`~~ **DONE 2026-07-19**: a room reset
  revives a corpse only under `OnRoomReenter`/`InPlace`; `DeadStaysDead`/`OnRest`
  corpses stay dead instead of being briefly alive for the rest of the frame.
  Pinned by `integration/respawn_policy_tests.rs`, poison-verified. Together with
  the earlier placement-pin fix this closes Jon's "NPCs seem to infinitely
  respawn";
- morph-ball (and transform-mode generally): worn presentation follows the
  body's active mode — design it as the general transform/worn-identity rule,
  not a morph-ball special case;
- shrine + glider sprite repair (shrine mechanic itself is still a stub);
- kernel-guide NPC: peaceful-state patrol around a home base (authored brain
  policy, existing vocabulary);
- possession-aware dialog: speaker/listener identity derives from the actors
  in the conversation, not "the player" (dialog already has stable identity;
  model listener-side adaptation);
- `AMBITION_START_CHARACTER=sanic`: trace why the persona grants
  blink/fireballs and loses move/jump in the full app — per-character
  ActionScheme data + host input hookups; fix as data/seams, not special cases.

**Exit:** each item demonstrated in the real app (feel ships blind where
visual; behavior verified headless where steppable).

## 10. Gameplay presentation profiles — **LANDED 2026-07-20**

Design of record:
[`triage/gameplay-presentation-profiles.md`](triage/gameplay-presentation-profiles.md)
(promoted and implemented 2026-07-20 — `077d3108a`, `5ac381d72`. All nine
promotion questions are resolved against source in its "Resolved questions"
section, and its "Implementation status" section records what landed, the three
things the implementation learned, and the four items deliberately left out).
GP1–GP5 are all ✅, plus a review-driven correction pass (`8a545077b`,
`77c788c2a`, `ce283d6bf`, `54892bb26`) that repaired the schedule handoff for
fixed-tick and GGRS hosts, made reserved surrounds real control placement
regions with an explicit fallback ladder, derived screen occlusion from
computed UI layout, and completed the canonical occlusion ordering key. What
remains is listed in the design doc as scoped follow-ups, not as unfinished
cards: the platform safe-area bridge (nothing exposes insets yet), overlap
fallback steps 2–4 (need a device to tune against), a participant-facing layout
preference (gated on product testing), authored surround art, and a
non-compactible movement stick (its art is owned by `virtual_joystick`).

Landscape phones are much wider than the gameplay composition, so virtual
controls cover the controlled actor. One subsystem, four independent policy
axes (viewport / framing / screen occupancy / activation), configured per
provider. **No engine branch may select behavior by game name.**

- ✅ **GP1** — pure policies + layout resolver in
  `ambition_platformer_primitives::gameplay_presentation`: fixed-aspect
  fitting, safe-region ∩ occlusion composition, three presets, no runtime
  camera change. Tested over 4:3 / 16:9 / 16:10 / 19.5:9 / 20:9 plus
  asymmetric safe-area insets;
- ✅ **GP2** — fixed-aspect runtime slice: host resolves one
  `ResolvedGameplayPresentation`, applies `Camera.viewport` to `MainCamera`,
  keeps `FrontHudCamera` full-screen, and feeds `CameraViewport` from the
  gameplay rect instead of the window. Proves `fixed_four_by_three()` for
  Super Mary O;
- ✅ **GP3** — soft subject framing: one new pure input (`CameraScreenFraming`)
  turns the normalized safe region into a per-axis deadzone on the camera
  target, *before* the existing room/zone clamp. Proves
  `high_speed_full_bleed()` for Sanic;
- ✅ **GP4** — occupancy-aware framing: `ScreenOccluder` (content-free, anchored
  like the existing `TouchExclusionZone`), published by the touch controls,
  composed into the safe region. Proves `adaptive_platformer()` — normal
  desktop framing, occlusion-aware only when touch-primary;
- ✅ **GP5** — named surround/HUD/control regions + Mary O surround
  presentation. Turned out to be REQUIRED rather than polish: a
  viewport-clipped camera never clears outside itself.

**Guardrails:** presentation only — profile selection must never change
simulation results, and camera composition must not flip on the last input
device (glyphs may, framing may not). The provider declaration is a field on
`PlatformerExperienceAuthoring`, *not* a neighboring registration API;
`ambition_demo_pocket` stays undeclared on purpose.

**Exit (met):** 16 pure resolver tests over the required aspect matrix, 11 host
runtime tests, 6 camera-deadzone tests, 6 real-provider pins in `app_it`, 3
surround tests, 2 touch-occupancy tests; `cargo test --workspace` fully green.
Oracle 4 has no subject in this codebase (there is no gameplay pointer→world
conversion) — see the design doc.

## Parallel maintenance — [sonnet unless noted]

Small non-blocking work when it does not collide with the campaigns:

- finish the bounded boss animator fold [opus]: converge `BossAnim`/boss frame
  projection toward the shared `CharacterAnim` vocabulary and retire obsolete
  `target_pos`-style mirrors where still live. Do **not** reopen boss body
  integration (`integrate_boss_bodies` already delegates to the canonical
  body kernel);

- planning-doc repairs queued by the deep review: rewrite
  `engine/room-transition-loading.md` to current-architecture shape (Phases 1–4
  landed; keep Phase 6 performance closure) [opus]; reframe
  `engine/immutable-content-…md` §7.5–7.6 + Phase 5 under ADR 0027 [opus];
  fold `engine/boss-system.md`'s surviving rules into `boss-design.md` and
  archive it; compress `engine/encounter-orchestration.md` to the durable
  model; archive `engine/shell-vanity-sequence.md` once VC5 lands;
- ~~repoint `headless-verification.md` at `ambition_sim_harness`~~ **DONE
  2026-07-19**, and it needed more than a repoint — every path in it was
  re-verified against HEAD. Four corrections: the harness surface is
  `crates/ambition_sim_harness/` (but `ambition_app/src/rl_sim/` is NOT gone —
  it survives as the thin Ambition binding; only its `runtime.rs` went, and the
  first draft of this repair got that wrong until the path was checked); the
  app's integration tests are ONE `app_it` target with `autotests = false` and
  50 module files, not `tests/*` targets; the binaries live under `game/`; and
  **"The horizon" had already landed** — `capture_scene` renders state→PNG
  through the real presentation plugins, so headless visual spot-checks are
  available now and blind visual work should ship an image;
- ~~single-source the demo remaining-lists in `demos/*.md`~~ **DONE 2026-07-19**:
  tracks #3/#4 and the two `status.md` rows now POINT at
  `demos/super-mary-o.md` / `demos/sanic.md` instead of restating them. The
  copies had already drifted (this queue's Mary-O copy had lost "enters the
  secret" from the scripted-run gate and omitted the post-acceptance levels
  bullet);
- **KB linter: 6 of 7 failures fixed 2026-07-19** (the four mechanically
  recomputed evidence markers the 07-18 rewrite dropped are restored with real
  values, `docs/concepts/invariants.md` gained frontmatter, AGENTS.md is back
  under the line cap). ONE remains, deliberately left red rather than papered
  over: 17 files carry inline `#[cfg(test)]` modules ≥200 lines, which
  `docs/concepts/test-placement.md` says belong in an adjacent `src/foo/tests.rs`.
  Fixing it means either performing those moves or reviewing each and recording
  an accepted-inline marker — both are per-file judgement, not bookkeeping.
  Files: `ambition_actors/src/{action_scheme.rs, features/ecs/autonomous_reconcile.rs}`,
  `ambition_audio/src/catalog.rs`, `ambition_characters/src/{action_scheme.rs,
  actor/character_catalog/{binding.rs, mod.rs}, equipment.rs}`,
  `ambition_encounter/src/{lifecycle.rs, waves.rs}`,
  `ambition_ldtk_map/src/conversion/mod.rs`,
  `ambition_platformer_provider/src/lifecycle.rs`,
  `ambition_sim_view/src/control_prompt.rs`,
  `ambition_touch_input/src/bevy_plugin.rs`,
  `ambition_content/src/presentation/dialog.rs`,
  `ambition_demo_mary_o/src/{flag.rs, lib.rs, powerups.rs}`.
  Watch the `use super::*` depth when moving — an adjacent child module keeps it,
  a nested one does not (see `integration/dash_tests.rs`'s header);
- ~~backfill the 8 unindexed `dev/journals/` entries into
  `dev/journals/index.md`~~ **ALREADY DONE — the bullet was stale on arrival**:
  the same 2026-07-19 deep-review commit that queued this also backfilled the
  index (+15 link rows; its own message says "journals index backfilled").
  Recomputed at HEAD: all 31 journal files are linked from `index.md`, 0
  unindexed;
- ui_nav adoption in `ambition_menu`/`ambition_settings_menu`; the shared
  input-suspended gate for cutscene/encounter [opus];
- one structurally complete content eviction at a time when a real named
  family remains in a reusable crate;
- ~~add `tree_sitter` + `tree_sitter_rust` to `run_developer_setup.sh`~~ **DONE
  2026-07-19**: setup now provisions the repo-root `.venv` that `scripts/*.py`
  use, so `scripts/ecs_inventory.py` regenerates on a fresh clone instead of
  dying on `ModuleNotFoundError` and silently leaving the committed navigation
  packets stale.

## Standing execution rule

Use Rust types, ownership, crate direction, visibility, and ordinary behavioral
acceptance tests before adding policy/scanner machinery. Historical journals stay
historical. Completed execution narratives do not remain in this live queue.
