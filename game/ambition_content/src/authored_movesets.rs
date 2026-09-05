//! Every moveset THIS CRATE authors, in one list.
//!
//! ⛔⛔ NOT THE SELECTABLE CAST, and the distinction cost a real proof. The
//! shark's one-hit survivability census scanned this list and read as a
//! statement about the game. Twenty-one fighters are selectable; this list holds
//! the ones whose tables live in THIS crate, and a hand-kept list narrows in
//! silence because the crate that owns it cannot know a fighter was added
//! somewhere else.
//!
//! ⚠ **AND IT HAD NARROWED, EXACTLY AS THAT SENTENCE PREDICTED — 12 of 19,
//! found 2026-09-05 and completed the same day.** This paragraph used to name
//! Pointed, Projectile and Pugnacious Polygon, the Author, the Performer, the
//! Officer and the Medic as fighters the roster seats and this list does not
//! hold. Six of those seven have their moveset file in this crate, so they were
//! never examples of the cast/list distinction — they were the gap, sitting
//! inside a sentence written to explain why a gap was fine. ⇒ Mary-O and Sanic
//! ARE that distinction: their tables live in their own crates, and no list
//! here can reach them.
//!
//! ⭐ THE CAST HAS AN AUTHORITY AND IT IS NOT A TABLE: `SmashRoster::assemble`
//! against a live `PreparedCharacterRegistry`, then each prepared character's
//! `kit.projectable_moveset()`. It costs an app, which is why this list existed
//! — but a census is worth an app, and
//! `a_recovery_mount_cannot_be_deleted_by_one_hit` now pays it.
//!
//! ⇒ WHAT THIS LIST IS FOR is the question it can actually answer: does every
//! move THIS CRATE authors drive its own seam correctly (`moveset_sound`). That
//! subject and this list are the same thing by construction.

use ambition_entity_catalog::MovesetContract;

/// Every table in this crate that authors move events, by the name a failure
/// should print.
pub fn tables() -> Vec<(&'static str, MovesetContract)> {
    vec![
        ("alice", crate::alice_moveset::alice_moveset()),
        ("bob", crate::bob_moveset::bob_moveset()),
        (
            "carl_stargan",
            crate::carl_stargan_moveset::carl_stargan_moveset(),
        ),
        (
            "cellular_automaton",
            crate::cellular_automaton_moveset::cellular_pulse_moveset(),
        ),
        ("goblin", crate::goblin_moveset::goblin_moveset()),
        (
            "ninja_shadow_oni_leader",
            crate::ninja_shadow_oni_leader_moveset::ninja_shadow_oni_leader_moveset(),
        ),
        (
            "emmy_noether",
            crate::emmy_noether_moveset::emmy_noether_moveset(),
        ),
        ("oiler", crate::oiler_moveset::oiler_moveset()),
        (
            "patent_clerk",
            crate::patent_clerk_moveset::patent_clerk_moveset(),
        ),
        (
            "pirate_admiral",
            crate::pirate_admiral_moveset::pirate_admiral_moveset(),
        ),
        (
            "player_robot",
            crate::player_robot_moveset::player_robot_moveset(),
        ),
        (
            "theorem_chain",
            crate::player_robot_moveset::theorem_chain_moveset(),
        ),
        // ⭐⭐ THE SEVEN THIS LIST NAMED AS ABSENT AND THEN WAS COMPLETED WITH,
        // 2026-09-05. The module doc above explained that the Smash roster seats
        // fighters this list does not hold — true of Mary-O and Sanic, whose
        // tables live in their own crates, and NOT true of these seven, whose
        // moveset files are in this crate. So the header's "every moveset THIS
        // CRATE authors" was the accurate sentence and the list was the thing
        // out of step: 12 of 19.
        ("author", crate::author_moveset::author_moveset()),
        ("medic", crate::medic_moveset::medic_moveset()),
        ("officer", crate::officer_moveset::officer_moveset()),
        ("performer", crate::performer_moveset::performer_moveset()),
        (
            "pointed_polygon",
            crate::pointed_polygon_moveset::pointed_polygon_moveset(),
        ),
        (
            "projectile_polygon",
            crate::projectile_polygon_moveset::projectile_polygon_moveset(),
        ),
        (
            "pugnacious_polygon",
            crate::pugnacious_polygon_moveset::pugnacious_polygon_moveset(),
        ),
    ]
}

#[cfg(test)]
mod reach_tests {
    use ambition_characters::smash_capture::{CaptureAttemptParams, CAPTURE_ATTEMPT};

    /// The ordinary ceiling for a grab's reach, in pixels.
    ///
    /// A fifth of the shipped smash platform's 480px width — the same number
    /// and the same reasoning as the smash demo's own ceiling, stated against
    /// the STAGE rather than the body so it means something a reader can check.
    const ORDINARY_REACH_PX: f32 = 96.0;

    /// What a declared TETHER may reach instead.
    ///
    /// A third of the platform. A tether is supposed to be startling; what it
    /// may not be is a grab that covers the stage.
    const TETHER_REACH_PX: f32 = 160.0;

