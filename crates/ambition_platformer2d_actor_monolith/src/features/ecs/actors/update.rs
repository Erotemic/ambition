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

/// **Run the brain tick with the log lent to this thread**, when there is one.
///
/// A brain publishes through `ambition_causal::record`, which writes to a
/// THREAD-LOCAL sink, and Bevy runs systems across worker threads — so those
/// facts were landing in nothing and being counted by `facts_lost_offthread()`.
///
/// ⚠ **the alternative was threading a recorder into `tick_with_actions`**, and
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

pub fn tick_actor_brains(
    // ⛔ **THESE SEVEN USED TO RIDE IN ONE TUPLE**, because this system sat at
    // Bevy's 16-parameter ceiling and packing was the only way under it. Packing
    // is not a contract: a tuple says these things arrive together, and nothing
    // about why. They are named again because the ceiling pressure is gone —
    // deleting the combat slot board freed two parameters and adopting
    // `CollisionWorld` freed two more — and because three of the seven turned out
    // to be one concept (`PerceivedWorld`) rather than three neighbours.
    world_time: Res<WorldTime>,
    // Accumulating sim-time, for the brain's reaction-latency lookback.
    sim_clock: Res<crate::features::GameplayElapsed>,
    // Peers, projectiles and hostility: what a body can perceive this tick.
    perceived: crate::features::ecs::perception::PerceivedWorld,
    // **The log, lent to whichever worker thread this tick lands on.**
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
    // **The collision read-API, not its three ingredients.** This system used to
    // carry the room, the moving-platform set and the feature overlay as separate
    // parameters and compose them itself — the same three lines eight production
    // systems each wrote out. `CollisionWorld` is the seam that already owned that
    // composition; the brain tick simply had never adopted it.
    collision: ambition_platformer2d_world::collision::CollisionWorld,
    // Neighbor index handed to the movement phase (surface-walker steering).
    mut steering: ResMut<ActorSteering>,
    // **Liveness of the bodies the actor query cannot see.** A fighter's foe is
    // often a controlled body, and a brain must perceive that its foe has died;
    // controlled bodies carry no actor cluster, so they are absent from `actors`
    // below. Keyed per entity, so this is not "the player" — it is every body in
    // this class, however many a session has. `BodyHealth` is the liveness
    // authority (NOT `BodyCombat.alive`, an actor-cluster mirror never synced for
    // a controlled body), consistent with `select_actor_targets`.
    //
    // ⭐ this used to read POSITION and `PlayerSlot` as well, to anchor a combat
    // slot board on "the primary player". That board is gone (see
    // `ambition_combat::crowd`), and with it the last reason generic brain
    // ticking needed to know where a player was standing.
    player_query: Query<
        (
            bevy::prelude::Entity,
            &ambition_characters::actor::BodyHealth,
        ),
        bevy::prelude::With<crate::actor::PlayerEntity>,
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
            // ⚠ **a possessed body is MATCHED here and deliberately not decided
            // for.** It carries `Brain::Player(slot)` (transferred by
            // `crate::abilities::traversal::possession`), and since 2026-08-14
            // `tick_controlled_brains` owns its intent frame a phase earlier —
            // this loop still runs the facts a body in a world has (reaction
            // decay, target liveness, crowd, disposition) and stops before the
            // decision. See the skip in the loop body for what that deletes.
            (
                Option<super::super::actor_clusters::ActorClusterQueryData>,
                // The body's per-tick resolved frame (ADR 0024): published by
                // the frame resolution phase before this brain tick, and the
                // SAME value `integrate_sim_bodies` moves the body under. The
                // brain interprets controller input and perceives "down"
                // through it — never through a private gravity lookup.
                Option<&ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame>,
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
                // FB4b §13.2: the body's own moveset, so the brain snapshot can
                // carry the ATTACK KIT. The fighter brain scores real moves with
                // real frame data and cannot reach a moveset itself —
                // `ambition_combat` depends on `ambition_characters`, not the
                // reverse — so this is the world-in port doing what it is for.
                // `Option` because a body with no moveset (a peaceful NPC, a
                // prop) has no kit, and an empty kit is the honest answer.
                Option<&crate::combat::moveset::ActorMoveset>,
                // **WHAT IS TRUE OF THIS BODY'S LOCOMOTION**, published once per
                // tick by the movement kernel. The brain snapshot's
                // `turns_at_walls` reads it instead of the spawn-time
                // `tuning.surface_walker` flag ADR 0024 §8 forbids at runtime.
                // `Option` because a body without the movement clusters has no
                // facts, and "not crawling" is the honest default for one.
                Option<&ambition_platformer2d_core::BodyMotionFacts>,
                // **THE BODY'S MOTION MODEL**, for the ONE burst-maneuver rule
                // (`resolve_burst_maneuver`): dodge-vs-dash availability lives
                // in the model's `AxisManeuverState` (air-dodge window, endlag)
                // as well as in the dodge/dash clusters. A driver that decides
                // its maneuver from CAPABILITIES instead names one thing and the
                // kernel performs another — see `SelfView::burst`.
                //
                // ⛔ **NOT `Option`, per ADR 0024 §1: absence is never a movement
                // policy.** It was optional until 2026-08-16, and the `None` arm
                // in `perception_body_for` read a missing component as
                // `AxisSweptMotion::default()` — the ADR's exact prohibition
                // ("no outer query may interpret a missing component as
                // axis-swept"), reached with the DEFAULT air-dodge window rather
                // than the body's. Requiring it costs nothing: the integration
                // query below (`integrate_actor_bodies`) already takes
                // `&mut MotionModel` non-optionally over the same archetype, so a
                // body without one is not integrated at all and has no locomotion
                // for a brain to reason about.
                &crate::features::MotionModel,
            ),
            // **IS THIS BODY IN A FIGHT?** Read for the stand-down rule below,
            // which pacifies a hostile actor that holds no combat target, and
            // for the read-model rebuild that would otherwise drop a
            // combatant's attack windup every frame. A ruleset declares its
            // fighters combatants; that is not a fact AI targeting is allowed to
            // revoke.
            //
            // ⛔ **this was `Has<MatchSeat>`, and a seat is not participation.**
            // An eliminated fighter keeps its seat — the body stays standing
            // until a ruleset removes it — so it went on holding attack state and
            // a place on the anti-clump board with no stocks left. It was also
            // the SECOND proxy for a fact `apply_actor_hit` was reading off a
            // third component; one authority answers all of them now.
            bevy::prelude::Has<crate::combat::components::ActiveCombatant>,
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
            Without<ambition_boss_encounter::BossConfig>,
            // **A DORMANT ACTOR DOES NOT DECIDE.** Only the brain sleeps: the
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
    let dt = world_time.sim_dt();
    // Accumulating sim-time for brain perception (reaction-latency lookback).
    let sim_now = sim_clock.0;
    // ⚠ **no room means no tick, exactly as before.** The room used to arrive as a
    // `Single`, and a `Single` that matches nothing makes Bevy skip the whole
    // system; `CollisionWorld` takes it optionally, so the skip has to be written
    // down. This is the one condition that legitimately ends this system early —
    // there is no geometry for a body to decide against — and it is not a
    // condition about whether a player exists.
    let Some(feature_world) = collision.solids() else {
        return;
    };
    // The live hostility table for every brain's world-out view this frame (§A7),
    // all-peaceful when a fixture registers none.
    let relations = perceived.relations();
    // ⛔⛔ **A SLOT BOARD ANCHORED ON "THE PRIMARY PLAYER" USED TO STAND HERE,
    // AND IT DROVE NOTHING.**
    //
    // It chose an anchor position from `PrimaryPlayer`, or the lowest
    // `PlayerSlot` when a build had no primary, and handed it to `assign_slots`
    // to arrange a crowd into numbered approach slots around it. No production
    // reader ever consumed the assignment — the per-actor position it produced
    // had been discarded since before the monolith split — so the single
    // largest player-centric assumption in generic actor simulation existed to
    // feed a mechanism with no consumer, and was rewound as rollback state on
    // top of that. Deleted with the board (`ambition_combat::crowd`).
    //
    // ⚠ **a world with no controlled body is ordinary**, and the shape that
    // proves it is the absence of an early return here. When the anchor was
    // live this read `let Some(player_pos) = ... else { return; }`, so a session
    // that declared no home avatar ticked NO actor brains at all — every actor
    // everywhere, not merely the ones near a player. Two seated fighters stood
    // on a platform with correct factions, correct targets and zero velocity,
    // and read as a seating bug for hours (2026-08-06). Nothing in this system
    // may become conditional on a player existing again.

    // ── OBSERVATION ──────────────────────────────────────────────────────
    // Look at every body once, then derive. `crowd_observation` owns the
    // derivations so the boundary between "look at the world" and "decide what
    // this body does" is a type rather than a place in a long function.
    let mut observation = super::crowd_observation::CrowdObservation::default();
    for (entity, health) in &player_query {
        observation.note_controlled_liveness(entity, health.current() > 0);
    }
    for (
        entity,
        _,
        _,
        disposition,
        _,
        target,
        _,
        _,
        _,
        _,
        (clusters, _, faction, _, _, _, _, _, _),
        in_a_fight,
    ) in &actors
    {
        // ⚠ **the two arms are the two ways to be in a fight.** Social hostility
        // is how an AI body joins one; `ActiveCombatant` is how a ruleset puts a
        // body in. A human-driven fighter is the second without ever being the
        // first — it holds no AI target, so asking only the disposition left it
        // off the picture it is standing in the middle of.
        let fighting =
            crate::combat::components::CombatStanding::of(*disposition, in_a_fight).takes_damage();
        observation.note_actor(
            entity,
            clusters.as_ref().is_some_and(|c| c.health.alive()),
            clusters
                .as_ref()
                .map(|c| super::crowd_observation::ObservedBody {
                    id: c.config.id.as_str(),
                    pos: c.kin.pos,
                    kind: c.config.tuning.crowd_kind(),
                    faction: faction.copied(),
                    foe: target.entity,
                }),
            fighting,
        );
    }
    let crowd = observation.finish();
    // The neighbour index is handed to the movement phase (surface-walker
    // anti-clump steering) rather than consumed here — the one piece of the
    // observation that leaves this system.
    steering.neighbor_by_id = crowd.neighbor_index().clone();

    // Pass 2: tick each actor's brain into its `ActorControl`. The slot-board
    // holding fallback that steers unassigned actors is folded into the brain
    // snapshot (crowding); movement integration is a separate phase.
    for (
        this_actor_entity,
        // aabb / identity / mounted belong to the movement + read-model phases;
        // the query still fetches them (one actor query shape) but the brain
        // phase reads only its intent inputs.
        _aabb,
        _identity,
        mut disposition,
        mut combat,
        target,
        mut brain,
        mut control,
        action_set,
        _mounted,
        (
            clusters,
            resolved_frame,
            faction,
            aggression,
            mut perception_memory,
            perception,
            moveset,
            motion_facts,
            motion_model,
        ),
        in_a_fight,
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
            Some(e) => crowd.is_alive(e),
            None => false,
        };
        // Disposition is DERIVED from having a combat target: an aggressive actor
        // with NO target stands down to Peaceful — it stops attacking empty air,
        // relabels as peaceful, and is re-provokable (strike it past the threshold)
        // again — but KEEPS its aggression mode, so it re-acquires and re-engages the
        // instant a foe reappears (retreat → escape → peaceful; reacquire →
        // fighting). A `Hostile` enemy keeps its live foe as its target, so it
        // never spuriously stands down. Relativity-neutral (any fighter, any
        // faction). This REPLACES the former hard pacify-to-passive, which dead-ended
        // a duel winner (couldn't be talked to or re-provoked, and mislabeled it).
        // ⛔ **UNLESS A MATCH SAYS OTHERWISE.** A seated fighter is a combatant
        // because two people decided it is, and that is not a fact about whether
        // it currently holds a target.
        //
        // ⭐ **and it is no longer about DAMAGE**, which is the half that has
        // moved: `apply_actor_hit` asks `CombatStanding`, so a fighter that
        // stood down is still damageable.
        //
        // ⚠ **nor is it about the read-model any more** (AC3, 2026-08-13). This
        // used to say that `BodyCombat::peaceful` dropped a stood-down fighter's
        // attack windup and swing timers every frame — true when written, and
        // now describing machinery that no longer exists: those fields were dead
        // and are deleted, and the per-frame rebuild that dropped them is gone.
        // What survives the correction is the anti-clump slot board, which stops
        // seeing a body that leaves the fight.
        //
        // ⚠ the original symptom is worth keeping written down, because it is
        // what the two questions sharing one field cost: a peaceful body takes
        // no health damage at all, so a fighter that stood down could not be hit,
        // could not be knocked out, and could not lose a stock.
        //
        // ⚠ **measured 2026-08-07, and it is worse than the test that found it.**
        // Two `Brain::Player` fighters hold no combat target — targeting hunts
        // live foes for a BRAIN — so in a human-versus-human match BOTH sides
        // stood down and neither could damage the other at all. The stage-kill
        // test only caught the blast-zone corner of it: seat 1 launched at
        // 2400px/s fell to y=5771 and kept falling, the gate writing a lethal
        // hit every tick that nothing would resolve.
        //
        // ⚠ it was masked by a bug. `provoke_actor_in_place` used to seize
        // `Brain::Player` on the first hit, which turned a human's fighter into
        // an AI body that acquired a target — so the fighters were hostile by
        // accident. Fixing that seizure (`d657a0e22`) is what exposed this.
        if disposition.is_hostile() && target.entity.is_none() && !in_a_fight {
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
            // passes via tuning (`is_hostile` / `body_contact_damage`); the
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
                // ⭐⭐ **A PARTICIPANT'S BODY IS NOT THIS SYSTEM'S TO DECIDE FOR.**
                // A possessed actor carries `Brain::Player(slot)`, and its
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
                // ⚠ everything ABOVE this point still runs for a possessed body:
                // reaction-timer decay, target liveness, disposition standdown and
                // the crowd observation are facts about a body in a world, not
                // decisions a driver makes.
                if brain.as_deref().is_some_and(|b| b.player_slot().is_some()) {
                    continue;
                }
                let brain_frame = if let Some(brain_ref) = brain.as_deref_mut() {
                    let crowding = crowd.crowding(&em.config.id);
                    let mut snapshot = build_enemy_brain_snapshot(
                        &em,
                        target_pos,
                        target_alive,
                        crowding,
                        dt,
                        sim_now,
                        enemy_gravity_dir,
                        moveset,
                        Some(brain_ref),
                        // A body with no movement clusters publishes no
                        // locomotion facts, and "none of them true" is the
                        // honest reading of that.
                        &motion_facts.copied().unwrap_or_default(),
                    );
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
                    // snapshot minus SELF.
                    let view_peers = perceived.peers_seen_by(this_actor_entity);
                    // Self's own move phase / i-frames come from the SAME per-tick
                    // snapshot every peer's do — one derivation (`body_phase`), so a
                    // body cannot read itself more precisely than its opponent reads it.
                    let self_peer = perceived.peer(this_actor_entity);
                    // **WHAT THIS BODY'S BURST BUTTON WOULD DO IF PRESSED NOW.**
                    // The kernel's own rule (`resolve_burst_maneuver`), asked one
                    // phase early — so the brain names the maneuver the body will
                    // actually perform instead of re-deriving the precedence.
                    //
                    // ⚠ a body on a NON-axis model has no `AxisManeuverState` to
                    // read, and the default reads as "no window open, no endlag".
                    // ⛔ that is a reading of a PRESENT model that is not
                    // axis-swept — a crawler genuinely has no air-dodge window —
                    // and NOT a reading of an absent component, which ADR 0024 §1
                    // forbids and the query above now makes unrepresentable.
                    let world_view = super::super::perception::build_world_view(
                        &super::super::perception::perception_body_for(
                            &em,
                            self_faction,
                            enemy_gravity_dir,
                            action_set,
                            self_peer,
                            aggression,
                            motion_model,
                        ),
                        &view_peers,
                        perceived.projectiles(),
                        &[],
                        &feature_world,
                        relations,
                        viewport_half,
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
                                snapshot.target_pos = em.kin.pos;
                                snapshot.target_alive = false;
                            }
                        }
                    }
                    let mut bf = ambition_characters::actor::control::ActorControlFrame::neutral();
                    let peaceful = ambition_characters::brain::ActionSet::peaceful();
                    let actions = action_set.unwrap_or(&peaceful);
                    with_causal_sink(&mut causal, || {
                        brain_ref.tick_with_actions(actions, &snapshot, Some(&world_view), &mut bf)
                    });
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
    // Whose cues this body emits (A13). Read-only, looked up by entity so it does
    // not have to ride the already-12-wide cluster tuple.
    presentation_source: Option<&ambition_sfx::PresentationSourceId>,
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
    feel: crate::time::feel::Platformer2dFeelTuningMonolith,
    // **This body's own movement feel, when its character authored one.**
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
    // ⭐ **HITSTOP: this body freezes on its own, and so does the one it hit.**
    // The resolver arms `combat.hitstop_timer` on the victim AND the attacker,
    // because a landed hit is one event, and `integrate_body` below spends it
    // (`let sim_dt = if combat.is_in_hitlag() { 0.0 } else { dt }`) exactly as
    // the home road does. One named predicate, asked on both roads.
    //
    // ⛔⛔ **this comment used to say the opposite — that an actor's sim dt is
    // NOT frozen, because "per-victim freezes in AI-vs-AI fights made duels
    // degenerate" — and it is SUPERSEDED, not merely out of date.** That
    // measurement predates D155, on a build where every authored launch
    // direction was inverted and a tumbling launch resolved as a landing, so
    // nobody was ever knocked anywhere: a feel verdict inherits the build it
    // was formed on. Jon ruled on it 2026-08-17 after seeing the fixed build —
    // *"hitlag is a combat/body semantic, not something that should depend on
    // whether a body happens to occupy the primary local-control road"* — and
    // the per-road distinction WAS the defect the ruling removes.
    //
    // ⛔ so if hitlag ever feels too sticky, tune its DURATION or SHAPE.
    // **Restoring a controlled-body/actor asymmetry here is forbidden**, and a
    // comment recommending it is how the last one nearly got restored.
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
        authored_tuning,
        combat,
    );
    if was_dead && em.health.alive() {
        combat.hit_flash = 0.24;
    }
    let shark_crashed = shark_charge_crashed(em, is_mounted, shark_charge_vec, previous_pos);
    let mut frame = frame;
    if shark_crashed {
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
    // Gated on ALIVE, because the gate is a position test and re-fires every
    // tick the body is past the margin. A corpse that has not been cleaned up
    // yet is still out there, so without this every dead body outside the world
    // writes a lethal hit once per frame, forever — the damage pass ignores
    // them all (it refuses hits on the dead), which makes it a silent per-frame
    // cost rather than a visible bug, which is worse.
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
    // Movement presentation for the body's frame: jump/dash/dodge/wall-jump/ledge/
    // shield/blink SFX+VFX + landing dust, through the SAME body-generic emitter
    // the player tick uses — so an AI fighter that dashes or wall-jumps produces
    // the same dust + SFX the player does, not the old blink-only actor branch
    // with its hand-copied second blink emit (fable review §A8). Fly-toggle +
    // shield are resolved INSIDE `em.update`'s shared pipeline.
    // The kernel NAMED what it did; hand that to the instrument beside the FX
    // that already read it. One publish covers every kernel velocity writer,
    // which is why this is not sixty-eight instrumentations.
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
    // A13: whose cues each body emits, looked up by entity. A separate read-only
    // query rather than another member of the cluster tuple, which is already at
    // twelve.
    body_sources: Query<&ambition_sfx::BodyPresentationSource>,
    world_time: Res<WorldTime>,
    world: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
        ambition_platformer2d_core::RoomGeometry,
    >,
    platform_set: Res<ambition_platformer2d_world::collision::MovingPlatformSet>,
    feel_tuning: Res<crate::time::feel::Platformer2dFeelTuningMonolith>,
    overlay: Res<FeatureEcsWorldOverlay>,
    steering: Res<ActorSteering>,
    active_tuning: Res<ambition_platformer2d_core::ActiveMovementTuning>,
    user_settings: Option<Res<ambition_persistence::settings::UserSettings>>,
    mut sfx: ambition_sfx::SfxWriter,
    mut vfx: MessageWriter<ambition_vfx::vfx::VfxMessage>,
    mut hit_events: MessageWriter<HitEvent>,
    // The kernel's operation list, for the causal instrument. `Option` so a
    // composition with no inspector registers nothing and publishes nothing —
    // the rule the damage path already documents.
    #[cfg(feature = "causal")] mut movement_ops: Option<
        MessageWriter<crate::causal::BodyMovementOps>,
    >,
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
            &ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame,
            &mut ambition_platformer2d_core::BodyMotionFacts,
            Option<super::super::actor_clusters::ActorClusterQueryData>,
            // The body's live move, if any — its authored per-window motion
            // lock scales the steering intent inside `integrate_actor_body`.
            Option<&crate::combat::moveset::MovePlayback>,
            // **The body's own FEEL, if its character authored one.**
            //
            // ⚠ this component was GRANTED to every seated fighter and read by
            // nobody on this path: `presentation.rs` inserts it precisely so a
            // seated fighter and a worn player move alike, and the only consumer
            // in the repository was the PLAYER loop below. So a character's
            // authored tuning reached it when worn and vanished when seated —
            // the exact asymmetry the seating comment says it exists to prevent
            // (found 2026-07-31 by authoring `slash_recoil: 0` on the smash
            // duelists and measuring no change at all).
            Option<&ambition_platformer2d_core::AuthoredMovementTuning>,
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
            Without<ambition_boss_encounter::BossConfig>,
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
            // ⛔ **the body's own reason set, because a hazard TILE is damage.**
            // A player who cannot be hurt — a super form, a transformation beat,
            // a scripted grant — must not be reset to spawn by walking over
            // spikes. `Option` because a home body without health is a valid
            // scratch/test body and there is nothing to ask.
            Option<&ambition_characters::actor::BodyHealth>,
            &ambition_characters::brain::ActorControl,
            &mut CenteredAabb,
            &mut crate::avatar::PlayerBodyFrameOutput,
            &mut MotionModel,
            &ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame,
            &mut ambition_platformer2d_core::BodyMotionFacts,
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
        With<crate::actor::PlayerEntity>,
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
        mut motion_model,
        resolved_frame,
        mut motion_facts,
        clusters,
        playback,
        authored_tuning,
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
            mounted.is_some(),
            &feature_world,
            combat_tuning,
            &steering,
            resolved_frame.get(),
            playback.map_or(1.0, |pb| pb.spec.motion_scale_at(pb.t)),
            dt,
            *feel_tuning,
            authored_tuning.map(|t| t.0),
            &mut sfx,
            &mut vfx,
            &mut hit_events,
            #[cfg(feature = "causal")]
            movement_ops.as_mut(),
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
        mut cluster_item,
        combat,
        health,
        control,
        mut hurtbox,
        mut frame_out,
        mut motion_model,
        resolved_frame,
        mut motion_facts,
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
        // ⭐⭐ **the SAME composited world the actors integrate against**, built
        // once per frame above rather than once per body here. This road used to
        // call `world_with_sandbox_solids` itself, from identical inputs — the
        // authored world, the platform set, the overlay — so every home body
        // CLONED the whole block list to rebuild a value that already existed a
        // few lines up. Two composite sites is also two places for the moving
        // platforms, gate solids, water and portal carves a body collides with to
        // drift apart (D117).
        let riding_up = crate::avatar::integrate_home_body(
            control.0,
            &feature_world,
            &mut clusters,
            combat,
            health.map_or_else(ambition_characters::actor::Invulnerability::none, |h| {
                h.health.invulnerable
            }),
            motion_facts.evading(),
            out_of_play,
            &mut hurtbox,
            &mut frame_out,
            &mut motion_model,
            player_motion_frame,
            player_tuning,
            player_feel,
            frame_dt,
            scaled_dt,
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
/// ⭐⭐ **this phase used to mirror `BodyCombat` too, and now mirrors nothing but
/// a name** (AC3). Every combat fact it wrote turned out to be a duplicate of an
/// authority the reader could ask directly: liveness (`BodyHealth`), melee
/// (`BodyMelee`), the sandbag flag (authored, set at construction), and three
/// fields nobody read at all. So the query loses `ActorDisposition`,
/// `BodyCombat` and `Has<ActiveCombatant>` — the last of which existed ONLY to
/// choose between a peaceful and a hostile rebuild that no longer happens.
///
/// It changes no control and moves no body. Runs after `integrate_actor_bodies`.
pub fn sync_actor_read_model(
    mut actors: Query<
        (
            &mut ActorIdentity,
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
/// frame's resolved position. Body-contact is OFF for a player-controlled
/// (possessed) body — its brain is `Brain::Player`, it fights for you, and its
/// body must not harm you on contact (the same effective-allegiance rule the melee
/// strike + boss damage use).
#[allow(clippy::too_many_arguments)]
pub fn apply_actor_contact_damage(
    // CM8: contact hits no longer emit feedback here (they used to fire the
    // player-hurt payload for EVERY victim). The struck body's own victim
    // consumer emits its `HurtFeedback` now, so this system only writes the
    // `HitEvent`.
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
            &ambition_characters::actor::BodyHealth,
            &ambition_platformer2d_core::BodyMotionFacts,
            &crate::actor::BodyShieldState,
            &ambition_characters::actor::BodyCombat,
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
        let Ok((hurtbox, victim_health, facts, shield, combat)) = victims.get(target_entity) else {
            continue;
        };
        if !crate::combat::util::body_vulnerable(
            victim_health.health.invulnerable,
            facts.evading(),
            shield,
            combat,
        ) {
            continue;
        }
        if let Some(damage) = attack.hit_event(attacker, target_entity, hurtbox.aabb()) {
            // CM8: this used to emit the player-hurt payload (PLAYER_DAMAGE + red
            // burst + debris) for EVERY victim — `is_player` was bound above but
            // ignored here, so an enemy body-checking another enemy played the
            // "player got hurt" cue. The feedback now belongs to the ONE
            // victim-side reaction, which reads the VICTIM's `HurtFeedback`: the
            // player keeps its red burst, an enemy gets the plain tick. The
            // routing use of `is_player` (Player vs Actor target) stays.
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
    requests: &[(String, ae::Vec2, crate::combat::crowd::CrowdKind)],
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
    requests: &[(String, ae::Vec2, crate::combat::crowd::CrowdKind)],
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
        let aerial = *kind_a == crate::combat::crowd::CrowdKind::Aerial;
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
            if aerial && *kind_b != crate::combat::crowd::CrowdKind::Aerial {
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

/// **The attacks this body can actually throw**, as the fighter brain reads them.
///
/// One row per move in the contract, with the frame data a player who read the
/// tables would know. Declaration order, which `MovesetContract.moves` is a
/// `Vec` — so the kit is stable across ticks and across a replay, and no sort is
/// needed to make it deterministic.
pub(super) fn attack_kit_of(
    moveset: Option<&crate::combat::moveset::ActorMoveset>,
    // The body's REAL posture this tick. The kit is what it can press NOW.
    grounded: bool,
    // ⚠ **only a FIGHTER brain reads the kit**, and building it is a `Vec` of
    // owned move ids and frame data — per actor, per tick. Every other brain in
    // the game would have paid for a list nothing looks at, which is a cost
    // §13.2 explicitly said to fix by rebuilding on moveset CHANGE. It does not
    // need that yet: the cheaper answer is not to build it for a brain that
    // cannot use it, and this is the one place that knows which brain a body has.
    brain: Option<&ambition_characters::brain::Brain>,
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

    // **ENUMERATE THE PRESSES, ASK WHAT EACH ONE REACHES.**
    //
    // ⚠ this listed `moveset.moves` — every move the body owns, whether or not
    // any input can invoke it — and the candidate carried no way to invoke the
    // one that won (GPT 5.6, 2026-07-31, finding 2). Two failures in one line: a
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
        (AttackVerb::Basic, crate::combat::moveset::ATTACK_VERB),
        (AttackVerb::Smash, crate::combat::moveset::SMASH_VERB),
        (AttackVerb::Special, crate::combat::moveset::SPECIAL_VERB),
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
            });
        }
    }
    kit
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
    // FB4b §13.2: the body's own moveset, or `None` for a body that has none.
    moveset: Option<&crate::combat::moveset::ActorMoveset>,
    // Which brain this body carries, so the kit is built only for one that reads
    // it. See `attack_kit_of`.
    brain: Option<&ambition_characters::brain::Brain>,
    // **What is TRUE of this body's locomotion**, published by the movement
    // kernel — see `turns_at_walls` below for why this replaced a tuning read.
    motion_facts: &ambition_platformer2d_core::BodyMotionFacts,
) -> ambition_characters::brain::BrainSnapshot {
    ambition_characters::brain::BrainSnapshot {
        actor_pos: em.kin.pos,
        actor_vel: em.kin.vel,
        actor_facing: em.kin.facing,
        control_down: gravity_dir,
        movement_frame_mode: ae::InputFrameMode::DEFAULT_MOVEMENT,
        aim_frame_mode: ae::InputFrameMode::DEFAULT_AIM,
        actor_on_ground: em.ground.on_ground,
        // Semantic side-contact FACT from the shared movement kernel. The brain
        // decides whether it means "turn around"; integration never mutates
        // facing merely because a wall exists.
        side_contact_normal: em.wall.on_wall.then_some(em.wall.wall_normal_x.signum()),
        // ⛔ **the second term read `em.config.tuning.surface_walker`, which ADR
        // 0024 §8 forbids at runtime** — that boolean is spawn-time SELECTION
        // (it chooses the `MotionModel` once) and is afterwards a stale copy of a
        // decision the body carries explicitly. The workspace policy checker had
        // been reporting it; nothing in the run's gate builds that crate's tests,
        // so it went unread (ledger D88).
        //
        // ⚠ the LOGIC is unchanged and is not a detail: a wall means "turn
        // around" to a walker and means "keep going" to a body whose entire
        // locomotion is walls.
        turns_at_walls: em.config.brain_profile.turns_at_walls && !motion_facts.adhesive_crawling,
        // FB4b §13.2: THE ATTACK KIT, from the body's real moveset. The fighter
        // brain scores real moves with real frame data and cannot reach a
        // moveset itself, so this is body-derived truth arriving through the
        // world-in port — exactly like `actor_aerial`.
        //
        // Built every tick like every other snapshot field. Correctness first:
        // if profiling ever complains, the fix is to rebuild it on moveset
        // CHANGE, not to let it go stale.
        attack_kit: attack_kit_of(moveset, em.ground.on_ground, brain),
        // WHICH BODY THIS IS, so a published decision fact can name its
        // subject. The brain cannot know — a snapshot is body state and identity
        // is the host's to assign — so it arrives through the world-in port like
        // the kit above. `config.id` is the id the rest of the actor system
        // already names this body by (targets, crowding, slot requests), so an
        // explanation joins against the same identity everything else uses.
        subject: Some(em.config.id.clone()),
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
        // **THE MOVEMENT LAW THIS BODY PLAYS UNDER**, for the brains that
        // predict rather than steer. The line above takes one number out of the
        // same tuning as a throttle scale; a rollout has to step the body
        // forward, so it needs the law and not one field of it.
        //
        // `body_tuning` is the same projection the rich integration path takes,
        // so the predictor and the integrator read one source — which is the
        // whole point. A shadow model that restates the engine's constants
        // predicts the wrong body the moment an archetype authors its own
        // gravity or jump, and it was silently predicting the wrong body even
        // before that.
        movement_tuning: Some(
            em.config
                .tuning
                .movement
                .body_tuning(em.config.tuning.max_run_speed),
        ),
        // **THE VERBS THAT LAW APPLIES TO**, from the body's own ability
        // cluster — the same component the movement kernel reads. A rollout that
        // asks whether a fall is recoverable has to drive the kernel, and the
        // kernel gates every air jump, wall grab and glide on this.
        abilities: Some(em.abilities.abilities),
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

/// Keep the actor's `ActorIdentity` read-model in step with its cluster.
///
/// ⭐⭐ **IT NO LONGER TOUCHES `BodyCombat` AT ALL** (AC3). What this function
/// did to that component, slice by slice: AC3.1.C deleted three fields it
/// rebuilt for no reader; AC3.2 replaced the save→rebuild→restore with in-place
/// writes; AC3.1.A and .B deleted the liveness and melee mirrors it wrote; and
/// AC3.1.D moved `training_dummy` to construction, where an AUTHORED fact
/// belongs. What is left is one string comparison.
///
/// ⇒ **that is the change-amplification answer stated as code**: adding a
/// reaction timer to `BodyCombat` now requires no edit here, and none in the
/// boss road either. It used to require a carry line in two hand-kept lists that
/// had already drifted apart.
///
/// ⚠ identity is rebuilt only when it actually differs. This runs per actor per
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
            &super::super::actor_clusters::BodyKinematics,
            &super::super::actor_clusters::ActorConfig,
            &ambition_characters::actor::BodyCombat,
            &ActorInteraction,
            &ActorDisposition,
            &ambition_characters::actor::BodyHealth,
        ),
        With<FeatureSimEntity>,
    >,
    mut vfx: MessageWriter<ambition_vfx::vfx::VfxMessage>,
    room_set: Option<
        ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<crate::rooms::RoomSet>,
    >,
    // App-local authored voice. Required so a mis-composed production App
    // cannot silently erase provider-authored dialogue.
    character_catalog: Res<ambition_characters::actor::character_catalog::CharacterCatalog>,
    // The prepared cast, when this composition registered one. OPTIONAL because
    // a composition with no registered characters is the ordinary case — but a
    // registered-only character has no catalog row, so this is the only place
    // its voice can come from.
    prepared_cast: Option<Res<crate::character_runtime::PreparedCharacterRegistry>>,
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
        // Structural tangibility gate (Jon 2026-07-22): a dead body does not
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
    /// **NOTHING IN `BodyCombat` IS WRITTEN BY THE PER-FRAME ACTOR SYNC.**
    ///
    /// ⭐⭐ **this destructure found D108 and then outlived the defect.** The sync
    /// used to REPLACE `*combat` wholesale and restore a hand-written list of
    /// timers — a list a comment described as *"the SAME fields the player
    /// carries"*, which nothing enforced. `landing_lag_timer` joined
    /// `BodyCombat` later, never joined the list, and was erased one frame after
    /// the moveset runtime set it.
    ///
    /// ⇒ AC3 removed every field the sync had a reason to write. Keeping the
    /// destructure keeps the claim honest: if a future field is added to this
    /// component AND written from the cluster, this stops compiling and somebody
    /// has to say why the read-model is growing a second authority again.
    #[allow(dead_code)]
    fn the_per_frame_sync_writes_none_of_these(combat: &ambition_characters::actor::BodyCombat) {
        let ambition_characters::actor::BodyCombat {
            // ── Reaction history the body owns. Never disturbed by the sync.
            damage_invuln_timer: _,
            hit_flash: _,
            hitstun_timer: _,
            recoil_lock_timer: _,
            hitstop_timer: _,
            landing_lag_timer: _,
            // ── Authored at construction (AC3.1.D), not re-derived per frame.
            training_dummy: _,
        } = combat;
    }
}
