//! The per-frame actor tick: syncing poses from feature AABBs, driving the
//! enemy + NPC updates, neighbor/crowding queries, and brain snapshots.

use super::super::*;
use super::*;

/// Keep actor-like gameplay poses in sync with the authoritative [`CenteredAabb`].
///
/// `ActorPose` is the gameplay action-origin read model used by the universal
/// brain/action resolver. Presentation `Transform`s are intentionally not the
/// source of truth for sim entities; they belong to rendered visual entities and
/// may have sprite anchors, scale, parent transforms, or cached bindings applied.
pub fn sync_actor_poses_from_feature_aabbs(
    mut actors: Query<
        (
            &CenteredAabb,
            &mut super::super::super::components::ActorPose,
            Option<&ActorRuntime>,
            Option<&super::super::enemy_clusters::BodyKinematics>,
            Option<super::super::boss_clusters::BossClusterRef>,
        ),
        With<FeatureSimEntity>,
    >,
) {
    for (aabb, mut pose, actor, kin, boss) in &mut actors {
        // Facing source: enemy clusters (BodyKinematics), NPC runtime,
        // or boss runtime; default to the current pose facing.
        let facing = match actor {
            // NPCs and enemies both carry the shared `BodyKinematics`
            // component, so facing reads from `kin` for either marker.
            Some(ActorRuntime::Npc) | Some(ActorRuntime::Enemy) => {
                kin.map(|k| k.facing).unwrap_or(pose.facing)
            }
            None => boss
                .map(|feature| feature.kin.facing)
                .unwrap_or(pose.facing),
        };
        *pose =
            super::super::super::components::ActorPose::from_parts(aabb.center, aabb.half_size, facing);
    }
}

