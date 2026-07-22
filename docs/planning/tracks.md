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
- ✅ **Audio/VFX/persistence/trace confirmed-frame quarantine — LANDED
  2026-07-21** (`ab8a5a564`, `14fbc6ec4`, `385a165ee`, `2eb14ef9e`). Track 1
  below is the account; the interim state described in the rest of this bullet
  is history. `SfxEmissionGate` is deleted, and its deletion was required
  rather than tidy-up.
- ◐ Audio replay-echo suppression + writer seam (`010c84369`) — the interim
  step, superseded above. **NOT
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
- ✅ **K2a world-manifest parameterization** (opus, 2026-07-21). The
  `OnceLock`, `install_world_manifest`, the free `world_manifest()` accessor,
  and the implicit `cfg(test)` fixture branch inside it are all DELETED.
  `WorldManifest` is now an ordinary owned value with two delivery routes,
  both carrying the same value from one owner:
  - **`&WorldManifest` argument** for readers that run pre-`App`, at
    plugin-build time, or as pure functions — `load_default`,
    `load_default_for_dev`, `merge_secondary_worlds`, `load_from_disk_at`,
    `to_room_set`, `LdtkHotReloadState::from_catalog`, the whole
    `build_sandbox_catalog*` family, and `AmbitionAssetSourcePlugin::for_profile`
    (a plugin VALUE built before `add_plugins`, so a `Res` genuinely cannot
    reach it).
  - **`Res<WorldManifest>`** (it derives `Resource`) for the in-schedule
    readers: `load_ldtk_asset_handle`, `spawn_ldtk_world_root(s_scoped)` on
    both the direct-entry and shell-host paths, `handle_ldtk_hot_reload`, and
    `setup_host_presentation_system`. `AmbitionContentPlugin::build` publishes
    it where `worlds::install()` used to sit; `init_sandbox_resources` threads
    the same value by reference through every preparation-time reader.

  **Oracle** (`app_it::world_manifest_parameterization`, 5 tests): two
  declarations with disjoint world files AND disjoint entry rooms compose in one
  process, in both orders, each keeping its own rooms and its own start room.

  ⚠ **STRENGTHENED 2026-07-21.** The original three tests said "two providers
  prepare" but built two bare `WorldManifest` VALUES and called pure functions
  over them — no `App`, no provider, no plugin. That is near-tautological as an
  isolation proof (a function taking the manifest by reference and reading
  nothing else cannot leak between callers) and it was blind to the route K2a
  actually changed: `insert_resource` at provider-build time, read as
  `Res<WorldManifest>` in schedule. Its poison test only bit because it stubbed
  those two pure readers directly. Two App-level oracles now cover the real
  boundary: `two_apps_keep_their_own_manifest_through_in_schedule_readers`
  builds two `App`s in one order, steps them INTERLEAVED in the other, and
  asserts each App's own scheduled reader saw its own entry room every frame;
  `the_real_content_provider_publishes_into_its_own_app_only` builds the actual
  `AmbitionContentPlugin` beside a second App and checks neither learned the
  other's declaration.

  Still uncovered, found while doing this: a live first-wins `OnceLock`
  (`EXTRA_ENTITY_CONVERTERS`, `ldtk_map/src/conversion/mod.rs:633`) sits one
  call away from `to_room_set`, with the same silently-dropped `Err` this track
  condemns. It is dormant — `install_ldtk_entity_converters` has zero callers —
  so it is a latent hazard, not a live bug.

  Two things fell out that the card did not predict:
  - `LdtkProject::load_static_map` had ZERO callers. Deleted.
  - `build_sandbox_catalog_without_worlds` +
    `sandbox_catalog_inputs_without_worlds` existed only because the global
    could not express "this game ships no worlds" except as a panic. With the
    manifest an argument that is just `&WorldManifest::default()`
    (`is_world_less()`), so the twin is deleted and the two procedural demos
    call the ordinary builder.
  - ⚠ Found by the change, not fixed by it:
    `ambition_actors/examples/render_room_geometry.rs` loaded through the
    global while never installing one, so it panicked the moment it ran. The
    explicit parameter turned that into a compile error; the example now
    builds its own manifest and works.

  **NOT done (K1-style remainder):** `PlatformerExperienceAuthoring` still has
  no `with_world_manifest` builder — the engine-level provider seam next to
  `with_presentation_profiles` is the natural owner, but Ambition's content
  plugin is today's publisher and adding a second writer for one user would be
  speculative. Fold it in with K2b, which touches that builder anyway.
