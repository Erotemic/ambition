//! Deliver native action results at the next actor read cut.

use ambition_extension_host::{
    AdmittedExtensions, ExtensionAppExt, ExtensionInvocations, ExtensionOutbox, ExtensionSet,
};
use ambition_extension_sdk::{action::*, phases::*};
use ambition_platformer2d_shared_tangle::{
    action_results::{self, ActionDispatch, ActionMailbox, ActionRecipient},
    lifecycle::{ObjectOccurrence, RoomFreeze, SessionScopedEntity},
    sim_id::SimId,
};
use bevy::prelude::*;

pub fn install(app: &mut App) {
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
        for (path, result) in mailbox.take_ready(tick.0, |ticket, path| {
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
            invocations.trigger_entry::<ActionResultPort>(
                &ACTOR_THINK,
                path,
                entry.descriptor.trigger.selector.clone(),
                entity,
                result,
            );
        }
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
    let submitted = world.resource_mut::<ExtensionOutbox>().drain::<CancelActionPort>(&ACTOR_THINK);
    for submitted in submitted {
        let ticket = submitted.value;
        if submitted.entry.rsplit_once('/').map(|(module, _)| module)
            != ticket.entry.rsplit_once('/').map(|(module, _)| module)
        {
            warn!("action cancellation refused: another module owns the ticket");
            continue;
        }
        ambition_platformer2d_shared_tangle::action_results::cancel_action(world, submitted.scope, &ticket);
    }
}
