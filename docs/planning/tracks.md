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
   ~~**Remaining in this track: FB4b, the decision rig — specced as
   fighter-brain.md §13.**~~ **THE RIG IS IN TREE** (verified 2026-08-06):
   `StateMachineCfg::Fighter`, `BrainSnapshot.attack_kit` with a real builder
   (`attack_kit_of`), the dispatcher arm and the `SnapshotCursor` arm all exist.
   The row said §13 was the executable next step after it had been executed.
   ⭐ **what was actually blocking `l3_earns_its_depth` was a missing PARAMETER,
   not a missing rig.** `ShadowTuning` had no friction term at all, so a body
   leaving a dash stopped dead in the shadow and coasted ~38px in the game — an
   under-predicted stopping distance, which is the direction that lets the
   movement veto approve a dash off the stage. Adding
   `ground_coast_decel`/`air_coast_decel` (veto untouched) took `ladder_probe`
   over 7 seeds from *every rung losing all three stocks inside ~10s* to *levels
   5/6/9 never self-KOing in a minute*, and REVERSED the A/B: `9/d0` loses a
   stock at 6.2s while `9/d12` survives the whole run.
   ▢ **still owed for the gate**: §8's scenario suite and the survival/damage
   ratios. A probe with one scenario and an opponent that cannot attack says the
   gate is worth authoring; it is not the gate.
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
  ⛔ **RE-EXAMINED 2026-08-06 when K2b landed, and DECLINED — measured, not
  assumed.** `WorldManifest` appears 80 times across the tree and NO game but
  Ambition declares one: `ambition_demo_sanic`, `ambition_demo_mary_o` and
  `ambition_demo_smash` name the type zero times between them. A builder method
  would therefore be a seam with exactly one caller, which is the
  pre-generalization the engine direction forbids outright. The row's own
  reasoning was right; folding it into K2b did not change the count.
  ✔ **the TWO-WRITERS half is DONE — verified 2026-08-07, the row was stale.**
  It read: the manifest has two writers of the same value, `ambition_content::
  plugin` and `ambition_app::app::resources`. There is exactly ONE
  `insert_resource` now (`ambition_content/src/plugin.rs:77`), and the app side
  carries the fix's own note at `resources.rs:91` — *"ONE writer, and it is the
  CONTENT plugin's (2026-08-06) … The provider that OWNS the worlds publishes the
  declaration; the host reads it."*
  ⚠ **the local `let world_manifest = …` in `resources.rs` is NOT the duplicate**
  and should not be "cleaned up" by a later reader: it is threaded BY REFERENCE
  into the preparation-time readers (catalog rows, the LDtk load, the room-set
  conversion, the hot-reload watcher) which run before any schedule and so cannot
  take a `Res`. That is the K2a no-process-global shape, and it is orthogonal to
  who inserts the resource. ⭐ recorded because the surviving `let` looks exactly
  like the defect the row described.
