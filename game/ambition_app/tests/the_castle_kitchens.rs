//! The kitchens under Castle Godwin's gatehouse: the castle's staff
//! (`tools/ambition_ldtk_tools/specs/castle_kitchens/make_specs.py`, talk in
//! `dialogue/sandbox/castle_godwin.yarn`).
//!
//! Jon (2026-10-30): "build out the world detail ... adding art, flavor,
//! character". The player starts in the kitchens, talks to the cook, the
//! night porter and the guild's steward, goes up the door to the gatehouse
//! and comes back down.

use crate::the_pirate_port::{a_session_in, go, talk};

const KITCHENS: &str = "castle_godwin_kitchens";
const GATEHOUSE: &str = "castle_godwin_gatehouse";

#[test]
fn the_castle_staff_talk_in_the_kitchens_under_the_gatehouse() {
    let mut app = a_session_in(KITCHENS);
    talk(&mut app, "npc_castle_osgood", "What do you do here?");
    talk(&mut app, "npc_castle_hargreave", "What are you cooking?");
    talk(&mut app, "npc_castle_dunmore", "What does the Guild want?");
    go(&mut app, GATEHOUSE, Some("kitchens_door"));
    go(&mut app, KITCHENS, Some("castle_godwin_gatehouse_kitchens"));
}
