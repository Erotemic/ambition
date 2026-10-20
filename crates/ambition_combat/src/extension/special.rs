//! The adapters of a fighter's special: `ambition.fighter.special`,
//! `ambition.combat.body_strike`, `ambition.combat.sleep_area`,
//! `ambition.resources.refill` and `ambition.combat.parry_window`. See the
//! port cards in `ambition_combat_port::special`. A body's statuses, which a
//! special's module holds on a body, are `super::status`.
//!
//! A move's effect whose key a module binds is that module's to answer. The
//! module reads the fighter's facts and the move's authored params, and asks
//! for what the special does; a demo's special can then be a module and not
//! Rust in the demo crate.

use ambition_characters::brain::action_set::{ActionRequest, SpecialActionSpec};
use ambition_characters::brain::ActorActionMessage;
use ambition_combat_port::{
    BodyStrikePort, FighterSpecial, ParryWindowPort, RefillResourcePort, SleepAreaPort, SpecialParam,
};
pub use ambition_combat_port::FighterSpecialPort;
use ambition_entity_catalog::ParamLeaf;
use ambition_extension_host::{AdmittedExtensions, ExtensionAppExt, ExtensionInvocations, ExtensionOutbox};
use ambition_extension_sdk::phases::TECHNIQUE_EXECUTION;
use ambition_extension_sdk::Port;
use ambition_platformer2d_core as ae;
use bevy::math::bounding::IntersectsVolume as _;
use bevy::prelude::*;

/// Install the trigger and the special's requests in `technique_execution`.
pub fn install_for_specials(app: &mut App) {
    app.install_extension_trigger::<FighterSpecialPort, _>(TECHNIQUE_EXECUTION, "ambition_combat", queue_fighter_specials);
    app.install_extension_request::<BodyStrikePort, _>(TECHNIQUE_EXECUTION, "ambition_combat", lower_body_strikes);
    app.install_extension_request::<SleepAreaPort, _>(TECHNIQUE_EXECUTION, "ambition_combat", lower_sleep_areas);
    app.install_extension_request::<RefillResourcePort, _>(TECHNIQUE_EXECUTION, "ambition_combat", lower_refills);
    app.install_extension_request::<ParryWindowPort, _>(TECHNIQUE_EXECUTION, "ambition_combat", lower_parry_windows);
    super::status::install(app);
    super::targets::install(app);
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

/// Each sleep a module asked for: every other body of the singer's live room
/// whose box meets the area sleeps for at least the asked time.
///
/// A floor, not an addition: two overlapping pulses take the longer one.
pub fn lower_sleep_areas(
    mut outbox: ResMut<ExtensionOutbox>,
    singers: Query<(&ae::BodyKinematics, &ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame)>,
    mut sleepers: Query<(Entity, &ae::CenteredAabb, &mut ambition_characters::actor::BodyCombat)>,
    rooms: ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
) {
    for submitted in outbox.drain::<SleepAreaPort>(&TECHNIQUE_EXECUTION) {
        let Ok((kin, frame)) = singers.get(submitted.scope) else {
            warn!("extension entry {} asked for a sleep around a body with no frame; refused", submitted.entry);
            continue;
        };
        let area = submitted.value;
        let basis = frame.basis();
        let facing = kin.facing.signum();
        let centre = kin.pos + basis.to_world(ae::Vec2::new(facing * area.forward[0], area.forward[1]));
        let reach = ae::CenteredAabb::new(centre, basis.to_world_half(ae::Vec2::from(area.half_extent))).aabb();
        // Another live room's bodies stand in another world: the song does
        // not reach them, whatever their coordinates.
        let room = rooms.of(submitted.scope);
        for (body, aabb, mut combat) in &mut sleepers {
            if body == submitted.scope || rooms.of(body) != room || !reach.intersects(&aabb.aabb()) {
                continue;
            }
            if combat.sleep_timer < area.duration_s {
                combat.sleep_timer = area.duration_s;
            }
        }
    }
}

/// Add each refill a module asked for to the named resource of its body.
pub fn lower_refills(
    mut outbox: ResMut<ExtensionOutbox>,
    mut banks: Query<&mut ambition_platformer2d_core::resources::ActorResources>,
) {
    for submitted in outbox.drain::<RefillResourcePort>(&TECHNIQUE_EXECUTION) {
        let refill = submitted.value;
        if !refill.amount.is_finite() || refill.amount < 0.0 {
            warn!(
                "extension entry {} asked to refill `{}` by {}; a refill only adds",
                submitted.entry, refill.resource, refill.amount
            );
            continue;
        }
        let resource = ambition_resource_spec::ResourceId::new(refill.resource);
        // A body that holds no such resource gains nothing (see the port card).
        if let Some(level) = banks
            .get_mut(submitted.scope)
            .ok()
            .and_then(|bank| bank.into_inner().level_of_mut(&resource))
        {
            level.refill(refill.amount);
        }
    }
}

/// Hold open each parry window a module asked for, on its body's guard.
///
/// The timer is the longer of the two, not the newer: a stance re-arms it
/// every live tick, and a short re-arm must not cut a longer window short.
/// It never sets `active`: `parrying()` reads the timer alone, and a raised
/// shield would add a held guard's blocking, guard cost and shieldstun.
pub fn lower_parry_windows(mut outbox: ResMut<ExtensionOutbox>, mut guards: Query<&mut ae::BodyShieldState>) {
    for submitted in outbox.drain::<ParryWindowPort>(&TECHNIQUE_EXECUTION) {
        let window = submitted.value;
        if !window.window_s.is_finite() || window.window_s <= 0.0 {
            warn!(
                "extension entry {} asked for a parry window of {} s; a window is more than 0",
                submitted.entry, window.window_s
            );
            continue;
        }
        let Ok(mut shield) = guards.get_mut(submitted.scope) else {
            continue;
        };
        shield.parry_window_timer = shield.parry_window_timer.max(window.window_s);
        // Armed on the same heartbeat, so a stance that stops asking stops
        // absorbing and parrying on the same tick.
        if window.absorbs_projectiles {
            shield.absorb_window_timer = shield.absorb_window_timer.max(window.window_s);
        }
    }
}
