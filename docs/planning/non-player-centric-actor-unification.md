# Non-player-centric actor unification — COMPLETE (2026-06-17)

**Status: DONE.** This was the design guide + north star for unifying every
character — player, NPC, enemy, boss — onto one actor system. The work landed on
`main`; the detailed run-log was pruned (its outcome is captured here, in project
memory, and in git history).

## North star (achieved)

Every character is a different *instance* of one actor system. The only genuinely
player-specific things are device input (already abstracted by the input layer),
the camera, and the HUD/UI. Body, movement, abilities, rendering, and combat are
shared machinery an actor opts into.

## Resulting architecture (what to know now)

- **One grounded spine** — `ambition_engine_core::integrate_normal_spine`
  (+ `NormalSpineCtx::bare`): gravity + run + fall-cap, gravity-direction-relative.
  Player feeds rich ability flags; enemies/NPCs feed `bare` + a per-actor
  `MovementTuning`. Fully gravity-relative (fall + run + jump + patrol wall-stop).
- **One floating mover** — `ambition_gameplay_core::features::step_floating_body`: shared
  by aerial enemy, aerial NPC, and boss (`accel: None` = snap to pattern velocity).
- **One render tail** — `apply_character_frame` (`ambition_render`); anim-picker
  richness differs by actor (correct pay-for-use).
- **Emergent platform riding** — a `Block` carries velocity; the collision sweep
  carries any grounded body resting on a moving solid. Static geometry is the
  zero-velocity degenerate case. No rider list.
- **Player vs body** — `PlayerEntity` = any player body; `PrimaryPlayer` = the one
  camera/HUD/gameplay-center body (`PrimaryPlayerOnly` alias). The brain-driven
  K-clone is a real `PlayerEntity` driven entirely by the shared player systems.
- **Combat** — projectile charge gated by the `ChargesProjectiles` capability
  marker, not `brain.is_player()`.

## Closed-with-reasoning (do NOT re-attempt)

- **`desired_vel` axis-unification** — the dual-meaning (floating = 2D velocity,
  grounded = 1D axis) is *essential* complexity; the `integrate_normal_spine`
  bridge is the correct handling, not tech debt.
- **Slug/parrot "ability components"** — composability is already satisfied:
  capabilities live in machinery (`step_surface_walker` / `step_floating_body`),
  content data opts in (`enemy_archetypes.ron`). Don't component-ize the flags.
- **Collapsing the three grounding sweeps into one** — gravity-resting vs
  surface-glued are genuinely different physics; a single generic sweep would be a
  wide generic surface. The shared *rule* is already unified.