- ▢ **K2b direct-entry activation** [opus]: route direct entry through the
  EXISTING `activate_prepared_platformer_sessions` /
  `PlatformerSessionBuilder` (no neighboring API). Oracle: the hand-built
  `SessionRoot` at `app/resources.rs:295-322` is deleted.

  **⚠ SCOPED 2026-07-21 (opus, structural trace done — the card badly
  understated this).** The blocker is not the `SessionRoot` spawn; it is that
  the spawn happens **at plugin-build time, before tick 0**, and activation
  happens **asynchronously over several `Update` frames**. Everything below is
  anchored and SCOPED — file:line trace, five numbered edits, two named
  structural risks, a three-stage plan. None of it is compiled or tested, so
  "pre-solved" (the earlier wording) overstated it: the settlement behavior is
  designed, not demonstrated.

  *Today (direct entry):* `publish_direct_prepared_session_root`
  (`app/resources.rs:295`, called from `app/plugins.rs:132` at the END of
  `add_simulation_plugins`) spawns `SessionRoot(SessionScopeId(0))` + live
  world + content + identity. The player comes later from
  `setup_simulation_system` (`app/setup_systems.rs:35`, `run_if(direct_entry)`),
  which calls the SAME `session::setup::simulation_world` the shell builder
  calls — but `UNSCOPED`, with a hardcoded
  `PLAYABLE_ROSTER[0]` default character, and it never inserts
  `GameplayInputOwner`.

  *Target:* direct entry is just **a shell host whose initial route is the
  gameplay route** — the recipe `ambition_demo_sanic_app/src/lib.rs:79-84`
  already proves (`ShellHostSpec::new(<gameplay_route>, <home_route>)`).
  No new API; `PlatformerExperienceAuthoring::install` already registers the
  preparation plan.

  **The edits** (in order):
  1. `app/cli.rs:790-818` — stop using `AmbitionShellHosted` as the
     discriminator; always compose the shell host + visuals, and in direct mode
     set the initial route to `AMBITION_GAMEPLAY_ROUTE` and skip the startup
     vanity sequence.
  2. Delete `publish_direct_prepared_session_root` + its call at
     `app/plugins.rs:132`.
  3. Drop four `run_if(direct_entry)` registrations and un-gate the host one:
     `sim_resources.rs:44` (`setup_simulation_system` — the system itself
     likely dies), `plugins.rs:288` (`spawn_ldtk_world_root`), `plugins.rs:515`
     (`setup_presentation_system`), `plugins.rs:525` (`spawn_map_menu`);
     `plugins.rs:517-523` becomes unconditional.
  4. Delete the direct audio branch `app/resources.rs:80-98` —
     `select_shell_audio_context` (`game_shell/src/session.rs:399`) owns
     selection + `SfxEmissionContext` on activation.
  5. **This is where the actual work is:** `headless.rs:124`,
     `rl_sim/mod.rs:64`, `bin/capture_scene.rs:143` add only
     `SandboxSimulationPlugin` and get their root for free at build time. They
     must compose the shell and **settle N frames until the world exists**.

  **Two risks, both structural:**
  - **sync→async.** ~35 integration files behind `tests/common/mod.rs` +
    `SandboxSim` (`rl_sim/mod.rs:64`), plus `run_headless`
    (`headless.rs:137-142` `.expect("active session RoomSet")`), do
    `App::new(); …; update(); read_the_world()`. After the migration the root
    exists only after the load barrier reaches `Ready` and all 8 preparation
    work items complete (`game_shell/src/preparation.rs:27-40`). **Do the
    settle helper FIRST, as its own commit, before deleting anything.**
  - **`SessionGatedSimulation` semantics flip.** Composing the shell installs
    `GameplaySessionBridgePlugin` → `SessionGatedSimulation`
    (`game_shell/src/session.rs:306`), flipping `simulation_authorized`,
    `session_world_exists`, `session_world_entity`, and
    `declare_gameplay_input_context` from "one root is enough" to "root scope
    must equal `ActiveSessionScope::current()`" — for the headless/RL harnesses
    too. The root also becomes `SessionScopedEntity`-tagged and therefore
    despawnable by `despawn_retired_session_entities`, a teardown-bug class
    that structurally cannot occur today.

  Note `SessionScopeId(0)` at `resources.rs:316` is an arbitrary placeholder,
  not special — the first shell activation also mints 0
  (`ActiveSessionScope::begin`), and they do not collide today only because
  direct entry never installs `SessionScopePlugin`. It disappears entirely.

  No test asserts on `publish_direct_prepared_session_root` or
  `SessionScopeId(0)` directly — the coverage is all implicit, which is exactly
  what makes risk 1 dangerous.

  **Suggested staging:** (K2b.1) land the settle helper + migrate
  `headless`/`rl_sim`/`capture_scene` to compose the shell and settle, keeping
  the build-time root as a fallback and proving both paths agree; (K2b.2)
  delete the build-time root and the four `direct_entry` gates; (K2b.3) delete
  `AmbitionShellHosted` / `shell_host::direct_entry` once nothing reads them,
  and fold `PlatformerExperienceAuthoring::with_world_manifest` in (the K2a
  remainder above) while that builder is already open.

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

