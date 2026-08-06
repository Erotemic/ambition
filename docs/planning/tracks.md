# Tracks — current executable queue

This file is the live queue, not a completion ledger. The completed July 15–16
architecture campaign is summarized in [`status.md`](status.md); the 2026-07-19
deep-review evidence behind the newer tracks is
[`../archive/reviews/deep-review-2026-07-19.md`](../archive/reviews/deep-review-2026-07-19.md).
Cross-track engine strategy — the capability destination and the dependency order
between campaigns — is
[`engine/competitive-2d-platformer-engine-roadmap.md`](engine/competitive-2d-platformer-engine-roadmap.md);
this queue may reorder bounded slices ahead of it when a concrete game or platform
blocker has higher immediate value, but should record why.

**Executor grades** (vision §7, restored): **[fable]** genuinely hard
design/kernel work; **[opus, fable-specced]** the spec in the named doc IS the
design — execute it verbatim and STOP at the first factual mismatch;
**[opus]** well-bounded engineering with shape + exit criteria;
**[sonnet]** mechanical with exact file lists. An agent may not deviate because
a step is hard or looks unnecessary; deviation is legitimate only when the code
contradicts the plan's factual assumptions — surface the mismatch.

**Standing fable-hard list:** ~~the FB6 rollout redesign (track 6)~~ **DONE
2026-07-30** — fighter-brain.md §12; what remains of FB6 is [opus,
fable-specced] execution; the falling-sand solver correctness pass
(pooling/termination — Jon: "getting falling sand to work right is part of
the engine"); boss-fight *quality* grammar beyond validation (boss-design.md's
open iteration loop).

## Relativity research lane (opened 2026-08-04)

- **SR-1 / TwinTrack signal-course candidate:** dimension-independent Minkowski clocks/events/Doppler mathematics, an opt-in 2D spacetime provider, rollback-safe proper clocks and cooldowns, analytic null signals, ordered receiver events, bounded worldline telemetry, and a complete Doppler-lock/radar-echo/reunion course. See [`engine/relativity.md`](engine/relativity.md) and [`demos/twintrack.md`](demos/twintrack.md). Local compile and visible-feel validation remain the candidate gate.
- **SR-2 / TwinTrack relativistic observatory candidate:** bounded compact-source worldlines, deterministic past-light-cone intersection, exact point-source aberration/Doppler, a synthetic distant-source sky, retarded source proxies, and a private dual-view spacetime instrument. The authoritative lab chart and ordinary movement authority remain unchanged. Local compile and visible-feel validation remain the candidate gate.
- **SR-3 / TwinTrack causal-pursuit candidate:** observer-local aim, exact constant-velocity null interception, a retarded-image moving target, swept pursuit hits, and lab/dual/optical-focus presentation modes. The challenge requires aiming at the future intercept rather than the apparent source. Local compile and visible-feel validation remain the candidate gate.
- **SR-4 / optical gameplay:** extend the observatory facts toward sensor fusion, simultaneity-sensitive gates, accelerated targets, and eventually retarded perception of extended scene geometry; keep exact compact-source claims separate from illustrative screen-space effects.
- **GR research path:** analytic curved providers first, then sampled/evolved fields. Minkowski remains the switchable flat limit and validation oracle. Do not begin dynamic numerical relativity by bypassing the spacetime-provider boundary.
- **Slower Light:** separate future 3D game; deferred until a 3D runtime exists.

## Hardest UNBLOCKED work, ranked (survey 2026-07-23)

One list, so choosing the next big push never requires re-deriving it. Ranked
by hardness × payoff; every item below is executable today per its own doc.

