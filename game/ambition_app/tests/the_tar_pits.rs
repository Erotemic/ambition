//! The Tar Pits: out past the end of the primeval trail, east of the canopy
//! (`tools/ambition_ldtk_tools/specs/primeval/tar_pits.py`).
//!
//! Jon (2026-10-30): "out of town zones with enemies", joined by side exits.
//! The player starts at the pits' west end, walks by keys west into the
//! canopy, and back east out to the pits.

use crate::the_pirate_port::{a_session_in, go};

const PITS: &str = "primeval_tar_pits";
const CANOPY: &str = "primeval_canopy";

#[test]
fn the_tar_pits_lie_east_of_the_canopy_by_its_side_exit() {
    let mut app = a_session_in(PITS);
    go(&mut app, CANOPY, Some("tar_west"));
    go(&mut app, PITS, Some("primeval_canopy_tar_pits"));
}
