//! The per-frame actor tick: syncing poses from feature AABBs, driving the
//! enemy + NPC updates, neighbor/crowding queries, and brain snapshots.

use super::super::*;
use super::*;
use ambition_combat::components::{
    ActorDisposition, ActorIdentity, ActorInteraction, CenteredAabb,
};
use ambition_combat::events::{HitEvent, HitMode, HitSource, HitTarget};
use ambition_platformer2d_shared_tangle::lifecycle::FeatureSimEntity;

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
            &mut ambition_combat::components::ActorPose,
            Option<&ambition_platformer2d_core::BodyKinematics>,
            Option<ambition_boss_encounter::BossClusterRef>,
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
        *pose =
            ambition_combat::components::ActorPose::from_parts(aabb.center, aabb.half_size, facing);
    }
}

/// Per-frame steering context handed from observation to the movement phase:
/// each actor's nearest same-kind neighbor, keyed by actor id. Computed
/// once by [`observe_actor_decision_inputs`] and read by `integrate_sim_bodies`
/// for surface-walker anti-clump
/// steering, so the movement phase doesn't recompute it. Rebuilt every frame.
#[derive(bevy::ecs::resource::Resource, Default)]
pub struct ActorSteering {
    pub neighbor_by_id: std::collections::HashMap<String, ae::Vec2>,
}

/// Immutable, frame-local world facts consumed by autonomous actor decisions.
///
/// This is a derived projection, rebuilt from authoritative body state before
/// every decision phase. It deliberately contains values rather than ECS
/// borrows: observation owns the cross-body scan; decision owns the stateful
/// brain call. Neither phase gets to perform the other's job by reaching back
/// into its queries.
// `pub`, because `tick_actor_brains` is publicly re-exported and takes it: a
// signature may not be more public than its own arguments. Narrowing the system
// instead would mean splitting it out of two `pub use` lists for a name nothing
// outside this crate consumes today.
#[derive(bevy::ecs::resource::Resource, Default)]
pub struct ActorDecisionFacts {
    crowd: super::crowd_observation::CrowdFacts,
}

/// Frame-local output of the autonomous DECIDE phase.
///
/// Decision owns brain state and produces plain intent values. Publication owns
/// the authoritative [`ambition_characters::control::ActorControl`] mutation.
/// Keeping the hand-off as data prevents a decision system from acquiring body
/// control authority just because movement consumes the result later in the tick.
#[derive(bevy::ecs::resource::Resource, Default)]
pub struct ActorDecisionFrames {
    frames: Vec<(
        Entity,
        ambition_characters::actor::control::ActorControlFrame,
    )>,
}

/// OBSERVE — build the cross-body facts this tick's autonomous decisions read.
///
/// Observation owns the population scan; decision consumes this derived
/// resource. The schedule edge between them is the same-tick contract.
pub(crate) fn observe_actor_decision_inputs(
    mut facts: ResMut<ActorDecisionFacts>,
    mut steering: ResMut<ActorSteering>,
    controlled: Query<
        (
            bevy::prelude::Entity,
            &ambition_characters::actor::BodyHealth,
        ),
        bevy::prelude::With<ambition_platformer2d_shared_tangle::markers::PlayerEntity>,
    >,
    actors: Query<
        (
            Entity,
            &ActorDisposition,
            &ambition_combat::components::ActorTarget,
            Option<super::super::actor_clusters::ActorClusterQueryDataReadOnly>,
            Option<&ambition_combat::components::ActorFaction>,
            bevy::prelude::Has<ambition_combat::components::ActiveCombatant>,
        ),
        (
            With<FeatureSimEntity>,
            // Preserve the pre-split actor population without granting this
            // observation phase mutable authority over these components.
            With<CenteredAabb>,
            With<ActorIdentity>,
            With<BodyCombat>,
            With<ambition_platformer2d_core::movement::MotionModel>,
            Without<ambition_platformer2d_shared_tangle::markers::PlayerEntity>,
            Without<ambition_boss_encounter::BossConfig>,
            Without<crate::features::ecs::dormancy::Dormant>,
        ),
    >,
) {
    let mut observation = super::crowd_observation::CrowdObservation::default();
    for (entity, health) in &controlled {
        observation.note_controlled_liveness(entity, health.current() > 0);
    }
    for (entity, disposition, target, body, faction, in_a_fight) in &actors {
        let fighting = ambition_combat::components::CombatStanding::of(*disposition, in_a_fight)
            .takes_damage();
        observation.note_actor(
            entity,
            body.as_ref().is_some_and(|body| body.health.alive()),
            body.as_ref()
                .map(|body| super::crowd_observation::ObservedBody {
                    id: body.config.id.as_str(),
                    pos: body.kin.pos,
                    kind: body.config.tuning.crowd_kind(),
                    faction: faction.copied(),
                    foe: target.entity,
                }),
            fighting,
        );
    }
    let crowd = observation.finish();
    steering.neighbor_by_id = crowd.neighbor_index().clone();
    facts.crowd = crowd;
}

/// MAINTAIN — advance actor reaction clocks before decision and movement.
///
/// Observation runs first, preserving the existing rule that perception samples
/// the pre-decay combat phase while movement consumes the decayed clocks.
pub(crate) fn maintain_actor_pre_decision_state(
    world_time: Res<WorldTime>,
    mut actors: Query<
        &mut BodyCombat,
        (
            With<FeatureSimEntity>,
            // Exact eligibility from the old fused brain query. `With` keeps
            // the population contract without borrowing unrelated authority.
            With<CenteredAabb>,
            With<ActorIdentity>,
            With<ActorDisposition>,
            With<ambition_combat::components::ActorTarget>,
            With<ambition_platformer2d_core::movement::MotionModel>,
            Without<ambition_platformer2d_shared_tangle::markers::PlayerEntity>,
            Without<ambition_boss_encounter::BossConfig>,
            Without<crate::features::ecs::dormancy::Dormant>,
        ),
    >,
) {
    let dt = world_time.sim_dt();
    for mut combat in &mut actors {
        combat.decay_reaction_timers(dt);
    }
}

/// The causal log's slot in the brain-tick system's parameter bundle.
///
/// `ambition_causal` is an optional, default-off dependency (§2e), so the slot
/// has to exist in both builds — a `#[cfg]` cannot be written on a tuple
/// element. `PhantomData` is the shipped build's answer: a zero-sized
/// `SystemParam` that costs the scheduler nothing.
#[cfg(feature = "causal")]
pub type CausalLend<'w> = Option<ResMut<'w, ambition_causal::CausalRecording>>;
#[cfg(not(feature = "causal"))]
pub type CausalLend<'w> = std::marker::PhantomData<&'w ()>;

/// Run the brain tick with the log lent to this thread, when there is one.
///
/// A brain publishes through `ambition_causal::record`, which writes to a
/// THREAD-LOCAL sink, and Bevy runs systems across worker threads — so those
/// facts were landing in nothing and being counted by `facts_lost_offthread()`.
///
/// the alternative was threading a recorder into `tick_with_actions`, and
/// it is worse: it puts the log on the simulation's own signatures. This crate
/// already refused that once — `record_player_movement_intent` runs AFTER the
/// brain tick precisely so "a system that only reads cannot be the thing that
/// broke the tick". The host opening the sink around the call keeps that
/// property and fixes the loss.
#[cfg(feature = "causal")]
fn with_causal_sink<T>(causal: &mut CausalLend<'_>, body: impl FnOnce() -> T) -> T {
    match causal.as_deref_mut() {
        // Lent only when something is actually listening: with instrumentation
        // off — the shipped default — this is one branch and the brain call is
        // byte-for-byte the old one.
        Some(log) if log.is_recording() => log.lend_to_thread(body),
        _ => body(),
    }
}

#[cfg(not(feature = "causal"))]
fn with_causal_sink<T>(_causal: &mut CausalLend<'_>, body: impl FnOnce() -> T) -> T {
    body()
}

