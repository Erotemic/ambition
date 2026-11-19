//! Deliver native action results at the next actor read cut.

use ambition_extension_host::{
    AdmittedExtensions, ExtensionAppExt, ExtensionInvocations, ExtensionOutbox, ExtensionSet,
};
use ambition_extension_sdk::{action::*, phases::*};
use ambition_platformer2d_shared_tangle::{
    action_results::{self, ActionDeliveryPolicy, ActionDispatch, ActionMailbox, ActionRecipient},
    lifecycle::{ObjectOccurrence, RoomFreeze, SessionScopedEntity},
    sim_id::SimId,
};
use bevy::prelude::*;

pub fn install(app: &mut App) {
    ambition_platformer2d_shared_tangle::action_results::register_action_domain(app, "actions.cancel",
        ambition_platformer2d_shared_tangle::action_results::ActionCancellationPolicy::Unsupported);
    use ambition_platformer2d_shared_tangle::schedule::{
        GameplaySimulationRoot, Platformer2dSimulationPhase, SimClockHead, SimScheduleExt,
    };
    let sim = app.sim_schedule();
    app.add_systems(
        sim,
        retire_sources
            .in_set(GameplaySimulationRoot)
            .after(ExtensionSet::Admit)
            .after(SimClockHead)
            .before(Platformer2dSimulationPhase::CoreSimulation),
    );
    app.install_extension_trigger::<ActionResultPort, _>(
        ACTOR_THINK,
        "ambition_platformer2d_runtime",
        queue_replies,
    );
    app.install_extension_submitted_request::<CancelActionPort, _>(
        ACTOR_THINK,
        "ambition_platformer2d_runtime",
        cancel_actions,
    );
    for phase in [
        ACTOR_THINK,
        TECHNIQUE_EXECUTION,
        WIELDED_USE,
        MODULE_ENTITY_TICK,
        BOSS_CONDUCT,
        BODY_CONTACT,
        HIT_REACTION,
    ] {
        app.install_extension_observation::<ActionResultsPort>(
            phase,
            "ambition_platformer2d_runtime",
            |world, scope| {
                Some(
                    world
                        .get::<ActionMailbox>(scope)
                        .map_or_else(ActionResults::default, ActionMailbox::history),
                )
            },
        );
    }
}

/// A callback belongs to the submitting module and names one installed entry.
pub fn begin(
    world: &mut World,
    scope: Entity,
    entry: &str,
    domain: &str,
    reply: Option<&ReplyTo>,
    signature: u64,
) -> ActionDispatch {
    let dispatch = action_results::begin_action(world, scope, entry, domain, reply, signature);
    if !dispatch.execute {
        return dispatch;
    }
    if let Some(reply) = reply {
        let valid = ambition_extension_host::reply_entry::<ActionResultPort>(
            &world.resource::<AdmittedExtensions>().0,
            entry,
            &reply.selector,
            &ACTOR_THINK,
        )
        .is_some();
        if !valid {
            action_results::publish_action_result(
                world,
                dispatch.reply.as_ref(),
                ActionStage::Rejected,
                true,
                "invalid_reply_entry",
                "The reply must name an action_result entry in the submitting module.",
                Vec::new(),
            );
            return ActionDispatch {
                execute: false,
                reply: None,
            };
        }
    }
    dispatch
}

