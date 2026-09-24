//! The Pirate Admiral's cutlass. The character's data states its moves: its
//! row says `default_action_set: "pirate_pistol"`, the roster comment beside
//! its id reads "pistol + cutlass", and its sprite is authored at
//! `collision_scale: 1.6`, the largest of the fighters with a table.

use ambition_platformer2d::character::CharacterDefinition;

/// It authors its locomotion, so it can build its own body. It ships as
/// `melee_brute_striker` (chase 110), whose speed is absolute, so the run
/// speed here changes nothing a player sees; it makes the body complete.
///
/// See the module doc. Reached through [`super::AUTHORED_CAST`], which also
/// makes this character buildable; there is no second list.
pub(crate) fn author(_id: &str, definition: CharacterDefinition) -> CharacterDefinition {
    let mut definition = definition
        .with_locomotion(ambition_characters::actor::CharacterLocomotion {
            run_speed: 110.0,
            move_style: ambition_characters::brain::MoveStyleSpec::Walk,
            ..Default::default()
        })
        // An admiral can ride a shark; that is the character's fact, not a
        // match's. Jon: *"Yes the admiral could fly on a shark in ambition… right
        // now the admiral doesn't ride the shark, but they should have the ability
        // to mount them if there is a shark mount amenable to being mounted."*
        // `npc_pirate_raider` says the same.
        //
        // A capability the character owns is granted by every road that builds
        // it, because `prepared_match` unions it in at realization. If the smash
        // seat granted it instead, `SmashSelect::roster_seeded` (the road from the
        // character-select grid) would build the admiral without `CanPilot`, and
        // the summoned shark could not be boarded.
        //
        // The class, not a particular shark. Which shark this admiral may board is
        // `MountReservedFor`, which stops the second admiral in a mirror match from
        // taking the first one's summon.
        .with_mount(ambition_characters::actor::CharacterMount {
            pilotable_classes: vec!["shark".to_string()],
            ..Default::default()
        });
    // Its moves are content, not code (fast-iteration I2, step 5): the table is
    // `assets/data/movesets/pirate_admiral.ron`, declared in `pack.ron`,
    // validated by the `moveset` schema and applied in
    // `crate::character_catalog::authored_intrinsics`, the one seam every
    // buildable character passes through. The Rust table is only the exporter's
    // source and the parity oracle's subject; the host reads neither, so editing
    // it changes nothing until it is re-exported.
    definition.vitals.max_health = Some(6);
    definition
}
