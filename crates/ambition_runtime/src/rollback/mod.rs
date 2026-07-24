//! Ambition's integration boundary for `ggrs` + `bevy_ggrs`.
//!
//! GGRS is the sole rollback authority. It owns frame requests, prediction,
//! snapshot history, entity recreation, load ordering, resimulation, confirmed
//! frames, and checksum comparison. Ambition contributes only:
//!
//! - the typed list of authoritative components/resources;
//! - deterministic checksum projections for float-heavy domain values;
//! - exact registration/content identity;
//! - the input bridge and session lifecycle policy.

use bevy::{
    ecs::schedule::{ExecutorKind, LogLevel, ScheduleBuildSettings},
    prelude::*,
};
use bevy_ggrs::{GgrsPlugin, RollbackFrameRate};

pub use bevy_ggrs::{
    AdvanceWorld, AdvanceWorldSystems, ConfirmedFrameCount, GgrsSchedule, LoadWorld,
    LoadWorldSystems, Rollback, RollbackFrameCount, RunGgrsSystems,
};

/// Ambition-owned work that must run after every `bevy_ggrs` entity/data/map restore.
#[derive(SystemSet, Clone, Debug, Hash, PartialEq, Eq)]
pub enum AmbitionLoadWorldSet {
    /// Reconcile authored/runtime pairs after all raw `Entity` handles have been remapped.
    Reconcile,
}

mod codec;
mod codecs;
mod motion_codec;
#[cfg(test)]
mod provenance_tests;
mod registry;
mod session;

pub use codec::*;
pub use codecs::{ensure_sim_id, heal_projectile_owners, mint_spawned_sim_ids};
pub use registry::*;
pub use session::*;

/// Installs the host-independent typed rollback schema used by prepared
/// content identity. Non-GGRS games retain this lightweight registry without
/// installing snapshot history, schedules, checksums, or session machinery.
pub struct AmbitionRollbackSchemaPlugin;

impl Plugin for AmbitionRollbackSchemaPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RollbackRegistry>();
        register_engine_rollback_state(app);
    }
}

/// FORK(ggrs-frame-timing): recover the intra-tick phase from the GGRS
/// driver's own fixed-timestep timing.
///
/// Presentation draws on the render clock while the sim advances on a fixed
/// tick, so a published pose is a step function; drawing it directly makes
/// anything that moves shudder against a smoothly-easing camera. Removing that
/// needs to know how far through the current tick this frame sits, and under
/// GGRS the only truthful source is the driver's own accumulator — the one
/// that decides when to advance. `Time<Fixed>::overstep_fraction()` answers it
/// for the plain fixed host and is unavailable here precisely because GGRS
/// banks its own time.
///
/// `GgrsFrameTiming` publishes that quantity as a supported accessor:
/// `overstep_fraction()` is the accumulator as a fraction of the *actual*
/// timestep the last driver pass used, so it stays correct during run-slow
/// catch-up where that timestep widens — better than dividing by the nominal
/// rate. This compiles against a `bevy_ggrs` fork that backports the accessor
/// onto the v0.21.0 / bevy-0.18 line; the `[patch.crates-io]` entry in the
/// workspace manifest carries the rationale and the condition that retires it.
///
/// A parallel accumulator was considered and rejected: it would agree only
/// while nothing interesting happened, and diverge during run-slow catch-up,
/// stalls, several advances in one frame, and rollback resimulation — exactly
/// when a wrong phase shows most. A presentation clock that lies during a
/// rollback is worse than no smoothing at all.
///
/// Retire when the accessor ships in a released `bevy_ggrs`: drop the
/// `[patch.crates-io]` entry, bump the requirement, and use the released type.
fn sample_ggrs_accumulator_phase(
    timing: Res<bevy_ggrs::GgrsFrameTiming>,
    mut phase: ResMut<ambition_sim_view::PresentationPhase>,
) {
    // `overstep_fraction` reports the accumulator as a fraction of the driver's
    // timestep in `[0, 1)`, and yields 0 before the first driver pass.
    phase.set(timing.overstep_fraction());
}

/// Installs GGRS schedules, snapshot storage, and session/request handling.
pub struct AmbitionRollbackPlugin;

impl Plugin for AmbitionRollbackPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(GgrsPlugin::<AmbitionGgrsConfig>::default())
            .insert_resource(RollbackFrameRate(crate::SIM_TICK_HZ as usize));

        // Publish the rollback host's intra-tick phase for the presented-pose
        // layer. Same set, same consumer, same ordering as the fixed-tick
        // sampler `ambition_sim_view` installs for itself — only the clock's
        // hiding place differs.
        app.add_systems(
            Update,
            sample_ggrs_accumulator_phase
                .in_set(ambition_sim_view::PresentedPoseSet)
                .before(ambition_sim_view::presented_pose::advance_presented_body_poses),
        );

        // Ambition's gameplay schedule is composed from explicit ordered phase
        // sets, but systems within a phase intentionally rely on deterministic
        // App construction order rather than hundreds of meaningless pairwise
        // edges. GGRS is a managed same-build contract: every peer runs the
        // same plugin graph. Execute that graph serially so conflicting systems
        // cannot race, and disable Bevy's ambiguity diagnostic for this one
        // schedule; the real determinism oracle is SyncTestSession resimulation.
        app.edit_schedule(GgrsSchedule, |schedule| {
            schedule.set_executor_kind(ExecutorKind::SingleThreaded);
            schedule.set_build_settings(ScheduleBuildSettings {
                ambiguity_detection: LogLevel::Ignore,
                ..default()
            });
        });

        app.configure_sets(
            bevy_ggrs::LoadWorld,
            AmbitionLoadWorldSet::Reconcile.after(bevy_ggrs::LoadWorldSystems::Mapping),
        )
        .add_systems(
            bevy_ggrs::LoadWorld,
            codecs::reconcile_brain_bindings.in_set(AmbitionLoadWorldSet::Reconcile),
        );
        session::install_session_bridge(app);
    }
}

fn room_set_checksum(rooms: &ambition_actors::rooms::RoomSet) -> u64 {
    let mut bytes = Vec::new();
    put_u64(&mut bytes, rooms.active as u64);
    put_u64(&mut bytes, rooms.start as u64);
    put_str(&mut bytes, &rooms.active_spec().id);
    checksum_bytes(&bytes)
}

fn ldtk_runtime_index_checksum(index: &ambition_actors::ldtk_world::LdtkRuntimeIndex) -> u64 {
    checksum_bytes(index.active_area().as_bytes())
}

