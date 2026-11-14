//! The Tar Pits and the Smoking Mountain, in the Jungle's layers
//! (`dinosaur.ldtk`, LDtk-owned; WORLD-LAYERS JL-1, 2026-11-14).
//!
//! Jon (2026-10-30): "out of town zones with enemies". The trail was one
//! line east to the mountain; it is folded into layers now. Under the ridge
//! road: the fossil beds, and east of them the tar pits, whose seep climbs
//! back up to the ridge, so the ridge, the sinkhole, the beds, the pits and
//! the seep are a loop. Over the ridge: the canopy, up a vine, and the
//! Smoking Mountain rising east of the canopy. Each join is walked by keys.

use crate::the_pirate_port::{a_session_in, go, talk};

const PITS: &str = "primeval_tar_pits";
const CANOPY: &str = "primeval_canopy";

const RIDGE: &str = "primeval_ridge";
const BEDS: &str = "primeval_fossil_beds";

/// Under the ridge road, a loop: down the sinkhole to the fossil beds, east
/// under the ridge to the tar pits, and up the tar seep to the ridge again.
#[test]
fn the_ridge_the_fossil_beds_and_the_tar_pits_are_a_loop() {
    let mut app = a_session_in(RIDGE);
    go(&mut app, BEDS, Some("ridge_sinkhole"));
    go(&mut app, PITS, Some("fossils_east"));
    go(&mut app, RIDGE, Some("tar_shaft"));
    go(&mut app, PITS, Some("ridge_tar_shaft"));
    go(&mut app, BEDS, Some("tar_west"));
}

/// Over the ridge road: up the vine to the canopy, and back down it.
#[test]
fn a_vine_climbs_from_the_ridge_road_to_the_canopy() {
    let mut app = a_session_in(RIDGE);
    go(&mut app, CANOPY, Some("ridge_vine"));
    go(&mut app, RIDGE, Some("canopy_vine"));
}

/// The trail's people talk: the ranger at the trailhead, the paleontologist
/// at the tar pits' dig camp (`dialogue/sandbox/primeval.yarn`).
#[test]
fn the_ranger_and_the_paleontologist_talk() {
    let mut app = a_session_in("primeval_trailhead");
    talk(&mut app, "npc_primeval_brandvold", "The chickens?");
    let mut app = a_session_in(PITS);
    talk(&mut app, "npc_primeval_feld", "What are you digging for?");
}

/// The Smoking Mountain rises east of the canopy, higher than it: the
/// player starts at the mountain's foot, walks west into the canopy, and
/// back east.
#[test]
fn the_smoking_mountain_rises_east_of_the_canopy() {
    let mut app = a_session_in("primeval_smoking_mountain");
    go(&mut app, CANOPY, Some("mountain_west"));
    go(&mut app, "primeval_smoking_mountain", Some("primeval_canopy_mountain"));
}

/// Professor Tock keeps the meteor watch on the Smoking Mountain's rim: the
/// player climbs the shelves by keys to him (`primeval.yarn`).
#[test]
fn the_meteor_watch_is_kept_on_the_rim() {
    let mut app = a_session_in("primeval_smoking_mountain");
    talk(&mut app, "npc_primeval_tock", "Listening for what?");
}

/// Mildred Twitchell watches the rookery's birds from her hide (`primeval.yarn`
/// `primeval_twitchell`), past the penguins and the shoebill.
#[test]
fn the_birdwatcher_counts_the_rookerys_penguins() {
    let mut app = a_session_in("primeval_rookery");
    talk(&mut app, "npc_primeval_twitchell", "What are you watching?");
}

/// The ranger station: up onto the hut's deck at the trailhead and in at
/// its door; Ranger Dlamini talks on the radio (`primeval.yarn`
/// `primeval_dlamini`), and out again.
#[test]
fn the_ranger_station_is_inside_the_trailheads_hut() {
    let mut app = a_session_in("primeval_trailhead");
    go(&mut app, "primeval_ranger_station", Some("trailhead_ranger_door"));
    talk(&mut app, "npc_primeval_dlamini", "What do you do here?");
    go(&mut app, "primeval_trailhead", Some("station_door"));
}

/// The fossil beds, under the ridge road down its sinkhole (a vertical
/// exit, a ladder): Dr Stratton talks by her board of the layers
/// (`primeval.yarn` `primeval_stratton`); the ladder goes up to the ridge
/// road and back down. (Up first, as the Mobius Mine's test.)
#[test]
fn the_fossil_beds_lie_under_the_ridge_road() {
    let mut app = a_session_in("primeval_fossil_beds");
    talk(&mut app, "npc_primeval_stratton", "What is wrong with upstairs?");
    go(&mut app, "primeval_ridge", Some("fossils_shaft"));
    go(&mut app, "primeval_fossil_beds", Some("ridge_sinkhole"));
}

/// The treetops over the trail (`primeval_treetops`): up the great
/// tree's rope ladder out of the run, east along the canopy, and down
/// through the gap in the leaves into the rookery; and back down the tree.
#[test]
fn the_treetops_run_over_the_trail_from_the_great_tree_to_the_rookery() {
    let mut app = a_session_in("primeval_run");
    go(&mut app, "primeval_treetops", Some("run_great_tree"));
    go(&mut app, "primeval_run", Some("treetops_trunk"));
    let mut app = a_session_in("primeval_treetops");
    go(&mut app, "primeval_rookery", Some("treetops_drop"));
}

/// The fossil beds' cracked wall at their west end (a breakable: hit it),
/// amber in the crack: the amber vault behind it, and back.
#[test]
fn a_cracked_wall_in_the_fossil_beds_hides_the_amber_vault() {
    use ambition_platformer2d::engine_core as ae;
    let mut app = a_session_in("primeval_fossil_beds");
    crate::the_pirate_port::break_the_wall(&mut app, "primeval_fossil_cracked_wall", ae::Vec2::new(110.0, 560.0), -1.0);
    go(&mut app, "primeval_amber_vault", Some("secret_fossils_west"));
    go(&mut app, "primeval_fossil_beds", Some("secret_amber_east"));
}
