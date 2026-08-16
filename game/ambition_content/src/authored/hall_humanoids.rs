//! **THE HALL HUMANOIDS** — Alice, Bob and Emmy No-Ether.
//!
//! They are the last four characters that could not build a body from their own
//! definition, and every one of them was missing exactly one fact: **locomotion**.
//! Not health, not a moveset, not capabilities — a walk.
//!
//! ⭐⭐ **that is why deleting the body-assist seam waited on content
//! rather than on engineering.** The body-assist seam existed to correct bodies
//! for characters that could not state their own, and the population it served
//! was fourteen on 2026-08-13. Seven of those were the pirates and Carl Stargan,
//! whose vitals Jon settled that day. These four are the remainder, and their
//! gap was never a decision anybody was waiting on.
//!
//! ⚠ **authoring it changes no shipped behaviour, and that is checkable rather
//! than hopeful.** Three of them carry `default_brain: "patrol_peaceful"` and
//! Emmy carries `stand_still`; `PatrolCfg::speed` is an absolute walk speed in
//! px/s (28 for that preset), not a fraction of the body's. So the amble a
//! visitor sees in the Hall is the POLICY's, and `run_speed` is what the body
//! could do if something ever drove it — possession, a match, a provocation.
//!
//! ⛔ **Emmy gets the same walk as the other three**, and the reason is the rule
//! Jon stated on 2026-08-13: *"do not make him permanently peaceful/passive
//! merely because he may normally appear as a friendly NPC"* — said of Carl, and
//! general. A body that is TOLD to stand still is not a body that cannot move,
//! and encoding her placement's `stand_still` as a zero run speed would put a
//! controller policy inside a body fact where nothing could ever override it.
//!
//! ⚠ **one file for the rest**, the same P2.16 rule the pirate crew file
//! follows: files differing by nothing at all would be the copy `AUTHORED_CAST`
//! exists to refuse. If one of them grows a moveset or a distinct build, it
//! earns its own file that day.
//!
//! ⭐ **Oiler did, on 2026-08-16** — a sixteen-move smash repertoire built on
//! his own twenty-three rendered effects. He is [`super::npc_oiler`] now, and he
//! still takes [`HUMANOID_RUN_SPEED`] from here, because a mechanic with a
//! wrench walks like the folk he walks among.

use ambition_characters::actor::CharacterLocomotion;
use ambition_characters::brain::MoveStyleSpec;
use ambition_platformer2d_actor_monolith::character_runtime::CharacterDefinition;

/// The ordinary humanoid walk, shared by the Hall's people. Between the goblin's
/// 170 and the salvage guard's 225 — these are folk, not fighters, and the
/// number is Jon's kind of tuning: change it after playing, not by reopening a
/// decision.
pub(super) const HUMANOID_RUN_SPEED: f32 = 210.0;

/// See the module doc. Reached through [`super::AUTHORED_CAST`], which is also
/// what makes these characters buildable — there is no second list to remember.
pub(crate) fn author(_id: &str, definition: CharacterDefinition) -> CharacterDefinition {
    let mut definition = definition.with_locomotion(CharacterLocomotion {
        run_speed: HUMANOID_RUN_SPEED,
        move_style: MoveStyleSpec::Walk,
        ..Default::default()
    });
    // Jon 2026-08-13: ordinary humanoid/NPC baseline.
    definition.vitals.max_health = Some(4);
    definition
}
