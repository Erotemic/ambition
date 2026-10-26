//! A player body that is not the first player has Mary-O's powers: the form
//! ladder and the quasar. They read `PrimaryPlayer` only.

use bevy::prelude::*;

use ambition_platformer2d::actors::empowerment::Empowered;
use ambition_platformer2d::characters::actor::WornCharacter;
use ambition_platformer2d::characters::equipment::WornEquipment;
use ambition_platformer2d::platformer::markers::PrimaryPlayer;

/// The demo in 1-1 with her seated and settled.
fn seated() -> (App, Entity) {
    let mut app = ambition_demo_mary_o_app::build_demo_app();
    let body = (0..600)
        .find_map(|_| {
            app.update();
            let world = app.world_mut();
            world.query_filtered::<Entity, With<PrimaryPlayer>>().iter(world).next()
        })
        .expect("Mary-O takes a seat");
    for _ in 0..30 {
        app.update();
    }
    (app, body)
}

/// Equip `row` on `body`, run some ticks, and read what she wears and
/// whether the quasar burns.
fn after_equipping(app: &mut App, body: Entity, row: ambition_platformer2d::characters::equipment::EquipmentRow) -> (String, bool) {
    let mut entity = app.world_mut().entity_mut(body);
    if !entity.contains::<WornEquipment>() {
        entity.insert(WornEquipment::default());
    }
    entity.get_mut::<WornEquipment>().unwrap().equip(row);
    for _ in 0..10 {
        app.update();
    }
    let worn = app.world().get::<WornCharacter>(body).unwrap().id().to_string();
    (worn, app.world().get::<Empowered>(body).is_some())
}

/// ⭐ Each power works the same for the first player (the control) and for
/// a player body that is not the first player.
#[test]
fn a_player_who_is_not_the_first_player_has_her_powers() {
    let mut seen = Vec::new();
    for first in [true, false] {
        let (mut app, body) = seated();
        if !first {
            app.world_mut().entity_mut(body).remove::<PrimaryPlayer>();
        }
        let before = app.world().get::<WornCharacter>(body).unwrap().id().to_string();
        let (grown, _) = after_equipping(&mut app, body, ambition_demo_mary_o::powerups::star_wand());
        let (_, star) = after_equipping(&mut app, body, ambition_demo_mary_o::star::pocket_quasar());
        seen.push((first, before != grown, star));
    }
    assert_eq!(seen[0], (true, true, true), "the control: the first player grows and the quasar burns");
    assert_eq!(seen[1], (false, true, true), "a player who is not the first grows, and the quasar burns");
}
