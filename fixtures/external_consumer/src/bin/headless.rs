//! Outlander running headlessly — the Phase-6 "runs visibly and headlessly
//! from the same content" proof, headless half. Mirrors the in-repo standalone
//! demo shells (`ambition_demo_mary_o_app`): engine foundation + host +
//! minimal shell + THIS crate's provider plugin, zero engine edits.
//!
//! Not a boot smoke test: `run_outlander_walkthrough` fails unless the
//! Outlander session ACTIVATES (room constructed, player + staged sentry
//! present) and the ridge gate actually transits the walking body onto the
//! upper ledge. An earlier draft updated an empty un-routed host 120 times and
//! called that success (GPT 5.6 review finding).

fn main() {
    let mut app = outlander::build_outlander_app();
    match outlander::run_outlander_walkthrough(&mut app) {
        Ok(report) => {
            println!(
                "outlander: session active after {} ticks (room {:?}, player + sentry verified)",
                report.ticks_to_activate,
                outlander::OUTLANDER_ROOM_ID,
            );
            println!(
                "outlander: ridge gate transited the player after {} ticks of walking; \
                 resting at ({:.1}, {:.1}) on the upper ledge",
                report.ticks_to_gate, report.player_pos.x, report.player_pos.y,
            );
        }
        Err(error) => {
            eprintln!("outlander: FAILED: {error}");
            std::process::exit(1);
        }
    }
}
