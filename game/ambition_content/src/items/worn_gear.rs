//! Ambition's worn gear: the pack's `data/worn_gear.ron` (schema `worn_gear`,
//! owned by the items capability), contributed to the
//! [`WornGearManifest`](ambition_platformer2d_shared_tangle::worn_gear::WornGearManifest).
//!
//! The simulation reads the manifest (a conductor puts a gear on a body), so
//! it is registered by the content plugin that every composition installs.

use ambition_items::content_schema::worn_gear::{AuthoredGearAnchor, AuthoredWornGear};
use ambition_platformer2d_shared_tangle::worn_gear::{GearAnchor, GearFit, WornGearAppExt, WornGearEntry};
use bevy::math::Vec2;
use bevy::prelude::{App, Resource};

/// The gear ids Ambition's pack registered: the rows a reload restates, and
/// no other provider's.
#[derive(Resource, Clone, Debug, Default)]
pub(crate) struct AmbitionWornGear(pub(crate) Vec<String>);

fn entry(id: &str, gear: &AuthoredWornGear) -> WornGearEntry {
    WornGearEntry {
        id: id.to_owned(),
        anchor: match gear.anchor {
            AuthoredGearAnchor::Head => GearAnchor::Head,
        },
        slot: gear.slot.clone(),
        back: gear.back.clone(),
        front: gear.front.clone(),
        size: Vec2::new(gear.size.0, gear.size.1),
        socket: Vec2::new(gear.socket.0, gear.socket.1),
        fits: gear
            .fits
            .iter()
            .map(|(sheet, fit)| {
                let fit = GearFit {
                    track: fit.track.clone(),
                    scale: fit.scale,
                    offset: Vec2::new(fit.offset.0, fit.offset.1),
                    turn_degrees: fit.turn,
                };
                (sheet.clone(), fit)
            })
            .collect(),
    }
}

/// The manifest entries `pack` states. A pack with no `worn_gear` source has
/// no gear.
pub(crate) fn entries(pack: &ambition_content_pack::PreparedContentPack) -> Vec<WornGearEntry> {
    ambition_items::content_schema::worn_gear::lowered_worn_gear(pack)
        .map(|rows| rows.iter().map(|(id, gear)| entry(id, gear)).collect())
        .unwrap_or_default()
}

/// Contribute the selected pack's gear to the manifest.
pub(crate) fn register(app: &mut App) {
    let pack = crate::pack::select(app.world_mut());
    let entries = entries(&pack);
    let ids = entries.iter().map(|entry| entry.id.clone()).collect();
    app.register_worn_gear(entries);
    app.insert_resource(AmbitionWornGear(ids));
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d_shared_tangle::worn_gear::WornGearManifest;

    /// The shipped helmet is the pack file's row: its slot, both pictures,
    /// and a fit for each of its three wearers on the track that is their
    /// head (GNU-ton is drawn baked: his fit's track is not read, and his
    /// fit moves it up to his head).
    #[test]
    fn the_space_helmet_is_the_pack_files_row() {
        let mut app = App::new();
        register(&mut app);
        let manifest = app.world().resource::<WornGearManifest>();
        let helmet = manifest.get("space_helmet").expect("the pack declares the space helmet");
        assert_eq!(helmet.slot, "head");
        assert_eq!(helmet.anchor, GearAnchor::Head);
        assert!(helmet.back.is_some() && helmet.front.is_some());
        let tracks: Vec<(&str, &str)> = helmet.fits.iter().map(|(sheet, fit)| (sheet.as_str(), fit.track.as_str())).collect();
        assert_eq!(
            tracks,
            vec![("giant_gnu", "head"), ("gnu_ton_scholar_boss", "head"), ("player_robot_v3", "head")]
        );
        assert_eq!(app.world().resource::<AmbitionWornGear>().0, vec!["space_helmet".to_owned()]);
    }
}
