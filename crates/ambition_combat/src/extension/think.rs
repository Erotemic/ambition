//! The adapters of a custom brain: `ambition.actor.think` and
//! `ambition.actor.fire`. See the port cards in
//! `ambition_combat_port::think`.
//!
//! A body whose placement authors a `CharacterBrain::Custom` key that a module
//! binds decides in `actor_think`. That phase is after the brains published
//! their frames and before the control gate, so what the module asks for is
//! in the frame the gate restricts, and a demo's enemy behaviour can be a
//! module and not Rust in the demo crate.

use ambition_characters::actor::control::ActorFireRequest;
use ambition_characters::actor::BodyHealth;
use ambition_characters::perception::{ModuleObservation, PerceptionRequirement};
use ambition_characters::control::{ActorControl, DrivingParticipant};
use ambition_combat_port::{ActorFirePort, ActorSightedThinkPort, ActorThink, ActorThinkPort};
use ambition_entity_catalog::placements::CharacterBrain;
use ambition_extension_host::{AdmittedExtensions, ExtensionAppExt, ExtensionInvocations, ExtensionOutbox};
use ambition_extension_sdk::phases::ACTOR_THINK;
use ambition_extension_sdk::Port;
use ambition_platformer2d_core as ae;
use ambition_platformer2d_shared_tangle::markers::PlayerEntity;
use ambition_platformer2d_shared_tangle::sim_id::SimId;
use bevy::prelude::*;

use crate::actor_tuning::ActorConfig;
use crate::components::{ActorTarget, FeatureId};

/// Install the trigger port, the fire request and holding a status in
/// `actor_think`.
pub fn install_for_actor_think(app: &mut App) {
    app.install_extension_trigger::<ActorThinkPort, _>(
        ACTOR_THINK,
        "ambition_combat",
        queue_actor_thinks.run_if(ambition_extension_host::trigger_is_bound::<ActorThinkPort>),
    );
    // OBS3: the same decision with the body's own sight as its knowledge.
    app.install_extension_trigger::<ActorSightedThinkPort, _>(
        ACTOR_THINK,
        "ambition_combat",
        queue_sighted_actor_thinks.run_if(ambition_extension_host::trigger_is_bound::<ActorSightedThinkPort>),
    );
    app.install_extension_submitted_request::<ActorFirePort, _>(ACTOR_THINK, "ambition_combat", lower_actor_fires);
    super::status::install_for_actor_think(app);
}

/// The bodies a custom-brain trigger invokes, and the facts it reads.
type ThinkingBodies<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static ActorConfig,
        &'static ae::BodyKinematics,
        Option<&'static ae::BodyGroundState>,
        &'static BodyHealth,
        Option<&'static ActorTarget>,
        Option<&'static SimId>,
        Option<&'static FeatureId>,
        Option<&'static ModuleObservation>,
    ),
    (Without<PlayerEntity>, Without<DrivingParticipant>),
>;

/// The keys port `P` binds among the admitted entries.
fn bound_keys<P: Port>(admitted: &AdmittedExtensions) -> Vec<&str> {
    admitted
        .0
        .entries
        .iter()
        .filter(|e| e.descriptor.trigger.port == P::KEY)
        .map(|e| e.descriptor.trigger.selector.as_ref())
        .collect()
}

/// One invocation of `P` for each live body whose custom brain key `P` binds,
/// in the order of the body's simulation identity, so a rewind invokes them in
/// the same order. A rewind makes new entities, so an `Entity` does not decide
/// the order. `target` answers where the body's target is for this port, or
/// `None` to leave the body uninvoked this tick.
fn queue_thinks<P: Port<Value = ActorThink>>(
    admitted: &AdmittedExtensions,
    invocations: &mut ExtensionInvocations,
    bodies: &ThinkingBodies,
    target: impl Fn(Option<&ActorTarget>, Option<&ModuleObservation>) -> Option<Option<[f32; 2]>>,
) {
    let bound = bound_keys::<P>(admitted);
    if bound.is_empty() {
        return;
    }
    let mut thinking: Vec<_> = bodies
        .iter()
        .filter(|(_, config, _, _, health, ..)| {
            matches!(&config.brain, CharacterBrain::Custom(key) if bound.contains(&key.as_str())) && health.alive()
        })
        .collect();
    thinking.sort_by(|a, b| a.6.map(SimId::as_str).cmp(&b.6.map(SimId::as_str)));
    for (entity, config, kin, ground, health, actor_target, _, placement, observation) in thinking {
        let CharacterBrain::Custom(key) = &config.brain else { continue };
        let Some(target) = target(actor_target, observation) else {
            continue;
        };
        invocations.trigger::<P>(
            &ACTOR_THINK,
            key.clone(),
            entity,
            None,
            false,
            ActorThink {
                position: [kin.pos.x, kin.pos.y],
                velocity: [kin.vel.x, kin.vel.y],
                size: [kin.size().x, kin.size().y],
                facing: kin.facing,
                on_ground: ground.is_some_and(|ground| ground.on_ground),
                health: health.current(),
                target,
                placement: placement.map(|placement| placement.as_str().to_owned()),
            },
        );
    }
}