    /// The grabs allowed past [`ORDINARY_REACH_PX`], and why.
    ///
    /// ⛔⛔ AN ALLOWLIST RATHER THAN A RAISED CEILING, and the difference is the
    /// whole guard. Lifting the single ceiling to 160 would let EVERY fighter
    /// grow a tether silently, one authored number at a time, and the guard
    /// would report nothing until the whole roster reached a third of the stage.
    /// Naming the exceptions makes "this fighter has a tether" a reviewed fact
    /// with a line number.
    /// ⭐ NAMES THE AUTHORED GRAB ONLY. `author_standing_grab` derives a running
    /// variant by cloning the standing grab's WINDOWS — so a tether standing
    /// grab is automatically a tether dash grab, which is genre-correct and is
    /// also not a second decision. Listing `…_grab_dash` separately would make
    /// the derivation look like an independent permission and let the two drift.
    const TETHERS: &[&str] = &[
        // The grid's ranged fighter. Samus's grab is a tether, and hers is the
        // only combat identity on the roster built around distance.
        "polygon_projectile_grab",
    ];

    /// Is this move a declared tether, or the running variant the engine derives
    /// from one?
    fn declared_tether(move_id: &str) -> bool {
        TETHERS.iter().any(|tether| {
            move_id == *tether || move_id.strip_suffix("_dash") == Some(*tether)
        })
    }

    /// No authored grab reaches further than the stage allows, across every
    /// moveset THIS CRATE authors.
    ///
    /// ⛔⛔ ITS SIBLING IN THE SMASH DEMO COULD NOT SEE THIS CRATE AT ALL, and
    /// that is why this exists rather than a shared helper.
    /// `no_grab_this_demo_authors_reaches_further_than_the_stage_allows`
    /// (renamed from `no_authored_grab_…` in this same commit, because the old
    /// name was the overclaim) iterates the stand-in kit and George — the two
    /// movesets the demo owns — while
    /// `ambition_demo_smash` does not depend on `ambition_content`, so eleven
    /// selectable fighters' grabs were outside a guard whose NAME says
    /// "no authored grab". A reader checking whether tethers were covered would
    /// have read that name and stopped.
    ///
    /// ⇒ The population is `tables()`, which was itself 12 of the 19 movesets
    /// this crate authors until the same day this landed. A guard is only as
    /// honest as the list it walks.
    #[test]
    fn no_authored_grab_reaches_further_than_the_stage_allows() {
        let mut seen = 0usize;
        let mut tethers_seen = 0usize;
        for (who, set) in super::tables() {
            for spec in &set.moves {
                for window in &spec.windows {
                    let Some(effect) = window.sustain_effect.as_ref() else {
                        continue;
                    };
                    if effect.key != CAPTURE_ATTEMPT {
                        continue;
                    }
                    let params: CaptureAttemptParams = effect
                        .params
                        .hydrate()
                        .expect("an authored capture attempt must hydrate");
                    seen += 1;
                    // The far edge of the reach box along the captor's facing.
                    let reach = params.offset.0.abs() + params.half_extents.0.abs();
                    let is_tether = declared_tether(&spec.id);
                    if is_tether {
                        tethers_seen += 1;
                    }
                    let ceiling = if is_tether {
                        TETHER_REACH_PX
                    } else {
                        ORDINARY_REACH_PX
                    };
                    assert!(
                        reach <= ceiling,
                        "{who}'s `{}` reaches {reach}px (offset {:?} + half \
                         {:?}), past the {ceiling}px ceiling. If this is a \
                         deliberate tether, add its move id to TETHERS here in \
                         the same commit that authors it; if it is a typo, this \
                         is the only thing that would have caught it",
                        spec.id,
                        params.offset,
                        params.half_extents,
                    );
                    assert!(
                        params.half_extents.0 > 0.0 && params.half_extents.1 > 0.0,
                        "{who}'s `{}` has a non-positive grab box {:?}, so it \
                         can never catch anybody",
                        spec.id,
                        params.half_extents,
                    );
                }
            }
        }
        // ⛔ THE POPULATION FLOOR. This crate authors several standing grabs; a
        // run that found none would pass every assertion above and mean the
        // capture key, the window shape or `tables()` had moved under it.
        assert!(
            seen >= 3,
            "only {seen} authored capture attempt(s) were found across \
             {} movesets, so this guard is measuring nothing rather than passing",
            super::tables().len(),
        );
        // ⛔ AND THE ALLOWLIST MUST BE LIVE. A `TETHERS` entry naming a move that
        // no longer exists is a permission nobody can see being granted, and it
        // would keep the ceiling raised for a move id a typo could reintroduce.
        // ⛔ AND THE ALLOWLIST MUST BE LIVE. An entry naming a move that no
        // longer exists is a permission nobody can see being granted. Each
        // tether contributes TWO — itself and the derived running variant — so
        // a count below that also catches the derivation silently disappearing.
        assert_eq!(
            tethers_seen,
            TETHERS.len() * 2,
            "TETHERS names {} move(s), which should appear as {} entries once \
             each derived `_dash` variant is counted, but {tethers_seen} were \
             found: {TETHERS:?}. Either an entry names a move that does not \
             exist, or `author_standing_grab` stopped deriving the running grab",
            TETHERS.len(),
            TETHERS.len() * 2,
        );
    }
}
