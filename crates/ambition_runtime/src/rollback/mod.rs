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

/// Installs GGRS schedules, snapshot storage, and session/request handling.
pub struct AmbitionRollbackPlugin;

impl Plugin for AmbitionRollbackPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(GgrsPlugin::<AmbitionGgrsConfig>::default())
            .insert_resource(RollbackFrameRate(crate::SIM_TICK_HZ as usize));

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
        );

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
        .rollback_resource_canonical::<ambition_actors::features::PendingMountLinks>(
            ENGINE,
            "resource.pending_mount_links",
        )
        .rollback_resource_canonical::<ambition_actors::session::reset::SandboxResetRequested>(
            ENGINE,
            "resource.sandbox_reset_requested",
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
    app.declare_rollback_derived::<ambition_projectiles::ProjectileOwner>(
        ENGINE,
        "derived.projectile_owner",
        "re-resolved from ProjectileOwnerId by the ordinary identity maintenance system",
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
    );
}