1. ~~**Phase 4 — room lifecycle onto ONE construction transaction**~~
   **LANDED 2026-07-23** (`637797649`, `58c43e900`, `d1e26aa79`,
   `37e041810`): every authored family in a shipped room is a construction
   plan row, the outer roster is exactly `planned_ids()`, all five lifecycle
   paths share one transaction, and stale content bindings are refused at the
   boundary. Recorded-open in 4g: live identity index, staging world.
   **Phase 5 LANDED 2026-07-23** (`4af17280e`→`f5fdbb4b9`): coverage forcing
   functions, demo registration (+ behavioral restore tests that caught the
   unanchored mode owners, `d73391c8a`), and the track-0 exit oracle GREEN
   un-ignored — strike volumes joined the rollback envelope, the Combat
   chain consumes its own frame's messages, the clock chain moved to the
   frame tail, and the victim-hit FIFO is lifecycle-voided + checksummed
   (`fd7ddbc0c`). **Phase 6 second slice LANDED** (`50ea3efbe`): Outlander
   LAUNCHES — activation walk + ridge-gate proof shared by binary and
   fixture test, `external consumer: outlander` gate job.
   **← CURRENT (Jon, 2026-07-23): Phase 6 remainder (visible-shell half,
   task-7 measurements) per the campaign doc.** Task-7 error-diagnostics
   slice LANDED 2026-07-24 (`35755d097`): headless content-load failures now
   surface the provider reason (`ShellCommandRejection::LoadFailed` carries
   `Vec<LoadFailure>`) + a host-agnostic `log_shell_routing_failures` system,
   instead of a route stalling silently. Remaining: the visible run (needs a
   display) and the quantitative workflow measurements.
2. ~~**FB6 rollout redesign** (track 6, [fable])~~ **DESIGNED AND
   IMPLEMENTED 2026-07-30** (fighter-brain.md §12, slices FB6a–e): the
   shadow-model rollout stack is in `brain::fighter::rollout`, striking with
   the hit-response kernel carved to `ambition_platformer2d_core::hit_response`,
   with the determinism, bench, and real-sim fidelity instruments beside it.
   **Remaining in this track: FB4b, the decision rig — specced as
   fighter-brain.md §13, [opus, fable-specced].** It is what unlocks the
   ladder self-play rig, the APM/reaction humanity checks, and FB6e's
   `l3_earns_its_depth`; ladder rows keep `rollout_depth: 0` until that gate
   exists.
3. ~~**Matchbox two-peer transport + predicted-A/corrected-B oracle**
   (netcode.md "next online slice"; unblocked since the confirmed-frame
   quarantine landed 2026-07-21).~~ **DEFERRED to the Super Smash Siblings era
   (Jon, 2026-07-24).** Matchbox/P2P netplay is out of scope until a game needs
   it; the transport, the coordinated peer barrier, and the External rebase seam
   stay documented seams, not queue items. Agents keep drifting here — pick a
   different lane. The sync-test rollback foundation is enough for now.
4. **Participant-action PA1–PA7** (participant-action-system.md) — per-seat
   input authorities; foundation for local-N SSB.
5. **The 311-site `add_systems(Update, ...)` everything-schedule**
   (code_smells.md 2026-07-23; perf campaign) — 10–18% executor self-time in
   every phase, title screen included.
6. ~~**CM8 victim-side feedback seam** (track 8; design pre-solved in
   combat-model.md §8; includes a live enemy-vs-enemy feedback bug).~~
   **LANDED 2026-07-24** (`defc4e78e`): ONE `emit_hit_feedback` (attack owns
   `strike_sfx`, victim owns `HurtFeedback`); the three `is_player` payload
   forks — incl. the live enemy-vs-enemy `PLAYER_DAMAGE`+red-burst bug — are
   deleted. OWED is a content pass only (per-archetype hurt authoring +
   distinct sounds), not engine work. See combat-model.md §8 "As landed".
6.5. ~~**The binding resolution boundary**~~ **LANDED 2026-07-25** — every
   cross-layer reference (anim row, item art, brain key, patrol path, room
   link) resolves once at construction into a typed handle; what does not
   resolve lands in ONE report naming the declarer and the available ids.
   `SheetRecord::row_index_of` and both `HashMap<String, _>` art maps are
   DELETED, so the `unwrap_or(0)` / silent-placeholder paths cannot be
   rewritten. Swept in the real `prepare` (both channels) and in
   `RoomSet::from_parts`. See
   [`engine/binding-resolution-boundary.md`](engine/binding-resolution-boundary.md).
   Remaining namespaces (moveset clips, music, recipes, dialogue) are small
   slices, logged in `dev/journals/code_smells.md`.
7. **Role evictions** (track 7) and the **glob-import untangle**
   (code_smells.md 2026-06-26, 30-site worklist) — incremental, parallel-safe.

