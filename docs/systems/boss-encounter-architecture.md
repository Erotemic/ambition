# Boss encounter architecture direction

> **Actor unification (ADR 0016):** bosses ARE actors. `BossEncounterPhase`
> lives in `crate::brain::boss_pattern` (not a boss-only module); the goal is one
> unified actor+brain+boss-runtime unit with only *named* boss data in
> `ambition_content`.


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
HitEvent { source: PlayerSlash | PlayerProjectile, target, volume } (player hits boss)
  ↓
apply_feature_hit_events (features/ecs/damage.rs)
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

Boss damage used to route through a `GameplayEffect::DamageBoss` bus
variant + an `apply_boss_damage_effects` reader. That variant was a no-op
observation seam and both it and the reader have been **deleted** (the
`GameplayEffect` enum itself was later split into focused messages —
`SetFlagRequested` / `QuestAdvanceRequested` / `SwitchActivated` /
`GameplaySfxRequested`; none of those carry boss damage). Boss damage is
now applied **inline** in the hit path (`apply_boss_hit` →
`record_boss_damage`); a future tracing / quest / replay observer should
add its own focused message rather than reviving the old bus seam.

`record_boss_damage` returns `Option<BossDamageOutcome>` (`None` when
the runtime id has no registered encounter — gracefully degrades
when test fixtures don't install the boss machine). Unit tests in
`boss_encounter::damage::tests` lock in the four outcome paths.

## Boss encounter spec as content (ADR 0017, current ownership)

The encounter's numeric fields (HP, phase thresholds, timings,
music ids) are content rather than Rust constants. Each authored boss ships an
encounter RON under `crates/ambition_content/assets/data/boss_encounters/<id>.ron`.
`ambition_content::bosses::install_boss_roster` embeds and installs those specs
into gameplay-core's generic holder during sandbox resource initialization.

`ambition_gameplay_core` owns the reusable schema and encounter state machine.
It does not own the named boss roster. Behavior/reward data comes from
`crates/ambition_content/assets/data/boss_profiles.ron`; LDtk owns spatial
placement. That is the current ADR 0017 split: Rust = reusable behavior/schema,
RON in `ambition_content` = named game content, LDtk = space.

Layered guards keep the migration honest:

- `specs::tests::every_on_disk_ron_matches_an_authored_profile` — orphan RON
  files (typo'd filename, leftover from rename) trip with a focused diff.
- `specs::tests::load_boss_specs_from_disk_has_no_duplicate_ids` — two files
  with the same `id:` would make roster resolution ambiguous.
- Python `tests/test_boss_encounters_ron.py` — schema pins fire without
  compilation (catches missing field / out-of-range fraction / negative timing /
  filename↔id mismatch).
