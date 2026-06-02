//! ECS actor types and the per-frame actor tick.
//!
//! `ActorRuntime` is the unified component that backs every authored
//! NPC, authored hostile enemy, and dynamic encounter-spawned mob.
//! Peaceful and hostile actors share the same entity identity so a
//! peaceful NPC can flip to hostile in place after enough strikes
//! rather than being moved between containers.

use super::*;

fn shark_charge_crashed(
    enemy: &EnemyRuntime,
    is_mounted: bool,
    charge_vec: ae::Vec2,
    previous_pos: ae::Vec2,
) -> bool {
    !is_mounted
        && enemy.archetype == EnemyArchetype::BurningFlyingShark
        && charge_vec.x.abs() > enemy.archetype.chase_speed() * 1.5
        && (enemy.pos.x - previous_pos.x).abs() < 0.01
        && enemy.vel.x.abs() < 0.01
        && enemy.alive
}

/// Unified ECS runtime body for authored NPCs and enemy-shaped actors.
///
/// The variants describe which legacy runtime body still owns the low-level
/// movement/combat fields, not who the actor is willing to fight. Current
/// willingness to fight lives in [`ActorAggression`] and mirrored
/// [`ActorDisposition`]. This keeps the old NPC/enemy storage usable while
/// moving gameplay policy out into ECS components.
#[derive(Component, Clone, Debug)]
pub enum ActorRuntime {
    Npc(NpcRuntime),
    Enemy(EnemyRuntime),
}

impl ActorRuntime {
    pub fn id(&self) -> &str {
        match self {
            Self::Npc(actor) => actor.id.as_str(),
            Self::Enemy(actor) => actor.id.as_str(),
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Npc(actor) => actor.name.as_str(),
            Self::Enemy(actor) => actor.name.as_str(),
        }
    }

    pub fn aabb(&self) -> ae::Aabb {
        match self {
            Self::Npc(actor) => actor.aabb(),
            Self::Enemy(actor) => actor.aabb(),
        }
    }

    pub fn pos(&self) -> ae::Vec2 {
        match self {
            Self::Npc(actor) => actor.pos,
            Self::Enemy(actor) => actor.pos,
        }
    }

    pub fn size(&self) -> ae::Vec2 {
        match self {
            Self::Npc(actor) => actor.size,
            Self::Enemy(actor) => actor.size,
        }
    }

    pub fn facing(&self) -> f32 {
        match self {
            Self::Npc(actor) => actor.facing,
            Self::Enemy(actor) => actor.facing,
        }
    }

    pub fn disposition(&self) -> ActorDisposition {
        match self {
            Self::Npc(_) => ActorDisposition::Peaceful,
            Self::Enemy(_) => ActorDisposition::Hostile,
        }
    }

    pub fn visual_kind(&self) -> FeatureVisualKind {
        match self {
            Self::Npc(_) => FeatureVisualKind::Npc,
            Self::Enemy(enemy) => enemy.visual_kind(),
        }
    }

    pub fn visible(&self) -> bool {
        match self {
            Self::Npc(_) => true,
            Self::Enemy(enemy) => enemy.alive,
        }
    }

    pub fn flash(&self) -> bool {
        match self {
            Self::Npc(npc) => npc.hit_flash > 0.0,
            Self::Enemy(enemy) => {
                enemy.hit_flash > 0.0 || enemy.attack.is_winding_up() || enemy.attack.is_active()
            }
        }
    }

    pub fn feature_view(&self) -> FeatureView {
        let rotation_rad = match self {
            Self::Npc(_) => 0.0,
            Self::Enemy(enemy) => enemy.rotation_rad(),
        };
        FeatureView {
            pos: self.pos(),
            size: self.size(),
            kind: self.visual_kind(),
            visible: self.visible(),
            flash: self.flash(),
            switch_on: false,
            rotation_rad,
        }
    }

