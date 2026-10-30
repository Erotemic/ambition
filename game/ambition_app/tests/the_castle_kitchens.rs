//! The kitchens under Castle Godwin's gatehouse: the castle's staff
//! (`tools/ambition_ldtk_tools/specs/castle_kitchens/make_specs.py`, talk in
//! `dialogue/sandbox/castle_godwin.yarn`).
//!
//! Jon (2026-10-30): "build out the world detail ... adding art, flavor,
//! character". The player starts in the kitchens, talks to the cook, the
//! night porter and the guild's steward, goes up the door to the gatehouse
//! and comes back down; then through the door at the kitchens' east end to
//! the Complaints Department, where Agnes Mudge holds ticket 4,117.

use crate::the_pirate_port::{a_session_in, go, talk};

const KITCHENS: &str = "castle_godwin_kitchens";
const GATEHOUSE: &str = "castle_godwin_gatehouse";
const COMPLAINTS: &str = "castle_godwin_complaints";

#[test]
fn the_castle_staff_talk_in_the_kitchens_under_the_gatehouse() {
    let mut app = a_session_in(KITCHENS);
    talk(&mut app, "npc_castle_osgood", "What do you do here?");
    talk(&mut app, "npc_castle_hargreave", "What are you cooking?");
    talk(&mut app, "npc_castle_dunmore", "What does the Guild want?");
    go(&mut app, GATEHOUSE, Some("kitchens_door"));
    go(&mut app, KITCHENS, Some("castle_godwin_gatehouse_kitchens"));
}

#[test]
fn agnes_mudge_waits_in_the_complaints_department_behind_the_kitchens() {
    let mut app = a_session_in(KITCHENS);
    go(&mut app, COMPLAINTS, Some("kitchens_complaints_door"));
    talk(&mut app, "npc_evil_mudge", "What number are you?");
    go(&mut app, KITCHENS, Some("complaints_door"));
}

/// The Prototype Workshop, down a door from the barracks by its failed
/// prototypes: Dr. Gideon Sprocket talks, and the barracks is back up.
#[test]
fn the_castle_engineer_talks_in_the_prototype_workshop() {
    let mut app = a_session_in("castle_godwin_workshop");
    talk(&mut app, "npc_castle_sprocket", "What do you make here?");
    go(&mut app, "castle_godwin_barracks", Some("workshop_door"));
}
