# EnemyRuntime → ECS dissolution inventory

Goal: delete `EnemyRuntime` (and the `ActorRuntime::Enemy(EnemyRuntime)`
bridge). Enemy state lives on the entity as authoritative ECS
components; the per-tick integration mutates those components in place
(player cluster pattern — no runtime scratchpad, no one-way mirror sync).

Shotgun approach approved: intermediate breakage OK; this inventory is the
contract for "no feature lost."

## Field → component target

| EnemyRuntime field | Target component (authoritative) |
| --- | --- |
| id, name, sprite_override_npc_name | `ActorIdentity` (exists) |
| pos, size | `FeatureAabb` (center, half_size) (exists) |
| vel | `EnemyKinematics.vel` (NEW) |
| facing | `EnemyKinematics.facing` (NEW) — also mirrored to ActorPose.facing |
| health | `ActorHealth` (exists) |
| alive | `ActorHealth.alive()` is derivable; keep explicit `EnemyLife.alive` (NEW) |
| hit_flash | `ActorCombatState.hit_flash` (exists, make authoritative) |
| attack (windup/active/cooldown/pending_axis) | `EnemyAttack` (NEW, owns ActorAttackState) |
| respawn_timer | `EnemyLife.respawn_timer` (NEW) |
| ai_mode | `ActorIntent` (exists, make authoritative) |
| archetype | `EnemyArchetypeComp` (NEW) |
| brain (EnemyBrain) | `EnemyBrainKind` (NEW) — distinct from the `Brain` driver component |
| motion (Option<PathMotion>) | `EnemyMotionPath` (NEW) |
| spawn (ActorSpawnState) | `EnemySpawnBaseline` (NEW) |
| surface (on_ground/surface_normal/gravity_scale/air_jumps) | `EnemySurface` (NEW) |

## Behaviors/features that MUST survive (from EnemyRuntime::update + helpers)

- [ ] hit_flash decay each tick
- [ ] dead handling: respawn_timer countdown; FiniteSandbag auto-respawn (alive/health/pos/vel/hit_flash); ai_mode=Dead; neutral frame
- [ ] attack timer tick + windup→active edge arming (ActorAttackState::tick)
- [ ] AI-mode evaluation (evaluate_character_ai_output) → ai_mode for HUD/anim
- [ ] surface-walker (PuppySlug) custom integration: crawl floors/walls/ceilings, corner wrap, ledge/fall, neighbor reverse; writes pos/vel/facing/surface_normal/on_ground
- [ ] normal kinematic integration via step_kinematic: gravity_scale, on_ground, air jumps, facing, pos/vel
- [ ] PathMotion advance (patrol) — Combatant/etc. patrol paths
- [ ] air-jump reset on landing; decrement on air jump
- [ ] shark charge-crash detection (shark_charge_crashed reads archetype/pos/vel/alive)
- [ ] reset_to_spawn: restore archetype/size/pos/vel/health/attack/surface/etc. from spawn baseline
- [ ] mount/dismount: PirateOnShark → PirateRaider + BurningFlyingShark; spawn_size/gravity_scale overrides (mount.rs)
- [ ] begin_melee_attack (cooldown gate, windup, pending_axis, ai_mode=Telegraph)
- [ ] attack_aabb / attack_aabb_dir / attack_telegraph_aabb (hitbox geometry from pos/size/facing)
- [ ] hitbox spawn on windup→active edge (update_ecs_actors)
- [ ] brain snapshot build (build_enemy_brain_snapshot reads pos/vel/facing/on_ground/health/attack/archetype)
- [ ] component snapshot (ActorIdentity/Disposition/Health/CombatState/Intent/Cooldowns)
- [ ] visual_kind / rotation_rad / sprite override resolution (rendering)
- [ ] bark_anchor (banter), combat banter on hit
- [ ] damage application (apply_actor_hit: vel knockback, health.damage, archetype branches, kill flags/debris/sfx)
- [ ] death/kill: alive=false, respawn policy flags, FiniteSandbag respawn_timer
- [ ] debug overlay (attack aabb, hurt box), trace recorder
- [ ] save sync (provoked/dead flags), reset (room re-enter)

## Consumer files (must compile + behave after migration)

ecs/actors.rs (integration), enemies.rs (struct+update+helpers), ecs/damage.rs,
ecs/brain_effects.rs, ecs/brain_builders.rs, ecs/spawn.rs, ecs/spawn_actors.rs,
ecs/spawn_mounts.rs, ecs/mount.rs, ecs/aggression.rs, ecs/save_sync.rs,
ecs/reset.rs, ecs/anim_helpers.rs, ecs/target_volumes.rs, components.rs,
brain/snapshot.rs, character_ai.rs, content/banter.rs, conversion_tests.rs,
dev/debug_overlay.rs, presentation/rendering/{features,hit_flash,world,
pirate_weapon,deep_dream}.rs, presentation/character_sprites/anim.rs,
enemy_projectile/visuals.rs, app/plugins.rs.

## Plan (commits)

1. Inventory (this doc).
2. Define new components + `EnemyMut<'a>` view + QueryData (compiles, unused).
3. Port EnemyRuntime::update → operate on EnemyMut; rewrite update_ecs_actors.
4. Migrate all consumers off `ActorRuntime::Enemy(enemy)` field reads.
5. Delete EnemyRuntime + ActorRuntime::Enemy; fix fallout; tests green.