/// DECIDE — tick autonomous actor brains from the already-observed world facts.
///
/// This phase mutates only decision-owned state (`Brain`, `PerceptionMemory`) and
/// produces frame-local [`ActorDecisionFrames`]. It does not write
/// [`ambition_characters::control::ActorControl`]; the following PUBLISH phase
/// owns that mutation. Body reaction clocks, disposition maintenance, cross-body
/// observation, movement, and read-model projection are separate scheduled phases.
///
/// Peaceful and hostile actors share the same entity identity; target and
/// disposition transitions have already settled before this phase. Dynamic
/// encounter-spawned mobs use the same path.
pub fn tick_actor_brains(
    // Inputs stay named by authority. Parameter count is not a reason to hide
    // independent contracts in a tuple or context bag.
    world_time: Res<WorldTime>,
    // Accumulating sim-time, for the brain's reaction-latency lookback.
    sim_clock: Res<crate::features::GameplayElapsed>,
    // Peers, projectiles and hostility: what a body can perceive this tick.
    perceived: crate::features::ecs::perception::PerceivedWorld,
    // Capture, as a fact this phase reads and hands on. The brain never
    // touches `CapturedBy` — a pure decision reaching into the ECS is what the
    // perception layer exists to prevent — so the relationship is resolved HERE,
    // where the queries live, and travels as three plain values.
    captives: Query<(
        bevy::prelude::Entity,
        &ambition_combat::capture::CapturedBy,
        // the RELATION says who holds whom; the RULESET's state says how long
        // and how many. `Option` because a hold this ruleset has no opinion
        // about is a real thing.
        Option<&ambition_characters::smash_capture::SmashHoldState>,
    )>,
    // The log, lent to whichever worker thread this tick lands on.
    //
    // A brain publishes through `ambition_causal::record`, which writes to a
    // THREAD-LOCAL sink — and Bevy runs systems across worker threads, so
    // those facts were being counted by `facts_lost_offthread()` and dropped.
    // Threading a recorder down into `tick_with_actions` was the alternative
    // and is worse: it would put the log on the simulation's own signatures,
    // and this crate has already refused that once (the movement observer
    // runs AFTER the brain tick so "a system that only reads cannot be the
    // thing that broke the tick").
    //
    // So the HOST opens the sink around the brain call instead.
    mut causal: CausalLend,
    // `CollisionWorld` is the seam that already owned that composition; the brain tick simply had
    // never adopted it.
    collision: ambition_platformer2d_world::collision::CollisionWorld,
    // Cross-body liveness/crowding was observed in the preceding phase. The
    // decision loop reads the resulting values and does not rescan the actor
    // population itself.
    decision_facts: Res<ActorDecisionFacts>,
    mut decisions: ResMut<ActorDecisionFrames>,
    mut actors: Query<
        (
            Entity,
            &ambition_combat::components::ActorTarget,
            // Stateful decision policy. The frame it produces is buffered in
            // `ActorDecisionFrames`; this phase never borrows `ActorControl`.
            // `Option` because dynamically-spawned actors (debug tools, scripted
            // spawns) might skip brain attachment and therefore decide neutral.
            Option<&mut ambition_characters::brain::Brain>,
            // Driver authority suppresses autonomous decisions and contributes
            // to the body's effective faction view.
            Option<&ambition_characters::control::DrivingParticipant>,
            bevy::prelude::Has<ambition_characters::control::ActorControl>,
            // ActionSet — read for the Smash brain so it knows which
            // attacks (melee / ranged) the actor can commit. `Option`
            // so dynamically-spawned actors without a set still tick.
            Option<&ambition_characters::brain::ActionSet>,
            (
                // The generated read-only view of the COMPLETE actor cluster.
                // Using the existing cluster shape preserves the old eligibility
                // contract exactly while removing decision's mutable body authority.
                Option<super::super::actor_clusters::ActorClusterQueryDataReadOnly>,
                // The brain interprets controller input and perceives "down" through it — never
                // through a private gravity lookup.
                Option<&ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame>,
                // Faction is still a self-view input; crowd observation consumed
                // its own copy in the preceding phase.
                Option<&ambition_combat::components::ActorFaction>,
                // §A7: this body's per-entity grudge, so its world-out `WorldView`
                // resolves a same-faction grudge-duel opponent as hostile (matching
                // `select_actor_targets`), not by faction alone. `Option` — a body
                // with no personal feud has no grudge component read here.
                Option<&ambition_combat::components::ActorAggression>,
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
                // FB4b §13.2: the body's own moveset, so the brain snapshot can
                // carry the ATTACK KIT. The fighter brain scores real moves with
                // real frame data and cannot reach a moveset itself —
                // `ambition_combat` depends on `ambition_characters`, not the
                // reverse — so this is the world-in port doing what it is for.
                // `Option` because a body with no moveset (a peaceful NPC, a
                // prop) has no kit, and an empty kit is the honest answer.
                Option<&ambition_combat::moveset::ActorMoveset>,
                // THE MOVE THAT CURRENTLY OWNS THIS BODY, so the attack kit
                // can say which of its candidates could be STARTED this tick —
                // see `ActionLegality`. Here for exactly the reason the moveset
                // above is: the brain reads no ECS, so a fact it needs about the
                // body arrives through the world-in port.
                //
                // A brain inferring "I look like I am in recovery" would be answering a different
                // question and would be wrong for every move with a cancel window.
                Option<&ambition_combat::moveset::MovePlayback>,
                // WHAT IS TRUE OF THIS BODY'S LOCOMOTION, published once per
                // tick by the movement kernel. The brain snapshot's
                // `turns_at_walls` reads it instead of the spawn-time
                // `tuning.surface_walker` flag ADR 0024 §8 forbids at runtime.
                // `Option` because a body without the movement clusters has no
                // facts, and "not crawling" is the honest default for one.
                Option<&ambition_platformer2d_core::BodyMotionFacts>,
                // THE BODY'S MOTION MODEL, for the ONE burst-maneuver rule
                // (`resolve_burst_maneuver`): dodge-vs-dash availability lives
                // in the model's `AxisManeuverState` (air-dodge window, endlag)
                // as well as in the dodge/dash clusters. A driver that decides
                // its maneuver from CAPABILITIES instead names one thing and the
                // kernel performs another — see `SelfView::burst`.
                //
                // Requiring it costs nothing: the integration query below
                // (`integrate_sim_bodies`) already takes `&mut MotionModel` non-optionally over
                // the same archetype, so a body without one is not integrated at all and has no
                // locomotion for a brain to reason about.
                &ambition_platformer2d_core::movement::MotionModel,
            ),
        ),
        // Exclude BOSSES too: they carry the shared actor read-models
        // (`ActorIdentity`/`ActorDisposition`/… synced by `sync_boss_actor_components`) but have NO
        // actor cluster, so without this they'd match here (cluster = `None`) and get ticked by the
        // actor loop ON TOP of their own `tick_boss_brains_system` — a double brain tick.
        (
            With<FeatureSimEntity>,
            // Removing mutable access must not broaden the phase population.
            // These were mandatory members of the old fused actor query; keep
            // them as eligibility only, not as decision authority.
            With<CenteredAabb>,
            With<ActorIdentity>,
            With<ActorDisposition>,
            With<BodyCombat>,
            Without<ambition_platformer2d_shared_tangle::markers::PlayerEntity>,
            Without<ambition_boss_encounter::BossConfig>,
            // A DORMANT ACTOR DOES NOT DECIDE. Only the brain sleeps: the
            // body still integrates, so a dormant actor mid-fall keeps falling
            // and simply stops choosing. Absent on every actor that declares no
            // `DormancyPolicy`, so this filter changes nothing for content that
            // has not asked. See `features::ecs::dormancy`.
            Without<crate::features::ecs::dormancy::Dormant>,
        ),
    >,
) {
    // Sim clock: enemies, NPCs, encounter mobs all advance on the
    // gameplay clock so bullet-time / pause / hitstop freeze them
    // alongside the player. ADR 0010 + reference_lessons_learned.
    // A derived decision buffer is rebuilt from scratch every tick, including
    // ticks with no collision world. Publication may therefore never replay a
    // stale autonomous frame after a schedule gate or world transition.
    decisions.frames.clear();

    let dt = world_time.sim_dt();
    // Accumulating sim-time for brain perception (reaction-latency lookback).
    let sim_now = sim_clock.0;
    let Some(feature_world) = collision.solids() else {
        return;
    };
    // The live hostility table for every brain's world-out view this frame (§A7),
    // all-peaceful when a fixture registers none.
    let relations = perceived.relations();
    // a world with no controlled body is ordinary, and the shape that proves it is the
    // absence of an early return here. When the anchor was live this read `let Some(player_pos)
    // = ... else { return; }`, so a session that declared no home avatar ticked NO actor brains
    // at all — every actor everywhere, not merely the ones near a player. Nothing in this
    // system may become conditional on a player existing again.

    // The population scan and body-state maintenance have already run. This
    // loop is now one authority: evaluate autonomous decision state and produce
    // the resulting intent value for the following publish phase.
    for (
        this_actor_entity,
        target,
        mut brain,
        driver,
        has_control,
        action_set,
        (
            body,
            resolved_frame,
            faction,
            aggression,
            mut perception_memory,
            perception,
            moveset,
            playback,
            motion_facts,
            motion_model,
        ),
    ) in &mut actors
    {
        // This actor's combat-target liveness. `select_actor_targets` already
        // dropped a dead/absent foe (it only ever targets a LIVE candidate, and a
        // faction-feud fighter has no target once its foe is gone), so `entity ==
        // None` here means "no one to fight" → the brain idles (peaceful behavior).
        let target_alive = match target.entity {
            Some(e) => decision_facts.crowd.is_alive(e),
            None => false,
        };
        // `target.pos` is populated by `select_actor_targets`
        // (#17.8); it defaults to the actor's spawn-of-game position
        // when no players exist yet (pre-spawn / post-death-of-all),
        // and is the primary player's pos in the single-player
        // production game.
        let target_pos = target.pos;
        {
            // A production actor carries the complete read-only decision view.
            // `Option` keeps incomplete debug/scripted fixtures from being
            // silently filtered out of the outer query; they simply have no
            // autonomous decision to evaluate.
            let Some(body) = body else {
                continue;
            };
            {
                // Every brain-attached actor builds its snapshot + world-view and
                // ticks its brain into an `ActorControlFrame`. The following PUBLISH
                // phase commits that value to `ActorControl`, which movement and the
                // EFFECTS consumers (`emit_brain_action_messages` → melee / ranged)
                // both read. Smash / Patrol / MeleeBrute / Skirmisher /
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
                // A PARTICIPANT'S BODY IS NOT THIS SYSTEM'S TO DECIDE FOR.
                // A possessed actor carries `DrivingParticipant(slot)`, and its
                // `ActorControl` is produced a whole phase earlier by
                // `tick_controlled_brains` — the one seam that turns participant
                // control into an intent frame, for the home avatar and for this
                // body by the same rule.
                //
                // What this skip DELETES is what a human was paying for to move a
                // stick: an enemy brain snapshot, a perception policy, a world view
                // built over the collision world, a believed-target derivation, and
                // a MUTATION of this body's `PerceptionMemory` — none of which the
                // player-brain translator reads. It is not free to build them
                // either: possessing a body would restart its sight memory every
                // tick from a view nobody consulted.
                //
                // Observation and state maintenance already ran for a driven
                // body; only the autonomous decision is suppressed here.
                if driver.is_some() {
                    continue;
                }
                let brain_frame = if let Some(brain_ref) = brain.as_deref_mut() {
                    let crowding = decision_facts.crowd.crowding(&body.config.id);
                    let capture = ambition_combat::capture::systems::CaptureFacts::resolve(
                        this_actor_entity,
                        &captives,
                    );
                    let mut snapshot = build_enemy_brain_snapshot(
                        &body,
                        target_pos,
                        target_alive,
                        crowding,
                        dt,
                        sim_now,
                        enemy_gravity_dir,
                        moveset,
                        Some(brain_ref),
                        // THE BODY'S RUNNING MOVE, so the kit can say which
                        // of its candidates the body could actually BEGIN this
                        // tick. Threaded rather than re-queried because the
                        // brain layer reads no ECS, exactly like the moveset
                        // above.
                        playback,
                        // A body with no movement clusters publishes no
                        // locomotion facts, and "none of them true" is the
                        // honest reading of that.
                        &motion_facts.copied().unwrap_or_default(),
                        capture,
                    );
                    // §A7 PERCEPTION POLICY: how this body learns where its foe is — a
                    // typed, per-body [`Perception`], defaulting to `Omniscient` (the
                    // BASIC mode) when the component is absent. There is NO "perception
                    // resource missing" fallback anywhere: the target branch below is the
                    // deliberate policy, not an accident of whether `PerceptionPeers` was
                    // init'd. Production actors are granted `Sighted` by `ensure_perception`;
                    // fixtures (and the boss, a separate tick) default to `Omniscient`.
                    let perception_policy = perception.copied().unwrap_or_default();
                    // Headless world-out view for this body (S4/S5), built ALWAYS for the
                    // brain's tactical queries (line-of-fire over the SAME derived
                    // collision world `feature_world` the body integrates against — never a
                    // parallel sensor). Body-generic (guardrail #1): the same
                    // `build_world_view` the player-robot body uses. The SELF-view is
                    // HONEST — real (possession-aware) faction, `can_fire` reflecting a real
                    // ranged slot, hostility against the LIVE `FactionRelations` + grudge.
                    let self_faction = ambition_combat::targeting::effective_faction(
                        faction
                            .copied()
                            .unwrap_or(ambition_characters::actor::ActorFaction::Enemy),
                        // always `None` on this branch — the loop `continue`s
                        // above for any body a participant drives — and stated
                        // rather than folded away, because the SELF-view is
                        // documented as "real (possession-aware) faction" and a
                        // future reader must see which term carries that.
                        driver,
                    );
                    // The other bodies this actor perceives (§A7): the pre-collected
                    // snapshot minus SELF.
                    let view_peers = perceived.peers_seen_by(this_actor_entity);
                    // Self's own move phase / i-frames come from the SAME per-tick
                    // snapshot every peer's do — one derivation (`body_phase`), so a
                    // body cannot read itself more precisely than its opponent reads it.
                    let self_peer = perceived.peer(this_actor_entity);
                    // WHAT THIS BODY'S BURST BUTTON WOULD DO IF PRESSED NOW.
                    // The kernel's own rule (`resolve_burst_maneuver`), asked one
                    // phase early — so the brain names the maneuver the body will
                    // actually perform instead of re-deriving the precedence.
                    //
                    // a body on a NON-axis model has no `AxisManeuverState` to
                    // read, and the default reads as "no window open, no endlag".
                    // that is a reading of a PRESENT model that is not
                    // axis-swept — a crawler genuinely has no air-dodge window —
                    // and NOT a reading of an absent component, which ADR 0024 §1
                    // forbids and the query above now makes unrepresentable.
                    let world_view = super::super::perception::build_world_view(
                        &super::super::perception::perception_body_for(
                            &body,
                            self_faction,
                            enemy_gravity_dir,
                            action_set,
                            self_peer,
                            aggression,
                            motion_model,
                            capture,
                        ),
                        &view_peers,
                        perceived.projectiles(),
                        &[],
                        &feature_world,
                        relations,
                        perception_policy,
                        sim_now,
                    );
                    // Sight and memory answer together; an `Omniscient` body
                    // already carries the global `ActorTarget` and is not
                    // overridden. Perceiving nobody is a real answer (idle).
                    if let Some(believed) = super::super::perception::believed_target(
                        perception_policy,
                        &world_view,
                        perception_memory.as_deref_mut(),
                        dt,
                    ) {
                        match believed {
                            Some(pos) => {
                                snapshot.target_pos = pos;
                                snapshot.target_alive = true;
                            }
                            None => {
                                snapshot.target_pos = body.kin.pos;
                                snapshot.target_alive = false;
                            }
                        }
                    }
                    let mut bf = ambition_characters::actor::control::ActorControlFrame::neutral();
                    let peaceful = ambition_characters::brain::ActionSet::peaceful();
                    let actions = action_set.unwrap_or(&peaceful);
                    with_causal_sink(&mut causal, || {
                        crate::brain_tick::tick_brain_with_actions(
                            brain_ref,
                            actions,
                            &snapshot,
                            Some(&world_view),
                            &mut bf,
                        )
                    });
                    bf
                } else {
                    ambition_characters::actor::control::ActorControlFrame::neutral()
                };
                let _ = enemy_gravity_dir;
                // Decision ends in a plain value. The next phase is the only
                // autonomous writer of `ActorControl`; movement cannot begin until
                // that publish has completed. A body with no control component still
                // advances its brain state, matching the old fused path.
                if has_control {
                    decisions.frames.push((this_actor_entity, brain_frame));
                }
            }
        }
    }
}

