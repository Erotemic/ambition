//! Static authored room-feature spawn helpers.
//!
//! These functions stay family-specific so adding an authored static
//! feature remains "add a RoomSpec Vec + add one loop in spawn.rs".

use super::*;
use crate::features::{ChestBundle, PickupBundle};
use ambition_entity_catalog::placements::PlacementSchema;
use ambition_platformer2d_shared_tangle::lifecycle::{SessionSpawnScope, SpawnSessionScopedExt};
use bevy::prelude::Name;

fn damage_volume_from_authored(
    authored: &ambition_platformer2d_world::rooms::Authored<ambition_platformer2d_world::rooms::HazardVolumeSpec>,
) -> ambition_combat::DamageVolume {
    let mut damage = ambition_combat::Damage::new(
        authored.payload.damage,
        authored.payload.kind,
        authored.payload.team,
    );
    damage.knockback = ambition_platformer2d_core::Vec2::new(
        authored.payload.knockback[0],
        authored.payload.knockback[1],
    );
    damage.hitstop_seconds = authored.payload.hitstop_seconds;
    ambition_combat::DamageVolume {
        id: authored.id.clone(),
        aabb: authored.aabb,
        damage,
        respawn: authored.payload.respawn,
        path_id: authored.payload.path_id.clone(),
        motion: authored.payload.motion.clone(),
        enabled: authored.payload.enabled,
    }
}

fn pickup_kind_from_spec(kind: &ambition_platformer2d_world::rooms::PickupKind) -> ambition_interaction::PickupKind {
    match kind {
        ambition_platformer2d_world::rooms::PickupKind::Health { amount } => {
            ambition_interaction::PickupKind::Health { amount: *amount }
        }
        ambition_platformer2d_world::rooms::PickupKind::Currency { amount } => {
            ambition_interaction::PickupKind::Currency { amount: *amount }
        }
        ambition_platformer2d_world::rooms::PickupKind::Ability { ability_id } => {
            ambition_interaction::PickupKind::Ability {
                ability_id: ability_id.clone(),
            }
        }
        ambition_platformer2d_world::rooms::PickupKind::StoryFlag { flag } => {
            ambition_interaction::PickupKind::StoryFlag { flag: flag.clone() }
        }
        ambition_platformer2d_world::rooms::PickupKind::Custom(value) => {
            ambition_interaction::PickupKind::Custom(value.clone())
        }
    }
}

fn pickup_from_authored(
    authored: &ambition_platformer2d_world::rooms::Authored<ambition_platformer2d_world::rooms::PickupSpec>,
) -> ambition_interaction::Pickup {
    ambition_interaction::Pickup {
        id: authored.id.clone(),
        kind: pickup_kind_from_spec(&authored.payload.kind),
        respawn: authored.payload.respawn,
        collected: authored.payload.collected,
    }
}

fn chest_state_from_spec(state: ambition_platformer2d_world::rooms::ChestStateSpec) -> ambition_interaction::ChestState {
    match state {
        ambition_platformer2d_world::rooms::ChestStateSpec::Closed => ambition_interaction::ChestState::Closed,
        ambition_platformer2d_world::rooms::ChestStateSpec::Opening => ambition_interaction::ChestState::Opening,
        ambition_platformer2d_world::rooms::ChestStateSpec::Opened => ambition_interaction::ChestState::Opened,
    }
}

fn chest_from_authored(
    authored: &ambition_platformer2d_world::rooms::Authored<ambition_platformer2d_world::rooms::ChestSpec>,
) -> ambition_interaction::Chest {
    ambition_interaction::Chest {
        id: authored.id.clone(),
        state: chest_state_from_spec(authored.payload.state),
        reward: authored.payload.reward.as_ref().map(pickup_kind_from_spec),
        persistent: authored.payload.persistent,
    }
}

