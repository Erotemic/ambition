//! Select the held portal gun's art from the next shot's end.

use ambition_combat::held_items::HeldItem;
use ambition_portal2d::PortalGenerator;
use ambition_render::rendering::HeldItemAppearance;
use bevy::prelude::*;

const PORTAL_GUN: &str = "portal_gun";
const ORANGE_ART: HeldItemAppearance = HeldItemAppearance {
    item_id: PORTAL_GUN,
    art_id: "portal_gun_orange",
};

pub(super) fn install(app: &mut App) {
    app.add_systems(
        Update,
        select_portal_gun_art
            .before(ambition_render::rendering::posed_hand::HeldItemPresentationSet),
    );
}

fn select_portal_gun_art(
    mut commands: Commands,
    bodies: Query<
        (Entity, Option<&HeldItem>, Option<&PortalGenerator>, Option<&HeldItemAppearance>),
        Or<(With<PortalGenerator>, With<HeldItemAppearance>)>,
    >,
) {
    for (body, item, generator, appearance) in &bodies {
        let orange = item.is_some_and(|item| item.id() == PORTAL_GUN)
            && generator.is_some_and(|generator| generator.next_color.is_end_b());
        match (orange, appearance == Some(&ORANGE_ART)) {
            (true, false) => {
                commands.entity(body).try_insert(ORANGE_ART);
            }
            (false, true) => {
                commands.entity(body).try_remove::<HeldItemAppearance>();
            }
            _ => {}
        }
    }
}
