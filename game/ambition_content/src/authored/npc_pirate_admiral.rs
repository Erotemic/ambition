//! THE PIRATE ADMIRAL'S CUTLASS. The second fighter taken off the generic
//! repertoire floor (P3.24), and the character was already telling us
//! what its moves are: its row says `default_action_set: "pirate_pistol"`,
//! the roster comment beside its id reads "pistol + cutlass", and its
//! sprite is authored at `collision_scale: 1.6` — the largest of the three
//! fighters with a table.
//!
//! MOVES ONLY. A table is the whole job.

use ambition_platformer2d::character::CharacterDefinition;

/// AC5: it authors its LOCOMOTION too, which is the one fact that stood
/// between this character and building its own body. It ships as
/// `melee_brute_striker` (chase 110), and that preset's speed is absolute, so
/// stating the body's run speed here changes nothing a player sees today — it
/// makes the body complete, which is what let the body-assist seam go.
///
/// a moveset without a body was the exact shape the assist seam existed for: a
/// character rich enough to state its swings and not yet able to state its walk.
///
/// See the module doc. Reached through [`super::AUTHORED_CAST`], which is also
/// what makes this character buildable — there is no second list to remember.
pub(crate) fn author(_id: &str, definition: CharacterDefinition) -> CharacterDefinition {
    let mut definition = definition
        .with_locomotion(ambition_characters::actor::CharacterLocomotion {
            run_speed: 110.0,
            move_style: ambition_characters::brain::MoveStyleSpec::Walk,
            ..Default::default()
        })
        // ⭐⭐ AN ADMIRAL CAN RIDE A SHARK, AND THAT IS THE CHARACTER'S FACT, NOT
        // A MATCH'S. Jon: *"Yes the admiral could fly on a shark in ambition…
        // right now the admiral doesn't ride the shark, but they should have the
        // ability to mount them if there is a shark mount amenable to being
        // mounted."* `npc_pirate_raider` already says exactly this, one file
        // over; the admiral not saying it was an omission.
        //
        // ⛔⛔ IT USED TO BE MANUFACTURED BY THE SMASH SEAT, and that is how the
        // up-B shipped broken: `smash_roster` granted the class per seat and
        // `SmashSelect::roster_seeded` — the road a player actually travels from
        // the character-select grid — assembled its participants from scratch
        // and never did. The admiral reached the match with no `CanPilot`, the
        // board was refused, and the summoned shark just stood there. A
        // capability the CHARACTER owns is granted by every road that builds it,
        // because `prepared_match` unions it in at realization.
        //
        // ⚠ THE CLASS, NOT A PARTICULAR SHARK. Which shark this admiral may
        // board is a separate question with a separate answer — see
        // `MountReservedFor`, which is what stops the second admiral in a mirror
        // match from stealing the first one's summon.
        .with_mount(ambition_characters::actor::CharacterMount {
            pilotable_classes: vec!["shark".to_string()],
            ..Default::default()
        })
        .with_moveset(crate::pirate_admiral_moveset::pirate_admiral_moveset());
    definition.vitals.max_health = Some(6);
    definition
}
