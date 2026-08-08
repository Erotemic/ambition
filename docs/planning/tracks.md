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

⛔ **`▢` MEANS OPEN, AND IT IS THE ONLY INDEX.** Six sections here were open
assignments carrying no mark at all (2026-08-07) — §4, §6, §7, §8, §9 and
Parallel maintenance — so an agent grepping `▢` to find work missed every one of
them while reading 700 lines of finished narrative. If a section has work left,
its heading carries `▢`. ⚠ and the converse bit too: `▢` was being used for "a
point I am making" as well as "a thing to do", which is what made a count of
marks useless in the retired queues. One meaning only.

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
- **SR-3 / TwinTrack causal-pursuit candidate:** observer-local aim, exact constant-velocity null interception, a light-delayed moving target, swept pursuit hits, and lab/dual/optical-focus presentation modes. The challenge requires aiming at the future intercept rather than the apparent source. Local compile and visible-feel validation remain the candidate gate.
- **SR-4 / TwinTrack Relativity Plaza candidate:** replace the one-dimensional rail/debug-panel experience with permanent proper-velocity 2D free flight, clock-bearing characters, identity/payload/destination-bearing light dialogue, Doppler dance, light tag, full-world lab/optical modes, explicit high-school terminology, and a labeled 2+1D spacetime sculpture. Jump and flight-toggle controls are absent by capability rather than hidden in presentation. Local compile, scripted plaza acceptance, and visible classroom-feel validation remain the candidate gate.
- **SR-5 / TwinTrack Relativity Festival polish candidate:** add a guided synchronization/drift/acceleration opening, world-space proper clocks, labeled light-message packets, a continuous G2→G3 Doppler instrument, three progressively less-assisted light-tag rounds, and a scrub-able post-reunion spacetime replay. No new simulation authority or universal overhead is introduced. Local compile, scripted festival acceptance, and visible classroom-feel validation remain the candidate gate.
- **SR-5 / optical gameplay:** extend the observatory facts toward sensor fusion, simultaneity-sensitive gates, accelerated targets, and eventually light-delayed perception of extended scene geometry; keep exact compact-source claims separate from illustrative screen-space effects.
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
   ⛔⛔ **AND THE INSTRUMENT MEASURES THE ENGINE FLOOR, NOT AMBITION'S LADDER
   (found 2026-08-07).** `ladder_probe` rosters `duelist_l{N}`, whose archetypes
   read `brain_template: Fighter, fighter_level: Some(N)` — so they resolve
   through the same path a shipped fighter does. But the binary composes only
   `CausalPlugin`; it never installs `AmbitionContentPlugin`, so the
   `AuthoredFighterLadder` resource is absent and every rung falls back to
   `FighterBrainProfile::for_level`.
   ⭐ **the two differ exactly where the probe's own A/B lives.** `for_level`
   turns `rollout_depth: 12` on at level ≥ 6 — which is why the probe's comment
   says *"level 5 → 6 is NOT a depth experiment; it is five changes at once"* —
   while `fighter_brain_ladder.ron` authors `rollout_depth: 0` on EVERY rung,
   deliberately, gated on FB6e's instruments. So under the shipped ladder there is
   no depth change between rungs at all, and that confound is a property of the
   FLOOR rather than of the game.
   ⛔ **CORRECTION, same day: the remedy above was wrong, and the reason is the
   more useful finding.** I first wrote that the fix is "one `insert_resource`
   now that the ladder loads". It is not. `ladder_probe` lives in
   `ambition_demo_smash_app`, whose manifest states the rule: *"A demo app depends
   on `ambition_platformer2d`, never on `ambition_app`. That is the demo gate."*
   `fighter_brain_ladder.ron` is **AMBITION's** content. Installing it into a
   smash-demo binary would be one game reading another game's ladder, and the
   dependency the fix needs is the one the gate forbids.
   ⭐ **so the probe measuring `for_level` is CORRECT, and the doctrine says so.**
   `fighter-brain.md` §4: *"Games/demos ship their own rows — it's content."* The
   smash demo ships none, so its fighters get the engine floor, which is exactly
   what a game that has authored no ladder should get.
   ▢ **what is actually owed, restated**: decide WHOSE ladder the gate calibrates.
   If Ambition's, the probe belongs where Ambition's content is reachable. If
   smash's, **smash has to author one first** — and then its rungs, not Ambition's,
   are what §8's suite measures. ⚠ either way a numbers table that does not name
   the ladder it measured is worse than none, and today it would have said
   "Ambition" while measuring the engine default.
   ✔ **that half is FIXED 2026-08-07**: `ladder_probe` prints its ladder before
   any row — *"LADDER: engine floor (`FighterBrainProfile::for_level`) — this demo
   authors no ladder of its own"* — and states what it costs the table below
   (rungs gain `rollout_depth: 12` at level ≥ 6, so the level column confounds
   depth with reaction/APM/noise/read-weight; the forced-depth A/B is the only
   clean depth comparison there). ⭐ the same table means two different things
   depending on whose ladder ran, and it now says which. ⚠ its doc comment also
   records why this is NOT fixed by loading Ambition's ladder, because that is the
   fix the floor line invites and the demo gate forbids.
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
- ✅ **FS2/FS3 sand slice** — narrative archived 2026-08-07 to [`../archive/tracks-landed-sections.md`](../archive/tracks-landed-sections.md).
- ✅ **PARTICIPANT-CENTERED INPUT, startup/launcher vertical slice** — narrative archived 2026-08-07 to [`../archive/tracks-landed-sections.md`](../archive/tracks-landed-sections.md).
- ✅ **K1a movement tuning** — narrative archived 2026-08-07 to [`../archive/tracks-landed-sections.md`](../archive/tracks-landed-sections.md).
- ✅ **K2a world-manifest parameterization** — narrative archived 2026-08-07 to [`../archive/tracks-landed-sections.md`](../archive/tracks-landed-sections.md).
- ✔ **K2b direct-entry activation — DONE 2026-08-06.** The hand-built
  `SessionRoot` is deleted; `compose_ambition_gameplay_host` is the one
  composition, green across the `ambition_app` suite and the workspace policy
  job. ⭐ the order inside it — `AmbitionShellHosted` → simulation plugin → shell
  — is the part worth remembering. **319 lines of execution narrative archived
  2026-08-07** to [`../archive/tracks-landed-sections.md`](../archive/tracks-landed-sections.md);
  its two open rows are kept here in full because they are the live part.

  ▢ **K2b-i — `SessionScopedEntity` is registered rollback state and no scope
  marker's sibling is.** `no_snapshot_registration_is_inert_*` reports an
  archetype whose components are registered while the entity carries no rollback
  anchor. Narrowed 2026-08-07 to exactly ONE component: five of the six sit in
  `PROVENANCE_ONLY`, and `SessionScopedEntity` (`scope.session`,
  `primitives.rs:110`) does not. ⚠ the question is an ASYMMETRY — `RoomScopedEntity`
  and `SessionScopedEntity` are both write-once markers stamped at construction —
  and it is **Jon's**, written up in full at
  [`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md) (*"Is a
  SESSION scope marker construction provenance, the way a ROOM scope marker
  is?"*). Summarised rather than restated so the two cannot drift.

  ✔ **K2b-ii — CLOSED 2026-08-08. Every entry point composes the shell**, and the
  last one was already composing it when this row said otherwise.
  * `headless.rs:132` — migrated (`compose_ambition_gameplay_host`).
  * `rl_sim/mod.rs:84` — migrated (K2b edit 2); the build-time publisher is gone.
  * `capture_scene.rs` — migrated **2026-08-06** in `9266bdca9`, not 08-08. The
    08-08 re-measurement cited `:286`, which is the `add_plugins` line; the
    composition call twelve lines below it was already
    `compose_ambition_shell_host_booting_to(…, AMBITION_GAMEPLAY_ROUTE)`.
  ⛔ **the warning below was written and then violated by its own author in the
  same paragraph** — "read the composition call" is right, and citing the import
  site is what re-derivation looks like when it is wearing the fix.
  ⛔ **do not re-derive this with a grep for `AmbitionGameSimulationPlugin`** —
  the *import* survives migration, and so does the `add_plugins` line in any
  VISIBLE composition, which must still name the simulation plugin to interleave
  presentation before the shell. That search points the wrong way twice.

  ⭐ **What the row was actually for, and it paid off: the migration had eaten the
  room.** K2b edit 5 composed the shell and inherited the activation lifecycle,
  but nothing installed `install_ambition_shell_visuals` — the only registrar of
  `SessionRoomVisualsPlugin` and `ambition_activate_session_visuals`, which spawn
  parallax, static room visuals, signage and the LDtk spine ON ACTIVATION. The
  Startup path that used to do it had been deleted with the `direct_entry` gate
  (K2b edit 3). For two days the phone proxy photographed a **void with a HUD on
  it** — exit 0, valid PNG, player + NPC + HUD + touch bezel all drawn, no world —
  because everything that hangs off the SESSION still worked and only what hangs
  off the ROOM did not.
  ⚠ **this is the param-panic class arriving silently.** The same fork panicked
  twice before (`VisualQualityPlugin` 2026-07-31, `sync_portal_quality_budget`
  2026-08-04, both recorded in `plugins.rs`) and was caught in minutes each time.
  It did not panic here because the missing piece was a SPAWN system, not a `Res`
  reader. **A composition that half-runs is worse than one that refuses to.**
  ✔ **THE FORK IS DELETED (2026-08-08, same day).** The item named below —
  a pre-simulation hook on `build_visible_app` — is
  `build_visible_app_with(render, shell_hosted, |app| …)`, one closure run after
  `AmbitionShellHosted` and before the simulation plugin builds, which is the
  deadline every composition input has. `build_visible_app` is now that function
  with an empty closure, so there is exactly ONE visible builder in the tree.
  `capture_scene` calls it for a ROOM and for a ROUTE; the two differ by the
  boolean that picks the initial route and by which capture systems get added,
  and by nothing else. Its second app builder, its copy of `desktop_asset_root`,
  its `pin_the_clock`, its guarded `HostGameplayPresentationPlugin` add and
  `--show-window` (which only ever opened a blank window — every camera is
  retargeted to the offscreen image) are all gone: **−288 lines of composition,
  +36 of hook.**
  ⭐ **four of the five drifts are now STRUCTURALLY impossible**, because there is
  no second composition to forget: the display surface, the shell visuals, the
  audio/persistence redirect and the asset root are stated once and inherited.
  `--dev-overlays` and `--combat-overlay` remain possible — they are *systems*,
  and `install_room_capture` / `install_route_capture` still list theirs
  separately, which is the irreducible half (a room needs the snapshot
  applier, a route needs camera adoption).
  ⚠ what the shared builder ALSO brought, unasked and correct: room captures now
  run the GGRS simulation host, `serialize_frame_schedules`, and the `game://`
  asset source, none of which the hand-assembled app had. Measured: the sim pose
  is byte-identical (`950.0000, 904.0000` after 12 warmup ticks, both runs), and
  27 pixels of the robot's foot differ by one animation step.
  ⚠ and one thing it took away, put back deliberately: `OffscreenGpu` disables
  `LogPlugin` (right for tests that build several Apps per process, wrong for a
  binary that builds one), which silenced every engine `INFO`/`WARN` a room
  capture used to print. `capture_scene` re-adds it after the group, for both
  modes — route mode had been running without them since it was written.

  The historical reasoning, kept because the *next* tool will hit it: there was no
  composer a room capture could call. `compose_ambition_gameplay_host` is the
  sim-only shorthand (a visible host must build presentation between the
  simulation and the shell, which is why `build_visible_app` also spells the three
  steps out), and `build_visible_app` could not be used because `StartRoomOverride`
  / `StartRoomMustResolve` / `StartingCharacterOverride` are consumed at
  PLUGIN-BUILD time by `init_sandbox_resources`. So `capture_scene` hand-assembled,
  and that hand-assembly silently ate five things: `--route` as a positional, the
  headless display surface, `--dev-overlays`, `--combat-overlay`, and the room.
  ⚠ **two different things are named `direct_entry`**: `shell_host.rs:51` records
  its own as already deleted, while `cli.rs:245,271,886` carries a live
  `cli_direct_entry()`. K2b.2/K2b.3 below refer to the first.
  **Staging, as it now stands:** (K2b.1) migrate `capture_scene` and prove both
  paths agree; (K2b.2) delete the build-time root and the remaining
  `direct_entry` gates; (K2b.3) delete `AmbitionShellHosted` once nothing reads
  it, folding in `PlatformerExperienceAuthoring::with_world_manifest` while that
  builder is open.

**Bounded hygiene** — [sonnet unless noted]
- ✅ **The smallest inventory smoke worth keeping — RESOLVED 2026-08-07**, and two of its three prescribed parts turned out not to be the right work. Narrative archived to [`../archive/tracks-landed-sections.md`](../archive/tracks-landed-sections.md).
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
  ◐ **smells refiled — TWO large entries closed and the file MEASURED, 2026-08-07.**
  Both remaining `BIFURCATION:` entries are resolved, each verified by checking
  the symbols they name rather than re-reading the prose: the 2026-06-28
  player-vs-actor MELEE fork (`ActorAttackState` gone; `PlayerAttackState`,
  `attack_advance_system` and even its own proposed remedy `spawn_melee_hitbox`
  survive only in comments recording their deletion), and the 2026-07-19
  `is_player` FEEDBACK fork (shipped as CM8 with `HurtFeedback::ENEMY` as the
  victim profile — the elegant resolution the entry asked for, plus Jon's
  per-attack binding).
  ⭐⭐ **and the file does NOT need a sweep, which is the measurement worth
  keeping.** 92 entries, 29 marked resolved. Sampling three open ones —
  `.cargo/config.toml` hardcoding a home target dir, `regen_sprites.sh` restating
  the renderer's sheet list, vestigial bfs sand plumbing — found **3 of 3 STILL
  TRUE**.
  ⭐ **that is the exact opposite of the carried queues** (4 of 4 already closed),
  and the reason is structural: **a smell entry describes a PROPERTY OF THE CODE**
  — "this file hardcodes a path" — which stays true until someone fixes it. **A
  queue row describes WORK TO DO**, which goes false the moment anyone does it,
  usually somebody who updated the code and not the row. So a smells journal ages
  WELL and a queue ages BADLY, and they want opposite maintenance: refile a smell
  when its code changes, re-check a queue row before believing it.
  ⚠ this is the same distinction `docs/planning/README.md` now states as
  citation-vs-situation, arriving from the other direction: the journal is
  naturally citations because an entry names a file and a property.
  ▢ still open: tracks→open cards, archive provenance writer+validator.

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

- ✔ **0. Pay down the GGRS correctness debt — LARGELY LANDED 2026-07-19.** Narrative archived to [`../archive/tracks-landed-sections.md`](../archive/tracks-landed-sections.md).

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

## ▢ 4. Close one complete Sanic act — [opus]

Corrected 2026-07-19: the ring economy (35 authored rings, collect SFX) and the
badnik enemy loop (stomp-with-bounce AND roll-through defeat through shared
contact/combat) are **landed** — the old "bits" and "one enemy loop" bullets
are done. The remaining work is the list in [`demos/sanic.md`](demos/sanic.md),
which already declares itself the single source — **refer, never copy** (these
are the copies that "drifted independently — again").

Do not absorb movement/contact work owned by another active campaign.

## 5. Provenance + three-origin `ConstructionPlan` vertical slice — **LANDED 2026-07-22**

Landed; moved verbatim to [the archive](../archive/tracks-landed-sections.md).

## ▢ 6. Correct the fighter-rollout design before FB6 — [fable]

Unchanged, and now urgent-if-touched: FB6 as written depends on the DELETED
snapshot engine (`snapshot.take/restore`) and is unimplementable at HEAD.

- prefer a fixed work budget over a wall-clock cutoff (or make decisions
  recorded external inputs);
- rollouts need a hypothetical state reconstructed solely from allowed
  `Perceived` facts, or a deliberately limited perceived-state forward model —
  now necessarily built on GGRS-era machinery.

**Exit:** the determinism and no-cheat contracts are explicit enough that an L3
implementation cannot accidentally violate either one.

## ▢ 7. Role evictions from the sim heart — [opus]

Deep-review §6 remains useful as the first set of role-driven evictions, but
the old "no size-driven carve" ruling is retired. These tasks now serve the
active incremental actor-monolith campaign in
[`engine/actor-monolith-decomposition.md`](engine/actor-monolith-decomposition.md):

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

## ▢ 8. Combat unification batch — [opus] (first item LANDED 2026-07-19)

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

- ~~name the strike victim (`StrikeVictim` query data)~~ **DONE 2026-08-08** —
  the first slice promoted out of
  [`triage/bevy-system-parameter-architecture.md`](triage/bevy-system-parameter-architecture.md)
  (recommendation 1, Case A's own named candidate "damage victims"), and the
  direct continuation of the projectile victim-loop unification above.
  **Systems:** `ambition_combat::hitbox::apply_hitbox_damage` (melee) and
  `ambition_platformer2d_actor_monolith::projectile::systems::step_projectiles`
  (one of the 12 systems at Bevy's 16-param ceiling, and one carrying an explicit
  tuple-slot workaround) now iterate ONE named role,
  `ambition_combat::hitbox::StrikeVictim`, instead of two positional tuples.
  **Deleted:** the 10-member and 7-member victim tuples; the nested
  `Option<(BodyOffense, BodyMotionFacts, BodyShieldState, BodyCombat)>` arity
  workaround; **both** `victim_frames: Query<&ResolvedMotionFrame>` lookup params —
  a per-victim component that had been reached by a second query in each system
  through byte-identical `.map(|f| f.basis()).unwrap_or(default)` ladders, now
  `StrikeVictimItem::knockback_side` once. `apply_hitbox_damage` 11 → 10 params;
  `step_projectiles` 19 → 18 real params (its packed tuple 4 → 3 members, still
  packed — the ceiling is not the point, see below).
  **Invariants:** the victim SET of each system is unchanged. Melee's four
  REQUIRED cluster members were bound to `_vuln` and never read (since §A2 i-frames
  resolve in `resolve_body_hit`), so they became a `With<..>` filter — `With<T>`
  and `&T` match identical archetypes, and the narrowing is now visible at the call
  site instead of buried in whether one tuple wrote `Option<(..)>`. One victim loop
  per family, one relational rule (`damage_lands*`), `is_player` still picking only
  payload policy.
  **Measurements:** all access added by the shared view is READ-ONLY, so no new
  mutable conflict is possible — melee gains a `BodyPresentationSource` read;
  projectiles gain `DamageableVolumes`/`BodyHealth`/`CombatTuning`/`MatchTeam` and
  drop `BodyOffense`/`BodyMotionFacts`/`BodyCombat`; `ResolvedMotionFrame` is net
  unchanged in both (it moved from a standalone query into the view).
  B0001 is a RUNTIME panic and no build catches it, so the headless gate is the
  probe: `app_it` 320 passed / 0 failed / 10 ignored before (199.7 s) and after
  (211.0 s); `ambition_combat` 142 and the monolith 1213, both 0 failed.
  **No new test.** The invariant this slice owns is "the victim set did not
  change", which the existing combat suites already assert behaviourally — and
  this document's own test strategy rules out the alternative: *"source-text tests
  that merely count parameters or demand a specific type name are not
  appropriate."*
  ⭐ **What it exposed, which is the real yield:** `step_projectiles` never
  consulted `DamageableVolumes` at all — it tests the coarse `CenteredAabb` while
  melee and feature hits both ask `strike_reaches_victim`. Its comment nonetheless
  claimed it shared "the SAME published hurtbox" as melee. **A published silhouette
  — an authored hurtbox timeline, a boss's active parts, an EMPTY list meaning
  intangible — decides whether a sword lands on a body and has never decided
  whether a BOLT does.** The tuple that would have carried it had run out of arity,
  and `apply_feature_hit_events` documents that exact cause for its own workaround.
  Left OPEN below, deliberately: closing it is a combat behaviour change (it also
  retires `strict_intersects`, which rejects edge-touching where the shared rule
  accepts it) and does not belong riding on a structural commit.
- ▢ **make a projectile hit the silhouette its victim published**, i.e. swap
  `step_projectiles`' coarse-box test for `victim.reached_by(..)` — the one-line
  call the card above put within reach, plus the test that pins it and a
  re-baseline of whatever projectile-range expectations move. Authored
  invulnerability windows and boss part-hitboxes currently do not apply to
  projectiles at all.

**Exit:** no `is_player` branch selects an effect payload anywhere in combat;
a moveset volume can author its own strike/hit effects and the goblin swipe
sounds different from the sword.

## ▢ 9. Player-facing repairs (Jon's fix list) — [opus]

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
- ~~possession-aware dialog: speaker/listener identity derives from the actors
  in the conversation, not "the player"~~ **DONE 2026-08-08**, landed as a side
  effect of the GPT 5.6 review campaign rather than as this item. Verified:
  `interact.rs:118` takes `speaker_id` from the **subject body** — whatever is
  driving, possessed or not — and `:138` takes `listener_id` from the
  interactable's own character id. **No `is_player` or "the player" branch exists
  anywhere in the conversation path.** `conversation::opening::driving_slot`
  answers "whose body is this" from the `Brain`, so a seat that possessed an
  actor and walked it to an NPC is attributed correctly *without the module
  knowing possession exists*, and `ConversationInstanceId` carries **both**
  bodies' `SimId`s.
  ⚠ **what is genuinely NOT done is the second clause**: *model listener-side
  adaptation*. Identity is possession-aware; nothing yet makes an NPC say a
  different line because of who is wearing the body in front of it.
  `$speaker_is_self` is the one adaptation that exists.
- ✔ **`AMBITION_START_CHARACTER=sanic` — FIXED 2026-08-08, and it was not an
  ActionScheme question at all.** Traced in the composed host: *any* selection
  other than the experience's authored default failed the preparation barrier's
  `validate-provider-defaults` work item (`retryable(false)`), so the session
  **never activated** — no world, no body, no message. Sanic was only the id Jon
  typed; `goblin` and `capture_scene --character <id>` were equally dead.
  ⭐ **the seam: a provider's authored DEFAULT and a session's SELECTION are two
  facts.** The identical equality check had already been deleted from
  `prepare_platformer_content` on 2026-07-29 under a comment saying exactly
  that — but the twin in `PlatformerPreparation::prepare` runs FIRST and returns
  early, so the corrected copy was unreachable by the case it was written for.
  The barrier now owns only the audio-provider question; resolvability has ONE
  owner. Measured after the fix (`app_it -- starting_character_selection`, and a
  three-way probe): Sanic wears its own name and `SurfaceMomentum` row, runs
  449px/60t vs the protagonist's 265 and jumps 103px vs 83, carries **no**
  `ChargesProjectiles` and an empty moveset. So the *fireballs* half was already
  gone (the 2026-07-05 `overlay_character_moveset` deletion); *blink* is the home
  body's own traversal grant (dev `EditableAbilitySet`), not something the
  persona grants — that is the documented "the box keeps its traversal kit"
  design, and a separate product question if Jon wants it changed.

**Exit:** each item demonstrated in the real app (feel ships blind where
visual; behavior verified headless where steppable).

## 10. Gameplay presentation profiles — **LANDED 2026-07-20**

Landed; moved verbatim to [the archive](../archive/tracks-landed-sections.md).

## ▢ Parallel maintenance — [sonnet unless noted]

Small non-blocking work when it does not collide with the campaigns:

- ▢ **materialize `SessionSeatId` / `ControlChannelId` when seating/topology
  next moves substantially** [opus]. ⛔ **not a campaign to start on its own** —
  the GPT 5.6 review through `43373f72d` says so explicitly, and the
  behavioural defect underneath it is already fixed (`LocalChannelPlan`, which
  must not be undone). What remains is an identity conflation:

  ```text
  LocalInputSource   what somebody picked up          — sparse, separated ✔
  ParticipantId      the PERSON                       — outlives the session
  SessionSeatId      a seat in this session's topology — MISSING
  ControlChannelId   a deterministic input channel    — MISSING
  PlayerSlot         what the simulation reads
  ```

  One number carries three of those. The lifetimes genuinely differ: a
  participant survives relaunch, seat reassignment and possession; a channel
  belongs to one session's topology and dies with it. **The standing rule until
  they separate** — stated in `ambition_input/src/channels.rs` and
  `participant_seat.rs`, which is where somebody about to break it is reading —
  is that new code must not add ARITHMETIC equality between `ParticipantId` and
  `PlayerSlot`/a GGRS handle; route through `LocalChannelPlan`, so a future
  `ControlChannelId` replaces the spelling in one place instead of in every
  caller. ⭐ the review's own lesson, worth keeping: *a reproduction can be
  fixed while the identity model that enabled it remains conflated.*

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