#[allow(clippy::type_complexity)]
fn queue_replies(
    admitted: Res<AdmittedExtensions>,
    tick: Res<ambition_time::SimTick>,
    time: Res<ambition_time::WorldTime>,
    pace: RoomFreeze,
    mut scopes: Query<(
        Entity,
        &SessionScopedEntity,
        Option<&SimId>,
        &ObjectOccurrence,
        &mut ActionMailbox,
    )>,
    mut invocations: ResMut<ExtensionInvocations>,
    policies: ambition_platformer2d_world::governing_rules::RulesOf<ActionDeliveryPolicy>,
) {
    let mut order: Vec<_> = scopes
        .iter()
        .map(|(entity, session, id, room, _)| {
            (
                ActionRecipient {
                    session: session.0.0,
                    occurrence: *room,
                    subject: id.map(|id| id.as_str().to_owned()),
                },
                entity,
            )
        })
        .collect();
    order.sort_by(|a, b| a.0.cmp(&b.0));
    for (_, entity) in order {
        let Ok((_, _, _, _, mut mailbox)) = scopes.get_mut(entity) else {
            continue;
        };
        if time.sim_dt() <= 0.0 || pace.scale_of(entity) <= 0.0 {
            continue;
        }
        let policy = policies.of(entity).copied().unwrap_or_default();
        for (sequence, path, result) in mailbox.queue_ready(tick.0, policy, |ticket, path| {
            ambition_extension_host::reply_entry::<ActionResultPort>(
                &admitted.0,
                &ticket.entry,
                path,
                &ACTOR_THINK,
            )
            .is_some()
        }) {
            let entry = admitted
                .0
                .entries
                .iter()
                .find(|entry| entry.path == path)
                .unwrap();
            invocations.trigger_entry_with_receipt::<ActionResultPort>(
                &ACTOR_THINK,
                path,
                entry.descriptor.trigger.selector.clone(),
                entity,
                result,
                ambition_extension_host::InvocationReceipt {sequence, notify: settle_delivery},
            );
        }
    }
}

fn settle_delivery(world: &mut World, scope: Entity, sequence: u64,
    event: ambition_extension_host::InvocationEvent) {
    use ambition_extension_host::InvocationEvent;
    let tick = world.get_resource::<ambition_time::SimTick>().map_or(0, |tick| tick.0);
    let policy = if matches!(&event, InvocationEvent::Faulted) {
        let mut state = bevy::ecs::system::SystemState::<
            ambition_platformer2d_world::governing_rules::RulesOf<ActionDeliveryPolicy>>::new(world);
        state.get(world).ok().and_then(|rules| rules.of(scope).copied()).unwrap_or_default()
    } else { ActionDeliveryPolicy::default() };
    let Some(mut mailbox) = world.get_mut::<ActionMailbox>(scope) else { return; };
    match event {
        InvocationEvent::Started => mailbox.delivery_started(sequence, tick),
        InvocationEvent::Committed => mailbox.delivery_committed(sequence),
        InvocationEvent::Faulted => mailbox.delivery_faulted(sequence, policy),
        InvocationEvent::Unavailable | InvocationEvent::Retired | InvocationEvent::IdleReset => mailbox.delivery_unavailable(sequence),
    }
}

fn retire_sources(
    admitted: Res<AdmittedExtensions>,
    mut mailboxes: Query<&mut ActionMailbox>,
) {
    for mut mailbox in &mut mailboxes {
        mailbox.retire_absent(|entry| {
            admitted.0.entries.iter().any(|active| active.path == entry)
        });
    }
}

fn cancel_actions(world: &mut World) {
    use action_results::{publish_action_result, cancel_action};
    let submitted = world.resource_mut::<ExtensionOutbox>().drain::<CancelActionPort>(&ACTOR_THINK);
    for submitted in submitted {
        let request = submitted.value;
        let mut bytes = Vec::new();
        <CancelActionPort as ambition_extension_sdk::Port>::encode(&request, &mut bytes);
        let dispatch = begin(world, submitted.scope, &submitted.entry, "actions.cancel",
            Some(&request.reply), ambition_extension_sdk::digest::digest_bytes(&bytes));
        if !dispatch.execute { continue; }
        let authorized = submitted.entry.rsplit_once('/').map(|(module, _)| module)
            == request.ticket.entry.rsplit_once('/').map(|(module, _)| module);
        let result = if authorized {
            cancel_action(world, submitted.scope, &request.ticket).map_err(|refusal| refusal.code())
        } else { Err("cancel_foreign_module") };
        let (stage, code) = match result {
            Ok(()) => (ActionStage::Applied, "action_canceled"),
            Err(code) => (ActionStage::Rejected, code),
        };
        publish_action_result(world, dispatch.reply.as_ref(), stage, true, code, "", Vec::new());
    }
}
