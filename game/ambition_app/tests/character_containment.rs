//! Every character in the shipped host stays inside a plain room.
//!
//! Sanic's surface-momentum policy had no horizontal collision on its riding arm for the whole
//! life of the project, and nothing noticed, because the only level it was ever played in was a
//! hand-authored chain course with nothing to run into. Put him in a room with walls and he ran
//! out of it and fell forever.
//!
//! The instrument is embarrassingly cheap: put the character in a box, hold a
//! direction, check it is still in the box. `ambition_platformer2d_core` owns the
//! probe so a game embedding this engine can run it over its own cast; this
//! test is the half that knows the POPULATION — every character the shipped
//! host registers, which is where a provider's new cast member arrives.
//!
//! ## Why the catalog and not the prepared registry
//!
//! The catalog is what `motion_model_spec_for_character_id` reads, so it is the
//! authority on which policy a character actually plays under. Probing anything
//! else would test a copy of the decision rather than the decision.

use ambition_platformer2d::characters::actor::character_catalog::CharacterCatalog;
use ambition_platformer2d::engine_core::movement::containment::{
    probe_containment, walled_box, ContainmentProbe,
};
use ambition_platformer2d::engine_core::{Aabb, LocalAxes, Vec2};
use ambition_app::app::{build_visible_app, VisibleRenderMode};

const ROOM: Vec2 = Vec2::new(960.0, 540.0);
const WALL_PX: f32 = 16.0;

#[test]
fn no_registered_character_can_leave_a_plain_walled_room() {
    let app = build_visible_app(VisibleRenderMode::NoWindow, true);
    let catalog = app.world().resource::<CharacterCatalog>();
    let world = walled_box(ROOM, WALL_PX);
    let bounds = Aabb {
        min: Vec2::ZERO,
        max: ROOM,
    };

    // Sorted, so a failure names the same character on every run and the
    // population is not at the mercy of map iteration order.
    let mut ids: Vec<&String> = catalog.iter().map(|(id, _)| id).collect();
    ids.sort();
    assert!(
        !ids.is_empty(),
        "the shipped host registered no characters at all, so this is probing \
         an empty population and would pass forever"
    );

    let mut escapes = Vec::new();
    let mut sized = 0usize;
    for id in &ids {
        let spec = ambition_platformer2d::actors::avatar::motion_model_spec_for_character_id(catalog, id);
        // The character's OWN body, not a generic one.
        //
        // `None` for a character whose sheet publishes no body metrics — the
        // probe falls back to the engine default rather than inventing a size,
        // and the count below keeps that from quietly becoming everybody.
        let body = ambition_platformer2d::actors::character_sprites::sprite_body_collision_for_character_id_in(
            &Default::default(),
            catalog,
            id,
            Vec2::new(28.0, 44.0),
        )
        .map(|collision| collision.collision);
        if body.is_some() {
            sized += 1;
        }
        // BOTH directions.
        for (label, axes) in [
            ("right", LocalAxes::new(1.0, 0.0)),
            ("left", LocalAxes::new(-1.0, 0.0)),
        ] {
            let probe = match body {
                Some(size) => ContainmentProbe::holding(axes).with_body_size(size),
                None => ContainmentProbe::holding(axes),
            };
            let outcome = probe_containment(&world, spec, world.spawn, bounds, probe);
            if !outcome.contained() {
                escapes.push(format!(
                    "{id} holding {label}: left the room by {:.0}px (ended at \
                     {:?})",
                    outcome.max_escape_px, outcome.final_pos
                ));
            }
        }
    }

    eprintln!(
        "[containment] {} characters x 2 directions ({sized} with authored body \
         sizes, the rest on the engine default)",
        ids.len()
    );
    assert!(
        sized * 2 > ids.len(),
        "only {sized} of {} characters resolved an authored body size, so this \
         is mostly probing the default box and the row's claim about \
         character-level containment would be hollow",
        ids.len()
    );
    assert!(
        escapes.is_empty(),
        "{} of {} characters can walk out of a plain walled room:\n  {}\n\n\
         A character that cannot be contained by four solid blocks cannot be \
         played in any level the LDtk importer produces — the room is not the \
         problem, the movement policy is.",
        escapes.len(),
        ids.len() * 2,
        escapes.join("\n  ")
    );
}
