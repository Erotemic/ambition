//! Correlated results for native world-contact consumption.

use ambition_extension_host::{ExtensionAppExt, ExtensionOutbox};
use ambition_extension_sdk::{action::ActionStage, phases::BODY_CONTACT};
use ambition_platformer2d_shared_tangle::action_results::publish_action_result;
use ambition_world_port::consumption::{ConsumeContactPort, ConsumeSelfPort, DestructibleStatePort, DestructibleState};
use bevy::prelude::*;

const DOMAIN: &str = "world.consume_contact";

pub fn install(app: &mut App) {
    ambition_platformer2d_shared_tangle::action_results::register_action_domain(app, DOMAIN,
        ambition_platformer2d_shared_tangle::action_results::ActionCancellationPolicy::Unsupported);
    ambition_platformer2d_shared_tangle::action_results::register_action_domain(app, "world.consume_self",
        ambition_platformer2d_shared_tangle::action_results::ActionCancellationPolicy::Unsupported);
    app.install_extension_submitted_request::<ConsumeSelfPort, _>(ambition_extension_sdk::phases::ACTOR_THINK,
        "ambition_breakables", lower_self);
    app.install_extension_context_observation::<DestructibleStatePort>(ambition_extension_sdk::phases::ACTOR_THINK,
        "ambition_breakables", |world, context| Some(world.get::<ambition_breakables::BreakableFeature>(context.scope).map(|feature|
            DestructibleState {health: feature.state().health.current, maximum: feature.state().health.max, consumed: feature.broken()})));
    app.install_extension_submitted_request::<ConsumeContactPort, _>(
        BODY_CONTACT, "ambition_breakables", lower);
}

fn lower(world: &mut World) {
    let requests = world.resource_mut::<ExtensionOutbox>().drain::<ConsumeContactPort>(&BODY_CONTACT);
    for submitted in requests {
        let request = submitted.value;
        let dispatch = ambition_platformer2d_shared_tangle::action_results::begin_action_request::<ConsumeContactPort>(
            world, submitted.state_scope, &submitted.entry, &request);
        if !dispatch.execute { continue; }
        let outcome = (|| {
            let catalog = world.get_resource::<ambition_items::ItemCatalog>().ok_or("inventory_unavailable")?;
            let exchange = ambition_platformer2d_items::inventory::resolve(catalog, &request.exchange)?;
            ambition_breakables::consumption::consume_contact(world, submitted.scope, request.contact, &exchange)
        })();
        let (stage, code, subjects) = match outcome {
            Ok(receipt) => (ActionStage::Applied, "contact_consumed", vec![receipt.object.as_str().to_owned()]),
            Err(code) => (ActionStage::Rejected, code, Vec::new()),
        };
        publish_action_result(world, dispatch.reply.as_ref(), stage, true, code, "", subjects.into_iter().map(ambition_extension_sdk::action::ActionSubject::named).collect());
    }
}


fn lower_self(world: &mut World) {
    let requests = world.resource_mut::<ExtensionOutbox>().drain::<ConsumeSelfPort>(&ambition_extension_sdk::phases::ACTOR_THINK);
    for submitted in requests {
        let dispatch = ambition_platformer2d_shared_tangle::action_results::begin_action_request::<ConsumeSelfPort>(
            world, submitted.state_scope, &submitted.entry, &submitted.value);
        if !dispatch.execute { continue; }
        let outcome = (|| {
            let catalog = world.get_resource::<ambition_items::ItemCatalog>().ok_or("inventory_unavailable")?;
            let exchange = ambition_platformer2d_items::inventory::resolve(catalog, &submitted.value.exchange)?;
            use ambition_platformer2d_shared_tangle::lifecycle::{SessionScopedEntity, resolve_object};
            let session = world.get::<SessionScopedEntity>(submitted.scope).ok_or("invalid_scope")?.0;
            let recipient = resolve_object(world, session, submitted.value.recipient).map_err(|_| "recipient_not_local")?;
            ambition_breakables::consumption::consume_self(world, submitted.scope, recipient, &exchange)
        })();
        let (stage, code) = match outcome {
            Ok(_) => (ActionStage::Applied, "object_consumed"),
            Err(code) => (ActionStage::Rejected, code),
        };
        publish_action_result(world, dispatch.reply.as_ref(), stage, true, code, "", Vec::new());
    }
}
