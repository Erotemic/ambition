//! A room look takes back what it gave to things that are not its own when
//! the quality tier has no budget for it, and gives it again when the budget
//! comes back. The room does not load again in between.
//!
//! A look draws its own pieces (the quads with its material), and it also
//! dresses the room's doors (`EntityArt`) and inks the signs on its pale side
//! (`WorldLabel` colours, and a halo of four copies under the sign). On Potato
//! the pieces were despawned and the doors and signs kept their dress over
//! plain blocks (review of f0409fcc, 2026-10-09).

use ambition_content::presentation::room_look::{RoomPlateMaterial, RoomStateMaterial};
use ambition_platformer2d::persistence::settings::{UserSettings, VisualQualityProfile};
use ambition_platformer2d::render::rendering::label_layout::{StaticWorldLabel, WorldLabel};
use ambition_platformer2d::render::rendering::{EntityArt, LoadingZoneVisual};
use ambition_platformer2d::sprite_sheet::game_assets::EntitySprite;
use bevy::prelude::*;
use bevy::sprite_render::MeshMaterial2d;

/// What the look has put in the room.
#[derive(Debug, Clone, PartialEq)]
struct Dress {
    /// Quads that draw the look with a shader: the sky, the overlay.
    pieces: usize,
    /// Quads that draw the look's architecture from its plates.
    plates: usize,
    /// Doors whose art is the look's and not the art of their kind.
    look_doors: usize,
    /// Each static sign's colours, by owner, and the text copies under it.
    signs: Vec<(String, [u8; 4], Option<[u8; 4]>, usize)>,
}

fn dress(app: &mut App) -> Dress {
    let world = app.world_mut();
    let pieces = world.query_filtered::<(), With<MeshMaterial2d<RoomStateMaterial>>>().iter(world).count();
    let plates = world.query_filtered::<(), With<MeshMaterial2d<RoomPlateMaterial>>>().iter(world).count();
    let look_doors = world
        .query_filtered::<&EntityArt, With<LoadingZoneVisual>>()
        .iter(world)
        .filter(|art| !matches!(art.0, EntitySprite::DoorZone | EntitySprite::EdgeExit))
        .count();
    let bytes = |color: Color| color.to_srgba().to_u8_array();
    let mut signs: Vec<_> = world
        .query_filtered::<(&WorldLabel, Option<&Children>), With<StaticWorldLabel>>()
        .iter(world)
        .map(|(label, children)| {
            (
                label.owner_id.clone(),
                bytes(label.text_color),
                label.outline_color.map(bytes),
                children.map_or(0, |children| children.len()),
            )
        })
        .collect();
    signs.sort();
    Dress { pieces, plates, look_doors, signs }
}

fn set_tier(app: &mut App, profile: VisualQualityProfile) {
    app.world_mut().resource_mut::<UserSettings>().video.quality.profile = profile;
    for _ in 0..30 {
        app.update();
    }
}

#[test]
fn a_look_takes_its_doors_and_its_ink_back_on_potato_and_gives_them_again() {
    let mut app = crate::an_edit_reaches_the_shipped_game::a_running_shipped_session();
    // This app has no renderer, so no plugin made the registry of the look's
    // material, and the look's systems wait for it. With it they run as in a
    // window: they spawn, dress and ink, and nothing is drawn.
    app.init_asset::<RoomStateMaterial>();
    app.init_asset::<RoomPlateMaterial>();
    let full = app.world().resource::<UserSettings>().video.quality.profile;
    assert_ne!(full, VisualQualityProfile::Potato, "premise: the round trip needs two tiers");
    for _ in 0..30 {
        app.update();
    }
    // The plates are drawn by a task off the main thread. Wait for the task,
    // and not for a number of updates.
    let waited = std::time::Instant::now();
    while dress(&mut app).plates == 0 && waited.elapsed() < std::time::Duration::from_secs(60) {
        std::thread::sleep(std::time::Duration::from_millis(5));
        app.update();
    }
    let dressed = dress(&mut app);
    assert!(dressed.plates > 0, "premise: the hub's architecture is drawn from plates: {dressed:?}");
    // ⛔ THE PREMISES: the hub has a look, the look dressed a door, and it
    // inked a sign. With none of these the round trip compares nothing.
    assert!(dressed.pieces > 0, "premise: the hub's look is presented: {dressed:?}");
    assert!(dressed.look_doors > 0, "premise: no door has the look's art: {dressed:?}");
    let inked = dressed.signs.iter().filter(|sign| sign.3 >= 4).count();
    assert!(inked > 0, "premise: no sign has the look's ink and halo: {dressed:?}");

    set_tier(&mut app, VisualQualityProfile::Potato);
    let plain = dress(&mut app);
    assert_eq!(
        (plain.pieces + plain.plates, plain.look_doors),
        (0, 0),
        "on Potato the look left pieces or dressed doors behind: {plain:?}"
    );
    let still_inked: Vec<_> = plain.signs.iter().filter(|sign| sign.3 > 0).collect();
    assert!(still_inked.is_empty(), "on Potato a sign kept the look's halo: {still_inked:?}");
    let recoloured = dressed.signs.iter().zip(&plain.signs).filter(|(a, b)| (a.1, a.2) != (b.1, b.2)).count();
    assert_eq!(recoloured, inked, "each inked sign, and no other, has other colours on Potato");

    set_tier(&mut app, full);
    assert_eq!(dress(&mut app), dressed, "back on the first tier the room is not dressed as it was");
}
