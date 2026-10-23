//! Retire parallax themes outside live rooms and their neighbours.
//!
//! Keep themes that the neighbouring-room prefetch may use. This
//! policy belongs to the app; the sprite-sheet asset set only
//! provides the retention operation. Offscreen eviction must never
//! lower visual quality. Bevy releases image memory only after the
//! last handle, including handles on drawn layers, is dropped.

use bevy::prelude::*;

use ambition_platformer2d::sprite_sheet::game_assets::{GameAssets, ParallaxTheme};
// ⛔ Through the FACADE, not `ambition_platformer2d_world` directly. The app is a
// consumer like any other and the compiler enforces it: the world crate is not
// an `ambition_app` dependency, which is the capability boundary working.

/// Keep each live room's theme plus its one-hop neighbours'; drop the rest.
///
/// Runs when that keep-set changes: "which rooms are live" is the only input
/// this policy has. It read the sole live room, so while two rooms were live
/// it did not run and no theme was retired (OW1).
pub(crate) fn retire_departed_parallax_themes(
    rooms: ambition_platformer2d::world::rooms::LiveRoomSpecs,
    mut assets: ResMut<GameAssets>,
    // The keep-set of the last run.
    mut kept: Local<Vec<ParallaxTheme>>,
    // The live rooms of the last run: the keep-set is a function of them and
    // the room set, so it is built only when either changes. Building it each
    // frame looked up every live room's neighbours and their themes.
    mut last_live: Local<Vec<usize>>,
) {
    let live_unchanged = rooms.live_rooms().map(|(_, definition)| definition.index()).eq(last_live.iter().copied());
    if live_unchanged && !rooms.set_is_changed() {
        return;
    }
    let live: Vec<usize> = rooms.live_rooms().map(|(_, definition)| definition.index()).collect();
    last_live.clone_from(&live);
    let keep = parallax_keep_set(rooms.rooms(), &live);
    // ⛔ Only on a change. Running every frame would call `retain` on a map that
    // has not moved, and would fight `ensure_parallax_layers_for_room` on the
    // frame a new theme is being loaded.
    if keep.is_empty() || *kept == keep {
        return;
    }
    *kept = keep.clone();

    let before = assets.parallax_layers.resident_themes();
    // The terrain skin of a theme is kept as long as its parallax.
    assets.room_dressing.retain_themes(|theme| keep.contains(&theme));
    let retired = assets
        .parallax_layers
        .retain_themes(|theme| keep.contains(&theme));
    if retired == 0 {
        return;
    }

    let departed = before
        .into_iter()
        .filter(|theme| !keep.contains(theme))
        .map(ParallaxTheme::key)
        .collect::<Vec<_>>();
    let live_ids = live
        .iter()
        .filter_map(|&index| rooms.rooms().rooms.get(index))
        .map(|room| room.id.as_str())
        .collect::<Vec<_>>();
    bevy::log::info!(
        target: "ambition_platformer2d::assets",
        "[parallax] retired {retired} layers of themes [{}] — keeping [{}] for live room(s) [{}] and their neighbours",
        departed.join(", "),
        keep.iter().copied().map(ParallaxTheme::key).collect::<Vec<_>>().join(", "),
        live_ids.join(", "),
    );
}

