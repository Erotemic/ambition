//! The adapters of a fighter's special: `ambition.fighter.special` and
//! `ambition.combat.body_strike`. See the port cards in
//! `ambition_combat_port::special`.
//!
//! A move's effect whose key a module binds is that module's to answer. The
//! module reads the fighter's facts and the move's authored params, and asks
//! for what the special does; a demo's special can then be a module and not
//! Rust in the demo crate.

use ambition_characters::brain::action_set::{ActionRequest, SpecialActionSpec};
use ambition_characters::brain::ActorActionMessage;
use ambition_combat_port::{BodyStrikePort, FighterSpecial, SpecialParam};
pub use ambition_combat_port::FighterSpecialPort;
use ambition_entity_catalog::ParamLeaf;
use ambition_extension_host::{AdmittedExtensions, ExtensionAppExt, ExtensionInvocations, ExtensionOutbox};
use ambition_extension_sdk::phases::TECHNIQUE_EXECUTION;
use ambition_extension_sdk::Port;
use ambition_platformer2d_core as ae;
use bevy::prelude::*;

/// Install the trigger and the strike request in `technique_execution`.
pub fn install_for_specials(app: &mut App) {
    app.install_extension_trigger::<FighterSpecialPort, _>(TECHNIQUE_EXECUTION, "ambition_combat", queue_fighter_specials);
    app.install_extension_request::<BodyStrikePort, _>(TECHNIQUE_EXECUTION, "ambition_combat", lower_body_strikes);
}

/// One invocation for each press of a bound special, in the order of the
/// presses.
pub fn queue_fighter_specials(
    admitted: Res<AdmittedExtensions>,
    mut messages: MessageReader<ActorActionMessage>,
    mut invocations: ResMut<ExtensionInvocations>,
    bodies: Query<&ae::BodyKinematics>,
) {
    let bound: Vec<&str> = admitted
        .0
        .entries
        .iter()
        .filter(|e| e.descriptor.trigger.port == FighterSpecialPort::KEY)
        .map(|e| e.descriptor.trigger.selector.as_ref())
        .collect();
    for message in messages.read() {
        let ActionRequest::Special { spec: SpecialActionSpec::Special(key), params } = &message.request else {
            continue;
        };
        if !bound.contains(&key.as_str()) {
            continue;
        }
        let Ok(kin) = bodies.get(message.actor) else {
            continue;
        };
        let params = params
            .leaves()
            .into_iter()
            .map(|(path, leaf)| {
                let value = match leaf {
                    ParamLeaf::Number(n) => SpecialParam::Number(n as f32),
                    ParamLeaf::Text(text) => SpecialParam::Text(text),
                    ParamLeaf::Flag(flag) => SpecialParam::Flag(flag),
                };
                (path, value)
            })
            .collect();
        invocations.trigger::<FighterSpecialPort>(
            &TECHNIQUE_EXECUTION,
            key.clone(),
            message.actor,
            message.move_instance,
            false,
            FighterSpecial {
                position: [kin.pos.x, kin.pos.y],
                facing: kin.facing.signum(),
                params,
            },
        );
    }
}

/// Each strike a module asked for, on the body it ran for, as a move's own
/// volume: it follows the body, and the body's side decides whom it hurts.
pub fn lower_body_strikes(
    mut commands: Commands,
    mut outbox: ResMut<ExtensionOutbox>,
    bodies: Query<(&ae::BodyKinematics, &ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame)>,
) {
    for submitted in outbox.drain::<BodyStrikePort>(&TECHNIQUE_EXECUTION) {
        let Ok((kin, frame)) = bodies.get(submitted.scope) else {
            warn!("extension entry {} asked for a strike on a body with no frame; refused", submitted.entry);
            continue;
        };
        let strike = submitted.value;
        let facing = kin.facing.signum();
        crate::strike::spawn_body_strike(
            &mut commands,
            submitted.scope,
            ae::Vec2::new(facing * strike.forward[0], strike.forward[1]),
            facing,
            frame.basis(),
            ae::Vec2::from(strike.half_extent),
            strike.damage,
            strike.knockback,
            strike.lifetime_s,
            strike.hit_sound.as_deref().map(ambition_sfx::SfxId::new),
        );
    }
}
