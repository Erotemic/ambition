# Boss encounter architecture direction

The current sandbox bosses share the same coarse encounter skeleton: intro,
phase thresholds, transition, stagger, enrage, death, music request, and reward
sync. That is useful scaffolding, but it should not become the place where every
boss-specific trick is hard-coded.

Future boss work should keep three layers distinct:

1. **Encounter progression**: phase timing, save-state transitions, music and
   cutscene requests, and victory events. This remains the generic state-machine
   layer.
2. **Boss behavior**: movement, attacks, arena interactions, tells, and special
   vulnerabilities. This should become per-boss data/code rather than more
   branches inside the generic encounter update loop.
3. **Rewards and aftermath**: defeat drops, quest advancement, arena cleanup,
   and reload synchronization. Mockingbird's pirate-hoard chest is the first
   example, but future bosses should use a reward table/profile instead of
   adding one-off `sync_<boss>_...` systems.

The sandbox-side `boss_encounter` module is split around these seams so richer
bosses can add behavior profiles without turning the facade into a long mixed
system. If new bosses start needing custom gravity, moving arena hazards,
scripted props, or multi-stage weak points, prefer introducing a per-boss runtime
profile or behavior plugin over extending the generic encounter loop with named
special cases.

## Boss HP authority (2026-05-20)

After OVERNIGHT-TODO #8, the engine `ae::BossEncounterState` is the
single source of truth for boss HP. The sandbox `BossRuntime.health`
is a one-way mirror updated each frame by `update_boss_encounters`;
gameplay code SHOULD NOT mutate `boss.health` directly. The damage
path is:

```text
DamageEvent (player hits boss)
  ↓
apply_feature_damage_events (content/features/ecs/damage.rs)
  ↓
record_boss_damage (boss_encounter/damage.rs)
  ↓ apply_player_damage on the engine state
  ↓ publish_events  → music / cutscene / banner requests
  ↓
BossDamageOutcome { hp_remaining, killed, applied }
  ↓ damage.rs mirrors `hp_remaining` onto `boss.health.current`
  ↓ damage.rs fires death VFX / banner / debris when `killed`
  ↓ damage.rs suppresses hit VFX when `applied == false`
    (invulnerable phase swallowed the damage)
```

`apply_boss_damage_effects` (in `content/features/bus.rs`) used to be
the indirection layer that fed engine state via
`GameplayEffect::DamageBoss`. The damage application happens inline
in `apply_feature_damage_events` now; the bus reader stays as a typed
seam for future tracing / quest / replay hooks that want to observe
boss damage without re-routing through the registry.

`record_boss_damage` returns `Option<BossDamageOutcome>` (`None` when
the runtime id has no registered encounter — gracefully degrades
when test fixtures don't install the boss machine). Unit tests in
`boss_encounter::damage::tests` lock in the four outcome paths.
