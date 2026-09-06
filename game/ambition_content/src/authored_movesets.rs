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

#[cfg(test)]
mod flow_tests {
    use super::tables;

    /// ⭐⭐ EVERY AUTHORED FLOW IN EVERY SHIPPED ROSTER VALIDATES — the POPULATION,
    /// not the two moves that happen to have one today.
    ///
    /// ⛔ `TechniqueFlow::problems()` exists because each of its failures is
    /// SILENT at runtime: a transition past the end of the list, a flow with no
    /// reachable `Finish`, a `Wait` that can never time out. Each produces a move
    /// that plays and does nothing, or a fighter stuck in a special — and neither
    /// reads as a data error to whoever is holding the controller.
    ///
    /// ⚠ THE PER-FIGHTER TESTS ARE NOT THIS TEST. The oni's and the goblin's each
    /// validate their own flow, so a flow authored on a THIRD fighter tomorrow is
    /// covered by neither. ⇒ This asks the question of the whole crate, which is
    /// the only shape that stays true as the roster grows.
    /// ⛔⛔ EVERY HELD ITEM A MOVE CREATES HAS ART, OR IT IS A PLACEHOLDER QUAD.
    ///
    /// Jon, 2026-09-05, asked for three icons — the mine, the bomb and the
    /// ponytail — and this is the executable form of that ask. `HeldItemArt`'s
    /// own doc says the resolution is *"absent / unmatched → the placeholder
    /// quad"*, so a move that spawns a pickup nobody drew ships a grey box that
    /// no test notices and every player does.
    ///
    /// ⭐ THE SCAN IS STRUCTURAL, NOT A LIST OF TECHNIQUES. `ParamValue` is a
    /// `ron::Value`, so this walks every authored effect's params for a field
    /// literally named `item_id` — whatever technique owns it. A future move
    /// that names a held item is covered without anybody remembering this test,
    /// which is the failure mode a hand-kept list of keys always has.
    ///
    /// ⚠ IT PASSES, AND I EXPECTED IT TO FAIL — which corrected the row it was
    /// written for. The campaign recorded the mine, the bomb and the ponytail as
    /// "drawing the placeholder quad", and I read that as missing REGISTRATIONS.
    /// They are all three registered (`items/held_visuals.rs`); my earlier grep
    /// matched only literal `HeldItemArtEntry::new("…")` calls and missed how
    /// these are declared. ⇒ What is actually missing is the ART FILE — no
    /// `polygon_*.png` exists anywhere in the asset tree.
    ///
    /// ⛔ AND THAT IS NOT THIS TEST'S QUESTION, deliberately. Sprites are
    /// GENERATED and gitignored, so a Rust test asserting a PNG exists would
    /// fail on every checkout that has not run the sprite pipeline. Presence
    /// belongs to `scripts/check_published_sheets_are_present.py`, which asks the
    /// renderer what it claims to install. What THIS holds is the half that is
    /// always true on every machine: a move that names a held item must have an
    /// entry, or the id resolves to nothing whatever the asset tree looks like.
    #[test]
    fn every_held_item_a_move_creates_has_art() {
        use bevy::prelude::App;

        // What the roster ASKS FOR: every `item_id` any authored effect names.
        fn item_ids_in(params: &ambition_platformer2d::entity_catalog::ParamValue) -> Vec<String> {
            let ron::Value::Map(map) = &params.0 else {
                return Vec::new();
            };
            map.iter()
                .filter(|(key, _)| {
                    matches!(key, ron::Value::String(name) if name == "item_id")
                })
                .filter_map(|(_, value)| match value {
                    ron::Value::String(id) => Some(id.clone()),
                    _ => None,
                })
                .collect()
        }

        let mut wanted: std::collections::BTreeSet<String> = Default::default();
        for (_, contract) in tables() {
            for mv in &contract.moves {
                for window in &mv.windows {
                    if let Some(effect) = window.sustain_effect.as_ref() {
                        wanted.extend(item_ids_in(&effect.params));
                    }
                }
                for event in &mv.events {
                    if let ambition_platformer2d::entity_catalog::MoveEventKind::Effect(effect) =
                        &event.kind
                    {
                        wanted.extend(item_ids_in(&effect.params));
                    }
                }
            }
        }

        // ⛔ ANTI-VACUITY. A walk that finds no item at all passes forever, and
        // it is what a structural scan looks like when the field is renamed.
        assert!(
            !wanted.is_empty(),
            "no authored effect names an `item_id`, so this guard is comparing \
             an empty set against the manifest"
        );

        // What the game DRAWS.
        let mut app = App::new();
        app.add_plugins(crate::items::AmbitionItemRosterPlugin);
        let drawn: std::collections::BTreeSet<String> = app
            .world()
            .get_resource::<ambition_platformer2d::platformer::held_item_art::HeldItemArtManifest>()
            .map(|manifest| manifest.0.iter().map(|e| e.item_id.clone()).collect())
            .unwrap_or_default();
        assert!(
            !drawn.is_empty(),
            "the item roster plugin registered no held-item art at all, so every \
             id below would be reported missing for the wrong reason"
        );

        let missing: Vec<&String> = wanted.difference(&drawn).collect();
        assert!(
            missing.is_empty(),
            "these held items are created by a move and have no art, so they draw \
             the placeholder quad: {missing:?}"
        );
    }

