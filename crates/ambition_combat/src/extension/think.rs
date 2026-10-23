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
use ambition_characters::control::{ActorControl, DrivingParticipant};
use ambition_combat_port::{ActorFirePort, ActorThink, ActorThinkPort};
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

/// Install the trigger port and the fire request in `actor_think`.
pub fn install_for_actor_think(app: &mut App) {
    app.install_extension_trigger::<ActorThinkPort, _>(
        ACTOR_THINK,
        "ambition_combat",
        queue_actor_thinks.run_if(ambition_extension_host::trigger_is_bound::<ActorThinkPort>),
    );
    app.install_extension_submitted_request::<ActorFirePort, _>(ACTOR_THINK, "ambition_combat", lower_actor_fires);
}

/// One invocation for each live body whose custom brain key is bound, each
/// tick with a gameplay step, in the order of the body's simulation identity,
/// so a rewind invokes them in the same order. A rewind makes new entities, so
/// an `Entity` does not decide the order.
#[allow(clippy::type_complexity)]
pub fn queue_actor_thinks(
    admitted: Res<AdmittedExtensions>,
    time: Res<ambition_time::WorldTime>,
    mut invocations: ResMut<ExtensionInvocations>,
    bodies: Query<
        (
            Entity,
            &ActorConfig,
            &ae::BodyKinematics,
            Option<&ae::BodyGroundState>,
            &BodyHealth,
            Option<&ActorTarget>,
            Option<&SimId>,
            Option<&FeatureId>,
        ),
        (Without<PlayerEntity>, Without<DrivingParticipant>),
    >,
    tracked: Query<&ae::BodyKinematics>,
) {
    if time.sim_dt() <= 0.0 {
        return;
    }
    let bound: Vec<&str> = admitted
        .0
        .entries
        .iter()
        .filter(|e| e.descriptor.trigger.port == ActorThinkPort::KEY)
        .map(|e| e.descriptor.trigger.selector.as_ref())
        .collect();
    if bound.is_empty() {
        return;
    }
    let mut thinking: Vec<_> = bodies
        .iter()
        .filter_map(|(entity, config, kin, ground, health, target, id, placement)| match &config.brain {
            CharacterBrain::Custom(key) if bound.contains(&key.as_str()) && health.alive() => {
                Some((entity, key.clone(), kin, ground, health, target, id, placement))
            }
            _ => None,
        })
        .collect();
    thinking.sort_by(|a, b| a.6.map(SimId::as_str).cmp(&b.6.map(SimId::as_str)));
    for (entity, key, kin, ground, health, target, _, placement) in thinking {
        // The target where it is now: `ActorTarget` holds where the body last
        // saw it.
        let target = target
            .and_then(|t| t.entity)
            .and_then(|tracked_body| tracked.get(tracked_body).ok())
            .map(|body| [body.pos.x, body.pos.y]);
        invocations.trigger::<ActorThinkPort>(
            &ACTOR_THINK,
            key,
            entity,
            None,
            false,
            ActorThink {
                position: [kin.pos.x, kin.pos.y],
                velocity: [kin.vel.x, kin.vel.y],
                size: [kin.size.x, kin.size.y],
                facing: kin.facing,
                on_ground: ground.is_some_and(|ground| ground.on_ground),
                health: health.current(),
                target,
                placement: placement.map(|placement| placement.as_str().to_owned()),
            },
        );
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