/// [`ActorThinkPort`]: the authoritative road. The target is where the body's
/// `ActorTarget` is NOW, whatever the body can see. A body the native
/// decision did not decide on this step (dormant, skipped) is not invoked.
pub fn queue_actor_thinks(
    admitted: Res<AdmittedExtensions>,
    time: Res<ambition_time::WorldTime>,
    mut invocations: ResMut<ExtensionInvocations>,
    bodies: ThinkingBodies,
    tracked: Query<&ae::BodyKinematics>,
) {
    if time.sim_dt() <= 0.0 {
        return;
    }
    queue_thinks::<ActorThinkPort>(&admitted, &mut invocations, &bodies, |target, observation| {
        // Only a body the native decision decided on this step thinks; its
        // knowledge is still the live target.
        observation?.decision?;
        Some(
            target
                .and_then(|t| t.entity)
                .and_then(|tracked_body| tracked.get(tracked_body).ok())
                .map(|body| [body.pos.x, body.pos.y]),
        )
    });
}

/// [`ActorSightedThinkPort`] (OBS3): the target this step's native decision
/// admitted for the body (`ModuleObservation::decision`). A body with no
/// decision on this step is not invoked, whatever it believed before; an empty
/// belief is `None`, never the target's live position.
pub fn queue_sighted_actor_thinks(
    admitted: Res<AdmittedExtensions>,
    time: Res<ambition_time::WorldTime>,
    mut invocations: ResMut<ExtensionInvocations>,
    bodies: ThinkingBodies,
) {
    if time.sim_dt() <= 0.0 {
        return;
    }
    queue_thinks::<ActorSightedThinkPort>(&admitted, &mut invocations, &bodies, |_, observation| {
        let decision = observation?.decision?;
        Some(decision.believed.map(|pos| [pos.x, pos.y]))
    });
}

/// Resolve each custom-brain body's MODULE observation demand from the
/// admitted bindings (OBS3), before the actor decision reads it: a body whose
/// key a sighted think entry binds demands `TargetBelief`, one an
/// authoritative entry binds demands nothing but still carries the component,
/// and a body no think binding names loses it. Compares before writing, so an
/// unchanged demand costs no change tick; a replaced or reloaded binding is
/// picked up the next tick it runs.
///
/// It also withdraws the previous step's decision product from every body,
/// before the native decision owner's eligibility filters run, so only a body
/// decided on this step carries one. A replayed step runs this again.
pub fn resolve_module_observation_demand(
    mut commands: Commands,
    admitted: Option<Res<AdmittedExtensions>>,
    mut bodies: Query<(Entity, &ActorConfig, Option<&mut ModuleObservation>)>,
) {
    let (sighted, authoritative) = admitted.as_deref().map_or((Vec::new(), Vec::new()), |admitted| {
        (bound_keys::<ActorSightedThinkPort>(admitted), bound_keys::<ActorThinkPort>(admitted))
    });
    for (entity, config, observation) in &mut bodies {
        let demand = match &config.brain {
            CharacterBrain::Custom(key) if sighted.contains(&key.as_str()) => Some(PerceptionRequirement::TargetBelief),
            CharacterBrain::Custom(key) if authoritative.contains(&key.as_str()) => Some(PerceptionRequirement::None),
            _ => None,
        };
        match (demand, observation) {
            (None, Some(_)) => {
                commands.entity(entity).remove::<ModuleObservation>();
            }
            (None, None) => {}
            (Some(demand), Some(mut observation)) => {
                if observation.demand != demand {
                    observation.demand = demand;
                }
                if observation.decision.is_some() {
                    observation.decision = None;
                }
            }
            (Some(demand), None) => {
                commands.entity(entity).insert(ModuleObservation { demand, decision: None });
            }
        }
    }
}

/// The fire a module asked for goes into the body's control frame, before the
/// control gate.
pub fn lower_actor_fires(mut outbox: ResMut<ExtensionOutbox>, mut bodies: Query<&mut ActorControl>) {
    for submitted in outbox.drain::<ActorFirePort>(&ACTOR_THINK) {
        let Ok(mut control) = bodies.get_mut(submitted.scope) else {
            warn!("extension entry {} asked a body with no control frame to fire; refused", submitted.entry);
            continue;
        };
        control.0.fire = Some(ActorFireRequest::world_space(ae::Vec2::from(submitted.value.direction)));
    }
}
