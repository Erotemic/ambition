//! The Author — the sword archetype's table, wielded with a pen.
//!
//! His rig is the Pointed Polygon's, retargeted: the pen occupies the arming
//! sword's exact axis and length, which is why every one of the archetype's
//! 136 clips reads correctly on him without a pose being re-authored. The
//! spacing that follows from that reach is the spacing he fights at, so his
//! frame data IS the archetype's rather than a copy of it that will drift.
//!
//! What is his own is the NAME on it — and, since 2026-08-27, his RECOVERY.
//! See [`author_moveset`].

use ambition_platformer2d::entity_catalog::MovesetContract;

/// When he vanishes. Slower than the robot's blink, because his is a written
/// edit rather than a machine's phase-out.
const TELEPORT_AT_S: f32 = 0.18;

/// When the move ends. The tail is him being drawn back in.
const TELEPORT_ENDS_S: f32 = 0.48;

/// Complete sword-fundamentals repertoire, attributed to the Author.
///
/// ⭐⭐ HIS UP-B IS HIS OWN, and it is the one place he departs from the
/// archetype. Jon, 2026-08-27: *"Mewtwo / Palutena / Zelda style teleports…
/// the animation for the author teleport up b is different, instead of a
/// phase-out effect, it is more of a affine transform to a point, with a store
/// of star flash for the blink out, and the opposite of that for the blink in
/// at the destination spot."*
///
/// ⭐ THE MECHANIC IS THE ROBOT'S; THE LOOK IS NOT. Both fighters author the
/// same `smash.teleport` technique, with the same ledge assist, and differ only
/// in the two effect ids they name — which is exactly what Jon described and is
/// why the look travels in the params instead of being built into the engine.
///
/// ⛔ IT REPLACES THE ARCHETYPE'S `rising_edge`, a spinning rise. That move is
/// the Pointed Polygon's identity and stays hers; a fighter who borrows a table
/// may still own a slot in it.
///
/// ⚠ THE ART IS ONE ROW USED TWICE. `four_point_glint` is the star flash Jon
/// named; the *"opposite of that"* — the same glint converging rather than
/// bursting — is a sheet row that does not exist. Drawing it is an art job, and
/// pointing both ends at the row that DOES exist is honest in the meantime;
/// pointing the arrival at some unrelated effect would not be.
pub fn author_moveset() -> MovesetContract {
    let mut set = crate::archetype_moveset::under_own_name(
        crate::pointed_polygon_moveset::pointed_polygon_moveset(),
        &["polygon", "pointed_polygon"],
        "author",
    );
    crate::special_slots::replace_special(&mut set, "special_up", authors_teleport());
    set
}

/// The Author's recovery: he edits himself out and back in somewhere else.
fn authors_teleport() -> ambition_platformer2d::entity_catalog::MoveSpec {
    let spec = ambition_characters::moveset_authoring::hitless_special(
        "author_revision",
        "special_up",
        TELEPORT_AT_S,
        TELEPORT_ENDS_S,
    );
    let mut spec = spec;
    spec.display_name = Some("Revision".to_string());
    let spec = ambition_characters::smash_teleport::author_teleport(
        spec,
        TELEPORT_AT_S,
        ambition_characters::smash_teleport::TeleportParams {
            // Aimed, like every recovery: any direction given between the
            // press and the transit at `TELEPORT_AT_S`, and straight up from a
            // player who gave none. That startup IS the aim window, which is
            // why the number above is the knob and there is not a second one.
            // See `TeleportParams::behind_nearest_foe`.
            behind_nearest_foe: false,
            behind_gap: 0.0,
            // Further than the robot's, and slower to come out: he pays for the
            // distance in the frames before it.
            distance: 250.0,
            // ⭐⭐ THE LEDGE ASSIST, the same radius the robot gets. It is a
            // property of recovering onto a stage rather than of either fighter,
            // so two fighters wanting it should get the same number until one of
            // them has a reason not to.
            ledge_assist: 44.0,
            // ⭐ THE SAME WINDOW THE ROBOT GETS, and the same reasoning as the
            // ledge assist beside it: intangibility through a vanish is a
            // property of teleporting, not of either fighter, so two fighters
            // wanting it get the same number until one has a reason not to.
            intangible_s: 0.12,
            depart_vfx: "four_point_glint".to_string(),
            arrive_vfx: "four_point_glint".to_string(),
        },
    );
    let spec = ambition_characters::moveset_authoring::sfx(spec, 0.0, "player.attack.charge");
    // ⛔⛔ NO AUTHORED BLINK CUE HERE. `apply_authored_teleports` emits
    // `PLAYER_BLINK` itself at the transit, for EVERY authored teleport — so a
    // move-timeline event at `TELEPORT_AT_S` asked the same frame for the same
    // cue down a second road, and Author's Revision requested it twice
    //. The executor is the one authority, which is what it already
    // is for every other teleport in the game.
    //
    // ⚠ THE OTHER `player.blink` AUTHORSHIPS ARE NOT THIS, and the test is
    // whether the move RUNS THE EXECUTOR — not whose move it is. The Actor's
    // trap (`author_trapdoor`) and Alice's side-B (an `impulse`) never do, so
    // their cue is chosen rather than duplicated. ⛔ any move authored through
    // `author_teleport` is on the other side of that line and must not carry
    // one.
    // ⛔⛔ THROUGH THE SLOT, so it costs what an up-B costs. This move is
    // inserted AFTER `SmashRepertoire::into_contract` has lowered the table it
    // joins, so nothing else will stamp `gates.recovery` on it — and an up-B
    // that spends nothing is flight. Restating the rule here instead would put a
    // second copy of it beside the one place that decides it.
    ambition_characters::smash_repertoire::UpSpecial::Standard(spec).into_spec()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ⛔ THE SWAP IS COMPLETE, and each half is asserted: the verb points at the
    /// new move, the new move is IN the table, and the archetype's rise is gone
    /// rather than left unreachable.
    #[test]
    fn the_author_recovers_by_teleporting_and_the_archetypes_rise_is_gone() {
        let set = author_moveset();
        assert_eq!(
            set.verbs.get("special_up").map(String::as_str),
            Some("author_revision"),
            "his up-B must be the teleport"
        );
        assert!(
            set.moves.iter().any(|m| m.id == "author_revision"),
            "…and the move it names must be in the table"
        );
        assert!(
            !set.moves.iter().any(|m| m.id == "author_rising_edge"),
            "…and the archetype's spinning rise must not be left behind \
             unreachable, where every census that walks `moves` reports it as \
             part of his kit"
        );
    }

    /// ⛔ AND IT IS STILL A RECOVERY. `UpSpecial::Standard` set
    /// `gates.recovery` on the move it lowered; a replacement inserted after
    /// that lowering has to carry the cost itself, or the Author gets an
    /// unlimited teleport.
    #[test]
    fn the_replacement_still_spends_the_airtimes_recovery() {
        let set = author_moveset();
        let up = set
            .moves
            .iter()
            .find(|m| m.id == "author_revision")
            .expect("his up-B is in the table");
        assert_ne!(
            up.gates.recovery,
            ambition_platformer2d::entity_catalog::RecoveryUse::None,
            "an up-B that costs nothing is flight"
        );
    }
}
