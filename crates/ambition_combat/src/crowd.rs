//! Where a body contests space when it fights.
//!
//! A crowd of attackers converging on one target reads as a pile unless
//! something spaces them out. That job belongs to the brain's per-actor
//! crowding signal — personal-space pressure computed from the positions of the
//! other bodies in the fight — and this enum is the one fact that signal needs
//! that positions do not carry: a flyer holds its distance from other flyers,
//! and ignores the bodies on the ground beneath it.
//!
//! ⛔⛔ **THIS MODULE USED TO OWN AN ATTACK-SLOT BOARD, AND THE BOARD WAS DEAD.**
//! `CombatSlotBoard` allocated numbered approach slots around a target and a
//! holding ring for the attackers that missed out; `assign_slots` filled it
//! every tick from `tick_actor_brains`, and it was rewound as rollback state.
//! Nothing read the result. The per-actor position it produced had been
//! discarded (`let _ = slot_pos`) since before the monolith split — recorded in
//! `dev/journals/code_smells.md` on 2026-07-02, which asked for exactly this
//! decision: does the board earn its keep against the crowding signal? It does
//! not, and it cost more than nothing to keep: the board was anchored on
//! "the primary player, or the lowest `PlayerSlot`", which is the single
//! largest player-centric assumption inside generic actor simulation, and it
//! was there to feed a mechanism with no consumer.

/// Whether a fighting body contests ground space or airspace.
///
/// The crowding signal reads this to scope who counts as a neighbour: an aerial
/// body keeps its distance from other aerial bodies over a wider radius, and a
/// grounded body spaces itself against grounded ones. Nothing else distinguishes
/// them — this is not a family, a capability, or a movement mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CrowdKind {
    /// Fights from the ground.
    Ground,
    /// Fights from the air, and spaces itself against other flyers.
    Aerial,
}
