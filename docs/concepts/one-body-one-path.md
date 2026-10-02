---
id: one-body-one-path
aliases: []
status: current
authority: durable-concept
last_verified: 2026-10-02
related_docs:
  - docs/concepts/invariants.md
  - docs/concepts/movement-collision.md
  - docs/adr/0020-mounts-and-vehicles.md
---

# One body, one path

**The player is an actor.** Every rule that fires for one controller kind fires
for all of them, through the same code, or it is a fork.

`AGENTS.md` states the rule. This page holds the detail.

## The bifurcation smell test

Before you write anything keyed to "player", or to "actor / enemy / boss" (an
attack, a hitbox, a damage rule, a VFX/SFX emit, a shield, a reset, a state
machine, a brain hook), ask:

> **Does the other controller kind already do this on its own code path?**

If yes, you found a **fork**. Unify onto the single shared seam and delete the
other side. Do not add a second site.

- ⛔ A parallel emission site, state component, system or spec for an effect that
  already exists is a bug, even if it compiles and every test passes. A green test
  on a forked path proves only that the fork works.
- If you cannot complete the merge in one pass, do not add the parallel path "for
  now". Route the new caller through the existing seam (extract one shared fn,
  system or event if none exists). Log the remaining merge in
  `dev/journals/code_smells.md` with `BIFURCATION:` as the first word.
- "Unification" means **delete one path**. It does not mean "make two paths
  behave alike".

## The current shape

This section is a status inventory. It can go stale; the rule above does not.

**Melee is one path end to end.** The state (`MovePlayback`, read as a
`MeleeSwing` through `melee_swing_of`), the swing model (`AttackSpec`), the slash
VFX (`emit_melee_slash` in `ambition_combat::util`), the strike spawn and body
contact resolution are shared by every body. `trigger_moveset_moves` and
`advance_move_playback` (`ambition_combat::moveset`) spawn one gravity-resolved
volume that drives the damage `Hitbox` and the slash.
`ambition_combat::hitbox::apply_hitbox_damage` resolves every `FollowOwner`
strike through one victim loop: owner exclusion, relationship policy, published
hurtbox geometry, per-hitbox dedup and victim-specific knockback. `Player` +
`World` hitboxes are a separate world-AOE primitive, not a second melee path.

⛔ Do not reintroduce a `PlayerAttackState` / `ActorAttackState` split, a second
slash emit, or a per-frame player damage loop. Every melee is an
`"attack"`-verb moveset move riding `MovePlayback`.

**A landed body strike is one fact.** `apply_hitbox_damage` publishes
`LandedBodyHit { hitbox, attacker, victim, volume, contact }` when it commits the
targeted damage event. Move confirms and authored `on_hit` techniques consume
that fact; they do not rediscover overlap, faction policy, self-exclusion or
victim identity. Body pogo consumes the resolved victim plus its `PogoPolicy`.
Ordinary bodies may publish `PogoTargetVolumes` for affordance, but they are
never flattened into collision-world `PogoOrb` blocks. Only an explicit
`PogoTargetContributor` opts an ECS feature into world rebound geometry.

**Non-body recipients.** `HitTarget` (`ambition_combat::events`) has:

- `Body(Entity)`: one pre-resolved body victim;
- `Feature(Entity)`: one pre-resolved boss or breakable, named by the contact
  that selected it (the projectile road writes this);
- `UnresolvedFeatures`: the remainder of a melee or area strike after body
  victims were resolved. Consumers scan only non-body targets. It is scaffolding
  that goes away when bosses and breakables are directly resolvable victims;
- `Volume`: scan everything; the world-AOE primitive;
- `OrbMatch`: pogo orb matching.

⛔ Do not treat `UnresolvedFeatures` as `Volume`. A consumer that conflates them
damages every body twice.

**A hit says what KIND it was, never WHO.** `HitSource` is a cause: `Melee`,
`Projectile`, `Contact`, `Hazard`, `LeftTheWorld`, `Pogo`. Whose swing it is,
whether it hits heavy, and whether a damage slider applies are questions for the
attacker or the victim, which the event names. ⛔ Do not put a faction or
controller word into the cause vocabulary. Identity (self-exclusion) beats every
relationship rule, in both resolvers.