/// Entity-free canonical projection of the staged victim-hit FIFO.
///
/// The exact `Entity` handles (`attacker`, pre-resolved targets) stay out —
/// the stable-id contract keeps allocator-local values out of every checksum —
/// but everything that decides what the hit DOES participates, so a diverged
/// queue surfaces as a sync-test mismatch at the staging frame instead of one
/// frame later as mystery damage.
fn pending_player_hits_checksum(pending: &ambition_combat::events::PendingPlayerHitEvents) -> u64 {
    use ambition_combat::events::{HitKnockbackMagnitude, HitMode, HitSource, HitTarget};
    let mut bytes = Vec::new();
    put_u64(&mut bytes, pending.0.len() as u64);
    for event in &pending.0 {
        let bounds = event.volume.bounds();
        put_vec2(&mut bytes, bounds.min);
        put_vec2(&mut bytes, bounds.max);
        put_i32(&mut bytes, event.damage);
        let (source_tag, source_payload) = match event.source {
            HitSource::PlayerSlash { knock_x } => (0u8, knock_x),
            HitSource::PlayerProjectile => (1, 0.0),
            HitSource::PogoBounce => (2, 0.0),
            HitSource::Hazard => (3, 0.0),
            HitSource::EnemyBody => (4, 0.0),
            HitSource::EnemyAttack => (5, 0.0),
            HitSource::EnemyProjectile => (6, 0.0),
            HitSource::EnemyChargeCrash => (7, 0.0),
            HitSource::BossBody => (8, 0.0),
            HitSource::BossAttack => (9, 0.0),
        };
        put_u8(&mut bytes, source_tag);
        put_f32(&mut bytes, source_payload);
        put_bool(&mut bytes, event.attacker.is_some());
        put_u8(
            &mut bytes,
            match event.target {
                HitTarget::Volume => 0,
                HitTarget::Player(_) => 1,
                HitTarget::Actor(_) => 2,
                HitTarget::OrbMatch => 3,
            },
        );
        put_u8(
            &mut bytes,
            match event.mode {
                HitMode::Knockback => 0,
                HitMode::SafeRespawn => 1,
            },
        );
        match &event.knockback {
            None => put_bool(&mut bytes, false),
            Some(kb) => {
                put_bool(&mut bytes, true);
                put_f32(&mut bytes, kb.dir);
                match kb.magnitude {
                    HitKnockbackMagnitude::FeelScale(value) => {
                        put_u8(&mut bytes, 0);
                        put_f32(&mut bytes, value);
                    }
                    HitKnockbackMagnitude::LaunchSpeed(value) => {
                        put_u8(&mut bytes, 1);
                        put_f32(&mut bytes, value);
                    }
                }
                put_vec2(&mut bytes, kb.source_pos);
                put_vec2(&mut bytes, kb.impact_pos);
                match kb.launch_dir {
                    None => put_bool(&mut bytes, false),
                    Some(dir) => {
                        put_bool(&mut bytes, true);
                        put_vec2(&mut bytes, dir);
                    }
                }
            }
        }
        for key in &event.ignored_targets {
            put_str(&mut bytes, key);
        }
    }
    checksum_bytes(&bytes)
}

const ENGINE: &str = "ambition_runtime";

