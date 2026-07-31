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
    LoadWorldSystems, Rollback, RollbackFrameCount, RunGgrsSystems, SaveWorld,
};

/// Ambition-owned work that must run after every `bevy_ggrs` entity/data/map restore.
#[derive(SystemSet, Clone, Debug, Hash, PartialEq, Eq)]
pub enum AmbitionLoadWorldSet {
    /// Reconcile authored/runtime pairs after all raw `Entity` handles have been remapped.
    Reconcile,
}

mod codec;
mod codecs;
mod probes;
#[cfg(test)]
mod provenance_tests;
mod registry;
mod session;

pub use codec::*;
pub use codecs::{ensure_sim_id, heal_projectile_owners, mint_spawned_sim_ids};
pub use probes::*;
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
        // layer. Same set as the fixed-tick sampler `ambition_sim_view` installs
        // for itself — only the clock's hiding place differs.
        //
        // Joining `SamplePhase` is the whole contract: the set is ordered before
        // every resampler by the owning plugin. This used to name ONE resampler
        // with a `.before`, which silently left the feature/actor poses racing
        // the phase they were supposed to be resampled against.
        app.add_systems(
            Update,
            sample_ggrs_accumulator_phase
                .in_set(ambition_sim_view::presented_pose::PresentedPoseStage::SamplePhase),
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

        // ── Per-component restore localization (opt-in) ──
        //
        // Census every registered component's checksum projection at SAVE, and
        // again at LOAD of the same frame, so a divergence NAMES the component
        // instead of naming a frame. Inert unless `RollbackRestoreAudit::enabled`,
        // because censusing every registered type on every save and load is far
        // too expensive to leave on — but installed unconditionally, so a
        // diagnostic session is one resource insert away rather than a rebuild.
        app.add_systems(bevy_ggrs::SaveWorld, probes::record_saved_census);
        app.add_systems(
            bevy_ggrs::LoadWorld,
            probes::compare_restored_census.after(AmbitionLoadWorldSet::Reconcile),
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
            HitSource::LeftTheWorld => (10, 0.0),
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
    app.require_rollback::<ambition_actors::features::transform_beat::TransformBeat>(
        ENGINE,
        "entity:transform_beat",
    )
    .require_rollback::<ambition_actors::rooms::RoomSet>(ENGINE, "root:room_set")
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
    .require_rollback::<ambition_actors::items::pickup::GroundItem>(ENGINE, "entity:ground_item")
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
        // **The match activation latch.** (AA2 / AC2)
        //
        // Published from inside the sim schedule on the tick the last seat is
        // filled, and it GATES two behaviours: seating returns early while it
        // exists, and the countdown treats it as proof the match is live. Left
        // unregistered, a rewind across activation restored the fighters — or
        // un-spawned them — and left the latch pointing at a future in which
        // they existed, so seating refused to rebuild the roster it had just
        // lost while the countdown carried on.
        //
        // Correct because `bevy_ggrs` restores ABSENCE as well as value: a
        // rewind to before activation REMOVES this, seating sees no match, and
        // the roster is reconstructed from the same authored inputs.
        //
        // ⚠ **this registration is load-bearing, and there is now a fixture that
        // says so.** Remove it and
        // `rollback_match_activation::a_rewind_across_the_activation_frame_reconstructs_the_same_match`
        // fails on "the restored world was not pre-activation after all", while
        // the two older tests in that file stay green — they cannot reach a
        // pre-activation frame, and their docstring admits it.
        //
        // AA2's lifecycle half is CLOSED as of the seating transaction: seats
        // are resolved and validated before any is built, then constructed in
        // one command flush with this latch, so there is no "between two seats"
        // state a rewind can land in.
        .rollback_resource_optional_canonical::<ambition_actors::character_runtime::ActiveMatch>(
            ENGINE,
            "resource.active_match",
        )
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
        // G2b: id → live encounter entity, remapped on every load. A presence
        // probe over a singleton resource sees "still present"; this sees an id
        // pointing at the wrong encounter. Folded in the map's own (sorted) key
        // order, so a permutation between two ids is a difference.
        .rollback_resource_clone_entity_set::<ambition_encounter::EncounterRegistry>(
            ENGINE,
            "resource.encounter_registry",
            |registry| registry.ids.values().copied().collect(),
        )
        .rollback_resource_map_entities::<ambition_encounter::EncounterRegistry>(
            ENGINE,
            "map.resource.encounter_registry",
        )
        // G2b: probed through the possessed/home pair's stable identities. A
        // presence probe over a singleton resource sees "still present" and
        // nothing else — and a restore that exchanged the possessed body for the
        // home avatar would invert the whole possession while folding the same
        // census, which is why the ORDER of the pair is folded in.
        .rollback_resource_clone_entity_set::<ambition_actors::abilities::traversal::possession::PossessionState>(
            ENGINE,
            "resource.possession_state",
            |state| state.possessed.into_iter().chain(state.home).collect(),
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
    // G2b: probed through the OWNER's stable identity, paired with the hitbox's
    // own — the same treatment `ProjectileOwner` has. A strike volume remapped onto
    // the wrong body damages the wrong faction's targets, and a presence count
    // could not tell that from a correct restore.
    app.rollback_component_clone_entity_ref::<ambition_vfx::Hitbox>(
        ENGINE,
        "combat.hitbox",
        |hitbox| hitbox.owner,
    )
    .rollback_map_entities::<ambition_vfx::Hitbox>(ENGINE, "map.hitbox")
    // The victims this strike has ALREADY hit. Losing one from the set is a
    // sustained overlap re-hitting a body it already hit, which is exactly the
    // kind of one-frame difference the aggregate reports as a desync with no
    // name attached.
    .rollback_component_clone_entity_set::<ambition_vfx::HitboxHits>(
        ENGINE,
        "combat.hitbox_hits",
        |hits| hits.hit.iter().copied().collect(),
    )
    .rollback_map_entities::<ambition_vfx::HitboxHits>(ENGINE, "map.hitbox_hits")
    .rollback_component_clone_probed::<ambition_vfx::HitboxLifetime>(
        ENGINE,
        "combat.hitbox_lifetime",
        |lifetime| lifetime.remaining_s.to_bits() as u64,
    )
    .rollback_component_clone_entity_ref::<ambition_combat::moveset::StrikeVolume>(
        ENGINE,
        "combat.strike_volume",
        |volume| volume.owner,
    )
    .rollback_map_entities::<ambition_combat::moveset::StrikeVolume>(ENGINE, "map.strike_volume")
    // G2b: probed through the fired victims' stable identities. A presence
    // count sees the component and nothing of WHO is in the set, so a remap
    // redirecting one victim to the wrong body changes no census — and the
    // visible consequence is a sustained overlap re-firing an on-hit at a body
    // it has already fired at.
    .rollback_component_clone_entity_set::<ambition_combat::on_hit::HitboxOnHit>(
        ENGINE,
        "combat.hitbox_on_hit",
        |on_hit| on_hit.fired_victims(),
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
        // A live match's per-body state. Registered together because they are
        // one decision — match activation — landing on a body, and a rewind that
        // kept some and dropped others would produce a fighter that is half in
        // the match (AA2 / AC2, both GPT 5.6 reviews, 2026-07-29).
        .rollback_component_canonical::<ambition_actors::character_runtime::MatchSeat>(
            ENGINE,
            "actor.match_seat",
        )
        .rollback_component_canonical::<ambition_combat::targeting::MatchTeam>(
            ENGINE,
            "actor.match_team",
        )
        // S4 — the stocks loop's own state. A stock count that is NOT rollback
        // state un-spends itself on a rewind: the body comes back and the count
        // does not, so a fighter loses the same stock twice or never loses it at
        // all. Elimination is the same fact one step later, and a rewind that
        // restores a fighter while leaving it eliminated is a body standing in a
        // match nothing will ever let it play.
        .rollback_component_canonical::<ambition_combat::components::FighterStocks>(
            ENGINE,
            "entity:fighter_stocks",
        )
        .rollback_component_canonical::<ambition_combat::stocks::FighterEliminated>(
            ENGINE,
            "entity:fighter_eliminated",
        )
        // The "already announced" latch for a stocks match's outcome. Registered
        // as STATE rather than left a `Local`, because a rewind across the
        // deciding frame must be able to UN-decide the match — a latch that
        // survives it would swallow the re-announcement on the replay and the
        // ruleset would never hear that the match ended.
        .rollback_resource_canonical::<ambition_combat::stocks::StocksMatchSettled>(
            ENGINE,
            "resource.stocks_match_settled",
        )
        .rollback_component_canonical::<ambition_combat::components::RulesetOwnsDeath>(
            ENGINE,
            "actor.ruleset_owns_death",
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
        .rollback_component_canonical::<ambition_characters::actor::attack_gesture::AttackGestureState>(
            ENGINE,
            "actor.attack_gesture_state",
        )
        .rollback_component_canonical::<ambition_characters::actor::attack_gesture::AttackGestureTuning>(
            ENGINE,
            "actor.attack_gesture_tuning",
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
    // The transformation beat's VALUE, not just its participation. The anchor
    // above only installs `bevy_ggrs::Rollback`; without this the beat's
    // `remaining` and — worse — the `was_invulnerable` it borrowed never
    // restore, so a rewind into the middle of a transformation can leave a body
    // permanently untouchable.
    .rollback_component_clone_probed::<ambition_actors::features::transform_beat::TransformBeat>(
        ENGINE,
        "actor.transform_beat",
        |beat| beat.remaining.to_bits() as u64 ^ u64::from(beat.was_invulnerable) << 32,
    )
    // The REQUEST is state for the same reason. It is necessarily written a
    // frame before it is consumed, and as a message it died in that gap.
    .rollback_component_clone::<ambition_actors::features::transform_beat::TransformBeatRequested>(
        ENGINE,
        "actor.transform_beat_requested",
    )
    // "A sequence is driving this body." Derived from the sequence that owns it,
    // but derived a phase LATER than it is read — the blanking runs in
    // `PlayerInput` and the death beat that inserts the marker runs in
    // `GameplayEffects` — so a restore that did not carry it would hand the
    // player one live frame in the middle of a scripted sequence.
    .rollback_component_clone::<ambition_characters::brain::ScriptedControl>(
        ENGINE,
        "actor.scripted_control",
    )
    // The pose pin the beat and the snake shell both write. Snapshotting the
    // slot itself is owner-agnostic and therefore correct for both: a restore
    // reinstates whatever pin was actually in force. Deriving it from beat state
    // instead would fight the shell for a component it does not own.
    .rollback_component_clone::<ambition_actors::features::ActorAnimOverride>(
        ENGINE,
        "actor.anim_override",
    )
    .rollback_component_clone::<ambition_actors::features::BossConfig>(ENGINE, "boss.config")
    // G2b: a rig IS its slot→limb map, and the map is remapped on every load.
    // A presence count sees "one rig, still here" while the left hand hangs off
    // the right shoulder.
    //
    // The first repair projected `limbs.values()` into the entity-SET census and
    // claimed the slot order came with it. It did not: that census folds targets
    // with a commutative sum, so the two hands trading slots is the same multiset
    // and the same digest — the probe was blind to the one failure the comment
    // named (GPT 5.6, 2026-07-27). The MAP census folds each slot's discriminant
    // against its limb's identity, which is what makes an exchange visible.
    .rollback_component_clone_entity_map::<ambition_actors::features::LimbRig>(
        ENGINE,
        "limb.rig",
        |rig| {
            rig.limbs
                .iter()
                .map(|(slot, limb)| (*slot as u64, *limb))
                .collect()
        },
    )
    .rollback_map_entities::<ambition_actors::features::LimbRig>(ENGINE, "map.limb_rig")
    // G2b: which HOST this limb belongs to. Remapped onto the wrong body, the
    // limb station-keeps around a stranger and strikes where that stranger is.
    .rollback_component_clone_entity_ref::<ambition_actors::features::Limb>(
        ENGINE,
        "limb.member",
        |limb| limb.of,
    )
    .rollback_map_entities::<ambition_actors::features::Limb>(ENGINE, "map.limb_member")
    .rollback_component_clone::<ambition_actors::features::LimbRouteState>(
        ENGINE,
        "limb.route_state",
    )
    .rollback_component_clone::<ambition_actors::features::LimbIntents>(ENGINE, "limb.intents")
    .rollback_component_clone::<ambition_actors::features::CanPilot>(ENGINE, "mount.can_pilot")
    .rollback_component_clone::<ambition_actors::features::Mass>(ENGINE, "mount.mass")
    // G2b: who is riding. A remap that seats the wrong rider locks a body to a
    // mount it never boarded, and the count of occupied slots is unchanged.
    .rollback_component_clone_entity_set::<ambition_actors::features::MountSlot>(
        ENGINE,
        "mount.slot",
        |slot| slot.rider.into_iter().collect(),
    )
    .rollback_map_entities::<ambition_actors::features::MountSlot>(ENGINE, "map.mount_slot")
    .rollback_component_clone::<ambition_actors::features::Mountable>(ENGINE, "mount.mountable")
    .rollback_component_clone::<ambition_actors::features::Mounted>(ENGINE, "mount.mounted")
    .rollback_component_clone_entity_ref::<ambition_actors::features::RidingOn>(
        ENGINE,
        "mount.riding_on",
        |riding| riding.mount,
    )
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
    // The UN-GRANTED baseline the live `ActionSet` / `ActorMoveset` are a pure
    // function of (`identity + worn equipment`). Registering the two derived
    // halves and not their base is the `WornEquipment` oversight again: a rewind
    // restored the live kit but left the baseline at whatever an abandoned future
    // derived, so the next `reconcile_equipment_grants` — fired by any armor
    // spend or pickup — recomputed the live kit from the WRONG base and the
    // resimulation stopped matching. That is precisely what
    // `combat_equipment_switch_and_breakable_survive_forced_rollback_identically`
    // caught: it went red when the protagonist's re-rig changed which kit the
    // overlay derives, and stayed red because nothing rewound the base.
    .rollback_component_clone::<ambition_characters::brain::action_set::IdentityKit>(
        ENGINE,
        "actor.identity_kit",
    )
    // The MEMO that says the identity baseline above is current for this body.
    //
    // It looks like a cache and is not safe to treat as one. `apply_worn_character_gameplay`
    // re-derives a persona when this record disagrees with the body's worn id or
    // the cast generation — so a rewind that restores an EARLIER `WornCharacter`
    // while leaving this at the abandoned future's id makes the record say
    // "already applied" about a kit the body no longer wears. The derive skips,
    // and the resimulation runs a fighter with somebody else's moves.
    //
    // Exactly the `IdentityKit` oversight one entry up, one level further out:
    // registering a derived value and not the record of what derived it. Found by
    // `every_component_in_a_boss_arena_is_registered_derived_or_waived` within
    // minutes of the component existing (2026-07-29).
    // PROBED over both fields, because a desync here is silent by construction:
    // the wrong baseline does not corrupt a number, it makes the persona derive
    // SKIP — and a presence-only probe would see the component and nothing about
    // which cast it claims. The id is hashed rather than counted so two bodies
    // that swapped identities during a rewound frame do not read as identical.
    .rollback_component_clone_probed::<ambition_actors::avatar::PersonaBaseline>(
        ENGINE,
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
    )
    // **THE PROJECTION'S OWN MEMO**, and the third time this exact shape has had
    // to be registered rather than assumed derived — after `IdentityKit` and
    // `PersonaBaseline` directly above.
    //
    // `project_prepared_character_definitions` early-exits when this record
    // agrees with the body's worn id and the cast generation, and it also
    // records what it GRANTED so it can retract exactly that. Leave it out of
    // rollback and a rewind restores an earlier `WornCharacter` while the memo
    // still claims the abandoned future's id: the projection skips, and the body
    // resimulates wearing a kit — hurtboxes, movement tuning, sprite-posed body
    // — that its identity no longer asks for. `granted` makes it worse than a
    // stale read, because retraction is driven from it: the wrong record retracts
    // the wrong facts.
    //
    // ⚠ it was ALWAYS unregistered; nothing caught it because no tested room had
    // a body carrying one. Registering the protagonist's own incarnations as
    // characters (2026-07-29) put it on the PLAYER, so it appeared in every room
    // at once and three coverage gates went red together. The component did not
    // become dangerous that day — it became visible.
    //
    // PROBED over the id, the generation AND the grant set, for the reason
    // `PersonaBaseline` is: a desync here is silent by construction. It does not
    // corrupt a number, it makes a derive SKIP, and a presence-only probe would
    // see the component and nothing about what it claims.
    .rollback_component_clone_probed::<ambition_actors::character_runtime::ProjectedCharacterKit>(
        ENGINE,
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
    )
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
    // The quad's placement travels with its size: both are re-derived per pose
    // from the sheet, so restoring one without the other would leave a body
    // drawn at the right scale in the wrong place until the next pose change.
    .rollback_component_clone::<ambition_combat::components::ActorSpriteOffset>(
        ENGINE,
        "actor.sprite_offset",
    )
    // The pose→geometry binding itself. Constant per body, but a body the
    // rewind RE-CREATES must come back still bound to its sheet — otherwise it
    // silently reverts to whatever box it was spawned with and never recovers.
    // A body's pose clock ACCUMULATES, and its elapsed value selects which hurtbox
    // keyframe is live -- so a rewind that lost it would resolve a body's damageable
    // silhouette from a different instant than the confirmed timeline did.
    .rollback_component_clone_probed::<ambition_actors::character_runtime::BodyPoseClock>(
        ENGINE,
        "actor.body_pose_clock",
        |clock| checksum_bytes(clock.pose.as_bytes()) ^ clock.elapsed_s.to_bits() as u64,
    )
    // Authored and immutable at runtime, but bevy_ggrs DESTROYS AND RECREATES
    // rollback entities: unregistered, the doc is simply absent afterwards and the
    // body silently reverts to its sprite-derived compatibility box forever. Same
    // reasoning as `SwitchFeature`.
    .rollback_component_clone::<ambition_actors::character_runtime::AuthoredHurtboxes>(
        ENGINE,
        "actor.authored_hurtboxes",
    )
    .rollback_component_clone::<ambition_actors::character_sprites::SpritePosedBody>(
        ENGINE,
        "actor.sprite_posed_body",
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
    // A chest's PAYLOAD AND STATE, and the marker that says it was opened.
    //
    // `Collected` and `PickupFeature` were both registered; their chest
    // counterparts were not, so a chest opened in an abandoned future kept its
    // reward spent through the rewind — the exact defect the comment above
    // describes for bricks, one feature family over. `Opened` is the same marker
    // class as `PlayerVisual`: bevy_ggrs recreates the entity and an unregistered
    // marker simply does not come back.
    //
    // Found by A19's unswept-population sweep; no room the sweep visited had ever
    // contained a chest.
    .rollback_component_clone::<ambition_combat::components::ChestFeature>(ENGINE, "feature.chest")
    .rollback_component_clone::<ambition_combat::components::Opened>(ENGINE, "feature.opened")
    .rollback_component_clone_probed::<ambition_combat::components::RespawnTimer>(
        ENGINE,
        "feature.respawn_timer",
        |timer| timer.0.to_bits() as u64,
    )
    .rollback_component_clone_probed::<ambition_combat::components::StandTimer>(
        ENGINE,
        "feature.stand_timer",
        |timer| timer.0.to_bits() as u64,
    )
    .rollback_component_clone::<ambition_combat::hazard_runtime::HazardFeature>(
        ENGINE,
        "feature.hazard",
    )
    // Switch liveness. The `SwitchActivated` MESSAGE is cleared on rollback, but
    // the state that message produced was not rewound — so a switch flipped in an
    // abandoned future stayed on.
    .rollback_component_clone_probed::<ambition_actors::encounter::SwitchOn>(
        ENGINE,
        "feature.switch_on",
        |on| u64::from(on.0),
    )
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
    // Same reasoning once more, for the tag on the player's body. The portal host
    // asks `With<PlayerVisual>, Without<PortalSceneBody>` to decide what to tag
    // as a portal scene body, so a recreated player that came back without the
    // tag would stop being seen by portal staging entirely.
    .rollback_component_clone::<ambition_platformer_primitives::lifecycle::PlayerVisual>(
        ENGINE,
        "lifecycle.player_visual",
    )
    .rollback_component_clone::<ambition_combat::components::PogoPolicy>(
        ENGINE,
        "feature.pogo_policy",
    )
    // The two pogo CAPABILITY markers, beside the policy and volumes that were
    // already registered. Same reasoning as `PlayerVisual`: bevy_ggrs recreates
    // the entity, and a marker that does not come back silently revokes a
    // capability — `apply_pogo_bounce` gates on `PogoTarget`, and a
    // stand-to-crumble surface's pogo affordance IS `PogoTargetContributor`. A
    // body that stops being bounceable after a rewind is a gameplay divergence
    // that no amount of correct geometry can repair.
    //
    // Found by sweeping rooms nobody had swept before (A19). Their registered
    // siblings sat two lines away this whole time.
    .rollback_component_clone::<ambition_combat::on_hit::PogoTarget>(ENGINE, "feature.pogo_target")
    .rollback_component_clone::<ambition_combat::components::PogoTargetContributor>(
        ENGINE,
        "feature.pogo_target_contributor",
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
    // Which sheet a pickup is drawn with. Only a RUNTIME-spawned pickup carries
    // it, and a runtime-spawned pickup is exactly the thing a rewind
    // re-creates — dropping it would leave the resimulated loot invisible while
    // the original was drawn, which is the bug this component exists to fix.
    .rollback_component_clone::<ambition_actors::features::PickupArt>(ENGINE, "feature.pickup_art")
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
    // The renderer's runtime-visual DISCOVERY marker. An actor staged outside the
    // authored `RoomSpec` lists (a duel fighter, a runtime-spawned mount) is only
    // given the sprite pipeline because it carries this, so losing it across a
    // restore leaves the actor rendering invisibly for the rest of the session.
    // Same class as `PlayerVisual`, which this instrument caught the same way:
    // presentation, but presentation whose ABSENCE is permanent. Surfaced by the
    // A20 mounted-pair sweep — no swept ROOM stages an actor imperatively.
    .rollback_component_clone::<ambition_combat::components::RuntimeStagedActor>(
        ENGINE,
        "marker.runtime_staged_actor",
    )
    // Which provider's bank an entity's cues come out of (G1). For a BODY this is
    // republished every sim tick from its worn character, so it would survive a
    // restore either way — but for a PROJECTILE it is stamped once, at spawn, from a
    // firer that may be dead by the time the bolt lands. Unregistered, bevy_ggrs
    // recreates the bolt without it and the impact reverts to the session's voice
    // for the rest of the shot's life. Same class as `PlayerVisual` and
    // `RuntimeStagedActor`: presentation, but presentation whose ABSENCE is
    // permanent. Probed by value, because the value is the whole fact and a count
    // of "how many entities have a source" says nothing about WHOSE.
    .rollback_component_clone_probed::<ambition_sfx::BodyPresentationSource>(
        ENGINE,
        "presentation.body_source",
        |source| checksum_bytes(source.id().as_str().as_bytes()),
    )
    // The marker that says the per-tick derivation OWNS that source and may retract
    // it. Losing it across a restore would strand a body's source: the derivation
    // stops maintaining what it can no longer recognise as its own.
    .rollback_component_clone::<ambition_sfx::DerivedPresentationSource>(
        ENGINE,
        "presentation.body_source_derived",
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
    .rollback_component_clone_probed::<ambition_portal::PortalTransitCooldown>(
        ENGINE,
        "portal.transit_cooldown",
        |cooldown| cooldown.remaining.to_bits() as u64,
    )
    // The pickup's ARM TIMER is ticked every sim tick by `arm_portal_pickups`, so a
    // rewind that kept an abandoned future's timer would let the same press that
    // dropped a gun immediately re-grab it — or refuse a grab the confirmed
    // timeline allowed. Surfaced by the coverage sweep only once its population was
    // derived from the rollback vocabulary: a pickup carries neither
    // `FeatureSimEntity` nor `BodyKinematics`.
    .rollback_component_clone::<ambition_portal::PortalGunPickup>(ENGINE, "portal.gun_pickup")
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
    app.declare_rollback_derived_component::<ambition_actors::avatar::body_integration::PlayerBodyFrameOutput>(
        ENGINE,
        "derived.player_body_frame_output",
        "republished by body integration every simulation frame",
    )
    // A per-tick MIRROR of the item's own body, not a second authority:
    // `sync_ground_items_to_transitable` overwrites pos/vel/half_extent from the
    // authoritative `GroundItem` (registered state) before portal core reads it, and
    // `sync_transitable_to_ground_items` mirrors the possibly-teleported result
    // straight back. Snapshotting it would give one body two restorable positions.
    .declare_rollback_derived_component::<ambition_portal::PortalTransitable>(
        ENGINE,
        "derived.portal_transitable",
        "mirrored from the item's authoritative body every frame, before transit reads it",
    )
    .declare_rollback_derived_component::<ambition_actors::body_mode::BodyModeCapabilities>(
        ENGINE,
        "derived.body_mode_capabilities",
        "projected from the active body mode each frame",
    )
    .declare_rollback_derived_component::<ambition_actors::control::PlayerInputFrame>(
        ENGINE,
        "derived.player_input_frame",
        "copied from GGRS PlayerInputs at the head of every frame",
    )
    .declare_rollback_derived_component::<ambition_characters::action_scheme::ActorActionScheme>(
        ENGINE,
        "derived.actor_action_scheme",
        "reconciled from abilities, moveset, and action set",
    )
    .declare_rollback_derived_component::<ambition_characters::action_scheme::ResolvedTechniqueEdges>(
        ENGINE,
        "derived.resolved_technique_edges",
        "cleared and republished from current input every frame",
    )
    // Recomputed every tick from the authored doc plus the move/pose clocks before
    // anything tests against it, so there is nothing to restore -- and registering
    // it would invite someone to MUTATE it, which is how a hurtbox stops being a
    // pure function of authoritative state (§4.11).
    .declare_rollback_derived_component::<ambition_actors::character_runtime::ResolvedHurtboxes>(
        ENGINE,
        "derived.resolved_hurtboxes",
        "recomputed from AuthoredHurtboxes plus the move and pose clocks each tick",
    )
    .declare_rollback_derived_component::<ambition_characters::actor::attack_gesture::ResolvedAttackGesture>(
        ENGINE,
        "derived.resolved_attack_gesture",
        "republished from ActorControl and rollback-backed gesture history before move triggering",
    )
    .declare_rollback_derived_component::<bevy::prelude::GlobalTransform>(
        ENGINE,
        "derived.global_transform",
        "Bevy transform propagation rebuilds it from Transform and hierarchy",
    )
    .declare_rollback_derived_resource::<ambition_actors::features::ActorSteering>(
        ENGINE,
        "derived.actor_steering",
        "rebuilt from the authoritative actor population before movement",
    )
    // AE6. Derived, not state: `project_combat_rules` rebuilds it in WorldPrep
    // every tick from the match's declaration folded over the world's baseline,
    // both of which outlive any rollback window — the declaration is route
    // lifecycle (`Update`, outside the sim) and the baseline is authored tuning.
    // Registering it as STATE would be the borrow again: a rewind would restore
    // a rules value independently of the declaration that produced it, and the
    // two could then disagree for a frame.
    .declare_rollback_derived_resource::<ambition_combat::rules::ResolvedCombatTuning>(
        ENGINE,
        "derived.resolved_combat_tuning",
        "refolded from DeclaredCombatRules over the world baseline every WorldPrep",
    )
    .declare_rollback_derived_resource::<ambition_characters::brain::SlotControls>(
        ENGINE,
        "derived.slot_controls",
        "republished from GGRS PlayerInputs at the head of every frame",
    )
    .declare_rollback_derived_resource::<ambition_platformer_primitives::markers::ControlledSubject>(
        ENGINE,
        "derived.controlled_subject",
        "resolved from the entity carrying Brain::Player for the active slot",
    )
    .declare_rollback_derived_resource::<ambition_portal::PlayerMovementIntent>(
        ENGINE,
        "derived.portal_player_movement_intent",
        "republished from the current controller frame before portal transit",
    )
    .declare_rollback_derived_resource::<ambition_portal::PortalCarves>(
        ENGINE,
        "derived.portal_carves",
        "rebuilt from placed portals and transit occupancy each frame",
    )
    .declare_rollback_derived_resource::<ambition_platformer_primitives::class_b::ClassBRemapLog>(
        ENGINE,
        "derived.class_b_remap_log",
        "frame-local diagnostic ledger cleared before every simulation step",
    )
    .declare_rollback_derived_resource::<ambition_platformer_primitives::gravity::GravityZones>(
        ENGINE,
        "derived.gravity_zones",
        "rebuilt from authoritative GravityZone components before body integration",
    )
    .declare_rollback_derived_resource::<ambition_portal::PortalHostDepths>(
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
    // ⚠ This was DECLARED DERIVED, on the promise that
    // `heal_projectile_owners` re-resolves it from
    // `SpawnOrigin::Dynamic { parent }`. The promise is not kept: that system's
    // query requires `&SpawnOrigin`, and enemy projectiles carry NONE — measured,
    // `has_origin=false` for every live projectile in the oracle route. So after
    // bevy_ggrs recreated the entity the component was simply gone, the shot's
    // `HitEvent` was emitted with `attacker: None`, and the firer's `ranged` move
    // never learned it connected. That is the equipment oracle's divergence:
    // `MovePlayback.landed_hit` true on three passes and false on the fourth.
    //
    // It is now ordinary rollback state with entity remapping — the same pairing
    // `MovePlayback` uses for its own `live_boxes` handles. A derived declaration
    // is only as good as the system that honours it, and this one names a
    // component the system cannot even see.
    // The boss's SIM-OWNED animation cursor, and the hurtbox sample derived from
    // it. Neither was rollback state, and the coverage sweep never visited a room
    // with a boss in it, so nothing said so. See `rollback_coverage`'s boss-arena
    // sweep, added with this.
    app.rollback_component_cursor::<ambition_actors::boss_encounter::sprites::BossAnimFrame>(
        ENGINE,
        "component.boss_anim_frame",
    )
    .declare_rollback_derived_component::<ambition_actors::features::BossAnimationFrameSample>(
        ENGINE,
        "derived.boss_animation_frame_sample",
        "republished every tick by drive_boss_animators from the rewound BossAnimFrame cursor",
    );

    // G2: probed through the OWNER's stable `SimId`, not by counting carriers. The
    // presence probe this used to carry could not tell a correct remap from one that
    // put back the right number of owners and pointed a bolt at the wrong body —
    // which is the failure mode this registration exists to prevent, so the probe
    // was blind to precisely the thing it was added for.
    app.rollback_component_clone_entity_ref::<ambition_projectiles::ProjectileOwner>(
        ENGINE,
        "component.projectile_owner",
        |owner| owner.0,
    )
    .rollback_map_entities::<ambition_projectiles::ProjectileOwner>(
        ENGINE,
        "map.projectile_owner",
    )
    .declare_rollback_derived_component::<ambition_engine_core::body_clusters::BodyEnvironmentContact>(
        ENGINE,
        "derived.body_environment_contact",
        "rewritten every movement step from body geometry and the live world",
    )
    .declare_rollback_derived_component::<ambition_platformer_primitives::frame_env::ResolvedMotionFrame>(
        ENGINE,
        "derived.resolved_motion_frame",
        "published every tick from the live environment",
    )
    .declare_rollback_derived_component::<ambition_engine_core::BodyMotionFacts>(
        ENGINE,
        "derived.body_motion_facts",
        "republished from MotionModel every movement step",
    )
    .declare_rollback_derived_component::<ambition_platformer_primitives::orientation::SurfaceUpright>(
        ENGINE,
        "derived.surface_upright",
        "republished from support facts every movement step",
    )
    .declare_rollback_derived_component::<ambition_sim_view::BodyPoseView>(
        ENGINE,
        "derived.body_pose_view",
        "SimView projection rebuilt every tick",
    )
    .declare_rollback_derived_component::<ambition_sim_view::ProjectileView>(
        ENGINE,
        "derived.projectile_view",
        "SimView projection rebuilt every tick",
    )
    .declare_rollback_derived_component::<ambition_actors::boss_encounter::EncounterProgress>(
        ENGINE,
        "derived.encounter_progress",
        "recomputed from lifecycle and participant health every tick",
    )
    // Frame-derived RESOURCES (Phase 5 resource-coverage pass): each is
    // republished by its ordinary maintenance system before anything reads it,
    // so a rewind that keeps a stale value is overwritten before it matters.
    .declare_rollback_derived_resource::<ambition_engine_core::control_frame::ControlFrame>(
        ENGINE,
        "derived.control_frame",
        "per-tick input frame regenerated from the synchronized input stream",
    )
    .declare_rollback_derived_resource::<ambition_platformer_primitives::frame_env::ForceZones>(
        ENGINE,
        "derived.force_zones",
        "per-tick zone snapshot rebuilt by collect_force_zones",
    )
    .declare_rollback_derived_resource::<ambition_platformer_primitives::feature_overlay::FeatureEcsWorldOverlay>(
        ENGINE,
        "derived.feature_ecs_world_overlay",
        "collision contributions rebuilt from ECS feature state every tick",
    )
    .declare_rollback_derived_resource::<ambition_actors::features::ecs::perception::PerceptionPeers>(
        ENGINE,
        "derived.perception_peers",
        "perception snapshot rebuilt every tick before brains read it",
    )
    .declare_rollback_derived_resource::<ambition_actors::features::ecs::perception::PerceptionProjectiles>(
        ENGINE,
        "derived.perception_projectiles",
        "perception snapshot rebuilt every tick before brains read it",
    )
    .declare_rollback_derived_resource::<ambition_actors::encounter::EncounterSwitchIndex>(
        ENGINE,
        "derived.encounter_switch_index",
        "rebuilt from SwitchFeature + SwitchOn components each frame",
    )
    .declare_rollback_derived_resource::<ambition_encounter::entity::EncounterView>(
        ENGINE,
        "derived.encounter_view",
        "presentation-intent read model republished each tick",
    )
    .declare_rollback_derived_resource::<ambition_actors::affordances::PlayerAffordances>(
        ENGINE,
        "derived.player_affordances",
        "affordance read model recomputed per frame from body state",
    )
    .declare_rollback_derived_resource::<ambition_actors::affordances::intent::PlayerIntent>(
        ENGINE,
        "derived.player_intent",
        "affordance read model recomputed per frame from control input",
    )
    .declare_rollback_derived_resource::<ambition_actors::affordances::interactable_proximity::NearestInteractable>(
        ENGINE,
        "derived.nearest_interactable",
        "proximity read model recomputed per frame",
    )
    .declare_rollback_derived_resource::<ambition_actors::affordances::pogo_proximity::PogoTargetBelow>(
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
    // S4 — the stocks loop's two messages. Both are written INSIDE the sim
    // schedule, so a rewind that un-happens the KO must un-happen the
    // announcement too: a `BodyKnockedOut` left in the buffer would be re-read on
    // the replay and spend a second stock for one knockout, and a stale
    // `FighterStockSpent` would have a ruleset respawn a fighter that never fell.
    .clear_message_on_rollback::<ambition_combat::stocks::BodyKnockedOut>(
        ENGINE,
        "message.body_knocked_out",
    )
    .clear_message_on_rollback::<ambition_combat::stocks::FighterStockSpent>(
        ENGINE,
        "message.fighter_stock_spent",
    )
    .clear_message_on_rollback::<ambition_combat::stocks::StocksMatchDecided>(
        ENGINE,
        "message.stocks_match_decided",
    )
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
    // A same-tick handshake: the reset processor announces it, and the teardown
    // systems chained after it read it. A cursor GGRS did not rewind would let
    // that teardown fire for a reset the resimulation never committed to — the
    // held items and portals of a session that was, on this timeline, never
    // reset.
    .clear_message_on_rollback::<ambition_actors::session::reset::SandboxResetCommitted>(
        ENGINE,
        "message.sandbox_reset_committed",
    )
    .clear_message_on_rollback::<ambition_actors::features::ecs::damage_apply::WalletShieldSpent>(
        ENGINE,
        "message.wallet_shield_spent",
    )
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
