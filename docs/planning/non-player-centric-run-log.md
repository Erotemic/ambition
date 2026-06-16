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

## Stage 1 — decompose the rest of the player monolith into `abilities` functions

Peel each remaining verb into `movement/abilities.rs` (or keep `control.rs`'s
already-extracted blink/attack): facing+buffer, fly-toggle, dodge, shield,
jump-release (control phase); jump-buffer/wall-jump/double-jump, gravity, run,
fast-fall, glide, flight, climb, wall-abilities, fall-cap, rebound (sim phase).
Each a pure refactor; replay byte-identical. Improve feel only deliberately + flag.

### Notes / decisions / behavior changes
- (run begun; Stage 0 landed)
