//! The finishing blow pulls the camera in.
//!
//! ⭐ THE GAMEPLAY FACT ALREADY EXISTED AND WAS ALREADY SPENT — this module is
//! the wire, not the fact. `StocksMatchDecided` is written once by the
//! ruleset-facing half of the loop, when `last_side_standing` first answers, the
//! clock expires, or somebody stops the match. The parity inventory carried
//! *"no fact anywhere says this blow is the finishing one"* for months; two do.
//!
//! ⛔⛔ AND THE MATCH-LEVEL FACT IS THE RIGHT ONE, not the per-fighter one.
//! [`crate::stocks::FighterStockSpent::eliminated`] says *this fighter* lost
//! their last stock — which is the finishing blow in a two-fighter match and is
//! NOT in a four-fighter one, where it fires on every elimination. Keying the
//! zoom to the verdict makes it player-count-agnostic and fires it exactly once.
//!
//! Like the camera shake beside it, this publishes an INTENT and touches no
//! presentation state: a rollback host's first pass at a frame is unconfirmed,
//! so a decision a later correction erases must not have already moved the
//! camera.

use bevy::prelude::*;

use ambition_platformer2d_shared_tangle::camera_ease::FinishZoomRequest;

/// Full strength. A match ending is the one beat in a match that does not need
/// a magnitude — there is no such thing as a more-decisive victory — so the
/// request carries `1.0` and [`FinishZoomTuning`] owns how far that is.
///
/// [`FinishZoomTuning`]: ambition_platformer2d_shared_tangle::camera_ease::FinishZoomTuning
const DECIDED_CLOSENESS: f32 = 1.0;

/// Ask for the finishing zoom when the match is decided BY A WINNER.
///
/// ⛔ `MatchVerdict::winner()` and not a `matches!` on the message: a `Draw` and
/// a `NoContest` get no victory zoom, and the type exists precisely to make that
/// collapse opt-in. Jon, on the `Exit Match` command: *"It should not award an
/// ordinary winner/loser result."* An abandoned match is not a finish.
pub fn zoom_camera_on_decided_match(
    mut decided: MessageReader<crate::stocks::StocksMatchDecided>,
    mut requests: MessageWriter<FinishZoomRequest>,
) {
    for message in decided.read() {
        if message.outcome.winner().is_some() {
            requests.write(FinishZoomRequest {
                closeness: DECIDED_CLOSENESS,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stocks::{MatchVerdict, StocksMatchDecided};

    fn app() -> App {
        let mut app = App::new();
        app.add_message::<StocksMatchDecided>();
        app.add_message::<FinishZoomRequest>();
        app.add_systems(Update, zoom_camera_on_decided_match);
        app
    }

    fn requests_after(outcome: MatchVerdict) -> usize {
        let mut app = app();
        app.world_mut().write_message(StocksMatchDecided { outcome });
        app.update();
        let messages = app.world().resource::<Messages<FinishZoomRequest>>();
        let mut cursor = messages.get_cursor();
        cursor.read(messages).count()
    }

    #[test]
    fn a_decided_match_asks_for_the_finishing_zoom() {
        assert_eq!(
            requests_after(MatchVerdict::Winner("left".into())),
            1,
            "a winner is the finishing beat"
        );
    }

    /// ⛔ THE ARM THAT MATTERS, and it is the whole reason this reads
    /// `winner()` instead of "was a match decided". Both of these DECIDE a
    /// match; neither is a victory.
    #[test]
    fn a_draw_and_a_no_contest_get_no_victory_zoom() {
        assert_eq!(requests_after(MatchVerdict::Draw), 0, "a draw is not a win");
        assert_eq!(
            requests_after(MatchVerdict::NoContest),
            0,
            "an abandoned match is not a win"
        );
    }

    /// A quiet frame publishes nothing, so a host that never decides a match
    /// never pays for this.
    #[test]
    fn no_decision_asks_for_nothing() {
        let mut app = app();
        app.update();
        let messages = app.world().resource::<Messages<FinishZoomRequest>>();
        let mut cursor = messages.get_cursor();
        assert_eq!(cursor.read(messages).count(), 0);
    }
}