    /// ⛔⛔ AN AUTHORED PORTAL RISE HAS TO LAND INSIDE THE STAGE A PLAYER CAN SEE.
    ///
    /// Jon, 2026-09-05, playing it: *"the second portal appears too high, I want
    /// it to be placed so its above the main surface level, but in the visible
    /// part of the stage."* Alice's up-B opened its exit **320 px** above her,
    /// and the smash ruleset's ceiling blast margin is **240** — so the exit sat
    /// outside the playable box entirely. That is why it read as "too high"
    /// rather than merely tall.
    ///
    /// ⛔ THE BOUND IS ANOTHER CRATE'S NUMBER, AND THAT IS THE WEAKNESS OF THIS
    /// GUARD, stated rather than hidden. `ambition_demo_smash::CEILING_BLAST_MARGIN_PX`
    /// owns it; this crate cannot depend on the ruleset, so the value is repeated
    /// here. ⇒ It is `pub` over there with a doc pointing at this test, so a
    /// change has one place that names the other — but nothing MAKES them agree,
    /// and a reviewer moving the margin has to remember this line.
    ///
    /// ⚠ IT DOES NOT PIN THE VALUE. 150 is tuning and Jon's to move; what this
    /// holds is that whatever it becomes stays somewhere a player can watch it.
    #[test]
    fn an_authored_portal_rise_stays_inside_the_stage() {
        use ambition_platformer2d::characters::smash_portal::{PortalPairParams, PORTAL_PAIR};

        /// `ambition_demo_smash::CEILING_BLAST_MARGIN_PX`, repeated because this
        /// crate is below the ruleset and cannot read it.
        const CEILING_BLAST_MARGIN_PX: f32 = 240.0;

        // ⛔ THE TIMELINE'S EVENTS, NOT A WINDOW'S SUSTAIN. `author_portal_pair`
        // attaches the pair as a `MoveEvent` at a beat — it takes a TIME — and
        // scanning sustains found nothing at all. The anti-vacuity arm below is
        // what caught that, which is the whole reason it is there.
        let mut checked = 0usize;
        for (fighter, contract) in tables() {
            for mv in &contract.moves {
                for event in &mv.events {
                    let ambition_platformer2d::entity_catalog::MoveEventKind::Effect(effect) =
                        &event.kind
                    else {
                        continue;
                    };
                    if effect.key != PORTAL_PAIR {
                        continue;
                    }
                    let params: PortalPairParams =
                        effect.params.hydrate().expect("portal pair params hydrate");
                    checked += 1;
                    assert!(
                        params.rise < CEILING_BLAST_MARGIN_PX,
                        "{fighter}/{}: opens its exit {} px up, and the ruleset's \
                         ceiling blast margin is {CEILING_BLAST_MARGIN_PX}. The \
                         destination is outside the playable box — a recovery \
                         that KOs whoever takes it, and off the screen besides.",
                        mv.id,
                        params.rise,
                    );
                }
            }
        }

        // ⛔ ANTI-VACUITY. No authored portal pair means this walked nothing,
        // which is what a roster that dropped the move looks like.
        assert!(
            checked >= 1,
            "no shipped move authors a portal pair, so this guard is checking an \
             empty set"
        );
    }

