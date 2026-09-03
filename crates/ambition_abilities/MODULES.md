# `ambition_abilities` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_abilities** — The WIELDED ability kit, carved out of the actor kernel (D33, 2026-09-03).

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`ability_cooldown`](src/ability_cooldown.rs) | Shared per-body cooldown for movement abilities such as Blink and Grapple. |
| [`ranged`](src/ranged/mod.rs) | Ranged abilities: beam, meteor, shockwave, vortex, volley, bomb, sentry. |
| [`test_support`](src/test_support.rs) | Test-only fixtures for ability modules. |
| [`thrown`](src/thrown/mod.rs) | Thrown abilities: the gravity grenade. |
| [`traversal`](src/traversal/mod.rs) | Traversal abilities a held item FIRES: blink, dive, grapple, mark/recall. |

_5 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._

### ⛔ What is deliberately NOT here

**`possession`, `teleport`, `trapdoor`, `flyline`** stayed in
`ambition_platformer2d_actor_monolith::abilities`. They share that directory name
and nothing else: their systems are registered by
`ambition_platformer2d_runtime`, not by the item-pickup family, and `possession`
is named 87 times outside the directory (`teleport` 61) by `body_custody`,
`control::authority`, `features::ecs::dormancy` and `control::input_systems`.
That is control authority. Moving it here would give this crate a home the
RUNTIME registers systems out of and the KERNEL depends on for `PossessionState`
— the coupling renamed rather than reduced.

**`thrown::puppy_slug_gun`** stayed because it SPAWNS A BODY, through the
kernel-private `features::spawn_runtime_minion`. The cross-crate seam already
exists — `ambition_vfx::Effect::Summon` carrying a `SummonSpec`, which
`ambition_combat` emits and the kernel's construction executor materialises into
`ActorConstructionParams::SummonedMinion` — so the gun is the one caller
BYPASSING the canonical construction model, not a caller missing an abstraction.
Routing it needs `SummonSpec` to carry the summon's `ActorAggression` and the
ally marker inserted after the spawn: a behaviour change, and this carve was
behaviour-neutral.

`docs/planning/engine/actor-monolith-decomposition.md` carries both arguments
with their numbers, so neither gets carved by line count later.

### Two edges that point kernel-ward, on purpose

- **The three-variant `.chain()` is the KERNEL's.** It orders
  `ItemPickupSet::CoreHeldItems` (owned by `ambition_held_items`) against this
  crate's two, so neither owner can name both sides.
  `schedule_tests::the_chain_is_not_ours` fails if it appears here — and it
  asserts ABSENCE from the graph, not an empty set, because a set no plugin
  names is not in the graph at all.
- **`teleport.rs` reaches `blink_target` across the line.** Teleport stayed,
  blink moved, and they share the one destination-clamping rule.

### The one test that moved the other way

`a_sentry_bolt_damages_the_enemy_it_was_fired_at` lives in
`ambition_platformer2d_actor_monolith::projectile::sentry_bolt_damage_tests`. It
chains `update_sentries` → `materialize_projectiles_for_this_tick` →
`stamp_new_projectile_allegiance` → `step_projectiles`, and the last two are the
kernel's. A test needing two crates belongs where both are visible; keeping it
here would have required the edge this carve removed.

### Schedule

`AbilitySimulationPlugin` configures `ItemPickupSet::ThrownItemEffects` and
`ItemPickupSet::WieldedAbilities` — their nesting in `PlayerSimulation` — and
registers all 18 members. `src/schedule_tests.rs` pins that by SHAPE on a bare
`App`: 5 and 13 direct members, both variants inside the phase.