/// Tick ECS actors. Peaceful and hostile actors share the same entity identity
/// and can switch disposition in-place; dynamic encounter-spawned mobs use the
/// same `ActorRuntime::Enemy` path with an `EncounterMob` marker.
pub fn update_ecs_actors(
    mut commands: Commands,
    world_time: Res<WorldTime>,
    world: Res<crate::GameWorld>,
    gravity: crate::physics::GravityCtx,
    platform_set: Res<crate::MovingPlatformSet>,
    feel_tuning: Res<crate::time::feel::SandboxFeelTuning>,
    overlay: Res<FeatureEcsWorldOverlay>,
    mut slot_board: ResMut<crate::combat::slots::CombatSlotsRes>,
    mut sfx: MessageWriter<crate::audio::SfxMessage>,
    mut vfx: MessageWriter<ambition_effects::vfx::VfxMessage>,
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
            &crate::player::BodyKinematics,
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
            &mut CenteredAabb,
            &mut ActorRuntime,
            &mut ActorIdentity,
            &mut ActorDisposition,
            &mut ActorHealth,
            &mut ActorCombatState,
            &mut ActorIntent,
            &mut ActorCooldowns,
            &super::super::super::components::ActorTarget,
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
            Option<&super::super::Mounted>,
            // Enemy cluster components — `None` on NPC actors. The
            // enemy branch runs its integration through these via
            // `EnemyMut`.
            //
            // `Possessed` is nested with the cluster data (not a new top-level
            // tuple field) to stay within Bevy's query-tuple arity: when set,
            // the actor is driven from the player's input instead of its brain
            // (`crate::abilities::traversal::possession`).
            (
                Option<super::super::enemy_clusters::EnemyClusterQueryData>,
                Option<&crate::abilities::traversal::possession::Possessed>,
            ),
        ),
        // The player carries the unified `BodyKinematics` too, and
        // `player_query` above reads it; exclude the player here so this
        // `&mut BodyKinematics` actor query is provably disjoint from it
        // (player / actor archetypes never overlap).
        (With<FeatureSimEntity>, Without<crate::player::PlayerEntity>),
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
    let mut requests: Vec<(String, ae::Vec2, crate::combat::slots::SlotKind)> = Vec::new();
    for (_, _, actor, _, _, _, _, _, _, _, _, _, _, _, (clusters, _)) in &actors {
        if matches!(actor, ActorRuntime::Enemy) {
            if let Some(c) = clusters {
                if c.status.alive {
                    requests.push((c.config.id.clone(), c.kin.pos, c.config.tuning.slot_kind()));
                }
            }
        }
    }
    let slot_requests: Vec<crate::combat::slots::SlotRequest> = requests
        .iter()
        .map(|(id, pos, kind)| crate::combat::slots::SlotRequest {
            actor_id: id.as_str(),
            actor_pos: *pos,
            kind: *kind,
        })
        .collect();
    crate::combat::slots::assign_slots(&mut slot_board.0, player_pos, &slot_requests);

    // Per-kind holding-position fallback for actors that didn't win a
    // slot (see `compute_holding_positions`).
    let holding_pos_by_id = compute_holding_positions(&slot_board.0, &requests, player_pos);

    // Per-actor nearest-same-kind-neighbor index (see
    // `compute_nearest_neighbors`).
    let neighbor_by_id = compute_nearest_neighbors(&requests);

    // Per-actor crowding signal for brains that need personal space.
    let crowding_by_id = compute_crowding_by_id(&requests);

    // Pass 2: tick each actor with its assigned slot position. Falls
    // back to the slot's holding-ring position when this actor didn't
    // win a slot so it still has a sensible steering target.
    let combat_tuning = feel_tuning.feature_combat_tuning();
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
        (mut clusters, possessed),
    ) in &mut actors
    {
        // `target.pos` is populated by `select_actor_targets`
        // (#17.8); it defaults to the actor's spawn-of-game position
        // when no players exist yet (pre-spawn / post-death-of-all),
        // and is the primary player's pos in the single-player
        // production game.
        let target_pos = target.pos;
        match &mut *actor {
            // NPCs are ticked by the dedicated `update_ecs_npcs`
            // system — they don't participate in the enemy slot board
            // / crowding / body-contact passes, so they skip this loop.
            ActorRuntime::Npc => continue,
            ActorRuntime::Enemy => {
                // Enemy state is authoritative in the cluster
                // components; borrow them as an EnemyMut view for the
                // whole branch.
                let cq = clusters
                    .as_mut()
                    .expect("enemy entity carries cluster components");
                let mut em = cq.as_enemy_mut();
                let slot_pos = if let Some(slot) = slot_board.0.slot_for(&em.config.id) {
                    Some(slot.world_pos(target_pos))
                } else if em.status.alive {
                    // No slot assigned — fall back to the per-actor
                    // holding-ring position computed above. Multiple
                    // unassigned actors of the same kind are spread
                    // round-robin across all holding positions of
                    // that kind rather than sharing slot 0.
                    holding_pos_by_id.get(&em.config.id).copied()
                } else {
                    None
                };
                let nearest_neighbor = neighbor_by_id.get(&em.config.id).copied();
                // Capture pre-tick attack state so we can detect
                // the windup → active edge below. The enemy timer update
                // is the only path that performs the transition, so
                // observing the edge lets us spawn the strike's
                // `Hitbox` entity exactly once per begin-attack
                // instead of polling overlap every tick.
                let was_winding_up = em.attack.is_winding_up();
                let was_active = em.attack.is_active();

                // Every brain-attached enemy ticks its brain FIRST to
                // build an authoritative frame, then calls
                // `enemy.update` with that frame. The frame drives
                // the integration step (Patrol / Chase / Approach all
                // actually move the actor) AND lands in `ActorControl`
                // for the EFFECTS consumers (so melee + ranged fire).
                // Smash + Patrol + MeleeBrute + Skirmisher + Sniper +
                // Wanderer all flow through this single path.
                //
                // Actors without a brain (dynamically-spawned debug
                // entities) get a neutral frame and stand still —
                // production spawn paths always attach a brain, so
                // this is the safe no-op fallback.
                let _ = slot_pos;
                let is_mounted = mounted.is_some();
                let previous_pos = em.kin.pos;
                // Localized gravity: each enemy feels the gravity of the column
                // it is standing in (its own position), not one global field.
                let enemy_gravity_sign = gravity.sign_at(em.kin.pos);
                let brain_frame = if let Some(p) = possessed {
                    // POSSESSED: drive this actor from the player's input through
                    // its OWN ActorControlFrame — the same translation the player
                    // brain uses — so it moves/attacks via its own update path.
                    let crowding = crowding_by_id.get(&em.config.id).copied();
                    let snapshot = build_enemy_brain_snapshot(&em, target_pos, crowding, dt);
                    let mut bf = crate::actor::control::ActorControlFrame::neutral();
                    crate::brain::player::tick_player_brain_from_control(
                        &p.control, &snapshot, &mut bf,
                    );
                    // `desired_vel` is a direction (the player's input axis); the
                    // enemy integration approaches it directly, so scale it to a
                    // real speed or the possessed body crawls at ~1 px/s.
                    bf.desired_vel *= crate::abilities::traversal::possession::POSSESSED_MOVE_SPEED;
                    bf
                } else if let Some(brain_ref) = brain.as_deref_mut() {
                    let crowding = crowding_by_id.get(&em.config.id).copied();
                    let snapshot = build_enemy_brain_snapshot(&em, target_pos, crowding, dt);
                    let mut bf = crate::actor::control::ActorControlFrame::neutral();
                    let peaceful = crate::brain::ActionSet::peaceful();
                    let actions = action_set.unwrap_or(&peaceful);
                    brain_ref.tick_with_actions(actions, &snapshot, &mut bf);
                    bf
                } else {
                    crate::actor::control::ActorControlFrame::neutral()
                };
                let body_contact_damage_enabled = !brain.as_deref().is_some_and(crate::brain::Brain::is_player)
                        // A POSSESSED actor is on your side — its body never hurts
                        // you on contact (its melee + ranged already redirect at
                        // its former allies; contact just stops harming the player).
                        && possessed.is_none()
                        && em.config.tuning.body_contact_damage;
                let mut brain_frame = brain_frame;
                brain_frame.body_contact_damage_enabled = body_contact_damage_enabled;
                let shark_charge_vec = brain_frame.desired_vel;

                let frame = em.update(
                    &feature_world,
                    target_pos,
                    combat_tuning,
                    nearest_neighbor,
                    dt,
                    is_mounted,
                    brain_frame,
                    enemy_gravity_sign,
                );
                let shark_crashed =
                    shark_charge_crashed(&em, is_mounted, shark_charge_vec, previous_pos);
                let mut frame = frame;
                if shark_crashed {
                    hit_events.write(HitEvent {
                        volume: em.aabb(),
                        damage: em.status.health.current.max(1),
                        source: HitSource::EnemyChargeCrash,
                        attacker: None,
                        target: HitTarget::Volume,
                        mode: HitMode::Knockback,
                        knockback: None,
                        ignored_targets: Vec::new(),
                    });
                    frame = crate::actor::control::ActorControlFrame::neutral();
                }
                // Publish the actor's footprint ORIENTED to its reference frame —
                // the single source of truth read by the debug overlay, player
                // hurtbox, and target volumes, so the box matches the rotated
                // sprite. A surface-walker's frame is its clung surface
                // (`-surface_normal`); everyone else's is gravity at their position.
                // `to_world_half` swaps width<->height only under sideways gravity /
                // a wall — vertical gravity (down/up) is unchanged, so replay stays
                // byte-identical.
                let down = if em.config.tuning.surface_walker {
                    -em.surface.surface_normal
                } else {
                    gravity.dir_at(em.kin.pos)
                };
                let body_frame = crate::engine_core::AccelerationFrame::new(down);
                aabb.center = em.kin.pos;
                aabb.half_size = body_frame.to_world_half(em.kin.size * 0.5);

                if let Some(control) = control.as_deref_mut() {
                    control.0 = frame;
                }
                // Active-edge: windup just finished AND attack
                // timer is now positive (and wasn't already). Spawn
                // the strike's `Hitbox` entity here so the overlap
                // check moves to `apply_hitbox_damage` instead of
                // polling every frame from this system.
                if was_winding_up
                    && !em.attack.is_winding_up()
                    && em.attack.is_active()
                    && !was_active
                    && em.status.alive
                {
                    // Directional swing: read the axis the brain
                    // committed to in `begin_melee_attack`. Forward
                    // axis falls back to the actor's facing; up /
                    // down attacks place the hitbox above / below
                    // the body instead of in front.
                    let attack_box = em.attack_aabb_dir(em.attack.pending_axis);
                    let local_offset = attack_box.center() - em.kin.pos;
                    // A POSSESSED enemy swings for the player's side, so its
                    // hitbox damages its former allies (through the player-
                    // faction branch of `apply_hitbox_damage`) instead of you.
                    let hitbox_faction = if possessed.is_some() {
                        super::super::super::components::ActorFaction::Player
                    } else {
                        super::super::super::components::ActorFaction::Enemy
                    };
                    super::super::hitbox::spawn_melee_hitbox(
                        &mut commands,
                        actor_entity,
                        hitbox_faction,
                        local_offset,
                        attack_box.half_size(),
                        1,
                        1.0,
                        em.attack.active_timer,
                    );
                }
                // Mirror the cluster state onto the ECS read-model
                // components consumers still read (identity / disposition
                // / health / combat / intent / cooldowns). Replaces the
                // post-match sync_actor_components_from_runtime for the
                // enemy path.
                sync_actor_components_from_enemy(
                    &em,
                    &mut identity,
                    &mut disposition,
                    &mut health,
                    &mut combat,
                    &mut intent,
                    &mut cooldowns,
                );
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
                if target_vulnerable && em.status.alive && body_contact_damage_enabled {
                    if let Some(damage) = em.body_contact_damage(target_entity, target_body) {
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
        // Enemies mirrored from their clusters inside the branch above;
        // NPCs are handled entirely by `update_ecs_npcs`.
    }
}

/// Tick peaceful/hostile NPC actors. Split out from
/// [`update_ecs_actors`] because NPCs carry the actor-generic
/// [`BodyKinematics`] / [`ActorSurfaceState`] / [`ActorMotionPath`]
/// components that the enemy cluster query *also* borrows mutably — a
/// single query containing both `Option<EnemyClusterQueryData>` and
/// `Option<NpcClusterQueryData>` would panic on conflicting access to
/// those shared components. NPCs also skip the enemy-only slot board,
/// crowding, and body-contact machinery, so a dedicated system is both
/// necessary and simpler.
pub fn update_ecs_npcs(
    world_time: Res<WorldTime>,
    world: Res<crate::GameWorld>,
    gravity: crate::physics::GravityCtx,
    platform_set: Res<crate::MovingPlatformSet>,
    overlay: Res<FeatureEcsWorldOverlay>,
    mut npcs: Query<
        (
            &mut CenteredAabb,
            super::super::npc_clusters::NpcClusterQueryData,
            &super::super::super::components::ActorTarget,
            Option<&mut crate::brain::Brain>,
            Option<&mut crate::brain::ActorControl>,
            &mut ActorIdentity,
            &mut ActorDisposition,
            &mut ActorHealth,
            &mut ActorCombatState,
            &mut ActorIntent,
            &mut ActorCooldowns,
            // Possession: when present, the player drives this NPC's body
            // through its own control frame (the unification flex on a
            // peaceful actor) instead of its brain.
            Option<&crate::abilities::traversal::possession::Possessed>,
        ),
        With<FeatureSimEntity>,
    >,
) {
    let dt = world_time.sim_dt();
    let feature_world = world_with_sandbox_solids(&world.0, &platform_set.0, &overlay);
    // NPC brains run Patrol / StandStill, which don't read the absolute
    // sim clock; 0.0 is safe (mirrors `update_ecs_actors`).
    let sim_time = 0.0;
    for (
        mut aabb,
        mut clusters,
        target,
        mut brain,
        mut control,
        mut identity,
        mut disposition,
        mut health,
        mut combat,
        mut intent,
        mut cooldowns,
        possessed,
    ) in &mut npcs
    {
        let target_pos = target.pos;
        let mut npc = clusters.as_npc_mut();
        // Localized gravity: each NPC feels the gravity of the column it stands
        // in (its own position), so an NPC in a gravity room reorients on its own.
        let gravity_sign = gravity.sign_at(npc.kin.pos);
        let frame = if let Some(p) = possessed {
            // POSSESSED: drive the NPC body from the player's input through its
            // OWN ActorControlFrame — the unification flex ("drive any actor's
            // ActionSet via the human control path") on a peaceful actor.
            // `tick_player_brain_from_control` emits `desired_vel` as a direction
            // (`actor_facing` is the only snapshot field it reads); scale it to a
            // real walk speed. A peaceful NPC carries no attack verbs, so Attack
            // is a harmless no-op on it.
            let mut snapshot = crate::brain::BrainSnapshot::idle();
            snapshot.actor_facing = npc.kin.facing;
            let mut bf = crate::actor::control::ActorControlFrame::neutral();
            crate::brain::player::tick_player_brain_from_control(&p.control, &snapshot, &mut bf);
            if bf.facing.abs() > 0.001 {
                npc.kin.facing = bf.facing;
            }
            npc.integrate_velocity(
                bf.desired_vel.x * crate::abilities::traversal::possession::POSSESSED_MOVE_SPEED,
                &feature_world,
                dt,
                gravity_sign,
            );
            bf
        } else if let Some(brain) = brain.as_deref_mut() {
            npc.tick_via_brain(
                brain,
                &feature_world,
                target_pos,
                sim_time,
                dt,
                gravity_sign,
            )
        } else {
            // Brainless peaceful actor — should not happen post-Chunk 3
            // (spawn attaches a brain), but build one inline so the tick
            // is never skipped if components drift.
            let mut fallback = npc.build_brain();
            npc.tick_via_brain(
                &mut fallback,
                &feature_world,
                target_pos,
                sim_time,
                dt,
                gravity_sign,
            )
        };
        if let Some(control) = control.as_deref_mut() {
            control.0 = frame;
        }
        // Footprint oriented to gravity (the kernel guide, raiders, etc.), so the
        // debug box + hurtbox match the gravity-rotated sprite. Byte-identical for
        // vertical gravity; swaps width<->height only sideways.
        let npc_frame =
            crate::engine_core::AccelerationFrame::new(gravity.dir_at(npc.kin.pos));
        aabb.center = npc.kin.pos;
        aabb.half_size = npc_frame.to_world_half(npc.kin.size * 0.5);
        // Mirror the NPC clusters onto the read-model components.
        sync_actor_components_from_npc(
            &npc,
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
/// Per-actor nearest-same-kind-neighbor index (O(N²), N ≤ a few). Used
/// by brain snapshots as a "personal space" signal so two aerial actors
/// close to each other can push apart even when their slot anchors are
/// far apart. Returns the position of each actor's nearest same-kind
/// neighbor; actors with no same-kind peer are absent from the map.
pub(crate) fn compute_nearest_neighbors(
    requests: &[(String, ae::Vec2, crate::combat::slots::SlotKind)],
) -> std::collections::HashMap<String, ae::Vec2> {
    let mut neighbor_by_id: std::collections::HashMap<String, ae::Vec2> =
        std::collections::HashMap::new();
    for (id_a, pos_a, kind_a) in requests {
        let mut nearest: Option<(f32, ae::Vec2)> = None;
        for (id_b, pos_b, kind_b) in requests {
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
    neighbor_by_id
}

/// Per-kind holding-position fallback: actors that didn't win a combat
/// slot are distributed round-robin across the holding positions of all
/// slots of their kind, ordered stably by actor id so the assignment
/// doesn't flicker frame to frame. Without this, every unassigned actor
/// of a kind shared one slot's holding point and visually clumped. Pure
/// over the board + per-tick requests so it is unit-testable.
pub(crate) fn compute_holding_positions(
    board: &crate::combat::slots::CombatSlotBoard,
    requests: &[(String, ae::Vec2, crate::combat::slots::SlotKind)],
    player_pos: ae::Vec2,
) -> std::collections::HashMap<String, ae::Vec2> {
    let mut unassigned_by_kind: std::collections::HashMap<
        crate::combat::slots::SlotKind,
        Vec<&str>,
    > = std::collections::HashMap::new();
    for (id, _pos, kind) in requests {
        if board.slot_for(id).is_none() {
            unassigned_by_kind
                .entry(*kind)
                .or_default()
                .push(id.as_str());
        }
    }
    let mut holding_pos_by_id: std::collections::HashMap<String, ae::Vec2> =
        std::collections::HashMap::new();
    for (kind, mut ids) in unassigned_by_kind {
        let kind_slots: Vec<usize> = board
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
                board.slots[slot_idx].holding_pos(player_pos),
            );
        }
    }
    holding_pos_by_id
}

/// Per-actor crowding signal (personal-space pressure) consumed by
/// brains like Smash so clustered actors push apart. Aerial actors use a
/// wider radius and only count *other aerial* actors (so flyers like
/// sharks don't stack), while ground actors use a tighter radius. Pure
/// over the per-tick slot requests `(id, pos, kind)` so it is
/// unit-testable in isolation from the actor tick.
pub(crate) fn compute_crowding_by_id(
    requests: &[(String, ae::Vec2, crate::combat::slots::SlotKind)],
) -> std::collections::HashMap<String, crate::brain::CrowdingSignal> {
    const CROWDING_RADIUS_PX: f32 = 80.0;
    const AERIAL_CROWDING_RADIUS_PX: f32 = 220.0;
    let mut crowding_by_id: std::collections::HashMap<String, crate::brain::CrowdingSignal> =
        std::collections::HashMap::new();
    for (id_a, pos_a, kind_a) in requests {
        let mut count: u8 = 0;
        let mut centroid = ae::Vec2::ZERO;
        let aerial = *kind_a == crate::combat::slots::SlotKind::Aerial;
        let radius = if aerial {
            AERIAL_CROWDING_RADIUS_PX
        } else {
            CROWDING_RADIUS_PX
        };
        for (id_b, pos_b, kind_b) in requests {
            if id_a == id_b {
                continue;
            }
            if aerial && *kind_b != crate::combat::slots::SlotKind::Aerial {
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
    crowding_by_id
}

/// Build a `BrainSnapshot` for an enemy actor's per-tick brain call.
/// Carries the per-frame body / target / cooldown view every brain
/// backend reads from; `crowding` is only consulted by the Smash
/// brain, but always populating it keeps the snapshot uniform across
/// state-machine variants.
fn build_enemy_brain_snapshot(
    em: &super::super::enemy_clusters::EnemyMut<'_>,
    target_pos: ae::Vec2,
    crowding: Option<crate::brain::CrowdingSignal>,
    dt: f32,
) -> crate::brain::BrainSnapshot {
    crate::brain::BrainSnapshot {
        actor_pos: em.kin.pos,
        actor_vel: em.kin.vel,
        actor_facing: em.kin.facing,
        actor_on_ground: em.surface.on_ground,
        alive: em.status.alive,
        target_pos,
        target_alive: true,
        sim_time: 0.0,
        dt,
        attack_cooldown_remaining: em.attack.cooldown,
        attack_windup_remaining: em.attack.windup_timer,
        attack_active_remaining: em.attack.active_timer,
        attack_recover_remaining: 0.0,
        stun_remaining: 0.0,
        wall_contact: None,
        player_input: None,
        crowding,
        terrain: None,
        air_jumps_remaining: em.surface.air_jumps_remaining,
    }
}

/// Mirror the authoritative enemy clusters onto the ECS read-model
/// components consumers read.
pub fn sync_actor_components_from_enemy(
    em: &super::super::enemy_clusters::EnemyMut<'_>,
    identity: &mut ActorIdentity,
    disposition: &mut ActorDisposition,
    health: &mut ActorHealth,
    combat: &mut ActorCombatState,
    intent: &mut ActorIntent,
    cooldowns: &mut ActorCooldowns,
) {
    // Identity is stable after spawn — only rebuild it (which clones the id/name
    // strings AND wakes Bevy change-detection on `ActorIdentity`) when it actually
    // differs, not every tick. This mirror runs per actor per frame, so the
    // unconditional rebuild was pure clone + change-mark churn.
    if identity.id != em.config.id
        || identity.name != em.config.name
        || identity.sprite_override_npc_name != em.config.sprite_override_npc_name
    {
        *identity = ActorIdentity::new(em.config.id.clone(), em.config.name.clone())
            .with_sprite_override(em.config.sprite_override_npc_name.clone());
    }
    *disposition = ActorDisposition::Hostile;
    *health = ActorHealth::new(em.status.health);
    *combat = ActorCombatState::hostile(
        em.status.alive,
        em.status.hit_flash,
        em.attack.windup_timer,
        em.attack.active_timer,
        em.config.tuning.is_sandbag,
    );
    *intent = ActorIntent::new(em.status.ai_mode);
    *cooldowns = ActorCooldowns {
        attack_cooldown: em.attack.cooldown,
        respawn_timer: em.status.respawn_timer,
    };
}

/// Per-NPC ambient-bark timing (decremented by sim dt; deterministic jitter).
#[derive(Default)]
pub struct NpcIdleBarkState {
    timers: std::collections::HashMap<String, f32>,
    rotations: std::collections::HashMap<String, u32>,
}

/// Deterministic ~6–10s ambient-bark interval keyed by NPC id + counter — a
/// tiny FNV hash so we don't pull `rand` in for one cadence offset (mirrors the
/// boss idle-bark jitter).
fn npc_idle_bark_jitter(id: &str, counter: u32) -> f32 {
    let mut h: u32 = 2166136261;
    for b in id.bytes() {
        h = (h ^ b as u32).wrapping_mul(16777619);
    }
    h ^= counter.wrapping_mul(2654435761);
    6.0 + (h % 4000) as f32 / 1000.0
}

/// Ambient NPC chatter: a peaceful NPC carrying an idle-bark pool
/// ([`crate::features::npcs::npc_idle_bark_line`]) mutters a line every
/// ~6–10s, so it feels alive between conversations. Skips hostile NPCs and any
/// still showing a hit-flash bubble (so it never talks over a hit bark). The
/// stochastic parrot is the first user; any NPC gains barks by adding a pool.
pub fn tick_npc_idle_barks(
    world_time: Res<WorldTime>,
    npcs: Query<
        (
            &super::super::enemy_clusters::BodyKinematics,
            &super::super::npc_clusters::NpcConfig,
            &super::super::npc_clusters::NpcStatus,
        ),
        With<FeatureSimEntity>,
    >,
    mut vfx: MessageWriter<ambition_effects::vfx::VfxMessage>,
    mut state: Local<NpcIdleBarkState>,
) {
    let dt = world_time.scaled_dt;
    if dt <= 0.0 {
        return;
    }
    for (kin, config, status) in &npcs {
        if status.hostile || status.hit_flash > 0.0 {
            continue;
        }
        let rotation = *state.rotations.get(&config.id).unwrap_or(&0);
        let Some(line) = super::super::npcs::npc_idle_bark_line(config, rotation) else {
            continue;
        };
        let timer = state
            .timers
            .entry(config.id.clone())
            .or_insert_with(|| npc_idle_bark_jitter(&config.id, 0));
        *timer -= dt;
        if *timer > 0.0 {
            continue;
        }
        let anchor = kin.pos + ae::Vec2::new(0.0, -kin.size.y * 0.72 - 16.0);
        vfx.write(ambition_effects::vfx::VfxMessage::SpeechBubble {
            pos: anchor,
            text: line.to_string(),
        });
        let next = rotation.wrapping_add(1);
        state.rotations.insert(config.id.clone(), next);
        state
            .timers
            .insert(config.id.clone(), npc_idle_bark_jitter(&config.id, next));
    }
}