fn breakable_collision_from_spec(
    collision: ambition_platformer2d_world::rooms::BreakableCollisionSpec,
) -> ambition_interaction::BreakableCollision {
    match collision {
        ambition_platformer2d_world::rooms::BreakableCollisionSpec::None => {
            ambition_interaction::BreakableCollision::None
        }
        ambition_platformer2d_world::rooms::BreakableCollisionSpec::Solid => {
            ambition_interaction::BreakableCollision::Solid
        }
        ambition_platformer2d_world::rooms::BreakableCollisionSpec::OneWayUp => {
            ambition_interaction::BreakableCollision::OneWayUp
        }
    }
}

fn breakable_trigger_from_spec(
    trigger: ambition_platformer2d_world::rooms::BreakableTriggerSpec,
) -> ambition_interaction::BreakableTrigger {
    match trigger {
        ambition_platformer2d_world::rooms::BreakableTriggerSpec::OnHit => ambition_interaction::BreakableTrigger::OnHit,
        ambition_platformer2d_world::rooms::BreakableTriggerSpec::OnStand => {
            ambition_interaction::BreakableTrigger::OnStand
        }
        ambition_platformer2d_world::rooms::BreakableTriggerSpec::Either => {
            ambition_interaction::BreakableTrigger::Either
        }
    }
}

fn breakable_state_from_spec(
    state: ambition_platformer2d_world::rooms::BreakableStateSpec,
) -> ambition_interaction::BreakableState {
    match state {
        ambition_platformer2d_world::rooms::BreakableStateSpec::Intact => ambition_interaction::BreakableState::Intact,
        ambition_platformer2d_world::rooms::BreakableStateSpec::Cracking => {
            ambition_interaction::BreakableState::Cracking
        }
        ambition_platformer2d_world::rooms::BreakableStateSpec::Broken => ambition_interaction::BreakableState::Broken,
        ambition_platformer2d_world::rooms::BreakableStateSpec::Respawning => {
            ambition_interaction::BreakableState::Respawning
        }
    }
}

fn breakable_from_authored(
    authored: &ambition_platformer2d_world::rooms::Authored<ambition_platformer2d_world::rooms::BreakableSpec>,
) -> ambition_interaction::Breakable {
    ambition_interaction::Breakable {
        id: authored.id.clone(),
        state: breakable_state_from_spec(authored.payload.state),
        health: ambition_characters::actor::Health {
            current: authored.payload.health_current,
            max: authored.payload.health_max,
            invulnerable: Default::default(),
        },
        respawn: authored.payload.respawn,
        collision: breakable_collision_from_spec(authored.payload.collision),
        trigger: breakable_trigger_from_spec(authored.payload.trigger),
        debris_cue: authored.payload.debris_cue.clone(),
        pogo_refresh: authored.payload.pogo_refresh,
    }
}

fn interaction_kind_from_spec(
    kind: &ambition_platformer2d_world::rooms::InteractionKindSpec,
) -> ambition_interaction::InteractionKind {
    match kind {
        ambition_platformer2d_world::rooms::InteractionKindSpec::Door { target } => {
            ambition_interaction::InteractionKind::Door {
                target: target.clone(),
            }
        }
        ambition_platformer2d_world::rooms::InteractionKindSpec::Npc {
            character_id,
            dialogue_id,
            patrol_radius,
            patrol_path_id,
            brain_override,
        } => ambition_interaction::InteractionKind::Npc {
            character_id: character_id.clone(),
            dialogue_id: dialogue_id.clone(),
            patrol_radius: *patrol_radius,
            patrol_path_id: patrol_path_id.clone(),
            brain_override: brain_override.clone(),
        },
        ambition_platformer2d_world::rooms::InteractionKindSpec::Chest => ambition_interaction::InteractionKind::Chest,
        ambition_platformer2d_world::rooms::InteractionKindSpec::Pickup => ambition_interaction::InteractionKind::Pickup,
        ambition_platformer2d_world::rooms::InteractionKindSpec::Breakable => {
            ambition_interaction::InteractionKind::Breakable
        }
        ambition_platformer2d_world::rooms::InteractionKindSpec::Custom(value) => {
            ambition_interaction::InteractionKind::Custom(value.clone())
        }
    }
}

