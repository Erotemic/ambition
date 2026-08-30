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
                    // ⭐⭐ HIS SHOT LEAVES THE BARREL, NOT HIS MIDRIFF. Without a
                    // `Discharge` this defaulted to `Muzzle::BodyOrigin`, whose
                    // spawn is `origin + (0, -8)` — a purely VERTICAL offset, so
                    // the round was born at his centre while the gun and its
                    // flare are drawn out at his hand. `Muzzle::Hand`'s own doc
                    // says it exists for exactly this: *"so the shot leaves the
                    // barrel the player can see rather than the fighter's
                    // midriff."* The hand is mirrored by `facing_sign`, so this
                    // is correct in both directions.
                    //
                    // ⛔⛔ AND THIS IS WHAT "STILL FIRING BACKWARDS" WAS. The
                    // velocity was never wrong — `officer_probe` proves vel.x
                    // agrees with facing both ways. A round that is born behind
                    // the visible muzzle and then travels forward reads as
                    // coming out of the wrong place however the velocity is
                    // signed, which is the complaint a sign check cannot see.
                    //
                    // ⚠ THE PROBE'S OWN "+9.3 AHEAD" WAS NOT A MUZZLE OFFSET.
                    // It samples the first tick the round is VISIBLE, one tick
                    // after spawn: 560 px/s ÷ 60 Hz = 9.33. It was reading one
                    // tick of travel and reporting it as the spawn offset, so
                    // the instrument called `BodyOrigin` "ahead of him".
                    .with_discharge(ambition_characters::brain::action_set::Discharge {
                        muzzle: ambition_characters::brain::action_set::Muzzle::Hand {
                            ahead: 10.0,
                        },
                        // No cue of its own: `officer_the_draw` already plays
                        // the draw at 0.116s, and naming a second one here
                        // would be a cue this table has to keep in step with a
                        // weapon it does not own.
                        ..Default::default()
                    })
                    // ⭐ AND IT LOOKS LIKE A BULLET. Registered in
                    // `crate::projectiles`; without this the id is empty and
                    // resolves to the engine's generic quad.
                    .with_visual(ambition_characters::brain::action_set::PISTOL_ROUND_VISUAL)
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

#[cfg(test)]
mod tests {
    use ambition_characters::brain::action_set::{Muzzle, PISTOL_ROUND_VISUAL};

    /// ⭐⭐ THE SHOT LEAVES THE BARREL, AND IT LOOKS LIKE A BULLET.
    ///
    /// Jon, 2026-08-30: *"the officer is still firing backwards."* His round's
    /// VELOCITY was never wrong — `officer_probe` shows `vel.x` agreeing with
    /// facing in both directions. Two presentation facts were:
    ///
    /// * no `Discharge`, so the shot defaulted to [`Muzzle::BodyOrigin`], whose
    ///   spawn offset is purely VERTICAL — the round was born at his sternum
    ///   while the gun and its muzzle flare are drawn out at his hand;
    /// * no `visual`, so an empty id resolved to `ProjectileArt::generic()`, the
    ///   engine's orange-red QUAD. A symmetric quad also makes `FlipToTravel` a
    ///   no-op, which is why the flip looked innocent under inspection.
    ///
    /// ⛔ THIS GOES THROUGH `author_for`, not `author`, for the reason Emmy's
    /// test does: authoring a weapon on a character no table reaches would be
    /// indistinguishable from authoring nothing.
    #[test]
    fn the_officers_round_leaves_his_hand_and_carries_his_own_art() {
        let author = super::super::author_for("officer")
            .expect("the Officer is in AUTHORED_CAST, or nothing he authors is reachable");
        let definition = author(
            "officer",
            super::super::CharacterDefinition::new("officer", "The Officer", "ambition"),
        );
        let ranged = definition
            .action_set
            .as_ref()
            .and_then(|a| a.ranged.as_ref())
            .expect("the Officer states a ranged action — `The Draw` fires it");

        let discharge = ranged
            .discharge
            .as_ref()
            .expect("his sidearm authors a discharge, or the shot is born at his midriff");
        assert!(
            matches!(discharge.muzzle, Muzzle::Hand { .. }),
            "the Officer's gun is DRAWN — his shot must be born at the hand the \
             `shoot` clip puts it in, not at `BodyOrigin`, which is horizontally \
             ON him and reads as firing from the wrong place however the \
             velocity is signed"
        );

        assert_eq!(
            ranged.visual.as_deref(),
            Some(PISTOL_ROUND_VISUAL),
            "his round must carry its own art; an absent visual id resolves to \
             the engine's generic quad, which is both wrong for a pistol and \
             symmetric enough to hide a flip error"
        );
    }
}