    /// ⛔⛔ EVERY CANCEL TARGET RESOLVES, AND THE ROSTER ACTUALLY USES THE
    /// CONDITIONAL CANCEL.
    ///
    /// A `Cancelable` window's `into` list shares one namespace — literal move
    /// ids, verbs, and the classes in `CANCEL_CLASS_NAMES`. A name in none of
    /// the three is a DEAD STRING: the window opens, the press is looked up, and
    /// nothing answers. Silent, and indistinguishable from a move whose author
    /// never wrote a follow-up.
    ///
    /// ⭐ AND THE SECOND HALF IS THE ONE THAT CAUGHT SOMETHING. Measured
    /// 2026-09-05: `CancelCondition::OnHit`, `OnWhiff` and `OnBlock` had **zero**
    /// customers across the whole roster, while `OnHit`'s own doc describes the
    /// genre's most-pressed sequence ("combo confirm — jab chains into jab2 on
    /// hit"). A capability with no customer is a capability nobody has proved,
    /// so this guard requires one to exist.
    #[test]
    fn every_cancel_target_resolves_and_a_confirm_is_authored() {
        use ambition_platformer2d::entity_catalog::{
            base_verb_of, cancel_names_for, CancelCondition, WindowTag, CANCEL_CLASS_NAMES,
        };

        let mut dead: Vec<String> = Vec::new();
        let mut confirms = 0usize;
        let mut windows = 0usize;
        for (fighter, contract) in tables() {
            let ids: std::collections::BTreeSet<&str> =
                contract.moves.iter().map(|m| m.id.as_str()).collect();
            // ⛔⛔ THE NAMESPACE IS WIDER THAN `CANCEL_CLASS_NAMES`, and reading
            // that const as the whole of it made this guard's first run report a
            // FALSE POSITIVE on the medic's `smash`. The const omits `smash`,
            // `grab` and `taunt`, but `cancel_names_for` hands them to the
            // trigger seam — a smash press offers `["smash", "attack",
            // "any_attack"]` — so `into: ["smash"]` resolves perfectly.
            //
            // ⇒ DERIVED FROM THE REAL AUTHORITY rather than from a list I
            // believed: every verb this contract binds, its BASE, and every name
            // a press of that base offers.
            let mut verbs: std::collections::BTreeSet<&str> =
                contract.verbs.keys().map(String::as_str).collect();
            for verb in contract.verbs.keys() {
                let base = base_verb_of(verb);
                verbs.insert(base);
                for name in cancel_names_for(base, false) {
                    verbs.insert(name);
                }
                for name in cancel_names_for(base, true) {
                    verbs.insert(name);
                }
            }
            for mv in &contract.moves {
                for window in &mv.windows {
                    let WindowTag::Cancelable { into, condition } = &window.tag else {
                        continue;
                    };
                    windows += 1;
                    if !matches!(condition, CancelCondition::Always) {
                        confirms += 1;
                    }
                    for target in into {
                        let known = CANCEL_CLASS_NAMES.contains(&target.as_str())
                            || verbs.contains(target.as_str())
                            || ids.contains(target.as_str());
                        if !known {
                            dead.push(format!(
                                "{fighter}/{}: cancels into `{target}`, which is not a move id \
                                 in that contract, not one of its verbs, and not a cancel class",
                                mv.id
                            ));
                        }
                    }
                }
            }
        }

        assert!(
            dead.is_empty(),
            "cancel windows naming nothing:\n  {}",
            dead.join("\n  ")
        );
        // ⛔ ANTI-VACUITY on the census itself: a roster with no cancel windows
        // at all would satisfy the emptiness above forever.
        assert!(
            windows >= 1,
            "no shipped move authors a Cancelable window, so this guard walked \
             nothing"
        );
        assert!(
            confirms >= 1,
            "no shipped move authors a CONDITIONAL cancel. `OnHit`, `OnWhiff` and \
             `OnBlock` exist, and the genre is built on them — a roster that only \
             ever cancels on `Always` swings its second punch into a raised \
             shield, which hands the defender a free punish and takes the read \
             out of the exchange."
        );
    }

