//! Room-level verification that the vector-gravity showcase rooms actually
//! exercise the flagship in a real LDtk-authored room (the engine has unit tests
//! for the *mechanic*; these prove the *content* wires up to it):
//!
//! - `wall_run` — walk right into a rightward field and get carried onto the
//!   right wall (the flagship as a SHOWCASE).
//! - `ceiling_cross` — walk into an upward field off a ledge, fall onto the
//!   ceiling, and cross a hazard floor that's otherwise lethal (the flagship as a
//!   TRAVERSAL TOOL).
//!
//! Both drive only "hold right" and assert on the public `Platformer2dSimHarness` observation,
//! so they're fast (sub-second) and don't depend on rendering.

use crate::common::{fixed_60hz_room_sim, hold_right};

#[test]
fn wall_run_field_pulls_the_player_onto_the_right_wall() {
    let mut sim = fixed_60hz_room_sim("wall_run");

    // Spawns in the left, normal-gravity strip (x≈80), well left of the field
    // (which starts at x=260) and the right wall (x=624).
    let spawn = sim.observation().player_pos;
    assert!(
        spawn.0 < 200.0,
        "player should spawn in the left strip, got x={}",
        spawn.0
    );

    // Walk right: cross into the rightward GravityZone (x>260), whose gravity then
    // carries the player onto the right wall (x≈624).
    for _ in 0..90 {
        sim.step(hold_right());
    }

    let (px, py) = sim.observation().player_pos;
    assert!(
        px > 540.0,
        "the rightward field should carry the player onto the right wall \
         (got x={px}, y={py}); if vector gravity weren't wired for this room the \
         player would stop near the field boundary instead",
    );
    // And it didn't fling them out of the room.
    assert!(
        px < 640.0 && (16.0..752.0).contains(&py),
        "player stays inside the room (x={px}, y={py})",
    );
}

#[test]
fn ceiling_cross_inverts_the_player_onto_the_ceiling_to_cross_the_hazard() {
    let mut sim = fixed_60hz_room_sim("ceiling_cross");

    // Spawns on the left ledge (x≈70), left of the death floor x[300,720].
    let spawn = sim.observation().player_pos;
    assert!(
        spawn.0 < 300.0,
        "player should spawn on the left ledge, got x={}",
        spawn.0
    );

    // Walk right: off the ledge into the upward field, which inverts the player
    // onto the ceiling; keep walking and they cross above the hazard to the right
    // ledge. (Plenty of ticks for the ~190-tick crossing.)
    for _ in 0..260 {
        sim.step(hold_right());
    }

    let obs = sim.observation();
    let (px, py) = obs.player_pos;
    // Reached the right ledge — only possible via the ceiling, since the floor
    // strip x[300,720] is a HazardBlock that respawns on contact.
    assert!(
        px > 720.0,
        "player should have crossed to the right ledge via the ceiling \
         (got x={px}, y={py})",
    );
    // ...and survived: a hazard touch respawns, so a clean cross leaves resets at 0.
    assert_eq!(
        obs.resets, 0,
        "player crossed without dying on the death floor (resets={})",
        obs.resets,
    );
}

/// Holding right through `ceiling_cross` with the player-relative camera
/// (`CameraReferenceFrame::SubjectFrame`) crosses on the ceiling, and the
/// world turns over once going in (Jon, 2026-10-19: "If I just hold right in
/// ceiling cross the camera goes wild. It used to just flip the screen once
/// and let me continue moving in that direction").
///
/// Under that camera every movement mode resolves body-relative
/// (`InputFrameMode::under_camera`), and the gravity of the body is the
/// screen's. The moment the field turns gravity up, a held right becomes the
/// body's right, which is world-left: it walks the body back out of the
/// field, gravity turns down, the camera turns back, right is world-right
/// again, and so on. Each turn of gravity is a half turn of the camera.
///
/// Measured 2026-10-19: 30 turns in 360 ticks, the body bouncing between x 248
/// and 262 (the field begins at 270) while it rises. With the world-fixed
/// camera the same hold crosses (the test above).
#[test]
#[ignore = "BUG (2026-10-19): fails, 30 turns of gravity in 6 s at the field's left edge; un-ignore with the fix"]
fn ceiling_cross_with_the_player_relative_camera_turns_the_world_over_once() {
    use ambition_platformer2d::engine_core::{CameraReferenceFrame, InputFrameMode};
    use ambition_platformer2d::persistence::settings::UserSettings;

    let mut sim = fixed_60hz_room_sim("ceiling_cross");
    sim.world_mut().resource_mut::<UserSettings>().gameplay.camera_reference_frame =
        CameraReferenceFrame::SubjectFrame;
    // Publishes the RESOLVED mode (body-relative under this camera) to the seat.
    sim.set_movement_frame_mode(InputFrameMode::ScreenRelative);

    let mut gravity = sim.observation().gravity_dir;
    let mut turns = Vec::new();
    for tick in 0..360 {
        sim.step(hold_right());
        let obs = sim.observation();
        if obs.gravity_dir != gravity {
            turns.push((tick, obs.gravity_dir, obs.player_pos));
            gravity = obs.gravity_dir;
        }
    }
    let obs = sim.observation();
    // In, and out over the right ledge: two turns of gravity at most.
    assert!(
        turns.len() <= 2,
        "holding right, gravity (and the player-relative camera) turned {} times: {:?}",
        turns.len(),
        &turns[..turns.len().min(8)],
    );
    assert!(
        obs.player_pos.0 > 720.0 && obs.resets == 0,
        "holding right with the player-relative camera did not cross to the right ledge \
         (at {:?}, resets {}; gravity turns {:?})",
        obs.player_pos,
        obs.resets,
        &turns[..turns.len().min(8)],
    );
}
