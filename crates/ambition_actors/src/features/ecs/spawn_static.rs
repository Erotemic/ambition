//! Static authored room-feature spawn helpers.
//!
//! These functions stay family-specific so adding an authored static
//! feature remains "add a RoomSpec Vec + add one loop in spawn.rs".

use super::*;
use crate::features::{ChestBundle, PickupBundle};
use crate::platformer_runtime::prelude::SpawnScopedExt;
use ambition_entity_catalog::placements::PlacementSchema;
use bevy::prelude::Name;

fn damage_volume_from_authored(
    authored: &crate::rooms::Authored<crate::rooms::HazardVolumeSpec>,
) -> crate::combat::DamageVolume {
    let mut damage = crate::combat::Damage::new(
        authored.payload.damage,
        authored.payload.kind,
        authored.payload.team,
    );
    damage.knockback = ambition_engine_core::Vec2::new(
        authored.payload.knockback[0],
        authored.payload.knockback[1],
    );
    damage.hitstop_seconds = authored.payload.hitstop_seconds;
    crate::combat::DamageVolume {
        id: authored.id.clone(),
        aabb: authored.aabb,
        damage,
        respawn: authored.payload.respawn,
        path_id: authored.payload.path_id.clone(),
        motion: authored.payload.motion.clone(),
        enabled: authored.payload.enabled,
    }
}

fn pickup_kind_from_spec(kind: &crate::rooms::PickupKindSpec) -> ambition_interaction::PickupKind {
    match kind {
        crate::rooms::PickupKindSpec::Health { amount } => {
            ambition_interaction::PickupKind::Health { amount: *amount }
        }
        crate::rooms::PickupKindSpec::Currency { amount } => {
            ambition_interaction::PickupKind::Currency { amount: *amount }
        }
        crate::rooms::PickupKindSpec::Ability { ability_id } => {
            ambition_interaction::PickupKind::Ability {
                ability_id: ability_id.clone(),
            }
        }
        crate::rooms::PickupKindSpec::StoryFlag { flag } => {
            ambition_interaction::PickupKind::StoryFlag { flag: flag.clone() }
        }
        crate::rooms::PickupKindSpec::Custom(value) => {
            ambition_interaction::PickupKind::Custom(value.clone())
        }
    }
}

fn pickup_from_authored(
    authored: &crate::rooms::Authored<crate::rooms::PickupSpec>,
) -> ambition_interaction::Pickup {
    ambition_interaction::Pickup {
        id: authored.id.clone(),
        kind: pickup_kind_from_spec(&authored.payload.kind),
        respawn: authored.payload.respawn,
        collected: authored.payload.collected,
    }
}

fn chest_state_from_spec(state: crate::rooms::ChestStateSpec) -> ambition_interaction::ChestState {
    match state {
        crate::rooms::ChestStateSpec::Closed => ambition_interaction::ChestState::Closed,
        crate::rooms::ChestStateSpec::Opening => ambition_interaction::ChestState::Opening,
        crate::rooms::ChestStateSpec::Opened => ambition_interaction::ChestState::Opened,
    }
}

fn chest_from_authored(
    authored: &crate::rooms::Authored<crate::rooms::ChestSpec>,
) -> ambition_interaction::Chest {
    ambition_interaction::Chest {
        id: authored.id.clone(),
        state: chest_state_from_spec(authored.payload.state),
        reward: authored.payload.reward.as_ref().map(pickup_kind_from_spec),
        persistent: authored.payload.persistent,
    }
}

fn breakable_collision_from_spec(
    collision: crate::rooms::BreakableCollisionSpec,
) -> ambition_interaction::BreakableCollision {
    match collision {
        crate::rooms::BreakableCollisionSpec::None => {
            ambition_interaction::BreakableCollision::None
        }
        crate::rooms::BreakableCollisionSpec::Solid => {
            ambition_interaction::BreakableCollision::Solid
        }
        crate::rooms::BreakableCollisionSpec::OneWayUp => {
            ambition_interaction::BreakableCollision::OneWayUp
        }
    }
}

