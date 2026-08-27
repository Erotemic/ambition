//! The Officer — easter-egg brawler humanoid.
//!
//! A state trooper who wandered into a fighting game: out of uniform from the
//! neck down, entirely in character from the neck up. He is the Pugnacious
//! Polygon's archetype wearing a different person — unarmed, close-range, and
//! on the same skeleton and clip vocabulary.
//!
//! Nothing may depend on him being selectable. He is meant to be found.

use ambition_platformer2d::character::CharacterDefinition;

pub(crate) fn author(_id: &str, definition: CharacterDefinition) -> CharacterDefinition {
    let mut definition = definition
        .with_locomotion(ambition_characters::actor::CharacterLocomotion {
            // The brawler archetype's own number, for the reason his moveset is
            // the brawler's: he is that archetype.
            run_speed: 230.0,
            move_style: ambition_characters::brain::MoveStyleSpec::Walk,
            ..Default::default()
        })
        // ⭐⭐ HE CARRIES A SIDEARM, AND THAT IS THE CHARACTER'S FACT. His rig
        // has a `holster` on his back and a `sidearm` his `shoot` clip puts in
        // his hand; the gun being drawn is art, and the gun being able to fire
        // is this. `MoveEventKind::Ranged` on his side special asks the body
        // what its ranged action is, and with nothing brandished the answer is
        // the one stated here — the same division `npc_pirate_raider` makes one
        // file over: *a character states what it DOES and the item states what
        // it HOLDS*.
        //
        // ⛔ NOT AN `equips` ITEM, WHICH IS HOW THE ADMIRAL'S GUN-SWORD WORKS.
        // A held item is a PROP: a second sprite drawn in the hand. The
        // Officer's gun is already on his own sheet, so equipping one would put
        // two guns in one fist. See `crate::officer_moveset`.
        .with_action_set(ambition_characters::brain::ActionSet {
            ranged: Some(
                ambition_characters::brain::RangedActionSpec::pistol(560.0, 7)
                    // ⭐ THE HALF-PLANE IS JON'S RULE, and it is the weapon's
                    // rule rather than this fighter's: the player picks a side
                    // and the gun picks the angle within it. Shorter than the
                    // admiral's 360 because a service pistol is not a
                    // gun-sword — it reaches a spacing exchange, not the stage.
                    .with_aim_assist(ambition_characters::brain::action_set::AimAssist::half_plane(280.0))
                    // ⛔⛔ THE MOVE'S OWN RECOVERY IS THE CADENCE. `refire_s` is
                    // checked where the move is ACCEPTED, so a recharge on top
                    // of a 0.7s special would refuse a shot the move had already
                    // been accepted to fire: the animation plays, the flash
                    // plays, and nothing comes out. The admiral's gun-sword
                    // states the same 0.0 for the same reason.
                    .with_refire(0.0),
            ),
            // ⛔ HIS WALK, RESTATED. `ActionSet` is a whole authority and its
            // default `move_style` is not his — leaving it out would quietly
            // overwrite the locomotion two lines up.
            move_style: ambition_characters::brain::MoveStyleSpec::Walk,
            ..Default::default()
        })
        .with_moveset(crate::officer_moveset::officer_moveset());
    definition.vitals.max_health = Some(6);
    definition
}
