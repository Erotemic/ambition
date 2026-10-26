//! Lower authored placements into live domain state.

use crate::bundles::{ChestBundle, PickupBundle};
use ambition_combat::components::CenteredAabb;
use ambition_entity_catalog::placements::PlacementSchema;
use ambition_platformer2d_shared_tangle::construction::RootScope;
use ambition_platformer2d_shared_tangle::lifecycle::SessionSpawnScope;
use bevy::prelude::*;

fn pickup_kind_from_spec(
    kind: &ambition_platformer2d_world::rooms::PickupKind,
) -> ambition_interaction::PickupKind {
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
    authored: &ambition_platformer2d_world::rooms::Authored<
        ambition_platformer2d_world::rooms::PickupSpec,
    >,
) -> ambition_interaction::Pickup {
    ambition_interaction::Pickup {
        id: authored.id.clone(),
        kind: pickup_kind_from_spec(&authored.payload.kind),
        respawn: authored.payload.respawn,
    }
}

fn chest_from_authored(
    authored: &ambition_platformer2d_world::rooms::Authored<
        ambition_platformer2d_world::rooms::ChestSpec,
    >,
) -> ambition_interaction::Chest {
    // ⛔ THE OPENED STATE IS NOT A FIELD OF THE COMPONENT. A `Chest::state` that
    // the runtime did not read once let an authored-open chest grant its
    // reward again; `ChestSpec::opened` lowers into the `Opened` marker in
    // `spawn_chest_into`, which is what the runtime gates on.
    ambition_interaction::Chest {
        id: authored.id.clone(),
        reward: authored.payload.reward.as_ref().map(pickup_kind_from_spec),
    }
}

pub fn lower_pickup_placement(
    record: &ambition_platformer2d_world::placements::PlacementRecord,
    ctx: &mut ambition_platformer2d_construction::placements::LoweringCtx<'_, '_, '_>,
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
    // Q152: a pickup collected before its room retired is built collected
    // for the time its regrowth still needs.
    let remaining = ctx.facts.scheduled_remaining(ctx.room_id, &authored.id);
    spawn_pickup_into(&mut ctx.scope.reborrow(), &authored, remaining);
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
    authored: &ambition_platformer2d_world::rooms::Authored<
        ambition_platformer2d_world::rooms::PickupSpec,
    >,
) -> bevy::ecs::entity::Entity {
    // `PickupBundle` already carries `RoomScopedEntity` (via `FeatureRenderedBundle`),
    // so insert through the session-only helper — `insert_room_in_session` would
    // prepend a second `RoomScopedEntity` and trip Bevy's duplicate-component panic.
    let root = commands.spawn_empty().id();
    spawn_pickup_into(
        &mut RootScope::new(commands, session_scope, root),
        authored,
        None,
    );
    root
}

/// Populate one pickup onto a root the construction executor allocated.
/// `collected_for`: built collected, regrowing after that many seconds (Q152).
pub fn spawn_pickup_into(
    scope: &mut RootScope,
    authored: &ambition_platformer2d_world::rooms::Authored<
        ambition_platformer2d_world::rooms::PickupSpec,
    >,
    collected_for: Option<f32>,
) {
    let feature_aabb = CenteredAabb::from_aabb(authored.aabb);
    scope.insert_session_scoped((
        Name::new(format!("Feature pickup: {}", authored.name)),
        PickupBundle::new(
            &authored.id,
            &authored.name,
            feature_aabb,
            pickup_from_authored(authored),
        ),
    ));
    // Carry the authored art id onto the entity so a pickup spawned at RUNTIME
    // (with no room spec behind it) can still be drawn — see `PickupArt`.
    if let Some(sprite) = authored.payload.sprite.clone() {
        scope.insert(crate::pickups::PickupArt(sprite));
    }
    if let Some(seconds) = collected_for {
        scope.insert((
            crate::pickups::Collected,
            ambition_world_returns::RespawnTimer(seconds),
        ));
    }
}

pub fn lower_chest_placement(
    record: &ambition_platformer2d_world::placements::PlacementRecord,
    ctx: &mut ambition_platformer2d_construction::placements::LoweringCtx<'_, '_, '_>,
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
    spawn_chest_into(&mut ctx.scope.reborrow(), &authored);
}

/// Populate one chest onto a root the construction executor allocated.
pub fn spawn_chest_into(
    scope: &mut RootScope,
    authored: &ambition_platformer2d_world::rooms::Authored<
        ambition_platformer2d_world::rooms::ChestSpec,
    >,
) {
    let feature_aabb = CenteredAabb::from_aabb(authored.aabb);
    // `ChestBundle` already carries `RoomScopedEntity` (via `FeatureRenderedBundle`),
    // so insert through the session-only helper — see the note in `spawn_pickup`.
    scope.insert_session_scoped((
        Name::new(format!("Feature chest: {}", authored.name)),
        ChestBundle::new(
            &authored.id,
            &authored.name,
            feature_aabb,
            chest_from_authored(authored),
        ),
    ));
    // Authored open, or remembered open (`construction::spend_request`): the
    // same canonical state a chest a body opened has (Q105).
    if authored.payload.opened {
        scope.insert(ambition_world_items::containers::Opened);
    }
}
