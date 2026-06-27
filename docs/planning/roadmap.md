# Roadmap

One elegance-ordered plan. The engine is sequenced so each step deletes the
duplication the next depends on; the game is built on the finished engine (though
content seeds it along the way). Every phase is verified against the **real headless
sim** ([`engine/headless-verification.md`](engine/headless-verification.md)); behavior
is not sacred pre-polish; delete, don't bridge.

---

## Phase 0 — foundation (done)

Shared movement spine + floating mover + blink + directional block rule; relational
damage (`FactionRelations`, actor-vs-actor); headless `WorldView` + `WorldMemory` with
line-of-fire over real geometry; the player kit as actor capabilities + the
`player_robot` archetype; movement physics as composable per-archetype data; the two
bridges (`ActorControlFrame::to_input_state`, `BodyMovementTuning::spine_tuning`); and
the *proven* seam — a brain-driven body already runs the exact player movement core
(`player_clone`).

## Engine (the keystone chain)

1. **Route bodies through the player pipeline.** Actor bodies carry the player movement
   clusters; the per-actor update calls `update_player_*_clusters`; **delete
   `integrate_standard_enemy_body`**. Enemies gain wall-cling / ledge-grab / dodge /
   variable-jump. Reconcile surface-walkers and aerial free-movers.
   ([`engine/unified-actors.md`](engine/unified-actors.md) §the path.)
2. **Collapse the `Player*` / `Actor*` dual hierarchy** — *the keystone*. Move shared
   sim-state onto the `Actor*` vocabulary; the ~20-module player dependency sink
   dissolves. Sliced, one component family at a time.
   ([`engine/architecture.md`](engine/architecture.md).)
3. **De-player-center the rest** — `ControlFrame` → entity-local `ActorIntent` (~46
   systems); projectile attribution → source/faction; `AggressionMode` names a faction,
   not "the player".
4. **Extract leaf crates** — with the sink gone, split the runtime domains into crates
   → faster compile (goal 1) + reusability (goal 4).
5. **Pluginize the domains** — each a Bevy plugin with owned vocabulary + extension
   points, the `ambition_portal` shape copied (goal 3).
6. **Get named content out of the foundation** — bosses / abilities / rooms move to the
   content crate behind install-time data seams; prove the reusability oracle by
   extracting one domain clean (goal 2 + 4).

## Cross-cutting engine work (parallelizable)

- **Sprite renderer** — measure-by-default core landed; chase melee animation/hitbox
  agreement ([`engine/sprite-renderer.md`](engine/sprite-renderer.md)).
- **Boss system** — entity-local structure landed; remaining is content + feel
  ([`engine/boss-system.md`](engine/boss-system.md)).
- **Headless render-to-disk** — the last verification gap (state → image for visual
  spot-checks); closing it removes the final "I can't verify".

## Game (on the finished engine; content may seed earlier)

- **The intro spine** — wake → raid → escape → the Kernel hub, one room per beat.
- **The Alice/Bob handshake arc** — upgrade-as-theorem (crypto-as-traversal) + Eve
  observes / Mallory modifies.
- **Story bosses** — PCA, Mockingbird, Clockwork Warden, each a failed objective
  function on the engine boss system.
- **Defer** the faction bloat until the spine lands ([`game/vision.md`](game/vision.md)).

## Standing practices (folded from old planning notes)

- **Docs are trustworthy or deleted.** Keep this plan honest; an out-of-date plan is
  worse than none.
- **Data-driven ECS; LDtk owns space; RON owns tuning/audio.** Authoring flows
  data → components → systems → messages.
- **Evaluate ecosystem crates before rolling custom** — document rejections. Standing
  candidates worth a look when their use case lands: `bevy_asset_loader`, `bevy-tnua`,
  `big-brain`/`dogoap` (AI), `vleue_navigator` (nav). Don't adopt speculatively.
- **The validation habit** — a change isn't done until it's exercised in the real sim.
