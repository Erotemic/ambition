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

## RESUME HERE → Stage 2 — one composable body (the big structural pivot)

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

### Notes / decisions / behavior changes
- Stage 0 + Stage 1 landed (4 commits incl. plan); foundation green + replay-identical.
- desired_vel dual-meaning (P10) + the `single_mut()` player systems (P9) get
  resolved in Stages 3/6 respectively; see the pain-points journal.
