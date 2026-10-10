//! Retire the parallax themes the player has walked away from.
//!
//! ⛔⛔ WHY THIS FILE EXISTS. `ensure_parallax_layers_for_room` lazy-loads a
//! theme's four layers the first time a room asks for them, and until 2026-09-02
//! nothing could ever release one — not because a caller forgot, but because
//! `ParallaxLayerSet` had no eviction API at all and `GameAssets` is built once
//! in `Startup`. Every theme a session visited stayed resident for the life of
//! the process; nine themes x four layers is the ceiling a walk can reach.
//!
//! ⭐ THE OWNERSHIP RULE LIVES HERE, NOT IN THE SET. `retain_themes` takes a
//! predicate precisely so `ambition_sprite_sheet` never learns what a room, a
//! neighbour or a transition is. This module supplies the only policy: **keep
//! each live room's theme and the themes of its one-hop neighbours.** That is
//! the same shape the character-page residency rule follows, and the same
//! adjacency the preparation prefetch uses — `RoomSet::neighboring_room_indices`,
//! the presentation-neutral seam that already exists for exactly this question.
//!
//! ⭐ THE RULE MATCHES WHAT THE PREFETCH ALREADY LOADS, which is why it is the
//! right rule and not merely a plausible one. `prefetch_neighbor_room_preparation_system`
//! calls `build_room_asset_manifest` for each one-hop neighbour, and that
//! function's first act is `ensure_parallax_layers_for_room` for the room it is
//! describing. So standing still already loads the active theme plus every
//! neighbour's; retaining exactly that set drops only themes belonging to rooms
//! the player has walked away from, and never fights the prefetch.
//!
//! ⚠ TWO DIFFERENT NUMBERS, AND THE SMALLER ONE IS THE REAL ONE.
//!
//! What this rule PERMITS, computed over `sandbox.ldtk`'s 60 rooms: a mean of
//! 1.8 themes against the unbounded ceiling of 9, worst case **6** at
//! `central_hub_main`, which has 21 exits into six biomes.
//!
//! ⭐ What is actually RESIDENT there is **3** — measured by
//! `scripts/measure_parallax_retire.sh`, which observed `[Hub, Basement, Boss]`
//! in the hub. The difference is [`NEIGHBOR_PREFETCH_ROOM_BUDGET`] = 4: the
//! prefetch prepares at most four neighbours however many the room has, so the
//! themes that can be loaded at once are capped at active + 4 regardless of a
//! hub's degree. This rule's keep-set is therefore WIDER than what is ever
//! loaded, which is why it never fights the prefetch and never evicts something
//! about to be wanted.
//!
//! ⛔ So the honest claim is bounded twice over, and the adjacency count is the
//! looser bound. An earlier version of this comment quoted the 6 alone and
//! implied the hub keeps six themes' worth of layers resident; it does not, and
//! a reader sizing the leak from that number would have overestimated it by
//! double.
//!
//! ⚠ THIS IS A RESIDENCY CHANGE AND NOTHING ELSE. Jon's ruling, 2026-09-02:
//! nothing may LOWER visual quality for cost reasons. Retiring an off-screen
//! theme is fine; a lower-resolution parallax is not. This module never touches
//! a quality budget, never chooses a scaled variant, and a theme it retires
//! reloads at full authored quality the moment a room asks for it again.
//!
//! ⚠ AND DROPPING A HANDLE IS NECESSARY, NOT SUFFICIENT. Bevy frees an image
//! when its LAST handle drops, so a spawned `ParallaxLayerVisual` holding a
//! clone keeps the pixels alive whatever this reports. Only the active theme is
//! ever spawned, so the neighbour rule cannot strand a drawn layer — and the
//! app-side guard asserts the image actually leaves `Assets<Image>` rather than
//! trusting the count below.

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
) {
    let live: Vec<usize> = rooms.live_rooms().map(|(_, definition)| definition.index()).collect();
    let keep = parallax_keep_set(rooms.rooms(), &live);
    // ⛔ Only on a change. Running every frame would call `retain` on a map that
    // has not moved, and would fight `ensure_parallax_layers_for_room` on the
    // frame a new theme is being loaded.
    if keep.is_empty() || *kept == keep {
        return;
    }
    *kept = keep.clone();

    let before = assets.parallax_layers.resident_themes();
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
