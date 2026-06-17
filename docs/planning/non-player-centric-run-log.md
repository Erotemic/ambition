# Non-player-centric actor unification — run log

Live progress for the autonomous run guided by
`non-player-centric-actor-unification.md`. Newest entries at the bottom of each
stage. Jon reviews the cumulative diff on `main`; this log is the trail + the
resume point.

**Decisions (confirmed):** improve player feel opportunistically · movement +
combat · bosses last · through Stage 6 (player → input+camera+HUD).

## Stage 0 — vertical slice (dash peeled into the composable `abilities` module) ✅

- [x] Mapped the movement monolith (verbs + ordering + cluster fields) via subagent.
- [x] Created `movement/abilities.rs` — the destination the monolith decomposes into.
- [x] Peeled `dash` into `abilities::apply_dash` (reads/writes only its own fields).
- [x] Replay byte-identical (pure refactor); 166 engine tests green.
- [x] Committed.

**Design refinement (recorded):** the engine movement is plain functions on a
borrowed `PlayerClustersMut` (deliberately Bevy-free for headless/test use), NOT
Bevy systems. So the decomposition is FUNCTION-level first: each ability becomes
an `apply_<verb>` in `movement/abilities.rs`, called by the integration in a fixed
order (the "phase pipeline" made explicit as call order). The ECS-system /
optional-component lift (true query pay-for-use) comes in Stage 2 (composable
body), once every verb is a clean self-contained function. This keeps each step a
pure, replay-guarded refactor.

## Stage 1 — decompose the player monolith ✅ (substantially complete)

- [x] **1a** — control-phase verbs → `abilities::apply_{intent,fly_toggle,dodge,
  shield,jump_release}` (+ dash from Stage 0). The control phase
  (`update_player_control_with_clusters`) is now an explicit ordered sequence of
  ability calls. (blink/attack remain as `control::handle_*` — already extracted;
  move into `abilities` later if worth it.)
- [x] **1b** — sim-phase mode-select made symmetric: normal branch →
  `integration::integrate_normal_clusters`, joining the already-separate climb +
  flight integrators. The shared physics SPINE is now one named function.
- Replay byte-identical at every step; 166 engine tests green.

**State of the sim phase:** already largely composed of function calls
(`handle_jump_buffer_clusters`, the X/Y sweeps in `collision.rs`,
`apply_wall_abilities_clusters`, the ledge subsystem). Remaining inline bits in
`integrate_velocity_clusters` are spine post-collision resolution (ground-refresh,
rebound) — minor; extract opportunistically. **Stage 1 is effectively done.**

## Stage 2 ✅ — the shared spine made actor-generic (the convergence seam)

