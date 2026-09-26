//! The versus stage's two duelists, as content.
//!
//! Who they are, how hard they hit and what they are hittable through is the
//! stage's content pack (`assets/pack.ron`): two catalog rows and two move
//! files, one grammar with two sets of numbers. This module registers that cast
//! and states the stage's ability ceiling, which is a ruleset fact, not a
//! character's.

/// The versus pack: `assets/pack.ron` and every source it declares, with the
/// path `pack.ron` spells.
pub static PACK: ambition_platformer2d::content::EmbeddedPack =
    ambition_platformer2d::content::EmbeddedPack::new(
        include_str!("../../assets/pack.ron"),
        &[
            (
                "data/character_catalog.ron",
                include_str!("../../assets/data/character_catalog.ron"),
            ),
            (
                "data/movesets/arena_duelist_long.ron",
                include_str!("../../assets/data/movesets/arena_duelist_long.ron"),
            ),
            (
                "data/movesets/arena_duelist_close.ron",
                include_str!("../../assets/data/movesets/arena_duelist_close.ron"),
            ),
        ],
    );

/// Register the duelists: the catalog fragment and every row as a character,
/// with its move table. `default_character` is this experience's catalog
/// default.
pub fn register_versus_cast(app: &mut bevy::prelude::App, default_character: &str) {
    PACK.cast(VERSUS_PROVIDER, Some(default_character)).register(app);
}

/// THE STAGE'S CEILING: the verbs a duelist may use here. `versus.rs` declares
/// `MatchAbilities::at_most(..)`, a ceiling and no floor: a character keeps what
/// it authored, minus what this duel forbids, and an unauthored character gets
/// nothing (permission is not a grant). Each duelist's row grants itself exactly
/// this kit (`abilities` in `character_catalog.ron`), so the ceiling narrows
/// nothing a fighter authored.
///
/// `reset` and `interact` come from `basic()`, and the rows grant them too, so
/// the duel plays as it did.
pub const VERSUS_FIGHTER_KIT: ambition_platformer2d::engine_core::AbilitySet =
    ambition_platformer2d::engine_core::AbilitySet {
        attack: true,
        fast_fall: true,
        // ⭐ STATED, NOT INHERITED — and it used to be inherited from a place no
        // authoring could see. `basic()` leaves this false, and a duelist still
        // double-jumped because `ActorBody::from_kit` unioned a locomotion floor
        // into every actor body after the character had spoken. That union is
        // gone (a union cannot express a refusal, so no character could decline
        // it), which would have taken this fighter's second jump with it.
        //
        // Writing it here keeps the duel exactly as it plays and puts the bit
        // where it can be read and declined — the same reason the note above
        // gives for leaving `reset` and `interact` riding in from `basic()`
        // rather than tidying them: behaviour-neutral, or not worth making.
        double_jump: true,
        ..ambition_platformer2d::engine_core::AbilitySet::basic()
    };

