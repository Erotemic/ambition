//! Rollback declaration owned by the actor runtime.
//!
//! The actor runtime names only state defined in this crate. The host supplies GGRS machinery
//! through [`RollbackRegistrar`].

use ambition_platformer2d_core::snapshot::checksum_bytes;
use ambition_platformer2d_core::snapshot::RollbackRegistrar;

const OWNER: &str = env!("CARGO_PKG_NAME");

/// Register everything the actors domain needs rewound.
pub fn register_rollback_state<R>(registrar: &mut R)
where
    R: RollbackRegistrar,
{
    // Causal readers keep non-rollback `Local` cursors, so their message
    // channels must be cleared rather than replay rows from an abandoned future.
    #[cfg(feature = "causal")]
    {
        registrar.clear_message_on_rollback::<crate::causal::BodyMovementOps>(
            OWNER,
            "message.causal_body_movement_ops",
        );
        registrar.clear_message_on_rollback::<ambition_damage::BodyHitResolved>(
            OWNER,
            "message.causal_body_hit_resolved",
        );
        registrar.clear_message_on_rollback::<ambition_damage::BodyReactionApplied>(
            OWNER,
            "message.causal_body_reaction_applied",
        );
    }

    registrar.require_rollback::<crate::features::transform_beat::TransformBeat>(
        OWNER,
        "entity:transform_beat",
    );
    registrar.require_rollback::<ambition_held_items::GroundItem>(OWNER, "entity:ground_item");
    registrar.require_rollback::<ambition_held_items::SettledItem>(OWNER, "entity:settled_item");
    // `RoomScopedEntity` governs lifetime, not rewindability; moving world
    // items therefore require their own rollback registration.
    registrar.require_rollback::<ambition_world_items::world_item::WorldItem>(OWNER, "entity:world_item");
    registrar
        .require_rollback::<crate::gravity::GravityFlipSwitch>(OWNER, "entity:gravity_flip_switch");
    // the heal shrine, for the same reason as the portal gun pickup
    // . It carries `SimId`, `SpawnOrigin` and
    // `TransactionId`, had no anchor, and so those registrations were inert on
    // it. Its own component is waived as authored geometry — the heal reads it
    // and never writes it — but the anchor is not about the shrine's data; it is
    // about whether GGRS reproduces the ENTITY on a resimulated timeline.
    registrar.require_rollback::<crate::shrine::HealShrine>(OWNER, "entity:heal_shrine");
    // ⛔⛔ TWO AUTONOMOUS EMITTERS THAT WERE NOT IN THE ROLLBACK VOCABULARY AT
    // ALL — not inert, ABSENT. A turret and a singularity are both spawned
    // mid-match by an ability, both outlive the frame that made them, and both
    // carry timers that decide when the world changes: the sentry's cooldown
    // decides which tick a bolt is emitted on, the well's `remaining_s` decides
    // how long every body in radius keeps being pulled. Nothing saved either, so
    // a rewind across a deployment kept the future's turret, its future
    // cooldown, and the shots that cooldown had already authorized.
    //
    // ⚠ THE ANCHOR IS WHY THE COVERAGE SWEEP NEVER SAID SO. A one-shot census
    // walks the entities a booted room HAS; a turret exists only after somebody
    // fires, so its absence from the registry read exactly like a pass. See the
    // scenario sweep this landed with.
    registrar.require_rollback::<crate::abilities::ranged::sentry::Sentry>(OWNER, "entity:sentry");
    registrar.rollback_component_clone_probed::<crate::abilities::ranged::sentry::Sentry>(
        OWNER,
        "ability.sentry",
        |sentry| {
            ((sentry.pos.x.to_bits() as u64) << 32)
                ^ (sentry.pos.y.to_bits() as u64)
                ^ ((sentry.remaining_s.to_bits() as u64) << 16)
                ^ (sentry.fire_cooldown.to_bits() as u64)
        },
    );
    registrar.require_rollback::<crate::abilities::ranged::vortex::VortexWell>(
        OWNER,
        "entity:vortex_well",
    );
    registrar.rollback_component_clone_probed::<crate::abilities::ranged::vortex::VortexWell>(
        OWNER,
        "ability.vortex_well",
        |well| {
            ((well.center.x.to_bits() as u64) << 32)
                ^ (well.center.y.to_bits() as u64)
                ^ (well.remaining_s.to_bits() as u64)
        },
    );
    // It is not an actor fact and not an every-game fact — it is an authoring format's. Its
    // declaration now lives in `ambition_platformer2d_ldtk`; the runtime's opt-in `LdtkWorldPlugin`
    // installs that domain offer only for LDtk games.
    // The shot's launch-time combat allegiance is intentionally owned here, not
    // by `ambition_projectiles`: the reusable projectile model is forbidden to
    // depend on the character/combat vocabulary from which this stamp is built.
    // The old central runtime adapter reached the type through
    // `runtime::projectile_schedule`; domain-owned registration instead follows
    // the concrete type to the actor-side projectile integration that owns it.
    registrar.rollback_component_canonical::<crate::projectile::ProjectileAllegiance>(
        OWNER,
        "projectile.allegiance",
    );
    registrar.rollback_resource_optional_canonical::<crate::character_runtime::ActiveMatch>(
        OWNER,
        "resource.active_match",
    );
    // The stocks ruleset's verdict is *the outcome for match X*, stamped with the
    // `MatchInstance` the receipt above publishes — so a rewind that restores one and not the
    // other would restore a verdict about a match that is not running. Registered together,
    // they rewind together.
    registrar.rollback_resource_canonical::<crate::features::stocks_match::StocksMatchSettled>(
        OWNER,
        "resource.stocks_match_settled",
    );
    // …and whether it refused to be settled. Sudden death is entered by NOT
    // deciding, so this latch is the only thing standing between a level timeout
    // and re-entering the tie on every tick that follows.
    registrar.rollback_resource_canonical::<crate::features::stocks_match::SuddenDeathEntered>(
        OWNER,
        "resource.sudden_death_entered",
    );
    // …and HOW LONG it has been fought. Counted, not derived: the timeout and
    // the item cadence both read it, and "how many ticks was this paused" is
    // written nowhere a rewind could recompute it from.
    registrar
        .rollback_resource_canonical::<crate::character_runtime::live_match_clock::LiveMatchTicks>(
            OWNER,
            "resource.live_match_ticks",
        );
    // …and the announcement it makes once. Written inside the sim and read
    // inside it (the stage puts its survivors on the authored damage), so a
    // reader's `Local` cursor has to rewind with the latch above — otherwise a
    // rewind across the entering frame leaves a match in sudden death whose
    // fighters were never placed.
    registrar.clear_message_on_rollback::<crate::features::stocks_match::SuddenDeathBegan>(
        OWNER,
        "message.sudden_death_began",
    );
    registrar.rollback_resource_canonical::<crate::features::GameplayElapsed>(
        OWNER,
        "resource.gameplay_elapsed",
    );
    registrar.rollback_resource_canonical::<crate::session::reset::NewGameResetRequested>(
        OWNER,
        "resource.sandbox_reset_requested",
    );
    registrar
        .rollback_resource_canonical::<crate::session::lifecycle_commit::PendingLifecycleCommit>(
            OWNER,
            "resource.pending_lifecycle_commit",
        );
    registrar.rollback_resource_clone_entity_set::<crate::abilities::traversal::possession::PossessionState>(
            OWNER,
            "resource.possession_state",
            |state| state.possessed.into_iter().chain(state.home).collect(),
        );
    registrar
        .rollback_resource_map_entities::<crate::abilities::traversal::possession::PossessionState>(
            OWNER,
            "map.resource.possession_state",
        );
    registrar.rollback_component_canonical::<crate::character_runtime::MatchSeat>(
        OWNER,
        "actor.match_seat",
    );
    registrar
        .rollback_component_cursor::<ambition_body_seed::ActorMotionPath>(OWNER, "actor.motion_path");
    registrar.rollback_component_canonical::<crate::features::ecs::perception::Perception>(
        OWNER,
        "actor.perception",
    );
    registrar.rollback_component_canonical::<crate::features::ecs::perception::PerceptionMemory>(
        OWNER,
        "actor.perception_memory",
    );
    // ⛔ THE TYPE MOVED to `shared_tangle::temporary_control` (2026-08-26); the
    // STABLE NAME `actor.temporary_control` deliberately did NOT. It is an
    // identity on the wire, not an address. The registration stays here because
    // the monolith is still what installs the control modes it records.
    registrar.rollback_component_canonical::<ambition_platformer2d_shared_tangle::temporary_control::TemporaryControl>(
        OWNER,
        "actor.temporary_control",
    );
    registrar.rollback_component_canonical::<ambition_platformer2d_core::body_clusters::ActorSurfaceState>(
        OWNER,
        "actor.surface_state",
    );
    registrar.rollback_component_clone::<crate::control::LocalPlayer>(OWNER, "player.local_marker");
    // The body a reset hands back, split out of `actor.config` (2026-08-26) so a
    // mount dismount can restore a rider without naming the monolith's authored
    // actor definition. Same shape as the row above: authored, immutable after
    // spawn, cloned rather than encoded.
    registrar.rollback_component_clone::<ambition_platformer2d_shared_tangle::body::SpawnBaseline>(
        OWNER,
        "actor.spawn_baseline",
    );
    registrar.rollback_component_clone_probed::<crate::features::transform_beat::TransformBeat>(
        OWNER,
        "actor.transform_beat",
        |beat| beat.remaining.to_bits() as u64,
    );
    registrar.rollback_component_clone::<crate::features::transform_beat::TransformBeatRequested>(
        OWNER,
        "actor.transform_beat_requested",
    );
    // ⛔ THE STABLE NAME STAYS `mount.mass` THOUGH THE TYPE LEFT `mount`. It is
    // an IDENTITY on the wire, not an address: renaming it to match the new home
    // would be a declared schema change bought for tidiness, and every peer would
    // have to agree to it. The type moved to `shared_tangle::body` because two
    // domains share it; the wire does not care where a type lives.
    registrar.rollback_component_clone::<ambition_platformer2d_shared_tangle::body::Mass>(
        OWNER,
        "mount.mass",
    );
    // An ARMED challenge, counting down to a fight.
    //
    // it was not rollback state, and it is the `SaveRestored` failure
    // in another domain. `tick_pending_challenges` REMOVES it in the sim
    // schedule; a rewind past that removal restored everything the removal
    // implied and left the removal itself standing, so the fight the narrative
    // armed was quietly disarmed by a rollback. The insert is a simulation
    // decision now (`arm_requested_challenges`), so it belongs in the snapshot
    // like every other simulation decision.
    registrar.rollback_component_clone_entity_set::<crate::features::PendingChallenge>(
        OWNER,
        "actor.pending_challenge",
        |pending| pending.challenger.into_iter().collect(),
    );
    registrar
        .rollback_map_entities::<crate::features::PendingChallenge>(OWNER, "map.pending_challenge");
    registrar.rollback_component_clone::<ambition_held_items::StashedActionSet>(
        OWNER,
        "actor.stashed_action_set",
    );
    registrar.rollback_component_clone_probed::<crate::avatar::PersonaBaseline>(
        OWNER,
        "actor.persona_baseline",
        |baseline| {
            use std::hash::{Hash, Hasher};
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            baseline.id.hash(&mut hasher);
            // The STANDING physicals are in the probe for the same reason
            // `ProjectedCharacterKit.granted` is: retraction is driven from them,
            // so a rewind that restores the record without them retracts a
            // replacement to the wrong numbers and nothing reads wrong until a
            // character swap. Mass is hashed by its bit pattern — this is a
            // checksum, not an arithmetic comparison, so `to_bits` is exactly
            // right and `f32`'s missing `Hash` is not an obstacle.
            baseline.displaced.max_health.hash(&mut hasher);
            baseline
                .displaced
                .mass
                .map(|mass| mass.map(f32::to_bits))
                .hash(&mut hasher);
            hasher.finish() ^ baseline.generation.get().rotate_left(32)
        },
    );
    registrar.rollback_component_clone_probed::<crate::character_runtime::ProjectedCharacterKit>(
        OWNER,
        "actor.projected_character_kit",
        |projected| {
            use std::hash::{Hash, Hasher};
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            projected.id.hash(&mut hasher);
            projected.granted.hurtboxes.hash(&mut hasher);
            projected.granted.movement_tuning.hash(&mut hasher);
            projected.granted.posed_body.hash(&mut hasher);
            hasher.finish() ^ projected.generation.get().rotate_left(32)
        },
    );
    registrar.rollback_component_clone_probed::<crate::character_runtime::BodyPoseClock>(
        OWNER,
        "actor.body_pose_clock",
        |clock| checksum_bytes(clock.pose.as_bytes()) ^ clock.elapsed_s.to_bits() as u64,
    );
    registrar.rollback_component_clone::<crate::character_runtime::AuthoredHurtboxes>(
        OWNER,
        "actor.authored_hurtboxes",
    );
    registrar.rollback_component_clone::<crate::features::PickupCollectLock>(
        OWNER,
        "feature.pickup_collect_lock",
    );
    registrar.rollback_component_clone::<crate::features::PickupArt>(OWNER, "feature.pickup_art");
    registrar
        .rollback_component_clone::<ambition_held_items::GroundItem>(OWNER, "item.ground_item");
    // ⭐ SLEEP IS SIMULATION STATE, and this one is presence/absence rather than
    // a value. `ground_item_physics` skips a settled object entirely, so a
    // rewind past the frame an object landed must also un-settle it — otherwise
    // the replay steps a resting item on one peer and not the other, and the
    // divergence is silent until the positions have drifted apart.
    //
    // ⭐⭐ PROBED, NOT PRESENCE-ONLY, since it stopped being a marker. It carries
    // `impact_speed` now — the speed the step that settled it was travelling —
    // and the Smash bomb detonates on that value. A presence probe satisfies the
    // coverage test while seeing NOTHING of it, so a rewind that restored the
    // component with a stale speed would be invisible: the item is settled on
    // both peers and the bomb goes off on one. `rollback_exit_oracle` is what
    // said so, on the run after the field was added.
    // ⛔⛔ IT DECIDES WHETHER AN OBJECT IN THE WORLD IS GOING TO EXPLODE, and it
    // lives on a `GroundItem`, which is already an anchor. A rewind that lost it
    // disarms a live bomb; a rewind that restored it onto a caught bomb re-arms
    // one in a hand. It replaced a velocity heuristic that WAS rollback state by
    // accident (`GroundItem` carries `vel`), so this is the same coverage moved
    // to the fact that actually decides.
    registrar.rollback_component_clone_probed::<ambition_held_items::ReleasedAs>(
        OWNER,
        "item.released_as",
        |released| match released.0 {
            ambition_held_items::Release::Throw => 1,
            ambition_held_items::Release::Drop => 2,
        },
    );
    registrar.rollback_component_clone_probed::<ambition_held_items::SettledItem>(
        OWNER,
        "item.settled_item",
        |settled| settled.impact_speed.to_bits() as u64,
    );
    // CUSTODY IS SIMULATION STATE, not a cache. It decides on every later
    // frame whether the item is drawn, stepped by `ground_item_physics`, and
    // grabbable — so a rewind that restored the wrong value leaves the same axe
    // both in a hand and on the floor, or makes a carried axe fall out of it.
    // It replaced a despawn/spawn pair, which GGRS reproduced through the
    // entity anchor; the state that took over the same job owes the same
    // coverage.
    //
    // `_entity_set` rather than `_entity_ref`: `InWorld` names no body at all,
    // so the handle is a zero-or-one set. The probe therefore measures WHICH
    // body is holding it through that body's stable identity — a restore that
    // hands the item to the wrong holder changes this census and would not
    // change a presence count.
    registrar.rollback_component_clone_entity_set::<ambition_held_items::ItemCustody>(
        OWNER,
        "item.item_custody",
        |custody| match custody {
            ambition_held_items::ItemCustody::InWorld => Vec::new(),
            ambition_held_items::ItemCustody::Held { holder } => {
                vec![*holder]
            }
        },
    );
    registrar.rollback_map_entities::<ambition_held_items::ItemCustody>(OWNER, "map.item_custody");
    // The pickup's ATTRACTION POLICY rides the same entity as the pickup, so a
    // rewind that recreates a dropped coin has to recreate whether it comes to
    // you. Authored at spawn and never mutated — but "never mutated" is not
    // "never needs restoring" when the entity itself is rollback state.
    //
    // caught by `rollback_exit_oracle`'s PER-FRAME census within an hour of
    // the component existing, because a dropped coin is transient — spawned and
    // despawned inside the route — and the one-shot sweep in `rollback_coverage`
    // cannot see it. That is B3b's first blind spot, demonstrating itself.
    registrar.rollback_component_clone_probed::<crate::features::ecs::pickups::PickupMagnet>(
        OWNER,
        "item.pickup_magnet",
        |magnet| {
            use std::hash::{Hash, Hasher};
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            magnet.range.to_bits().hash(&mut hasher);
            magnet.speed.to_bits().hash(&mut hasher);
            hasher.finish()
        },
    );
    // Presence is authoritative because gameplay edge-triggers on this marker;
    // rollback must restore whether the actor was dormant, not merely re-derive it.
    registrar.rollback_component_clone::<crate::features::ecs::dormancy::Dormant>(
        OWNER,
        "actor.dormant",
    );
    // The radius is part of authoritative dormancy policy, so probe the value
    // rather than only the component's presence.
    registrar.rollback_component_clone_probed::<crate::features::ecs::dormancy::DormancyPolicy>(
        OWNER,
        "actor.dormancy_policy",
        |policy| {
            use crate::features::ecs::dormancy::DormancyPolicy;
            use std::hash::{Hash, Hasher};
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            match policy {
                DormancyPolicy::Never => 0u8.hash(&mut hasher),
                DormancyPolicy::AwakeNearObservers { radius } => {
                    1u8.hash(&mut hasher);
                    // Bit pattern: this is a checksum, not an arithmetic
                    // comparison.
                    radius.to_bits().hash(&mut hasher);
                }
            }
            hasher.finish()
        },
    );
    registrar.rollback_component_clone::<crate::features::ecs::SpawnedThisAttempt>(
        OWNER,
        "lifecycle.spawned_this_attempt",
    );
    registrar.rollback_component_clone_probed::<ambition_world_items::world_item::WorldItem>(
        OWNER,
        "item.world_item",
        |item| {
            use std::hash::{Hash, Hasher};
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            // Bit patterns: this is a checksum, not an arithmetic comparison.
            item.pos.x.to_bits().hash(&mut hasher);
            item.pos.y.to_bits().hash(&mut hasher);
            item.half_extent.x.to_bits().hash(&mut hasher);
            item.half_extent.y.to_bits().hash(&mut hasher);
            item.sprite.hash(&mut hasher);
            // WHAT COLLECTING IT DOES, which the box and the sprite do not say.
            // Two pickups can sit in the same place looking the same and equip
            // different rows, and only this tells them apart.
            //
            // the ROW IDENTITY, not the row's authored numbers. `EquipmentRow`
            // carries modifiers, grants and an on-hit rule, and those are
            // CONTENT — read from the same files, identical for a given id in
            // one build, and therefore incapable of differing between two
            // timelines of one session. The id and the exclusive slot are what
            // a divergent spawn would change.
            match &item.payload {
                ambition_world_items::world_item::WorldItemPayload::Equip(row) => {
                    row.id.hash(&mut hasher);
                    row.exclusive_slot.hash(&mut hasher);
                }
            }
            hasher.finish()
        },
    );
    // AN ENGINE COMPONENT IS REGISTERED ONCE, BY THE ENGINE. `Empowered`
    // lives in `features::empowerment`, and Mary-O and Sanic each registered it
    // from their own plugin — which is fine in a composition holding one demo
    // and a PANIC in the app, which holds both: bevy_ggrs refuses a second
    // `ComponentSnapshotPlugin` for one type ("plugin was already added"), and
    // 56 app tests died on that one line. Two games owning one engine type is
    // duplicate authority; the engine owns it.
    registrar.rollback_component_clone_probed::<crate::features::empowerment::Empowered>(
        OWNER,
        "feature.empowered",
        |empowered| {
            use std::hash::{Hash, Hasher};
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            // The REMAINING clock matters as much as the traits: an empowerment
            // restored with the wrong time left expires on a different frame,
            // and expiry is what flips invulnerability back off.
            empowered.remaining.map(f32::to_bits).hash(&mut hasher);
            empowered.traits.bits().hash(&mut hasher);
            hasher.finish()
        },
    );
    // The motion PLAN and its cursor travel together — `ItemMotion`'s own doc
    // says a cursor without its plan is meaningless — so one registration
    // restores both halves of where the pickup is in its arc.
    registrar.rollback_component_clone_probed::<ambition_world_items::item_motion::ItemMotion>(
        OWNER,
        "item.motion",
        |motion| {
            use std::hash::{Hash, Hasher};
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            // WHERE IT IS IN ITS ARC: how far the rise has got, how fast it is
            // travelling, and which way it turned last.
            motion.emerged_for().to_bits().hash(&mut hasher);
            motion.velocity().x.to_bits().hash(&mut hasher);
            motion.velocity().y.to_bits().hash(&mut hasher);
            motion.facing().to_bits().hash(&mut hasher);
            // AND THE PLAN IT IS FOLLOWING. The cursor above is meaningless
            // without it — `emerged_for` is read against `emerge.seconds`, and
            // the same cursor under a different plan is a different pickup
            // mid-arc. The plan is authored and does not normally change, which
            // is exactly why a timeline where it DID is worth catching.
            let plan = &motion.plan;
            plan.emerge
                .map(|e| (e.distance.to_bits(), e.seconds.to_bits()))
                .hash(&mut hasher);
            plan.speed.to_bits().hash(&mut hasher);
            plan.facing.to_bits().hash(&mut hasher);
            plan.gravity.to_bits().hash(&mut hasher);
            plan.bounce.to_bits().hash(&mut hasher);
            plan.turns_at_walls.hash(&mut hasher);
            hasher.finish()
        },
    );
    registrar.rollback_component_clone::<crate::gravity::GravityFlipSwitch>(
        OWNER,
        "gravity.flip_switch",
    );
    registrar.declare_rollback_derived_component::<crate::avatar::body_integration::PlayerBodyFrameOutput>(
        OWNER,
        "derived.player_body_frame_output",
        "republished by body integration every simulation frame",
    );
    registrar.declare_rollback_derived_component::<crate::body_mode::BodyModeCapabilities>(
        OWNER,
        "derived.body_mode_capabilities",
        "projected from the active body mode each frame",
    );
    registrar.declare_rollback_derived_component::<crate::character_runtime::ResolvedHurtboxes>(
        OWNER,
        "derived.resolved_hurtboxes",
        "recomputed from AuthoredHurtboxes plus the move and pose clocks each tick",
    );
    registrar.declare_rollback_derived_resource::<crate::features::ActorDecisionFacts>(
        OWNER,
        "derived.actor_decision_facts",
        "rebuilt from authoritative body state before every autonomous decision phase",
    );
    registrar.declare_rollback_derived_resource::<crate::features::ActorDecisionFrames>(
        OWNER,
        "derived.actor_decision_frames",
        "rebuilt by autonomous decision and consumed by same-tick control publication",
    );
    registrar.declare_rollback_derived_resource::<crate::features::ActorSteering>(
        OWNER,
        "derived.actor_steering",
        "rebuilt from the authoritative actor population before movement",
    );
    registrar.declare_rollback_derived_component::<ambition_boss_encounter::attack_geometry::BossAnimationFrameSample>(
        OWNER,
        "derived.boss_animation_frame_sample",
        "republished every tick by drive_boss_animators from the rewound BossAnimFrame cursor",
    );
    registrar
        .declare_rollback_derived_resource::<crate::features::ecs::perception::PerceptionPeers>(
            OWNER,
            "derived.perception_peers",
            "perception snapshot rebuilt every tick before brains read it",
        );
    registrar.declare_rollback_derived_resource::<crate::features::ecs::perception::PerceptionProjectiles>(
        OWNER,
        "derived.perception_projectiles",
        "perception snapshot rebuilt every tick before brains read it",
    );
    registrar
        .clear_message_on_rollback::<crate::features::BrainCommand>(OWNER, "message.brain_command");
    // What a conversation asked the simulation for, released by the
    // narrative ledger at the head of the tick it was stamped for. Cleared on
    // load for the same reason as every other released fact: the resimulated
    // tick is handed it again from the ledger rather than remembering it from
    // the branch that was abandoned.
    registrar.clear_message_on_rollback::<crate::features::ChallengeRequested>(
        OWNER,
        "message.challenge_requested",
    );
    registrar.clear_message_on_rollback::<crate::features::ReleaseProvocation>(
        OWNER,
        "message.release_provocation",
    );
    registrar.clear_message_on_rollback::<crate::features::SpawnActorRequest>(
        OWNER,
        "message.spawn_actor_request",
    );
    registrar.clear_message_on_rollback::<crate::session::reset::NewGameResetCommitted>(
        OWNER,
        "message.sandbox_reset_committed",
    );
    registrar.clear_message_on_rollback::<ambition_damage::WalletShieldSpent>(
        OWNER,
        "message.wallet_shield_spent",
    );
    registrar.clear_message_on_rollback::<crate::avatar::PlayerHealRequested>(
        OWNER,
        "message.player_heal_requested",
    );
    registrar.clear_message_on_rollback::<crate::avatar::trail::TrailContinuityBreak>(
        OWNER,
        "message.trail_continuity_break",
    );
    registrar.clear_message_on_rollback::<crate::session::reset::RoomReplayRequested>(
        OWNER,
        "message.room_replay_requested",
    );
    // Save application is a rollback-relevant latch because the state it guards
    // rewinds even though the literal Update systems that set it do not resimulate.
    registrar.rollback_resource_clone::<crate::session::durable_horizon::SaveRestored>(
        OWNER,
        "resource.save_restored",
    );
    // ⛔⛔ THIS WAS TWO `Local`s ON A SIM SYSTEM.
    // `restore_checkpoint_on_session_start` runs in `PlayerSimulation`, and a
    // `Local` does not rewind: a rollback crossing the frame it routed on would
    // resimulate with the memory already past the crossing, so one timeline asks
    // for the crossing and the other believes it already did.
    //
    // ⭐ PROBED BY WHICH GENERATION, not by presence. A restore that brought
    // back the wrong generation makes one timeline re-ask for a crossing the
    // other already spent, and a presence probe sees none of that.
    registrar.rollback_resource_clone_checksum::<crate::shrine::CheckpointResumeProgress>(
        OWNER,
        "resource.checkpoint_resume_progress",
        "which session generation the resume has routed and placed",
        crate::shrine::CheckpointResumeProgress::checksum,
    );

    // ⛔⛔ EVENT-CREATED AUTHORITATIVE STATE, AND THAT IS WHY IT WAS MISSING. The
    // boot census can only see components that exist in the INITIAL world; every
    // component below is inserted later, by an ability firing, so nothing
    // structural ever asked whether it rewinds. Four gameplay decisions were
    // being made from state a rewind left at its future value.
    //
    // ⭐ PROBED, NOT PRESENCE-ONLY. Each of these is a NUMBER that decides an
    // outcome — where a recall puts a body, which tick a bomb goes off — and a
    // presence probe satisfies the coverage oracle while seeing none of it.
    registrar.rollback_component_clone_probed::<crate::abilities::traversal::mark_recall::PlayerMark>(
        OWNER,
        "ability.player_mark",
        // WHERE the mark is, not merely that one exists. Recall teleports to this
        // position, so a rewind across setting or moving the mark that kept the
        // future position puts the body somewhere the resimulation never chose.
        |mark| match mark.pos {
            Some(pos) => {
                ((pos.x.to_bits() as u64) << 32) ^ (pos.y.to_bits() as u64) ^ 0x9e37_79b9
            }
            None => 0,
        },
    );
    registrar.rollback_component_clone_probed::<crate::abilities::ranged::bomb::BombFuse>(
        OWNER,
        "ability.bomb_fuse",
        // The countdown, because WHICH TICK it reaches zero is the explosion.
        |fuse| fuse.timer.to_bits() as u64,
    );
    registrar
        .rollback_component_clone_probed::<crate::abilities::thrown::gravity_grenade::GravityGrenadeFuse>(
            OWNER,
            "ability.gravity_grenade_fuse",
            // Same shape as the bomb: the tick this reaches zero is the tick a
            // gravity well opens, and a well moves every body inside it.
            |fuse| fuse.timer.to_bits() as u64,
        );
    // ✔ THE PARALLEL PROJECTILE IS GONE (K2, 2026-09-02). `item.held_projectile`
    // stood here: `HeldProjectile` rode on `BodyKinematics` while its own
    // damage, range and splash were not rollback state, so a rewind restored
    // where a held shot WAS and kept what it would do when it landed. The row
    // was added as "registered, not endorsed", naming the real repair — fold
    // held shots into `ProjectileSpawnRequest` and delete the second projectile
    // simulation. That is done: a held item's shot is a `LiveProjectile` whose
    // `ProjectileGameplay` (registered by the projectile domain, splash
    // included) is the only in-flight state it has.

    // A MARKER, and presence is the whole of it — the summon cap counts these,
    // so a rewind that dropped one lets a fourth slug through.
    registrar.rollback_component_clone::<crate::abilities::thrown::puppy_slug_gun::PuppySlugAlly>(
        OWNER,
        "ability.puppy_slug_ally",
    );

    // Item checkpoint baselines declare their rewind/checksum obligations beside
    // the item horizon that owns capture, restore and durable adoption.
    crate::items::pickup::minted_horizon::register_checkpoint_rollback_state(registrar);

    registrar
        .declare_rollback_derived_resource::<crate::world::gated_lock_walls::GatedLockWallCache>(
        OWNER,
        "derived.gated_lock_wall_cache",
        "authored gated walls for the active room; recomputed from the room set and LDtk project",
    );
    registrar
        .declare_rollback_derived_resource::<crate::world::gated_lock_walls::GatedLockWallVerdicts>(
            OWNER,
            "derived.gated_lock_wall_verdicts",
            "why each authored gated wall stands; rebuilt from the walls' verdicts every gate tick",
        );
    registrar.declare_rollback_derived_resource::<
        crate::world::authored_switch_commands::AuthoredSwitchCommands,
    >(
        OWNER,
        "derived.authored_switch_commands",
        "authored switch verbs prepared for the active room; recomputed from the room set and LDtk project",
    );
}