Hard but **gated**: falling-sand fluids (⛔ Jon's explicit go-ahead required);
boss-quality grammar (wants Jon's calibration in the loop). Phase 5 /
Milestone D was UNGATED 2026-07-23: Phase 4 landed and the ADR-0027 rewrite
is done (campaign doc §7.5 + §Phase 5).

## ★ EXECUTION WAVE 1 (GPT-dialog keystones, 2026-07-19) — runs FIRST

The GPT 5.6 dialog converged on six keystone initiatives; framing and full
cards live in [`../reviews/fable-reply-2026-07-19-c.md`](../reviews/fable-reply-2026-07-19-c.md) §2
with mechanism corrections in [`-d`](../reviews/fable-reply-2026-07-19-d.md) —
this section is the bounded first wave, not a restatement. Vocabulary note
(Jon): feature bundles are "app configurations", never "personas".

**Immediate correctness** — [opus, fable-specced]
- ✅ Damage-multiplier semantics (`5148b4820`): incoming = difficulty ×
  assist only; slider scales outgoing. ✅ **Follow-up DONE 2026-07-24:** the
  outgoing `player_damage_multiplier` slider now scales player MELEE too (it
  already scaled projectiles), through the ONE `apply_feature_hit_events` seam —
  `event.damage` is scaled once for a `PlayerSlash` source, so enemy melee and
  the separate incoming difficulty/assist scale are untouched, and projectiles
  (scaled at spawn) don't double-scale. The investigation found the misnamed
  `offense.damage_multiplier: i32` is a DEAD field (written by dev-tools +
  snapshotted, read nowhere; stale `AttackSpec.damage_override` comment) —
  logged in `dev/journals/code_smells.md`, not wired.
- ✅ Keyboard-preset authority (`b10e45fbb`): `UserSettings.controls
  .keyboard_preset_index` is the one source; `DeveloperRuntimeState.preset_index`
  DELETED (it had no writer — the picker was a no-op).
- ✅ Portal composition + gate (`e4edd4acb`): host `portal` forwards
  `ambition_platformer2d_runtime/portal`; `demo_shell_smoke` 6/6 under `portal_render`;
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
  `apply_editable_movement_tuning` in `DevEditApplySet`. `ambition_platformer2d_core`
  promoted dev-dep → dep in `ambition_platformer2d_provider` (no new graph edge;
  actors already pulled it in). Remaining `EditableMovementTuning` references
  are editor paths only (inspector registration, settings/kaleidoscope
  writers, seeding, test fixtures) — verify with
  `rg EditableMovementTuning -g '*.rs' | grep -v ambition_dev_tools`.
  A live-edit test pins that F3 still reaches the sim; an adapter bug it
  caught (Bevy counts insertion as a change, so the mirror's defaults
  stomped authored tuning on frame one) is fixed and poison-tested.
  **LATER K1 completion** (unchanged, NOT done): deleting the
  `ambition_dev_tools` dep from actors/runtime still needs `DeveloperRuntimeState`,
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
    `build_platformer2d_asset_catalog*` family, and `AmbitionAssetSourcePlugin::for_profile`
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
    `ambition_platformer2d_actor_monolith/examples/render_room_geometry.rs` loaded through the
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
     `AmbitionGameSimulationPlugin` and get their root for free at build time. They
     must compose the shell and **settle N frames until the world exists**.

  **Two risks, both structural:**
  - **sync→async.** ~35 integration files behind `tests/common/mod.rs` +
    `Platformer2dSimHarness` (`rl_sim/mod.rs:64`), plus `run_headless`
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
- ✅ **DONE** (verified 2026-07-24) — Kill the vacuous projectile-anchor
  `.all()` (`desync_canary.rs`) + ONE strong mutable-state rewind canary.
  `assert_family_anchored` now REFUSES to pass on an empty family (asserts the
  marker population is non-empty before counting unanchored, so a family with
  no members can no longer "pass" vacuously). The strong mutable-state rewind
  canary is the track-0 exit oracle (`rollback_exit_oracle.rs`, Phase 5 task 4):
  melee hit + armor spend + switch flip + brick break survive a forced rollback
  window checksum-identically. No scenario matrix — the old Track-0 exit list is
  opportunity, not contract.
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

Landed; moved verbatim to [the archive](../archive/tracks-landed-sections.md).

## 2. Build-graph hygiene (compile-time wins) — **LANDED 2026-07-19**

Landed; moved verbatim to [the archive](../archive/tracks-landed-sections.md).

## 2.5 Make `RoomReplayRequested` a real seam — **LANDED 2026-07-21** (`cf5095576`, `7743d224f`)

Landed; moved verbatim to [the archive](../archive/tracks-landed-sections.md).

## 3. Close Super Mary-O level 1 — **LEVEL-1 GATE CLOSED 2026-07-21** (`d92791435`)

Landed; moved verbatim to [the archive](../archive/tracks-landed-sections.md).

## 3.5 Room transitions are an ENGINE capability — **LANDED 2026-07-25**

Landed; moved verbatim to [the archive](../archive/tracks-landed-sections.md).

## 4. Close one complete Sanic act — [opus]

Corrected 2026-07-19: the ring economy (35 authored rings, collect SFX) and the
badnik enemy loop (stomp-with-bounce AND roll-through defeat through shared
contact/combat) are **landed** — the old "bits" and "one enemy loop" bullets
are done. The remaining work is the list in [`demos/sanic.md`](demos/sanic.md),
which already declares itself the single source — **refer, never copy** (these
are the copies that "drifted independently — again").

Do not absorb movement/contact work owned by another active campaign.

## 5. Provenance + three-origin `ConstructionPlan` vertical slice — **LANDED 2026-07-22**

Landed; moved verbatim to [the archive](../archive/tracks-landed-sections.md).

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

- move `ambition_platformer2d_actor_monolith/src/menu/` (product Map-tab/settings-IR content) to the
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
oracle "add a character/item without editing core" holds for items — identity and
policy enter through the provider-owned seam, with no closed engine roster and no
game-named core branch.

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
  2026-07-19**, and it was hiding a real bug. `ambition_portal2d_presentation`
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
  `ambition_sim_view`, which depends on `ambition_platformer2d_actor_monolith`; consuming it would
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

Landed; moved verbatim to [the archive](../archive/tracks-landed-sections.md).

## Parallel maintenance — [sonnet unless noted]

Small non-blocking work when it does not collide with the campaigns:

- finish the bounded boss animator fold [opus]: converge `BossAnim`/boss frame
  projection toward the shared `CharacterAnim` vocabulary and retire obsolete
  `target_pos`-style mirrors where still live. Do **not** reopen boss body
  integration (`integrate_boss_bodies` already delegates to the canonical
  body kernel);

- planning-doc repairs queued by the deep review: rewrite
  `engine/room-transition-loading.md` to current-architecture shape (Phases 1–4
  landed; keep Phase 6 performance closure) [opus]; ~~reframe
  `engine/immutable-content-…md` §7.5–7.6 + Phase 5 under ADR 0027~~ **DONE
  2026-07-23** (rewritten as rollback-envelope hardening; Milestone D
  annotated with what ADR 0027 already delivers);
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
  Files: `ambition_platformer2d_actor_monolith/src/{action_scheme.rs, features/ecs/autonomous_reconcile.rs}`,
  `ambition_audio/src/catalog.rs`, `ambition_characters/src/{action_scheme.rs,
  actor/character_catalog/{binding.rs, mod.rs}, equipment.rs}`,
  `ambition_encounter/src/{lifecycle.rs, waves.rs}`,
  `ambition_platformer2d_ldtk/src/conversion/mod.rs`,
  `ambition_platformer2d_provider/src/lifecycle.rs`,
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
- **declared-id resolution checks — LOW PRIORITY, triaged 2026-07-25**
  ([`triage/declared-id-resolution-checks.md`](triage/declared-id-resolution-checks.md)):
  five things in the Mary-O/Sanic round were invisible or unreachable because a
  declared string id named a target that did not exist, and every one failed
  SILENTLY — `Option` cannot distinguish "this build has no assets" from "this
  content named nothing". A boot-time pass is REJECTED (Jon, startup cost); the
  triage records the two zero-runtime-cost options (extend the existing
  `every_*` registry tests in the direction they do not currently check, plus
  `error_once!` at the ~6 miss sites) and the compile-time end state;
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
