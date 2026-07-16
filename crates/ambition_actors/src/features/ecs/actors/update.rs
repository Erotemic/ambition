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

/// Per-frame steering context handed from the brain-tick phase to the movement
/// phase: each actor's nearest same-kind neighbor, keyed by actor id. Computed
/// once by `tick_actor_brains` (which already runs the slot-board / crowding
/// pass) and read by `integrate_actor_bodies` for surface-walker anti-clump
/// steering, so the movement phase doesn't recompute it. Rebuilt every frame.
#[derive(bevy::ecs::resource::Resource, Default)]
pub struct ActorSteering {
    pub neighbor_by_id: std::collections::HashMap<String, ae::Vec2>,
}

/// PHASE — tick actor brains. For every brain-driven actor: advance its reaction
/// timers, derive disposition standdown, build the perception snapshot (+ slot
/// input for a possessed `Brain::Player` body), tick the brain, and write the
/// resulting `ActorControlFrame` into `ActorControl`. This phase ticks NO body
/// position and mirrors NO read-model — brain → intent, full stop. The movement
/// phase (`integrate_actor_bodies`) reads the `ActorControl` written here. Also
/// runs the shared slot-board / crowding / neighbor pass that feeds each snapshot
/// and publishes the neighbor index to `ActorSteering` for the movement phase.
///
/// Peaceful and hostile actors share the same entity identity and switch
/// disposition in-place; dynamic encounter-spawned mobs use the same path.
pub fn tick_actor_brains(
    // Bundled into one SystemParam slot: this system is at Bevy's 16-param
    // ceiling, so the accumulating sim clock + the slot-based controller input
    // ride alongside `WorldTime`. `SlotControls` feeds any actor carrying a
    // `Brain::Player(slot)` (a possessed body) its controller frame.
    (
        world_time,
        sim_clock,
        slot_controls,
        faction_relations,
        perception_peers,
        perception_projectiles,
    ): (
        Res<WorldTime>,
        Res<crate::features::GameplayElapsed>,
        Res<ambition_characters::brain::SlotControls>,
        // The LIVE faction hostility table (init'd in `features::mod`), so a brain's
        // world-out `WorldView` resolves real hostility — not the all-false
        // `::default()` the perception build used to pass (§A7). `Option` matches
        // `select_actor_targets` for test fixtures that skip the resource.
        Option<Res<crate::combat::targeting::FactionRelations>>,
        // Pre-collected peers snapshot (§A7): the other bodies this actor perceives,
        // populated by `collect_perception_peers` before this tick. `Option` so test
        // fixtures that skip the resource fall back to an empty (terrain-only) view.
        Option<Res<crate::features::ecs::perception::PerceptionPeers>>,
        // Pre-collected projectiles snapshot (§A7): the live shots this actor perceives.
        Option<Res<crate::features::ecs::perception::PerceptionProjectiles>>,
    ),
    world: ambition_platformer_primitives::lifecycle::SessionWorldRef<
        ambition_engine_core::RoomGeometry,
    >,
    user_settings: Option<Res<ambition_persistence::settings::UserSettings>>,
    platform_set: Res<ambition_world::collision::MovingPlatformSet>,
    overlay: Res<FeatureEcsWorldOverlay>,
    mut slot_board: ResMut<crate::combat::slots::CombatSlotsRes>,
    // Neighbor index handed to the movement phase (surface-walker steering).
    mut steering: ResMut<ActorSteering>,
    // The slot-board anchor + per-target liveness read the player's position and
    // health. Multi-player ready: liveness is keyed per entity. `BodyHealth` is the
    // liveness authority (NOT `BodyCombat.alive`, an actor-cluster mirror never
    // synced for the player), consistent with `select_actor_targets`.
    player_query: Query<
        (
            bevy::prelude::Entity,
            Option<&crate::control::PlayerSlot>,
            &crate::actor::BodyKinematics,
            &ambition_characters::actor::BodyHealth,
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
                // The body's per-tick resolved frame (ADR 0024): published by
                // the frame resolution phase before this brain tick, and the
                // SAME value `integrate_sim_bodies` moves the body under. The
                // brain interprets controller input and perceives "down"
                // through it — never through a private gravity lookup.
                Option<&ambition_platformer_primitives::frame_env::ResolvedMotionFrame>,
                // Faction — read to scope the anti-clump crowding signal to
                // SAME-faction allies. Without this, two hostiles of different
                // factions (the spectator-duel fighters) count each other as
                // crowding neighbors and the anti-clump back-actor rule freezes
                // both. `Option` to match the other cluster-nested reads.
                Option<&super::super::super::components::ActorFaction>,
                // §A7: this body's per-entity grudge, so its world-out `WorldView`
                // resolves a same-faction grudge-duel opponent as hostile (matching
                // `select_actor_targets`), not by faction alone. `Option` — a body
                // with no personal feud has no grudge component read here.
                Option<&super::super::super::components::ActorAggression>,
                // §A7: this body's persistent world-belief, updated each tick from its
                // fresh `WorldView` so its brain can pursue a foe that has left the
                // viewport. Attached by `ensure_perception`; `Option` for the
                // one-frame gap before it lands (and for perception-less fixtures).
                Option<&mut crate::features::ecs::perception::PerceptionMemory>,
                // This body's PERCEPTION policy (how it learns where its foe is).
                // Attached (`Sighted`) by `ensure_perception`; `Option` reads as the
                // default `Perception::Omniscient` (the basic mode) when absent — so a
                // fixture that wires up no perception targets omnisciently, no fallback.
                Option<&crate::features::ecs::perception::Perception>,
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
        //
        // POLICY, not a fold target (E6(d), Codex 2026-07-07): this is a SWARM
        // system — per-target slot-board arbitration, sighted-perception memory,
        // and anti-clump crowding — which a boss doesn't participate in. The
        // bounded `BossAttackIntent → general move-intent` / boss-brain fold fails
        // the cheap test because it would add a boss branch that SKIPS that swarm
        // machinery while also translating boss-profile fire intent and
        // possession→special mapping. That is an adapter, not deletion of a path.
        // Keep `tick_boss_brains_system` as the non-swarm boss orchestrator; the
        // shared seams are `ActorControlFrame`, `ActorMoveset`, and the move
        // playback projection.
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
    let feature_world =
        ambition_world::collision::world_with_sandbox_solids(&world.0, &platform_set.0, &overlay);
    // Resolve the live hostility table once (default = all-peaceful) for every
    // brain's world-out view this frame (§A7).
    let relations_fallback = crate::combat::targeting::FactionRelations::default();
    let relations: &crate::combat::targeting::FactionRelations =
        faction_relations.as_deref().unwrap_or(&relations_fallback);
    let control_frame_modes = user_settings
        .as_deref()
        .map_or(ae::ControlFrameModes::default(), |s| {
            s.gameplay.control_frame_modes()
        });
    // Pick the slot-board anchor: the primary player by default, or
    // fall back to the lowest available PlayerSlot so combat slot
    // assignment stays deterministic on a multi-player non-primary build.
    // AMBITION_REVIEW(determinism): never use raw `Query::iter().next()` as
    // a player fallback here; Bevy entity iteration order is not the player order.
    // The slot read is optional so legacy/fixture primary-player entities that
    // have not yet been stamped with a `PlayerSlot` still anchor enemy brains;
    // only the non-primary fallback depends on slot ordering.
    let primary_entity = primary_q.single().ok();
    let slot_anchor_pos = primary_entity
        .and_then(|e| player_query.get(e).ok())
        .or_else(|| {
            player_query.iter().min_by_key(|(_, slot, _, _)| {
                slot.copied().unwrap_or(crate::control::PlayerSlot::PRIMARY)
            })
        })
        .map(|(_, _, kin, _)| kin.pos);
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
    for (entity, _, _, health) in &player_query {
        alive_by_entity.insert(entity, health.current() > 0);
    }
    for (entity, _, _, disposition, _, _, _, target, _, _, _, _, (clusters, _, faction, _, _, _)) in
        &actors
    {
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

    // Per-actor nearest-same-kind-neighbor index (see
    // `compute_nearest_neighbors`). Handed to the movement phase via
    // `ActorSteering` for surface-walker anti-clump steering.
    steering.neighbor_by_id = compute_nearest_neighbors(&requests);

    // Per-actor crowding signal for brains that need personal space.
    let crowding_by_id = compute_crowding_by_id(&requests, &faction_by_id, &opponent_id_by_id);

    // Pass 2: tick each actor's brain into its `ActorControl`. The slot-board
    // holding fallback that steers unassigned actors is folded into the brain
    // snapshot (crowding); movement integration is a separate phase.
    for (
        this_actor_entity,
        // aabb / identity / intent / cooldowns / mounted belong to the movement +
        // read-model phases; the query still fetches them (one actor query shape)
        // but the brain phase reads only its intent inputs.
        _aabb,
        _identity,
        mut disposition,
        mut combat,
        _intent,
        _cooldowns,
        target,
        mut brain,
        mut control,
        action_set,
        _mounted,
        (clusters, resolved_frame, faction, aggression, mut perception_memory, perception),
    ) in &mut actors
    {
        // Body-generic reaction timers on the body's authoritative `BodyCombat`
        // (the same fields the player carries): the post-hit i-frame the actor
        // gates re-hits on, the damage-blink the renderer reads, and the §A2
        // stagger set (hitstun / recoil-lock / hitstop) the movement phase
        // consumes. Decremented for every actor each tick, alive or dead — the
        // SAME decay the boss tick runs (§A1).
        combat.decay_reaction_timers(dt);

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
                // Read-only view of the body for the perception snapshot; the brain
                // tick mutates no cluster state (it writes the intent frame). Actual
                // integration happens in `integrate_actor_bodies`.
                let em = cq.as_actor_mut();

                // Every brain-attached actor builds its snapshot + world-view and
                // ticks its brain into an `ActorControlFrame`. The frame lands in
                // `ActorControl`, which the movement phase (`integrate_actor_bodies`)
                // and the EFFECTS consumers (`emit_brain_action_messages` → melee /
                // ranged) both read. Smash / Patrol / MeleeBrute / Skirmisher /
                // Sniper / Wanderer all flow through this single path. A body without
                // a brain gets a neutral frame (production spawns always attach one).
                //
                // The body's authoritative per-tick frame (resolved once; the
                // SAME value integration consumes). A cluster-bearing body
                // always carries it; absence skips the whole actor, loudly
                // caught by the reachability suites (like MotionModel).
                let Some(resolved_frame) = resolved_frame else {
                    continue;
                };
                let enemy_gravity_dir = resolved_frame.down();
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
                    // §A7 PERCEPTION POLICY: how this body learns where its foe is — a
                    // typed, per-body [`Perception`], defaulting to `Omniscient` (the
                    // BASIC mode) when the component is absent. There is NO "perception
                    // resource missing" fallback anywhere: the target branch below is the
                    // deliberate policy, not an accident of whether `PerceptionPeers` was
                    // init'd. Production actors are granted `Sighted` by `ensure_perception`;
                    // fixtures (and the boss, a separate tick) default to `Omniscient`.
                    let perception_policy = perception.copied().unwrap_or_default();
                    let viewport_half = match perception_policy {
                        // Omniscient still gets a tactical view (for the brain's
                        // line-of-fire), just at the default extent; its TARGET ignores it.
                        super::super::perception::Perception::Omniscient => {
                            super::super::perception::DEFAULT_VIEWPORT_HALF
                        }
                        super::super::perception::Perception::Sighted { viewport_half } => {
                            viewport_half
                        }
                    };
                    // Headless world-out view for this body (S4/S5), built ALWAYS for the
                    // brain's tactical queries (line-of-fire over the SAME derived
                    // collision world `feature_world` the body integrates against — never a
                    // parallel sensor). Body-generic (guardrail #1): the same
                    // `build_world_view` the player-robot body uses. The SELF-view is
                    // HONEST — real (possession-aware) faction, `can_fire` reflecting a real
                    // ranged slot, hostility against the LIVE `FactionRelations` + grudge.
                    let self_faction = crate::combat::targeting::effective_faction(
                        faction
                            .copied()
                            .unwrap_or(ambition_characters::actor::ActorFaction::Enemy),
                        Some(&*brain_ref),
                    );
                    // The other bodies this actor perceives (§A7): the pre-collected
                    // snapshot minus SELF. Empty when the resource is absent (a bare
                    // fixture) → a terrain-only view, exactly as before.
                    let view_peers: Vec<super::super::perception::PerceptionPeer> =
                        perception_peers
                            .as_ref()
                            .map(|p| {
                                p.0.iter()
                                    .filter(|peer| peer.entity != this_actor_entity)
                                    .cloned()
                                    .collect()
                            })
                            .unwrap_or_default();
                    // Self's own move phase / i-frames come from the SAME per-tick
                    // snapshot every peer's do — one derivation (`body_phase`), so a
                    // body cannot read itself more precisely than its opponent reads it.
                    let self_peer = perception_peers
                        .as_ref()
                        .and_then(|p| p.0.iter().find(|peer| peer.entity == this_actor_entity));
                    let world_view = super::super::perception::build_world_view(
                        &super::super::perception::PerceptionBody {
                            pos: em.kin.pos,
                            vel: em.kin.vel,
                            facing: em.kin.facing,
                            // FB1: was `em.kin.size` — the FULL size handed to a HALF
                            // extent. `WorldView::reachable` swept a box twice the body.
                            half_extent: em.kin.size * 0.5,
                            faction: self_faction,
                            gravity_down: enemy_gravity_dir,
                            on_ground: em.ground.on_ground,
                            aerial: em.surface.gravity_scale <= 0.001,
                            alive: em.health.alive(),
                            can_fire: action_set.is_some_and(|a| a.ranged.is_some()),
                            // Movement capability is read off the body's own
                            // `AbilitySet` — the single authority every body
                            // shares — not a parallel `CombatCapabilities` mirror.
                            can_blink: em.abilities.abilities.blink,
                            can_dash: em.abilities.abilities.dash,
                            can_shield: em.abilities.abilities.shield,
                            phase: self_peer.map(|p| p.phase).unwrap_or_default(),
                            phase_remaining: self_peer.map_or(0.0, |p| p.phase_remaining),
                            invulnerable: self_peer.is_some_and(|p| p.invulnerable),
                            damage_taken: em.health.damage_taken(),
                            health_max: em.health.max(),
                            // A grudge makes ONE same-faction body a foe (the duel
                            // mechanism); carry it so this body's `nearest_hostile`
                            // matches the foe `select_actor_targets` would pick.
                            grudge: aggression.and_then(|a| a.grudge),
                        },
                        &view_peers,
                        perception_projectiles
                            .as_ref()
                            .map(|p| p.0.as_slice())
                            .unwrap_or(&[]),
                        &[],
                        &feature_world,
                        relations,
                        viewport_half,
                        sim_now,
                    );
                    if let Some(mem) = perception_memory.as_deref_mut() {
                        mem.0.update(&world_view, dt);
                    }
                    match perception_policy {
                        // OMNISCIENT (fixtures + any body not granted senses): the snapshot
                        // already carries the global `ActorTarget` (`target_pos`/
                        // `target_alive` from `build_enemy_brain_snapshot`) — nothing to
                        // override, the body simply knows.
                        super::super::perception::Perception::Omniscient => {}
                        // SIGHTED: the brain observes its foe through the world-out port —
                        // redirect the snapshot's target onto the nearest foe IN VIEW, or,
                        // when none is visible, the most-confident foe the body REMEMBERS
                        // (pursuit of one that left the viewport, invariant I6). Perceiving
                        // nobody ⇒ no target (idle).
                        super::super::perception::Perception::Sighted { .. } => {
                            let perceived =
                                world_view.nearest_hostile().map(|a| a.pos).or_else(|| {
                                    perception_memory
                                        .as_deref()
                                        .and_then(|m| m.0.last_known_hostile().map(|r| r.pos))
                                });
                            match perceived {
                                Some(pos) => {
                                    snapshot.target_pos = pos;
                                    snapshot.target_alive = true;
                                }
                                None => {
                                    snapshot.target_pos = em.kin.pos;
                                    snapshot.target_alive = false;
                                }
                            }
                        }
                    }
                    let mut bf = ambition_characters::actor::control::ActorControlFrame::neutral();
                    let peaceful = ambition_characters::brain::ActionSet::peaceful();
                    let actions = action_set.unwrap_or(&peaceful);
                    brain_ref.tick_with_actions(actions, &snapshot, Some(&world_view), &mut bf);
                    bf
                } else {
                    ambition_characters::actor::control::ActorControlFrame::neutral()
                };
                let _ = enemy_gravity_dir;
                // Hand the brain-produced intent to the movement phase: the seam is
                // `ActorControl`, which `integrate_actor_bodies` reads next. This
                // phase writes NO body position and mirrors NO read-model.
                if let Some(control) = control.as_deref_mut() {
                    control.0 = brain_frame;
                }
            }
        }
    }
}