    /// Build the enemy-shaped combat runtime used when an authored NPC's
    /// aggression policy decides it should fight. This is a temporary adapter
    /// around the remaining `EnemyRuntime` holdout: the NPC's identity and ECS
    /// components stay on the same entity, while the low-level movement/attack
    /// timers are represented with the existing enemy body until that runtime is
    /// split into smaller components.
    pub(crate) fn enemy_runtime_for_npc_combat(npc: &NpcRuntime) -> EnemyRuntime {
        let brain_id = hostile_enemy_brain_for_npc(npc);
        let mut enemy = EnemyRuntime::new(
            npc.id.clone(),
            npc.name.clone(),
            npc.aabb(),
            crate::actor::EnemyBrain::Custom(brain_id.into()),
            &[],
        );
        enemy.pos = npc.pos;
        enemy.spawn.pos = npc.spawn;
        enemy.size = ae::Vec2::new(npc.size.x.max(22.0), npc.size.y.max(38.0));
        enemy.spawn.size = enemy.size;
        enemy.vel = npc.vel;
        enemy.facing = npc.facing;
        enemy.surface.on_ground = npc.on_ground;
        if npc.name != "Kernel Guide NPC" {
            enemy.sprite_override_npc_name = Some(npc.name.clone());
        }
        enemy
    }
}

fn hostile_enemy_brain_for_npc(npc: &NpcRuntime) -> &'static str {
    let dialogue_id = match &npc.interactable.kind {
        crate::interaction::InteractionKind::Npc { dialogue_id, .. } => dialogue_id.as_deref(),
        _ => None,
    };
    let id = npc.id.to_ascii_lowercase();
    let name = npc.name.to_ascii_lowercase();
    let dialogue = dialogue_id.unwrap_or("").to_ascii_lowercase();
    let looks_like_pirate_heavy = id.contains("pirate_heavy")
        || name.contains("broadside bess")
        || name.contains("iron mary")
        || name.contains("salt annet")
        || dialogue.contains("pirate_heavy");
    if looks_like_pirate_heavy {
        return "pirate_heavy";
    }
    let looks_like_pirate = id.contains("pirate")
        || name.contains("pirate")
        || name.contains("quartermaster")
        || name.contains("lookout")
        || name.contains("navigator")
        || dialogue.contains("pirate");
    if looks_like_pirate {
        return "pirate_raider";
    }
    "medium_striker"
}

pub(crate) fn actor_component_snapshot(
    actor: &ActorRuntime,
) -> (
    ActorIdentity,
    ActorDisposition,
    ActorHealth,
    ActorCombatState,
    ActorIntent,
    ActorCooldowns,
) {
    match actor {
        ActorRuntime::Npc(npc) => (
            ActorIdentity::new(npc.id.clone(), npc.name.clone()),
            ActorDisposition::Peaceful,
            ActorHealth::new(crate::actor::Health::new(1)),
            ActorCombatState::peaceful(npc.strikes, npc.hit_flash),
            ActorIntent::new(crate::character_ai::CharacterAiMode::Idle),
            ActorCooldowns::default(),
        ),
        ActorRuntime::Enemy(enemy) => (
            ActorIdentity::new(enemy.id.clone(), enemy.name.clone())
                .with_sprite_override(enemy.sprite_override_npc_name.clone()),
            ActorDisposition::Hostile,
            ActorHealth::new(enemy.health),
            ActorCombatState::hostile(
                enemy.alive,
                enemy.hit_flash,
                enemy.attack.windup_timer,
                enemy.attack.active_timer,
                enemy.archetype.is_sandbag(),
            ),
            ActorIntent::new(enemy.ai_mode),
            ActorCooldowns {
                attack_cooldown: enemy.attack.cooldown,
                respawn_timer: enemy.respawn_timer,
            },
        ),
    }
}

pub(crate) fn sync_actor_components_from_runtime(
    actor: &ActorRuntime,
    identity: &mut ActorIdentity,
    disposition: &mut ActorDisposition,
    health: &mut ActorHealth,
    combat: &mut ActorCombatState,
    intent: &mut ActorIntent,
    cooldowns: &mut ActorCooldowns,
) {
    let (next_identity, next_disposition, next_health, next_combat, next_intent, next_cooldowns) =
        actor_component_snapshot(actor);
    *identity = next_identity;
    *disposition = next_disposition;
    *health = next_health;
    *combat = next_combat;
    *intent = next_intent;
    *cooldowns = next_cooldowns;
}

