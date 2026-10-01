//! Boss special-attack Techniques. Every one is a procedural module now
//! (`ambition_content_modules`): it reads the boss's trigger port and submits
//! typed requests, and the engine owns no boss-special behaviour. The modules
//! are declared here, by [`BossSpecialContentPlugin`].
//!
//! Their old native systems are kept as test-only reference traces
//! (`*_reference_tests.rs`), and `module_parity_tests` holds every module to
//! them, on the linked and the WASM road.

use bevy::prelude::*;

/// Which actors a `Special(<key>)` message names this tick, and **the use of
/// the move that asked for it**.
///
/// ⛔⛤ THE OCCURRENCE COMES FROM THE MESSAGE, AND A TECHNIQUE MUST NOT
/// RE-DERIVE IT. A technique frequently spawns a projectile that flies for
/// seconds. By the time the bolt lands, the move that fired it can be over and
/// a different move can be playing on the same body. A damage result that
/// carries no occurrence is credited to whatever plays at that moment — the A12
/// defect. So the occurrence travels with the request, and reading the owner's
/// `MovePlayback` inside a technique reintroduces exactly the bug this avoids.
///
/// ⭐ `None` IS A CORRECT ANSWER. A boss brain that presses its special
/// directly, with no move behind it, has no occurrence to give. Techniques pass
/// the `Option` through unchanged rather than inventing a number.
///
/// Returns a map rather than a set because every caller needs the value, and a
/// set makes the value unavailable at exactly the site that has to spend it.
#[cfg(test)]
pub(crate) fn actors_firing(
    messages: &mut MessageReader<ambition_characters::brain::ActorActionMessage>,
    key: &str,
) -> std::collections::HashMap<Entity, Option<u32>> {
    use ambition_characters::brain::action_set::{ActionRequest, SpecialActionSpec};
    let mut firing = std::collections::HashMap::new();
    for msg in messages.read() {
        if let ActionRequest::Special {
            spec: SpecialActionSpec::Special(fired),
            ..
        } = &msg.request
        {
            if fired == key {
                firing.insert(msg.actor, msg.move_instance);
            }
        }
    }
    firing
}

#[cfg(test)]
mod apple_rain_reference_tests;
#[cfg(test)]
mod echo_fan_reference_tests;
#[cfg(test)]
mod eye_beam_reference_tests;
#[cfg(test)]
mod gradient_cascade_reference_tests;
#[cfg(test)]
mod gradient_nova_reference_tests;
#[cfg(test)]
mod minima_trap_reference_tests;
#[cfg(test)]
mod mode_collapse_reference_tests;
#[cfg(test)]
pub(crate) mod module_parity_tests;
#[cfg(test)]
mod overfit_volley_reference_tests;
#[cfg(test)]
mod overflow_flood_reference_tests;
#[cfg(test)]
mod saddle_point_reference_tests;
#[cfg(test)]
mod seismic_stomp_reference_tests;

use ambition_extension_host::ExtensionAppExt;

/// Declares the boss-special modules. Their per-boss records live in the
/// extension host's store (`extension.body_records`), so a boss carries no
/// technique component and this crate registers no technique rollback state.
///
/// Installed by [`super::AmbitionBossContentPlugin`].
pub struct BossSpecialContentPlugin;

impl Plugin for BossSpecialContentPlugin {
    fn build(&self, app: &mut App) {
        for module in ambition_content_modules::modules() {
            app.add_extension_module(module);
        }
    }
}