/// Anti-clump route steering for the adhesive crawler: does a same-kind
/// neighbor sit directly ahead along the crawl tangent (derived from the
/// published support normal + facing)? Pure, so the reversal rule is
/// unit-testable without the phase context.
pub(crate) fn crawler_neighbor_blocks(
    pos: ae::Vec2,
    size: ae::Vec2,
    facing: f32,
    surface_normal: ae::Vec2,
    neighbor: ae::Vec2,
) -> bool {
    let n = surface_normal;
    let tangent = ae::Vec2::new(-n.y * facing, n.x * facing);
    let delta = neighbor - pos;
    let along = delta.dot(tangent);
    let perp = delta.dot(n);
    let body_long = size.x * 0.5;
    let body_thick = size.y * 0.5;
    along > 0.0 && along < body_long + 6.0 && perp.abs() < body_thick + 4.0
}

/// The per-body ACTOR movement integrator — the actor-species sibling of
/// [`crate::avatar::integrate_home_body`]. Both bottom out in the SAME engine seam
/// (`ae::step_motion`, reached here via `ActorMut::update` →
/// `integrate_body`); this wrapper adds the actor-species orchestration the home
/// body doesn't need (dead/revive, AI evaluation, surface-walker step, flight
/// tuning) and reacts to the integration: the revive flash on the dead→alive edge,
/// the shark-charge crash `HitEvent`, the blink SFX/VFX for a teleport, and the
/// frame-oriented `CenteredAabb` publish. It writes the post-integration frame back
/// onto `ActorControl` so `emit_brain_action_messages` sees the same frame the old
/// fused loop did.
#[allow(clippy::too_many_arguments)]
pub(crate) fn integrate_actor_body(
    actor_entity: Entity,
    em: &mut ActorMut<'_>,
    aabb: &mut CenteredAabb,
    combat: &mut BodyCombat,
    mut control: Option<&mut ambition_characters::brain::ActorControl>,
    mut anim: Option<&mut crate::actor::BodyAnimFacts>,
    // The body's coarse footprint size: `Some` (a boss's composite render
    // envelope, from `BodyEnvelope`) publishes the `CenteredAabb` at that size;
    // `None` (every ordinary actor) publishes it at `em.kin.size` — the
    // collision box IS the footprint. This is the envelope split (AJ5.1) that
    // lets a boss share this ONE integrator instead of a bespoke arm.
    envelope: Option<ae::Vec2>,
    // The body's motion IDENTITY (AJ11 / R9.1): `None` / `AxisSwept` = the
    // axis-role swept path below; `SurfaceMomentum` dispatches to the
    // surface-follower solver — a policy field on the ONE integrator, the
    // `Perception` pattern, never a parallel system.
    motion_model: &mut MotionModel,
    target_pos: ae::Vec2,
    is_mounted: bool,
    feature_world: &ae::World,
    combat_tuning: crate::features::FeatureCombatTuning,
    steering: &ActorSteering,
    motion_frame: ae::MotionFrame,
    // The live move's authored motion lock (`MoveSpec::motion_scale_at` of the
    // body's current `MovePlayback`; `1.0` with no move playing). Applied to the
    // controller's steering intent HERE — controller attempts, body enforces —
    // so a committed heavy strike damps its owner for every controller alike
    // (autonomous brain, possession, replay).
    move_motion_scale: f32,
    dt: f32,
    feel: crate::time::feel::SandboxFeelTuning,
    sfx: &mut ambition_sfx::SfxWriter,
    vfx: &mut MessageWriter<ambition_vfx::vfx::VfxMessage>,
    hit_events: &mut MessageWriter<HitEvent>,
) {
    // The brain's intent for this body, produced upstream in `tick_actor_brains`.
    let mut brain_frame = control
        .as_deref()
        .map(|c| c.0)
        .unwrap_or_else(ambition_characters::actor::control::ActorControlFrame::neutral);
    // The move motion lock scales steering INTENT magnitude only (both the
    // grounded throttle and the free-mover command) — frame-agnostic, and
    // action edges (melee/fire/jump) pass through untouched.
    let move_motion_scale = move_motion_scale.clamp(0.0, 1.0);
    if move_motion_scale < 1.0 {
        brain_frame.locomotion *= move_motion_scale;
        brain_frame.velocity_target *= move_motion_scale;
    }
    let previous_pos = em.kin.pos;
    // Pre-update grounded snapshot for the shared movement-fx landing dust (§A8).
    let was_grounded = em.ground.on_ground;
    let shark_charge_vec = brain_frame.velocity_target;
    // Respawn blink: `em.update` revives a dead body in place; apply the revive
    // flash here on the dead→alive transition (the damage-blink lives on
    // `BodyCombat`).
    let was_dead = !em.health.alive();
    // `motion_frame` is the body's per-tick resolved frame, published ONCE by
    // the frame resolution phase and read from the body's
    // `ResolvedMotionFrame` by this driver — the same value the brain
    // interpreted controller input in earlier this tick.
    // Crawler route steering is CONTROLLER-side: reverse the crawl when a
    // same-kind neighbor blocks the path ahead (anti-clump). The kernel only
    // moves; the ECS resolves steering intent.
    if matches!(motion_model, MotionModel::AdhesiveCrawler(_)) {
        if let Some(neighbor) = steering.neighbor_by_id.get(&em.config.id).copied() {
            if crawler_neighbor_blocks(
                em.kin.pos,
                em.kin.size,
                em.kin.facing,
                em.surface.surface_normal,
                neighbor,
            ) {
                // Steering is controller intent: flip the crawl direction. The
                // kernel owns velocity (the attached crawl rewrites it each
                // tick from the new facing).
                em.kin.facing = -em.kin.facing;
            }
        }
    }
    // NOTE on hitstop: the resolver arms `combat.hitstop_timer` on every body,
    // but an actor's sim dt is NOT frozen by it (tried; per-victim freezes in
    // AI-vs-AI fights made duels degenerate — fighters spent whole bouts
    // frozen). The player-involved hitstop beat stays the global-clock rule
    // (`emit_player_time_intent_system`); a per-body proper-time beat is a
    // future ProperTimeScale concern (ADR 0011 seam).
    let (frame, move_events) = em.update(
        feature_world,
        target_pos,
        combat_tuning,
        dt,
        is_mounted,
        brain_frame,
        motion_model,
        motion_frame,
        feel,
        (combat.hitstun_timer, combat.recoil_lock_timer),
    );
    if was_dead && em.health.alive() {
        combat.hit_flash = 0.24;
    }
    let shark_crashed = shark_charge_crashed(em, is_mounted, shark_charge_vec, previous_pos);
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
    // Movement presentation for the body's frame: jump/dash/dodge/wall-jump/ledge/
    // shield/blink SFX+VFX + landing dust, through the SAME body-generic emitter
    // the player tick uses — so an AI fighter that dashes or wall-jumps produces
    // the same dust + SFX the player does, not the old blink-only actor branch
    // with its hand-copied second blink emit (fable review §A8). Fly-toggle +
    // shield are resolved INSIDE `em.update`'s shared pipeline.
    crate::features::emit_movement_fx(
        sfx,
        vfx,
        &move_events,
        em.kin.pos,
        em.kin.facing,
        em.kin.size,
        em.ground.on_ground,
        Some(was_grounded),
    );
    // Arm the op-driven overlay POSES this body earned this frame (the wall-jump
    // push-off) on its `BodyAnimFacts`, through the SAME body-generic arming the
    // player tick runs — so an AI fighter that wall-jumps shows the kick pose, not
    // just the dust (§A9 follow-up). `advance_actor_anim_overlays` decays it.
    if let Some(anim) = anim.as_deref_mut() {
        crate::features::arm_movement_anim_overlays(anim, &move_events);
    }
    // Publish the actor's footprint ORIENTED to its reference frame (a
    // surface-walker's frame is its clung surface; everyone else's is gravity at
    // their position), the single source of truth read by the debug overlay,
    // player hurtbox, and target volumes. `surface_normal` is kept LIVE for
    // every body by `em.update` (§B2), so it IS the frame — no conditional.
    let down = -em.surface.surface_normal;
    // The footprint size: a boss's coarse render envelope if it carries one,
    // else the collision box (`em.kin.size`) — the ordinary actor, whose
    // collision box IS its footprint. This is the one universal `CenteredAabb`
    // publish rule (AJ5.1); it replaces the boss's old bespoke render-sized
    // publish, so the same `to_world_half(size*0.5)` box comes out either way.
    let footprint = envelope.unwrap_or(em.kin.size);
    let body = crate::features::collision_aabb(&crate::features::SimpleActorGeometry {
        pos: em.kin.pos,
        size: footprint,
        facing: em.kin.facing,
        frame_down: down,
    });
    aabb.center = body.center();
    aabb.half_size = body.half_size();
    // Publish the post-integration frame (identical to the brain frame except a
    // shark-crash zeroes it) so `emit_brain_action_messages` — which runs after
    // WorldPrep — sees the same frame the old fused loop did.
    if let Some(control) = control.as_deref_mut() {
        control.0 = frame;
    }
}