/// PUBLISH — commit this tick's autonomous decisions to body control.
///
/// This is intentionally the only autonomous `ActorControl` mutation after brain
/// evaluation. The system is narrow enough that Bevy's access graph expresses
/// the authority edge directly: DECIDE writes a derived buffer, PUBLISH writes
/// control, and integration only follows after PUBLISH.
pub(crate) fn publish_actor_decision_frames(
    decisions: Res<ActorDecisionFrames>,
    mut controls: Query<
        &mut ambition_characters::control::ActorControl,
        Without<ambition_characters::control::DrivingParticipant>,
    >,
) {
    for (entity, frame) in &decisions.frames {
        if let Ok(mut control) = controls.get_mut(*entity) {
            control.0 = *frame;
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
    // Whose cues this body emits (A13). Read-only, looked up by entity so it does
    // not have to ride the already-12-wide cluster tuple.
    presentation_source: Option<&ambition_sfx::PresentationSourceId>,
    em: &mut ActorMut<'_>,
    aabb: &mut CenteredAabb,
    combat: &mut BodyCombat,
    mut control: Option<&mut ambition_characters::control::ActorControl>,
    mut anim: Option<&mut ambition_characters::actor::BodyAnimFacts>,
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
    // Somebody is riding THIS body. Guards the charge-crash suicide.
    is_being_ridden: bool,
    // Somebody is carrying THIS body. Declines the locomotion pass.
    pose_owned_externally: bool,
    feature_world: &ae::World,
    combat_tuning: ambition_combat::events::FeatureCombatTuning,
    steering: &ActorSteering,
    motion_frame: ae::MotionFrame,
    // The live move's authored motion lock (`MoveSpec::motion_scale_at` of the
    // body's current `MovePlayback`; `1.0` with no move playing). Applied to the
    // controller's steering intent HERE — controller attempts, body enforces —
    // so a committed heavy strike damps its owner for every controller alike
    // (autonomous brain, possession, replay).
    move_motion_scale: f32,
    // The move PLAYING on this body — the last term of the helpless derivation,
    // and the VALUE rather than a bool because only a RECOVERY postpones it.
    playing_a_move: Option<&ambition_combat::moveset::MovePlayback>,
    // Is this body TUMBLING? The caller reads the PUBLISHED projection
    // (`BodyMotionFacts::tumbling`), which is what the post-hit gate needs so a
    // falling body's tech press survives the stagger. Threaded rather than
    // derived here for the same reason `authored_tuning` below is: the fact
    // lives on the entity and this function takes clusters.
    tumbling: bool,
    // Is this body's death window open (ADR 0033)? Threaded for the same reason
    // `tumbling` is — the fact lives on the entity and this function takes
    // clusters — and read on BOTH roads, which is the part that was missing.
    out_of_play: bool,
    dt: f32,
    feel: ambition_combat::feel::Platformer2dFeelTuningMonolith,
    // This body's own movement feel, when its character authored one.
    //
    // The actor twin of the player loop's `authored_tuning`. Threaded rather
    // than resolved here because the component lives on the entity and this
    // function takes clusters, not a `World`.
    authored_tuning: Option<ae::MovementTuning>,
    sfx: &mut ambition_sfx::SfxWriter,
    vfx: &mut MessageWriter<ambition_vfx::vfx::VfxMessage>,
    hit_events: &mut MessageWriter<HitEvent>,
    // The kernel's own operation list, for the causal instrument only. `Option`
    // for the reason the damage path documents: an instrument that can take
    // down a composition is worse than no instrument. See
    // `crate::causal::BodyMovementOps`.
    #[cfg(feature = "causal")] movement_ops: Option<
        &mut MessageWriter<crate::causal::BodyMovementOps>,
    >,
    // The other solid bodies this one may not walk through, resolved from
    // the pre-integration snapshot by the caller. Inert for a body whose
    // composition never granted the capability, which is every body outside a
    // ruleset that opted its cast in.
    contact_field: ae::BodyContactField<'_>,
) {
    // The brain's intent for this body, produced upstream in `tick_actor_brains`.
    let brain_frame = control
        .as_deref()
        .map(|c| c.0)
        .unwrap_or_else(ambition_characters::actor::control::ActorControlFrame::neutral);
    // The move motion lock scales steering INTENT magnitude only — and it is
    // `ActorControlFrame`'s own rule now, because the HOME road applies the
    // identical one and two copies would drift.
    let brain_frame = brain_frame.damped_by_move_motion(move_motion_scale);
    let previous_pos = em.kin.pos;
    let shark_charge_vec = brain_frame.velocity_target.vec();
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
    // One named predicate, asked on both roads.
    //
    // so if hitlag ever feels too sticky, tune its DURATION or SHAPE.
    // Restoring a controlled-body/actor asymmetry here is forbidden, and a
    // comment recommending it is how the last one nearly got restored.
    let (frame, move_events) = em.update(
        feature_world,
        target_pos,
        combat_tuning,
        dt,
        // ⛔ THE RIDER'S FACT, NOT THE MOUNT'S. `is_being_ridden` guards the
        // charge-crash below and belongs to a body somebody is sitting ON;
        // what `update` needs is whether somebody is carrying THIS one.
        pose_owned_externally,
        brain_frame,
        motion_model,
        motion_frame,
        playing_a_move,
        feel,
        authored_tuning,
        combat,
        tumbling,
        out_of_play,
        contact_field,
    );
    if was_dead && em.health.alive() {
        combat.hit_flash = 0.24;
    }
    let shark_crashed = shark_charge_crashed(em, is_being_ridden, shark_charge_vec, previous_pos);
    let mut frame = frame;
    if shark_crashed {
        // ⛔⛔ THIS KILLS OUTRIGHT, WHATEVER THE POOL IS. The damage below is the
        // body's ENTIRE remaining health, so a charge crash is a detonation and
        // not a hit — which is why raising a summoned shark's HP does nothing
        // about it. Named in the log because "its health pool reached zero" and
        // "it blew itself up" are the same reading downstream and want different
        // fixes.
        bevy::log::info!(
            target: "ambition::mount",
            "charge crash DETONATES: entity={actor_entity:?} being_ridden={is_being_ridden}              health_spent={}",
            em.health.current().max(1),
        );
        hit_events.write(HitEvent {
            strike_sfx: None,
            volume: em.aabb().into(),
            damage: em.health.current().max(1),
            source: HitSource::Contact,
            attacker: Some(actor_entity),
            target: HitTarget::Volume,
            mode: HitMode::Knockback,
            knockback: None,
            ignored_targets: Vec::new(),
        });
        frame = ambition_characters::actor::control::ActorControlFrame::neutral();
    }
    // THE BLAST ZONE. The kernel's gate flags a body that left the world, and
    // for an actor that flag was read for presentation and then dropped on the
    // floor — so an enemy that walked into a pit, or a fighter knocked off a
    // stage, fell FOREVER: still alive, still ticked, still accelerating, still
    // in every broadcast loop, with nothing anywhere to end it. Actors are never
    // teleported to the player spawn, and the comment promising that "the
    // actor's damage / OOB systems own that" named systems that do not exist.
    //
    // The fix rides the channel the shark-crash above already uses: a
    // world-caused death is a lethal hit whose attacker is nobody. That buys
    // the whole existing death pipeline unchanged — `RulesetOwnsDeath`, the
    // authored respawn policy, the banner, the death cue, and an
    // `ActorDiedMessage` carrying `HitSource::LeftTheWorld`.
    //
    // Gated on ALIVE, because the gate is a position test and re-fires every tick the body is
    // past the margin.
    if em.health.alive() && move_events.reset == Some(ae::ResetCause::LeftTheWorld) {
        hit_events.write(HitEvent {
            strike_sfx: None,
            volume: em.aabb().into(),
            damage: em.health.current().max(1),
            source: HitSource::LeftTheWorld,
            // Nobody. Crediting the last attacker for a knock-off is a RULES
            // question a platform fighter answers with its own hitlag memory,
            // and this seam must not pre-empt it by guessing.
            attacker: None,
            // This body, resolved: the blast zone caught exactly one body, and
            // a broadcast over its AABB would also catch whoever chased it out.
            target: HitTarget::Body(actor_entity),
            mode: HitMode::Knockback,
            knockback: None,
            ignored_targets: Vec::new(),
        });
    }
    // Fly-toggle + shield are resolved INSIDE `em.update`'s shared pipeline. The kernel NAMED
    // what it did; hand that to the instrument beside the FX that already read it. One publish
    // covers every kernel velocity writer, which is why this is not sixty-eight
    // instrumentations.
    #[cfg(feature = "causal")]
    if let Some(writer) = movement_ops {
        if !move_events.operations.is_empty() {
            writer.write(crate::causal::BodyMovementOps {
                body: actor_entity,
                ops: move_events.operations.clone(),
            });
        }
    }
    crate::features::emit_movement_fx(
        sfx,
        vfx,
        &move_events,
        em.kin.pos,
        em.kin.facing,
        em.kin.size,
        presentation_source,
    );
    // Arm the op-driven overlay POSES this body earned this frame (the wall-jump
    // push-off) on its `BodyAnimFacts`, through the SAME body-generic arming the
    // player tick runs — so an AI fighter that wall-jumps shows the kick pose, not
    // just the dust (§A9 follow-up). `advance_actor_anim_overlays` decays it.
    if let Some(anim) = anim.as_deref_mut() {
        crate::features::arm_ground_contact_anim_overlay(anim, move_events.ground_contact);
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
    ambition_combat::body_geometry::publish_body_footprint(
        aabb,
        em.kin.pos,
        footprint,
        em.kin.facing,
        down,
    );
    // Publish the post-integration frame (identical to the brain frame except a
    // shark-crash zeroes it) so `emit_brain_action_messages` — which runs after
    // WorldPrep — sees the same frame the old fused loop did.
    if let Some(control) = control.as_deref_mut() {
        control.0 = frame;
    }
}

/// Snapshot grounded solid-body contact boxes before any body integrates.
///
/// All bodies resolve against the same entry poses, avoiding query-order
/// dependence. Airborne bodies are excluded; standing on another body is handled
/// by footstool semantics. The snapshot is cleared every tick before republishing.
pub fn snapshot_body_contact(
    mut snapshot: ResMut<ambition_platformer2d_shared_tangle::body::BodyContactSnapshot>,
    bodies: Query<(
        bevy::prelude::Entity,
        &ambition_platformer2d_shared_tangle::body::BodyKinematics,
        &ae::BodyGroundState,
        &ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame,
        &ambition_platformer2d_shared_tangle::body::BodyContact,
    )>,
) {
    snapshot.clear();
    for (entity, kinematics, ground, frame, contact) in &bodies {
        if !ground.on_ground {
            continue;
        }
        snapshot.push(
            entity,
            kinematics.aabb_oriented(frame.down()),
            // its ENTRY velocity — this pass runs before any body resolves its
            // controller, which is the whole point of a common snapshot.
            kinematics.vel,
            contact.resistance,
        );
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
/// Surface-walker anti-clump steering reads the neighbor index
/// [`observe_actor_decision_inputs`] published to [`ActorSteering`].
/// The cue and event writers this integration publishes through.
///
/// ONE PARAMETER RATHER THAN FOUR, and the reason is a hard ceiling rather than
/// taste. A Bevy system takes at most sixteen parameters; this one sat at
/// sixteen and the `causal` feature adds the seventeenth, so **with that feature
/// on `integrate_sim_bodies` stopped being a system at all** — and the error
/// says `no method named `in_set``, naming the registration rather than the
/// limit, which is why an optional feature could be unbuildable without anyone
/// noticing. Bundling is what the camera resolve already does for the same
/// reason.
#[derive(bevy::ecs::system::SystemParam)]
pub struct BodyIntegrationCues<'w> {
    pub sfx: ambition_sfx::SfxWriter<'w>,
    pub vfx: MessageWriter<'w, ambition_vfx::vfx::VfxMessage>,
    pub hit_events: MessageWriter<'w, HitEvent>,
    /// The kernel's operation list, for the causal instrument. `Option` so a
    /// composition with no inspector registers nothing and publishes nothing —
    /// the rule the damage path already documents.
    #[cfg(feature = "causal")]
    pub movement_ops: Option<MessageWriter<'w, crate::causal::BodyMovementOps>>,
}

#[allow(clippy::too_many_arguments)]
pub fn integrate_sim_bodies(
    // A13: whose cues each body emits, looked up by entity. A separate read-only
    // query rather than another member of the cluster tuple, which is already at
    // twelve.
    body_sources: Query<&ambition_sfx::BodyPresentationSource>,
    // Empty in every composition that has not granted the capability, and an empty snapshot
    // answers `BodyContactField::NONE` for every body.
    contact: Res<ambition_platformer2d_shared_tangle::body::BodyContactSnapshot>,
    // The per-body blocker list, reused across bodies and across ticks. a
    // `Local` rather than a fresh `Vec` per body: this is the innermost loop of
    // the movement phase.
    mut contact_scratch: Local<Vec<ae::BodyContactBlocker>>,
    world_time: Res<WorldTime>,
    world: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
        ambition_platformer2d_core::RoomGeometry,
    >,
    platform_set: Res<ambition_platformer2d_world::collision::MovingPlatformSet>,
    feel_tuning: Res<ambition_combat::feel::Platformer2dFeelTuningMonolith>,
    overlay: Res<ambition_platformer2d_shared_tangle::feature_overlay::FeatureEcsWorldOverlay>,
    steering: Res<ActorSteering>,
    active_tuning: Res<ambition_platformer2d_core::ActiveMovementTuning>,
    user_settings: Option<Res<ambition_persistence::settings::UserSettings>>,
    mut cues: BodyIntegrationCues,
    mut actors: Query<
        (
            Entity,
            &mut CenteredAabb,
            &mut BodyCombat,
            &ambition_combat::components::ActorTarget,
            Option<&mut ambition_characters::control::ActorControl>,
            Option<&mut ambition_characters::actor::BodyAnimFacts>,
            // ⛔⛔ THE SADDLE, NOT THE RIDER'S MARKER. This row used to read
            // `Option<&Mounted>`, and `Mounted` is stamped on the RIDER — see
            // `mount::board`, which puts `RidingOn`/`Mounted` on the rider and
            // `MountSlot` on the mount. So a shark CARRYING a rider never had
            // `Mounted`, this always resolved false, and the `!is_mounted` guard
            // on the charge-crash suicide never fired. The pirate boarded and the
            // shark detonated itself against the stage about twenty milliseconds
            // later, every single time, taking the recovery with it.
            //
            // ⭐ THE QUESTION IS "AM I BEING RIDDEN", and only the saddle answers
            // it. Asking the rider's marker of a mount is a category error that
            // reads as a plain `false`, which is why it survived: the guard was
            // there, it was spelled right, and it was wired to a component this
            // entity can never have.
            Option<&ambition_mount::MountSlot>,
            // ⛔⛔ THE OTHER END OF THE SAME RELATIONSHIP, and it is a different
            // question. `MountSlot` above says "somebody is riding ME"; this
            // says "somebody is carrying me". A body can be either, neither, or
            // — a rider on a mount that is itself ridden — both.
            bevy::prelude::Has<ambition_platformer2d_core::PoseOwnedExternally>,
            &mut MotionModel,
            &ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame,
            &mut ambition_platformer2d_core::BodyMotionFacts,
            Option<super::super::actor_clusters::ActorClusterQueryData>,
            // The body's live move, if any — its authored per-window motion
            // lock scales the steering intent inside `integrate_actor_body`.
            Option<&ambition_combat::moveset::MovePlayback>,
            // The body's own FEEL, if its character authored one.
            //
            // this component was GRANTED to every seated fighter and read by nobody on this path:
            // `presentation.rs` inserts it precisely so a seated fighter and a worn player move
            // alike, and the only consumer in the repository was the PLAYER loop below.
            Option<&ambition_platformer2d_core::AuthoredMovementTuning>,
            // Has this body's death window been opened (ADR 0033)? ⛔⛔ THIS WAS
            // HARD-CODED `false` at the `step_body` call, under a comment
            // stating as a FACT that `OutOfPlay` is only ever granted to a
            // participant's body — true only while `open_death_interlude` was
            // the sole opener. A Smash fighter is NOT a `PlayerEntity`, so when
            // the stocks respawn beat began opening a window (D201) the body it
            // was meant to hold still went on integrating: it kept the velocity
            // that launched it and coasted through the wait, and a held jump
            // still reached it.
            //
            // The player query below has always read this. Two roads, one
            // question — see the `playing_a_move` note on `ActorMut::update`,
            // which is the same defect on the same function.
            bevy::prelude::Has<ambition_combat::death_rules::OutOfPlay>,
        ),
        (
            With<FeatureSimEntity>,
            Without<ambition_platformer2d_shared_tangle::markers::PlayerEntity>,
            // POLICY (§A1/R1.1): a boss integrates through the SAME
            // `integrate_actor_body` (R1.1 dissolved its bespoke integrator), but is
            // driven from its OWN chain-1 `integrate_boss_bodies` — deliberately kept in
            // that schedule slot so the boss's presentation ordering stays byte-identical.
            // Excluding it here prevents a double integrate. Folding the boss INTO this
            // query (the optional "no boss arm") would need a chain reorder for a BLIND
            // one-frame pose lag, and the boss's chain-1 presentation systems remain
            // regardless — so the carve-out is a presentation-ordering choice, not an
            // un-unified integrator.
            Without<ambition_boss_encounter::BossConfig>,
        ),
    >,
    // Home/player bodies (primary + any brain-driven clone). Disjoint from the
    // actor query (`With<PlayerEntity>` vs `Without<PlayerEntity>`), so both borrow
    // in the same system. Each carries the SAME movement clusters an actor does; the
    // home body just also owns the `PlayerBodyFrameOutput` reset/presentation seam.
    mut players: Query<
        (
            // Whose body this is, so the contact snapshot can hand it every
            // OTHER solid body's box without including its own.
            Entity,
            ae::BodyClusterQueryData,
            // MUTABLE so the body step can bank and spend the automatic
            // displacement it owes after a hitlag — see `BodyCombat::asdi_owed`.
            &mut BodyCombat,
            // the body's own reason set, because a hazard TILE is damage.
            // A player who cannot be hurt — a super form, a transformation beat,
            // a scripted grant — must not be reset to spawn by walking over
            // spikes. `Option` because a home body without health is a valid
            // scratch/test body and there is nothing to ask.
            Option<&ambition_characters::actor::BodyHealth>,
            &ambition_characters::control::ActorControl,
            &mut CenteredAabb,
            &mut crate::avatar::PlayerBodyFrameOutput,
            &mut MotionModel,
            &ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame,
            &mut ambition_platformer2d_core::BodyMotionFacts,
            // The body's live move, exactly as the actor query above carries it:
            // its authored per-window motion lock scales this body's steering
            // intent too. ⛔⛔ IT WAS ABSENT HERE, so every rule expressed as a
            // motion lock — a committed swing, and the charge root Jon asked for
            // by name — was live for brain-driven bodies and silently off for the
            // one a human drives.
            Option<&ambition_combat::moveset::MovePlayback>,
            // A body that authors its own axis feel (a demo protagonist) carries
            // this; the shared sandbox protagonist does not and tracks the F3
            // dev tuning live (see the per-body resolve below).
            Option<&ambition_platformer2d_core::AuthoredMovementTuning>,
            // The ridden-surface presentation fact this integration publishes
            // (the roll righting reflex tilts a rider's feet onto it).
            Option<&mut ambition_platformer2d_shared_tangle::orientation::SurfaceUpright>,
            // Has this participant's attempt already ended (ADR 0033)? The
            // world's reset gate must not act on a body that has already lost —
            // see the guard inside `integrate_home_body`.
            bevy::prelude::Has<ambition_combat::death_rules::OutOfPlay>,
        ),
        With<ambition_platformer2d_shared_tangle::markers::PlayerEntity>,
    >,
) {
    let dt = world_time.sim_dt();
    let feature_world = ambition_platformer2d_world::collision::world_with_sandbox_solids(
        &world.0,
        &platform_set.0,
        &overlay,
    );
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
        pose_owned_externally,
        mut motion_model,
        resolved_frame,
        mut motion_facts,
        clusters,
        playback,
        authored_tuning,
        out_of_play,
    ) in &mut actors
    {
        let Some(mut cq) = clusters else {
            continue;
        };
        let mut em = cq.as_actor_mut();
        integrate_actor_body(
            actor_entity,
            body_sources.get(actor_entity).ok().map(|s| s.id()),
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
            // Ridden means somebody is IN the saddle, not merely that a saddle
            // exists: an empty `MountSlot` outlives its rider's dismount.
            mounted.is_some_and(|slot| slot.rider.is_some()),
            pose_owned_externally,
            &feature_world,
            combat_tuning,
            &steering,
            resolved_frame.get(),
            playback.map_or(1.0, |pb| pb.motion_scale_now()),
            playback,
            // LAST TICK's published tumble, which is the read this is owed:
            // the projection below is written after the step, and a tech window
            // is many ticks long.
            motion_facts.tumbling,
            out_of_play,
            dt,
            *feel_tuning,
            authored_tuning.map(|t| t.0),
            &mut cues.sfx,
            &mut cues.vfx,
            &mut cues.hit_events,
            #[cfg(feature = "causal")]
            cues.movement_ops.as_mut(),
            contact.field_for(actor_entity, &mut contact_scratch),
        );
        // Publish the semantic movement facts this step produced (ADR 0024):
        // presentation/combat consumers read THESE, never policy internals.
        *motion_facts = ambition_platformer2d_core::BodyMotionFacts::from_model(&motion_model);
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
    let editable_player_tuning = active_tuning.0;
    let _ = &user_settings;
    let player_feel = *feel_tuning;
    let frame_dt = world_time.raw_dt;
    let scaled_dt = world_time.scaled_dt;
    for (
        player_entity,
        mut cluster_item,
        mut combat,
        health,
        control,
        mut hurtbox,
        mut frame_out,
        mut motion_model,
        resolved_frame,
        mut motion_facts,
        playback,
        authored_tuning,
        mut surface_upright,
        out_of_play,
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
        let riding_up = crate::avatar::integrate_home_body(
            control.0,
            &feature_world,
            &mut clusters,
            &mut combat,
            health.map_or_else(ambition_characters::actor::Invulnerability::none, |h| {
                h.health.invulnerable
            }),
            motion_facts.evading(),
            motion_facts.tumbling,
            out_of_play,
            &mut hurtbox,
            &mut frame_out,
            &mut motion_model,
            player_motion_frame,
            player_tuning,
            player_feel,
            playback.map_or(1.0, |pb| pb.motion_scale_now()),
            frame_dt,
            scaled_dt,
            // The move itself, not merely whether one is playing: only a
            // RECOVERY postpones helplessness, which is why the derivation reads
            // the spec's gate rather than the component's presence.
            playback.map(|pb| &*pb),
            contact.field_for(player_entity, &mut contact_scratch),
        );
        *motion_facts = ambition_platformer2d_core::BodyMotionFacts::from_model(&motion_model);
        // Input-relative facts the model projection can't know: republished
        // here, beside the ridden-surface fact (all are per-tick projections of
        // this integration; the roll reflex and the anim picker consume them).
        motion_facts.skidding =
            crate::avatar::surface_skidding(&motion_model, control.0.locomotion.x);
        if let Some(surface) = surface_upright.as_mut() {
            surface.up = riding_up;
        }
    }
}

/// PHASE — sync the actor identity read-model.
///
/// Every combat fact it wrote turned out to be a duplicate of an authority the reader could ask
/// directly: liveness (`BodyHealth`), melee (`BodyMelee`), the sandbag flag (authored, set at
/// construction), and three fields nobody read at all. So the query loses `ActorDisposition`,
/// `BodyCombat` and `Has<ActiveCombatant>` — the last of which existed ONLY to choose between a
/// peaceful and a hostile rebuild that no longer happens.
///
/// It changes no control and moves no body. Runs after `integrate_sim_bodies`.
pub fn sync_actor_read_model(
    mut actors: Query<
        (
            &mut ActorIdentity,
            Option<super::super::actor_clusters::ActorClusterQueryData>,
        ),
        (
            With<FeatureSimEntity>,
            Without<ambition_platformer2d_shared_tangle::markers::PlayerEntity>,
            // POLICY (§A1): a boss mirrors its read-model through its OWN chain-1
            // `sync_boss_actor_components` (which ALSO carries boss-specific encounter
            // fields — phase, timers), so it is excluded here to avoid a double sync.
            // Same non-swarm-orchestration policy as `tick_actor_brains` /
            // `integrate_boss_bodies`: the boss runs its own chain-1, deliberately.
            Without<ambition_boss_encounter::BossConfig>,
        ),
    >,
) {
    for (mut identity, clusters) in &mut actors {
        let Some(mut cq) = clusters else {
            continue;
        };
        let em = cq.as_actor_mut();
        sync_actor_components_from_cluster(&em, &mut identity);
    }
}

/// Observer phase — body-contact damage. Reads each actor's POST-movement body
/// overlap against the player it targets and emits a `HitEvent` when they touch.
/// A pure observer of integrated body state: it ticks no brain, moves no body,
/// and mirrors no read-model — it only watches the world and emits damage facts.
/// Runs after `update_ecs_actors` (movement) so the overlap it checks is this
/// frame's resolved position. Body-contact is OFF for a participant-driven
/// (possessed) body — it holds a `DrivingParticipant`, it fights for you, and its
/// body must not harm you on contact (the same effective-allegiance rule the melee
/// strike + boss damage use).
#[allow(clippy::too_many_arguments)]
pub fn apply_actor_contact_damage(
    // The struck body's own victim consumer emits its `HurtFeedback` now, so this system only
    // writes the `HitEvent`.
    mut hit_events: MessageWriter<HitEvent>,
    mut set: bevy::ecs::system::ParamSet<(
        Query<
            (
                Entity,
                &ambition_combat::components::ActorTarget,
                Option<&ambition_characters::control::DrivingParticipant>,
                // ⭐⭐ IS THIS BODY SEATED IN A MATCH? D206. Whether touching a
                // body hurts is a CHARACTER trait
                // (`CharacterBodyBlueprint::contact_damage`), and that is right
                // for the overworld: a goblin you walk into hurts you. It is
                // wrong for a versus match, where a fighter's body is never a
                // permanent hazard — the genre puts damage in MOVES.
                //
                // MEASURED: goblin vs `perfect_cellular_automaton`, both
                // authoring a contact-damage block, traded **6,908 Contact hit
                // events in 3,776 ticks — 109.8/s** against 29.5/s of melee,
                // because this system writes an event EVERY TICK two bodies
                // overlap and the only thing pacing it is the victim's i-frames.
                // That is what Jon heard as *"a bad sfx problem with goblin and
                // pca"*: every one of those events asks for `player.hit`, the
                // unauthored default for an enemy-profile victim. A george
                // mirror was quiet at 223 — not because the engine behaves
                // differently, but because George authors no contact block.
                //
                // ⛔ THE SEAT AND NOT THE TUNING. `actor_clusters` builds this
                // tuning on the SHARED character→body road, which the overworld
                // NPC and the seated fighter both take, so it cannot answer a
                // question about the match. And a ruleset that reached in and
                // rewrote the tuning would be editing a construction-time fact
                // that a re-seat puts straight back. The seat is the authority
                // and this is the read that asks it.
                //
                // ⛔ NOT a claim that a fighter can never harm on contact. A
                // move that wants a damaging body state grants it as a move —
                // Sanic's ball dash, a super form, a spiked shell — and none of
                // those flows through this permanent trait.
                Has<crate::character_runtime::MatchSeat>,
                Option<super::super::actor_clusters::ActorClusterQueryData>,
            ),
            // Bosses are contact attackers through THIS shared system now (fable
            // AD2): their `body_contact_damage` tuning is driven from
            // `behavior.body_damage` at spawn, so no `Without<BossConfig>` carve-out.
            (
                With<FeatureSimEntity>,
                Without<ambition_platformer2d_shared_tangle::markers::PlayerEntity>,
            ),
        >,
        // Victims: any body with a published footprint — a player, an NPC a
        // provoked enemy tracks, a duel opponent. The ONE vulnerability rule
        // (§A5) + the ONE published hurtbox (§A6).
        Query<(
            &CenteredAabb,
            &ambition_characters::actor::BodyHealth,
            &ambition_platformer2d_core::BodyMotionFacts,
            &ambition_platformer2d_core::BodyShieldState,
            &ambition_characters::actor::BodyCombat,
        )>,
    )>,
) {
    // Pass 1 — snapshot each live contact attack while the attacker's clusters
    // are borrowed.
    let mut pending: Vec<(Entity, Entity, crate::features::enemies::ContactAttack)> = Vec::new();
    for (actor_entity, target, driver, seated_in_a_match, clusters) in &mut set.p0() {
        let Some(mut cq) = clusters else {
            continue;
        };
        let em = cq.as_actor_mut();
        // Body-contact hazard is off for any participant-driven body; derived
        // from the DRIVER (no possession special-case), gated by the body's
        // authored `body_contact_damage` tuning.
        let enabled =
            driver.is_none() && !seated_in_a_match && em.config.tuning.body_contact_damage;
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
        let Ok((hurtbox, victim_health, facts, shield, combat)) = victims.get(target_entity) else {
            continue;
        };
        if !ambition_combat::util::body_vulnerable(
            victim_health.health.invulnerable,
            facts.evading(),
            shield,
            combat,
        ) {
            continue;
        }
        if let Some(damage) = attack.hit_event(attacker, target_entity, hurtbox.aabb()) {
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
    requests: &[(String, ae::Vec2, ambition_combat::crowd::CrowdKind)],
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

/// Per-actor crowding signal (personal-space pressure) consumed by
/// brains like Smash so clustered actors push apart. Aerial actors use a
/// wider radius and only count *other aerial* actors (so flyers like
/// sharks don't stack), while ground actors use a tighter radius. Pure
/// over the per-tick slot requests `(id, pos, kind)` so it is
/// unit-testable in isolation from the actor tick.
pub(crate) fn compute_crowding_by_id(
    requests: &[(String, ae::Vec2, ambition_combat::crowd::CrowdKind)],
    faction_by_id: &std::collections::HashMap<String, ambition_combat::components::ActorFaction>,
    // id → the id of the body it's actively fighting (its `ActorTarget`), so a foe is
    // never mistaken for an ally to spread from — even a SAME-faction one (two `Npc`
    // duelists feuding via a grudge).
    opponent_id_by_id: &std::collections::HashMap<String, String>,
) -> std::collections::HashMap<String, ambition_characters::brain::smash::CrowdingSignal> {
    const CROWDING_RADIUS_PX: f32 = 80.0;
    const AERIAL_CROWDING_RADIUS_PX: f32 = 220.0;
    let mut crowding_by_id: std::collections::HashMap<
        String,
        ambition_characters::brain::smash::CrowdingSignal,
    > = std::collections::HashMap::new();
    for (id_a, pos_a, kind_a) in requests {
        let mut count: u8 = 0;
        let mut centroid = ae::Vec2::ZERO;
        let aerial = *kind_a == ambition_combat::crowd::CrowdKind::Aerial;
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
            if aerial && *kind_b != ambition_combat::crowd::CrowdKind::Aerial {
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
                ambition_characters::brain::smash::CrowdingSignal {
                    same_faction_count: count,
                    other_faction_count: 0,
                    away_dir: away,
                    pressure: ambition_characters::brain::smash::CrowdingSignal::compute_pressure(
                        count, 0,
                    ),
                },
            );
        }
    }
    crowding_by_id
}

/// The attacks this body can actually throw, as the fighter brain reads them.
///
/// One row per move in the contract, with the frame data a player who read the
/// tables would know. Declaration order, which `MovesetContract.moves` is a
/// `Vec` — so the kit is stable across ticks and across a replay, and no sort is
/// needed to make it deterministic.
pub(super) fn attack_kit_of(
    moveset: Option<&ambition_combat::moveset::ActorMoveset>,
    // The body's REAL posture this tick. The kit is what it can press NOW.
    grounded: bool,
    // only a FIGHTER brain reads the kit, and building it is a `Vec` of
    // owned move ids and frame data — per actor, per tick. Every other brain in
    // the game would have paid for a list nothing looks at, which is a cost
    // §13.2 explicitly said to fix by rebuilding on moveset CHANGE. It does not
    // need that yet: the cheaper answer is not to build it for a brain that
    // cannot use it, and this is the one place that knows which brain a body has.
    brain: Option<&ambition_characters::brain::Brain>,
    // The move that currently owns the body, if any. Each candidate carries
    // whether it could be STARTED this tick, answered by the same
    // `cancel_permits` call `trigger_moveset_moves` makes — see
    // `ActionLegality`. `None` means nothing owns the body and everything is
    // startable.
    playback: Option<&ambition_combat::moveset::MovePlayback>,
) -> Vec<ambition_characters::brain::fighter::options::AttackCandidate> {
    use ambition_characters::brain::{Brain, StateMachineCfg};
    if !matches!(
        brain,
        Some(Brain::StateMachine(StateMachineCfg::Fighter { .. }))
    ) {
        return Vec::new();
    }
    let Some(moveset) = moveset else {
        return Vec::new();
    };
    use ambition_characters::actor::attack_gesture::AttackDir;
    use ambition_characters::brain::fighter::options::{
        AttackBinding, AttackCandidate, AttackVerb,
    };

    // ENUMERATE THE PRESSES, ASK WHAT EACH ONE REACHES.
    //
    // this listed `moveset.moves` — every move the body owns, whether or not
    // any input can invoke it — and the candidate carried no way to invoke the
    // one that won. Two failures in one line: a
    // move with no binding (a buff, a summon, an on-hit technique) could be
    // SCORED and then come out as a generic swing, and the winner's identity had
    // nowhere to travel.
    //
    // Asking the moveset's OWN resolver — `move_for_directional_verb`, the same
    // function `trigger_moveset_moves` calls — makes the kit executable by
    // construction: every candidate is a move some press reaches, and the press
    // is the candidate. The chain falls back (`attack_air_up` → `attack_up` →
    // `attack_air` → `attack`), so a body that authors only a base attack yields
    // that move once and a body with the full directional set yields each.
    //
    // The POSTURE is the body's real one, never a choice: a brain that could
    // claim `Grounded` while airborne would pick a move its body cannot perform,
    // which is the no-cheat contract's whole subject.
    let mut kit: Vec<AttackCandidate> = Vec::new();
    for (verb, verb_name) in [
        (AttackVerb::Basic, ambition_combat::moveset::ATTACK_VERB),
        (AttackVerb::Smash, ambition_combat::moveset::SMASH_VERB),
        (AttackVerb::Special, ambition_combat::moveset::SPECIAL_VERB),
    ] {
        for direction in [
            AttackDir::Neutral,
            AttackDir::Forward,
            AttackDir::Back,
            AttackDir::Up,
            AttackDir::Down,
        ] {
            let Some(spec) = moveset
                .0
                .move_for_directional_verb(verb_name, direction, grounded)
            else {
                continue;
            };
            // One entry per (move, press). A base-only moveset answers all five
            // directions with the same move, and the NEUTRAL press is the
            // honest binding for it — the others would claim a direction the
            // resolution ignores, and a scored duplicate is a thumb on the
            // scale for whichever move happens to answer more presses.
            if kit.iter().any(|c| c.move_id == spec.id) {
                continue;
            }
            kit.push(AttackCandidate {
                move_id: spec.id.clone(),
                frames: spec.frame_data(),
                binding: AttackBinding { verb, direction },
                legality: legality_of(playback, verb_name, &spec.id),
            });
        }
    }
    // AND THE GRAB, which the three loops above cannot reach: it answers its
    // own button, not a direction on one of theirs.
    if let Some(grab) = capture_candidate(moveset, grounded, playback) {
        kit.push(grab);
    }
    kit
}

/// CAN THE BODY BEGIN THIS MOVE THIS TICK? — asked of the same function that
/// actually decides it.
///
/// A brain inferring legality from "I look like I am in recovery" would answer a different question
/// and be wrong for every move that authors a cancel window — the proxy-instrument failure, one
/// layer up from where it usually shows up.
///
/// the name list must match `trigger_moveset_moves` exactly: the verb the
/// press resolves through, plus the resolved move id. That is the one cancel
/// namespace, and asking with a different list would make this answer a
/// question nothing enforces.
fn legality_of(
    playback: Option<&ambition_combat::moveset::MovePlayback>,
    verb_name: &str,
    move_id: &str,
) -> ambition_characters::brain::fighter::options::ActionLegality {
    use ambition_characters::brain::fighter::options::ActionLegality;
    let Some(pb) = playback else {
        // Nothing owns the body: every candidate is startable.
        return ActionLegality::Now;
    };
    if pb
        .spec
        .cancel_permits(pb.t, pb.landed_hit, &[verb_name, move_id])
    {
        ActionLegality::Now
    } else {
        ActionLegality::BlockedByPlayback
    }
}

/// The authored GRAB, priced as an option like any other technique.
///
/// A CPU that knew "George grabs at 44px" would be a CPU that stops working the day George is
/// retuned, and a second fighter would need a second constant.
///
/// Two things the ordinary derivation cannot supply, because a grab lands no hit
/// volume and `frame_data` reads volumes:
///
/// ```text
/// coverage / reach   the capture ATTEMPT's rect — where this grab can catch
/// ignores_guard      true; the shield is not the answer to a grab
/// ```
///
/// A grab deals NO DAMAGE, and `max_damage` is what this move does on contact.
/// What a capture is really worth is that the opponent is HELD — a fact the
/// generic option scorer has no term for and should not grow one for: "how
/// valuable is a hold" is platform-fighter policy (the throw it sets up, the
/// escape risk, the percent, the stage position), and this is a scorer shared by
/// every actor in every game the engine runs.  the honest number here is
/// zero, and the missing one is the fighter capability's.
fn capture_candidate(
    moveset: &ambition_combat::moveset::ActorMoveset,
    grounded: bool,
    playback: Option<&ambition_combat::moveset::MovePlayback>,
) -> Option<ambition_characters::brain::fighter::options::AttackCandidate> {
    use ambition_characters::actor::attack_gesture::AttackDir;
    use ambition_characters::brain::fighter::options::{
        AttackBinding, AttackCandidate, AttackVerb,
    };
    use ambition_characters::smash_capture::{CaptureAttemptParams, CAPTURE_ATTEMPT};

    let spec = moveset.0.move_for_directional_verb(
        ambition_entity_catalog::GRAB_VERB,
        AttackDir::Neutral,
        grounded,
    )?;
    // The attempt SUSTAINS across the Active window, so it rides `sustain_effect`
    // rather than the event list — see `author_standing_grab`.
    let attempt: CaptureAttemptParams = spec
        .windows
        .iter()
        .filter_map(|window| window.sustain_effect.as_ref())
        .find(|effect| effect.key == CAPTURE_ATTEMPT)
        .and_then(|effect| effect.params.hydrate().ok())?;
    let mut frames = spec.frame_data();
    frames.coverage = Some(ambition_entity_catalog::MoveCoverage {
        min: (
            attempt.offset.0 - attempt.half_extents.0,
            attempt.offset.1 - attempt.half_extents.1,
        ),
        max: (
            attempt.offset.0 + attempt.half_extents.0,
            attempt.offset.1 + attempt.half_extents.1,
        ),
    });
    frames.reach = attempt.offset.0 + attempt.half_extents.0;
    frames.ignores_guard = true;
    Some(AttackCandidate {
        move_id: spec.id.clone(),
        frames,
        binding: AttackBinding {
            verb: AttackVerb::Grab,
            direction: AttackDir::Neutral,
        },
        legality: legality_of(playback, ambition_entity_catalog::GRAB_VERB, &spec.id),
    })
}

/// Build a `BrainSnapshot` for an enemy actor's per-tick brain call.
/// Carries the per-frame body / target / cooldown view every brain
/// backend reads from; `crowding` is only consulted by the Smash
/// brain, but always populating it keeps the snapshot uniform across
/// state-machine variants.
#[allow(clippy::too_many_arguments)]
fn build_enemy_brain_snapshot(
    body: &super::super::actor_clusters::ActorClusterQueryDataReadOnlyItem<'_, '_>,
    target_pos: ae::Vec2,
    target_alive: bool,
    crowding: Option<ambition_characters::brain::smash::CrowdingSignal>,
    dt: f32,
    sim_time: f32,
    gravity_dir: ae::Vec2,
    // FB4b §13.2: the body's own moveset, or `None` for a body that has none.
    moveset: Option<&ambition_combat::moveset::ActorMoveset>,
    // Which brain this body carries, so the kit is built only for one that reads
    // it. See `attack_kit_of`.
    brain: Option<&ambition_characters::brain::Brain>,
    // The move that currently owns this body, so the attack kit can say
    // which candidates could be STARTED this tick. See `attack_kit_of`.
    playback: Option<&ambition_combat::moveset::MovePlayback>,
    // What is TRUE of this body's locomotion, published by the movement
    // kernel — see `turns_at_walls` below for why this replaced a tuning read.
    motion_facts: &ambition_platformer2d_core::BodyMotionFacts,
    // The capture relationship, resolved by the caller — which holds the capture
    // query. Threaded rather than looked up so the brain layer keeps its
    // property of reading no ECS. LAST on purpose, and a STRUCT for the same
    // reason: inserting a term mid-list silently shifted two positional
    // arguments into the wrong slots and the compiler reported it as a type
    // error three parameters away.
    capture: ambition_combat::capture::systems::CaptureFacts,
) -> ambition_characters::brain::BrainSnapshot {
    ambition_characters::brain::BrainSnapshot {
        actor_pos: body.kin.pos,
        actor_vel: body.kin.vel,
        actor_facing: body.kin.facing,
        control_down: gravity_dir,
        movement_frame_mode: ae::InputFrameMode::DEFAULT_MOVEMENT,
        aim_frame_mode: ae::InputFrameMode::DEFAULT_AIM,
        actor_on_ground: body.ground.on_ground,
        // Semantic side-contact FACT from the shared movement kernel. The brain
        // decides whether it means "turn around"; integration never mutates
        // facing merely because a wall exists.
        side_contact_normal: body
            .wall
            .on_wall
            .then_some(body.wall.wall_normal_x.signum()),
        // the LOGIC is unchanged and is not a detail: a wall means "turn
        // around" to a walker and means "keep going" to a body whose entire
        // locomotion is walls.
        turns_at_walls: body.config.brain_profile.turns_at_walls && !motion_facts.adhesive_crawling,
        // FB4b §13.2: THE ATTACK KIT, from the body's real moveset. The fighter
        // brain scores real moves with real frame data and cannot reach a
        // moveset itself, so this is body-derived truth arriving through the
        // world-in port — exactly like `actor_aerial`.
        //
        // Built every tick like every other snapshot field.
        attack_kit: attack_kit_of(moveset, body.ground.on_ground, brain, playback),
        // WHICH BODY THIS IS, so a published decision fact can name its
        // subject. The brain cannot know — a snapshot is body state and identity
        // is the host's to assign — so it arrives through the world-in port like
        // the kit above. `config.id` is the id the rest of the actor system
        // already names this body by (targets, crowding, slot requests), so an
        // explanation joins against the same identity everything else uses.
        subject: Some(body.config.id.clone()),
        // The brain steers 2D `velocity_target` whenever the body is in FLIGHT — a
        // pure free-mover (gravity_scale == 0) OR a grounded-base hybrid that has
        // toggled flight on (`flight.fly_enabled`). Without the `fly_enabled` half a
        // hybrid that takes off keeps perceiving itself grounded and re-toggles the
        // fly intent every tick (flip-flop) instead of sustaining flight. Matches the
        // integrator's flight-limb predicate (`fly_enabled && abilities.fly`).
        actor_aerial: body.surface.gravity_scale <= 0.001 || body.flight.fly_enabled,
        alive: body.health.alive(),
        captured: capture.captured,
        captured_for: capture.captured_for,
        holding_captive: capture.holding_captive,
        pummels_landed: capture.pummels_landed,
        target_pos,
        // Resolved from the target entity's body-alive state by the caller.
        target_alive,
        // Own health fraction — the Smash brain watches it drop to trigger a regroup
        // (back off + reset after taking a beating).
        health_fraction: {
            let max = body.health.max().max(1) as f32;
            (body.health.current() as f32 / max).clamp(0.0, 1.0)
        },
        // Real, accumulating sim-time (scaled by bullet-time / pause) — NOT a
        // hardcoded 0.0. The Smash brain's reaction latency (`obs_history`
        // lookback by `reaction_delay_s`) only functions when this advances, so
        // threading it is what makes the difficulty knob live in-engine.
        sim_time,
        dt,
        // ⛔⛔ THE THROTTLE SCALE, WHICH FOR A FLYING BODY IS ITS FLIGHT SPEED —
        // this field's own doc says so (*"a boss's flight speed for a body that
        // flies"*) and this site handed it the RUN speed regardless. The flight
        // limb normalises a commanded velocity by `flight_terminal_speed`, so a
        // human possessing a body whose `chase_speed` exceeds its `max_run_speed`
        // got `max_run_speed / flight_speed` of the stick and could not reach the
        // body's own top speed (D117). ⚠ latent on the shipped cast — no flyer
        // authors `chase_speed` — which is why it survived: a defect the content
        // cannot currently express.
        max_run_speed: if body.flight.fly_enabled {
            body.config.tuning.flight_speed()
        } else {
            body.config.tuning.max_run_speed
        },
        // THE MOVEMENT LAW THIS BODY PLAYS UNDER, for the brains that
        // predict rather than steer. The line above takes one number out of the
        // same tuning as a throttle scale; a rollout has to step the body
        // forward, so it needs the law and not one field of it.
        //
        // `body_tuning` is the same projection the rich integration path takes, so the
        // predictor and the integrator read one source — which is the whole point.
        movement_tuning: Some(
            body.config
                .tuning
                .movement
                .body_tuning(body.config.tuning.max_run_speed),
        ),
        // THE VERBS THAT LAW APPLIES TO, from the body's own ability
        // cluster — the same component the movement kernel reads. A rollout that
        // asks whether a fall is recoverable has to drive the kernel, and the
        // kernel gates every air jump, wall grab and glide on this.
        abilities: Some(body.abilities.abilities),
        attack_cooldown_remaining: body.attack.cooldown,
        attack_windup_remaining: body.attack.windup_remaining(),
        attack_active_remaining: body.attack.active_remaining(),
        attack_recover_remaining: 0.0,
        stun_remaining: 0.0,
        // BossPattern-only inputs — inert for actor bodies.
        boss_encounter_phase: None,
        world_size: ae::Vec2::ZERO,
        front_wall_clearance: None,
        player_input: None,
        crowding,
        terrain: None,
        air_jumps_remaining: body.jump.air_jumps_available,
    }
}

/// Keep the actor's `ActorIdentity` read-model in step with its cluster.
///
///  that is the change-amplification answer stated as code: adding a reaction timer to
/// `BodyCombat` now requires no edit here, and none in the boss road either.
///
/// identity is rebuilt only when it actually differs. This runs per actor per
/// frame, and an unconditional rebuild is a string clone plus a spurious
/// change-detection tick for every actor in the room.
pub fn sync_actor_components_from_cluster(
    em: &super::super::actor_clusters::ActorMut<'_>,
    identity: &mut ActorIdentity,
) {
    if identity.id != em.config.id
        || identity.name != em.config.name
        || identity.sprite_override_npc_name != em.config.sprite_override_npc_name
    {
        *identity = ActorIdentity::new(em.config.id.clone(), em.config.name.clone())
            .with_sprite_override(em.config.sprite_override_npc_name.clone());
    }
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
            &ambition_platformer2d_core::BodyKinematics,
            &ambition_combat::actor_tuning::ActorConfig,
            &ambition_characters::actor::BodyCombat,
            &ActorInteraction,
            &ActorDisposition,
            &ambition_characters::actor::BodyHealth,
        ),
        With<FeatureSimEntity>,
    >,
    mut vfx: MessageWriter<ambition_vfx::vfx::VfxMessage>,
    room_set: Option<
        ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
            ambition_platformer2d_world::rooms::RoomSet,
        >,
    >,
    // App-local authored voice. Required so a mis-composed production App
    // cannot silently erase provider-authored dialogue.
    character_catalog: Res<ambition_characters::actor::character_catalog::CharacterCatalog>,
    // The prepared cast, when this composition registered one. OPTIONAL because
    // a composition with no registered characters is the ordinary case — but a
    // registered-only character has no catalog row, so this is the only place
    // its voice can come from.
    prepared_cast: Option<Res<ambition_characters::prepared::PreparedCharacterRegistry>>,
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
    for (kin, config, combat, interaction, disposition, health) in &npcs {
        // Structural tangibility gate: a dead body does not
        // present — an intangible corpse says nothing, ambient or otherwise.
        if disposition.is_hostile() || combat.hit_flash > 0.0 || !health.alive() {
            continue;
        }
        let rotation = *state.rotations.get(&config.id).unwrap_or(&0);
        let Some(line) = super::super::npcs::npc_ambient_bark_line(
            catalog,
            prepared_cast.as_deref(),
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

#[cfg(test)]
mod body_combat_rebuild_contract {
    /// NOTHING IN `BodyCombat` IS WRITTEN BY THE PER-FRAME ACTOR SYNC.
    ///
    /// `landing_lag_timer` joined `BodyCombat` later, never joined the list, and was erased one
    /// frame after the moveset runtime set it.
    ///
    /// Keeping the destructure keeps the claim honest: if a future field is added to this
    /// component AND written from the cluster, this stops compiling and somebody has to say why
    /// the read-model is growing a second authority again.
    #[allow(dead_code)]
    fn the_per_frame_sync_writes_none_of_these(combat: &ambition_characters::actor::BodyCombat) {
        let ambition_characters::actor::BodyCombat {
            // ── Reaction history the body owns. Never disturbed by the sync.
            damage_invuln_timer: _,
            hit_flash: _,
            hitstun_timer: _,
            recoil_lock_timer: _,
            hitstop_timer: _,
            asdi_owed: _,
            landing_lag_timer: _,
            // ── Republished every tick from the live move by
            // `project_move_defense_windows`, which is the ONE writer. This
            // sync must never touch it: a second writer would fight the
            // projection for the same field on alternating frames.
            armored: _,
            // ── Authored at construction (AC3.1.D), not re-derived per frame.
            training_dummy: _,
        } = combat;
    }
}