**Architecture fork taken (mine, per the decision doc):** Stage 2 as literally
written said "collapse the 4 cluster query-datas toward one." I deliberately did
NOT do a cosmetic mega-merge of `PlayerClusterQueryData`/`Enemy`/`Npc`/`Boss` into
one struct. Reason: that fights the pay-for-use principle Jon explicitly endorsed
(a slug shouldn't carry 18 player components, nor sit in the rich queries), and the
per-actor-TYPE clusters (enemy attack state, NPC dialogue, boss encounter phase)
legitimately differ. The body is ALREADY shared (`BodyKinematics`); the real
convergence is the SPINE + opt-in ability components. So Stage 2 = make the
normal-mode spine actor-generic so a non-player can run the literal player physics
core, gated only by a tiny context.

- Extracted `integrate_normal_spine(&mut vel, &mut fast_falling, &mut gliding,
  NormalSpineCtx, input, dt, tuning)` from `integrate_normal_clusters`. The player
  adapter projects its rich ability clusters → `NormalSpineCtx` flags; the function
  body is unchanged (field renames only) → **player byte-identical** (replay green,
  166 engine tests green).
- `NormalSpineCtx { on_ground, blink_grace, water, can_fast_fall, can_glide,
  can_move_horizontal }` is the pay-for-use seam: `NormalSpineCtx::bare(on_ground)`
  is what an actor with NO player ability components presents (run + fall, nothing
  else). Both `pub`, re-exported from `movement/mod.rs`, so `ambition_sandbox` can
  feed enemies/NPCs through the SAME core in Stage 3.
- Commit: `feat(movement): Stage 2 — extract the actor-generic normal-mode spine`.

## Stage 3 — enemies/NPCs onto the shared spine

### 3-core ✅ — grounded enemies + NPCs run the LITERAL player spine
- `integrate_standard_enemy_body` (grounded branch) and `NpcRuntime::integrate_velocity`
  no longer hand-roll gravity + run + fall-cap. They build a per-actor `MovementTuning`
  (gravity, gravity_dir, `run_accel`/`air_accel = ENEMY_RUN_ACCEL`, friction 0,
  `max_run_speed = |desired_vel.x|`, `max_fall_speed = ENEMY_MAX_FALL`) and call the
  shared `ae::integrate_normal_spine(..., NormalSpineCtx::bare(on_ground), ...)` — the
  exact physics core the player runs. The collision sweep (`step_kinematic`) now runs
  with `gravity = 0` (the spine already applied gravity) — the same intent/sweep split
  the player uses.
- **Byte-identical under vertical gravity** (the mapping `axis_x = sign(desired)`,
  `max_run_speed = |desired|`, friction 0 reproduces the old `approach(.., 650·dt)`
  run exactly; the spine's gravity + fall-cap formula is identical to `step_kinematic`'s).
  916 sandbox tests green; 166 engine tests green.
- The duplicate *physics* is gone; `integrate_standard_enemy_body` / `integrate_velocity`
  remain as thin brain→body orchestrators (jump impulse, motion-path advance, patrol
  facing-flip, aerial branch) — that glue is per-actor and stays.
- Commit: `feat(physics): Stage 3 — grounded enemies/NPCs run the shared spine`.

### Still open in Stage 3
- **Aerial** enemies/NPCs (parrot) still use the bespoke 2D `approach` integrator
  (not the player FLIGHT spine). Folding them onto `integrate_flight_clusters` is a
  follow-up (flight tuning mapping); their behavior is already gravity-free + correct.
- **Content abilities** (slug surface-crawl, parrot dive-bomb): surface-crawl already
  exists as `surface_walker` behavior; promoting these to first-class ability COMPONENTS
  is content polish, deferred — it does not block the structural unification (Stages 4-7).

## Stage 4 ✅ — rendering tail unified

**Scope decision (mine):** the rendering MECHANISM is already shared — player,
enemy, and NPC all animate through `CharacterAnimator` + `Sprite` atlas + the
gravity-aware facing flip + the hit-flash silhouette overlay. The only thing that
legitimately differs between `animate_player` and `animate_characters` is the anim
SELECTION richness (`pick_player_anim` reads 18 player clusters for crouch / slide
/ ladder / blink / dash / ledge / dodge / shield; `pick_enemy_anim`/`pick_npc_anim`
read a small actor state). That difference is correct pay-for-use, NOT duplication
to merge — forcing the player into the `FeatureVisual` iteration would mean dragging
its rich clusters into the actor query for zero behavior gain and real visual risk
(can't GUI-verify). So Stage 4 = extract the duplicated frame-application TAIL.

- New `apply_character_frame(sprite, animator, anim, dt, facing, gravity_dir, color)`
  in `rendering/actors/animation.rs`: request anim → tick → push atlas frame →
  gravity-aware flip → set tint. Both `animate_player` and `animate_characters` now
  call it; the per-actor systems shrank to "gather state → pick anim → pick tint".
- Byte-identical by construction (same operations, extracted). Render crate builds
  clean; player tint stays `WHITE` (overlay flashes), enemy attack tint preserved.
- Commit: `refactor(render): Stage 4 — extract the shared actor animation tail`.

## Stage 5 ✅ — bosses onto the shared movement (the floating free-mover path)

Bosses were ALREADY on the shared body (`BodyKinematics`) + shared collision
sweep (`step_kinematic`) — they're floating actors whose pattern brain emits a
`desired_vel` each tick. So there was no gravity/run spine to fold them onto (they're
zero-g). The genuinely-special boss parts (the `BossPattern` brain, multi-volume
hurtboxes, encounter phases) correctly stay as boss components.

The real remaining MOVEMENT duplication was the "floating free-mover" shape, which
the boss, aerial enemies, and aerial NPCs (the parrot) each hand-rolled: smooth (or
snap) `vel` toward `desired_vel`, then sweep with gravity off. Unified into one
`features::step_floating_body(body, world, desired_vel, accel, max_fall, dt)`:
- **Boss** `integrate_body`: `accel: None` (snap to the pattern's exact velocity).
- **Aerial enemy / aerial NPC**: `accel: Some(900·dt)` (smooth approach).
- Byte-identical under normal gravity (the three already agreed on gravity-free,
  `gravity_dir = (0,1)`, `drop_through = false` for the floating sweep). 916 sandbox
  tests green. This is the floating counterpart to `integrate_normal_spine`: now an
  actor either RUNS (grounded spine) or FLIES (floating mover) through one shared
  function each, whether it's the player, an enemy, an NPC, or a boss.
- Commit: `refactor(physics): Stage 5 — unify the floating free-mover (boss+aerial)`.

## Stage 6 ✅ (substantially pre-existing) — player peeled to input+camera+HUD

**Finding:** Stage 6's core thesis — "the only genuinely player-specific things are
input, camera, and HUD/UI" — is ALREADY realized in the codebase (it predates this
run). Evidence:
- **Camera / HUD / fx / UI are scoped to `PrimaryPlayer`** via the `PrimaryPlayerOnly`
  query alias (`With<PlayerEntity> + With<PrimaryPlayer>`): `camera_follow`
  (`rendering/camera.rs`, doc already frames it as the primary target, multi-player-
  ready), the HUD (`app/hud.rs`), screen-space fx (`render/fx.rs`, `morph_ball.rs`),
  held-item visuals, and menu mana effects all filter on it.
- **Input flows through `ActorControlFrame` for BOTH** the primary (`engine_input_
  from_actor_control` in the sim phase) and the clone (`input_from_actor_control`) —
  the universal-brain seam. Possession = point a device at another actor's control.
- **Body state is `PlayerEntity`-scoped**, distinct from the `PrimaryPlayer`
  camera/HUD identity. So "player body" and "the viewport's player" are already
  separate concepts.

**This run's concrete Stage 6 change:** removed the open-coded
`gravity_field.as_deref().map_or((0,1), |g| g.dir)` idiom (repeated across the
player control tick, sim attack tick, hit tick, and the clone driver) into one
`physics::gravity_dir_or_default(field)` helper — the per-frame "read world gravity"
seam every actor tick shares. Replay byte-identical; player-clone live test green.
Commit: `refactor(player): Stage 6 — share the gravity-dir read; document the
realized primary/body boundary`.

**Deferred (documented, not done):** fully collapsing the remaining `single_mut()`
body/combat systems into ONE loop over every player-bodied entity (so the clone
drops its bespoke `drive_player_clones`). That requires peeling the global concerns
(moving-platform riding, hard-fall screen-shake, sandbox reset) OUT of
`player_simulation_phase` into primary-only systems — a wide, GUI-feel-sensitive
change the replay fixture only partially guards (it covers the primary, not the
multi-body topology). Not safe to land blind in an autonomous run; left as the
explicit next step with the boundary already drawn (`PrimaryPlayer` vs `PlayerEntity`).

## Stage 7 ✅ — combat: capability-gated projectile charge + the desired_vel contract

- **The `brain.is_player()` projectile-charge gate is gone.**
  `emit_player_projectile_tick_messages` now gates on a `ChargesProjectiles`
  capability marker (new, ambition_actor) instead of brain type. Only the player
  bundle carries it today → **byte-identical** (same single emitter; 916 sandbox +
  55 projectile/charge tests + replay green), but the charge mechanic is now
  pay-for-use and travels with the BODY, so possession keeps it. Enemies/bosses with
  a `ranged` ActionSet (their own projectiles) are correctly excluded — the marker is
  a distinct capability, not the ActionSet slot.
- **`desired_vel` dual meaning (P10) made an explicit contract.** Documented the two
  encodings (floating = velocity; grounded player = axis, grounded enemy = velocity)
  AND the BRIDGE that already unifies them: `integrate_standard_enemy_body` maps the
  enemy velocity onto the shared `integrate_normal_spine` (`max_run_speed = |vel.x|`,
  `axis_x = sign`). So both reach the same grounded spine and agree — the velocity
  encoding is a bridge, not a second path. The clean axis-only end-state (every brain
  emits normalized axis + run speed in tuning) is the deferred follow-up — a wide,
  enemy-feel-sensitive brain rewrite not worth the risk now that the bridge is sound.
- Commit: `feat(combat): Stage 7 — capability-gate projectile charge; document desired_vel`.

## Run complete (Stages 0–7)
All seven stages landed on `main`. The actor stack is now: ONE grounded physics
spine (`integrate_normal_spine`) + ONE floating mover (`step_floating_body`), shared
by player / enemy / NPC / boss; one rendering tail; the player reduced to its
genuinely-player-centric shell (camera/HUD/UI behind `PrimaryPlayer`, input via
`ActorControl`); and combat charge gated by capability, not identity. Two deliberate
deferrals are documented (the full single_mut→loop body merge, and the desired_vel
axis-only unification) — both are wide/GUI-risky and gated on work the replay fixture
can't guard blind; the seams for both are in place.

## Follow-up run (post-Stage-7) — finishing the deferrals, ordered to avoid rework

Plan (Jon, minimize redundant work): 1) decouple the globals from the player tick →
2) make the clone a full PlayerEntity body → 3) ONE sweep generalizing every
player-singleton system to iterate (drops `drive_player_clones`, fixes the K-clone
sprite for free) → 4) `desired_vel` axis-unification (needs an enemy-position parity
harness) → 5) content (aerial→flight spine, slug/parrot ability components).