fn breakable_trigger_from_spec(
    trigger: crate::rooms::BreakableTriggerSpec,
) -> ambition_interaction::BreakableTrigger {
    match trigger {
        crate::rooms::BreakableTriggerSpec::OnHit => ambition_interaction::BreakableTrigger::OnHit,
        crate::rooms::BreakableTriggerSpec::OnStand => {
            ambition_interaction::BreakableTrigger::OnStand
        }
        crate::rooms::BreakableTriggerSpec::Either => {
            ambition_interaction::BreakableTrigger::Either
        }
    }
}

fn breakable_state_from_spec(
    state: crate::rooms::BreakableStateSpec,
) -> ambition_interaction::BreakableState {
    match state {
        crate::rooms::BreakableStateSpec::Intact => ambition_interaction::BreakableState::Intact,
        crate::rooms::BreakableStateSpec::Cracking => {
            ambition_interaction::BreakableState::Cracking
        }
        crate::rooms::BreakableStateSpec::Broken => ambition_interaction::BreakableState::Broken,
        crate::rooms::BreakableStateSpec::Respawning => {
            ambition_interaction::BreakableState::Respawning
        }
    }
}

fn breakable_from_authored(
    authored: &crate::rooms::Authored<crate::rooms::BreakableSpec>,
) -> ambition_interaction::Breakable {
    ambition_interaction::Breakable {
        id: authored.id.clone(),
        state: breakable_state_from_spec(authored.payload.state),
        health: ambition_characters::actor::Health {
            current: authored.payload.health_current,
            max: authored.payload.health_max,
            invulnerable: false,
        },
        respawn: authored.payload.respawn,
        collision: breakable_collision_from_spec(authored.payload.collision),
        trigger: breakable_trigger_from_spec(authored.payload.trigger),
        debris_cue: authored.payload.debris_cue.clone(),
        pogo_refresh: authored.payload.pogo_refresh,
    }
}

fn interaction_kind_from_spec(
    kind: &crate::rooms::InteractionKindSpec,
) -> ambition_interaction::InteractionKind {
    match kind {
        crate::rooms::InteractionKindSpec::Door { target } => {
            ambition_interaction::InteractionKind::Door {
                target: target.clone(),
            }
        }
        crate::rooms::InteractionKindSpec::Npc {
            character_id,
            dialogue_id,
            patrol_radius,
            patrol_path_id,
        } => ambition_interaction::InteractionKind::Npc {
            character_id: character_id.clone(),
            dialogue_id: dialogue_id.clone(),
            patrol_radius: *patrol_radius,
            patrol_path_id: patrol_path_id.clone(),
        },
        crate::rooms::InteractionKindSpec::Chest => ambition_interaction::InteractionKind::Chest,
        crate::rooms::InteractionKindSpec::Pickup => ambition_interaction::InteractionKind::Pickup,
        crate::rooms::InteractionKindSpec::Breakable => {
            ambition_interaction::InteractionKind::Breakable
        }
        crate::rooms::InteractionKindSpec::Custom(value) => {
            ambition_interaction::InteractionKind::Custom(value.clone())
        }
    }
}

pub(super) fn interactable_from_authored(
    authored: &crate::rooms::Authored<crate::rooms::InteractableSpec>,
) -> ambition_interaction::Interactable {
    ambition_interaction::Interactable {
        id: authored.id.clone(),
        prompt: authored.payload.prompt.clone(),
        aabb: authored.aabb,
        kind: interaction_kind_from_spec(&authored.payload.kind),
        requires_facing: authored.payload.requires_facing,
        enabled: authored.payload.enabled,
    }
}

#[cfg(feature = "portal")]
fn portal_color_from_spec(
    color: crate::rooms::PortalChannelColorSpec,
) -> ambition_portal::PortalChannelColor {
    match color {
        crate::rooms::PortalChannelColorSpec::Purple => ambition_portal::PortalChannelColor::Purple,
        crate::rooms::PortalChannelColorSpec::Yellow => ambition_portal::PortalChannelColor::Yellow,
        crate::rooms::PortalChannelColorSpec::Teal => ambition_portal::PortalChannelColor::Teal,
        crate::rooms::PortalChannelColorSpec::Red => ambition_portal::PortalChannelColor::Red,
        crate::rooms::PortalChannelColorSpec::Green => ambition_portal::PortalChannelColor::Green,
        crate::rooms::PortalChannelColorSpec::Magenta => {
            ambition_portal::PortalChannelColor::Magenta
        }
        crate::rooms::PortalChannelColorSpec::Cyan => ambition_portal::PortalChannelColor::Cyan,
        crate::rooms::PortalChannelColorSpec::Rose => ambition_portal::PortalChannelColor::Rose,
        crate::rooms::PortalChannelColorSpec::Indexed(n) => {
            ambition_portal::PortalChannelColor::Indexed(n)
        }
    }
}

