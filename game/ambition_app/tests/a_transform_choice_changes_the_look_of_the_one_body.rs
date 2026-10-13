#![cfg(feature = "rl_sim")]
//! A character with a transform is one character, and the Hall shows each of
//! its looks on one body.
//!
//! Jon, 2026-10-13: "We also need to make Hinter's transformed variant and
//! normal variant two sides of the same character. ... In the hall each
//! transform character should have a dialog option that requests that they do
//! a transform to a particular state".
//!
//! The shipped app, the generated Hall and the three shipped catalogs. The
//! conversation's `<<transform>>` writes a `WearRequested`; this writes the
//! same message, because the simulation harness has no dialog box to choose
//! in (the choice itself is in `hall.yarn`, and the dialogue lint reads it).

use crate::common::{base, fixed_60hz_room_sim};
use ambition_platformer2d::characters::actor::{RecharacterizeBody, WornCharacter};
use bevy::prelude::{Entity, Name};

fn wearers(sim: &mut ambition_app::Platformer2dSimHarness, character: &str) -> Vec<Entity> {
    let world = sim.world_mut();
    let mut query = world.query::<(Entity, &WornCharacter)>();
    query
        .iter(world)
        .filter(|(_, worn)| worn.id() == character)
        .map(|(entity, _)| entity)
        .collect()
}

fn ask(sim: &mut ambition_app::Platformer2dSimHarness, body: Entity, character: &str) {
    let id = ambition_platformer2d::platformer::lifecycle::LiveBodyId::of_entity(sim.world(), body)
        .expect("a placed body has a SimId");
    sim.world_mut()
        .write_message(ambition_platformer2d::actors::features::WearRequested {
            body: id,
            character: WornCharacter::new(character).character().clone(),
        });
}

#[test]
fn the_hall_seats_a_character_once_and_a_transform_puts_each_look_on_that_body() {
    let mut sim = fixed_60hz_room_sim("hall_of_characters");
    sim.step_n(base(), 10);

    for (character, looks) in [
        ("npc_jeff_hinter", &["npc_jeff_hinter_armored"][..]),
        ("sanic", &["super_sanic"][..]),
        ("mary_o", &["mary_o_tall", "mary_o_fire", "mary_o_cosmic"][..]),
    ] {
        let bodies = wearers(&mut sim, character);
        assert_eq!(bodies.len(), 1, "the Hall seats `{character}` {} times", bodies.len());
        let body = bodies[0];
        let worn = |sim: &ambition_app::Platformer2dSimHarness| {
            sim.world().get::<WornCharacter>(body).expect("the body stays").id().to_string()
        };
        let name = |sim: &ambition_app::Platformer2dSimHarness| {
            sim.world().get::<Name>(body).map(|name| name.as_str().to_string())
        };
        let own_name = name(&sim);
        let mut name_of_a_look = None;

        for look in looks {
            assert!(
                wearers(&mut sim, look).is_empty(),
                "`{look}` is a look of `{character}`, and the Hall gives it a place of its own"
            );
            ask(&mut sim, body, look);
            sim.step_n(base(), 4);
            // A look the prepared cast does not hold is refused, and the body
            // wears again what it wore: this is the look, applied.
            assert_eq!(worn(&sim), *look, "`{character}` did not become `{look}`");
            assert!(
                sim.world().get::<RecharacterizeBody>(body).is_none(),
                "the template of `{look}` was asked for and never applied"
            );
            assert_ne!(name(&sim), own_name, "`{character}` wears `{look}` and is still named as before");
            name_of_a_look = name(&sim);
        }

        // The way back: the same body is the character again.
        ask(&mut sim, body, character);
        sim.step_n(base(), 4);
        assert_eq!(worn(&sim), character);
        // (A body that is built is named by its placement, and a body that
        // wears is named by its character, so the name is not the first one.)
        assert_ne!(name(&sim), name_of_a_look, "`{character}` is itself again and has the name of a look");
        assert_eq!(wearers(&mut sim, character), vec![body]);
    }
}