### Step 1 ✅ — peel the platform ADVANCE out of the per-entity player tick
- The only genuinely SHARED mutation inside `player_simulation_phase` was
  `platform.update(sim_dt)` — it advances every moving platform. Left inside a tick
  that step 3 will iterate over N bodies, it would advance platforms N× per frame.
- Hoisted it into a once-per-frame step in the primary caller
  (`player_simulation_system`, in `player_tick.rs`), using the same `sandbox_dt`.
  `MovingPlatformState` now records `last_delta`; the per-entity phase READS
  `platform.last_delta()` for ride / ledge-carry and takes platforms as `&[]` (no
  mutation). Replay byte-identical (the ledge-match's 8px tolerance absorbs the
  sub-pixel pre/post-advance shift); 916 sandbox + 179 app tests green.
- **Note on shake + reset:** the hard-fall screen-shake and sandbox-reset are also
  primary-only, but they are CONSEQUENCES of the per-entity sim, not shared
  mutations — they gate trivially with `is_primary` when the caller becomes a loop
  in step 3. Peeling them now then re-touching the phase in step 3 would be the
  redundant work Jon flagged, so they stay put until step 3 (the natural seam).
- Commit: `refactor(player): step 1 — platform advance is once-per-frame, not in the per-entity tick`.

### Step 2 ✅ — the clone is a full VISUAL player body (it has a sprite now)