/// The themes to keep: each live room's (`live` are indices into `rooms`),
/// then each one's one-hop neighbours', each once, in that order.
pub(crate) fn parallax_keep_set(
    rooms: &ambition_platformer2d::world::rooms::RoomSet,
    live: &[usize],
) -> Vec<ParallaxTheme> {
    let mut keep = Vec::new();
    let mut add = |index: usize| {
        if let Some(room) = rooms.rooms.get(index) {
            let theme = ParallaxTheme::from_room_metadata(&room.metadata);
            // A theme with a corrupted state is loaded with it
            // (`ensure_parallax_layers_for_room`), and is kept with it.
            for theme in std::iter::once(theme).chain(theme.corrupted()) {
                if !keep.contains(&theme) {
                    keep.push(theme);
                }
            }
        }
    };
    for &index in live {
        add(index);
    }
    for &index in live {
        for neighbour in rooms.neighboring_room_indices_of(index) {
            add(neighbour);
        }
    }
    keep
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d::world::rooms::{LoadingZone, LoadingZoneActivation, RoomLink, RoomSet, RoomSpec};

    /// Room `id` in theme `theme`, with a door `to_<other>` for each of `doors`.
    fn room(id: &str, theme: ParallaxTheme, doors: &[&str]) -> RoomSpec {
        let mut spec = RoomSpec::new(
            id,
            ambition_platformer2d::engine_core::World::new(
                id,
                ambition_platformer2d::engine_core::Vec2::new(400.0, 300.0),
                ambition_platformer2d::engine_core::Vec2::ZERO,
                Vec::new(),
            ),
        );
        spec.metadata.visual_profile.parallax_theme = Some(theme.key().to_string());
        spec.loading_zones = doors
            .iter()
            .map(|other| LoadingZone {
                id: format!("to_{other}"),
                name: format!("to_{other}"),
                activation: LoadingZoneActivation::Door,
                aabb: ambition_platformer2d::engine_core::Aabb::new(
                    ambition_platformer2d::engine_core::Vec2::new(10.0, 10.0),
                    ambition_platformer2d::engine_core::Vec2::splat(8.0),
                ),
            })
            .collect();
        spec
    }

    fn link(from: &str, to: &str) -> RoomLink {
        RoomLink {
            from_room: from.into(),
            from_zone: format!("to_{to}"),
            to_room: to.into(),
            to_zone: format!("to_{from}"),
            bidirectional: true,
        }
    }

    /// OW1: with two live rooms, each one's theme and its neighbours' are
    /// kept. `a` (Hub) leads to `b` (Cave); `c` (Lab) leads to `d`
    /// (Basement); `e` (Boss) is no neighbour of a live room. The control is
    /// `a` alone, which keeps Hub and Cave only.
    #[test]
    fn every_live_room_keeps_its_theme_and_its_neighbours() {
        let rooms = RoomSet::from_parts_or_panic(
            "a",
            vec![
                room("a", ParallaxTheme::Hub, &["b"]),
                room("b", ParallaxTheme::Cave, &["a"]),
                room("c", ParallaxTheme::Lab, &["d"]),
                room("d", ParallaxTheme::Basement, &["c"]),
                room("e", ParallaxTheme::Boss, &[]),
            ],
            vec![link("a", "b"), link("c", "d")],
        );
        assert_eq!(
            parallax_keep_set(&rooms, &[0]),
            vec![ParallaxTheme::Hub, ParallaxTheme::Cave],
            "control: one live room keeps its theme and its neighbour's"
        );
        assert_eq!(
            parallax_keep_set(&rooms, &[0, 2]),
            vec![ParallaxTheme::Hub, ParallaxTheme::Lab, ParallaxTheme::Cave, ParallaxTheme::Basement],
            "two live rooms keep both themes and both neighbourhoods, and not Boss"
        );
    }

    /// A theme that has a corrupted state keeps it: the look of the room
    /// draws it and no room names it. The control is `Hub`, which has none.
    #[test]
    fn a_theme_keeps_its_corrupted_state_with_it() {
        let rooms = RoomSet::from_parts_or_panic(
            "a",
            vec![room("a", ParallaxTheme::HubClean, &["b"]), room("b", ParallaxTheme::Hub, &["a"])],
            vec![link("a", "b")],
        );
        assert_eq!(
            parallax_keep_set(&rooms, &[0]),
            vec![ParallaxTheme::HubClean, ParallaxTheme::HubCorrupt, ParallaxTheme::Hub],
        );
        assert_eq!(parallax_keep_set(&rooms, &[1])[0], ParallaxTheme::Hub);
    }
}