## 1. Quarantine external effects to confirmed GGRS frames — **LANDED 2026-07-21**

`ambition_engine_core::ConfirmedFrameBoundary` is the host's published answer to
"which frames can never be simulated again", and
`ambition_runtime::external_effects` is the mechanism that keys irreversible
work to it. **Deferral, not suppression** — the distinction is the whole track.

The sim's effect channel became an **outbox**: cleared at the start of each
advance, journaled at the end under the frame that produced it, released back
into the same channel once that frame confirms. Presentation consumers are
unchanged and unaware. Re-simulating a frame REPLACES its intents *including
with nothing at all*, which is the half a boolean gate structurally cannot
express and is what erases a phantom.

- ✅ **Classification** (`quarantine_presentation_effects` — the list IS the
  classification, pinned by `only_presentation_facing_effects_are_quarantined`):
  `OwnedSfxMessage`, `VfxMessage`, `ExplosionRequest`, `FireworksRequest`,
  `DebrisBurstMessage`. ⚠ **The work-list was wrong about `EffectRequest`** —
  all three of its readers are sim-side (`apply_effects` spawns hitboxes,
  `apply_summon_effects` spawns minions, `apply_enemy_projectile_effects`), as
  is `SpawnProjectile`'s. Deferring one would not quarantine an external effect;
  it would change what the simulation computes. The split is "who reads it", not
  "effect-shaped name", and erring permissive is a desync, not a duplicate sound.
- ✅ **Buffer by frame + session identity**; ✅ **release exactly once, in
  simulation order**; ✅ **discard the abandoned branch at `LoadWorld`** and
  invalidate on session replacement (a generation counter on the boundary).
- ✅ **`SfxEmissionGate` DELETED**, and the deletion is load-bearing rather than
  tidying: suppressing at emit time destroys the corrected sound before anything
  downstream can decide whether the prediction it replaces was ever heard.
- ✅ **Persistence** (`385a165ee`): the autosave is gated on the world holding no
  predicted state, and change detection is replaced by a comparison against what
  was last committed. The second is what makes the first safe — `is_changed()` is
  consumed by a system that ran and declined to write, so any run condition in
  front of it silently swallows real changes. Settings deliberately get the value
  comparison but NOT the gate (not rollback state, all writers menu-side); the
  reasoning and its expiry condition are recorded at the call site.
- ✅ **Forensic trace** (`2eb14ef9e`): rows keyed by `sim_frame`, corrections
  replace predictions in place. The old `simulation_pass_is_authoritative` gate
  was *neither* option the review offered — "authoritative" meant FIRST PASS, so
  a mispredicted frame kept its guess permanently. Anomaly detection and dump
  arming stay first-pass (a file write must happen once) while the rows inside
  the dump still get corrected, since the flush runs in `PostUpdate`.

**Two traps worth keeping.** `Messages::drain` takes both of Bevy's
double-buffers, so without the start-of-advance clear the previous render
frame's already-released effects get journaled again and replayed — poison-tested
by `without_the_clear_the_effect_would_be_replayed`. And registering the release
in `PreUpdate` is not enough: with no edge against `RunGgrsSystems` Bevy may
release *before* the advances, and the next clear then wipes what was just
handed to presentation, silently, because the journal already counted it. The
integration oracle found that one; the unit tests structurally could not.

