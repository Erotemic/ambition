//! The Officer: easter-egg brawler humanoid.
//!
//! A state trooper who wandered into a fighting game: out of uniform from the
//! neck down, in character from the neck up. He is the Pugnacious Polygon's
//! archetype as a different person: unarmed, close-range, on the same
//! skeleton and clip vocabulary.
//!
//! Nothing may depend on him being selectable. He is meant to be found.

use ambition_platformer2d::character::CharacterDefinition;

pub(crate) fn author(_id: &str, definition: CharacterDefinition) -> CharacterDefinition {
    let mut definition = definition
        .with_locomotion(ambition_characters::actor::CharacterLocomotion {
            // The brawler archetype's number: he is that archetype.
            run_speed: 230.0,
            move_style: ambition_characters::brain::MoveStyleSpec::Walk,
            ..Default::default()
        })
        // He carries a sidearm. His rig has a `holster` and a `sidearm` that his
        // `shoot` clip puts in his hand; drawing the gun is art, firing it is this.
        // `MoveEventKind::Ranged` on his side special asks the body for its ranged
        // action, and with nothing brandished the answer is stated here. As in
        // `npc_pirate_raider`: a character states what it does and an item states
        // what it holds.
        //
        // Not an `equips` item (the Admiral's gun-sword). A held item is a prop, a
        // second sprite in the hand. The Officer's gun is on his own sheet, so an
        // item would put two guns in one fist. See `crate::officer_moveset`.
        .with_action_set(ambition_characters::brain::ActionSet {
            ranged: Some(
                ambition_characters::brain::RangedActionSpec::pistol(560.0, 7)
                    // The half-plane is Jon's rule, and it belongs to the weapon: the player
                    // picks a side and the gun picks the angle within it. Shorter than the
                    // admiral's 360: a service pistol reaches a spacing exchange, not the
                    // stage.
                    .with_aim_assist(ambition_characters::brain::action_set::AimAssist::half_plane(280.0))
                    // His shot leaves the barrel, not his midriff. The default
                    // `Muzzle::BodyOrigin` spawns at `origin + (0, -8)`, a vertical offset only,
                    // so the round would appear at his centre while the gun and flare are drawn
                    // at his hand. `Muzzle::Hand` is for this case; `facing_sign` mirrors the
                    // hand, so it is correct in both directions.
                    //
                    // The velocity was always correct (`officer_probe` shows vel.x agrees
                    // with facing). A round born behind the visible muzzle still reads as
                    // coming out of the wrong place, which a sign check cannot see.
                    //
                    // `officer_probe` samples the first visible tick, one tick after spawn, so
                    // its offset includes one tick of travel (560 px/s ÷ 60 Hz = 9.33 px). It
                    // is not the spawn offset.
                    .with_discharge(ambition_characters::brain::action_set::Discharge {
                        muzzle: ambition_characters::brain::action_set::Muzzle::Hand {
                            ahead: 10.0,
                        },
                        // No cue of its own: `officer_the_draw` already plays the draw at
                        // 0.116s, and a second cue here would have to stay in step with a weapon
                        // this table does not own.
                        ..Default::default()
                    })
                    // It looks like a bullet. Registered in `crate::projectiles`; without
                    // this the id is empty and resolves to the engine's generic quad.
                    .with_visual(ambition_characters::brain::action_set::PISTOL_ROUND_VISUAL)
                    // The move's own recovery is the cadence. `refire_s` is checked where
                    // the move is accepted, so a recharge on a 0.7s special would refuse a
                    // shot the move was already accepted to fire: animation and flash play,
                    // and nothing comes out. The admiral's gun-sword also uses 0.0.
                    .with_refire(0.0),
            ),
            // His walk, restated. `ActionSet` is a whole authority and its default
            // `move_style` is not his; omitting it would overwrite the locomotion above.
            move_style: ambition_characters::brain::MoveStyleSpec::Walk,
            ..Default::default()
        });
    // His move table is not compiled in. It is
    // `assets/data/movesets/officer.ron`, declared in `pack.ron`, validated by
    // the `moveset` schema and applied in
    // `crate::character_catalog::authored_intrinsics`, the one seam every
    // buildable character passes through.
    //
    // The Rust table is not a host input. It is the exporter's source and the
    // parity oracle's subject
    // (`the_officers_content_table_is_the_table_he_used_to_compile_with`).
    // Editing it changes nothing until it is re-exported: the file is the
    // authority, and a fallback would give the fighter two.
    definition.vitals.max_health = Some(6);
    definition
}

#[cfg(test)]
mod tests {
    use ambition_characters::brain::action_set::{Muzzle, PISTOL_ROUND_VISUAL};

    /// The shot leaves the barrel, and it looks like a bullet.
    ///
    /// The round's velocity is correct (`officer_probe` shows `vel.x` agreeing
    /// with facing both ways). Two presentation facts are checked:
    ///
    /// * a `Discharge`: without it the shot defaults to [`Muzzle::BodyOrigin`],
    ///   whose spawn offset is vertical only, so the round appears at his sternum
    ///   while the gun and flare are drawn at his hand;
    /// * a `visual`: an empty id resolves to `ProjectileArt::generic()`, the
    ///   engine's quad. A symmetric quad also makes `FlipToTravel` a no-op.
    ///
    /// This goes through `author_for`, not `author`, like Emmy's test: authoring
    /// a weapon on a character no table reaches would look like authoring
    /// nothing.
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
