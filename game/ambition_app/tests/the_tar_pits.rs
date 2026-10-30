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
