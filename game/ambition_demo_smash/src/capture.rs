//! **The Smash ruleset's capture adapter: authored effect keys → typed requests.**
//!
//! ```text
//! Smash authoring     an EffectRef on a move's window or timeline
//!        ↓
//! THIS MODULE         recognises the key, hydrates the params
//!        ↓
//! combat/body runtime CaptureAttemptRequested / Pummel / Throw
//! ```
//!
//! ⛔ **the generic body runtime never matches `"smash.capture_throw"`**, and
//! this adapter never touches body ECS state. Each half does the thing it is the
//! right place for: a ruleset knows what its own authored strings mean, and a
//! body runtime knows how to hold and launch a body. Collapsing them would put
//! Smash vocabulary in the engine or body surgery in the game, and both are the
//! dependency this split exists to avoid.
//!
//! ⚠ an unrecognised key falls through untouched — other techniques ride the
//! same channel, and a `continue` here is how they stay unaffected.

use bevy::prelude::*;

use ambition_platformer2d::characters::brain::action_set::{ActionRequest, SpecialActionSpec};
use ambition_platformer2d::characters::brain::ActorActionMessage;
use ambition_platformer2d::characters::smash_capture::{
    CaptureAttemptParams, CapturePummelParams, CaptureThrowParams, CAPTURE_ATTEMPT, CAPTURE_PUMMEL,
    CAPTURE_THROW,
};
use ambition_platformer2d::combat::capture::{
    CaptureAttemptRequested, CapturePummelRequested, CaptureThrowRequested,
};
use ambition_platformer2d::engine_core as ae;

/// Translate this tick's authored capture effects into typed runtime requests.
pub fn translate_smash_capture_effects(
    mut actions: MessageReader<ActorActionMessage>,
    mut attempts: MessageWriter<CaptureAttemptRequested>,
    mut pummels: MessageWriter<CapturePummelRequested>,
    mut throws: MessageWriter<CaptureThrowRequested>,
) {
    for message in actions.read() {
        let ActionRequest::Special { spec, params } = &message.request else {
            continue;
        };
        let SpecialActionSpec::Special(key) = spec else {
            continue;
        };
        match key.as_str() {
            CAPTURE_ATTEMPT => {
                // ⚠ a params typo is a STARTUP error, not a silent default: the
                // key registers `check_hydrates` with the param-schema registry,
                // so a fighter's bad grab data fails the content pass. Reaching
                // here with unhydratable params means the registration is
                // missing, which is worth the log rather than a silent skip.
                match params.hydrate::<CaptureAttemptParams>() {
                    Ok(p) => attempts.write(CaptureAttemptRequested {
                        captor: message.actor,
                        offset: ae::Vec2::new(p.offset.0, p.offset.1),
                        half_extents: ae::Vec2::new(p.half_extents.0, p.half_extents.1),
                        hold_offset: ae::Vec2::new(p.hold_offset.0, p.hold_offset.1),
                    }),
                    Err(err) => {
                        warn!("smash capture attempt params did not hydrate: {err}");
                        continue;
                    }
                };
            }
            CAPTURE_PUMMEL => match params.hydrate::<CapturePummelParams>() {
                Ok(p) => {
                    pummels.write(CapturePummelRequested {
                        captor: message.actor,
                        damage: p.damage,
                    });
                }
                Err(err) => warn!("smash pummel params did not hydrate: {err}"),
            },
            CAPTURE_THROW => match params.hydrate::<CaptureThrowParams>() {
                Ok(p) => {
                    throws.write(CaptureThrowRequested {
                        captor: message.actor,
                        damage: p.damage,
                        knockback: p.knockback,
                        knockback_growth: p.knockback_growth,
                        launch_dir: ae::Vec2::new(p.launch_dir.0, p.launch_dir.1),
                    });
                }
                Err(err) => warn!("smash throw params did not hydrate: {err}"),
            },
            _ => continue,
        }
    }
}