/// PHASE — integrate sim bodies. The ONE scheduled movement phase for every
/// non-boss sim body: it reads each body's brain-produced `ActorControl` and moves
/// it through the shared movement kernel (`ae::step_motion`).
///
/// There is no separate home/player movement route. The phase is a thin driver over
/// TWO per-body integrators that are SIBLINGS — each bottoms out in that one engine
/// seam, differing only in the species-specific orchestration wrapped around it:
/// - ACTOR bodies (`FeatureSimEntity`, not player, not boss): [`integrate_actor_body`]
///   (AI eval + surface-walker/flight tuning around the seam, then blink SFX/VFX,
///   the shark-charge crash, and the frame-oriented `CenteredAabb` publish).
/// - HOME/PLAYER bodies (`PlayerEntity`): [`crate::avatar::integrate_home_body`]
///   (hitstun gate + ledge-platform carry + reset teleport around the seam, writing
///   the `PlayerBodyFrameOutput` hand-off the home reset-policy + presentation phases
///   consume).
///
/// The two live in disjoint queries because they are disjoint archetypes
/// (`With<PlayerEntity>` vs `Without<PlayerEntity>`) with different cluster shapes —
/// they cannot share one Bevy loop, but they DO share the one movement seam, which
/// is the whole point of the unification.
///
/// It integrates position ONLY — it ticks no brain and mirrors no read-model.
/// Surface-walker anti-clump steering reads the neighbor index `tick_actor_brains`
/// published to [`ActorSteering`].
#[allow(clippy::too_many_arguments)]
pub fn integrate_sim_bodies(
    world_time: Res<WorldTime>,
    world: ambition_platformer_primitives::lifecycle::SessionWorldRef<
        ambition_engine_core::RoomGeometry,
    >,
    platform_set: Res<ambition_world::collision::MovingPlatformSet>,
    feel_tuning: Res<crate::time::feel::SandboxFeelTuning>,
    overlay: Res<FeatureEcsWorldOverlay>,
    steering: Res<ActorSteering>,
    editable_tuning: Res<ambition_dev_tools::dev_tools::EditableMovementTuning>,
    user_settings: Option<Res<ambition_persistence::settings::UserSettings>>,
    mut sfx: ambition_sfx::SfxWriter,
    mut vfx: MessageWriter<ambition_vfx::vfx::VfxMessage>,
    mut hit_events: MessageWriter<HitEvent>,
    mut actors: Query<
        (
            Entity,
            &mut CenteredAabb,
            &mut BodyCombat,
            &super::super::super::components::ActorTarget,
            Option<&mut ambition_characters::brain::ActorControl>,
            Option<&mut crate::actor::BodyAnimFacts>,
            Option<&super::super::Mounted>,
            &mut MotionModel,
            &ambition_platformer_primitives::frame_env::ResolvedMotionFrame,
            &mut ambition_engine_core::BodyMotionFacts,
            Option<super::super::actor_clusters::ActorClusterQueryData>,
            // The body's live move, if any — its authored per-window motion
            // lock scales the steering intent inside `integrate_actor_body`.
            Option<&crate::combat::moveset::MovePlayback>,
        ),
        (
            With<FeatureSimEntity>,
            Without<crate::actor::PlayerEntity>,
            // POLICY (§A1/R1.1): a boss integrates through the SAME
            // `integrate_actor_body` (R1.1 dissolved its bespoke integrator), but is
            // driven from its OWN chain-1 `integrate_boss_bodies` — deliberately kept in
            // that schedule slot so the boss's presentation ordering stays byte-identical.
            // Excluding it here prevents a double integrate. Folding the boss INTO this
            // query (the optional "no boss arm") would need a chain reorder for a BLIND
            // one-frame pose lag, and the boss's chain-1 presentation systems remain
            // regardless — so the carve-out is a presentation-ordering choice, not an
            // un-unified integrator.
            Without<super::super::boss_clusters::BossConfig>,
        ),
    >,
    // Home/player bodies (primary + any brain-driven clone). Disjoint from the
    // actor query (`With<PlayerEntity>` vs `Without<PlayerEntity>`), so both borrow
    // in the same system. Each carries the SAME movement clusters an actor does; the
    // home body just also owns the `PlayerBodyFrameOutput` reset/presentation seam.
    mut players: Query<
        (
            ae::BodyClusterQueryData,
            &BodyCombat,
            &ambition_characters::brain::ActorControl,
            &mut CenteredAabb,
            &mut crate::avatar::PlayerBodyFrameOutput,
            &mut MotionModel,
            &ambition_platformer_primitives::frame_env::ResolvedMotionFrame,
            &mut ambition_engine_core::BodyMotionFacts,
            // A body that authors its own axis feel (a demo protagonist) carries
            // this; the shared sandbox protagonist does not and tracks the F3
            // dev tuning live (see the per-body resolve below).
            Option<&ambition_engine_core::AuthoredMovementTuning>,
        ),
        With<crate::actor::PlayerEntity>,
    >,
) {
    let dt = world_time.sim_dt();
    let feature_world =
        ambition_world::collision::world_with_sandbox_solids(&world.0, &platform_set.0, &overlay);
    let combat_tuning = feel_tuning.feature_combat_tuning();
    // ── ACTOR bodies (the per-body integrator, symmetric with the home body's) ──
    for (
        actor_entity,
        mut aabb,
        mut combat,
        target,
        mut control,
        mut anim,
        mounted,
        mut motion_model,
        resolved_frame,
        mut motion_facts,
        clusters,
        playback,
    ) in &mut actors
    {
        let Some(mut cq) = clusters else {
            continue;
        };
        let mut em = cq.as_actor_mut();
        integrate_actor_body(
            actor_entity,
            &mut em,
            &mut aabb,
            &mut combat,
            control.as_deref_mut(),
            anim.as_deref_mut(),
            // No actor carries a `BodyEnvelope` today — its collision box is its
            // footprint, so `CenteredAabb` publishes from `kin.size` (None).
            None,
            &mut motion_model,
            target.pos,
            mounted.is_some(),
            &feature_world,
            combat_tuning,
            &steering,
            resolved_frame.get(),
            playback.map_or(1.0, |pb| pb.spec.motion_scale_at(pb.t)),
            dt,
            *feel_tuning,
            &mut sfx,
            &mut vfx,
            &mut hit_events,
        );
        // Publish the semantic movement facts this step produced (ADR 0024):
        // presentation/combat consumers read THESE, never policy internals.
        *motion_facts = ambition_engine_core::BodyMotionFacts::from_model(&motion_model);
    }

    // ── HOME/PLAYER bodies, integrated in this SAME phase ──────────────────────
    // The home body is not a separate gameplay species: it runs the LITERAL same
    // engine entry through `integrate_home_body`, right here beside the actor
    // bodies. The tuning is built once (gravity direction + control-frame mode) and
    // shared by every player body (primary + clone); the two-clock precision-blink
    // affordance rides on `control_dt` inside the helper. No sandbox/room reset and
    // no presentation happen here — those are the home reset-POLICY and
    // PRESENTATION phases, which read the `PlayerBodyFrameOutput` this writes.
    // The shared F3 dev tuning is the fallback; a body that authors its own feel
    // (below) overrides it per-body. Built once, cheaply copied.
    let editable_player_tuning = editable_tuning.as_engine();
    let _ = &user_settings;
    let player_feel = *feel_tuning;
    let frame_dt = world_time.raw_dt;
    let scaled_dt = world_time.scaled_dt;
    for (
        mut cluster_item,
        combat,
        control,
        mut hurtbox,
        mut frame_out,
        mut motion_model,
        resolved_frame,
        mut motion_facts,
        authored_tuning,
    ) in &mut players
    {
        // Per-body feel: an authored protagonist keeps its own tuning; the
        // sandbox protagonist tracks the live inspector sliders. This is the
        // axis-path twin of a SurfaceMomentum body's params escaping the refresh.
        let player_tuning = authored_tuning
            .map(|t| t.0)
            .unwrap_or(editable_player_tuning);
        let mut clusters = cluster_item.as_clusters_mut();
        let player_motion_frame = resolved_frame.get();
        crate::avatar::integrate_home_body(
            control.0,
            &world.0,
            &mut clusters,
            combat,
            &mut hurtbox,
            &mut frame_out,
            &platform_set.0,
            &mut motion_model,
            player_motion_frame,
            player_tuning,
            player_feel,
            frame_dt,
            scaled_dt,
            &overlay,
        );
        *motion_facts = ambition_engine_core::BodyMotionFacts::from_model(&motion_model);
    }
}

