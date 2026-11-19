//! Deliver native action results at the next actor read cut.

use ambition_extension_host::{
    AdmittedExtensions, ExtensionAppExt, ExtensionOutbox, ExtensionSet,
};
use ambition_extension_sdk::{action::*, phases::*};
use ambition_platformer2d_shared_tangle::{
    action_results::{self, ActionMailbox},
};
use bevy::prelude::*;

pub fn install(app: &mut App) {
    app.install_action_refusal_handler(action_results::refuse_action_submission);
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
        crate::behavior_inbox::collect::<ActionMailbox>(crate::behavior_inbox::Lane::Outcome),
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
        app.install_extension_context_observation::<ActionResultsPort>(
            phase,
            "ambition_platformer2d_runtime",
            |world, context| {
                Some(
                    world
                        .get::<ActionMailbox>(context.state_scope)
                        .map_or_else(ActionResults::default, ActionMailbox::history),
                )
            },
        );
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
        let dispatch = ambition_platformer2d_shared_tangle::action_results::begin_action_request::<CancelActionPort>(
            world, submitted.state_scope, &submitted.entry, &request);
        if !dispatch.execute { continue; }
        let authorized = submitted.entry.rsplit_once('/').map(|(module, _)| module)
            == request.ticket.entry.rsplit_once('/').map(|(module, _)| module);
        let result = if authorized {
            cancel_action(world, submitted.state_scope, &request.ticket).map_err(|refusal| refusal.code())
        } else { Err("cancel_foreign_module") };
        let (stage, code) = match result {
            Ok(()) => (ActionStage::Applied, "action_canceled"),
            Err(code) => (ActionStage::Rejected, code),
        };
        publish_action_result(world, dispatch.reply.as_ref(), stage, true, code, "", Vec::new());
    }
}
