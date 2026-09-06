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
//! the active room's theme and the themes of its one-hop neighbours.** That is
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

use ambition_platformer2d::platformer::lifecycle::SessionWorldRef;
use ambition_platformer2d::sprite_sheet::game_assets::{GameAssets, ParallaxTheme};
// ⛔ Through the FACADE, not `ambition_platformer2d_world` directly. The app is a
// consumer like any other and the compiler enforces it: the world crate is not
// an `ambition_app` dependency, which is the capability boundary working.
use ambition_platformer2d::world::rooms::RoomSet;

/// Keep the active room's theme plus its one-hop neighbours'; drop the rest.
///
/// Runs when the room set changes — the same trigger the preparation prefetch
/// keys off, because "which room is active" is the only input this policy has.
pub(crate) fn retire_departed_parallax_themes(
    room_set: SessionWorldRef<RoomSet>,
    mut assets: ResMut<GameAssets>,
) {
    // ⛔ Only on a change. Running every frame would call `retain` on a map that
    // has not moved, and would fight `ensure_parallax_layers_for_room` on the
    // frame a new theme is being loaded.
    if !room_set.is_changed() {
        return;
    }
    let Some(active) = room_set.rooms.get(room_set.active) else {
        return;
    };

    let mut keep = vec![ParallaxTheme::from_room_metadata(&active.metadata)];
    for index in room_set.neighboring_room_indices() {
        if let Some(neighbour) = room_set.rooms.get(index) {
            let theme = ParallaxTheme::from_room_metadata(&neighbour.metadata);
            if !keep.contains(&theme) {
                keep.push(theme);
            }
        }
    }

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
    bevy::log::info!(
        target: "ambition_platformer2d::assets",
        "[parallax] retired {retired} layers of themes [{}] — keeping [{}] for room '{}' and {} neighbour(s)",
        departed.join(", "),
        keep.iter().copied().map(ParallaxTheme::key).collect::<Vec<_>>().join(", "),
        active.id,
        keep.len().saturating_sub(1),
    );
}
