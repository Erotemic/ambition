//! The speedway badnik: a stompable, roll-through-able walker, pure content.
//!
//! The Mary-O crony pattern applied to Sanic's verbs, with no engine edits:
//!
//! - Placement comes from the demo's LDtk file: `EnemySpawn` entities with
//!   `brain: "sanic_badnik"` (or another of his badnik kinds) lower into
//!   `RoomSpec::enemy_spawns`, and the engine's room staging spawns them.
//! - Body, walk, and contact damage come from the generic combatant, with the
//!   health and movement of the kind's character row.
//! - The defeat is Sanic's, not Mary-O's: a descending bounce on the head
//!   (classic stomp), or any touch while rolling (ball dash or crouch-roll) or
//!   while the touch harms (the super form). It is a module
//!   (`ambition_demo_sanic_modules::badnik`) on the `body_contact` road, so it
//!   decides before the shared contact damage and the squashed badnik hurts
//!   nobody that tick. What it reads of this game is answered here: his
//!   pack's badnik rules, and whether each toucher rolls or harms.
//!
//! Every type it names comes through the `ambition_platformer2d` umbrella (the
//! E9 oracle).

use bevy::prelude::*;

use ambition_demo_sanic_modules::rules::{BadnikRulesPort, ContactTouchersPort, ToucherFacts};
use ambition_platformer2d::extension::sdk::phases::BODY_CONTACT;
use ambition_platformer2d::extension::{ExtensionAppExt, ObservationContext};

/// The roster brain key of the plain badnik.
pub use ambition_demo_sanic_modules::badnik::{BADNIK_BRAIN_KEY, BADNIK_KINDS};

/// Answer the squash module's observations, and declare its module. A
/// composition with no extension host admits no module and needs no port.
pub fn install(app: &mut App) {
    if app.has_extension_host() {
        app.install_extension_observation::<BadnikRulesPort>(BODY_CONTACT, "ambition_demo_sanic", |_, _| {
            Some(crate::rules::sanic_rules().badnik_observed())
        });
        app.install_extension_context_observation::<ContactTouchersPort>(
            BODY_CONTACT,
            "ambition_demo_sanic",
            touchers_of,
        );
    }
    for module in ambition_demo_sanic_modules::modules() {
        app.add_extension_module(module);
    }
}

/// Each toucher of the contact this invocation answers, in the trigger's
/// order: whether it rolls, and whether its touch harms. Squashing on touch is
/// a trait the body holds (its grant and the character it wears), not a name
/// it wears.
fn touchers_of(world: &World, context: ObservationContext<'_>) -> Option<Vec<ToucherFacts>> {
    use ambition_platformer2d::combat::empowerment::{empowerment_of, Empowered};
    use ambition_platformer2d::characters::actor::CharacterForm;
    let table = world.get_resource::<ambition_platformer2d::combat::extension::contact::ContactTouchers>()?;
    let Some((_, _, touchers)) = table.0.iter().find(|(body, ..)| *body == context.scope) else {
        return Some(Vec::new());
    };
    let cast = ambition_platformer2d::characters::session_cast::session_cast(world);
    Some(
        touchers
            .iter()
            .map(|&toucher| {
                let Ok(body) = world.get_entity(toucher) else {
                    return ToucherFacts::default();
                };
                ToucherFacts {
                    rolling: body.contains::<crate::ball_dash::Rolling>(),
                    harms_on_contact: empowerment_of(body.get::<Empowered>(), body.get::<CharacterForm>(), cast)
                        .holds(ambition_platformer2d::character::Empowerment::HARMS_ON_CONTACT),
                }
            })
            .collect(),
    )
}