This is also the fix for "why does the K-spawned clone have no sprite": it was a
placeholder colored box, never wired into the character-sprite path.

- **Clone spawn** (`player_clone.rs`) now attaches the real textured player sprite +
  `CharacterAnimator` + `PlayerSpriteBaseline` + feet anchor (mirroring
  `scene_setup`'s primary visual; falls back to a tinted box if the sheet didn't
  load — headless), plus `PlayerAnimState` / `PlayerCombatState` /
  `PlayerBlinkCameraState` + the `PlayerVisual` marker.
- **`animate_player` generalized** from a `get_mut(entities.player)` single lookup to
  a loop over every `With<PlayerVisual>` body, with per-entity
  `Option<&ActivePlayerAttack>` (primary keeps its attack rows; a clone has None and
  animates from movement). The player body is not special to rendering — only the
  camera/HUD are. Primary anim logic is unchanged (replay green; render/app/boundary
  suites green).
- **Deliberately NOT a `PlayerEntity` yet:** the movement/combat player systems still
  `single_mut()` over `With<PlayerEntity>`, so the marker swap (and dropping the
  bespoke `drive_player_clones`) waits for step 3, when those become loops. The clone
  thus moves via its bespoke driver but RENDERS via the shared path.
- **Blind visual fix** — I can't see the GUI: the clone should now show the animated
  player sprite when spawned with **K** (Jon to eyeball). The movement is unchanged
  and proven by `player_clone_live`.
