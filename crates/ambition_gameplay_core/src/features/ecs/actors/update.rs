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
    // Bundled into one SystemParam slot: this system is at Bevy's 16-param
    // ceiling, so the accumulating sim clock + the slot-based controller input
    // ride alongside `WorldTime`. `SlotControls` feeds any actor carrying a
    // `Brain::Player(slot)` (a possessed body) its controller frame.
    (world_time, sim_clock, slot_controls): (
        Res<WorldTime>,
        Res<crate::features::GameplayElapsed>,
        Res<ambition_characters::brain::SlotControls>,
    ),
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
            &crate::actor::BodyKinematics,
            &crate::actor::BodyOffense,
            &crate::actor::BodyDodgeState,
            &crate::actor::BodyShieldState,
            &crate::actor::BodyCombat,
            // The player's liveness AUTHORITY is its health (the single BodyHealth
            // source), NOT `BodyCombat.alive` — that field is an actor-cluster
            // mirror that is never synced for the player, so it reads `false` and
            // made every enemy treat a live player as dead (→ the Smash brain idled
            // instead of engaging). Read health here, consistent with how
            // `select_actor_targets` decides the player is a valid target.
            &crate::actor::BodyHealth,
        ),
        bevy::prelude::With<crate::actor::PlayerEntity>,
    >,
    primary_q: bevy::prelude::Query<
        bevy::prelude::Entity,
        (
            bevy::prelude::With<crate::actor::PlayerEntity>,
            bevy::prelude::With<crate::actor::PrimaryPlayer>,
        ),
    >,
    mut actors: Query<
        (
            Entity,
            &mut CenteredAabb,
            &mut ActorIdentity,
            // Mutable: a hostile fighter whose foe has died is pacified back to
            // Peaceful here, so it resumes normal NPC behavior (and can be talked
            // to) instead of menacing a corpse.
            &mut ActorDisposition,
            &mut BodyCombat,
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
            // A possessed body needs no special query field: it simply carries
            // `Brain::Player(slot)` (transferred by
            // `crate::abilities::traversal::possession`) and is driven through
            // the SAME brain tick every actor uses, reading its slot's frame
            // from `SlotControls`.
            (
                Option<super::super::actor_clusters::ActorClusterQueryData>,
                // Faction — read to scope the anti-clump crowding signal to
                // SAME-faction allies. Without this, two hostiles of different
                // factions (the spectator-duel fighters) count each other as
                // crowding neighbors and the anti-clump back-actor rule freezes
                // both. `Option` to match the other cluster-nested reads.
                Option<&super::super::super::components::ActorFaction>,
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
            Without<crate::actor::PlayerEntity>,
            Without<super::super::boss_clusters::BossConfig>,
        ),
    >,
) {
    // Sim clock: enemies, NPCs, encounter mobs all advance on the
    // gameplay clock so bullet-time / pause / hitstop freeze them
    // alongside the player. ADR 0010 + reference_lessons_learned.
    let dt = world_time.sim_dt();
    // Accumulating sim-time for brain perception (reaction-latency lookback).
    let sim_now = sim_clock.0;
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
        .map(|(_, kin, _, _, _, _, _)| kin.pos);
    let Some(player_pos) = slot_anchor_pos else {
        return;
    };

    // Pass 1: collect slot requests from every live hostile enemy.
    // The slot board is per-target (player) and arbitrates which
    // enemies are allowed to commit to an attack this tick; the
    // others hold at the outer ring. This is the anti-clump layer.
    let mut requests: Vec<(String, ae::Vec2, crate::combat::slots::SlotKind)> = Vec::new();
    // Faction per actor id, so the anti-clump crowding signal counts only
    // same-faction allies (fanning a swarm out) and never a different-faction
    // opponent — the spectator-duel fighters are hostile to each other and must
    // close, not anti-clump apart.
    let mut faction_by_id: std::collections::HashMap<
        String,
        super::super::super::components::ActorFaction,
    > = std::collections::HashMap::new();
    // Liveness of every potential TARGET (actors + players), keyed by entity, so a
    // fighter can perceive that its foe has died. A fighter's target is often
    // another actor (the spectator-duel pair target each other), so this can't come
    // from `player_query` alone. Defaults to alive when an entity isn't found.
    let mut alive_by_entity: std::collections::HashMap<Entity, bool> =
        std::collections::HashMap::new();
    // Entity → actor id, so a fighter's CURRENT TARGET entity (its foe this frame —
    // a faction-opponent OR a personal grudge) can be resolved to an id for the
    // anti-clump rule: a body you're actively fighting is an opponent to close on,
    // not an ally to spread away from. This is what lets two SAME-faction `Npc`
    // duelists (feuding via a grudge) close instead of mutually anti-clumping apart.
    let mut entity_to_id: std::collections::HashMap<Entity, String> =
        std::collections::HashMap::new();
    let mut target_entity_by_id: std::collections::HashMap<String, Entity> =
        std::collections::HashMap::new();
    for (entity, _, _, _, _, _, health) in &player_query {
        alive_by_entity.insert(entity, health.current() > 0);
    }
    for (entity, _, _, disposition, _, _, _, target, _, _, _, _, (clusters, faction)) in &actors {
        if let Some(c) = &clusters {
            alive_by_entity.insert(entity, c.health.alive());
            entity_to_id.insert(entity, c.config.id.clone());
        }
        // Only hostile actors compete for combat slots; peaceful actors don't
        // crowd the board ("enemy" == hostile disposition now).
        if disposition.is_hostile() {
            if let Some(c) = clusters {
                if c.health.alive() {
                    requests.push((c.config.id.clone(), c.kin.pos, c.config.tuning.slot_kind()));
                    if let Some(faction) = faction {
                        faction_by_id.insert(c.config.id.clone(), *faction);
                    }
                    if let Some(foe) = target.entity {
                        target_entity_by_id.insert(c.config.id.clone(), foe);
                    }
                }
            }
        }
    }
    // Resolve each fighter's target ENTITY to the target's id (dropping targets that
    // aren't crowd actors — e.g. the player). The anti-clump builder reads this to
    // treat the body you're fighting as an opponent, never a neighbor to flee.
    let opponent_id_by_id: std::collections::HashMap<String, String> = target_entity_by_id
        .iter()
        .filter_map(|(id, foe)| entity_to_id.get(foe).map(|fid| (id.clone(), fid.clone())))
        .collect();
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
    let crowding_by_id = compute_crowding_by_id(&requests, &faction_by_id, &opponent_id_by_id);

    // Pass 2: tick each actor with its assigned slot position. Falls
    // back to the slot's holding-ring position when this actor didn't
    // win a slot so it still has a sensible steering target.
    let combat_tuning = feel_tuning.feature_combat_tuning();
    for (
        actor_entity,
        mut aabb,
        mut identity,
        mut disposition,
        mut combat,
        mut intent,
        mut cooldowns,
        target,
        mut brain,
        mut control,
        action_set,
        mounted,
        (clusters, faction),
    ) in &mut actors
    {
        // Body-generic reaction timers on the body's authoritative `BodyCombat`
        // (the same fields the player carries): the post-hit i-frame the actor
        // gates re-hits on, and the damage-blink the renderer reads. Decremented
        // for every actor each tick, alive or dead.
        combat.damage_invuln_timer = (combat.damage_invuln_timer - dt).max(0.0);
        combat.hit_flash = (combat.hit_flash - dt).max(0.0);

        // This actor's combat-target liveness. `select_actor_targets` already
        // dropped a dead/absent foe (it only ever targets a LIVE candidate, and a
        // faction-feud fighter has no target once its foe is gone), so `entity ==
        // None` here means "no one to fight" → the brain idles (peaceful behavior).
        let target_alive = match target.entity {
            Some(e) => alive_by_entity.get(&e).copied().unwrap_or(true),
            None => false,
        };
        // Disposition is DERIVED from having a combat target: an aggressive actor
        // with NO target stands down to Peaceful — it stops attacking empty air,
        // relabels as peaceful, and is re-provokable (strike it past the threshold)
        // again — but KEEPS its aggression mode, so it re-acquires and re-engages the
        // instant a foe reappears (retreat → escape → peaceful; reacquire →
        // fighting). A HostileToPlayer enemy keeps the live player as its target, so
        // it never spuriously stands down. Relativity-neutral (any fighter, any
        // faction). This REPLACES the former hard pacify-to-passive, which dead-ended
        // a duel winner (couldn't be talked to or re-provoked, and mislabeled it).
        if disposition.is_hostile() && target.entity.is_none() {
            *disposition = ActorDisposition::Peaceful;
        }
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
                } else if em.health.alive() {
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
                let brain_frame = if let Some(brain_ref) = brain.as_deref_mut() {
                    let crowding = crowding_by_id.get(&em.config.id).copied();
                    let mut snapshot = build_enemy_brain_snapshot(
                        &em,
                        target_pos,
                        target_alive,
                        crowding,
                        dt,
                        sim_now,
                        enemy_gravity_dir,
                    );
                    // POSSESSION IS BRAIN TRANSFER: a body carrying
                    // `Brain::Player(slot)` (transferred by possession) reads its
                    // slot's controller frame from `SlotControls` through the SAME
                    // brain tick every actor uses — no special-case branch, no
                    // input-copy component. The player brain translates that frame
                    // in the body's own gravity + control frames, then the body
                    // moves/attacks/fires via its own ActionSet + update path
                    // exactly like any brain-driven actor. AI brains leave
                    // `player_input` `None` and ignore the control-frame modes.
                    if let Some(slot) = brain_ref.player_slot() {
                        snapshot.player_input = Some(slot_controls.get(slot));
                        snapshot.control_down = enemy_gravity_dir;
                        snapshot.movement_frame_mode = control_frame_modes.movement;
                        snapshot.aim_frame_mode = control_frame_modes.aim;
                    }
                    // Headless world-out view for this body (S4/S5): built over the
                    // SAME derived collision world `feature_world` the body integrates
                    // against, so the brain's line-of-fire gate is answered over real
                    // geometry (never a parallel sensor). Body-generic (guardrail #1):
                    // this is the same `build_world_view` the player-robot body will
                    // use. Peers / projectiles are wired in when the strong brain
                    // consumes them (S5); the terrain-only view today drives the LOF
                    // gate, so the body faction is immaterial here.
                    let world_view = super::super::perception::build_world_view(
                        &super::super::perception::PerceptionBody {
                            pos: em.kin.pos,
                            vel: em.kin.vel,
                            facing: em.kin.facing,
                            half_extent: em.kin.size,
                            faction: ambition_characters::actor::ActorFaction::Enemy,
                            gravity_down: enemy_gravity_dir,
                            on_ground: em.ground.on_ground,
                            aerial: em.surface.gravity_scale <= 0.001,
                            alive: em.health.alive(),
                            can_fire: true,
                            can_blink: em.caps.can_blink,
                            can_dash: em.caps.can_dash,
                            can_shield: em.caps.can_shield,
                        },
                        &[],
                        &[],
                        &[],
                        &feature_world,
                        &crate::combat::targeting::FactionRelations::default(),
                        super::super::perception::DEFAULT_VIEWPORT_HALF,
                        sim_now,
                    );
                    let mut bf = ambition_characters::actor::control::ActorControlFrame::neutral();
                    let peaceful = ambition_characters::brain::ActionSet::peaceful();
                    let actions = action_set.unwrap_or(&peaceful);
                    brain_ref.tick_with_actions(actions, &snapshot, Some(&world_view), &mut bf);
                    bf
                } else {
                    ambition_characters::actor::control::ActorControlFrame::neutral()
                };
                // Body-contact hazard is off for any player-controlled body: a
                // possessed actor carries `Brain::Player`, is on your side (its
                // melee + ranged redirect at its former allies), and its body
                // must not harm you on contact. Derived from the brain, so the
                // possession special-case (`possessed.is_none()`) is gone.
                let body_contact_damage_enabled = !brain
                    .as_deref()
                    .is_some_and(ambition_characters::brain::Brain::is_player)
                    && em.config.tuning.body_contact_damage;
                let mut brain_frame = brain_frame;
                brain_frame.body_contact_damage_enabled = body_contact_damage_enabled;
                let shark_charge_vec = brain_frame.velocity_target;

                // Respawn blink: `em.update` revives a dead body in place (HP reset)
                // when its respawn timer elapses. The damage-blink lives on the
                // body's `BodyCombat` now, so apply the revive flash here in the
                // driver (where it's in scope) on the dead→alive transition.
                let was_dead = !em.health.alive();
                let (frame, move_events) = em.update(
                    &feature_world,
                    target_pos,
                    combat_tuning,
                    nearest_neighbor,
                    dt,
                    is_mounted,
                    brain_frame,
                    enemy_gravity_dir,
                );
                if was_dead && em.health.alive() {
                    combat.hit_flash = 0.24;
                }
                let shark_crashed =
                    shark_charge_crashed(&em, is_mounted, shark_charge_vec, previous_pos);
                let mut frame = frame;
                if shark_crashed {
                    hit_events.write(HitEvent {
                        volume: em.aabb().into(),
                        damage: em.health.current().max(1),
                        source: HitSource::EnemyChargeCrash,
                        attacker: Some(actor_entity),
                        target: HitTarget::Volume,
                        mode: HitMode::Knockback,
                        knockback: None,
                        ignored_targets: Vec::new(),
                    });
                    frame = ambition_characters::actor::control::ActorControlFrame::neutral();
                }
                // Blink is folded onto the shared pipeline limb: `em.update` ran the
                // body's blink limb (ability-gated by the mask, collision-clamped by
                // the SAME path the player uses) and TELEPORTED the body. The driver
                // only reacts to the resulting `FrameEvents.blinks` with sfx/vfx —
                // and it emits the SAME feedback the player does: the clean
                // `BlinkEffects` flash (the quick / precision blink look), NOT the
                // `ClassicBurst` explosion that belongs to the held *item* blink. An
                // AI fighter blinking should read identically to the player blinking.
                for blink in &move_events.blinks {
                    sfx.write(crate::audio::SfxMessage::Play {
                        id: ambition_sfx::ids::PLAYER_BLINK,
                        pos: blink.to,
                    });
                    vfx.write(ambition_vfx::vfx::VfxMessage::BlinkEffects {
                        from: blink.from,
                        to: blink.to,
                        precision: blink.precision,
                    });
                }
                // Fly-toggle is resolved INSIDE the shared pipeline (invariant I3):
                // `em.update`'s control phase ran `apply_fly_toggle`, which flips
                // `flight.fly_enabled` from the brain's `fly_toggle_pressed` (gated by
                // the ability mask) exactly like the player. A manual toggle here used
                // to run too — a SECOND flip on the same intent that cancelled the
                // pipeline's, so a hybrid could never actually take off. Removed; the
                // pipeline is the one owner.
                // Shield is folded onto the shared pipeline limb: `em.update` ran
                // the body's `apply_shield` (the SAME `resolve_shield` rule the
                // player uses, ability-gated by the mask, dash-blocked by the
                // pipeline dash), resolving onto the body's ONE `BodyShieldState`.
                // The damage path reads `shield.active` off it — nothing to resolve here.
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
                    && em.health.alive()
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
                    // The swing's world-frame `AttackSpec` (the SAME spec the human
                    // player uses, rotated to world at `begin_melee_attack`). Its
                    // box is the fallback when the sprite sheet authors no per-anim
                    // hitbox, replacing the old bespoke `attack_aabb_dir`, so reach
                    // + placement match the player.
                    let world_spec = em.attack.swing.as_ref().map(|s| s.spec);
                    let spec_box = world_spec
                        .map(|s| ae::Aabb::new(em.kin.pos + s.hitbox_offset, s.hitbox_half_size));
                    // The authored sprite-manifest box is now gravity-aware (it
                    // rotates into the actor's frame), so use it under ANY gravity —
                    // no upright gate. Falls back to the spec box when the sheet
                    // authors no per-anim hitbox.
                    let attack_box = em
                        .config
                        .sprite_character_id
                        .as_deref()
                        .and_then(|cid| {
                            crate::character_sprites::actor_attack_hitbox_world(
                                cid,
                                enemy_melee_animation_for_axis(em.attack.pending_axis),
                                em.kin.pos,
                                em.kin.size,
                                em.kin.facing,
                                down,
                            )
                            // Enemy melee spawns a box hitbox today; collapse a
                            // shaped manifest volume to its bounds (shaped enemy
                            // melee is a later step).
                            .map(|v| v.bounds())
                        })
                        .or(spec_box)
                        .unwrap_or_else(|| em.attack_aabb_dir(em.attack.pending_axis));
                    // The strike carries the attacker's EFFECTIVE allegiance so the
                    // physical-damage rule (`can_damage`) resolves correctly: a
                    // Boss-faction duel robot's swing hits the Enemy-faction PCA, and
                    // a POSSESSED body (carrying `Brain::Player`) swings as `Player`
                    // and damages its former allies — WITHOUT its authored
                    // `ActorFaction` ever being mutated (no flip, no restore).
                    let hitbox_faction = crate::combat::targeting::effective_faction(
                        faction
                            .copied()
                            .unwrap_or(super::super::super::components::ActorFaction::Enemy),
                        brain.as_deref(),
                    );
                    // ONE strike spawn — the SAME `spawn_melee_strike` the player
                    // uses derives BOTH the damage hitbox AND the slash VFX from this
                    // one `attack_box`, so they can never diverge. Art KIND from the
                    // swing spec's intent (the SAME `slash_kind` mapping the player
                    // uses). Actors knock via `knockback_strength`; `knock_x` is 0.
                    let slash_kind = world_spec
                        .map(|s| crate::combat::attack::slash_kind(s.intent))
                        .unwrap_or(ambition_vfx::vfx::SlashKind::Arc);
                    super::super::hitbox::spawn_melee_strike(
                        &mut commands,
                        &mut vfx,
                        actor_entity,
                        hitbox_faction,
                        em.kin.pos,
                        attack_box,
                        1,
                        1.0,
                        0.0,
                        em.attack.active_remaining(),
                        slash_kind,
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
                let Ok((
                    _,
                    target_kin,
                    target_offense,
                    target_dodge,
                    target_shield,
                    target_combat,
                    _,
                )) = player_query.get(target_entity)
                else {
                    continue;
                };
                let target_body = target_kin.aabb();
                let target_dodge_rolling = target_dodge.roll_timer > 0.0;
                let target_vulnerable = !target_offense.invincible
                    && !target_dodge_rolling
                    && !target_shield.parrying()
                    && target_combat.vulnerable();
                if target_vulnerable && em.health.alive() && body_contact_damage_enabled {
                    if let Some(damage) =
                        em.body_contact_damage(actor_entity, target_entity, target_body)
                    {
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
    faction_by_id: &std::collections::HashMap<
        String,
        super::super::super::components::ActorFaction,
    >,
    // id → the id of the body it's actively fighting (its `ActorTarget`), so a foe is
    // never mistaken for an ally to spread from — even a SAME-faction one (two `Npc`
    // duelists feuding via a grudge).
    opponent_id_by_id: &std::collections::HashMap<String, String>,
) -> std::collections::HashMap<String, ambition_characters::brain::CrowdingSignal> {
    const CROWDING_RADIUS_PX: f32 = 80.0;
    const AERIAL_CROWDING_RADIUS_PX: f32 = 220.0;
    let mut crowding_by_id: std::collections::HashMap<
        String,
        ambition_characters::brain::CrowdingSignal,
    > = std::collections::HashMap::new();
    for (id_a, pos_a, kind_a) in requests {
        let mut count: u8 = 0;
        let mut centroid = ae::Vec2::ZERO;
        let aerial = *kind_a == crate::combat::slots::SlotKind::Aerial;
        let radius = if aerial {
            AERIAL_CROWDING_RADIUS_PX
        } else {
            CROWDING_RADIUS_PX
        };
        let faction_a = faction_by_id.get(id_a);
        for (id_b, pos_b, kind_b) in requests {
            if id_a == id_b {
                continue;
            }
            // Anti-clump is for ALLIES spreading out — an OPPONENT is to fight, not a
            // neighbor to avoid. Counting one freezes hostiles who should close (the
            // duel). A body is an opponent if it's a different faction OR it's the one
            // this fighter is actively targeting (its grudge foe in a same-faction
            // duel) — either way, don't anti-clump away from it.
            let different_faction = faction_a != faction_by_id.get(id_b);
            let is_my_target = opponent_id_by_id.get(id_a) == Some(id_b);
            if different_faction || is_my_target {
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
                    pressure: ambition_characters::brain::CrowdingSignal::compute_pressure(
                        count, 0,
                    ),
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
    target_alive: bool,
    crowding: Option<ambition_characters::brain::CrowdingSignal>,
    dt: f32,
    sim_time: f32,
    gravity_dir: ae::Vec2,
) -> ambition_characters::brain::BrainSnapshot {
    ambition_characters::brain::BrainSnapshot {
        actor_pos: em.kin.pos,
        actor_vel: em.kin.vel,
        actor_facing: em.kin.facing,
        control_down: gravity_dir,
        movement_frame_mode: ae::InputFrameMode::DEFAULT_MOVEMENT,
        aim_frame_mode: ae::InputFrameMode::DEFAULT_AIM,
        actor_on_ground: em.ground.on_ground,
        // The brain steers 2D `velocity_target` whenever the body is in FLIGHT — a
        // pure free-mover (gravity_scale == 0) OR a grounded-base hybrid that has
        // toggled flight on (`flight.fly_enabled`). Without the `fly_enabled` half a
        // hybrid that takes off keeps perceiving itself grounded and re-toggles the
        // fly intent every tick (flip-flop) instead of sustaining flight. Matches the
        // integrator's flight-limb predicate (`fly_enabled && abilities.fly`).
        actor_aerial: em.surface.gravity_scale <= 0.001 || em.flight.fly_enabled,
        alive: em.health.alive(),
        target_pos,
        // Real target liveness (was hardcoded `true`): a fighter whose foe is dead
        // perceives it and the Smash brain demotes to Idle instead of swinging at a
        // corpse. Resolved from the target entity's body-alive state by the caller.
        target_alive,
        // Own health fraction — the Smash brain watches it drop to trigger a regroup
        // (back off + reset after taking a beating).
        health_fraction: {
            let max = em.health.max().max(1) as f32;
            (em.health.current() as f32 / max).clamp(0.0, 1.0)
        },
        // Real, accumulating sim-time (scaled by bullet-time / pause) — NOT a
        // hardcoded 0.0. The Smash brain's reaction latency (`obs_history`
        // lookback by `reaction_delay_s`) only functions when this advances, so
        // threading it is what makes the difficulty knob live in-engine.
        sim_time,
        dt,
        max_run_speed: em.config.tuning.max_run_speed,
        attack_cooldown_remaining: em.attack.cooldown,
        attack_windup_remaining: em.attack.windup_remaining(),
        attack_active_remaining: em.attack.active_remaining(),
        attack_recover_remaining: 0.0,
        stun_remaining: 0.0,
        wall_contact: None,
        player_input: None,
        crowding,
        terrain: None,
        air_jumps_remaining: em.jump.air_jumps_available,
    }
}

/// Mirror the authoritative actor cluster onto the ECS read-model components
/// consumers read. Disposition is OWNED by spawn/provoke (not derived from the
/// cluster), so it is read here (to pick peaceful vs hostile `BodyCombat`)
/// but never written.
pub fn sync_actor_components_from_cluster(
    em: &super::super::actor_clusters::ActorMut<'_>,
    disposition: ActorDisposition,
    identity: &mut ActorIdentity,
    combat: &mut BodyCombat,
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
    // Health is no longer synced — it lives on the shared `BodyHealth` the cluster
    // (`em.health`) reads/writes directly; there is no separate copy to mirror.
    //
    // `BodyCombat` is otherwise a per-frame read-model rebuilt from the cluster's
    // derived presentation fields — EXCEPT the reaction timers (post-hit i-frame +
    // damage-blink), which are now the body's authoritative state (set on a landed
    // hit, decremented in the actor tick), the SAME fields the player carries. Carry
    // them across the read-model rebuild so the refresh can't wipe them.
    let damage_invuln_timer = combat.damage_invuln_timer;
    let hit_flash = combat.hit_flash;
    *combat = if disposition.is_hostile() {
        BodyCombat::hostile(
            em.health.alive(),
            hit_flash,
            em.attack.windup_remaining(),
            em.attack.active_remaining(),
            em.config.tuning.is_sandbag,
        )
    } else {
        BodyCombat::peaceful(0, hit_flash)
    };
    // (`hit_flash` is carried through the rebuild via the constructor param above;
    // `damage_invuln_timer` isn't a constructor field, so restore it explicitly.)
    combat.damage_invuln_timer = damage_invuln_timer;
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

/// Deterministic ambient-bark interval keyed by NPC id + counter — a tiny FNV
/// hash so we don't pull `rand` in for one cadence offset (mirrors the boss
/// idle-bark jitter). `base_s` is the floor and `span_ms` the jitter window:
/// the result is `base_s..base_s + span_ms/1000` seconds.
///
/// Cadence is set by the caller per occasion: a lone ambient NPC mutters fairly
/// often, but the Hall of Characters has ~100 pedestals barking at once, so it
/// uses a much longer interval — otherwise the gallery is a wall of noise.
fn npc_idle_bark_jitter(id: &str, counter: u32, base_s: f32, span_ms: u32) -> f32 {
    let mut h: u32 = 2166136261;
    for b in id.bytes() {
        h = (h ^ b as u32).wrapping_mul(16777619);
    }
    h ^= counter.wrapping_mul(2654435761);
    base_s + (h % span_ms.max(1)) as f32 / 1000.0
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
            &crate::actor::BodyCombat,
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
    // Bark cadence per occasion. The Hall packs ~100 pedestals into one room, so
    // it barks far less often than a lone ambient NPC to keep the gallery from
    // becoming a wall of speech bubbles. (base seconds, jitter window in ms.)
    let (bark_base_s, bark_span_ms) = match situation {
        ambition_characters::actor::character_catalog::BarkSituation::Hall => (28.0, 24_000),
        _ => (12.0, 8_000),
    };
    for (kin, config, combat, interaction, disposition) in &npcs {
        if disposition.is_hostile() || combat.hit_flash > 0.0 {
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
            .or_insert_with(|| npc_idle_bark_jitter(&config.id, 0, bark_base_s, bark_span_ms));
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
        state.timers.insert(
            config.id.clone(),
            npc_idle_bark_jitter(&config.id, next, bark_base_s, bark_span_ms),
        );
    }
}