    /// ⛔⛔ EVERY NAMED REFUSAL VARIANT RESOLVES TO A MOVE THAT EXISTS.
    ///
    /// `MoveGates::when_refused` carries a MOVE ID, and an id matching nothing
    /// degrades to "no fallback at all" — deliberately, so a typo cannot crash a
    /// match. ⇒ That is exactly why it needs a guard: the failure is a DEAD
    /// BUTTON, which is the thing the field exists to prevent, and it is
    /// indistinguishable from an author who simply chose not to write one.
    ///
    /// ⚠ THE VARIANT NEED NOT OWN A VERB. The goblin's uncharged dive is bound
    /// to no press and is reachable only through this field, so the question is
    /// membership in `moves` — what `move_by_id` actually searches — and not
    /// whether anything can press it.
    ///
    /// ⭐ ASKED OF THE WHOLE CRATE rather than per fighter, for the reason the
    /// flow census below gives: a variant authored on a THIRD fighter tomorrow
    /// is covered by no per-fighter test.
    #[test]
    fn every_named_move_variant_resolves() {
        let mut dangling: Vec<String> = Vec::new();
        let mut named = 0usize;
        for (fighter, contract) in tables() {
            for mv in &contract.moves {
                let Some(target) = mv.gates.when_refused.as_deref() else {
                    continue;
                };
                named += 1;
                if contract.move_by_id(target).is_none() {
                    dangling.push(format!(
                        "{fighter}/{}: when_refused names `{target}`, which no move in \
                         that contract carries",
                        mv.id
                    ));
                }
                if target == mv.id {
                    dangling.push(format!(
                        "{fighter}/{}: names ITSELF as its refusal variant. One hop, and \
                         the hop lands on the move that was just refused, so this is a \
                         dead button written the long way",
                        mv.id
                    ));
                }
            }
        }
        assert!(
            dangling.is_empty(),
            "refusal variants that cannot resolve:\n  {}",
            dangling.join("\n  ")
        );
        // ⛔ ANTI-VACUITY, and this guard needs it more than most: it passed
        // every day before the field existed, and would pass again the day
        // somebody deleted the last authored variant.
        assert!(
            named >= 1,
            "no move in any shipped roster authors `when_refused`, so this guard \
             is validating an empty set"
        );
    }

    #[test]
    fn every_authored_flow_in_the_shipped_rosters_validates() {
        let mut broken: Vec<String> = Vec::new();
        let mut flows = 0usize;
        for (fighter, contract) in tables() {
            for mv in &contract.moves {
                let Some(flow) = mv.flow.as_ref() else {
                    continue;
                };
                flows += 1;
                for problem in flow.problems() {
                    broken.push(format!("{fighter}/{}: {problem}", mv.id));
                }
            }
        }
        assert!(
            broken.is_empty(),
            "authored flows that cannot run:\n  {}",
            broken.join("\n  ")
        );
        // ⛔ ANTI-VACUITY. A census that walks no flows passes forever, and this
        // one would have passed every day before the first flow was authored —
        // including a day when somebody deleted them all.
        assert!(
            flows >= 2,
            "only {flows} authored flow(s) found across every shipped roster, so \
             this guard is validating an empty set"
        );
    }
}

#[cfg(test)]
mod expressiveness_census {
    use super::tables;