pub(super) fn interactable_from_authored(
    authored: &ambition_platformer2d_world::rooms::Authored<ambition_platformer2d_world::rooms::InteractableSpec>,
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
    color: ambition_platformer2d_world::rooms::PortalChannelColorSpec,
) -> ambition_portal2d::PortalChannelColor {
    match color {
        ambition_platformer2d_world::rooms::PortalChannelColorSpec::Purple => {
            ambition_portal2d::PortalChannelColor::Purple
        }
        ambition_platformer2d_world::rooms::PortalChannelColorSpec::Yellow => {
            ambition_portal2d::PortalChannelColor::Yellow
        }
        ambition_platformer2d_world::rooms::PortalChannelColorSpec::Teal => ambition_portal2d::PortalChannelColor::Teal,
        ambition_platformer2d_world::rooms::PortalChannelColorSpec::Red => ambition_portal2d::PortalChannelColor::Red,
        ambition_platformer2d_world::rooms::PortalChannelColorSpec::Green => ambition_portal2d::PortalChannelColor::Green,
        ambition_platformer2d_world::rooms::PortalChannelColorSpec::Magenta => {
            ambition_portal2d::PortalChannelColor::Magenta
        }
        ambition_platformer2d_world::rooms::PortalChannelColorSpec::Cyan => ambition_portal2d::PortalChannelColor::Cyan,
        ambition_platformer2d_world::rooms::PortalChannelColorSpec::Rose => ambition_portal2d::PortalChannelColor::Rose,
        ambition_platformer2d_world::rooms::PortalChannelColorSpec::Indexed(n) => {
            ambition_portal2d::PortalChannelColor::Indexed(n)
        }
    }
}

/// Populate one authored hazard onto a root the construction executor
/// allocated (the placement plan-row shape).
pub(crate) fn spawn_hazard_into(
    commands: &mut Commands,
    session_scope: SessionSpawnScope,
    root: bevy::ecs::entity::Entity,
    authored: &ambition_platformer2d_world::rooms::Authored<ambition_platformer2d_world::rooms::HazardVolumeSpec>,
    paths: &[(String, ambition_platformer2d_core::KinematicPath)],
) {
    let hazard = HazardRuntime::new_with_paths(
        authored.id.clone(),
        authored.name.clone(),
        authored.aabb,
        damage_volume_from_authored(authored),
        paths,
    );
    commands.insert_room_in_session(
        session_scope,
        root,
        (
            Name::new(format!("Feature hazard: {}", authored.name)),
            FeatureSimEntity,
            RoomVisual,
            FeatureId::new(authored.id.clone()),
            FeatureName::new(authored.name.clone()),
            CenteredAabb::from_center_size(hazard.pos, hazard.size),
            HazardFeature::new(hazard),
        ),
    );
}

