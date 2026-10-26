//! Lower authored placements into live domain state.

use super::*;
use ambition_combat::components::{
    DamageableVolumes, PogoPolicy, PogoTargetContributor, PogoTargetVolumes,
};
use ambition_entity_catalog::placements::PlacementSchema;
use ambition_platformer2d_shared_tangle::construction::RootScope;
use ambition_platformer2d_shared_tangle::lifecycle::RoomVisual;

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
        ambition_platformer2d_world::rooms::BreakableTriggerSpec::OnHit => {
            ambition_interaction::BreakableTrigger::OnHit
        }
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
        ambition_platformer2d_world::rooms::BreakableStateSpec::Intact => {
            ambition_interaction::BreakableState::Intact
        }
        ambition_platformer2d_world::rooms::BreakableStateSpec::Cracking => {
            ambition_interaction::BreakableState::Cracking
        }
        ambition_platformer2d_world::rooms::BreakableStateSpec::Broken => {
            ambition_interaction::BreakableState::Broken
        }
        ambition_platformer2d_world::rooms::BreakableStateSpec::Respawning => {
            ambition_interaction::BreakableState::Respawning
        }
    }
}

fn breakable_from_authored(
    authored: &ambition_platformer2d_world::rooms::Authored<
        ambition_platformer2d_world::rooms::BreakableSpec,
    >,
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
        pogo_refresh: authored.payload.pogo_refresh,
    }
}

pub fn lower_breakable_placement(
    record: &ambition_platformer2d_world::placements::PlacementRecord,
    ctx: &mut ambition_platformer2d_construction::placements::LoweringCtx<'_, '_, '_>,
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
    // OW5: a breakable left broken when its room retired is built broken for
    // the time its respawn still needs.
    let remaining = ctx.facts.scheduled_remaining(ctx.room_id, &authored.id);
    spawn_breakable_into(&mut ctx.scope.reborrow(), &authored, remaining);
}

/// Populate one breakable onto a root the construction executor allocated.
/// `broken_for`: built broken, respawning after that many seconds (OW5).
pub fn spawn_breakable_into(
    scope: &mut RootScope,
    authored: &ambition_platformer2d_world::rooms::Authored<
        ambition_platformer2d_world::rooms::BreakableSpec,
    >,
    broken_for: Option<f32>,
) {
    let feature_aabb = CenteredAabb::from_aabb(authored.aabb);
    let mut breakable = breakable_from_authored(authored);
    if broken_for.is_some() {
        let max = breakable.health.max;
        let _broke = breakable.apply_damage(max);
    }
    let breakable = &breakable;
    scope.insert_room_in_session((
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
    if let Some(seconds) = broken_for {
        scope.insert(ambition_world_returns::RespawnTimer(seconds));
    }
    if breakable.pogo_refresh
        || (breakable.collision.blocks_movement() && breakable.trigger.allows_stand())
    {
        // This feature explicitly contributes WORLD rebound geometry.
        scope.insert(PogoTargetContributor);
    }
}
