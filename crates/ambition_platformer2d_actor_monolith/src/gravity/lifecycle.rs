//! Gravity-zone lifecycle and room-reset policy. Extracted from
//! `ambition_portal2d::lifecycle` (Stage 6 follow-up): this is a *gravity
//! mechanic*, not portal behavior, so it owns its state here and must not
//! depend on `ambition_portal2d`.

use bevy::prelude::*;


/// Reset the AMBIENT gravity to the default (down) when a new world begins, so a
/// flipped room doesn't carry over. Only the ambient is authored state; the
/// presentation `GravityField` is a per-tick mirror of the primary body's
/// resolved frame with exactly one writer (`resolve_active_gravity`), so it
/// follows on the next resolution.
///
/// ⛔⛤ **IT READ ONE LIFECYCLE FACT AND THE GAME HAS THREE — MEASURED BY A
/// 2026-09-13 REVIEW.** `BaseGravity` is canonical ROLLBACK state and a mechanic
/// (the LDtk-authored `FlipGravity` switches write it), and its only reset roads were
/// `RoomReplayAdmitted` and a room TRANSITION's commit. **Reset New Game has its
/// own seam** — `NewGameResetCommitted` — and nothing here listened, so *"flip
/// gravity, Reset New Game"* started the fresh run upside down.
///
/// ⇒ Both facts mean the same thing to this domain: the world you were falling
/// through is gone. ⚠ The SESSION edge is not here: `BaseGravity` is a member of
/// `SessionScopedResources`, so activation and retirement clear it with the rest
/// of the live-session mirrors and under that aggregate's compiler guard.
pub fn reset_gravity_on_room_reset(
    mut resets: MessageReader<ambition_combat::events::RoomReplayAdmitted>,
    mut new_game: MessageReader<crate::session::reset::NewGameResetCommitted>,
    mut base: ResMut<ambition_platformer2d_shared_tangle::gravity::BaseGravity>,
) {
    let replayed = resets.read().next().is_some();
    let restarted = new_game.read().next().is_some();
    if !replayed && !restarted {
        return;
    }
    *base = ambition_platformer2d_shared_tangle::gravity::BaseGravity::default();
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d_shared_tangle::gravity::BaseGravity;

    /// ⛔⛤ **RESET NEW GAME HAS ITS OWN SEAM, AND AMBIENT GRAVITY WAS NOT
    /// LISTENING TO IT — MEASURED BY A 2026-09-13 REVIEW.**
    ///
    /// This reducer answered `RoomReplayAdmitted` alone. `Reset New Game` commits
    /// through `NewGameResetCommitted` and rebuilds the room WITHOUT a replay, so
    /// *"flip gravity, Reset New Game"* left the fresh run falling upward until
    /// some later room transition happened to correct it.
    ///
    /// ⭐ **BOTH ARMS, AND THE FIRST IS THE CONTROL.** A reducer wired only to the
    /// new seam would pass the second assertion and silently drop the replay road
    /// it already had.
    #[test]
    fn a_new_game_and_a_replay_both_put_gravity_back_down() {
        for (what, write) in [
            ("a room replay", 0usize),
            ("Reset New Game", 1usize),
        ] {
            let mut app = App::new();
            app.add_message::<ambition_combat::events::RoomReplayAdmitted>();
            app.add_message::<crate::session::reset::NewGameResetCommitted>();
            app.init_resource::<BaseGravity>();
            app.add_systems(Update, reset_gravity_on_room_reset);

            let flipped = ambition_platformer2d_core::Vec2::new(0.0, -1.0);
            app.world_mut().insert_resource(BaseGravity { dir: flipped });
            app.update();
            // ⚠ THE PREMISE: with no message, the flip must SURVIVE — otherwise
            // a reducer that resets unconditionally would pass both arms.
            assert_eq!(
                app.world().resource::<BaseGravity>().dir,
                flipped,
                "gravity reset with no lifecycle message at all, so neither arm \
                 below measures a reducer reacting to anything"
            );

            if write == 0 {
                app.world_mut()
                    .write_message(ambition_combat::events::RoomReplayAdmitted::manual());
            } else {
                app.world_mut()
                    .write_message(crate::session::reset::NewGameResetCommitted);
            }
            app.update();
            assert_eq!(
                app.world().resource::<BaseGravity>().dir,
                BaseGravity::default().dir,
                "{what} left the ambient gravity flipped, so the new world starts \
                 upside down"
            );
        }
    }
}
