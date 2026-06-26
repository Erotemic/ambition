//! The per-frame actor tick: syncing poses from feature AABBs, driving the
//! enemy + NPC updates, neighbor/crowding queries, and brain snapshots.

use super::super::*;
use super::*;

/// Map an enemy's committed melee axis to the player-style directional attack
/// animation row, so an enemy whose sheet authors an attack hitbox is read the
/// same data-driven way the player is. Mirrors `enemy_attack_aabb_dir`'s axis
/// branching (forward / up / down).
fn enemy_melee_animation_for_axis(axis: ambition_engine_core::Vec2) -> &'static str {
    if axis.x.abs() >= axis.y.abs() {
        "attack_side"
    } else if axis.y < 0.0 {
        "attack_up"
    } else {
        "attack_down"
    }
}

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
            Option<&super::super::actor_clusters::BodyKinematics>,
            Option<super::super::boss_clusters::BossClusterRef>,
        ),
        With<FeatureSimEntity>,
    >,
) {
    for (aabb, mut pose, kin, boss) in &mut actors {
        // Facing source: the unified actor cluster (BodyKinematics) for every
        // actor, or the boss runtime; default to the current pose facing.
        let facing = kin
            .map(|k| k.facing)
            .or_else(|| boss.map(|feature| feature.kin.facing))
            .unwrap_or(pose.facing);
        *pose = super::super::super::components::ActorPose::from_parts(
            aabb.center,
            aabb.half_size,
            facing,
        );
    }
}

