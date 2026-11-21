//! Training knobs: put the fighters back where they started, and say what
//! percent they stand on.
//!
//! A narrow API, run between frames by whatever asks for it (a tool, a test,
//! a training menu once there is one):
//!
//! ```ignore
//! world.run_system_once_with(reset_fighters, TrainingReset::at_percent(80))?;
//! ```
//!
//! It is not a sim system. Nothing in a match writes it, so a rewind never
//! replays it; a networked match has no training mode.

use bevy::prelude::*;

/// What [`reset_fighters`] does to the meters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TrainingReset {
    /// The percent every fighter stands on after the reset. `None` puts each
    /// back on the meter it starts its lives on: its handicap, or none.
    pub percent: Option<u32>,
}

impl TrainingReset {
    /// Back to the start: positions, and each fighter's own starting meter.
    pub const START: Self = Self { percent: None };

    /// Back to the start, every fighter on `percent`.
    pub fn at_percent(percent: u32) -> Self {
        Self { percent: Some(percent) }
    }
}

/// Every seated fighter still in play back where the match seated it
/// (`ambition_match::seat_placement` about its room's spawn), at rest, facing
/// in, with its swing cancelled, its reactions cleared and its meter stated.
///
/// A fighter waiting out a knockout is left to come back on its own: its death
/// beat owns it, and a reset that stood it up mid-beat would put a body in play
/// that still answers no input. An eliminated fighter stays out.
///
/// The meter is stated, not dealt: the pool stays full, as for a handicap. A
/// percent is measured against the match's pool (`handicap_damage`).
#[allow(clippy::type_complexity)]
pub fn reset_fighters(
    In(reset): In<TrainingReset>,
    mut commands: Commands,
    prepared: Option<Res<ambition_platformer2d::versus_match::PreparedMatch>>,
    rooms: ambition_platformer2d::platformer::lifecycle::LiveRoomOf<
        ambition_platformer2d::engine_core::RoomGeometry,
    >,
    mut fighters: Query<
        (
            Entity,
            &ambition_platformer2d::actor::MatchSeat,
            ambition_platformer2d::actor::BodyClusterQueryData,
            &mut ambition_platformer2d::actor::MotionModel,
            &mut ambition_platformer2d::characters::actor::BodyCombat,
            Option<&mut ambition_platformer2d::characters::actor::BodyHealth>,
            Option<&ambition_platformer2d::actor::FighterStocks>,
            Option<&mut ambition_platformer2d::combat::moveset::MovePlayback>,
        ),
        (
            Without<ambition_platformer2d::combat::death_rules::OutOfPlay>,
            Without<ambition_platformer2d::combat::stocks::FighterEliminated>,
        ),
    >,
) {
    let pool = prepared.as_deref().and_then(|prepared| prepared.rules().health_pool);
    for (body, seat, clusters, mut model, mut combat, health, stocks, playback) in &mut fighters {
        // A fighter in no live room has no stage to stand on.
        let Some(geometry) = rooms.of(body) else {
            continue;
        };
        let (at, facing) = ambition_platformer2d::versus_match::seat_placement(seat.0, geometry.0.spawn);
        let mut item = clusters;
        let mut clusters = item.as_clusters_mut();
        let air_jumps = model.air_jumps();
        ambition_platformer2d::engine_core::reset_body_clusters(
            &mut model,
            &mut clusters,
            at,
            ambition_platformer2d::engine_core::ResetFacing::Toward(facing),
            air_jumps,
        );
        if let Some(mut playback) = playback {
            ambition_platformer2d::combat::moveset::cancel_move_playback(
                &mut commands,
                body,
                &mut playback,
                ambition_platformer2d::combat::moveset::MoveEnd::LeftPlay,
            );
        }
        // Hitstun, a sleep, a burial: a reset fighter is free.
        combat.reset();
        if let Some(mut health) = health {
            let damage = match reset.percent {
                Some(percent) => crate::select::handicap_damage(percent, pool),
                None => stocks.map_or(0, |stocks| stocks.respawn_damage),
            };
            health.reset();
            health.set_damage_taken(damage);
        }
    }
}