    /// ⭐⭐ HOW MANY FIGHTERS HAVE A SPECIAL THAT DOES SOMETHING A STRIKE CANNOT —
    /// the goal's own complaint, measured instead of felt.
    ///
    /// Jon's standing goal says *"many have boring specials"*, and every roster
    /// decision on this campaign has been argued from a reading rather than a
    /// number. ⇒ This counts, from the authored DATA rather than from a grep over
    /// the source: a special is EXPRESSIVE when it carries a technique
    /// (`MoveEventKind::Effect`), a stance (`sustain_effect`), a flow, or a
    /// gravity regime. A strike with cues is not.
    ///
    /// ⛔ IT IS A RATCHET, NOT A TARGET. Asserting that EVERY fighter must be
    /// expressive would be a design claim nobody has made — a plain-strike
    /// brawler is a legitimate character. What is not legitimate is going
    /// BACKWARDS silently, so this holds the floor at what the roster has today
    /// and prints the ranking when it fails.
    ///
    /// ⚠ IT WALKS THIS CRATE'S TABLES, WHICH IS NOT THE SMASH GRID.
    /// `tables()` is *"every table in this crate that authors move events"* — so
    /// it includes `theorem_chain`, Robot **v2**'s DUEL-ARENA moveset, which
    /// shares a file with v3's platform-fighter table. It is counted as plain and
    /// that is correct: it is a two-hit combo demo and deliberately data-only.
    /// ⇒ Do not read the plain list as "boring smash fighters" without checking
    /// which composition each entry belongs to.
    ///
    /// ⚠ THE PRINTED LIST IS HALF THE POINT. When this fails, the message names
    /// which fighters are carrying the roster and which are not, which is the
    /// question "many have boring specials" was actually asking.
    #[test]
    fn the_roster_does_not_get_less_expressive() {
        use ambition_entity_catalog::MoveEventKind;

        /// The floor, raised deliberately as fighters gain techniques. Bumping it
        /// is a decision; watching it silently fall is the failure.
        const FLOOR: usize = 18;

        let mut expressive: Vec<&str> = Vec::new();
        let mut plain: Vec<&str> = Vec::new();
        for (fighter, contract) in tables() {
            // Only the SPECIALS: a jab with a technique is not what the goal is
            // asking about.
            let special_ids: Vec<&String> = contract
                .verbs
                .iter()
                .filter(|(verb, _)| verb.starts_with("special"))
                .map(|(_, id)| id)
                .collect();
            let rich = contract.moves.iter().any(|mv| {
                if !special_ids.iter().any(|id| **id == mv.id) {
                    return false;
                }
                mv.flow.is_some()
                    || mv.windows.iter().any(|w| {
                        w.sustain_effect.is_some()
                            // ⛔⛔ A VOLUME REACTION IS EXPRESSIVE AND THIS TEST
                            // SAID IT WAS NOT. `VolumeReaction::{Autolink,
                            // Windbox}` change what a hit DOES — a gather, a
                            // shove — and the first version of this census
                            // counted only techniques, stances, flows and
                            // gravity. ⇒ It called the cellular automaton plain
                            // while its `generation_collapse` autolinks victims
                            // into one cell, which is the most characterful move
                            // on that fighter. I was one step from authoring a
                            // second mechanic for a fighter that already had one,
                            // on the strength of my own guard's definition.
                            || w.volumes.iter().any(|v| v.reaction.is_some())
                    })
                    || mv.events.iter().any(|e| {
                        matches!(
                            e.kind,
                            MoveEventKind::Effect(_) | MoveEventKind::GravityModifier { .. }
                        )
                    })
            });
            if rich {
                expressive.push(fighter);
            } else {
                plain.push(fighter);
            }
        }

        // ⭐ A CENSUS THAT NAMES ITS MEMBERS. The count alone answers "did it get
        // worse"; the NAMES answer "who is next", which is the question anybody
        // running this actually has. Printed on the way past rather than only in
        // the failure message, because the failure message is unreachable while
        // the roster is healthy and that is exactly when you want the list.
        println!(
            "[expressiveness] {} expressive, {} plain\n  plain: {plain:?}",
            expressive.len(),
            plain.len()
        );
        assert!(
            expressive.len() >= FLOOR,
            "the roster's expressive-special count fell to {} (floor {FLOOR}).\n  \
             expressive: {expressive:?}\n  plain: {plain:?}",
            expressive.len()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d::entity_catalog::MoveEventKind;

    /// Does this move carry a TECHNIQUE, or is it a hitbox and nothing else?
    ///
    /// Both roads count: a technique fired at an instant is an `Effect` event,
    /// and one that is live for a window (a capture attempt, a charge) hangs off
    /// the window as a `sustain_effect`. Counting only the first is how a census
    /// reports a grab as featureless — see the tether row, where exactly that
    /// scan came up empty against a move whose whole point is its capture.
    fn techniques(spec: &ambition_platformer2d::entity_catalog::MoveSpec) -> Vec<String> {
        let mut keys: Vec<String> = spec
            .events
            .iter()
            .filter_map(|event| match &event.kind {
                MoveEventKind::Effect(effect) => Some(effect.key.clone()),
                _ => None,
            })
            .collect();
        keys.extend(
            spec.windows
                .iter()
                .filter_map(|window| window.sustain_effect.as_ref())
                .map(|effect| effect.key.clone()),
        );
        keys.sort();
        keys.dedup();
        keys
    }

    /// ⭐⭐ THE CENSUS JON ASKED FOR IN AS MANY WORDS: *"we have a lot of
    /// characters with boring specials, and when we build the code for these we
    /// should exercise them in the characters."* A special with no technique is
    /// a hitbox on a different button — it may be perfectly tuned, but nothing
    /// about it is the fighter's own.
    ///
    /// ⛔ IT ASSERTS ONLY THAT IT MEASURED SOMETHING. There is no correct number
    /// of bare specials: a brawler's up-B that is honestly just a rising hitbox
    /// is a legitimate design, and a floor on "techniques per fighter" would be
    /// this file inventing a quota. What it is for is READING — run it with
    /// `--nocapture` and the roster sorts itself by how much of each fighter is
    /// actually authored.
    ///
    /// ⛔⛔ AND "BARE" MEANS NO TECHNIQUE, NOT "BORING" — a distinction this
    /// census learned the hard way. Reading the first version I went looking for
    /// the emptiest fighter and found the Perfect Cellular Automaton with five
    /// bare specials, then opened its down-B: a three-pulse `multihit` collapse
    /// with authored autolink volumes, a telegraph, and its own cue. Nothing
    /// about it wants a technique key. ⇒ A fighter can be richly authored with
    /// none, so the census now reports what ELSE a special carries — extra
    /// windows, a charge, a start impulse, timeline events. A special with
    /// NEITHER a technique nor any of those is the only row that is honestly
    /// a hitbox on a different button, and those are the ones worth reading.
    #[test]
    fn the_census_of_specials_that_carry_no_technique() {
        let tables = tables();
        let mut bare_total = 0usize;
        let mut plain_total = 0usize;
        let mut special_total = 0usize;
        println!("\n{:<22} {:<34} {}", "FIGHTER", "SPECIAL", "TECHNIQUE");
        for (name, table) in &tables {
            let specials: Vec<&str> = table
                .verbs
                .iter()
                .filter(|(verb, _)| verb.starts_with("special"))
                .map(|(_, id)| id.as_str())
                .collect();
            for id in specials {
                let Some(spec) = table.moves.iter().find(|m| m.id == id) else {
                    continue;
                };
                special_total += 1;
                let keys = techniques(spec);
                // What a move carries besides a technique. Every one of these is
                // authoring a reader would call expressive.
                let mut extras: Vec<String> = Vec::new();
                let hitbox_windows = spec
                    .windows
                    .iter()
                    .filter(|window| !window.volumes.is_empty())
                    .count();
                if hitbox_windows > 1 {
                    extras.push(format!("{hitbox_windows} hit windows"));
                }
                if spec.smash_charge.is_some() {
                    extras.push("charge".to_string());
                }
                if spec.start_impulse.is_some() {
                    extras.push("impulse".to_string());
                }
                if !spec.events.is_empty() {
                    extras.push(format!("{} event(s)", spec.events.len()));
                }
                if keys.is_empty() {
                    bare_total += 1;
                    if extras.is_empty() {
                        plain_total += 1;
                    }
                }
                println!(
                    "{:<22} {:<34} {}",
                    name,
                    id,
                    if !keys.is_empty() {
                        keys.join(", ")
                    } else if extras.is_empty() {
                        "— PLAIN (no technique, no other authoring)".to_string()
                    } else {
                        format!("— no technique, but {}", extras.join(" + "))
                    },
                );
            }
        }
        println!(
            "\n{bare_total} of {special_total} specials across {} fighters carry \
             no technique; {plain_total} of those carry no other authoring \
             either, and those are the ones worth reading.",
            tables.len(),
        );
        assert!(
            special_total >= 20,
            "the census found only {special_total} specials across {} fighters, \
             so it has lost its corpus rather than found a tidy roster",
            tables.len(),
        );
    }
}
