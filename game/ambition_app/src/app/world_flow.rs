//! Room-flow + player-tick presentation glue that stays in the app host.
//!
//! What remains here after the world-runtime / combat-runtime drains:
//! - [`RoomClock`] — a sim-state plus clock-reset bundle for room-transition systems.
//! - [`ground_gap_below_feet`] — the room-transition landing diagnostic helper.
//! - the [`room_flow`] submodule (sandbox reset, room load, transition apply).
//!
//! The attack-phase machine, victim-side damage resolution, and movement-event
//! Sfx/Vfx emission moved DOWN into `ambition_actors::combat::{attack,
//! damage}` / `::player::movement_fx`; the sim half of room load moved into
//! `ambition_actors::rooms`.

use bevy::prelude::{MessageWriter, ResMut};

use ambition_platformer_primitives::feature_overlay::FeatureEcsWorldOverlay;
use ambition_engine_core::{self as ae, AabbExt};

/// Bundle of room-reset sim resources, so systems that already sit near Bevy's
/// 16-SystemParam limit (e.g. [`apply_room_transition_system`]) can request a
/// clock reset and mutate room-transition cooldown through one slot. The clock
/// reset is emitted as data and consumed by the time-control owner.
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct RoomClock<'w> {
    pub sim_state: ResMut<'w, ambition_actors::SandboxSimState>,
    pub clock_resets: MessageWriter<'w, ambition_actors::time::time_control::ClockResetRequest>,
}

mod room_flow;
pub use room_flow::ensure_requested_room_parallax_system;
pub(crate) use room_flow::{apply_room_transition_system, reset_sandbox};

/// Probe straight down from the player's feet for the nearest block
/// top (within 256 px). Returns `(distance, source)` where `source` is
/// `"world"`, `"overlay"`, or `"both"`. `None` means nothing — the
/// player is over a pit (real bug) or `to_room_set()` / overlay
/// rebuild hasn't materialised the floor yet (the race we're hunting).
fn ground_gap_below_feet(
    feet_y: f32,
    body: &ae::Aabb,
    world: &ae::World,
    feature_overlay: &FeatureEcsWorldOverlay,
) -> Option<(f32, &'static str)> {
    const MAX_PROBE_PX: f32 = 256.0;
    let probe = |blocks: &[ae::Block]| {
        let mut best: Option<f32> = None;
        for block in blocks {
            // X must overlap the player body.
            if block.aabb.right() <= body.left() || block.aabb.left() >= body.right() {
                continue;
            }
            // Only consider blocks whose top is below feet.
            let top = block.aabb.top();
            if top < feet_y {
                continue;
            }
            let gap = top - feet_y;
            if gap > MAX_PROBE_PX {
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