pub(crate) fn spawn_hazard(
    commands: &mut Commands,
    authored: &crate::rooms::Authored<crate::rooms::HazardVolumeSpec>,
    paths: &[(String, ambition_engine_core::KinematicPath)],
) {
    let hazard = HazardRuntime::new_with_paths(
        authored.id.clone(),
        authored.name.clone(),
        authored.aabb,
        damage_volume_from_authored(authored),
        paths,
    );
    commands.spawn_room_scoped((
        Name::new(format!("Feature hazard: {}", authored.name)),
        FeatureSimEntity,
        RoomVisual,
        FeatureId::new(authored.id.clone()),
        FeatureName::new(authored.name.clone()),
        CenteredAabb::from_center_size(hazard.pos, hazard.size),
        HazardFeature::new(hazard),
    ));
}

pub(crate) fn lower_hazard_placement(
    record: &crate::world::placements::PlacementRecord,
    ctx: &mut crate::world::placements::LoweringCtx<'_, '_, '_>,
) {
    let PlacementSchema::Hazard(spec) = &record.schema;
    let authored = crate::rooms::Authored {
        id: record.id.as_str().to_string(),
        name: record.name.clone(),
        aabb: record.aabb,
        payload: crate::rooms::HazardVolumeSpec {
            damage: spec.damage,
            knockback: spec.knockback,
            kind: spec.kind,
            team: spec.team,
            hitstop_seconds: spec.hitstop_seconds,
            respawn: spec.respawn,
            path_id: spec.path_id.clone(),
            motion: None,
            enabled: true,
        },
    };
    spawn_hazard(ctx.commands, &authored, ctx.paths);
}

pub(crate) fn spawn_pickup(
    commands: &mut Commands,
    authored: &crate::rooms::Authored<crate::rooms::PickupSpec>,
) {
    let feature_aabb = CenteredAabb::from_aabb(authored.aabb);
    commands.spawn_room_scoped((
        Name::new(format!("Feature pickup: {}", authored.name)),
        PickupBundle::new(
            &authored.id,
            &authored.name,
            feature_aabb,
            pickup_from_authored(authored),
        ),
    ));
}

pub(crate) fn spawn_ground_item(commands: &mut Commands, spec: &crate::rooms::GroundItemSpec) {
    // Resolve the held-item registry id -> HeldItemSpec. An unregistered or
    // feature-gated id is skipped (the item simply doesn't appear) -- the same
    // tolerance the retired `spawn_debug_ground_items_once` table had.
    let Some(held) = ambition_characters::brain::held_item_by_id(&spec.held_item) else {
        return;
    };
    commands.spawn_room_scoped((
        Name::new(format!("Ground item: {}", spec.name)),
        crate::items::pickup::GroundItem {
            spec: held,
            pos: spec.pos,
            vel: ambition_engine_core::Vec2::ZERO,
            half_extent: spec.half_extent,
        },
    ));
}

#[cfg(feature = "portal")]
pub(crate) fn spawn_portal_gun_spawn(
    commands: &mut Commands,
    spec: &crate::rooms::PortalGunSpawnSpec,
) {
    commands.spawn_room_scoped((
        Name::new(format!("Portal gun pickup: {}", spec.name)),
        ambition_portal::PortalGunPickup {
            pos: spec.pos,
            half_extent: spec.half_extent,
            // World-placed pickups spawn already armed (a just-dropped one delays).
            arm_timer: 0.0,
        },
    ));
}