- Commit: `feat(player): step 2 — clone is a full visual player body (shared animate_player)`.

## Riding made EMERGENT (folded in — the "should the clone ride?" tell)

Jon's catch: asking which entities ride a platform is per-entity feature-thinking;
riding is a *consequence* of being a body resting on something that moved. Fixing
it as a "carry system that lists body types" would be the same mistake, more general.
The right home: a solid can carry a velocity, and the collision SWEEP carries any
body resting on a moving solid. Static geometry is the `velocity == ZERO` degenerate
case. No rider list, no per-actor flag.

### Increment A ✅ — engine model + the shared `step_kinematic` sweep
- `ae::Block` gains `velocity: Vec2` (a moving platform's `last_delta`; `ZERO` for
  static). `MovingPlatformState::as_collision_block` sets it.
- `step_kinematic` (the sweep regular enemies / NPCs share): when a body is grounded,
  it probes the supporting block and adds that block's gravity-PERPENDICULAR velocity
  (the gravity-axis ride is already handled by gravity + landing). New tests:
  `a_body_rides_a_horizontally_moving_platform`, `a_body_does_not_ride_static_geometry`.
- Off moving platforms it's a no-op (ZERO velocity), so existing behavior is
  byte-identical: 916 sandbox + 40 platformer-runtime tests + replay green.
- **Two grounding paths still to carry** (the same fix, applied where each resolves
  ground): the **slug** (`step_surface_walker` — its own surface-snap path, NOT
  `step_kinematic`) and the **player/clone** (`sweep_player_*`). Doing those next, then
  delete the player's inline `is_riding` carry. The lingering need to touch >1 path is
  the two-sweeps fork; collapsing that is the deeper follow-up.
- Commit: `feat(physics): emergent platform riding — a Block carries velocity, the sweep carries riders`.

### Increment B ✅ — player + clone ride via the same emergent rule
- Added the identical carry to the player movement sweep: at the end of
  `integrate_velocity_clusters`, a grounded body resting on a moving solid is carried
  by the supporting block's gravity-perpendicular velocity (probe-based, orientation-
  agnostic — wall-walking rides sideways platforms). The player AND the brain-driven
  clone ride through this one core.
