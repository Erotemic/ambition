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