/// The complete engine-owned GGRS rollback registration set. Domain content
/// appends its own entries through [`AmbitionRollbackApp`].
pub fn register_engine_rollback_state(app: &mut App) {
    use ambition_engine_core::body_clusters as bc;
    use AmbitionRollbackApp as _;

    // Rollback participation. These anchors cover the canonical session root,
    // every simulated body, projectile-only entities, encounter authorities,
    // and any semantic-identity entity that does not fit those families.
    app.require_rollback::<ambition_actors::rooms::RoomSet>(ENGINE, "root:room_set")
        .require_rollback::<ambition_platformer_primitives::body::BodyKinematics>(
            ENGINE,
            "entity:body_kinematics",
        )
        .require_rollback::<ambition_projectiles::LiveProjectile>(ENGINE, "entity:live_projectile")
        .require_rollback::<ambition_encounter::EncounterLifecycle>(
            ENGINE,
            "entity:encounter_lifecycle",
        )
        .require_rollback::<ambition_platformer_primitives::lifecycle::FeatureSimEntity>(
            ENGINE,
            "entity:feature_sim_entity",
        )
        .require_rollback::<ambition_actors::items::pickup::GroundItem>(
            ENGINE,
            "entity:ground_item",
        )
        .require_rollback::<ambition_portal::PlacedPortal>(ENGINE, "entity:placed_portal")
        .require_rollback::<ambition_actors::gravity::GravityFlipSwitch>(
            ENGINE,
            "entity:gravity_flip_switch",
        )
        // In-flight strike volumes (moveset melee windows, DamageBox effects,
        // world AOEs). These are Commands-spawned mid-swing with a hit-once
        // set, so they MUST rewind like projectiles: a rollback window that
        // spans the volume's spawn or despawn edge otherwise re-simulates
        // against a fresh empty `HitboxHits` and the same swing hits the same
        // victim twice (the Phase-5 second-hit desync — an armed strike
        // re-staged its player hit on every late resim pass).
        .require_rollback::<ambition_vfx::Hitbox>(ENGINE, "entity:hitbox");

    // Canonical live-session root. Authored definitions are immutable and bound
    // by PreparedContentIdentity; only mutable selection/cursor state rewinds.
    app.rollback_component_clone_checksum::<ambition_actors::rooms::RoomSet>(
        ENGINE,
        "root.room_set",
        "bevy_ggrs clone snapshot + active/start room identity checksum",
        room_set_checksum,
    )
    .rollback_component_clone::<ambition_engine_core::RoomGeometry>(ENGINE, "root.geometry")
    .rollback_component_clone::<ambition_actors::rooms::ActiveRoomMetadata>(
        ENGINE,
        "root.active_room_metadata",
    )
    .rollback_component_clone_checksum::<ambition_actors::ldtk_world::LdtkRuntimeIndex>(
        ENGINE,
        "root.ldtk_runtime_index",
        "bevy_ggrs clone snapshot + active LDtk area checksum",
        ldtk_runtime_index_checksum,
    )
    .rollback_component_clone::<ambition_actors::rooms::RoomMusicRequest>(
        ENGINE,
        "root.room_music_request",
    )
    .rollback_component_clone::<ambition_encounter::EncounterMusicRequest>(
        ENGINE,
        "root.encounter_music_request",
    );

    // Global authoritative resources.
    app.rollback_resource_canonical::<ambition_time::SimTick>(ENGINE, "resource.sim_tick")
        .rollback_resource_canonical::<ambition_time::WorldTime>(ENGINE, "resource.world_time")
        .rollback_resource_canonical::<ambition_actors::features::GameplayElapsed>(
            ENGINE,
            "resource.gameplay_elapsed",
        )
        .rollback_resource_canonical::<ambition_world::collision::MovingPlatformSet>(
            ENGINE,
            "resource.moving_platform_set",
        )
        .rollback_resource_canonical::<ambition_projectiles::ProjectileSeqCounter>(
            ENGINE,
            "resource.projectile_seq_counter",
        )
        .rollback_resource_cursor::<ambition_combat::slots::CombatSlotsRes>(
            ENGINE,
            "resource.combat_slot_board",
        )
        .rollback_resource_clone::<crate::InputStreamRecorder>(
            ENGINE,
            "resource.input_stream_recorder",
        )
        .rollback_resource_canonical::<ambition_time::ClockState>(ENGINE, "resource.clock_state")
        .rollback_resource_canonical::<ambition_actors::time::time_control::RequestedClockScale>(
            ENGINE,
            "resource.requested_clock_scale",
        )
        .rollback_resource_canonical::<ambition_actors::time::time_control::RegimePolicy>(
            ENGINE,
            "resource.clock_regime_policy",
        )
        .rollback_resource_canonical::<ambition_platformer_primitives::time::SimDt>(
            ENGINE,
            "resource.sim_dt",
        )
        .rollback_resource_canonical::<ambition_platformer_primitives::gravity::BaseGravity>(
            ENGINE,
            "resource.base_gravity",
        )
        .rollback_resource_canonical::<ambition_platformer_primitives::gravity::GravityField>(
            ENGINE,
            "resource.gravity_field",
        )
        .rollback_resource_canonical::<ambition_actors::control::SlotInteractionState>(
            ENGINE,
            "resource.slot_interaction_state",
        )
        .rollback_resource_canonical::<ambition_actors::session::reset::SandboxResetRequested>(
            ENGINE,
            "resource.sandbox_reset_requested",
        )
        .rollback_resource_canonical::<ambition_actors::session::lifecycle_commit::PendingLifecycleCommit>(
            ENGINE,
            "resource.pending_lifecycle_commit",
        )
        .rollback_resource_canonical::<ambition_projectiles::enemy::EnemyProjectileState>(
            ENGINE,
            "resource.enemy_projectile_state",
        )
        .rollback_resource_canonical::<ambition_actors::SandboxSimState>(
            ENGINE,
            "resource.sandbox_sim_state",
        )
        .rollback_resource_clone::<ambition_persistence::save::SandboxSave>(
            ENGINE,
            "resource.sandbox_save",
        )
        .rollback_resource_clone::<ambition_persistence::quest::registry::QuestRegistry>(
            ENGINE,
            "resource.quest_registry",
        )
        .rollback_resource_clone::<ambition_items::OwnedItems>(
            ENGINE,
            "resource.owned_items",
        )
        .rollback_resource_clone::<ambition_encounter::EncounterRegistry>(
            ENGINE,
            "resource.encounter_registry",
        )
        .rollback_resource_map_entities::<ambition_encounter::EncounterRegistry>(
            ENGINE,
            "map.resource.encounter_registry",
        )
        .rollback_resource_clone::<ambition_actors::abilities::traversal::possession::PossessionState>(
            ENGINE,
            "resource.possession_state",
        )
        .rollback_resource_map_entities::<ambition_actors::abilities::traversal::possession::PossessionState>(
            ENGINE,
            "map.resource.possession_state",
        )
        .rollback_resource_clone::<ambition_portal::PortalFrameHistory>(
            ENGINE,
            "resource.portal_frame_history",
        )
        // Cross-frame FIFO: produced in `GameplayEffects`, drained in
        // `EncounterSimulation` — which is ordered EARLIER, so the queue is
        // non-empty across a save boundary and a rewind would otherwise replay
        // switch activations the confirmed timeline already applied
        // (deep review 2026-07-19 §2.2).
        .rollback_resource_clone::<ambition_actors::encounter::SwitchActivationQueue>(
            ENGINE,
            "resource.switch_activation_queue",
        )
        // Latent until something mutates them in-session, but a rewind that
        // keeps a predicted faction flip would be a silent desync — registered
        // ahead of the first mutating feature (Phase 5 resource-coverage pass).
        .rollback_resource_clone::<ambition_combat::targeting::FactionRelations>(
            ENGINE,
            "resource.faction_relations",
        )
        .rollback_resource_clone::<ambition_combat::targeting::FriendlyFire>(
            ENGINE,
            "resource.friendly_fire",
        )
        // Cross-frame FIFO: victim-side hits staged in `Combat`, drained by
        // `apply_player_hit_events` in the NEXT frame's `PlayerSimulation` —
        // same shape as `SwitchActivationQueue` above. Found by the Phase-5
        // exit oracle: as a message buffer this was cleared on LoadWorld, so a
        // rewind between the strike and the victim resolver un-hit the player.
        // Checksummed (entity-free projection) so a diverged queue trips the
        // sync test at the staging frame, not a frame later as applied damage.
        .rollback_resource_clone_checksum::<ambition_combat::events::PendingPlayerHitEvents>(
            ENGINE,
            "resource.pending_player_hit_events",
            "bevy_ggrs clone snapshot + entity-free staged-hit checksum projection",
            pending_player_hits_checksum,
        )
        .rollback_resource_map_entities::<ambition_combat::events::PendingPlayerHitEvents>(
            ENGINE,
            "map.resource.pending_player_hit_events",
        );

    // Core body state.
    app.rollback_component_canonical::<ambition_platformer_primitives::sim_id::SimId>(
        ENGINE,
        "entity.sim_id",
    )
    .rollback_component_canonical::<ambition_platformer_primitives::body::BodyKinematics>(
        ENGINE,
        "body.kinematics",
    )
    .rollback_component_canonical::<ambition_characters::actor::BodyHealth>(ENGINE, "body.health")
    .rollback_component_canonical::<ambition_platformer_primitives::sim_id::SimIdCounter>(
        ENGINE,
        "body.sim_id_counter",
    )
    // Provenance and the construction-ownership stamp travel with the entity,
    // so a blob-rebuilt body can still say where it came from — and which room
    // transaction owns it — when nothing around it can. Every planned family
    // carries both since Phase 4; losing the stamp across a rewind would read
    // as OwnershipLost at the next boundary verification.
    .rollback_component_canonical::<ambition_platformer_primitives::construction::TransactionId>(
        ENGINE,
        "component.construction_transaction_id",
    )
    .rollback_component_canonical::<ambition_platformer_primitives::construction::SpawnOrigin>(
        ENGINE,
        "entity.spawn_origin",
    )
    .rollback_component_canonical::<bc::BodyAbilities>(ENGINE, "body.abilities")
    .rollback_component_canonical::<bc::BodyGroundState>(ENGINE, "body.ground")
    .rollback_component_canonical::<bc::BodyWallState>(ENGINE, "body.wall")
    .rollback_component_canonical::<bc::BodyJumpState>(ENGINE, "body.jump")
    .rollback_component_canonical::<bc::BodyDashState>(ENGINE, "body.dash")
    .rollback_component_canonical::<bc::BodyFlightState>(ENGINE, "body.flight")
    .rollback_component_canonical::<bc::BodyBlinkState>(ENGINE, "body.blink")
    .rollback_component_canonical::<bc::BodyDodgeState>(ENGINE, "body.dodge")
    .rollback_component_canonical::<bc::BodyShieldState>(ENGINE, "body.shield")
    .rollback_component_canonical::<bc::BodyOffense>(ENGINE, "body.offense")
    .rollback_component_canonical::<bc::BodyLifetime>(ENGINE, "body.lifetime")
    .rollback_component_canonical::<bc::BodyActionBuffer>(ENGINE, "body.action_buffer")
    .rollback_component_canonical::<bc::BodyBaseSize>(ENGINE, "body.base_size")
    .rollback_component_canonical::<bc::SweepSample>(ENGINE, "body.sweep_sample")
    .rollback_component_canonical::<bc::BodyMana>(ENGINE, "body.mana");

    // In-flight strike volumes — the components on the `entity:hitbox` family
    // (see the require_rollback anchor above). Clone restore + entity mapping;
    // the hit-once sets are dedup truth whose loss re-lands landed hits, and
    // `StrikeVolume` is the owner/window key `retire_orphaned_strike_volumes`
    // reconciles against restored `MovePlayback.live_boxes`.
    app.rollback_component_clone::<ambition_vfx::Hitbox>(ENGINE, "combat.hitbox")
        .rollback_map_entities::<ambition_vfx::Hitbox>(ENGINE, "map.hitbox")
        .rollback_component_clone::<ambition_vfx::HitboxHits>(ENGINE, "combat.hitbox_hits")
        .rollback_map_entities::<ambition_vfx::HitboxHits>(ENGINE, "map.hitbox_hits")
        .rollback_component_clone::<ambition_vfx::HitboxLifetime>(ENGINE, "combat.hitbox_lifetime")
        .rollback_component_clone::<ambition_combat::moveset::StrikeVolume>(
            ENGINE,
            "combat.strike_volume",
        )
        .rollback_map_entities::<ambition_combat::moveset::StrikeVolume>(
            ENGINE,
            "map.strike_volume",
        )
        .rollback_component_clone::<ambition_combat::on_hit::HitboxOnHit>(
            ENGINE,
            "combat.hitbox_on_hit",
        )
        .rollback_map_entities::<ambition_combat::on_hit::HitboxOnHit>(ENGINE, "map.hitbox_on_hit");

    // Actor, combat, and brain state.
    app.rollback_component_canonical::<ambition_combat::components::BodyMelee>(ENGINE, "actor.body_melee")
        .rollback_component_canonical::<ambition_combat::components::ActorDisposition>(
            ENGINE,
            "actor.disposition",
        )
        .rollback_component_cursor::<ambition_combat::components::ActorAggression>(
            ENGINE,
            "actor.aggression",
        )
        .rollback_map_entities::<ambition_combat::components::ActorAggression>(
            ENGINE,
            "map.actor_aggression",
        )
        .rollback_component_canonical::<ambition_characters::actor::pose::ActorPose>(
            ENGINE,
            "actor.pose",
        )
        .rollback_component_canonical::<ambition_characters::actor::WornCharacter>(
            ENGINE,
            "actor.worn_character",
        )
        // Armor rows are SPENT by `resolve_body_hit`, so this is mutable combat
        // truth, not authored loadout: without it a rewind re-spends armor that
        // an abandoned future consumed (or keeps armor the confirmed timeline
        // already used up). `WornCharacter` was registered and this was not —
        // an oversight, not a policy split (deep review 2026-07-19 §2.2).
        .rollback_component_clone::<ambition_characters::equipment::WornEquipment>(
            ENGINE,
            "actor.worn_equipment",
        )
        .rollback_component_canonical::<ambition_platformer_primitives::orientation::ActorRoll>(
            ENGINE,
            "actor.roll",
        )
        .rollback_component_canonical::<ambition_combat::components::ActorCooldowns>(
            ENGINE,
            "actor.cooldowns",
        )
        .rollback_component_canonical::<ambition_characters::actor::body::BodyCombat>(
            ENGINE,
            "actor.body_combat",
        )
        .rollback_component_canonical::<ambition_engine_core::geometry::CenteredAabb>(
            ENGINE,
            "actor.centered_aabb",
        )
        .rollback_component_cursor::<ambition_actors::features::ActorMotionPath>(
            ENGINE,
            "actor.motion_path",
        )
        .rollback_component_canonical::<bc::BodyModeState>(ENGINE, "actor.body_mode")
        .rollback_component_canonical::<ambition_actors::features::ActorStatus>(
            ENGINE,
            "actor.status",
        )
        .rollback_component_canonical::<ambition_combat::components::ActorIntent>(
            ENGINE,
            "actor.intent",
        )
        .rollback_component_cursor::<ambition_combat::components::ActorTarget>(
            ENGINE,
            "actor.target",
        )
        .rollback_map_entities::<ambition_combat::components::ActorTarget>(
            ENGINE,
            "map.actor_target",
        )
        .rollback_component_resolved::<ambition_combat::moveset::MovePlayback>(
            ENGINE,
            "actor.move_playback",
        )
        .rollback_map_entities::<ambition_combat::moveset::MovePlayback>(
            ENGINE,
            "map.move_playback",
        )
        .rollback_component_canonical::<ambition_combat::components::BossPatternTimer>(
            ENGINE,
            "boss.pattern_timer",
        )
        .rollback_component_canonical::<ambition_combat::components::BossPhase>(
            ENGINE,
            "boss.phase",
        )
        .rollback_component_canonical::<ambition_characters::brain::boss_pattern::BossAttackState>(
            ENGINE,
            "boss.attack_state",
        )
        .rollback_component_canonical::<ambition_characters::brain::boss_pattern::BossAttackIntent>(
            ENGINE,
            "boss.attack_intent",
        )
        .rollback_component_canonical::<ambition_actors::features::ecs::perception::Perception>(
            ENGINE,
            "actor.perception",
        )
        .rollback_component_canonical::<ambition_actors::features::ecs::perception::PerceptionMemory>(
            ENGINE,
            "actor.perception_memory",
        )
        .rollback_component_cursor::<ambition_characters::brain::Brain>(ENGINE, "actor.brain")
        .rollback_component_canonical::<ambition_characters::actor::character_catalog::BrainBinding>(
            ENGINE,
            "actor.brain_binding",
        )
        .rollback_component_canonical::<ambition_characters::actor::character_catalog::AuthoredBrainContext>(
            ENGINE,
            "actor.authored_brain_context",
        )
        .rollback_component_canonical::<ambition_actors::features::TemporaryControl>(
            ENGINE,
            "actor.temporary_control",
        )
        .rollback_component_canonical::<ambition_actors::features::ActorSurfaceState>(
            ENGINE,
            "actor.surface_state",
        )
        .rollback_component_canonical::<ambition_combat::components::BodyEnvelope>(
            ENGINE,
            "actor.body_envelope",
        )
        .rollback_component_canonical::<bc::BodyLedgeState>(ENGINE, "actor.ledge")
        .rollback_component_canonical::<ambition_engine_core::MotionModel>(ENGINE, "actor.motion_model")
        .rollback_component_canonical::<bc::BodyComboTrace>(ENGINE, "actor.combo_trace")
        .rollback_component_canonical::<ambition_characters::brain::ActorControl>(
            ENGINE,
            "actor.control",
        )
        .rollback_component_canonical::<ambition_time::ProperTimeScale>(
            ENGINE,
            "actor.proper_time_scale",
        )
        .rollback_component_cursor::<ambition_actors::features::BossEncounter>(
            ENGINE,
            "boss.encounter",
        );

    // Complete rollback entity shapes. The old custom restore engine only
    // patched a narrow state subset and left the remaining components stale.
    // GGRS recreates entities, so every marker, authored/config component, and
    // mutable controller that a recreated actor needs is explicitly stored.
    app.rollback_component_canonical::<ambition_actors::avatar::PlayerSafetyState>(
        ENGINE,
        "player.safety_state",
    )
    .rollback_component_canonical::<ambition_characters::actor::BodyWallet>(ENGINE, "body.wallet")
    .rollback_component_canonical::<ambition_projectiles::PlayerProjectileState>(
        ENGINE,
        "player.projectile_state",
    )
    .rollback_component_clone::<ambition_actors::avatar::PlayerBlinkCameraState>(
        ENGINE,
        "player.blink_camera_state",
    )
    .rollback_component_clone::<ambition_actors::control::LocalPlayer>(
        ENGINE,
        "player.local_marker",
    )
    .rollback_component_clone::<ambition_actors::features::ActorConfig>(ENGINE, "actor.config")
    .rollback_component_clone::<ambition_actors::features::BossConfig>(ENGINE, "boss.config")
    .rollback_component_clone::<ambition_actors::features::LimbRig>(ENGINE, "limb.rig")
    .rollback_map_entities::<ambition_actors::features::LimbRig>(ENGINE, "map.limb_rig")
    .rollback_component_clone::<ambition_actors::features::Limb>(ENGINE, "limb.member")
    .rollback_map_entities::<ambition_actors::features::Limb>(ENGINE, "map.limb_member")
    .rollback_component_clone::<ambition_actors::features::LimbRouteState>(
        ENGINE,
        "limb.route_state",
    )
    .rollback_component_clone::<ambition_actors::features::LimbIntents>(ENGINE, "limb.intents")
    .rollback_component_clone::<ambition_actors::features::CanPilot>(ENGINE, "mount.can_pilot")
    .rollback_component_clone::<ambition_actors::features::Mass>(ENGINE, "mount.mass")
    .rollback_component_clone::<ambition_actors::features::MountSlot>(ENGINE, "mount.slot")
    .rollback_map_entities::<ambition_actors::features::MountSlot>(ENGINE, "map.mount_slot")
    .rollback_component_clone::<ambition_actors::features::Mountable>(ENGINE, "mount.mountable")
    .rollback_component_clone::<ambition_actors::features::Mounted>(ENGINE, "mount.mounted")
    .rollback_component_clone::<ambition_actors::features::RidingOn>(ENGINE, "mount.riding_on")
    .rollback_map_entities::<ambition_actors::features::RidingOn>(ENGINE, "map.riding_on")
    .rollback_component_clone::<ambition_actors::features::BossOverrides>(ENGINE, "boss.overrides")
    .rollback_component_clone::<ambition_actors::items::pickup::StashedActionSet>(
        ENGINE,
        "actor.stashed_action_set",
    )
    .rollback_component_clone::<ambition_characters::actor::BodyAnimFacts>(
        ENGINE,
        "actor.animation_facts",
    )
    .rollback_component_clone::<ambition_characters::actor::ActorFaction>(ENGINE, "actor.faction")
    .rollback_component_clone::<ambition_characters::brain::ChargesProjectiles>(
        ENGINE,
        "actor.charges_projectiles",
    )
    .rollback_component_clone::<ambition_characters::brain::PlayerSlot>(ENGINE, "actor.player_slot")
    .rollback_component_clone::<ambition_characters::brain::ActionSet>(ENGINE, "actor.action_set")
    .rollback_component_clone::<ambition_characters::brain::BossCapability>(
        ENGINE,
        "boss.capability",
    )
    .rollback_component_clone::<ambition_combat::components::CombatCapabilities>(
        ENGINE,
        "combat.capabilities",
    )
    .rollback_component_clone::<ambition_combat::components::CombatTuning>(ENGINE, "combat.tuning")
    .rollback_component_clone::<ambition_combat::components::ActorIdentity>(
        ENGINE,
        "actor.identity",
    )
    .rollback_component_clone::<ambition_combat::components::ActorInteraction>(
        ENGINE,
        "actor.interaction",
    )
    .rollback_component_clone::<ambition_combat::components::ActorRenderSize>(
        ENGINE,
        "actor.render_size",
    )
    .rollback_component_clone::<ambition_combat::components::BossDeathAnimation>(
        ENGINE,
        "boss.death_animation",
    )
    .rollback_component_clone::<ambition_combat::components::CombatKit>(ENGINE, "combat.kit")
    .rollback_component_clone::<ambition_combat::components::DamageableVolumes>(
        ENGINE,
        "feature.damageable_volumes",
    )
    .rollback_component_clone::<ambition_combat::components::FeatureId>(ENGINE, "feature.id")
    .rollback_component_clone::<ambition_combat::components::FeatureName>(ENGINE, "feature.name")
    // World features that MUTATE during play (deep review 2026-07-19 §2.2).
    // Without these a brick broken in an abandoned future stays broken through
    // the rewind, and the crumble/respawn countdowns resume from predicted
    // values instead of confirmed ones.
    .rollback_component_clone::<ambition_combat::components::BreakableFeature>(
        ENGINE,
        "feature.breakable",
    )
    .rollback_component_clone::<ambition_combat::components::RespawnTimer>(
        ENGINE,
        "feature.respawn_timer",
    )
    .rollback_component_clone::<ambition_combat::components::StandTimer>(
        ENGINE,
        "feature.stand_timer",
    )
    .rollback_component_clone::<ambition_combat::hazard_runtime::HazardFeature>(
        ENGINE,
        "feature.hazard",
    )
    // Switch liveness. The `SwitchActivated` MESSAGE is cleared on rollback, but
    // the state that message produced was not rewound — so a switch flipped in an
    // abandoned future stayed on.
    .rollback_component_clone::<ambition_actors::encounter::SwitchOn>(ENGINE, "feature.switch_on")
    // The switch's authored payload. Immutable at runtime, but bevy_ggrs
    // DESTROYS AND RECREATES rollback entities — anything not registered is
    // simply absent on the recreated entity, so an unregistered authored
    // component silently strips the switch of its identity after a rewind.
    .rollback_component_clone::<ambition_actors::encounter::SwitchFeature>(ENGINE, "feature.switch")
    // Same reasoning for the room-visual lifecycle tag: its siblings
    // (`RoomScopedEntity`, `SessionScopedEntity`) are registered, and losing the
    // tag on recreation would leak the entity past its room's teardown.
    .rollback_component_clone::<ambition_platformer_primitives::lifecycle::RoomVisual>(
        ENGINE,
        "lifecycle.room_visual",
    )
    .rollback_component_clone::<ambition_combat::components::PogoPolicy>(
        ENGINE,
        "feature.pogo_policy",
    )
    .rollback_component_clone::<ambition_combat::components::PogoTargetVolumes>(
        ENGINE,
        "feature.pogo_target_volumes",
    )
    .rollback_component_clone::<ambition_combat::held_items::HeldItem>(ENGINE, "actor.held_item")
    .rollback_component_clone::<ambition_combat::moveset::ActorMoveset>(ENGINE, "actor.moveset")
    .rollback_component_clone::<ambition_combat::moveset::MovesetMelee>(
        ENGINE,
        "actor.moveset_melee",
    )
    // The ranged sibling, and the pickup/solid-contributor features — found by
    // the combat-calibration-lab coverage sweep (the boot room has no ranged
    // enemy, no pickups, and no breakable, so the boot-room sweep could not
    // see them). Same recreated-entity reasoning as `SwitchFeature` above.
    .rollback_component_clone::<ambition_characters::brain::MovesetRanged>(
        ENGINE,
        "actor.moveset_ranged",
    )
    .rollback_component_clone::<ambition_combat::components::PickupFeature>(
        ENGINE,
        "feature.pickup",
    )
    // The collected latch. Unregistered, a rewind past a collection could not
    // REMOVE it: the resimulated pickup started already-collected, the magnet
    // skipped it (`Without<Collected>`), and its registered `CenteredAabb`
    // froze while the first pass had it moving — the exit oracle's first
    // checksum divergence (combat_calibration_lab, frames 10–12).
    .rollback_component_clone::<ambition_combat::components::Collected>(ENGINE, "feature.collected")
    // The mid-toss collection lock (a scattered ring's uncollectible window),
    // registered for the SAME reason `Collected` is: a rewind past the lock's
    // removal must restore it, or the resimulated ring would be collectible a
    // frame early — the magnet/collect guards read it, so it is authoritative.
    .rollback_component_clone::<ambition_actors::features::PickupCollectLock>(
        ENGINE,
        "feature.pickup_collect_lock",
    )
    .rollback_component_clone::<ambition_combat::components::SandboxSolidContributor>(
        ENGINE,
        "feature.sandbox_solid_contributor",
    )
    .rollback_component_clone::<ambition_encounter::Encounter>(ENGINE, "encounter.identity")
    .rollback_component_clone::<ambition_encounter::EncounterObjective>(
        ENGINE,
        "encounter.objective",
    )
    .rollback_component_clone::<ambition_encounter::EncounterCameraZoom>(
        ENGINE,
        "encounter.camera_zoom",
    )
    .rollback_component_clone::<ambition_encounter::EncounterLockWall>(
        ENGINE,
        "encounter.lock_wall",
    )
    .rollback_component_clone::<ambition_encounter::EncounterTrack>(ENGINE, "encounter.track")
    .rollback_component_clone::<ambition_engine_core::body_clusters::AbilityBase>(
        ENGINE,
        "body.ability_base",
    )
    .rollback_component_clone::<ambition_platformer_primitives::body::PrimaryBody>(
        ENGINE,
        "marker.primary_body",
    )
    .rollback_component_clone::<ambition_platformer_primitives::lifecycle::FeatureSimEntity>(
        ENGINE,
        "marker.feature_sim_entity",
    )
    .rollback_component_clone::<ambition_platformer_primitives::markers::PlayerEntity>(
        ENGINE,
        "marker.player_entity",
    )
    .rollback_component_clone::<ambition_platformer_primitives::markers::PrimaryPlayer>(
        ENGINE,
        "marker.primary_player",
    )
    .rollback_component_clone::<ambition_portal::PortalBody>(ENGINE, "portal.body")
    .rollback_component_clone::<ambition_portal::PortalPolicy>(ENGINE, "portal.policy")
    .rollback_component_clone::<ambition_portal::PortalTransit>(ENGINE, "portal.transit")
    .rollback_component_clone::<ambition_portal::PlacedPortal>(ENGINE, "portal.placed")
    // Portal-gun runtime (deep review 2026-07-19 §2.2). `PortalBody`/`Policy`/
    // `Transit`/`PlacedPortal` were registered but the gun-side state was not,
    // so a rewind could carry a cooldown latch or an in-flight shot in from an
    // abandoned future — permitting or blocking a transit the confirmed
    // timeline never saw.
    .rollback_component_clone::<ambition_portal::PortalTransitCooldown>(
        ENGINE,
        "portal.transit_cooldown",
    )
    .rollback_component_clone::<ambition_portal::PortalEmission>(ENGINE, "portal.emission")
    .rollback_component_clone::<ambition_portal::PortalShot>(ENGINE, "portal.shot")
    .rollback_component_clone::<ambition_portal::PortalGun>(ENGINE, "portal.gun")
    .rollback_component_clone::<ambition_actors::items::pickup::GroundItem>(
        ENGINE,
        "item.ground_item",
    )
    .rollback_component_clone::<ambition_actors::gravity::GravityFlipSwitch>(
        ENGINE,
        "gravity.flip_switch",
    )
    .rollback_component_clone::<ambition_actors::boss_encounter::EncounterDef>(
        ENGINE,
        "encounter.definition",
    )
    .rollback_component_clone::<ambition_actors::features::MountedBrainCache>(
        ENGINE,
        "mount.brain_cache",
    )
    .rollback_component_clone::<ambition_actors::features::MountedSize>(
        ENGINE,
        "mount.authored_size",
    )
    .rollback_component_clone::<bevy::prelude::Name>(ENGINE, "entity.name")
    .rollback_component_clone::<bevy::prelude::Transform>(ENGINE, "entity.transform");

    // These values are guaranteed to be republished before any downstream
    // consumer in each GGRS frame, so storing them would duplicate authority.
    app.declare_rollback_derived::<ambition_actors::avatar::body_integration::PlayerBodyFrameOutput>(
        ENGINE,
        "derived.player_body_frame_output",
        "republished by body integration every simulation frame",
    )
    .declare_rollback_derived::<ambition_actors::body_mode::BodyModeCapabilities>(
        ENGINE,
        "derived.body_mode_capabilities",
        "projected from the active body mode each frame",
    )
    .declare_rollback_derived::<ambition_actors::control::PlayerInputFrame>(
        ENGINE,
        "derived.player_input_frame",
        "copied from GGRS PlayerInputs at the head of every frame",
    )
    .declare_rollback_derived::<ambition_characters::action_scheme::ActorActionScheme>(
        ENGINE,
        "derived.actor_action_scheme",
        "reconciled from abilities, moveset, and action set",
    )
    .declare_rollback_derived::<ambition_characters::action_scheme::ResolvedTechniqueEdges>(
        ENGINE,
        "derived.resolved_technique_edges",
        "cleared and republished from current input every frame",
    )
    .declare_rollback_derived::<bevy::prelude::GlobalTransform>(
        ENGINE,
        "derived.global_transform",
        "Bevy transform propagation rebuilds it from Transform and hierarchy",
    )
    .declare_rollback_derived::<ambition_actors::features::ActorSteering>(
        ENGINE,
        "derived.actor_steering",
        "rebuilt from the authoritative actor population before movement",
    )
    .declare_rollback_derived::<ambition_characters::brain::SlotControls>(
        ENGINE,
        "derived.slot_controls",
        "republished from GGRS PlayerInputs at the head of every frame",
    )
    .declare_rollback_derived::<ambition_platformer_primitives::markers::ControlledSubject>(
        ENGINE,
        "derived.controlled_subject",
        "resolved from the entity carrying Brain::Player for the active slot",
    )
    .declare_rollback_derived::<ambition_portal::PlayerMovementIntent>(
        ENGINE,
        "derived.portal_player_movement_intent",
        "republished from the current controller frame before portal transit",
    )
    .declare_rollback_derived::<ambition_portal::PortalCarves>(
        ENGINE,
        "derived.portal_carves",
        "rebuilt from placed portals and transit occupancy each frame",
    )
    .declare_rollback_derived::<ambition_platformer_primitives::class_b::ClassBRemapLog>(
        ENGINE,
        "derived.class_b_remap_log",
        "frame-local diagnostic ledger cleared before every simulation step",
    )
    .declare_rollback_derived::<ambition_platformer_primitives::gravity::GravityZones>(
        ENGINE,
        "derived.gravity_zones",
        "rebuilt from authoritative GravityZone components before body integration",
    )
    .declare_rollback_derived::<ambition_portal::PortalHostDepths>(
        ENGINE,
        "derived.portal_host_depths",
        "republished from the authoritative collision world each frame",
    );

    // Scope, projectile, and encounter state.
    app.rollback_component_canonical::<ambition_platformer_primitives::lifecycle::RoomScopedEntity>(
        ENGINE,
        "scope.room",
    )
    .rollback_component_canonical::<ambition_platformer_primitives::lifecycle::SessionScopedEntity>(
        ENGINE,
        "scope.session",
    )
    .rollback_component_canonical::<ambition_platformer_primitives::projectile::ProjectileGameplay>(
        ENGINE,
        "projectile.gameplay",
    )
    .rollback_component_canonical::<ambition_projectiles::ProjectileSeq>(ENGINE, "projectile.seq")
    .rollback_component_canonical::<ambition_projectiles::ProjectileOwnerId>(
        ENGINE,
        "projectile.owner_id",
    )
    .rollback_component_canonical::<ambition_projectiles::ProjectileVisualId>(
        ENGINE,
        "projectile.visual_id",
    )
    .rollback_component_canonical::<ambition_projectiles::ProjectileKind>(
        ENGINE,
        "projectile.kind",
    )
    .rollback_component_canonical::<ambition_projectiles::LiveProjectile>(
        ENGINE,
        "projectile.live_marker",
    )
    .rollback_component_canonical::<ambition_projectiles::PlayerProjectile>(
        ENGINE,
        "projectile.player_marker",
    )
    .rollback_component_canonical::<ambition_projectiles::enemy::EnemyProjectile>(
        ENGINE,
        "projectile.enemy_marker",
    )
    .rollback_component_canonical::<ambition_encounter::EncounterLifecycle>(
        ENGINE,
        "encounter.lifecycle",
    )
    .rollback_component_clone_state::<ambition_encounter::EncounterParticipants>(
        ENGINE,
        "encounter.participants",
    )
    .rollback_map_entities::<ambition_encounter::EncounterParticipants>(
        ENGINE,
        "map.encounter_participants",
    )
    .rollback_component_resolved::<ambition_encounter::EncounterWaves>(
        ENGINE,
        "encounter.waves",
    );

    // Derived state: one maintenance path, never restore-only repair code.
    // NOTE this justification was wrong until 2026-07-22: it named
    // `ProjectileOwnerId`, which is the firer's raw config id and is EMPTY for
    // every player projectile, so it could not have carried the owner identity
    // for the largest pool in the game. The handle was actually recovered by
    // splitting the projectile's own `SimId` on `/`. It is now recovered from
    // declared provenance, which is what this line always claimed in spirit.
    app.declare_rollback_derived::<ambition_projectiles::ProjectileOwner>(
        ENGINE,
        "derived.projectile_owner",
        "re-resolved from SpawnOrigin::Dynamic { parent } by the ordinary identity maintenance system",
    )
    .declare_rollback_derived::<ambition_engine_core::body_clusters::BodyEnvironmentContact>(
        ENGINE,
        "derived.body_environment_contact",
        "rewritten every movement step from body geometry and the live world",
    )
    .declare_rollback_derived::<ambition_platformer_primitives::frame_env::ResolvedMotionFrame>(
        ENGINE,
        "derived.resolved_motion_frame",
        "published every tick from the live environment",
    )
    .declare_rollback_derived::<ambition_engine_core::BodyMotionFacts>(
        ENGINE,
        "derived.body_motion_facts",
        "republished from MotionModel every movement step",
    )
    .declare_rollback_derived::<ambition_platformer_primitives::orientation::SurfaceUpright>(
        ENGINE,
        "derived.surface_upright",
        "republished from support facts every movement step",
    )
    .declare_rollback_derived::<ambition_sim_view::BodyPoseView>(
        ENGINE,
        "derived.body_pose_view",
        "SimView projection rebuilt every tick",
    )
    .declare_rollback_derived::<ambition_sim_view::ProjectileView>(
        ENGINE,
        "derived.projectile_view",
        "SimView projection rebuilt every tick",
    )
    .declare_rollback_derived::<ambition_actors::boss_encounter::EncounterProgress>(
        ENGINE,
        "derived.encounter_progress",
        "recomputed from lifecycle and participant health every tick",
    )
    // Frame-derived RESOURCES (Phase 5 resource-coverage pass): each is
    // republished by its ordinary maintenance system before anything reads it,
    // so a rewind that keeps a stale value is overwritten before it matters.
    .declare_rollback_derived::<ambition_engine_core::control_frame::ControlFrame>(
        ENGINE,
        "derived.control_frame",
        "per-tick input frame regenerated from the synchronized input stream",
    )
    .declare_rollback_derived::<ambition_platformer_primitives::frame_env::ForceZones>(
        ENGINE,
        "derived.force_zones",
        "per-tick zone snapshot rebuilt by collect_force_zones",
    )
    .declare_rollback_derived::<ambition_platformer_primitives::feature_overlay::FeatureEcsWorldOverlay>(
        ENGINE,
        "derived.feature_ecs_world_overlay",
        "collision contributions rebuilt from ECS feature state every tick",
    )
    .declare_rollback_derived::<ambition_actors::features::ecs::perception::PerceptionPeers>(
        ENGINE,
        "derived.perception_peers",
        "perception snapshot rebuilt every tick before brains read it",
    )
    .declare_rollback_derived::<ambition_actors::features::ecs::perception::PerceptionProjectiles>(
        ENGINE,
        "derived.perception_projectiles",
        "perception snapshot rebuilt every tick before brains read it",
    )
    .declare_rollback_derived::<ambition_actors::encounter::EncounterSwitchIndex>(
        ENGINE,
        "derived.encounter_switch_index",
        "rebuilt from SwitchFeature + SwitchOn components each frame",
    )
    .declare_rollback_derived::<ambition_encounter::entity::EncounterView>(
        ENGINE,
        "derived.encounter_view",
        "presentation-intent read model republished each tick",
    )
    .declare_rollback_derived::<ambition_actors::affordances::PlayerAffordances>(
        ENGINE,
        "derived.player_affordances",
        "affordance read model recomputed per frame from body state",
    )
    .declare_rollback_derived::<ambition_actors::affordances::intent::PlayerIntent>(
        ENGINE,
        "derived.player_intent",
        "affordance read model recomputed per frame from control input",
    )
    .declare_rollback_derived::<ambition_actors::affordances::interactable_proximity::NearestInteractable>(
        ENGINE,
        "derived.nearest_interactable",
        "proximity read model recomputed per frame",
    )
    .declare_rollback_derived::<ambition_actors::affordances::pogo_proximity::PogoTargetBelow>(
        ENGINE,
        "derived.pogo_target_below",
        "proximity read model recomputed per frame",
    )
    .declare_dynamic_anchor::<ambition_platformer_primitives::projectile::ProjectileGameplay>(
        ENGINE,
        "dynamic.projectile",
        "Rollback entity recreation plus the complete projectile component family",
    );

    // Abandoned-future transient ingress must be empty after LoadWorld. Replayed
    // inputs and deterministic systems regenerate the correct messages.
    app.clear_message_on_rollback::<ambition_characters::brain::ActorActionMessage>(
        ENGINE,
        "message.actor_action",
    )
    .clear_message_on_rollback::<ambition_combat::events::HitEvent>(ENGINE, "message.hit_event")
    .clear_message_on_rollback::<ambition_combat::on_hit::OnHitEffectMessage>(
        ENGINE,
        "message.on_hit_effect",
    )
    .clear_message_on_rollback::<ambition_combat::moveset::MoveEventMessage>(
        ENGINE,
        "message.move_event",
    )
    .clear_message_on_rollback::<ambition_encounter::EncounterCommand>(
        ENGINE,
        "message.encounter_command",
    )
    .clear_message_on_rollback::<ambition_encounter::EncounterEventMsg>(
        ENGINE,
        "message.encounter_event",
    )
    .clear_message_on_rollback::<ambition_actors::features::BrainCommand>(
        ENGINE,
        "message.brain_command",
    )
    .clear_message_on_rollback::<ambition_actors::features::ReleaseProvocation>(
        ENGINE,
        "message.release_provocation",
    )
    .clear_message_on_rollback::<ambition_world::rooms::RoomLoaded>(ENGINE, "message.room_loaded")
    .clear_message_on_rollback::<ambition_actors::features::SpawnActorRequest>(
        ENGINE,
        "message.spawn_actor_request",
    )
    .clear_message_on_rollback::<ambition_world::rooms::RoomTransitionRequested>(
        ENGINE,
        "message.room_transition_requested",
    )
    .clear_message_on_rollback::<ambition_actors::ActorDiedMessage>(ENGINE, "message.actor_died")
    .clear_message_on_rollback::<ambition_actors::avatar::PlayerHealRequested>(
        ENGINE,
        "message.player_heal_requested",
    )
    .clear_message_on_rollback::<ambition_actors::avatar::trail::TrailContinuityBreak>(
        ENGINE,
        "message.trail_continuity_break",
    )
    .clear_message_on_rollback::<ambition_actors::boss_encounter::PayloadReleased>(
        ENGINE,
        "message.payload_released",
    )
    .clear_message_on_rollback::<ambition_encounter::EncounterGate>(
        ENGINE,
        "message.encounter_gate",
    )
    .clear_message_on_rollback::<ambition_actors::encounter::SwitchActivated>(
        ENGINE,
        "message.switch_activated",
    )
    .clear_message_on_rollback::<ambition_actors::features::MountDied>(ENGINE, "message.mount_died")
    .clear_message_on_rollback::<ambition_actors::session::reset::RoomReplayRequested>(
        ENGINE,
        "message.room_replay_requested",
    )
    .clear_message_on_rollback::<ambition_actors::time::time_control::ClockResetRequest>(
        ENGINE,
        "message.clock_reset_request",
    )
    .clear_message_on_rollback::<ambition_actors::time::time_control::ClockScaleRequest>(
        ENGINE,
        "message.clock_scale_request",
    )
    .clear_message_on_rollback::<ambition_combat::events::ActorStimulus>(
        ENGINE,
        "message.actor_stimulus",
    )
    .clear_message_on_rollback::<ambition_combat::events::GameplayBannerRequested>(
        ENGINE,
        "message.gameplay_banner_requested",
    )
    .clear_message_on_rollback::<ambition_combat::events::GameplaySfxRequested>(
        ENGINE,
        "message.gameplay_sfx_requested",
    )
    .clear_message_on_rollback::<ambition_combat::events::ResetRoomFeaturesEvent>(
        ENGINE,
        "message.reset_room_features",
    )
    .clear_message_on_rollback::<ambition_combat::events::SetFlagRequested>(
        ENGINE,
        "message.set_flag_requested",
    )
    .clear_message_on_rollback::<ambition_persistence::quest::QuestAdvanceRequested>(
        ENGINE,
        "message.quest_advance_requested",
    )
    .clear_message_on_rollback::<ambition_portal::ClearPortals>(ENGINE, "message.clear_portals")
    .clear_message_on_rollback::<ambition_portal::DropPortalGun>(ENGINE, "message.drop_portal_gun")
    .clear_message_on_rollback::<ambition_portal::FirePortalGun>(ENGINE, "message.fire_portal_gun")
    .clear_message_on_rollback::<ambition_portal::PickUpPortalGun>(
        ENGINE,
        "message.pick_up_portal_gun",
    )
    .clear_message_on_rollback::<ambition_portal::PortalBodyEntered>(
        ENGINE,
        "message.portal_body_entered",
    )
    .clear_message_on_rollback::<ambition_portal::PortalFireIntent>(
        ENGINE,
        "message.portal_fire_intent",
    )
    .clear_message_on_rollback::<ambition_portal::PortalGunEquipped>(
        ENGINE,
        "message.portal_gun_equipped",
    )
    .clear_message_on_rollback::<ambition_portal::PortalShotFired>(
        ENGINE,
        "message.portal_shot_fired",
    )
    .clear_message_on_rollback::<ambition_portal::TogglePortalGun>(
        ENGINE,
        "message.toggle_portal_gun",
    )
    .clear_message_on_rollback::<ambition_portal::BodyTeleported>(ENGINE, "message.body_teleported")
    .clear_message_on_rollback::<ambition_portal::PortalBodyTransited>(
        ENGINE,
        "message.portal_body_transited",
    )
    .clear_message_on_rollback::<ambition_projectiles::SpawnProjectile>(
        ENGINE,
        "message.spawn_projectile",
    )
    .clear_message_on_rollback::<ambition_sfx::OwnedSfxMessage>(ENGINE, "message.owned_sfx")
    .clear_message_on_rollback::<ambition_vfx::EffectRequest>(ENGINE, "message.effect_request")
    .clear_message_on_rollback::<ambition_vfx::vfx::DebrisBurstMessage>(
        ENGINE,
        "message.debris_burst",
    )
    .clear_message_on_rollback::<ambition_vfx::ExplosionRequest>(
        ENGINE,
        "message.explosion_request",
    )
    .clear_message_on_rollback::<ambition_vfx::FireworksRequest>(
        ENGINE,
        "message.fireworks_request",
    )
    .clear_message_on_rollback::<ambition_vfx::VfxMessage>(ENGINE, "message.vfx")
    .clear_message_on_rollback::<ambition_world::rooms::RespawnRoomVisualsRequested>(
        ENGINE,
        "message.respawn_room_visuals",
    )
    // Phase 5 resource-coverage pass: the remaining sim-facing buffers the
    // computed audit surfaced. Same policy as every entry above — empty after
    // LoadWorld, regenerated by replayed inputs.
    .clear_message_on_rollback::<ambition_actors::encounter::SwitchActivated>(
        ENGINE,
        "message.switch_activated",
    )
    .clear_message_on_rollback::<ambition_actors::features::MountDied>(ENGINE, "message.mount_died")
    .clear_message_on_rollback::<ambition_actors::session::reset::RoomReplayRequested>(
        ENGINE,
        "message.room_replay_requested",
    )
    .clear_message_on_rollback::<ambition_actors::time::time_control::ClockResetRequest>(
        ENGINE,
        "message.clock_reset_request",
    )
    .clear_message_on_rollback::<ambition_actors::time::time_control::ClockScaleRequest>(
        ENGINE,
        "message.clock_scale_request",
    )
    .clear_message_on_rollback::<ambition_combat::events::ActorStimulus>(
        ENGINE,
        "message.actor_stimulus",
    )
    .clear_message_on_rollback::<ambition_combat::events::GameplayBannerRequested>(
        ENGINE,
        "message.gameplay_banner_requested",
    )
    .clear_message_on_rollback::<ambition_combat::events::GameplaySfxRequested>(
        ENGINE,
        "message.gameplay_sfx_requested",
    )
    .clear_message_on_rollback::<ambition_combat::events::ResetRoomFeaturesEvent>(
        ENGINE,
        "message.reset_room_features",
    )
    .clear_message_on_rollback::<ambition_combat::events::SetFlagRequested>(
        ENGINE,
        "message.set_flag_requested",
    )
    .clear_message_on_rollback::<ambition_encounter::timeline::EncounterGate>(
        ENGINE,
        "message.encounter_gate",
    )
    .clear_message_on_rollback::<ambition_persistence::quest::QuestAdvanceRequested>(
        ENGINE,
        "message.quest_advance_requested",
    )
    .clear_message_on_rollback::<ambition_projectiles::spawn_message::SpawnProjectile>(
        ENGINE,
        "message.spawn_projectile",
    )
    .clear_message_on_rollback::<ambition_portal::ClearPortals>(ENGINE, "message.portal_clear")
    .clear_message_on_rollback::<ambition_portal::DropPortalGun>(ENGINE, "message.portal_gun_drop")
    .clear_message_on_rollback::<ambition_portal::FirePortalGun>(ENGINE, "message.portal_gun_fire")
    .clear_message_on_rollback::<ambition_portal::PickUpPortalGun>(
        ENGINE,
        "message.portal_gun_pick_up",
    )
    .clear_message_on_rollback::<ambition_portal::PortalBodyEntered>(
        ENGINE,
        "message.portal_body_entered",
    )
    .clear_message_on_rollback::<ambition_portal::PortalFireIntent>(
        ENGINE,
        "message.portal_fire_intent",
    )
    .clear_message_on_rollback::<ambition_portal::PortalGunEquipped>(
        ENGINE,
        "message.portal_gun_equipped",
    )
    .clear_message_on_rollback::<ambition_portal::PortalShotFired>(
        ENGINE,
        "message.portal_shot_fired",
    )
    .clear_message_on_rollback::<ambition_portal::TogglePortalGun>(
        ENGINE,
        "message.portal_gun_toggle",
    )
    .clear_message_on_rollback::<ambition_portal::BodyTeleported>(ENGINE, "message.body_teleported")
    .clear_message_on_rollback::<ambition_portal::PortalBodyTransited>(
        ENGINE,
        "message.portal_body_transited",
    );
}
