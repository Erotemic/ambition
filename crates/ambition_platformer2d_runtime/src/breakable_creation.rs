//! Correlated native breakable creation in every body request phase.

use ambition_breakables::creation::{BreakableCreationLimits, RuntimeBreakable};
use ambition_extension_host::{ExtensionOutbox, LowersIn};
use ambition_extension_sdk::action::ActionStage;
use ambition_platformer2d_shared_tangle::action_results::publish_action_result;
use ambition_platformer2d_world::governing_rules::RulesOf;
use ambition_world_port::breakables::*;
use bevy::{ecs::system::SystemState, prelude::*};

pub fn install(app: &mut App) {
    ambition_platformer2d_shared_tangle::action_results::register_action_domain(app, "world.create_breakables",
        ambition_platformer2d_shared_tangle::action_results::ActionCancellationPolicy::Unsupported);
    ambition_extension_host::install_submitted_request_in_body_phases!(app, CreateBreakablesPort,
        "ambition_breakables", lower);
}
type CreationRules = SystemState<RulesOf<'static, 'static, BreakableCreationLimits>>;
fn lower<L: LowersIn>(world: &mut World, mut rules: Local<Option<CreationRules>>) {
    let submitted = world
        .resource_mut::<ExtensionOutbox>()
        .drain::<CreateBreakablesPort>(&L::PHASE);
    if submitted.is_empty() {
        return;
    }
    let rules = rules.get_or_insert_with(|| SystemState::new(world));
    for submitted in submitted {
        let request = submitted.value;
        let dispatch = ambition_platformer2d_shared_tangle::action_results::begin_action_request::<CreateBreakablesPort>(
            world, submitted.state_scope, &submitted.entry, &request);
        if !dispatch.execute {
            continue;
        }
        let limits = rules
            .get(world)
            .ok()
            .and_then(|rules| rules.of(submitted.scope).copied())
            .unwrap_or_default();
        let result = crate::object_behaviors::prepare_creation(world, submitted.scope,
            &request.objects.iter().map(|object| object.bindings.as_slice()).collect::<Vec<_>>()).and_then(|bindings| {
            let objects = request.objects.iter().zip(bindings).map(|(object, bindings)| {
                use ambition_entity_catalog::placements as native;
                let mut spec = native::BreakableSpec::new(object.health);
                // Keep the requested health for native validation; the authored constructor clamps.
                spec.health_current = object.health;
                spec.health_max = object.health;
                spec.collision = match object.collision {
                    BreakableCollision::None => native::BreakableCollisionSpec::None,
                    BreakableCollision::Solid => native::BreakableCollisionSpec::Solid,
                    BreakableCollision::OneWay => native::BreakableCollisionSpec::OneWayUp,
                };
                spec.trigger = match object.trigger {
                    BreakableTrigger::Hit => native::BreakableTriggerSpec::OnHit,
                    BreakableTrigger::Stand => native::BreakableTriggerSpec::OnStand,
                    BreakableTrigger::Either => native::BreakableTriggerSpec::Either,
                };
                spec.respawn = object.respawn_after.map_or(
                    native::HazardRespawn::Never,
                    native::HazardRespawn::AfterSeconds,
                );
                spec.look = object.look.clone();
                spec.pogo_refresh = object.pogo_refresh;
                RuntimeBreakable {
                    key: object.key.clone(),
                    label: object.label.clone().unwrap_or_else(|| object.key.clone()),
                    aabb: ambition_platformer2d_core::Aabb::new(
                        object.at.into(),
                        (Vec2::from(object.size) * 0.5).into(),
                    ),
                    spec,
                    initialization: ambition_platformer2d_construction::runtime::ComposedInitialization {
                        root: (), owned: bindings,
                    },
                }
            })
            .collect::<Vec<_>>();
            ambition_breakables::creation::create_breakables(
                world,
                submitted.scope,
                &objects,
                limits,
            )
        });
        match result {
            Ok(created) => {
                let subjects = created.into_iter().map(|object|
                    ambition_platformer2d_shared_tangle::action_results::object_action_subject(world, object.entity)
                        .expect("a published breakable has a live occurrence")).collect();
                publish_action_result(world, dispatch.reply.as_ref(), ActionStage::Applied, true,
                    "breakables_created", "", subjects);
            }
            Err(message) => {
                publish_action_result(
                    world,
                    dispatch.reply.as_ref(),
                    ActionStage::Rejected,
                    true,
                    "breakable_creation_refused",
                    message,
                    Vec::new(),
                );
            }
        }
    }
}
