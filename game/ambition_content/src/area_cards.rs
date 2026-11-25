//! Ambition's areas (`data/area_cards.ron`): the card shown on entering each.
//! The engine draws the card ([`ambition_render::rendering::area_card`]);
//! which rooms make an area, and what it is called, is the game's.

use bevy::prelude::*;

#[cfg(feature = "render")]
use ambition_render::rendering::area_card::{AreaCard, AreaCards};

/// The cards `pack` states. A pack with no `area_cards` has none.
#[cfg(feature = "render")]
pub fn cards_of(pack: &ambition_content_pack::PreparedContentPack) -> AreaCards {
    let mut cards = AreaCards::default();
    for row in ambition_content_schemas::area_cards::lowered_area_cards(pack).into_iter().flatten() {
        let card = AreaCard { name: row.name.clone(), subtitle: row.subtitle.clone() };
        for room in &row.rooms {
            match room.strip_suffix('*') {
                Some(prefix) => cards.prefixes.push((prefix.to_string(), card.clone())),
                None => {
                    cards.rooms.insert(room.clone(), card.clone());
                }
            }
        }
    }
    cards
}

/// Insert the cards the selected pack states.
pub(crate) fn register(app: &mut App) {
    // The cards are the renderer's to draw; without it there is no reader.
    #[cfg(not(feature = "render"))]
    let _ = app;
    #[cfg(feature = "render")]
    let pack = crate::pack::select(app.world_mut());
    #[cfg(feature = "render")]
    app.insert_resource(cards_of(&pack));
}

/// Publish a reloaded pack's cards (the reload's `area_cards` family).
pub(crate) fn publish(world: &mut World, pack: &ambition_content_pack::PreparedContentPack) {
    #[cfg(not(feature = "render"))]
    let _ = (world, pack);
    #[cfg(feature = "render")]
    {
        let cards = cards_of(pack);
        if world.get_resource::<AreaCards>() != Some(&cards) {
            world.insert_resource(cards);
        }
    }
}