/// The provider these fighters are attributed to.
///
/// The versus experience, not the host and not either demo: a cue or a swing
/// these fighters make is the ARENA's, which is what keeps presentation source
/// attribution honest for a crossover stage.
pub const VERSUS_PROVIDER: &str = super::versus::VERSUS_EXPERIENCE;

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d::entity_catalog::{AttackDir, MovesetContract};

    #[test]
    fn a_duelist_answers_every_direction_and_the_smash_is_the_heavy_one() {
        let moveset = PACK.moveset("arena_duelist_long");
        for dir in [
            AttackDir::Neutral,
            AttackDir::Forward,
            AttackDir::Up,
            AttackDir::Down,
            AttackDir::Back,
        ] {
            assert!(
                moveset
                    .move_for_directional_verb("attack", dir, true)
                    .is_some(),
                "a grounded duelist pressed attack while holding {dir:?} and got \
                 nothing — a direction with no answer is a dead button"
            );
            assert!(
                moveset
                    .move_for_directional_verb("attack", dir, false)
                    .is_some(),
                "an AIRBORNE duelist got nothing for {dir:?}: the grounded-only \
                 moves must fall through to the plain jab, not refuse"
            );
        }

        // The airborne fighter falls through to the jab rather than getting the
        // grounded smash. This is the property the gates exist for, and asserting
        // only "something came back" above would pass while they did nothing.
        assert_eq!(
            moveset
                .move_for_directional_verb("attack", AttackDir::Forward, false)
                .map(|mv| mv.id.as_str()),
            Some("jab")
        );
        assert_eq!(
            moveset
                .move_for_directional_verb("attack", AttackDir::Forward, true)
                .map(|mv| mv.id.as_str()),
            Some("smash_forward")
        );
    }

    #[test]
    fn the_two_fighters_differ_in_numbers_and_not_in_grammar() {
        let (a, b) = (
            PACK.moveset("arena_duelist_long"),
            PACK.moveset("arena_duelist_close"),
        );
        assert_eq!(
            a.verbs, b.verbs,
            "the two fighters must answer the same buttons: a fighting game where \
             one character has a button the other does not is two games"
        );
        let heavy = |m: &MovesetContract| {
            m.move_by_id("smash_forward")
                .and_then(|mv| mv.windows.iter().find_map(|w| w.volumes.first()))
                .map(|v| v.damage)
                .expect("the smash lands a volume")
        };
        assert_ne!(
            heavy(&a),
            heavy(&b),
            "the two fighters are numerically identical, so the roster is one \
             character wearing two names"
        );
    }

    /// What each duelist's row grants is exactly the stage's ceiling, so the
    /// ceiling narrows nothing and an unauthored fighter would get nothing.
    ///
    /// `MatchAbilities::apply`'s old `authored.unwrap_or(self.permitted)` arm
    /// handed an unauthored character the whole CEILING, so PERMISSION became a
    /// GRANT. Retiring it was safe only because what the duelists author is
    /// what that arm handed them.
    #[test]
    fn what_the_duelists_author_is_exactly_the_stages_ceiling() {
        use ambition_platformer2d::engine_core::{AbilitySet, MatchAbilities};

        let rules = MatchAbilities::at_most(VERSUS_FIGHTER_KIT);
        assert_eq!(
            rules.apply(None),
            AbilitySet::NONE,
            "an unauthored fighter took the ceiling again — the migration bridge \
             D151 retired is back, and permission is a grant once more"
        );
        // non-vacuity: a kit of nothing would satisfy the equalities for the
        // wrong reason. These two fighters throw punches.
        assert!(
            VERSUS_FIGHTER_KIT.attack && VERSUS_FIGHTER_KIT.move_horizontal && VERSUS_FIGHTER_KIT.jump,
            "the versus kit cannot fight, so the equalities compare empty sets"
        );
        let catalog = ambition_platformer2d::characters::actor::character_catalog::lowered_catalog(
            PACK.prepared(),
        )
        .expect("the versus pack states its cast");
        for (id, row) in &catalog.characters {
            let grants = row.abilities.as_deref().expect("a duelist states its verbs");
            assert_eq!(
                rules.apply(Some(AbilitySet::compose(grants))),
                VERSUS_FIGHTER_KIT,
                "`{id}` does not author exactly the stage's kit, so the ceiling \
                 narrows it or it lacks a verb the duel expects"
            );
        }
        assert_eq!(catalog.characters.len(), 2, "the versus pack states two duelists");
    }

    /// A duelist on a pedestal says its own lines in every situation: the bark
    /// resolver reads the row's fallback pool before any other floor.
    #[test]
    fn a_duelist_speaks_in_its_own_voice() {
        use ambition_platformer2d::characters::actor::character_catalog::{
            lowered_catalog, BarkSituation, CharacterCatalog,
        };
        let catalog = CharacterCatalog::from_data(
            lowered_catalog(PACK.prepared()).expect("the versus pack states its cast").clone(),
        );
        for situation in [BarkSituation::Hall, BarkSituation::OnHit, BarkSituation::Provoked] {
            assert_eq!(
                catalog.bark_line("arena_duelist_long", situation, 0),
                Some("Reach is patience with a longer arm."),
                "the long guard at {situation:?}"
            );
            assert_eq!(
                catalog.bark_line("arena_duelist_close", situation, 1),
                Some("Your reach is a commitment. I am counting on it."),
                "the close guard at {situation:?}"
            );
        }
    }
}
