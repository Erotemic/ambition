//! Correlated results for native world-contact consumption.

use ambition_extension_host::{ExtensionAppExt, ExtensionOutbox};
use ambition_extension_sdk::{Port, action::ActionStage, phases::BODY_CONTACT};
use ambition_platformer2d_shared_tangle::action_results::publish_action_result;
use ambition_world_port::consumption::ConsumeContactPort;
use bevy::prelude::*;

const DOMAIN: &str = "world.consume_contact";

pub fn install(app: &mut App) {
    app.install_extension_submitted_request::<ConsumeContactPort, _>(
        BODY_CONTACT, "ambition_breakables", lower);
}

fn lower(world: &mut World) {
    let requests = world.resource_mut::<ExtensionOutbox>().drain::<ConsumeContactPort>(&BODY_CONTACT);
    for submitted in requests {
        let request = submitted.value;
        let mut bytes = Vec::new();
        ConsumeContactPort::encode(&request, &mut bytes);
        let dispatch = crate::action_replies::begin(world, submitted.scope, &submitted.entry,
            DOMAIN, Some(&request.reply), ambition_extension_sdk::digest::digest_bytes(&bytes));
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
        publish_action_result(world, dispatch.reply.as_ref(), stage, true, code, "", subjects);
    }
}
