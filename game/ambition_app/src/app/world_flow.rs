//! Room-flow + player-tick presentation glue that stays in the app host.
//!
//! What remains here after the world-runtime / combat-runtime drains:
//! - [`RoomClock`] — a sim-state plus clock-reset bundle for room-transition systems.
//! - [`ground_gap_below_feet`] — the room-transition landing diagnostic helper.
//! - the [`room_flow`] submodule (sandbox reset + authorized room commit);
//! - the [`room_transition_loading`] submodule (readiness transaction + preflight);
//! - the [`room_transition_assets`] submodule (room manifests, readiness, prefetch);
//! - the [`room_transition_presentation`] submodule (cover-first adaptive UI).
//!
//! The attack-phase machine, victim-side damage resolution, and movement-event
//! Sfx/Vfx emission moved DOWN into `ambition::actors::combat::{attack,
//! damage}` / `::player::movement_fx`; the sim half of room load moved into
//! `ambition::actors::rooms`.

use bevy::prelude::{MessageWriter, ResMut};

use ambition::engine_core::{self as ae, AabbExt};
use ambition::platformer::feature_overlay::FeatureEcsWorldOverlay;

/// Bundle of room-reset sim resources, so systems that already sit near Bevy's
/// 16-SystemParam limit (e.g. [`commit_ready_room_transition_system`]) can request a
/// clock reset and mutate room-transition cooldown through one slot. The clock
/// reset is emitted as data and consumed by the time-control owner.
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct RoomClock<'w> {
    pub sim_state: ResMut<'w, ambition::actors::SandboxSimState>,
    pub clock_resets: MessageWriter<'w, ambition::actors::time::time_control::ClockResetRequest>,
}

mod room_flow;
mod room_transition_assets;
mod room_transition_loading;
mod room_transition_presentation;
pub(crate) use room_flow::commit_ready_room_transition_system;
pub(crate) use room_transition_assets::{
    build_loaded_room_asset_manifest, ensure_room_character_sprites, inspect_room_asset_manifest,
    RoomAssetManifest,
};
pub(crate) use room_transition_loading::{
    advance_room_transition_content_epoch_system, authorize_ready_room_transition_system,
    begin_room_transition_load_system, finalize_unpresented_room_transition_failure_system,
    RoomTransitionContentEpoch, RoomTransitionLoadState,
};
pub(crate) use room_transition_presentation::install_room_transition_presentation;

/// Probe along the body's gravity direction from its feet for the nearest
/// landing face (within 256 px). Returns `(distance, source)` where `source`
/// is `"world"`, `"overlay"`, or `"both"`. `None` means nothing — the player is
/// over a pit (real bug) or `to_room_set()` / overlay rebuild hasn't
/// materialised the floor yet (the race we're hunting).
///
/// Frame-relative: "below feet" is +gravity, not world-down, so the diagnostic
/// stays meaningful under a gravity flip (identity under normal gravity). Blocks
/// are projected onto the gravity + side axes via [`ae::AabbExt`].
fn ground_gap_below_feet(
    body: &ae::Aabb,
    gravity_dir: ae::Vec2,
    world: &ae::World,
    feature_overlay: &FeatureEcsWorldOverlay,
) -> Option<(f32, &'static str)> {
    const MAX_PROBE_PX: f32 = 256.0;
    // Side axis ⊥ gravity (`gravity_half(side)` reuses the projection to get an
    // AABB's extent along it).
    let side = ae::Vec2::new(gravity_dir.y, -gravity_dir.x);
    let feet = body.feet_coord(gravity_dir);
    let body_side = body.center().dot(side);
    let body_side_half = body.gravity_half(side);
    let probe = |blocks: &[ae::Block]| {
        let mut best: Option<f32> = None;
        for block in blocks {
            // The body's cross-section (⊥ gravity) must overlap the block's.
            let block_side = block.aabb.center().dot(side);
            if (block_side - body_side).abs() >= body_side_half + block.aabb.gravity_half(side) {
                continue;
            }
            // Only consider blocks whose landing face is at/below the feet along
            // gravity.
            let gap = block.aabb.head_coord(gravity_dir) - feet;
            if gap < 0.0 || gap > MAX_PROBE_PX {
                continue;
            }
            best = Some(best.map_or(gap, |b| b.min(gap)));
        }
        best
    };
    let world_gap = probe(&world.blocks);
    let overlay_gap = probe(&feature_overlay.blocks);
    match (world_gap, overlay_gap) {
        (Some(a), Some(b)) if (a - b).abs() < 0.5 => Some((a.min(b), "both")),
        (Some(a), Some(b)) if a <= b => Some((a, "world")),
        (Some(_), Some(b)) => Some((b, "overlay")),
        (Some(a), None) => Some((a, "world")),
        (None, Some(b)) => Some((b, "overlay")),
        (None, None) => None,
    }
}