/// Keep actor-like gameplay poses in sync with the authoritative [`FeatureAabb`].
///
/// `ActorPose` is the gameplay action-origin read model used by the universal
/// brain/action resolver. Presentation `Transform`s are intentionally not the
/// source of truth for sim entities; they belong to rendered visual entities and
/// may have sprite anchors, scale, parent transforms, or cached bindings applied.
pub fn sync_actor_poses_from_feature_aabbs(
    mut actors: Query<
        (
            &FeatureAabb,
            &mut super::super::components::ActorPose,
            Option<&ActorRuntime>,
            Option<&BossFeature>,
        ),
        With<FeatureSimEntity>,
    >,
) {
    for (aabb, mut pose, actor, boss) in &mut actors {
        let facing = actor
            .map(ActorRuntime::facing)
            .or_else(|| boss.map(|feature| feature.boss.facing))
            .unwrap_or(pose.facing);
        *pose = super::super::components::ActorPose::from_aabb(*aabb, facing);
    }
}

/// Tick ECS actors. Peaceful and hostile actors share the same entity identity
/// and can switch disposition in-place; dynamic encounter-spawned mobs use the
/// same `ActorRuntime::Enemy` path with an `EncounterMob` marker.
pub fn update_ecs_actors(
    mut commands: Commands,
    world_time: Res<WorldTime>,
    world: Res<crate::GameWorld>,
    platform_set: Res<crate::MovingPlatformSet>,
    feel_tuning: Res<crate::time::feel::SandboxFeelTuning>,
    overlay: Res<FeatureEcsWorldOverlay>,
    mut slot_board: ResMut<crate::combat_slots::CombatSlotsRes>,
    mut sfx: MessageWriter<crate::audio::SfxMessage>,
    mut vfx: MessageWriter<crate::presentation::fx::VfxMessage>,
    mut debris: MessageWriter<DebrisBurstMessage>,
    mut hit_events: MessageWriter<HitEvent>,
    // Multi-player ready: iterate every player and resolve each
    // actor's body-contact check against the player its `ActorTarget`
    // points at. The combat slot board (which arbitrates which enemy
    // commits to an attack this tick) still anchors on a single
    // global "target player" position — primary today; per-player
    // slot boards are a follow-up. Single-player behavior is
    // identical because there's only one player.
    player_query: Query<
        (
            bevy::prelude::Entity,
            &crate::player::PlayerKinematics,
            &crate::player::PlayerOffense,
            &crate::player::PlayerDodgeState,
            &crate::player::PlayerShieldState,
            &crate::player::PlayerCombatState,
        ),
        bevy::prelude::With<crate::player::PlayerEntity>,
    >,
    primary_q: bevy::prelude::Query<
        bevy::prelude::Entity,
        (
            bevy::prelude::With<crate::player::PlayerEntity>,
            bevy::prelude::With<crate::player::PrimaryPlayer>,
        ),
    >,
    mut actors: Query<
        (
            Entity,
            &mut FeatureAabb,
            &mut ActorRuntime,
            &mut ActorIdentity,
            &mut ActorDisposition,
            &mut ActorHealth,
            &mut ActorCombatState,
            &mut ActorIntent,
            &mut ActorCooldowns,
            &super::super::components::ActorTarget,
            // Brain + ActorControl. The hostile tick runs the brain
            // and writes its `ActorControlFrame` output into
            // `ActorControl` so the downstream
            // `emit_brain_action_messages` resolver and the EFFECTS-
            // stage consumers see the brain's intent. `Option` on
            // both because dynamically-spawned actors (debug tools,
            // scripted spawns) might skip brain attachment.
            Option<&mut crate::brain::Brain>,
            Option<&mut crate::brain::ActorControl>,
            // ActionSet — read for the Smash brain so it knows which
            // attacks (melee / ranged) the actor can commit. `Option`
            // so dynamically-spawned actors without a set still tick.
            Option<&crate::brain::ActionSet>,
            Option<&super::Mounted>,
        ),
        With<FeatureSimEntity>,
    >,
) {
    // Sim clock: enemies, NPCs, encounter mobs all advance on the
    // gameplay clock so bullet-time / pause / hitstop freeze them
    // alongside the player. ADR 0010 + reference_lessons_learned.
    let dt = world_time.sim_dt();
    let feature_world = world_with_sandbox_solids(&world.0, &platform_set.0, &overlay);
    // Pick the slot-board anchor: the primary player by default, or
    // fall back to the first available player so combat slot
    // assignment still works on a multi-player non-primary build.
    let primary_entity = primary_q.single().ok();
    let slot_anchor_pos = primary_entity
        .and_then(|e| player_query.get(e).ok())
        .or_else(|| player_query.iter().next())
        .map(|(_, kin, _, _, _, _)| kin.pos);
    let Some(player_pos) = slot_anchor_pos else {
        return;
    };

    // Pass 1: collect slot requests from every live hostile enemy.
    // The slot board is per-target (player) and arbitrates which
    // enemies are allowed to commit to an attack this tick; the
    // others hold at the outer ring. This is the anti-clump layer.
    let mut requests: Vec<(String, ae::Vec2, crate::combat_slots::SlotKind)> = Vec::new();
    for (_, _, actor, _, _, _, _, _, _, _, _, _, _, _) in &actors {
        if let ActorRuntime::Enemy(enemy) = actor {
            if enemy.alive {
                requests.push((enemy.id.clone(), enemy.pos, enemy.archetype.slot_kind()));
            }
        }
    }
    let slot_requests: Vec<crate::combat_slots::SlotRequest> = requests
        .iter()
        .map(|(id, pos, kind)| crate::combat_slots::SlotRequest {
            actor_id: id.as_str(),
            actor_pos: *pos,
            kind: *kind,
        })
        .collect();
    crate::combat_slots::assign_slots(&mut slot_board.0, player_pos, &slot_requests);

    // Per-kind holding-position fallback: when an actor doesn't win
    // a slot, distribute the leftover actors across the holding
    // positions of all slots of their kind. Stable, deterministic
    // ordering by actor id so the assignment doesn't flicker
    // between frames.
    //
    // Without this, multiple unassigned actors of the same kind all
    // picked `slots.iter().find()`'s FIRST matching slot's
    // `holding_pos` — i.e. they shared a single fallback point and
    // visually clumped.
    let mut unassigned_by_kind: std::collections::HashMap<
        crate::combat_slots::SlotKind,
        Vec<&str>,
    > = std::collections::HashMap::new();
    for (id, _pos, kind) in &requests {
        if slot_board.0.slot_for(id).is_none() {
            unassigned_by_kind
                .entry(*kind)
                .or_default()
                .push(id.as_str());
        }
    }
    let mut holding_pos_by_id: std::collections::HashMap<String, ae::Vec2> =
        std::collections::HashMap::new();
    for (kind, mut ids) in unassigned_by_kind {
        let kind_slots: Vec<usize> = slot_board
            .0
            .slots
            .iter()
            .enumerate()
            .filter(|(_, s)| s.kind == kind)
            .map(|(i, _)| i)
            .collect();
        if kind_slots.is_empty() {
            continue;
        }
        ids.sort_unstable(); // stable round-robin order
        for (rank, id) in ids.into_iter().enumerate() {
            let slot_idx = kind_slots[rank % kind_slots.len()];
            holding_pos_by_id.insert(
                id.to_string(),
                slot_board.0.slots[slot_idx].holding_pos(player_pos),
            );
        }
    }

    // Per-actor nearest-same-kind-neighbor index (O(N²), N ≤ a few).
    // Used by brain snapshots as a "personal space" signal so two
    // aerial actors close to each other can push apart even when their
    // slot anchors are far apart.
    let mut neighbor_by_id: std::collections::HashMap<String, ae::Vec2> =
        std::collections::HashMap::new();
    for (id_a, pos_a, kind_a) in &requests {
        let mut nearest: Option<(f32, ae::Vec2)> = None;
        for (id_b, pos_b, kind_b) in &requests {
            if id_a == id_b || kind_a != kind_b {
                continue;
            }
            let d = (*pos_a - *pos_b).length_squared();
            if nearest.map(|(best, _)| d < best).unwrap_or(true) {
                nearest = Some((d, *pos_b));
            }
        }
        if let Some((_, pos)) = nearest {
            neighbor_by_id.insert(id_a.clone(), pos);
        }
    }

    // Per-actor crowding signal for brains that need personal space.
    // Aerial actors use a wider radius and only react to other aerial
    // actors, which keeps sharks from stacking while preserving the
    // tighter ground crowding Smash expects.
    const CROWDING_RADIUS_PX: f32 = 80.0;
    const AERIAL_CROWDING_RADIUS_PX: f32 = 220.0;
    let mut crowding_by_id: std::collections::HashMap<String, crate::brain::CrowdingSignal> =
        std::collections::HashMap::new();
    for (id_a, pos_a, kind_a) in &requests {
        let mut count: u8 = 0;
        let mut centroid = ae::Vec2::ZERO;
        let aerial = *kind_a == crate::combat_slots::SlotKind::Aerial;
        let radius = if aerial {
            AERIAL_CROWDING_RADIUS_PX
        } else {
            CROWDING_RADIUS_PX
        };
        for (id_b, pos_b, kind_b) in &requests {
            if id_a == id_b {
                continue;
            }
            if aerial && *kind_b != crate::combat_slots::SlotKind::Aerial {
                continue;
            }
            if pos_a.distance_squared(*pos_b) <= radius * radius {
                count = count.saturating_add(1);
                centroid += *pos_b;
            }
        }
        if count > 0 {
            centroid /= count as f32;
            let away = (*pos_a - centroid).normalize_or_zero();
            crowding_by_id.insert(
                id_a.clone(),
                crate::brain::CrowdingSignal {
                    same_faction_count: count,
                    other_faction_count: 0,
                    away_dir: away,
                    pressure: crate::brain::CrowdingSignal::compute_pressure(count, 0),
                },
            );
        }
    }

    // Pass 2: tick each actor with its assigned slot position. Falls
    // back to the slot's holding-ring position when this actor didn't
    // win a slot so it still has a sensible steering target.
    let combat_tuning = feel_tuning.feature_combat_tuning();
    // Brain templates with persistent timers (Wanderer's chatter
    // window, Skirmisher's fire cooldown) need an absolute clock.
    // No actor uses those today through this system — NPCs run
    // Patrol / StandStill which don't read sim_time — so 0.0 is
    // safe. The seam will accept a real clock when the first
    // Wanderer-driven actor migrates (puppy slug, daytime).
    let sim_time = 0.0;
    for (
        actor_entity,
        mut aabb,
        mut actor,
        mut identity,
        mut disposition,
        mut health,
        mut combat,
        mut intent,
        mut cooldowns,
        target,
        mut brain,
        mut control,
        action_set,
        mounted,
    ) in &mut actors
    {
        // `target.pos` is populated by `select_actor_targets`
        // (#17.8); it defaults to the actor's spawn-of-game position
        // when no players exist yet (pre-spawn / post-death-of-all),
        // and is the primary player's pos in the single-player
        // production game.
        let target_pos = target.pos;
        match &mut *actor {
            ActorRuntime::Npc(npc) => {
                let frame = if let Some(brain) = brain.as_deref_mut() {
                    npc.tick_via_brain(brain, &feature_world, target_pos, sim_time, dt)
                } else {
                    // Brainless peaceful actor — should not happen
                    // post-Chunk 3 (spawn attaches a brain), but
                    // fall back to building one inline so the tick
                    // is never skipped if components drift.
                    let mut fallback = npc.build_brain();
                    npc.tick_via_brain(&mut fallback, &feature_world, target_pos, sim_time, dt)
                };
                // Land the brain's frame in ActorControl so
                // `emit_brain_action_messages` and downstream
                // EFFECTS consumers see it.
                if let Some(control) = control.as_deref_mut() {
                    control.0 = frame;
                }
                aabb.center = npc.pos;
                aabb.half_size = npc.size * 0.5;
            }
            ActorRuntime::Enemy(enemy) => {
                let slot_pos = if let Some(slot) = slot_board.0.slot_for(&enemy.id) {
                    Some(slot.world_pos(target_pos))
                } else if enemy.alive {
                    // No slot assigned — fall back to the per-actor
                    // holding-ring position computed above. Multiple
                    // unassigned actors of the same kind are spread
                    // round-robin across all holding positions of
                    // that kind rather than sharing slot 0.
                    holding_pos_by_id.get(&enemy.id).copied()
                } else {
                    None
                };
                let nearest_neighbor = neighbor_by_id.get(&enemy.id).copied();
                // Capture pre-tick attack state so we can detect
                // the windup → active edge below. The runtime's
                // `update_attack_timers` is the only path that
                // performs the transition (windup → active), so
                // observing the edge lets us spawn the strike's
                // `Hitbox` entity exactly once per begin-attack
                // instead of polling overlap every tick.
                let was_winding_up = enemy.attack.is_winding_up();
                let was_active = enemy.attack.is_active();

                // Every brain-attached enemy ticks its brain FIRST to
                // build an authoritative frame, then calls
                // `enemy.update` with that frame. The frame drives
                // the integration step (Patrol / Chase / Approach all
                // actually move the actor) AND lands in `ActorControl`
                // for the EFFECTS consumers (so melee + ranged fire).
                // Smash + Patrol + MeleeBrute + Skirmisher + Sniper +
                // Wanderer all flow through this single path; the
                // legacy `build_control_frame` branch inside
                // `EnemyRuntime::update` was deleted in the
                // brain-authority GC pass.
                //
                // Actors without a brain (dynamically-spawned debug
                // entities) get a neutral frame and stand still —
                // production spawn paths always attach a brain, so
                // this is the safe no-op fallback.
                let _ = slot_pos;
                let is_mounted = mounted.is_some();
                let previous_pos = enemy.pos;
                let brain_frame = if let Some(brain_ref) = brain.as_deref_mut() {
                    let crowding = crowding_by_id.get(&enemy.id).copied();
                    let snapshot = build_enemy_brain_snapshot(enemy, target_pos, crowding, dt);
                    let mut bf = crate::actor_control::ActorControlFrame::neutral();
                    let peaceful = crate::brain::ActionSet::peaceful();
                    let actions = action_set.unwrap_or(&peaceful);
                    brain_ref.tick_with_actions(actions, &snapshot, &mut bf);
                    bf
                } else {
                    crate::actor_control::ActorControlFrame::neutral()
                };
                let body_contact_damage_enabled =
                    !brain.as_deref().is_some_and(crate::brain::Brain::is_player)
                        && enemy.archetype.body_contact_damage_enabled();
                let mut brain_frame = brain_frame;
                brain_frame.body_contact_damage_enabled = body_contact_damage_enabled;
                let shark_charge_vec = brain_frame.desired_vel;

                let frame = enemy.update(
                    &feature_world,
                    target_pos,
                    combat_tuning,
                    nearest_neighbor,
                    dt,
                    is_mounted,
                    brain_frame,
                );
                let shark_crashed =
                    shark_charge_crashed(enemy, is_mounted, shark_charge_vec, previous_pos);
                let mut frame = frame;
                if shark_crashed {
                    hit_events.write(HitEvent {
                        volume: enemy.aabb(),
                        damage: enemy.health.current.max(1),
                        source: HitSource::EnemyChargeCrash,
                        attacker: None,
                        target: HitTarget::Volume,
                        mode: HitMode::Knockback,
                        knockback: None,
                        ignored_targets: Vec::new(),
                    });
                    frame = crate::actor_control::ActorControlFrame::neutral();
                }
                aabb.center = enemy.pos;
                aabb.half_size = enemy.size * 0.5;

                if let Some(control) = control.as_deref_mut() {
                    control.0 = frame;
                }
                // Active-edge: windup just finished AND attack
                // timer is now positive (and wasn't already). Spawn
                // the strike's `Hitbox` entity here so the overlap
                // check moves to `apply_hitbox_damage` instead of
                // polling every frame from this system.
                if was_winding_up
                    && !enemy.attack.is_winding_up()
                    && enemy.attack.is_active()
                    && !was_active
                    && enemy.alive
                {
                    // Directional swing: read the axis the brain
                    // committed to in `begin_melee_attack`. Forward
                    // axis falls back to the actor's facing; up /
                    // down attacks place the hitbox above / below
                    // the body instead of in front.
                    let attack_box = enemy.attack_aabb_dir(enemy.attack.pending_axis);
                    let local_offset = attack_box.center() - enemy.pos;
                    super::hitbox::spawn_melee_hitbox(
                        &mut commands,
                        actor_entity,
                        super::super::components::ActorFaction::Enemy,
                        local_offset,
                        attack_box.half_size(),
                        1,
                        1.0,
                        enemy.attack.active_timer,
                    );
                }
                // Projectile spawns moved to the EFFECTS-stage
                // consumer `spawn_enemy_projectiles_from_brain_actions`
                // (Combat set, runs after `emit_brain_action_messages`).
                // The attack-swing damage check moved to the
                // Hitbox entity lifecycle (see above). Body-contact
                // damage stays polled — "you ran into the enemy"
                // is a per-tick integration test, not a discrete
                // strike.
                // Per-actor vulnerability + body check: look up the
                // player this enemy is currently tracking. Falls back
                // to no-op when the target's player entity is None
                // (e.g. dropped-to-volume targeting in test fixtures).
                // `target_entity` is threaded onto the emitted
                // `HitEvent::target` so the player-side reader lands
                // body-contact damage on this specific player rather
                // than falling back to primary.
                let Some(target_entity) = target.entity else {
                    continue;
                };
                let Ok((_, target_kin, target_offense, target_dodge, target_shield, target_combat)) =
                    player_query.get(target_entity)
                else {
                    continue;
                };
                let target_body = target_kin.aabb();
                let target_dodge_rolling = target_dodge.roll_timer > 0.0;
                let target_vulnerable = !target_offense.invincible
                    && !target_dodge_rolling
                    && !target_shield.parrying()
                    && target_combat.vulnerable();
                if target_vulnerable && enemy.alive && body_contact_damage_enabled {
                    if let Some(damage) = enemy.body_contact_damage(target_entity, target_body) {
                        let pos = damage
                            .knockback
                            .as_ref()
                            .map(|k| k.impact_pos)
                            .unwrap_or_else(|| damage.volume.center());
                        sfx.write(crate::audio::SfxMessage::Play {
                            id: ambition_sfx::ids::PLAYER_DAMAGE,
                            pos,
                        });
                        vfx.write(VfxMessage::Impact { pos });
                        vfx.write(VfxMessage::Burst {
                            pos,
                            count: 14,
                            speed: 300.0,
                            color: [1.0, 0.34, 0.28, 0.88],
                            kind: ParticleKind::Shard,
                        });
                        debris.write(DebrisBurstMessage {
                            pos,
                            cue: PhysicsDebrisCue::Impact,
                        });
                        hit_events.write(damage);
                    }
                }
            }
        }
        sync_actor_components_from_runtime(
            &actor,
            &mut identity,
            &mut disposition,
            &mut health,
            &mut combat,
            &mut intent,
            &mut cooldowns,
        );
    }
}