**Exit (met, with one clause narrowed).** `app_it::effect_quarantine`: the same
input script on the same GGRS host, once never rewinding and once rewinding
every step, must deliver the same effects in the same order. Poison-tested —
disabling the quarantine yields 46 effects against 10, each sound roughly five
times over, the original bug in its observable form.

⚠ **Not claimed: a live mispredicted remote input.** A sync test resimulates
with the *same* inputs, so its correction always equals its prediction and
A-versus-B cannot arise there. The A≠B rule is proven against the real systems
in `external_effects/tests.rs`
(`a_corrected_frame_replaces_what_the_prediction_produced` plus the
produces-nothing variant). Proving it end to end needs two peers, and ggrs's
handshake is wall-clock gated (200ms sync-retry interval), so a live two-peer
test would be timing-flaky in a repo whose determinism doctrine forbids exactly
that. **Owed when the Matchbox transport lands** — the transport and the proof
are the same piece of work, and that is the honest place for it.

**Still open from this track:** attach a Matchbox transport through the existing
`install_session` seam (unchanged by this work — no simulation system was
touched) and land the two-peer predicted-A/corrected-B oracle with it.

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

## 2.5 Make `RoomReplayRequested` a real seam — **LANDED 2026-07-21** (`cf5095576`, `7743d224f`)