**Who may fight whom is one policy.** `combat_relation`
(`ambition_combat::targeting`) answers `Foe` / `Neutral` / `Ally` with precedence
grudge → match team → authored faction. AI target selection and damage
resolution both call it. Every match seat carries a team. `Neutral` is hittable
but not huntable: damage is physical, targeting is relational. Do not collapse it
to a boolean.

**Simulation truth lives in the simulation crate.** `Hitbox` and its lifecycle
are in `ambition_combat::strike`, not in `ambition_vfx`. `HitSide` and the
`Effect` request vocabulary stay lower because `ambition_projectiles` names
`Effect` and sits below combat. Presentation does not reach back: the render
stand-in reads `CombatGeometryView`'s strike rows (strike entity, owner,
body-anchored flag, and the owner position the volume was resolved against).

**A weapon in hand owns the Attack press.** With a `HeldItem`
(`ambition_combat::held_items`) on the body, `trigger_moveset_moves` resolves
Attack through the item: its own melee verb if it authors one, otherwise the
item's own subject-generic system (throw, bolt, gauntlet). The wearer's `attack`
verbs are not revoked; only the resolution moves, and it moves back when the hand
is empty. ⛔ Do not "fix" a double-fire by deleting the wearer's verbs: the touch
Attack button exists only while the action scheme carries an Attack slot. An item
system that ends the holding (the throw) must also mark the press spent.

**Movement is one phase.** Every body integrates in `integrate_sim_bodies`
(`ambition_platformer2d_actor_monolith/src/features/ecs/actors/update.rs`). There
is no separate route for the controlled body.
`player_body_tick_is_not_the_gameplay_movement_route`
(`game/ambition_app/tests/unified_body_movement.rs`) forbids a separate
home-body movement system.

**Brain decision and body integration stay separate systems on purpose.**
`tick_actor_brains` decides and `integrate_sim_bodies` moves. What is shared is
the body-tick entry, not the orchestration. Do not merge phases into one
god-system.

**Facing is control output, not a collision side effect.** The movement kernel
publishes semantic contacts such as `BodyWallState`; it never reverses a body
because velocity stopped. Patrol/Wanderer policy may turn away from a real side
contact. Human input, fighter brains, scripted control, remote control and RL
keep the facing they chose.

**Observers do not need a privileged protagonist.**
`ambition_sim_view::CombatGeometryView` projects collision envelope, effective
hurtboxes and live strike volumes for every combat body. Debug rendering
consumes it whether zero, one or many bodies are under human control.

**Precision blink is an input affordance.** Responsive aim during bullet-time is
`InputState::control_dt`: a human sets it to the real frame dt; a brain leaves it
`0`. There is no second simulation.

## Six names for "player", and none is a body kind

The authoritative prose for each is on its definition. This table maps them.

| Name | What it is | Lifetime | ⛔ never use it for |
| --- | --- | --- | --- |
| `ParticipantId` | the person at a controller | outlives sessions, bodies and possession | anything a body does |
| `PlayerSlot` | the seat that participant occupies; `SlotControls[N]` is its control frame | the session's seating | "the protagonist"; slot 0 is a seat, not a role |
| `DrivingParticipant(slot)` | control authority: this body is driven by that seat | moves between bodies (that is possession) | a body kind; not `Brain`, which is AI policy |
| `PlayerEntity` | a body in the player population | the body | assuming there is exactly one |
| `PrimaryPlayer` | the home avatar: save identity, respawn anchor, inventory owner | the home body; a session may have none | "the currently controlled body" |
| `ControlledSubject` | the body a local presentation/control context follows | the frame | a second global actor identity |

⛔ **Zero `PrimaryPlayer` is a legitimate steady state.** A match under
`InitialBodyPolicy::NoInitialBody` lowers no home avatar, so every reader must be
correct at a count of zero. The failure shape is `single()` + `else { return }`,
which disables a whole subsystem (clock, camera, moving platforms) for every
entity. Read `crates/ambition_platformer2d_shared_tangle/src/markers.rs` before
you add a reader; it classifies each site.

**Generic simulation consults none of the six.** A body decides, moves, fights
and rides because of its capabilities and its control authority. A body-generic
system that needs one of these names is usually asking "which body" (an
`Entity`) or "which seat" (a `PlayerSlot`).

## Next

The unified action/ability timeline (cancel windows, movement locks,
armor/i-frames, resource costs, hurtbox swaps, animation binding) layers on the
one strike seam. It is not a second seam beside it.
