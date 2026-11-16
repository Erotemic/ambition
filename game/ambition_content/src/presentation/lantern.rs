//! The Lantern (dark rooms and tunnels, TN3): while the bag holds it, the
//! players' light in a dark room reaches farther and is warm. The first item
//! whose worth is seeing; presentation only, so it changes what a player
//! sees and nothing a peer or a replay computes.

use bevy::prelude::*;

use ambition_items::{Item, OwnedItems};
use ambition_platformer2d::presentation::PlayerLight;

/// How far a player's light reaches with the lantern (without it, 200).
const LANTERN_RADIUS: f32 = 360.0;
/// The lantern's flame, sRGB.
const LANTERN_COLOR: [f32; 3] = [1.0, 0.78, 0.48];

pub(super) fn install(app: &mut App) {
    app.add_systems(Update, carry_the_lantern);
}

/// Widen the players' light while the bag holds the lantern, and give it
/// back when the bag does not (a load of a save without it).
fn carry_the_lantern(owned: Option<Res<OwnedItems>>, light: Option<Res<PlayerLight>>, mut commands: Commands) {
    let carried = owned.is_some_and(|owned| owned.has(Item::Lantern));
    let wanted =
        if carried { PlayerLight { radius: LANTERN_RADIUS, color: Some(LANTERN_COLOR) } } else { PlayerLight::default() };
    if light.as_deref() != Some(&wanted) {
        commands.insert_resource(wanted);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_lantern_widens_the_players_light_while_it_is_in_the_bag() {
        let mut app = App::new();
        app.init_resource::<OwnedItems>();
        install(&mut app);
        app.update();
        assert_eq!(*app.world().resource::<PlayerLight>(), PlayerLight::default());
        app.world_mut().resource_mut::<OwnedItems>().grant(ambition_items::test_item_catalog(), Item::Lantern, 1);
        app.update();
        let light = *app.world().resource::<PlayerLight>();
        assert_eq!(light.radius, LANTERN_RADIUS);
        assert_eq!(light.color, Some(LANTERN_COLOR));
        app.insert_resource(OwnedItems::default());
        app.update();
        assert_eq!(*app.world().resource::<PlayerLight>(), PlayerLight::default(), "a bag without it gives it back");
    }
}
