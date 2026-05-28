//! ECS actor types and the per-frame actor tick.
//!
//! `ActorRuntime` is the unified component that backs every authored
//! NPC, authored hostile enemy, and dynamic encounter-spawned mob.
//! Peaceful and hostile actors share the same entity identity so a
//! peaceful NPC can flip to hostile in place after enough strikes
//! rather than being moved between containers.

use super::*;

/// Unified ECS runtime for authored NPCs and enemies.
///
/// The only meaningful gameplay distinction is disposition: peaceful actors
/// talk / patrol, hostile actors chase / attack. A peaceful NPC can flip into
/// the hostile branch in-place after enough strikes instead of being removed
/// from one runtime vector and reinserted into another.
#[derive(Component, Clone, Debug)]
pub enum ActorRuntime {
    Peaceful(NpcRuntime),
    Hostile(EnemyRuntime),
}

impl ActorRuntime {
    pub fn id(&self) -> &str {
        match self {
            Self::Peaceful(actor) => actor.id.as_str(),
            Self::Hostile(actor) => actor.id.as_str(),
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Peaceful(actor) => actor.name.as_str(),
            Self::Hostile(actor) => actor.name.as_str(),
        }
    }

    pub fn aabb(&self) -> ae::Aabb {
        match self {
            Self::Peaceful(actor) => actor.aabb(),
            Self::Hostile(actor) => actor.aabb(),
        }
    }

    pub fn pos(&self) -> ae::Vec2 {
        match self {
            Self::Peaceful(actor) => actor.pos,
            Self::Hostile(actor) => actor.pos,
        }
    }

    pub fn size(&self) -> ae::Vec2 {
        match self {
            Self::Peaceful(actor) => actor.size,
            Self::Hostile(actor) => actor.size,
        }
    }

    pub fn disposition(&self) -> ActorDisposition {
        match self {
            Self::Peaceful(_) => ActorDisposition::Peaceful,
            Self::Hostile(_) => ActorDisposition::Hostile,
        }
    }

    pub fn visual_kind(&self) -> FeatureVisualKind {
        match self {
            Self::Peaceful(_) => FeatureVisualKind::Npc,
            Self::Hostile(enemy) => enemy.visual_kind(),
        }
    }

    pub fn visible(&self) -> bool {
        match self {
            Self::Peaceful(_) => true,
            Self::Hostile(enemy) => enemy.alive,
        }
    }

    pub fn flash(&self) -> bool {
        match self {
            Self::Peaceful(npc) => npc.hit_flash > 0.0,
            Self::Hostile(enemy) => {
                enemy.hit_flash > 0.0 || enemy.attack_windup_timer > 0.0 || enemy.attack_timer > 0.0
            }
        }
    }