/// Tick ECS actors. Peaceful and hostile actors share the same entity identity
/// and can switch disposition in-place; dynamic encounter-spawned mobs use the
/// same hostile path with an `EncounterMob` marker.
pub fn update_ecs_actors(
    mut commands: Commands,
    world_time: Res<WorldTime>,
    world: Res<crate::RoomGeometry>,
    gravity: crate::physics::GravityCtx,
    user_settings: Option<Res<crate::persistence::settings::UserSettings>>,
    platform_set: Res<crate::MovingPlatformSet>,
    feel_tuning: Res<crate::time::feel::SandboxFeelTuning>,
    overlay: Res<FeatureEcsWorldOverlay>,
    mut slot_board: ResMut<crate::combat::slots::CombatSlotsRes>,
    mut sfx: MessageWriter<crate::audio::SfxMessage>,
    mut vfx: MessageWriter<ambition_vfx::vfx::VfxMessage>,
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
            &mut ActorIdentity,
            &ActorDisposition,
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
            Option<&mut ambition_characters::brain::Brain>,
            Option<&mut ambition_characters::brain::ActorControl>,
            // ActionSet — read for the Smash brain so it knows which
            // attacks (melee / ranged) the actor can commit. `Option`
            // so dynamically-spawned actors without a set still tick.
            Option<&ambition_characters::brain::ActionSet>,
            Option<&super::super::Mounted>,
            // The unified actor cluster — every actor (was-NPC + was-enemy)
            // carries it. The tick integrates through it via `ActorMut`.
            //
            // `Possessed` is nested with the cluster data (not a new top-level
            // tuple field) to stay within Bevy's query-tuple arity: when set,
            // the actor is driven from the player's input instead of its brain
            // (`crate::abilities::traversal::possession`).
            (
                Option<super::super::actor_clusters::ActorClusterQueryData>,
                Option<&crate::abilities::traversal::possession::Possessed>,
            ),
        ),
        // The player carries the unified `BodyKinematics` too, and
        // `player_query` above reads it; exclude the player here so this
        // `&mut BodyKinematics` actor query is provably disjoint from it
        // (player / actor archetypes never overlap).
        //
        // Exclude BOSSES too: they carry the shared actor read-models
        // (`ActorIdentity`/`ActorDisposition`/… synced by
        // `sync_boss_actor_components`) but have NO actor cluster, so without
        // this they'd match here (cluster = `None`) and get ticked by the actor
        // loop ON TOP of their own `tick_boss_brains_system` — a double brain
        // tick. The deleted `ActorRuntime` tag used to keep them out implicitly.
        (
            With<FeatureSimEntity>,
            Without<crate::player::PlayerEntity>,
            Without<super::super::boss_clusters::BossConfig>,
        ),
    >,
) {
    // Sim clock: enemies, NPCs, encounter mobs all advance on the
    // gameplay clock so bullet-time / pause / hitstop freeze them
    // alongside the player. ADR 0010 + reference_lessons_learned.
    let dt = world_time.sim_dt();
    let feature_world = world_with_sandbox_solids(&world.0, &platform_set.0, &overlay);
    let control_frame_modes = user_settings
        .as_deref()
        .map_or(ae::ControlFrameModes::default(), |s| {
            s.gameplay.control_frame_modes()
        });
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
    for (_, _, _, disposition, _, _, _, _, _, _, _, _, _, (clusters, _)) in &actors {
        // Only hostile actors compete for combat slots; peaceful actors don't
        // crowd the board ("enemy" == hostile disposition now).
        if disposition.is_hostile() {
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
        mut identity,
        disposition,
        mut health,
        mut combat,
        mut intent,
        mut cooldowns,
        target,
        mut brain,
        mut control,
        action_set,
        mounted,
        (clusters, possessed),
    ) in &mut actors
    {
        // `target.pos` is populated by `select_actor_targets`
        // (#17.8); it defaults to the actor's spawn-of-game position
        // when no players exist yet (pre-spawn / post-death-of-all),
        // and is the primary player's pos in the single-player
        // production game.
        let target_pos = target.pos;
        {
            // Every actor (was-NPC + was-enemy) shares the unified cluster.
            // Peaceful actors no-op the slot-board / body-contact / hostile
            // passes via tuning (`attacks_player` / `body_contact_damage`); the
            // brain drives patrol/idle. Borrow the cluster as an ActorMut view.
            let Some(mut cq) = clusters else {
                continue;
            };
            {
                let mut em = cq.as_actor_mut();
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
                let enemy_gravity_dir = gravity.dir_at(em.kin.pos);
                let brain_frame = if let Some(p) = possessed {
                    // POSSESSED: drive this actor from the player's input through
                    // its OWN ActorControlFrame — the same translation the player
                    // brain uses — so it moves/attacks via its own update path.
                    let crowding = crowding_by_id.get(&em.config.id).copied();
                    let mut snapshot = build_enemy_brain_snapshot(
                        &em,
                        target_pos,
                        crowding,
                        dt,
                        enemy_gravity_dir,
                    );
                    snapshot.control_down = enemy_gravity_dir;
                    snapshot.movement_frame_mode = control_frame_modes.movement;
                    snapshot.aim_frame_mode = control_frame_modes.aim;
                    let mut bf = ambition_characters::actor::control::ActorControlFrame::neutral();
                    ambition_characters::brain::player::tick_player_brain_from_control(
                        &p.control, &snapshot, &mut bf,
                    );
                    // The player brain emits normalized `locomotion`. A possessed
                    // body should move at POSSESSED_MOVE_SPEED regardless of its
                    // native capability: encode that as intent for the grounded
                    // path (a throttle of the body's own max), and as a direct
                    // world velocity for the aerial / free-mover path.
                    let possess_speed =
                        crate::abilities::traversal::possession::POSSESSED_MOVE_SPEED;
                    bf.velocity_target = bf.locomotion * possess_speed;
                    bf.locomotion *= possess_speed / em.config.tuning.max_run_speed.max(1.0);
                    bf
                } else if let Some(brain_ref) = brain.as_deref_mut() {
                    let crowding = crowding_by_id.get(&em.config.id).copied();
                    let snapshot = build_enemy_brain_snapshot(
                        &em,
                        target_pos,
                        crowding,
                        dt,
                        enemy_gravity_dir,
                    );
                    let mut bf = ambition_characters::actor::control::ActorControlFrame::neutral();
                    let peaceful = ambition_characters::brain::ActionSet::peaceful();
                    let actions = action_set.unwrap_or(&peaceful);
                    brain_ref.tick_with_actions(actions, &snapshot, &mut bf);
                    bf
                } else {
                    ambition_characters::actor::control::ActorControlFrame::neutral()
                };
                let body_contact_damage_enabled = !brain.as_deref().is_some_and(ambition_characters::brain::Brain::is_player)
                        // A POSSESSED actor is on your side — its body never hurts
                        // you on contact (its melee + ranged already redirect at
                        // its former allies; contact just stops harming the player).
                        && possessed.is_none()
                        && em.config.tuning.body_contact_damage;
                let mut brain_frame = brain_frame;
                brain_frame.body_contact_damage_enabled = body_contact_damage_enabled;
                let shark_charge_vec = brain_frame.velocity_target;

                let frame = em.update(
                    &feature_world,
                    target_pos,
                    combat_tuning,
                    nearest_neighbor,
                    dt,
                    is_mounted,
                    brain_frame,
                    enemy_gravity_dir,
                );
                let shark_crashed =
                    shark_charge_crashed(&em, is_mounted, shark_charge_vec, previous_pos);
                let mut frame = frame;
                if shark_crashed {
                    hit_events.write(HitEvent {
                        volume: em.aabb().into(),
                        damage: em.status.health.current.max(1),
                        source: HitSource::EnemyChargeCrash,
                        attacker: Some(actor_entity),
                        target: HitTarget::Volume,
                        mode: HitMode::Knockback,
                        knockback: None,
                        ignored_targets: Vec::new(),
                    });
                    frame = ambition_characters::actor::control::ActorControlFrame::neutral();
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
                // One shared computation of an actor's frame-oriented body box
                // (the same `collision_aabb` the damage path and tests use).
                let body = crate::features::collision_aabb(&crate::features::SimpleActorGeometry {
                    pos: em.kin.pos,
                    size: em.kin.size,
                    facing: em.kin.facing,
                    frame_down: down,
                });
                aabb.center = body.center();
                aabb.half_size = body.half_size();

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
                    // Prefer the actor's authored sprite-metadata attack hitbox
                    // (the same data-driven path the player and bosses use),
                    // falling back to the shared hardcoded melee volume when the
                    // sheet authors none. Gated to upright gravity: the manifest
                    // box is screen-axis, while the fallback rotates with a
                    // wall-clinger's frame, so a clung enemy keeps its oriented box.
                    let upright = down.x.abs() < 0.01 && down.y > 0.0;
                    let attack_box = em
                        .config
                        .sprite_character_id
                        .as_deref()
                        .filter(|_| upright)
                        .and_then(|cid| {
                            crate::character_sprites::actor_attack_hitbox_world(
                                cid,
                                enemy_melee_animation_for_axis(em.attack.pending_axis),
                                em.kin.pos,
                                em.kin.size,
                                em.kin.facing,
                            )
                            // Enemy melee spawns a box hitbox today; collapse a
                            // shaped manifest volume to its bounds (shaped enemy
                            // melee is a later step).
                            .map(|v| v.bounds())
                        })
                        .unwrap_or_else(|| em.attack_aabb_dir(em.attack.pending_axis));
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
                // components consumers still read (identity / health /
                // combat / intent / cooldowns). Disposition is owned by
                // spawn/provoke, so it is read (peaceful vs hostile combat
                // state) but not written here.
                sync_actor_components_from_cluster(
                    &em,
                    *disposition,
                    &mut identity,
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
                    if let Some(damage) = em.body_contact_damage(actor_entity, target_entity, target_body) {
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
        // Read-models mirrored from the unified cluster inside the block above.
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
) -> std::collections::HashMap<String, ambition_characters::brain::CrowdingSignal> {
    const CROWDING_RADIUS_PX: f32 = 80.0;
    const AERIAL_CROWDING_RADIUS_PX: f32 = 220.0;
    let mut crowding_by_id: std::collections::HashMap<String, ambition_characters::brain::CrowdingSignal> =
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
                ambition_characters::brain::CrowdingSignal {
                    same_faction_count: count,
                    other_faction_count: 0,
                    away_dir: away,
                    pressure: ambition_characters::brain::CrowdingSignal::compute_pressure(count, 0),
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
    em: &super::super::actor_clusters::ActorMut<'_>,
    target_pos: ae::Vec2,
    crowding: Option<ambition_characters::brain::CrowdingSignal>,
    dt: f32,
    gravity_dir: ae::Vec2,
) -> ambition_characters::brain::BrainSnapshot {
    ambition_characters::brain::BrainSnapshot {
        actor_pos: em.kin.pos,
        actor_vel: em.kin.vel,
        actor_facing: em.kin.facing,
        control_down: gravity_dir,
        movement_frame_mode: ae::InputFrameMode::DEFAULT_MOVEMENT,
        aim_frame_mode: ae::InputFrameMode::DEFAULT_AIM,
        actor_on_ground: em.surface.on_ground,
        alive: em.status.alive,
        target_pos,
        target_alive: true,
        sim_time: 0.0,
        dt,
        max_run_speed: em.config.tuning.max_run_speed,
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

/// Mirror the authoritative actor cluster onto the ECS read-model components
/// consumers read. Disposition is OWNED by spawn/provoke (not derived from the
/// cluster), so it is read here (to pick peaceful vs hostile `ActorCombatState`)
/// but never written.
pub fn sync_actor_components_from_cluster(
    em: &super::super::actor_clusters::ActorMut<'_>,
    disposition: ActorDisposition,
    identity: &mut ActorIdentity,
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
    *health = ActorHealth::new(em.status.health);
    *combat = if disposition.is_hostile() {
        ActorCombatState::hostile(
            em.status.alive,
            em.status.hit_flash,
            em.attack.windup_timer,
            em.attack.active_timer,
            em.config.tuning.is_sandbag,
        )
    } else {
        ActorCombatState::peaceful(0, em.status.hit_flash)
    };
    *intent = ActorIntent::new(em.status.ai_mode);
    *cooldowns = ActorCooldowns {
        attack_cooldown: em.attack.cooldown,
        respawn_timer: em.status.respawn_timer,
    };
}

/// Area id of the Hall of Characters (matches the generated level/area in
/// `generate_hall_of_characters.py`). When this is the active area, the idle
/// ticker switches NPC ambient barks to their `Hall` pool.
const HALL_OF_CHARACTERS_AREA: &str = "hall_of_characters";

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
            &super::super::actor_clusters::BodyKinematics,
            &super::super::actor_clusters::ActorConfig,
            &super::super::actor_clusters::ActorStatus,
            &ActorInteraction,
            &ActorDisposition,
        ),
        With<FeatureSimEntity>,
    >,
    mut vfx: MessageWriter<ambition_vfx::vfx::VfxMessage>,
    room_set: Option<Res<crate::rooms::RoomSet>>,
    mut state: Local<NpcIdleBarkState>,
) {
    let dt = world_time.scaled_dt;
    if dt <= 0.0 {
        return;
    }
    // While the player is touring the Hall of Characters, pedestals draw their
    // `Hall` bark pool (the fun gallery lines); everywhere else NPCs mutter
    // their `Idle` pool. Same ambient ticker, different occasion.
    let situation = match room_set.as_deref() {
        Some(rs) if rs.active_spec().id == HALL_OF_CHARACTERS_AREA => {
            ambition_characters::actor::character_catalog::BarkSituation::Hall
        }
        _ => ambition_characters::actor::character_catalog::BarkSituation::Idle,
    };
    for (kin, config, status, interaction, disposition) in &npcs {
        if disposition.is_hostile() || status.hit_flash > 0.0 {
            continue;
        }
        let rotation = *state.rotations.get(&config.id).unwrap_or(&0);
        let Some(line) = super::super::npcs::npc_ambient_bark_line(
            &interaction.interactable,
            &config.id,
            situation,
            rotation,
        ) else {
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
        vfx.write(ambition_vfx::vfx::VfxMessage::SpeechBubble {
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