- **Deleted the player's inline `is_riding` carry** + the riding debug log + the
  `PlayerPlatformRideState` bookkeeping from `player_simulation_phase`. Kept the
  LEDGE-platform carry (hanging off a moving edge is player-specific AND happens while
  NOT grounded, so the sweep carry can't apply). `PlayerPlatformRideState` is now
  vestigial (left on the bundle; removable later).
- Pinned with a new engine test `the_player_rides_a_horizontally_moving_platform`
  (the replay fixture's hub has no moving platforms, so replay can't guard riding —
  the test does). Replay still byte-identical; engine 167 + sandbox 916 + clone-live green.
- **Blind feel note:** standing-platform riding moved from before-sim (full delta) to
  end-of-sweep (perpendicular delta); identical for horizontal platforms (the common
  case). Jon to eyeball riding a platform as the player + as the K-clone.
- Commit: `feat(physics): player + clone ride via the emergent sweep carry (delete inline riding)`.

### Increment C ✅ — the slug rides too
`step_surface_walker` is the slug's own grounded path (glued to surfaces, not
gravity-resting). Added the carry at the top of its grounded branch: probe toward
the clung surface (`-surface_normal`) and add the supporting block's FULL velocity
(both axes — it's stuck to the surface, not merely resting under gravity).
`surface_solid_pred` already matches `BlinkWall` (the moving-platform kind), so it
finds the platform. Pinned with `a_surface_walker_rides_a_moving_platform` (isolates
the carry by comparing a moving platform against an identical static one — the crawl
is the same, the +5px difference is the ride). 917 sandbox tests green.

**Emergent riding is now complete across all three grounding paths** — regular
enemies/NPCs (`step_kinematic`), player + clone (`integrate_velocity_clusters`),
slug (`step_surface_walker`) — all from one rule: a body resting on a moving solid
is carried by it. The lingering need to touch three paths is the multi-sweep fork;
collapsing those into one sweep is the deeper follow-up the riding work motivates.
Commit: `feat(physics): surface-walkers ride moving platforms (slug fix)`.

### Next — 3c: clone → PlayerEntity, single_mut player systems → loops
Drop the bespoke `drive_player_clones`; the clone already renders + rides via the
shared core, so 3c is the marker swap + iterating the movement/combat systems.

## 3c — clone → PlayerEntity (movement half done; a big blocker surfaced)

### 3c-i ✅ — the player movement tick is now multi-body-ready
- `player_control_system` + `player_simulation_system` converted from `single_mut()`
  to a `for` loop over `With<PlayerEntity>`, each body driven by its own `ActorControl`.
- The world-global concerns are gated to the primary: `player_control_phase` /
  `player_simulation_phase` take `is_primary` and skip the sandbox RESET + camera
  SHAKE for non-primary bodies. The moving-platform ADVANCE moved into its own
  `advance_moving_platforms` system (primary's hitstop → `sim_dt`), chained between
  control and simulation — so it runs once and can't multiply over multiple bodies.
- The other player `single_mut()` systems (dev edits, input timers, interaction,
  reset input, room transition, attack-advance, cleanup) scoped to `PrimaryPlayerOnly`
  — they're genuinely primary concerns, and this makes them clone-safe.
- **Replay byte-identical** (loop over the single primary == the old single_mut; gates
  are no-ops with one primary); app-lib 179 + boundary 30 + clone-live green.
- Commit: `refactor(player): 3c-i — player movement tick iterates; globals gated to primary`.

### 3c-ii ⛔ BLOCKED — `PlayerEntity` is assumed singular in ~40 places
Making the clone a `PlayerEntity` is NOT safe yet. A sweep found **~40 `With<PlayerEntity>`
query sites** across sandbox + render + bins that call `single()`/`single_mut()` and
assume exactly one player: enemy + boss AI **targeting** (`actors/update.rs`,
`bosses/tick.rs`), **hazards / pickups / chests / breakables / targeting**
(`mechanics/combat/*`), **projectiles**, **affordances** (interactable / pogo / intent
proximity), **reset**, **rope**, player **pose / health / attack** systems, render
(`projectile_visuals`, `pirate_weapon`). With two `PlayerEntity` bodies, every one of
those `single()` calls returns `Err` and the system **silently no-ops** — the PRIMARY
player would stop taking hazard damage, grabbing pickups, being targeted, etc.

So "clone → PlayerEntity / drop `drive_player_clones`" requires first reclassifying
each of those ~40 sites: the ones that mean "**the** player" become `PrimaryPlayer`
(targeting, hazards, pickups, HUD, camera, reset), and only the genuinely all-bodies
ones stay `PlayerEntity` and iterate. That is the true scope of "PlayerEntity is a
singleton" baked through the codebase — a large, dedicated, mostly-GUI-unverifiable
sweep. The movement-tick half (3c-i) is the safe foundation; the codebase-wide
reclassification is the remaining piece. Clone keeps its `drive_player_clones` driver
until then (the honest per-body / primary-responsibility split).

### 3c-ii — RESUME HERE: the `PlayerEntity`→`PrimaryPlayer` reclassification sweep

Jon said push. Approach + safety: today there is exactly ONE `PlayerEntity` (the
primary), so `With<PlayerEntity>` and `PrimaryPlayerOnly` select the SAME entity —
changing a "the player" site to `PrimaryPlayer` is **byte-identical now** (replay
guards it). Mis-classification only manifests once the clone is a `PlayerEntity`
(caught by clone-live + eyeball). So: change every "THE player" site to
`PrimaryPlayer`; leave genuine all-bodies sites on `With<PlayerEntity>` (most already
`iter()`). THEN add `PlayerEntity` + the missing player components to the clone and
delete the movement half of `drive_player_clones`.

An Explore sweep classified ~113 `With<PlayerEntity>` sites (47 PRIMARY / 66
ALL-BODIES). **Its calls need my review** before editing — a few look wrong (e.g.
`ability_cooldown.rs:60`, `runtime/reset/mod.rs:92/219`, `body_mode/mechanics/mod.rs:60`
were tagged ALL-BODIES but are arguably primary; `player/systems.rs` input/pose/attack
mirror is genuinely all-bodies). The judgement rule: **anything the WORLD does to the
player** (hazards, pickups, chests, breakables, enemy/boss targeting, damage,
projectile-impacts) = all-bodies/iterate; **anything that IS the player's singular
identity** (HUD, camera, affordance hints, abilities fired on device input, portal
gun, rope, reset/persist/save, dev/headless single reads) = PRIMARY.

PRIMARY sites to switch to `PrimaryPlayer` (review each, then edit), by crate:
- **ambition_sandbox**: shrine.rs:37 · items/persist.rs:29,52 · items/pickup/mod.rs:563,658
  · abilities/thrown/puppy_slug_gun.rs:44 · abilities/traversal/{dive:90,grapple:46,
  possession:80,blink:51,mark_recall:64} · abilities/ranged/{sentry:56,shockwave:41,
  meteor:87,vortex:53,volley:45,beam:84} · portal/mod.rs:66 · mechanics/gravity/
  lifecycle.rs:50 · player_rope.rs:131,187,231 · player/affordances/{pogo_proximity:47,
  interactable_proximity:44,intent:145,mod:101,113} · features/ecs/actors/update.rs:83
  (primary fallback) · features/ecs/damage/mod.rs:138 (primary fallback)
- **ambition_render**: hud.rs:51,65,161 · rendering/item_visuals.rs:129 ·
  rendering/pirate_weapon.rs:101
- **ambition_portal_presentation**: visuals.rs:52,213
- **ambition_app**: bin/headless.rs:115,124 · menu/effects.rs:94 (already
  PrimaryPlayerOnly? verify) · app/player_clone.rs:70 (spawn reads primary — keep)
- **ambition_content**: portal/{inventory_adapter:55,103,105,111, input_adapter:53,
  fire_adapter:26, transit_body_adapter:52,141, ability_adapter:74,119}

ALL-BODIES (stay `With<PlayerEntity>`, must iterate — verify none still `single()`):
ability_cooldown:60 · items/pickup:348,356,416 · player/systems.rs:21,35,106,123 ·
mechanics/combat/{targeting:38,hazards:23,pickups:38,breakables:12,chests:16,hitbox:70}
· encounter/systems:54 · runtime/reset:92,219 · features/ecs/{interact:22,actors/
update:78,damage:133,bosses/tick:287} · projectile/systems:96,395 · body_mode/
mechanics:60 · render/projectile_visuals:46 · app/{headless:334,world_flow/room_flow:206,
rl_sim/runtime:*,dev_runtime:104} · content/bosses/specials/{mode_collapse,echo_fan,
overflow_flood,eye_beam,gradient_sentinel:313,469}

Then 3c-ii final: clone gets `PlayerEntity` + `PlayerInteractionState`,
`ActivePlayerAttack`, `PlayerSafetyState`, `PlayerInputFrame`, `PlayerPlatformRideState`
(the components the iterating movement queries require); reduce `drive_player_clones`
to a brain-tick that only fills `ActorControl` (movement now comes from the iterating
player systems); verify replay byte-identical + clone-live (clone runs entirely
through shared systems).

## (superseded) earlier Stage 3 framing

The decomposition foundation is in place. Next is the high-value, higher-risk work:

- **Stage 2 (composable body):** Define the shared actor body = `BodyKinematics +
  ActorSurfaceState + MovementTuning(per-actor component) + opt-in ability
  components`. Make the player an instance. Begin collapsing the 4 cluster
  query-datas (`PlayerClusterQueryData` / `Enemy` / `Npc` / `Boss`) toward one
  composable body + ability components. Compiler-driven, wide.
- **Stage 3 (enemies + NPCs onto the spine):** route them through
  `integrate_normal_clusters` (the gravity-direction-relative spine) with
  restricted ability sets + per-actor tuning; DELETE `integrate_standard_enemy_body`
  + the NPC integrators. **This is where the sideways-gravity NPC-fall bug is fixed
  for free** (pin a "NPC falls toward sideways gravity" headless test). Author the
  slug surface-crawl + parrot dive-bomb as CONTENT ability components (the open/
  composable proof). Non-player feel changes — pin new behavior, flag notable ones.
- **Stage 4** rendering unify · **Stage 5** bosses · **Stage 6** player → input+
  camera+HUD · **Stage 7** combat unify (in scope this run).

### Stage 3a ✅ — vector gravity for non-players (the sideways-gravity bug fix)
- Generalized `step_kinematic` (`ambition_platformer_runtime/kinematic.rs`) from a
  Y-only `gravity_sign` scalar to a 2D `KinematicTuning.gravity_dir`: gravity +
  fall-cap project onto the direction; "ground" is a contact on the gravity side
  (the X sweep owns landing under sideways gravity). Threaded `gravity_dir` (from
  `GravityCtx::dir_at`) through the enemy + NPC integration chains.
- **Fixes Jon's reported bug**: NPCs/enemies now fall toward left/right gravity, not
  just down/up. Vertical gravity byte-identical (replay green); 916 sandbox tests
  green; new `sideways_gravity_makes_a_body_fall_into_and_land_on_a_wall` test.
- Follow-up (Stage 3b): sideways JUMP/RUN gravity-relativity for non-players (the
  enemy run is still X-axis; jump still `* gravity_dir.y`). Then the deeper merge —
  enemies/NPCs share the literal player spine (`integrate_normal_clusters`) once the
  composable body (Stage 2) lands.

### Stage 3b ✅ — non-player run/jump gravity-relative
- Enemy + NPC run now acts along the gravity-perpendicular "side" axis (walk ALONG
  the wall), jump opposes `gravity_dir` in 2D. Byte-identical for vertical (replay
  green, 916 tests). Minor follow-up: patrol wall-stop facing-flip still reads vel.x.

### RESUME → Stage 2 (composable body) — the big structural pivot
Sideways gravity now fully works for non-players (3a fall + 3b run/jump) as targeted
incremental wins. Next is the deeper unification:
- **Stage 2:** one composable actor body = `BodyKinematics + ActorSurfaceState +
  per-actor MovementTuning + opt-in ability components`; collapse the 4 cluster
  query-datas (`PlayerClusterQueryData`/`Enemy`/`Npc`/`Boss`) toward one. Wide,
  compiler-driven.
- **Stage 3 (full):** enemies/NPCs run the LITERAL player spine
  (`integrate_normal_clusters`) with restricted ability sets; DELETE
  `integrate_standard_enemy_body` + the NPC integrators; author slug surface-crawl
  + parrot dive-bomb as content ability components.
- Then **4** rendering · **5** bosses · **6** player→input+camera+HUD · **7** combat.

### Notes / decisions / behavior changes
- Stage 0 + Stage 1 + Stage 3a landed; foundation green + replay-identical.
- desired_vel dual-meaning (P10) + the `single_mut()` player systems (P9) get
  resolved in Stages 3/6 respectively; see the pain-points journal.
