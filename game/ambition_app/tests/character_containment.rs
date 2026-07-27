//! **Every character in the shipped host stays inside a plain room.** (queue L6)
//!
//! L1 and L2 were the same shape, and neither was found by a test: a movement
//! policy and a level, each correct alone, broken together. Sanic's
//! surface-momentum policy had no horizontal collision on its riding arm for
//! the whole life of the project, and nothing noticed, because the only level
//! it was ever played in was a hand-authored chain course with nothing to run
//! into. Put him in a room with walls and he ran out of it and fell forever.
//!
//! The instrument is embarrassingly cheap: put the character in a box, hold a
//! direction, check it is still in the box. `ambition_engine_core` owns the
//! probe so a game embedding this engine can run it over its own cast; this
//! test is the half that knows the POPULATION — every character the shipped
//! host registers, which is where a provider's new cast member arrives.
//!
//! ## Why the catalog and not the prepared registry
//!
//! The catalog is what `motion_model_spec_for_character_id` reads, so it is the
//! authority on which policy a character actually plays under. Probing anything
//! else would test a copy of the decision rather than the decision.

use ambition::characters::actor::character_catalog::CharacterCatalog;
use ambition::engine_core::movement::containment::{
    probe_containment, walled_box, ContainmentProbe,
};
use ambition::engine_core::{Aabb, LocalAxes, Vec2};
use ambition_app::app::{build_visible_app, VisibleRenderMode};

/// A room the size of the versus arena — the smallest thing anybody would call
/// a level, and the one the defect was found in.
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
    for id in &ids {
        let spec = ambition::actors::avatar::motion_model_spec_for_character_id(catalog, id);
        // BOTH directions. A wall on one side is not evidence about the other,
        // and an asymmetric solver bug is exactly the kind that survives a
        // one-sided probe.
        for (label, axes) in [
            ("right", LocalAxes::new(1.0, 0.0)),
            ("left", LocalAxes::new(-1.0, 0.0)),
        ] {
            let outcome = probe_containment(
                &world,
                spec,
                world.spawn,
                bounds,
                ContainmentProbe::holding(axes),
            );
            if !outcome.contained() {
                escapes.push(format!(
                    "{id} holding {label}: left the room by {:.0}px (ended at \
                     {:?})",
                    outcome.max_escape_px, outcome.final_pos
                ));
            }
        }
    }

    eprintln!("[containment] {} characters x 2 directions", ids.len());
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