/// PHASE — sync actor read-model. Mirrors each actor's integrated body state onto
/// the ECS read-model components consumers read (`ActorIdentity` / `BodyCombat`
/// presentation fields / `ActorIntent` / `ActorCooldowns`). It changes no control
/// and moves no body — it only reflects already-integrated state. Runs after
/// `integrate_actor_bodies`. Disposition is owned by spawn/provoke, so it is read
/// (to pick peaceful vs hostile combat state) but not written.
pub fn sync_actor_read_model(
    mut actors: Query<
        (
            &ActorDisposition,
            &mut ActorIdentity,
            &mut BodyCombat,
            &mut ActorIntent,
            &mut ActorCooldowns,
            Option<super::super::actor_clusters::ActorClusterQueryData>,
        ),
        (
            With<FeatureSimEntity>,
            Without<crate::actor::PlayerEntity>,
            // POLICY (§A1): a boss mirrors its read-model through its OWN chain-1
            // `sync_boss_actor_components` (which ALSO carries boss-specific encounter
            // fields — phase, timers), so it is excluded here to avoid a double sync.
            // Same non-swarm-orchestration policy as `tick_actor_brains` /
            // `integrate_boss_bodies`: the boss runs its own chain-1, deliberately.
            Without<super::super::boss_clusters::BossConfig>,
        ),
    >,
) {
    for (disposition, mut identity, mut combat, mut intent, mut cooldowns, clusters) in &mut actors
    {
        let Some(mut cq) = clusters else {
            continue;
        };
        let em = cq.as_actor_mut();
        sync_actor_components_from_cluster(
            &em,
            *disposition,
            &mut identity,
            &mut combat,
            &mut intent,
            &mut cooldowns,
        );
    }
}