`ambition_runtime::sandbox_reset` now owns `reset_sandbox` and the ONE
`apply_room_replay_request_system`, carried to every host by
`RoomReplaySchedulePlugin` in `PlatformerEnginePlugins`. The two content anchors
(`ContentDialogueFollowupSet`, `ContentRoomReplayResetSet`) moved with it, since
the engine now owns the consumer they order against. Ambition keeps only its
reset-INPUT system (the button binding is Ambition's) with an explicit `.before`
edge to the consumer — the old `.chain()` was the only thing making those two
unambiguous.

**The blocker was a MODULE, not a dependency.** The card said the consumer was
stuck app-side because it called `reset_sandbox`, "a host/reset concern". In
fact `reset_sandbox` names only `engine_core`/`actors`/`characters`/`sfx`/`vfx`,
every one of which `ambition_runtime` already depended on; it was unmovable
only because it sat in `app::world_flow::room_flow`, which also composes
`load_room` with `ambition::render` spawns. Splitting the reset out of that
module is the entire unlock.

**Exit met.** Nine tests across the three hosts
(`ambition_demo_{mary_o,sanic}_app/tests/room_replay.rs`,
`ambition_app/tests/app_it -- room_replay_seam`): the seam itself, Mary-O's
TIMEOUT beat end to end, Sanic's act clear past the FULL `ACT_CLEAR_DWELL`, and
a one-request-one-reset count per host. Poison-tested both directions — dropping
the plugin fails all nine (Mary-O stays at her full 600px displacement, Sanic at
1060 vs a spawn of 160); re-adding a duplicate app-side registration fails the
hosted pair.

Two findings recorded rather than fixed:
- Sanic can clear the act and then coast off the end of the speedway into a pit
  death, inside his own 4s results dwell. That is why the act-clear proof stamps
  the cleared phase under controlled conditions instead of extending
  `act_completion.rs`: the death respawn rebuilds the room by itself, so that
  run cannot isolate a replay. Logged in `code_smells.md` (2026-07-21).
- A duplicate consumer is a hard Bevy panic in `ambition_app` (the reset-input
  `.before` edge cannot resolve against a twice-registered system) but SILENT in
  the demo apps, which have no such edge. Hence the count assertion.

Mary-O's open acceptance run can now assert the replay clause it was written
against ("waits through an actual replay into a fresh level").

## 3. Close Super Mary-O level 1 — **LEVEL-1 GATE CLOSED 2026-07-21** (`d92791435`)

The acceptance run landed: one state-aware controller plays spawn → ?-block →
milk → pit A → secret pipe → vault → 8 coins → return pipe → surface →
re-power → pits B and C → stair pyramid → pole → tally → a real replay back to
spawn, with no positional set-up and all three lives intact. **Nothing in the
codebase previously proved any pit was crossable.** Full clause-by-clause
account in [`demos/super-mary-o.md`](demos/super-mary-o.md) — single source,
do not copy back here.

Three bugs fell out of writing it, all invisible to the existing tests because
every prior proof either set her position past the terrain or asserted a value
the emitter wrote:
- the secret vault had **no working exit** (return pipe block derived from its
  interact band, floating it 48px clear of the floor). FIXED `cbc6902d2`. Its
  own "sealed vault" test stayed green by checking a body at the band's centre —
  inside solid rock — and the scripted seam run stayed green by teleporting her
  to exactly that unreachable point;
- **a body reset redefined the body** — `reset_body_clusters` hardcoded the
  default size into `base_size`, so any identity-driven size (a worn form, a
  mount, a boss phase) was silently unmade on every reset. FIXED `4e4bd0fd8` in
  `ambition_engine_core`; engine-wide, not Mary-O's;
- **pit B opens into the secret vault**. REPORTED, authoring call:
  [`triage/room-replay-followups-2026-07-21.md`](triage/room-replay-followups-2026-07-21.md) §5.

**Exit (met):** visible and headless customers use the same provider, body,
item, and level state with no Mary-O-only engine path.

**Now unblocked:** additional authored levels, which were gated behind this.

## 4. Close one complete Sanic act — [opus]

Corrected 2026-07-19: the ring economy (35 authored rings, collect SFX) and the
badnik enemy loop (stomp-with-bounce AND roll-through defeat through shared
contact/combat) are **landed** — the old "bits" and "one enemy loop" bullets
are done. The remaining work is the list in [`demos/sanic.md`](demos/sanic.md),
which already declares itself the single source — **refer, never copy** (these
are the copies that "drifted independently — again").

Do not absorb movement/contact work owned by another active campaign.

## 5. Provenance + three-origin `ConstructionPlan` vertical slice — **LANDED 2026-07-22**

Full account in
[`engine/immutable-content-and-transactional-construction.md`](engine/immutable-content-and-transactional-construction.md)
Phase 3 — single source, do not copy back here. Headlines:

`ambition_platformer_primitives::construction` is the content-free planner;
`ambition_actors::construction` puts the three real origin families through it
(authored `GroundItemSpec`, provider-staged `SpawnActorRequest`, `Effect::Summon`
minion). Every exit clause met, each with a named test.

**The result worth remembering is that provenance stopped being a spelling.**
`SpawnOrigin` is a snapshot-registered component; the one place in the tree that
parsed a `SimId` (`heal_projectile_owners`, `rsplit_once('/')`) is deleted. Two
stale claims fell out of doing it and are corrected: `ProjectileOwner`'s
registered derived-state justification named a field that is EMPTY for every
player projectile, and `SimId::as_str` documented itself as "never parsed" while
being parsed one crate away.

Three failures that were silent skips are now preflight failures — an authored
ground item naming an unregistered held item, a staged duellist grudging an
actor outside its batch, and two summons colliding on one authored id. Each had
been invisible because the spawner swallowed it.

⚠ **Deliberately partial, and that is the card, not a shortfall.** Only ONE
family per origin kind is migrated; authored placements, enemies, bosses,
shrines, gravity zones and portal guns still take the family-specific loops in
`RoomFeatureConstructionPlan::spawn`. Those are Phase 4's migration order.
`apply_spawn_actor_requests` also survives on purpose — programmatic scene setup
(RL reset, demo crony spawns) legitimately wants a message.

⚠ **A SECOND review round found four of those five repairs incomplete, and one
encoding a new wrong invariant** — the relation rule permitted cutting a
relation's target, which strands the untouched source on a dead `Entity` handle
(proven: `Grudge(1v0)` vs a rebuilt `1v1`). All now repaired: symmetric relation
rule + `relation_closure`; executor allocates the root via `ConstructionRoot` so
a recipe cannot commandeer or nominate one; `AcceptsFn` and the request's
`recipe` field deleted in favour of derived `recipe_of` + exhaustive
`construct`; counter advance queued as part of the commit; `ContentBinding`
replaces the thrice-overloaded epoch-zero sentinel.

⚠ **Checkpoint 1 of the third review round landed:** restored four relation
tests my own previous commit silently deleted (an edit truncated the file and the
reported count was never re-derived) and extended them to six cases; collapsed
`recipe_of`+`construct` into one `dispatch` so identity and behaviour cannot
drift; the construction registry now genuinely reaches the prepared-content
fingerprint as `construction.recipes` (it was documented as doing so for two
commits before it did); summon counter reservations carry the value planning read
and refuse a stale or missing counter BEFORE spawning. Actor recipes are now
documented as a CLOSED domain — providers register metadata, not executable
behaviour. **There is no enforced plan-to-world roster parity and the docs no
longer claim one.**

⚠ **Checkpoint 2 (substrate) landed:** the prepared plan now freezes its
resolved constructor (commit no longer re-dispatches — proven against a domain
whose `dispatch` flips on an atomic); summon reservation check+build+advance are
one exclusive-world boundary with the `max()` recovery deleted; relations carry
schema metadata that reaches the fingerprint; `verify_committed_roster` counts
identities and flags unplanned roots, with six adversarial recipes proving it.
⚠ It DETECTS, it does not prevent — Bevy commands do not roll back.

⚠ **Checkpoint 3 (substrate) landed:** verification became something a
transaction has to pass rather than a function tests could call. The baseline
holds entity + provenance per identity, not a `BTreeSet<SimId>` (which could not
tell an original from a replacement), refuses capture on a pre-existing
duplicate, and takes retirement/reconstruction as DECLARED rather than inferred
from the plan. Authoritative scope is now gathered by querying the world and
classified by component — this transaction's `TransactionId` stamp, another's,
an explicit `PresentationOnly` opt-out, or no ownership at all — so a caller can
no longer make the check incomplete by forgetting a root. Relations carry a
frozen `verify` beside their `wire` and are checked against committed components
(a receipt only proves the wiring function was CALLED). `fn_addr_eq` is gone from
registration semantics: it made a registry contract depend on codegen.
**`RoomFeatureConstructionPlan::spawn` no longer writes `RoomLoaded`** — a
queued capture runs before construction and a queued verify-and-publish after
it, and a fatal violation withholds publication.
⚠ Still a detector: there is no staging world, so nothing rolls back.
⚠ `Severity::Unmigrated` is a deliberate temporary hole — an identity with no
ownership stamp is reported, not fatal, because nine families still build roots
outside the planner.

⚠ **Phase 4 STEP 1 landed:** `ambition.limb` and `ambition.mount` are registered
relation kinds with bidirectional wiring AND bidirectional postcondition checks.
Relations gained a typed `RelationPayload` because `Limb`'s `slot` and
`home_offset` are both stated relative to the HOST — facts about the pairing, not
about either body — so the dump gained a payload column and the plan schema is
**v3**. One function writes both ends, which is what makes the half-write
unspellable; the rig case accumulates in canonical relation order because
`fan_out_limb_intents` reads it positionally. Reverse verification is not
redundant: a limb outside its host's rig is INERT and a mount whose `MountSlot`
does not point back stops obeying (`steer_mount_from_rider` queries
`With<MountSlot>`), while every forward-only assertion passes in both cases.
⚠ **No production caller declares either relation yet** — limbs/riders/mounts are
not plan rows, so the old paths still run. That is the next commit.

⚠ **PHASE 4 IS OTHERWISE NOT STARTED.** Nine authoritative families and one parallel
`apply_spawn_actor_requests` path remain outside the planner — the exact table is
in the campaign doc. Two known holes in the current parity claim: giant hand
limbs are authoritative roots no plan row names (and are reachable from inside a
planned recipe), and `Limb`/`RidingOn` are raw `Entity` relationships invisible
to cut-detection.

⚠ **An earlier same-day review found five transactional gaps the tests could
not see** — a counter spent before validation, an unchecked recipe/parameter
pairing, an executor trusting the `Entity` a recipe returned, a parent stored
twice, and `construct_one` silently dropping relations. All closed (plan schema
is now **v2**); the Phase 3 account lists them. **Read that list before starting
Phase 4** — every one was a boundary described as atomic with nothing enforcing
it, which is exactly the risk Phase 4 multiplies.

**Next in this campaign:** Phase 4 (migrate room lifecycle operations onto the
planner, in the order activation → reset → transition → hot reload → snapshot
reconstruction), which is what turns the remaining loops into plan rows. It also
owns the two limits this slice recorded rather than solved: enforcing
`ConstructionScope::content_epoch` at a commit boundary, and the live identity
index that would let a relation target an entity outside the plan.

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
