//! EMMY ETHEREAL'S BODY, now that she has answers.
//!
//! she leaves [`super::hall_humanoids`] under that file's own rule: *"one file
//! for four... If one of them grows a moveset or a distinct build, it earns its
//! own file that day."* This is that day, and it is the second time that rule has
//! fired this week — Oiler went first.
//!
//! the MOVESET is what is new, and it reaches the fighter through
//! `with_moveset` — see [`crate::emmy_noether_moveset`] for the table and why it is
//! shaped the way it is. Her `default_action_set` also stops being `peaceful` in
//! the same change; the two halves answer different questions (*may this body
//! attack* versus *what the attack is*) and a fighter needs both.

use ambition_characters::actor::CharacterLocomotion;
use ambition_characters::brain::MoveStyleSpec;
use ambition_platformer2d::character::CharacterDefinition;

/// See the module doc. Reached through [`super::AUTHORED_CAST`], which is also
/// what makes this character buildable — there is no second list to remember.
pub(crate) fn author(_id: &str, definition: CharacterDefinition) -> CharacterDefinition {
    let mut definition = definition
        .with_locomotion(CharacterLocomotion {
            run_speed: super::hall_humanoids::HUMANOID_RUN_SPEED,
            move_style: MoveStyleSpec::Walk,
            ..Default::default()
        })
        .with_moveset(crate::emmy_noether_moveset::emmy_noether_moveset())
        // EMMY'S ONE MECHANICAL JOKE, AND IT IS HER WHOLE SUBJECT.
        //
        // Two Emmys driven by CPUs think on the SAME deterministic cognitive
        // stream, so an Emmy-vs-Emmy mirror match on a symmetric stage plays as a
        // reflection: her barks are *"every symmetry hides a conservation law"*
        // and *"spin the rules and the laws don't change"*, and this is that
        // sentence made playable rather than spoken.
        //
        // NOT a synchronised animation, and it must never become one. The
        // property is *identical cognition + symmetric information → symmetric
        // behaviour*. Nothing compares the two bodies and nothing copies one's
        // action to the other; they agree because they are drawing from one
        // stream and reading a stage that happens to be symmetric.  the mirror
        // BREAKS the moment their observations diverge — one takes a hit, one
        // gets launched further, one is nearer a ledge — and that is correct.
        // A mirror that survived asymmetric information would be a puppet show,
        // and Noether's theorem is precisely the claim that the symmetry has to
        // be real for the consequence to hold.
        //
        // Emmy is the only character for which that was the interesting answer, so she now asks for
        // it out loud.
        .preserving_mirror_symmetry();
    // Unchanged by the kit — a mathematician with a theorem is not a bigger body.
    definition.vitals.max_health = Some(4);
    definition
}

#[cfg(test)]
mod tests {
    /// Emmy's mirror symmetry is AUTHORED and reachable through the one cast
    /// table — not a special case wired into the brain.
    ///
    /// it goes through [`super::super::author_for`] rather than calling
    /// [`super::author`] directly, so the test also proves she is IN
    /// `AUTHORED_CAST`. Authoring a trait on a character no table reaches would
    /// be indistinguishable from authoring nothing.
    #[test]
    fn emmy_authors_her_cpu_mirror_symmetry() {
        let author = super::super::author_for("npc_emmy_noether")
            .expect("Emmy is in AUTHORED_CAST, or nothing she authors is reachable");
        let definition = author(
            "npc_emmy_noether",
            super::super::CharacterDefinition::new("npc_emmy_noether", "Emmy Ethereal", "ambition"),
        );
        assert!(
            definition.preserves_mirror_symmetry,
            "Emmy stopped authoring the mirror-symmetry trait, so an Emmy-vs-Emmy \
             match now plays like any other pair of CPUs"
        );
    }

    #[test]
    fn an_ordinary_authored_character_does_not_preserve_mirror_symmetry() {
        let author = super::super::author_for("npc_pirate_admiral")
            .expect("the Admiral is in AUTHORED_CAST");
        let definition = author(
            "npc_pirate_admiral",
            super::super::CharacterDefinition::new(
                "npc_pirate_admiral",
                "Pirate Admiral",
                "ambition",
            ),
        );
        assert!(
            !definition.preserves_mirror_symmetry,
            "an ordinary fighter authored mirror symmetry, so the trait has \
             leaked into the default"
        );
    }
}