    pub fn feature_view(&self) -> FeatureView {
        let rotation_rad = match self {
            Self::Peaceful(_) => 0.0,
            Self::Hostile(enemy) => enemy.rotation_rad(),
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

    pub(crate) fn hostile_from_npc(npc: &NpcRuntime) -> EnemyRuntime {
        let mut enemy = EnemyRuntime::new(
            npc.id.clone(),
            npc.name.clone(),
            npc.aabb(),
            ae::EnemyBrain::Custom("medium_striker".into()),
            &[],
        );
        enemy.pos = npc.pos;
        enemy.spawn = npc.spawn;
        enemy.size = ae::Vec2::new(npc.size.x.max(22.0), npc.size.y.max(38.0));
        enemy.vel = npc.vel;
        enemy.facing = npc.facing;
        enemy.on_ground = npc.on_ground;
        if npc.name != "Kernel Guide NPC" {
            enemy.sprite_override_npc_name = Some(npc.name.clone());
        }
        enemy
    }
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
        ActorRuntime::Peaceful(npc) => (
            ActorIdentity::new(npc.id.clone(), npc.name.clone()),
            ActorDisposition::Peaceful,
            ActorHealth::new(ae::Health::new(1)),
            ActorCombatState::peaceful(npc.strikes, npc.hit_flash),
            ActorIntent::new(crate::character_ai::CharacterAiMode::Idle),
            ActorCooldowns::default(),
        ),
        ActorRuntime::Hostile(enemy) => (
            ActorIdentity::new(enemy.id.clone(), enemy.name.clone())
                .with_sprite_override(enemy.sprite_override_npc_name.clone()),
            ActorDisposition::Hostile,
            ActorHealth::new(enemy.health),
            ActorCombatState::hostile(
                enemy.alive,
                enemy.hit_flash,
                enemy.attack_windup_timer,
                enemy.attack_timer,
                enemy.archetype.is_sandbag(),
            ),
            ActorIntent::new(enemy.ai_mode),
            ActorCooldowns {
                attack_cooldown: enemy.attack_cooldown,
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

/// Tick ECS actors. Peaceful and hostile actors share the same entity identity
/// and can switch disposition in-place; dynamic encounter-spawned mobs use the
/// same `ActorRuntime::Hostile` path with an `EncounterMob` marker.
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
    mut player_damage: MessageWriter<PlayerDamageEvent>,
    // Enemies target the primary player today. Real "nearest hostile
    // actor of faction Player" target selection is OVERNIGHT-TODO
    // #17.8; the `PrimaryPlayerOnly` filter documents the targeting
    // decision at the query so a future per-actor `ActorTarget`
    // component lands as a query change, not a semantic shift.
    player_query: Query<
        (
            &crate::player::PlayerKinematics,
            &crate::player::PlayerOffense,
            &crate::player::PlayerDodgeState,
            &crate::player::PlayerShieldState,
            &crate::player::PlayerCombatState,
        ),
        crate::player::PrimaryPlayerOnly,
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
        ),
        With<FeatureSimEntity>,
    >,
) {
    // Sim clock: enemies, NPCs, encounter mobs all advance on the
    // gameplay clock so bullet-time / pause / hitstop freeze them
    // alongside the player. ADR 0010 + reference_lessons_learned.
    let dt = world_time.sim_dt();
    let feature_world = world_with_sandbox_solids(&world.0, &platform_set.0, &overlay);
    let Ok((kin, offense, dodge, shield, combat)) = player_query.single() else {
        return;
    };
    let player_pos = kin.pos;
    let player_body = kin.aabb();
    let dodge_rolling = dodge.roll_timer > 0.0;
    let player_vulnerable = !offense.invincible
        && !dodge_rolling
        && !shield.parrying()
        && combat.vulnerable();

    // Pass 1: collect slot requests from every live hostile enemy.
    // The slot board is per-target (player) and arbitrates which
    // enemies are allowed to commit to an attack this tick; the
    // others hold at the outer ring. This is the anti-clump layer.
    let mut requests: Vec<(String, ae::Vec2, ae::SlotKind)> = Vec::new();
    for (_, _, actor, _, _, _, _, _, _, _, _, _, _) in &actors {
        if let ActorRuntime::Hostile(enemy) = actor {
            if enemy.alive {
                requests.push((enemy.id.clone(), enemy.pos, enemy.archetype.slot_kind()));
            }
        }
    }
    let slot_requests: Vec<ae::SlotRequest> = requests
        .iter()
        .map(|(id, pos, kind)| ae::SlotRequest {
            actor_id: id.as_str(),
            actor_pos: *pos,
            kind: *kind,
        })
        .collect();
    ae::assign_slots(&mut slot_board.0, player_pos, &slot_requests);

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
    let mut unassigned_by_kind: std::collections::HashMap<ae::SlotKind, Vec<&str>> =
        std::collections::HashMap::new();
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
    // Used by the choreography for "personal space" steering so two
    // aerial actors close to each other push apart even when their
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

    // Per-actor crowding signal for Smash-brain enemies. Every entry
    // in `requests` is a same-faction (Enemy) actor since `requests`
    // is built from hostile actors only, so the count here IS the
    // same-faction count for Smash. Other-faction crowding (player,
    // peaceful NPCs) is left as 0 for now — the user's design weights
    // same-faction as the dominant signal anyway.
    const CROWDING_RADIUS_PX: f32 = 80.0;
    let mut crowding_by_id: std::collections::HashMap<String, crate::brain::CrowdingSignal> =
        std::collections::HashMap::new();
    for (id_a, pos_a, _) in &requests {
        let mut count: u8 = 0;
        let mut centroid = ae::Vec2::ZERO;
        for (id_b, pos_b, _) in &requests {
            if id_a == id_b {
                continue;
            }
            if pos_a.distance_squared(*pos_b) <= CROWDING_RADIUS_PX * CROWDING_RADIUS_PX {
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
    ) in &mut actors
    {
        // `target.pos` is populated by `select_actor_targets`
        // (#17.8); it defaults to the actor's spawn-of-game position
        // when no players exist yet (pre-spawn / post-death-of-all),
        // and is the primary player's pos in the single-player
        // production game.
        let target_pos = target.pos;
        match &mut *actor {
            ActorRuntime::Peaceful(npc) => {
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
            ActorRuntime::Hostile(enemy) => {
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
                let was_winding_up = enemy.attack_windup_timer > 0.0;
                let was_active = enemy.attack_timer > 0.0;

                // Smash-brain actors: run the brain FIRST to build
                // an authoritative frame, then call `enemy.update`
                // with that frame as the override. The override
                // drives the integration step (so movement follows
                // the brain — Approach, Retreat, Reposition all
                // actually move the actor) AND lands in
                // `ActorControl` for the EFFECTS consumers (so
                // attacks fire). Non-Smash actors keep the legacy
                // flow: `enemy.update` builds + integrates its own
                // frame.
                let brain_override = if let Some(brain_ref) = brain.as_deref_mut() {
                    if matches!(
                        brain_ref,
                        crate::brain::Brain::StateMachine(
                            crate::brain::StateMachineCfg::Smash { .. }
                        )
                    ) {
                        let crowding = crowding_by_id.get(&enemy.id).copied();
                        let snapshot =
                            build_smash_snapshot(enemy, target_pos, crowding, dt);
                        let mut brain_frame = ae::ActorControlFrame::neutral();
                        let peaceful = crate::brain::ActionSet::peaceful();
                        let actions = action_set.unwrap_or(&peaceful);
                        brain_ref.tick_with_actions(actions, &snapshot, &mut brain_frame);
                        Some(brain_frame)
                    } else {
                        None
                    }
                } else {
                    None
                };

                let frame = enemy.update(
                    &feature_world,
                    target_pos,
                    combat_tuning,
                    slot_pos,
                    nearest_neighbor,
                    dt,
                    brain_override,
                );
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
                    && enemy.attack_windup_timer <= 0.0
                    && enemy.attack_timer > 0.0
                    && !was_active
                    && enemy.alive
                {
                    // Directional swing: read the axis the brain
                    // committed to in `begin_melee_attack`. Forward
                    // axis falls back to the actor's facing; up /
                    // down attacks place the hitbox above / below
                    // the body instead of in front.
                    let attack_box = enemy.attack_aabb_dir(enemy.pending_attack_axis);
                    let local_offset = attack_box.center() - enemy.pos;
                    super::hitbox::spawn_melee_hitbox(
                        &mut commands,
                        actor_entity,
                        super::super::components::ActorFaction::Enemy,
                        local_offset,
                        attack_box.half_size(),
                        1,
                        1.0,
                        enemy.attack_timer,
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
                if player_vulnerable && enemy.alive {
                    if let Some(damage) = enemy.body_contact_damage(player_body) {
                        let pos = damage.impact_pos;
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
                        player_damage.write(damage);
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
fn build_smash_snapshot(
    enemy: &crate::content::features::EnemyRuntime,
    target_pos: ae::Vec2,
    crowding: Option<crate::brain::CrowdingSignal>,
    dt: f32,
) -> crate::brain::BrainSnapshot {
    crate::brain::BrainSnapshot {
        actor_pos: enemy.pos,
        actor_vel: enemy.vel,
        actor_facing: enemy.facing,
        actor_on_ground: enemy.on_ground,
        alive: enemy.alive,
        target_pos,
        target_alive: true,
        sim_time: 0.0,
        dt,
        attack_cooldown_remaining: enemy.attack_cooldown,
        attack_windup_remaining: enemy.attack_windup_timer,
        attack_active_remaining: enemy.attack_timer,
        attack_recover_remaining: 0.0,
        stun_remaining: 0.0,
        wall_contact: None,
        player_input: None,
        crowding,
        terrain: None,
        air_jumps_remaining: enemy.air_jumps_remaining,
    }
}