#[cfg(feature = "portal")]
pub(crate) fn spawn_portal(commands: &mut Commands, spec: &crate::rooms::PortalSpec) {
    // Authored static portal: the same `Portal` component the gun fires, but
    // pre-placed and color-paired. Room-scoped so a transition despawns it and
    // the loader re-spawns it; never gun-owned, so it persists without a gun.
    // Opening size: authored along-surface half-length if given, else default.
    let half_extent = match spec.half_length {
        Some(h) => ambition_portal::portal_half_extent_with_length(spec.normal, h),
        None => ambition_portal::portal_half_extent(spec.normal),
    };
    let mut entity = commands.spawn_room_scoped((
        Name::new(format!("Portal ({}): {}", spec.color.name(), spec.name)),
        ambition_portal::PlacedPortal::fixed(
            portal_color_from_spec(spec.color).channel(),
            spec.pos,
            spec.normal,
            half_extent,
        ),
    ));
    if let Some(link) = &spec.link {
        entity.insert(ambition_portal::PortalLink(ambition_portal::link_hash(
            link,
        )));
    }
}

pub(crate) fn spawn_shrine(commands: &mut Commands, spec: &crate::rooms::ShrineSpec) {
    commands.spawn_room_scoped((
        Name::new(format!("Heal/save shrine: {}", spec.name)),
        crate::shrine::HealShrine {
            pos: spec.pos,
            half_extent: spec.half_extent,
        },
    ));
}

pub(crate) fn spawn_gravity_zone(commands: &mut Commands, spec: &crate::rooms::GravityZoneSpec) {
    let mut entity = commands.spawn_room_scoped((
        Name::new(format!("Gravity zone: {}", spec.name)),
        ambition_platformer_primitives::gravity::GravityZone {
            aabb: ambition_engine_core::Aabb::new(spec.center, spec.half_extent),
            dir: spec.dir,
        },
    ));
    // A non-zero amplitude makes the column slide horizontally (the sliding
    // gravity demo); a static column omits the OscillatingZone.
    if spec.oscillate_amplitude > 0.0 {
        entity.insert(ambition_platformer_primitives::gravity::OscillatingZone {
            base_center: spec.center,
            half: spec.half_extent,
            amplitude_x: spec.oscillate_amplitude,
            freq: spec.oscillate_freq,
            phase: 0.0,
        });
    }
}

pub(crate) fn spawn_chest(
    commands: &mut Commands,
    authored: &crate::rooms::Authored<crate::rooms::ChestSpec>,
) {
    let feature_aabb = CenteredAabb::from_aabb(authored.aabb);
    commands.spawn_room_scoped((
        Name::new(format!("Feature chest: {}", authored.name)),
        ChestBundle::new(
            &authored.id,
            &authored.name,
            feature_aabb,
            chest_from_authored(authored),
        ),
    ));
}

pub(crate) fn spawn_breakable(
    commands: &mut Commands,
    authored: &crate::rooms::Authored<crate::rooms::BreakableSpec>,
) {
    let feature_aabb = CenteredAabb::from_aabb(authored.aabb);
    let breakable = breakable_from_authored(authored);
    let breakable = &breakable;
    let mut entity = commands.spawn_room_scoped((
        Name::new(format!("Feature breakable: {}", authored.name)),
        FeatureSimEntity,
        RoomVisual,
        FeatureId::new(authored.id.clone()),
        FeatureName::new(authored.name.clone()),
        feature_aabb,
        BreakableFeature::new(breakable.clone()),
        DamageableVolumes::default(),
        PogoPolicy::FromDamageable,
        PogoTargetVolumes::default(),
        StandTimer(0.0),
    ));
    if breakable.collision.blocks_movement() {
        entity.insert(SandboxSolidContributor);
    }
    if breakable.pogo_refresh
        || (breakable.collision.blocks_movement() && breakable.trigger.allows_stand())
    {
        // `PogoTargetContributor` feeds the flat player pogo (world `PogoOrb`
        // blocks); `PogoTarget` makes the SAME breakable an on-hit pogo target
        // for a moveset down-air — victim-pogo and world-orb pogo under one
        // capability (fable review R2.5). A factionless breakable is eligible
        // via the capability alone (`dispatch_hitbox_on_hit`).
        entity.insert((PogoTargetContributor, crate::combat::on_hit::PogoTarget));
    }
}