/// Observer phase — body-contact damage. Reads each actor's POST-movement body
/// overlap against the player it targets and emits a `HitEvent` when they touch.
/// A pure observer of integrated body state: it ticks no brain, moves no body,
/// and mirrors no read-model — it only watches the world and emits damage facts.
/// Runs after `update_ecs_actors` (movement) so the overlap it checks is this
/// frame's resolved position. Body-contact is OFF for a player-controlled
/// (possessed) body — its brain is `Brain::Player`, it fights for you, and its
/// body must not harm you on contact (the same effective-allegiance rule the melee
/// strike + boss damage use).
#[allow(clippy::too_many_arguments)]
pub fn apply_actor_contact_damage(
    mut sfx: ambition_sfx::SfxWriter,
    mut vfx: MessageWriter<ambition_vfx::vfx::VfxMessage>,
    mut debris: MessageWriter<DebrisBurstMessage>,
    mut hit_events: MessageWriter<HitEvent>,
    // Attackers (mutable clusters) and victims (read) alias the same actor
    // archetypes now that contact damage targets ANY tracked body (fable
    // review 2026-07-02 §A4) — the ParamSet sequences the two passes.
    mut set: bevy::ecs::system::ParamSet<(
        Query<
            (
                Entity,
                &super::super::super::components::ActorTarget,
                Option<&ambition_characters::brain::Brain>,
                Option<super::super::actor_clusters::ActorClusterQueryData>,
            ),
            // Bosses are contact attackers through THIS shared system now (fable
            // AD2): their `body_contact_damage` tuning is driven from
            // `behavior.body_damage` at spawn, so no `Without<BossConfig>` carve-out.
            (With<FeatureSimEntity>, Without<crate::actor::PlayerEntity>),
        >,
        // Victims: any body with a published footprint — a player, an NPC a
        // provoked enemy tracks, a duel opponent. The ONE vulnerability rule
        // (§A5) + the ONE published hurtbox (§A6).
        Query<(
            &CenteredAabb,
            &crate::actor::BodyOffense,
            &ambition_engine_core::BodyMotionFacts,
            &crate::actor::BodyShieldState,
            &ambition_characters::actor::BodyCombat,
            bevy::prelude::Has<crate::actor::PlayerEntity>,
        )>,
    )>,
) {
    // Pass 1 — snapshot each live contact attack while the attacker's clusters
    // are borrowed.
    let mut pending: Vec<(Entity, Entity, crate::features::enemies::ContactAttack)> = Vec::new();
    for (actor_entity, target, brain, clusters) in &mut set.p0() {
        let Some(mut cq) = clusters else {
            continue;
        };
        let em = cq.as_actor_mut();
        // Body-contact hazard is off for any player-controlled body; derived from
        // the brain (no possession special-case), gated by the body's authored
        // `body_contact_damage` tuning.
        let enabled = !brain.is_some_and(ambition_characters::brain::Brain::is_player)
            && em.config.tuning.body_contact_damage;
        if !enabled || !em.health.alive() {
            continue;
        }
        // The body this actor tracks (already resolved relationally by
        // `select_actor_targets` — a foe by faction or grudge); its entity is
        // stamped on the emitted `HitEvent::target` so the right victim
        // consumer lands the hit.
        let Some(target_entity) = target.entity else {
            continue;
        };
        if let Some(attack) = em.contact_attack() {
            pending.push((actor_entity, target_entity, attack));
        }
    }
    // Pass 2 — resolve each victim through its published hurtbox.
    let victims = set.p1();
    for (attacker, target_entity, attack) in pending {
        let Ok((hurtbox, offense, facts, shield, combat, is_player)) = victims.get(target_entity)
        else {
            continue;
        };
        if !crate::combat::util::body_vulnerable(offense, facts.dodge_rolling, shield, combat) {
            continue;
        }
        if let Some(damage) = attack.hit_event(attacker, target_entity, hurtbox.aabb(), is_player) {
            let pos = damage
                .knockback
                .as_ref()
                .map(|k| k.impact_pos)
                .unwrap_or_else(|| damage.volume.center());
            sfx.write(ambition_sfx::SfxMessage::Play {
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
///
/// NOTE: production no longer consumes this — the per-actor `slot_pos` it fed was
/// already dead (`let _ = slot_pos`) before the monolith split, so the whole
/// slot-board *steering* (`assign_slots` → holding-ring) is a latent no-op; the
/// brain's crowding signal drives spacing now. Kept (test-covered) pending a
/// decision to rip out slot-board steering entirely — logged in code_smells.
#[allow(
    dead_code,
    reason = "test-covered; slot-board steering is a pending removal"
)]
pub(crate) fn compute_holding_positions(
    board: &crate::combat::slots::CombatSlotBoard,
    requests: &[(String, ae::Vec2, crate::combat::slots::SlotKind)],
    player_pos: ae::Vec2,
) -> std::collections::HashMap<String, ae::Vec2> {
    // BTreeMap, not HashMap: the outer loop below assigns round-robin slot ranks,
    // so its iteration order is part of the output (N0.3 — no hash order in sim).
    let mut unassigned_by_kind: std::collections::BTreeMap<
        crate::combat::slots::SlotKind,
        Vec<&str>,
    > = std::collections::BTreeMap::new();
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
        // BossPattern-only inputs — inert for actor bodies.
        boss_encounter_phase: None,
        world_size: ae::Vec2::ZERO,
        front_wall_clearance: None,
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
    let hitstun_timer = combat.hitstun_timer;
    let recoil_lock_timer = combat.recoil_lock_timer;
    let hitstop_timer = combat.hitstop_timer;
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
    // the other reaction timers — post-hit i-frame + the §A2 stagger set — aren't
    // constructor fields, so restore them explicitly.)
    combat.damage_invuln_timer = damage_invuln_timer;
    combat.hitstun_timer = hitstun_timer;
    combat.recoil_lock_timer = recoil_lock_timer;
    combat.hitstop_timer = hitstop_timer;
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
/// ([`crate::features::npcs::npc_ambient_bark_line`]) mutters a line every
/// ~6–10s, so it feels alive between conversations. Skips hostile NPCs and any
/// still showing a hit-flash bubble (so it never talks over a hit bark). The
/// stochastic parrot is the first user; any NPC gains barks by adding a pool.
pub fn tick_npc_idle_barks(
    world_time: Res<WorldTime>,
    npcs: Query<
        (
            &super::super::actor_clusters::BodyKinematics,
            &super::super::actor_clusters::ActorConfig,
            &ambition_characters::actor::BodyCombat,
            &ActorInteraction,
            &ActorDisposition,
        ),
        With<FeatureSimEntity>,
    >,
    mut vfx: MessageWriter<ambition_vfx::vfx::VfxMessage>,
    room_set: Option<
        ambition_platformer_primitives::lifecycle::SessionWorldRef<crate::rooms::RoomSet>,
    >,
    // App-local authored voice. Required so a mis-composed production App
    // cannot silently erase provider-authored dialogue.
    character_catalog: Res<ambition_characters::actor::character_catalog::CharacterCatalog>,
    mut state: Local<NpcIdleBarkState>,
) {
    let dt = world_time.scaled_dt;
    if dt <= 0.0 {
        return;
    }
    let catalog = &*character_catalog;
    // In a GALLERY room (the Hall of Characters), pedestals draw their `Hall`
    // bark pool (the fun gallery lines); everywhere else NPCs mutter their
    // `Idle` pool. Same ambient ticker, different occasion — keyed off the
    // engine-generic `RoomMetadata::gallery` flag, not a content room id (C1).
    let is_gallery = room_set
        .as_deref()
        .map(|rs| rs.active_metadata().gallery)
        .unwrap_or(false);
    let situation = if is_gallery {
        ambition_characters::actor::character_catalog::BarkSituation::Hall
    } else {
        ambition_characters::actor::character_catalog::BarkSituation::Idle
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
            catalog,
            &interaction.interactable,
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
