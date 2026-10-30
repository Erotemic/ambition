//! The Tar Pits: out past the end of the primeval trail, east of the canopy
//! (`tools/ambition_ldtk_tools/specs/primeval/tar_pits.py`).
//!
//! Jon (2026-10-30): "out of town zones with enemies", joined by side exits.
//! The player starts at the pits' west end, walks by keys west into the
//! canopy, and back east out to the pits.

use crate::the_pirate_port::{a_session_in, go, talk};

const PITS: &str = "primeval_tar_pits";
const CANOPY: &str = "primeval_canopy";

#[test]
fn the_tar_pits_lie_east_of_the_canopy_by_its_side_exit() {
    let mut app = a_session_in(PITS);
    go(&mut app, CANOPY, Some("tar_west"));
    go(&mut app, PITS, Some("primeval_canopy_tar_pits"));
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

/// The Smoking Mountain: past the tar pits by a side exit. The player starts
/// at the mountain's foot, walks west into the pits, and back east.
#[test]
fn the_smoking_mountain_rises_east_of_the_tar_pits() {
    let mut app = a_session_in("primeval_smoking_mountain");
    go(&mut app, PITS, Some("mountain_west"));
    go(&mut app, "primeval_smoking_mountain", Some("tar_east"));
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