- ✔ **K2b direct-entry activation — DONE 2026-08-06.** The oracle is met: the
  hand-built `SessionRoot` is deleted. `compose_ambition_gameplay_host` is the
  one composition, and the whole `ambition_app` suite (471 tests) plus the
  workspace policy job are green without it.
  ⭐ **the ORDER inside that function is the part worth keeping.**
  `AmbitionShellHosted` → simulation plugin → shell. Getting the last two
  backwards panics inside Bevy parameter validation naming a system the caller
  never heard of, because the shell is an ADAPTER over a composed game rather
  than a composition of one.
  ⭐ **nine call sites had the recipe by hand**, which is why deleting the
  publisher turned every one into a failure at once — 33 tests — and the blast
  radius got MEASURED instead of estimated.
  ⛔⛔ **the deletion's real value was the four defects it exposed**, none of
  which K2b was about. Each had been invisible because the rollback harness
  composed the simulation plugin alone, so no checker had ever seen the shipped
  host:
  - **A global countdown with no owner froze every fighter.**
    `settle_versus_round` ran unconditionally and `VersusMatch` defaults to
    `Starting`, whose arm marks EVERY fighter `ScriptedControl` each tick. Any
    composition that installed the versus stage without being on its route
    gagged its own bodies. ⭐ same rule `track_versus_roster` learned about the
    roster one campaign earlier: not "a fighter exists", MINE.
    ⚠ **the symptom pointed at the wrong end.** Seat two authored 40 frames of
    right and moved 0.00px, which reads as input routing — and the input was
    measured intact through `PendingSeatInputs` → `LocalInputs` →
    `SlotControls[1]` → the brain tick, 190 times. Two plausible fixes (a
    missing `SlotControlLatches`, a clobbering host writer) were probed and
    REFUTED, which is the only reason neither shipped.
  - **18 rollback registrations were presence-only**, across mary_o, sanic,
    relativity and versus — each now carries a value projection.
    ⛔ `SpentPowerBlocks` is a `HashSet`, so an order-dependent one would have
    been nondeterministic between peers; all four set-shaped checksums XOR
    per-element hashes.
  - **`PortalGunPickup` and `HealShrine` had no rollback anchor**, so `SimId`,
    `SpawnOrigin` and `TransactionId` were inert on them. Invisible because
    direct entry built its world UNSCOPED and the sweep never looked.
  - **The schema baseline recorded a composition no player runs** — 50 rows
    added, ZERO removed, when the harness started composing the shipped host.
  ⭐ **the replay fixture is the cleanest evidence nothing else moved**: the
  regenerated trace is the old one shifted by exactly one frame
  (`new[i] == old[i+1]` for all 59, same landing), because activation takes
  frames. State what changed about a regenerated fixture or it proves nothing.

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

  **All five edits landed 2026-08-06.** 3 deleted 265 lines of the entry path
  (`spawn_ldtk_world_root`, both feature arms of `setup_presentation_system`,
  and the private chain under them — `presentation_world`,
  `presentation_world_inner`, `session_presentation`, `PresentationSetup`,
  `SessionPresentationSetup`), plus the `direct_entry` run condition itself.
  4 deleted the direct audio branch. 5 composed the shell in `headless.rs`,
  `rl_sim/mod.rs` and `bin/capture_scene.rs`.
  ⭐ **verified by CAPTURE, twice, because both deletions are silent-failure
  classes.** A deleted presentation builder passes every test and draws nothing;
  a deleted audio selection passes every test and plays nothing. The 640x360
  `--include-ui` capture of `central_hub_complex` is BYTE-IDENTICAL before and
  after edit 3, and reports `first owned SFX play attempt
  (owner=Some(Gameplay(0)))` after edit 4 — `Gameplay(0)`, not `Direct`, which
  is what makes deleting the other selection safe rather than merely compiling.
  ⚠ **`build_visible_app`'s `shell_hosted` parameter is KEPT, name and all.**
  Both arms are shell-hosted now and it only picks the initial route; renaming
  it would touch 33 call sites to restate a boolean whose two values are
  unchanged. The doc says so instead.

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

  ◐ **K2b.1 STARTED 2026-08-06: the settle helper is landed and in front of
  `run_headless`.** `settle_until_session_world(app, frames)` advances until
  `session_world_entity` answers and returns how many frames it took —
  `Ok(0)` for a build-time root, so putting it in front of a direct-entry
  caller changes nothing and the migration is stageable exactly as this row
  planned. Three tests pin the three cases: immediate, late, and never (an
  `Err(budget)` rather than a panic three lines later about a missing
  `RoomSet`).
  ✔ `run_headless` and `Platformer2dSimHarness::build` both settle now — the
  harness one matters most, because every caller reads the world IMMEDIATELY
  after construction (`room_ids()`, an observation, a `RoomSet`), which works
  today only because the root is there at build time. Best-effort while the
  build-time root still exists, a hard error in K2b.2.
  ⭐ `bin/capture_scene.rs` needs NOTHING: checked rather than assumed — it
  names no `session_world*` and holds no `.expect`, because it already waits
  through its own warmup and camera-adoption gate. The row listed it because it
  composes the plugin, and composing is not the same as reading.
  ✔ **the ~35 integration files behind `tests/common/mod.rs` need no edits** —
  they all construct through `Platformer2dSimHarness::build`, so the settle
  inside it covers every one. Risk 1 is retired at the seam rather than at 35
  call sites.
  ✔ **AND BOTH PATHS ARE NOW PROVEN TO AGREE** —
  `game/ambition_app/tests/direct_and_shell_agree.rs` builds direct entry and a
  shell host booted straight to `AMBITION_GAMEPLAY_ROUTE`, settles both through
  the same helper, and asserts they start in the SAME ROOM. Measured: direct
  settles in **0** frames, the shell in **2**. That is the coverage this row
  called *"all implicit, which is exactly what makes risk 1 dangerous"*.
  ⛔ **two things the test found that the plan did not say:**
  1. the shell composer is an ADAPTER, not a composition —
     `compose_ambition_shell_host` without `AmbitionGameSimulationPlugin` panics
     on frame one (`settle_versus_round` wants `Res<WorldTime>`);
  2. `AmbitionShellHosted` must be inserted BEFORE the sim plugin builds, or
     `publish_direct_prepared_session_root` runs anyway and the app gets TWO
     canonical roots — the `SessionScopeId(0)` collision this row predicts in
     prose, reproduced.
  ⭐ edit 1 landed additively: `compose_ambition_shell_host_booting_to(app,
  route)`, with the launcher default untouched.
  ✔ **edit 1 LANDED (2026-08-06): the shell host is composed either way**, and
  the mode only chooses the route — launcher when hosted, `AMBITION_GAMEPLAY_ROUTE`
  when `--direct`/`--start-room`. `AmbitionShellHosted` is now inserted
  unconditionally (before the sim plugins, per the collision above), so
  `publish_direct_prepared_session_root` never runs in a CLI-built app. 300 app
  tests green.
  ✔ **`run_headless` MIGRATED (2026-08-06)** — it composes the shell and boots
  to the gameplay route, so the headless report now runs the same activation a
  player does instead of a second way to start a game. 14 headless tests green.
  ⛔ **the HARNESS flip was TRIED and REVERTED, and the measurement is the
  scope.** Composing the shell inside `ambition_sim_composition` turns risk 2
  from prose into eight red tests, in two distinct families:
  * **the subject vanishes** — `desync_canary` panics at
    `the sandbox session has a controlled subject`. Under
    `SessionGatedSimulation` the root must belong to the ACTIVE scope, and the
    harness's first read happens against an activation that has not seated a
    body yet;
  * **the rewind stops agreeing** —
    `GGRS sync-test checksum mismatch at frames [2, 3, 4]`, plus
    `effect_quarantine`'s pair. A shell activation performs work DURING the
    frames the sync test is comparing, so the two runs diverge on activation
    rather than on gameplay.
  ⭐ **that second family is the real content of K2b.3**, and it is not a
  test-fixture problem: it says a rollback session must not begin until
  activation has settled.
  ✔ **the rule is STATED now (2026-08-06)**: `RollbackRefused::NoSessionWorld`,
  checked in `rollback::start` after activation and settle. A session opened
  over a world that does not exist yet is measuring CONSTRUCTION, and its
  checksum mismatch reads as a desync in the game — so refusing is the honest
  answer and the message names the helper to wait with. ⚠ it had no teeth to
  grow before: with a build-time root "the world exists" was true before frame
  one, so nothing could notice the rule was missing. 46 rollback tests green.
  ⭐ **CORRECTION (checked, not assumed): the lower-level road already stated
  the rule.** `install_rebased_sync_test_session` calls
  `warn_if_no_world_to_rewind`, whose text is the same diagnosis in almost the
  same words — *"frame zero is an EMPTY world … the frames that build the room
  will mismatch on every resimulation and GGRS will report it only as a checksum
  difference."* The row above overstated the gap; what was missing was the
  refusal at the higher authority, which is what landed.
  ✔ **but that check asked a NARROWER question, and the gap only opens under a
  shell host.** It accepted any `SessionRoot` entity, while
  `session_world_entity` also requires the root's scope to equal the active one
  whenever `SessionGatedSimulation` is installed — which the shell installs. A
  root left by a RETIRED activation satisfied the old check while every reader
  in the engine correctly saw no world, so the warning stayed silent for exactly
  the case it exists to catch. Both roads ask the same question now.
  ◐ **ATTEMPT TWO (2026-08-06): two of the three families are FIXED, and the
  third says the shape is wrong.**
  * ✔ *the subject family* — `settle_until_controlled_subject` waits for a
    seated body as well as a world, because every harness caller drives an actor
    on the next line. The `"the sandbox session has a controlled subject"` panic
    is gone.
  * ✔ *the checksum family* — and the cause was ORDERING, not the shell: the
    settle sat AFTER `start_sync_test_session` in `Platformer2dSimHarness::build`,
    so the session still opened over an activating world. Moved above it; all 8
    `desync_canary` tests pass with the harness composing the shell.
  * ⛔ *a THIRD family the plan never named, and it is the blocker*: composing
    the shell drags in EVERY PROVIDER, and providers register RENDER material
    state. `rollback_coverage` starts reporting
    `bevy_render::…PreparedMaterial2d<MaryOQuasarMaterial>` and
    `EntitiesNeedingSpecialization<…>` as unrewound resources in the harness
    world, plus an inert-registration failure. Waiving render materials into a
    rollback census would be lying about what a headless RL/oracle harness is.
  ◐ **ATTEMPT THREE (2026-08-06): the render family is GONE and ONE class
  remains.** `Material2dPlugin::<MaryOQuasarMaterial>` was being installed
  whenever an `EmbeddedAssetRegistry` existed — which every headless composition
  has — so the material's render-world resources landed in any app that merely
  had `AssetPlugin`. That is the same *"a proxy answers the question next
  door"* mistake the module's own doc already records about its RUN condition,
  repeated one line up at install time. It is gated on the render sub-app now,
  and Mary-O still draws (captured to check).
  * ✔ render resources: gone from the harness world.
  * ✔ `ContentEpoch` and `Messages<SessionScopeRetired>`: two honest waivers,
    identity and lifecycle respectively — a rewind cannot rebuild a session from
    other content, and must not un-retire a scope.
  * ▢ **ONE class left, and it is a real question rather than a waiver.**
    `no_snapshot_registration_is_inert_*` reports an archetype
    (`SpawnOrigin + TransactionId + RoomScopedEntity + SessionScopedEntity +
    SimId + Name`) whose components are REGISTERED as rollback state while the
    entity carries no rollback anchor — so the registration is a claim the
    engine does not honour. It exists only under the shell.
    ⭐ **MOVED TO [`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md)
    (2026-08-07)** — *"Is a SESSION scope marker construction provenance, the way
    a ROOM scope marker is?"* It stopped being an engineering question the moment
    it narrowed to one component, and the full write-up lives there so it is not
    restated in two places that can drift. Kept here in summary because the
    sweep is what found it.
    ⭐⭐ **NARROWED to one component, 2026-08-07.** Five of the six are in
    `PROVENANCE_ONLY` — `SpawnOrigin`, `TransactionId`, `SimId`,
    `RoomScopedEntity`, `Name` — so they are skipped by the RULE rather than by a
    waiver. The archetype is reported at all because of exactly ONE component:
    **`SessionScopedEntity`**, which is registered rollback state
    (`scope.session`, `primitives.rs:110`) and is NOT in that list.
    ⚠ **so the real question is an ASYMMETRY, and it is a short one**: both
    `RoomScopedEntity` and `SessionScopedEntity` are write-once SCOPE MARKERS
    stamped at construction, and the provenance rule's own argument — *"written
    ONCE and never again … a rewind that does not restore them restores exactly
    the values they already hold"* — reads identically for both. Is the omission
    deliberate? ⛔ **there is a reason to think it might be**: the sibling waiver
    for `Messages<SessionScopeRetired>` says a rewind *"must not un-retire a
    scope"*, so session lifetime has a rewind rule that room lifetime does not.
    Whether that rule reaches the MARKER as well as the retirement message is the
    whole question, and it is Jon's or the rollback owner's.
    ⛔ **and the sweep cannot currently see it.** Both
    `no_snapshot_registration_is_inert_*` assertions PASS, so the archetype does
    not appear in the boot world or a live match — it is in a composition no test
    sweeps. Naming the entity (done today) helps only once a test reaches that
    composition; until then the sweep is green about a class it never looks at.
    ✔ **the instrument NAMES the entity now (2026-08-07), so that probe never
    has to be written.** `inert_registrations` keyed each archetype to
    `names.intersection(&anchors)` — which is PROVABLY EMPTY at that point, since
    the loop `continue`s a few lines up whenever that intersection is non-empty.
    Every reported archetype carried an empty set beside it: the failure named a
    SHAPE and could never name a THING. It reports the entity's `Name` (deduped,
    so 40 copies of a prop stay one line), which is exactly what the next
    investigation below was told to go and print.
    ⚠ **narrowed by probe (2026-08-06, throwaway, not kept):** the CLI-built
    app — which composes the shell as of edit 1 — has **ZERO** construction
    roots without a body after settling. So the archetype is NOT a plain
    consequence of shell composition; it appears under the HARNESS's composition
    specifically (rollback enabled, a named room). The next investigation should
    probe inside `Platformer2dSimHarness` with the flip applied and print the
    entity's `Name`, rather than re-deriving that the shell is involved.
  ⭐ **the remaining work after that is a HEADLESS SHELL COMPOSITION** — routing
  and activation without the provider presentation — if the anchor question does
  not dissolve it.
  `compose_ambition_shell_host` is already headless in the sense that visuals
  are separate (`install_ambition_shell_visuals`), but the PROVIDER plugins it
  adds are not. That is the next design step, and it is a smaller and much more
  precise question than "migrate the harness".
  ▢ **edits 2-4 remain BLOCKED, and now for a measured reason.**
  The build-time root is not dead: `run_headless` and
  `Platformer2dSimHarness::build` compose `AmbitionGameSimulationPlugin`
  WITHOUT the CLI, so they never insert `AmbitionShellHosted` and still get
  their root at build time — and the four `direct_entry` gates are still true
  for exactly those apps. Deleting either now changes headless behaviour rather
  than removing a dead path. **The remaining work is migrating those two
  composers to compose the shell**, which the settle helper already prepared
  them for; the agreement test then says the room did not change.

  **Suggested staging:** (K2b.1) land the settle helper + migrate
  `headless`/`rl_sim`/`capture_scene` to compose the shell and settle, keeping
  the build-time root as a fallback and proving both paths agree; (K2b.2)
  delete the build-time root and the four `direct_entry` gates; (K2b.3) delete
  `AmbitionShellHosted` / `shell_host::direct_entry` once nothing reads them,
  and fold `PlatformerExperienceAuthoring::with_world_manifest` in (the K2a
  remainder above) while that builder is already open.

**Bounded hygiene** — [sonnet unless noted]
- ✅ **RESOLVED 2026-08-07, and two of its three parts are no longer the right
  work.** The row prescribed three things for "the smallest inventory smoke worth
  keeping" (`fable-reply-2026-07-19-b.md` §3–4). Checked each against the tree
  before doing any of them:
  * **widen the population by `BodyKinematics` — LANDED**, and went further than
    prescribed. `simulated_population` has THREE sources now, the third being
    vocabulary-derived (anything carrying a type the rollback registers). The
    review explicitly dropped registry-derived queries as a non-goal; the code's
    own comment justifies it with a gap the two tags provably cannot reach — a
    moveset strike volume lives six frames and carries neither tag, so no number
    of extra rooms reaches that family.
  * **per-filter anti-vacuity — LANDED TODAY, at a different granularity, and the
    difference is a measurement.** The review asked that each of the two filters
    assert ≥1 match. That was right when the helper served ONE boot room, where a
    filter matching nothing could only mean a broken filter. It serves TEN rooms
    now, and asserting it revealed that **`portal_lab` authors no
    `FeatureSimEntity` at all** — a legitimate room, not a broken fixture. So the
    floor asserts what is true of every fixture: a body exists, and the union is
    non-empty. ⛔ this was load-bearing and unguarded: poisoning the population to
    empty fails **11 of the 17** tests in the file, and every one of them would
    have passed silently before, because an empty population produces an empty
    unaccounted-list which is exactly what a clean sweep produces.
  * ⛔ **sequester into `tests/ambition_agent_guardrails/` — NO LONGER CORRECT, and
    doing it would invert the rule it came from.** Guardrails are agent tooling and
    are sequestered; product architecture is not. When this row was written the
    thing was one smoke test. It is now **19 tests in 2130 lines** verifying the
    ADR-0023 determinism contract across ten rooms, a live match, a mounted pair
    and a transient strike volume. Filing that under agent tooling would move the
    rollback correctness sweep out of the product's own verification. The rename
    to `rollback_inventory_smoke` goes with it: the file stopped being a smoke.
- ✅ **DONE** (verified 2026-07-24) — Kill the vacuous projectile-anchor
  `.all()` (`desync_canary.rs`) + ONE strong mutable-state rewind canary.
  `assert_family_anchored` now REFUSES to pass on an empty family (asserts the
  marker population is non-empty before counting unanchored, so a family with
  no members can no longer "pass" vacuously). The strong mutable-state rewind
  canary is the track-0 exit oracle (`rollback_exit_oracle.rs`, Phase 5 task 4):
  melee hit + armor spend + switch flip + brick break survive a forced rollback
  window checksum-identically. No scenario matrix — the old Track-0 exit list is
  opportunity, not contract.
- ✅ **DONE 2026-08-07** — `AGENTS.md`, "Landing when somebody else holds
  `main`", written from the situation it governs (this run works a worktree while
  another agent owns `main`). The rule and two `git` one-liners, no script: the
  reply this came from scoped the checker to "only if a second stale-base incident
  happens despite the recipe", and a checker now is exactly the machinery the very
  next section of that file forbids. ⚠ scoped to PARALLEL landings explicitly, per
  the same reply — *"imposing it on solo linear sessions on main adds friction
  where the failure mode cannot occur"*. ⛔ and it states that overlays are NOT
  banned, because the review flagged that phrasing: the forbidden operation is
  committing a stale tree SNAPSHOT without replaying its edits, not the delivery
  mechanism.
- ◐ **The ONE deletion-heavy docs pass** (tracks→open cards; smells refiled both
  directions; AGENTS ONE-BODY map extracted; archive provenance writer+validator;
  `run_source_analysis.sh`; reviews README) — then STOP.
  ✔ **`run_source_analysis.sh` and the reviews README already EXIST** — checked
  before doing anything, both present.
  ✔ **AGENTS ONE-BODY map EXTRACTED, 2026-08-07** →
  [`docs/concepts/one-body-one-path.md`](../concepts/one-body-one-path.md),
  following the `hall-of-characters-is-not-special` template this file already
  uses (a tight section in `AGENTS.md`, the detail and the rejected shortcuts in a
  concept page). The bullet went **2820 → 1446 characters**.
  ⭐ **the split is RULE vs STATUS, and that is the reusable part.** The bullet
  mixed a timeless rule (the smell test; "a green test on a forked path is
  worthless"; log the remainder as `BIFURCATION:`) with an INVENTORY of what
  happens to be unified today — melee's six symbols, the movement driver, the
  two-clock blink, what stays deliberately separate. The rule belongs where
  everyone reads it; the inventory goes stale and belongs in a page with a
  `last_verified` date. Nothing was dropped: every claim is in one file or the
  other.
  ⚠ **why this one was worth doing rather than any other line in `AGENTS.md`**:
  the file calls it *"the most-violated rule"*, and it was presented as a single
  unbroken ~2800-character paragraph in the cold-start doc. A rule nobody can
  finish reading is a rule that gets violated.
  ▢ still open: tracks→open cards, smells refiled both directions, archive
  provenance writer+validator.

**Design-before-code**
- ◐ **Cutscene authority — the DETERMINISTIC ELAPSED half landed 2026-08-06.**
  `tick_active_cutscene` read `Res<Time>`, which is the wrong clock twice over:
  the system runs in the sim schedule, and `sim_schedule()` IS `Update` under the
  `RenderFrame` host — so beat timings depended on how fast the machine drew
  frames, and two replays of one input stream could enter different beats. Under
  a fixed or GGRS host the frame clock happens to be deterministic, which is
  precisely the accident that hides a bug from the tests that would catch it.
  It advances on `WorldTime::sim_dt` now, which is also SCALED: a cutscene under
  slow motion slows with the scene instead of running at wall speed over a world
  in treacle.
  ✔ **the EDGE half is already correct** — checked rather than assumed.
  `update_cutscene_request_from_menu` treats advance as an edge (`menu_frame.select`)
  and accumulates the skip hold locally, letting only the completed threshold set
  `skip_cutscene`. That is what the row prescribes, already built.
  ⛔ **but the SPLIT is wrong, and this is the row's real content.** The whole
  `ambition_cutscene::` namespace is waived in `rollback_coverage` as *"scripted
  presentation sequence state"* — and it is not. `ActiveCutscene::is_playing()`
  drives a CAPTURING input-context claim (`CUTSCENE_CONTEXT`, priority
  `context_priority::CUTSCENE`), so while a cutscene plays the participant's
  gameplay input is suppressed. **Whether the player can act is gameplay truth.**
  A rewind into a frame where a cutscene was playing does not restore
  `ActiveCutscene`, so the resimulation can let the player act through beats they
  could not — a desync class the waiver's own wording hides.
  ⭐ **the shape, then:**
  * **SIM (registered, rewound):** `beat_index`, `elapsed` (already on the sim
    clock as of 2026-08-06), `finished`, and the identity of the playing script.
    These decide input capture and when the world resumes.
  * **PRESENTATION (derived, never rewound):** which portrait is drawn, fades,
    letterboxing, the partially-held skip accumulator — `skip_hold_seconds`
    belongs here and is currently on the crossing request struct.
  * **the crossing is two EDGES only:** "advance" and "skip completed". Both
    already exist; what does not exist is the registration of the first bullet.
  ✔ **IMPLEMENTED 2026-08-06.** `ActiveCutscene` is registered as
  `cutscene.playback`, OPTIONAL-canonical (a composition without cutscenes
  carries no such resource, and the plain form's checksum system panics on a
  missing one — the lifecycle domain next door learned that by turning eight
  oracle tests red). Schema v10 → v11: a v10 peer cannot reconstruct whether the
  participant was allowed to act. The blanket `ambition_cutscene::` waiver is
  now inert for this type because registration wins over it.
  ⭐ **and registering it caught a regression I had shipped an hour earlier.**
  Gating `Material2dPlugin` on a render app made `Assets<MaryOQuasarMaterial>`
  genuinely absent headless, and `attach_quasar_overlays` took it as `ResMut` —
  its run condition named three of the four collections its systems use, while
  the module's own doc claims it names *"exactly"* them. Found by the
  shipped-composition resource sweep, not by me.
  ✔ **the split is COMPLETE (2026-08-06).** `skip_hold_seconds` left
  `CutsceneAdvanceRequest` for its own `CutsceneSkipHold` resource: the request
  now carries two EDGES and nothing else, and the accumulator on the way to one
  is input-local state the HUD draws and the sim never reads. ⚠ it accumulates
  WALL time deliberately — a player holding a button for 1.2 seconds means 1.2
  seconds of their life, not of a slow-motion world's, which is the opposite of
  the `elapsed` decision one bullet up and the reason the two live apart.
- ✅ **LANDED — verified clause by clause 2026-08-07, having been written as
  design-before-code for a design that is now code.** Every part of the
  prescription exists:
  * **semantic-playback state shape** — `CutsceneRuntime { script, beat_index,
    elapsed }`, and `ambition_cutscene/src/lib.rs:65` states the property the row
    was asking for in the row's own terms: *"it is a pure function of `(script,
    beat_index, elapsed)`"*.
  * **vs derived presentation** — `ActiveCutscene { runtime, presentation }`, with
    `presentation` documented as *"a cache of a pure function, and the tick that
    advances `runtime` is the only writer"*. Stored rather than computed for a
    stated reason (Bevy readers cannot allocate a projection per frame), which is
    the derived-not-authoritative shape the row wanted.
  * **deterministic elapsed** — landed 2026-08-06, one bullet above.
  * **hold-to-skip stays a local accumulator, only the completed edge crosses** —
    landed 2026-08-06; `CutsceneSkipHold` is input-local and the comment at
    `update_cutscene_request_from_menu` says exactly that.
  ⚠ **one deliberate deviation, and it is the better answer.** The row specified
  Option A, *modes ride `ControlFrame`*. The edges ride **`MenuControlFrame`**
  instead — the menu-intent frame, not the gameplay one. A cutscene is not
  gameplay input, and routing a skip through the gameplay frame would put it in
  the channel that gets suppressed in UI modes.
  ▢ **what is genuinely NOT done, stated so it is not mistaken for done**: the row
  says *"through participant input"*, and the skip reads the GLOBAL
  `MenuControlFrame` rather than a seat's. So any seat skips for everyone, with no
  attribution — the same question R2 answered for dialogue. ⭐ **but the answer is
  probably different here, which is why this is a note and not a row**: a
  conversation has a TALKER and a cutscene has an audience. Everyone watching a
  shared cutscene has an equal claim to end it, so global may be correct rather
  than merely unimplemented. Worth one sentence from Jon before anyone builds
  per-seat skip attribution on the assumption that dialogue's answer transfers.

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