/// Build a `BrainSnapshot` for a Smash-brain enemy. Threads the
/// crowding signal computed once per tick by the actor driver.
/// `dt` is the gameplay clock so the Smash brain's mode dwell
/// accumulator runs on the same time domain as the rest of the
/// simulation.
/// Build a `BrainSnapshot` for an enemy actor's per-tick brain call.
/// Carries the per-frame body / target / cooldown view every brain
/// backend reads from; `crowding` is only consulted by the Smash
/// brain, but always populating it keeps the snapshot uniform across
/// state-machine variants.
fn build_enemy_brain_snapshot(
    enemy: &crate::content::features::EnemyRuntime,
    target_pos: ae::Vec2,
    crowding: Option<crate::brain::CrowdingSignal>,
    dt: f32,
) -> crate::brain::BrainSnapshot {
    crate::brain::BrainSnapshot {
        actor_pos: enemy.pos,
        actor_vel: enemy.vel,
        actor_facing: enemy.facing,
        actor_on_ground: enemy.surface.on_ground,
        alive: enemy.alive,
        target_pos,
        target_alive: true,
        sim_time: 0.0,
        dt,
        attack_cooldown_remaining: enemy.attack.cooldown,
        attack_windup_remaining: enemy.attack.windup_timer,
        attack_active_remaining: enemy.attack.active_timer,
        attack_recover_remaining: 0.0,
        stun_remaining: 0.0,
        wall_contact: None,
        player_input: None,
        crowding,
        terrain: None,
        air_jumps_remaining: enemy.surface.air_jumps_remaining,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn burning_shark_enemy() -> EnemyRuntime {
        let aabb = ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(126.0, 52.0));
        EnemyRuntime::new(
            "burning_shark".to_string(),
            "Burning Shark".to_string(),
            aabb,
            crate::actor::EnemyBrain::Custom("burning_flying_shark".into()),
            &[],
        )
    }

    #[test]
    fn sync_actor_pose_uses_feature_aabb_and_actor_facing() {
        use bevy::prelude::{App, Update};

        let mut app = App::new();
        app.add_systems(Update, sync_actor_poses_from_feature_aabbs);

        let mut enemy = burning_shark_enemy();
        enemy.facing = -1.0;
        let entity = app
            .world_mut()
            .spawn((
                FeatureSimEntity,
                FeatureAabb::from_center_size(ae::Vec2::new(40.0, 80.0), ae::Vec2::new(20.0, 30.0)),
                crate::features::ActorPose::default(),
                ActorRuntime::Enemy(enemy),
            ))
            .id();

        app.update();

        let entity_ref = app.world().entity(entity);
        let pose = entity_ref.get::<crate::features::ActorPose>().unwrap();
        assert_eq!(pose.center, ae::Vec2::new(40.0, 80.0));
        assert_eq!(pose.feet, ae::Vec2::new(40.0, 95.0));
        assert_eq!(pose.facing, -1.0);
        assert!(
            entity_ref
                .get::<bevy::transform::components::Transform>()
                .is_none(),
            "ActorPose sync should not require a gameplay Transform shim"
        );
    }

    #[test]
    fn shark_charge_crash_detects_solo_charge_wall_hit() {
        let mut enemy = burning_shark_enemy();
        let previous_pos = ae::Vec2::new(120.0, 80.0);
        enemy.pos = previous_pos;
        enemy.vel = ae::Vec2::ZERO;
        enemy.alive = true;
        let charge_vec = ae::Vec2::new(enemy.archetype.chase_speed() * 2.0, 0.0);
        assert!(shark_charge_crashed(
            &enemy,
            false,
            charge_vec,
            previous_pos
        ));
    }

    #[test]
    fn shark_charge_crash_ignores_mounted_or_noncharge_cases() {
        let mut enemy = burning_shark_enemy();
        let previous_pos = ae::Vec2::new(120.0, 80.0);
        enemy.pos = previous_pos;
        enemy.vel = ae::Vec2::ZERO;
        enemy.alive = true;
        let charge_vec = ae::Vec2::new(enemy.archetype.chase_speed() * 2.0, 0.0);
        assert!(!shark_charge_crashed(
            &enemy,
            true,
            charge_vec,
            previous_pos
        ));
        assert!(!shark_charge_crashed(
            &enemy,
            false,
            ae::Vec2::new(enemy.archetype.chase_speed(), 0.0),
            previous_pos
        ));
    }
}