pub(crate) fn lower_hazard_placement(
    record: &crate::world::placements::PlacementRecord,
    ctx: &mut crate::world::placements::LoweringCtx<'_, '_, '_>,
) {
    let PlacementSchema::Hazard(spec) = &record.schema else {
        return;
    };
    let authored = ambition_platformer2d_world::rooms::Authored {
        id: record.id.as_str().to_string(),
        name: record.name.clone(),
        aabb: record.aabb,
        payload: ambition_platformer2d_world::rooms::HazardVolumeSpec {
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
    spawn_hazard_into(
        ctx.commands,
        ctx.session_scope,
        ctx.root,
        &authored,
        ctx.paths,
    );
}

pub(crate) fn lower_interactable_placement(
    record: &crate::world::placements::PlacementRecord,
    ctx: &mut crate::world::placements::LoweringCtx<'_, '_, '_>,
) {
    let PlacementSchema::Interactable(spec) = &record.schema else {
        return;
    };
    let authored = ambition_platformer2d_world::rooms::Authored {
        id: record.id.as_str().to_string(),
        name: record.name.clone(),
        aabb: record.aabb,
        payload: spec.clone(),
    };
    super::spawn_actors::spawn_interactable_into(
        ctx.commands,
        &ctx.context.characters,
        &ctx.context.sheets,
        &ctx.context.prepared,
        ctx.session_scope,
        ctx.root,
        &authored,
        ctx.paths,
    );
}

pub(crate) fn lower_pickup_placement(
    record: &crate::world::placements::PlacementRecord,
    ctx: &mut crate::world::placements::LoweringCtx<'_, '_, '_>,
) {
    let PlacementSchema::Pickup(spec) = &record.schema else {
        return;
    };
    let authored = ambition_platformer2d_world::rooms::Authored {
        id: record.id.as_str().to_string(),
        name: record.name.clone(),
        aabb: record.aabb,
        payload: spec.clone(),
    };
    spawn_pickup_into(ctx.commands, ctx.session_scope, ctx.root, &authored);
}

/// Spawn ONE live pickup.
///
/// Public because authored placement is not the only way a pickup comes to
/// exist: a game may need to DROP one at runtime — rings scattering out of a
/// body that just took a hit, an enemy's loot, a chest's reward. The engine
/// could lower authored pickups but had no way to hand a game that same
/// capability, so any game wanting a drop had to rebuild the bundle itself and
/// would have drifted from the collection path the moment either side changed.
///
/// The spawned pickup is an ordinary one in every respect: the shared
/// `collect_ecs_pickups` credits it, so a dropped ring and an authored ring are
/// indistinguishable once they exist.
/// Returns the spawned pickup root, so a caller can decorate it further (e.g. a
/// tossed / scattered pickup that adds its own physics component).
pub fn spawn_pickup(
    commands: &mut Commands,
    session_scope: SessionSpawnScope,
    authored: &ambition_platformer2d_world::rooms::Authored<ambition_platformer2d_world::rooms::PickupSpec>,
) -> bevy::ecs::entity::Entity {
    // `PickupBundle` already carries `RoomScopedEntity` (via `FeatureRenderedBundle`),
    // so insert through the session-only helper — `insert_room_in_session` would
    // prepend a second `RoomScopedEntity` and trip Bevy's duplicate-component panic.
    let root = commands.spawn_empty().id();
    spawn_pickup_into(commands, session_scope, root, authored);
    root
}

/// Populate one pickup onto a root the construction executor allocated.
pub(crate) fn spawn_pickup_into(
    commands: &mut Commands,
    session_scope: SessionSpawnScope,
    root: bevy::ecs::entity::Entity,
    authored: &ambition_platformer2d_world::rooms::Authored<ambition_platformer2d_world::rooms::PickupSpec>,
) {
    let feature_aabb = CenteredAabb::from_aabb(authored.aabb);
    commands.insert_session_scoped(
        session_scope,
        root,
        (
            Name::new(format!("Feature pickup: {}", authored.name)),
            PickupBundle::new(
                &authored.id,
                &authored.name,
                feature_aabb,
                pickup_from_authored(authored),
            ),
        ),
    );
    // Carry the authored art id onto the entity so a pickup spawned at RUNTIME
    // (with no room spec behind it) can still be drawn — see `PickupArt`.
    if let Some(sprite) = authored.payload.sprite.clone() {
        commands
            .entity(root)
            .insert(crate::features::ecs::pickups::PickupArt(sprite));
    }
}

#[cfg(feature = "portal")]
pub(crate) fn lower_portal_placement(
    record: &crate::world::placements::PlacementRecord,
    ctx: &mut crate::world::placements::LoweringCtx<'_, '_, '_>,
) {
    let PlacementSchema::Portal(schema) = &record.schema else {
        return;
    };
    // Reconstruct the runtime-facing spec: the face center is the record's
    // authored footprint center (the converter set `pos = box center`), and the
    // Tier-0 mirror carries the axis-aligned normal / link / half-length.
    let center = ambition_platformer2d_core::Vec2::new(
        (record.aabb.min.x + record.aabb.max.x) * 0.5,
        (record.aabb.min.y + record.aabb.max.y) * 0.5,
    );
    let spec = ambition_platformer2d_world::rooms::PortalSpec {
        id: record.id.as_str().to_string(),
        name: record.name.clone(),
        color: schema.color,
        pos: center,
        normal: ambition_platformer2d_core::Vec2::new(schema.normal[0], schema.normal[1]),
        link: schema.link.clone(),
        half_length: schema.half_length,
    };
    spawn_portal_into(ctx.commands, ctx.session_scope, ctx.root, &spec);
}

#[cfg(feature = "portal")]
pub(crate) fn spawn_portal_into(
    commands: &mut Commands,
    session_scope: SessionSpawnScope,
    root: bevy::ecs::entity::Entity,
    spec: &ambition_platformer2d_world::rooms::PortalSpec,
) {
    let half_extent = match spec.half_length {
        Some(h) => ambition_portal2d::portal_half_extent_with_length(spec.normal, h),
        None => ambition_portal2d::portal_half_extent(spec.normal),
    };
    let mut entity = commands.insert_room_in_session(
        session_scope,
        root,
        (
            Name::new(format!("Portal ({}): {}", spec.color.name(), spec.name)),
            ambition_portal2d::PlacedPortal::fixed(
                portal_color_from_spec(spec.color).channel(),
                spec.pos,
                spec.normal,
                half_extent,
            ),
        ),
    );
    if let Some(link) = &spec.link {
        entity.insert(ambition_portal2d::PortalLink(ambition_portal2d::link_hash(
            link,
        )));
    }
}

pub(crate) fn lower_chest_placement(
    record: &crate::world::placements::PlacementRecord,
    ctx: &mut crate::world::placements::LoweringCtx<'_, '_, '_>,
) {
    let PlacementSchema::Chest(spec) = &record.schema else {
        return;
    };
    let authored = ambition_platformer2d_world::rooms::Authored {
        id: record.id.as_str().to_string(),
        name: record.name.clone(),
        aabb: record.aabb,
        payload: spec.clone(),
    };
    spawn_chest_into(ctx.commands, ctx.session_scope, ctx.root, &authored);
}

/// Populate one chest onto a root the construction executor allocated.
pub(crate) fn spawn_chest_into(
    commands: &mut Commands,
    session_scope: SessionSpawnScope,
    root: bevy::ecs::entity::Entity,
    authored: &ambition_platformer2d_world::rooms::Authored<ambition_platformer2d_world::rooms::ChestSpec>,
) {
    let feature_aabb = CenteredAabb::from_aabb(authored.aabb);
    // `ChestBundle` already carries `RoomScopedEntity` (via `FeatureRenderedBundle`),
    // so insert through the session-only helper — see the note in `spawn_pickup`.
    commands.insert_session_scoped(
        session_scope,
        root,
        (
            Name::new(format!("Feature chest: {}", authored.name)),
            ChestBundle::new(
                &authored.id,
                &authored.name,
                feature_aabb,
                chest_from_authored(authored),
            ),
        ),
    );
}

pub(crate) fn lower_breakable_placement(
    record: &crate::world::placements::PlacementRecord,
    ctx: &mut crate::world::placements::LoweringCtx<'_, '_, '_>,
) {
    let PlacementSchema::Breakable(spec) = &record.schema else {
        return;
    };
    let authored = ambition_platformer2d_world::rooms::Authored {
        id: record.id.as_str().to_string(),
        name: record.name.clone(),
        aabb: record.aabb,
        payload: spec.clone(),
    };
    spawn_breakable_into(ctx.commands, ctx.session_scope, ctx.root, &authored);
}

/// Populate one breakable onto a root the construction executor allocated.
pub(crate) fn spawn_breakable_into(
    commands: &mut Commands,
    session_scope: SessionSpawnScope,
    root: bevy::ecs::entity::Entity,
    authored: &ambition_platformer2d_world::rooms::Authored<ambition_platformer2d_world::rooms::BreakableSpec>,
) {
    let feature_aabb = CenteredAabb::from_aabb(authored.aabb);
    let breakable = breakable_from_authored(authored);
    let breakable = &breakable;
    let mut entity = commands.insert_room_in_session(
        session_scope,
        root,
        (
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
        ),
    );
    if breakable.pogo_refresh
        || (breakable.collision.blocks_movement() && breakable.trigger.allows_stand())
    {
        // This feature explicitly contributes WORLD rebound geometry.
        entity.insert(PogoTargetContributor);
    }
}
