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
    use ambition_entity_catalog::smash_capture::{CaptureAttemptParams, CAPTURE_ATTEMPT};

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

    /// ⛔⛔ **EVERY SHIPPED FLOW STILL RUNS THE TRACE IT WAS AUTHORED FOR** —
    /// A12's last acceptance row, asked of the AUTHORED CONTENT rather than of a
    /// synthetic fixture.
    ///
    /// `ambition_combat`'s own flow tests build their graphs by hand, so every
    /// one of them would keep passing while a shipped flow's edges were rewired
    /// underneath them. `problems()` runs at authoring and is STRUCTURAL — it
    /// catches a dangling edge and an unreachable `Finish` and says nothing about
    /// what the flow DOES. Nothing anywhere ran a shipped flow through the
    /// interpreter, so "existing flows retain their traces" was an acceptance row
    /// with no witness.
    ///
    /// ⭐ BOTH ROADS OF EVERY FLOW, because these are all `Wait`-shaped and a
    /// trace taken on one road cannot see the other. The satisfied road is the
    /// grab; the timeout road is the whiff, and the whiff emitting NOTHING is the
    /// half a careless edit breaks — an `on_timeout` pointed at the `Emit`
    /// instead of at `Finish` hands the fighter a free grab for missing, which is
    /// exactly the balance decision the goblin's own comment agonises over.
    ///
    /// ⚠ THE EXPECTED TABLE IS EXACT AND MUST COVER THE DISCOVERED SET. A new
    /// authored flow FAILS this test until somebody writes down what it emits.
    /// That is the ratchet: a hand-kept list that only ever gets read is the
    /// failure mode this file's own header describes.
    #[test]
    fn every_shipped_flow_still_runs_the_trace_it_was_authored_for() {
        use ambition_combat::moveset::{advance_move_playback, MoveEventMessage, MovePlayback};
        use ambition_entity_catalog::MoveEventKind;
        use bevy::prelude::*;

        // ⛔ A ROAD IS A CONTACT STATE, NOT A BOOLEAN. The oni's flow BRANCHES on
        // `Blocked` after waiting on `Overlapped`, so it has three roads, and a
        // two-road table would leave the branch untested — the exact shape of a
        // guard walking half its subject. `MoveContact::overlapped` is derived as
        // `landed || connected || blocked`, so a blocked road needs no separate
        // landed flag to count as having touched something.
        #[derive(Clone, Copy)]
        struct Road {
            what: &'static str,
            landed: bool,
            connected: bool,
            blocked: bool,
        }
        const WHIFFED: Road = Road {
            what: "touched nothing",
            landed: false,
            connected: false,
            blocked: false,
        };
        const CONNECTED: Road = Road {
            what: "connected with a body",
            landed: true,
            connected: true,
            blocked: false,
        };
        const BLOCKED: Road = Road {
            what: "was eaten by a guard",
            landed: true,
            connected: false,
            blocked: true,
        };

        let grab = ambition_entity_catalog::smash_capture::CAPTURE_ATTEMPT;
        let teleport = ambition_entity_catalog::smash_teleport::TELEPORT;

        // Every shipped flow, every road, and what it emits there.
        let expected: std::collections::BTreeMap<&str, Vec<(Road, Vec<&str>)>> = [
            (
                // THE GOBLIN'S TACKLE, three nodes: wait on the connect, grab,
                // finish. A landed charge grabs; a whiff is the punish window and
                // has to stay empty-handed.
                "headlong_charge",
                vec![
                    (CONNECTED, vec![grab]),
                    (WHIFFED, vec![]),
                    // ⭐ A BLOCKED CHARGE MUST NOT GRAB, and this road is the one
                    // the move's own comment argues for: waiting on the overlap
                    // "would hand the goblin a grab for running into a shield —
                    // the single most punishable thing in the genre becoming its
                    // best option". That is a balance decision written in prose
                    // beside an authored edge; this is the assertion of it.
                    (BLOCKED, vec![]),
                ],
            ),
            (
                // THE ONI LEADER'S IAIJUTSU, four nodes: wait on the overlap,
                // branch on the guard, teleport behind them, finish.
                "iaijutsu",
                vec![
                    (BLOCKED, vec![teleport]),
                    // ⛔ HE DOES NOT ESCAPE A HIT THAT LANDED. The branch's
                    // `otherwise` is `Finish`; a connect taking the teleport road
                    // would let him cut and vanish with no answer.
                    (CONNECTED, vec![]),
                    (WHIFFED, vec![]),
                ],
            ),
        ]
        .into_iter()
        .collect();

        /// What the flow emitted, in order.
        ///
        /// ⛔⛔ **A SYSTEM WITH A `MessageReader`, NOT A CURSOR TAKEN PER TICK,
        /// AND THE DIFFERENCE IS A WRONG ANSWER THAT LOOKS RIGHT.** The first
        /// version of this called `Messages::get_cursor()` inside the tick loop.
        /// A fresh cursor starts at the OLDEST buffered message and bevy holds
        /// messages for two frames, so every emission was counted TWICE and both
        /// flows reported `[grab, grab]`. A `MessageReader` in a system keeps its
        /// cursor in system-local state and reads each message once.
        ///
        /// ⚠ THE EMPTY ROADS COULD NOT HAVE CAUGHT IT — zero doubled is zero — so
        /// the roads that emit are the only arms with any power over this class
        /// of instrument bug. That is an argument for pinning the exact trace
        /// rather than asserting "the grab happened".
        #[derive(bevy::prelude::Resource, Default)]
        struct Seen(Vec<String>);

        fn capture(
            mut reader: bevy::prelude::MessageReader<MoveEventMessage>,
            mut seen: bevy::prelude::ResMut<Seen>,
        ) {
            for ev in reader.read() {
                if let MoveEventKind::Effect(effect) = &ev.kind {
                    seen.0.push(effect.key.clone());
                }
            }
        }

        /// One flow, driven through the real interpreter.
        fn trace(spec: &ambition_entity_catalog::MoveSpec, road: Road) -> Vec<String> {
            let mut app = App::new();
            app.add_message::<MoveEventMessage>();
            app.add_message::<ambition_combat::events::HitEvent>();
            app.add_message::<ambition_vfx::VfxMessage>();
            app.add_message::<ambition_sfx::OwnedSfxMessage>();
            app.add_message::<ambition_vfx::vfx::DebrisBurstMessage>();
            app.init_resource::<ambition_time::WorldTime>();
            app.insert_resource(
                ambition_characters::actor::character_catalog::CharacterCatalog::empty(),
            );
            app.init_resource::<ambition_combat::authored_volumes::AuthoredAttackVolumeResolver>();
            {
                let mut time = app.world_mut().resource_mut::<ambition_time::WorldTime>();
                time.scaled_dt = 1.0 / 60.0;
                time.raw_dt = 1.0 / 60.0;
            }
            app.init_resource::<Seen>();
            app.add_systems(Update, (advance_move_playback, capture).chain());

            let mut pb = MovePlayback::new(spec.clone(), 1.0);
            // The contact fact the flow branches on, stated up front — the same
            // fields the strike seam and the damage road write in production.
            pb.landed_hit = road.landed;
            pb.connected_hit = road.connected;
            pb.blocked_hit = road.blocked;
            let body = app
                .world_mut()
                .spawn((
                    // `advance_move_playback` narrows to combat bodies: with no
                    // faction the query does not match and the flow never runs.
                    ambition_combat::components::ActorFaction::Player,
                    ambition_platformer2d_core::BodyKinematics::default(),
                    ambition_platformer2d_core::CenteredAabb::from_center_size(
                        ambition_platformer2d_core::Vec2::ZERO,
                        ambition_platformer2d_core::Vec2::new(20.0, 40.0),
                    ),
                    pb,
                ))
                .id();
            let _ = body;

            // Long enough for every authored timeout here (0.30s) to expire and
            // for the move to play out.
            for _ in 0..120 {
                app.update();
            }
            app.world().resource::<Seen>().0.clone()
        }

        let mut found: std::collections::BTreeSet<String> = Default::default();
        let mut wrong: Vec<String> = Vec::new();
        for (_, contract) in tables() {
            for mv in &contract.moves {
                if mv.flow.is_none() {
                    continue;
                }
                found.insert(mv.id.clone());
                let Some(roads) = expected.get(mv.id.as_str()) else {
                    continue;
                };
                for (road, want) in roads {
                    let got = trace(mv, *road);
                    if got != *want {
                        wrong.push(format!(
                            "{}: when the strike {}, expected {want:?} and it ran {got:?}",
                            mv.id, road.what
                        ));
                    }
                }
            }
        }

        assert!(
            wrong.is_empty(),
            "a shipped flow no longer runs the trace it was authored for:\n  {}",
            wrong.join("\n  ")
        );

        // ⛔ THE RATCHET, both directions. A flow this crate authors that nobody
        // wrote an expectation for is not covered, and an expectation naming a
        // move that no longer authors a flow is a stale line pretending to guard
        // something.
        let want: std::collections::BTreeSet<String> =
            expected.keys().map(|k| (*k).to_string()).collect();
        assert_eq!(
            found, want,
            "the set of shipped flows and the set this test pins have drifted \
             apart — a new authored flow needs its trace written down here, and a \
             removed one needs its line taken out"
        );
        // ⛔ ANTI-VACUITY: an empty roster would satisfy both assertions above.
        assert!(
            found.len() >= 2,
            "this crate authors fewer than two flows, so the walk found almost \
             nothing and the guard is reporting on an empty population"
        );
    }

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
        fn item_ids_in(params: &ambition_entity_catalog::ParamValue) -> Vec<String> {
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

        // ⛔⛔ **IT WALKED TWO OF THE FOUR SITES A MOVE CAN NAME A TECHNIQUE
        // FROM.** This reached into `windows[..].sustain_effect` and
        // `events[..].kind` by hand and never looked at a volume's `on_hit` or a
        // flow node's `Emit` — so a held item created from either was invisible
        // here and shipped the placeholder quad, which is precisely the failure
        // this test exists to catch. `MoveSpec::effect_refs` is exhaustive by
        // destructure, so a fifth site is a compile error at the walk rather than
        // a silent gap in every consumer that hand-listed four.
        let mut wanted: std::collections::BTreeSet<String> = Default::default();
        for (_, contract) in tables() {
            for mv in &contract.moves {
                for (_site, effect) in mv.effect_refs() {
                    wanted.extend(item_ids_in(&effect.params));
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
        use ambition_entity_catalog::smash_portal::{PortalPairParams, PORTAL_PAIR};

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
                    let ambition_entity_catalog::MoveEventKind::Effect(effect) =
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
        use ambition_entity_catalog::{
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

    /// ⭐⭐ EVERY AUTHORED FLOW IN EVERY SHIPPED ROSTER VALIDATES — the POPULATION,
    /// not the two moves that happen to have one today.
    ///
    /// ⛔ THIS DOC WAS SITTING ON `every_held_item_a_move_creates_has_art`,
    /// twelve tests above its own function, where it read as a second paragraph
    /// of that test's rationale. A doc block detached from what it describes is
    /// the shape a deleted test leaves behind, and it is why the next reader
    /// cannot tell which guard makes which claim.
    ///
    /// ⛔ `TechniqueFlow::problems()` exists because each of its failures is
    /// SILENT at runtime: a transition past the end of the list, a flow with no
    /// reachable `Finish`, a `Wait` that can never time out, a cycle, a stranded
    /// node. Each produces a move that plays and does PART of what it says —
    /// which reads to whoever is holding the controller as a move that "doesn't
    /// work sometimes" rather than as bad data.
    ///
    /// ⚠ THE PER-FIGHTER TESTS ARE NOT THIS TEST. The oni's and the goblin's each
    /// validate their own flow, so a flow authored on a THIRD fighter tomorrow is
    /// covered by neither. ⇒ This asks the question of the whole crate, which is
    /// the only shape that stays true as the roster grows.
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

    /// Why this move does something a strike cannot — empty when it is a strike.
    ///
    /// ⛔⛔ ONE DEFINITION FOR TWO CENSUSES, AND EXTRACTING IT IS A REPAIR. This
    /// file holds two: `the_roster_does_not_get_less_expressive` counts FIGHTERS
    /// and ratchets; `the_census_of_specials_that_carry_no_technique` lists
    /// SPECIALS and reads. They were written with two different ideas of
    /// "expressive", and the second repeated the mistake the first had already
    /// fixed in a comment a few lines below — it did not count VOLUME REACTIONS,
    /// so it reported `officer_disperse`, "the first authored windbox on the
    /// roster", as carrying no authoring at all. ⇒ I read my own output and went
    /// looking for a way to improve that move, which is the second time this
    /// file records somebody about to author a mechanic for a fighter that
    /// already had one, on the strength of a guard's definition.
    ///
    /// ⭐ The GRAINS stay different on purpose — fighters against specials, a
    /// floor against a list. What cannot differ is what counts.
    fn expressive_reasons(
        mv: &ambition_entity_catalog::MoveSpec,
    ) -> Vec<&'static str> {
        let mut why = Vec::new();
        if mv.flow.is_some() {
            why.push("flow");
        }
        if mv.windows.iter().any(|w| w.sustain_effect.is_some()) {
            why.push("stance");
        }
        // A window with TWO volumes is a sweetspot: `StrikeRank` is the move's
        // own reading order and the strike seam takes the first that reaches, so
        // authoring a second volume is authoring where the move is strong.
        // ⛔ TWO DAMAGING VOLUMES, not two volumes. `wake` appends a windbox that
        // does no damage, so a plain `len() > 1` reported the goblin's dirt kick
        // as a "sweetspot" — which it is not, and the printed reason is what a
        // reader picks their next move from.
        if mv
            .windows
            .iter()
            .any(|w| w.volumes.iter().filter(|v| v.damage > 0).count() > 1)
        {
            why.push("sweetspot");
        }
        // `VolumeReaction::{Autolink, Windbox}` change what a hit DOES — a
        // gather, a shove — which is authoring a plain strike cannot express.
        if mv
            .windows
            .iter()
            .any(|w| w.volumes.iter().any(|v| v.reaction.is_some()))
        {
            why.push("volume reaction");
        }
        if mv
            .events
            .iter()
            .any(|e| {
                matches!(
                    e.kind,
                    ambition_entity_catalog::MoveEventKind::Effect(_)
                )
            })
        {
            why.push("technique");
        }
        if mv
            .events
            .iter()
            .any(|e| {
                matches!(
                    e.kind,
                    ambition_entity_catalog::MoveEventKind::GravityModifier { .. }
                )
            })
        {
            why.push("gravity regime");
        }
        // ⛔⛔ THE OTHER TWO MECHANICAL EVENT KINDS, AND LEAVING THEM OUT IS HOW
        // THIS DEFINITION UNDERCOUNTED. `MoveEventKind` has exactly six variants:
        // `Sfx` and `Vfx` are cosmetic, and `Effect`, `GravityModifier`, `Ranged`
        // and `Impulse` are mechanics. Counting the first two of the mechanics
        // and calling the rest "events" put Alice's `key_exchange` — a 640 px/s
        // committed lunge, authored as `MoveEventKind::Impulse` — in the same
        // bucket as a move whose whole authoring is a sound.
        // ⇒ A census over a closed enum should name every variant, so that adding
        // a seventh is a compile error here rather than a silent miscount.
        if mv.events.iter().any(|e| {
            matches!(
                e.kind,
                ambition_entity_catalog::MoveEventKind::Impulse { .. }
            )
        }) {
            why.push("impulse");
        }
        if mv.events.iter().any(|e| {
            matches!(
                e.kind,
                ambition_entity_catalog::MoveEventKind::Ranged
            )
        }) {
            why.push("ranged");
        }
        // ⛔⛔ AND THE SAME DISCIPLINE APPLIED TO THE OTHER TWO CLOSED SETS, so
        // that this definition is complete BY CONSTRUCTION rather than by
        // whatever the last author happened to need. Enumerating `MoveEventKind`
        // is what moved the answer from 48 to 13; enumerating `WindowTag`,
        // `HitVolume` and `MoveSpec`'s own fields is the rest of that job, and it
        // was done in one pass on 2026-09-06 after the third consecutive move I
        // sat down to author turned out to want a mechanic the census could not
        // see (set knockback, then armour, then a cancel).
        //
        // ⚠ THE COUNT THIS MOVES IS AN INSTRUMENT DELTA, NOT AUTHORING. Reported
        // separately for that reason: a definition that grows and a roster that
        // improves are two different facts and they must never be added together.
        // ⛔ MATCHED EXHAUSTIVELY IN A CLOSURE rather than pushed from a loop, so
        // a move with three invulnerable windows says "invuln" once — and so that
        // a seventh `WindowTag` is a COMPILE error here rather than a silent
        // omission, which is the whole lesson of the `MoveEventKind` miscount.
        let tagged = |wanted: fn(&ambition_entity_catalog::WindowTag) -> bool| {
            mv.windows.iter().any(|w| wanted(&w.tag))
        };
        // Invincibility frames: a move you can throw THROUGH something.
        if tagged(|t| {
            matches!(t, ambition_entity_catalog::WindowTag::Invuln)
        }) {
            why.push("invuln");
        }
        // Super armour: you get hit and you swing anyway.
        if tagged(|t| matches!(t, ambition_entity_catalog::WindowTag::Armor)) {
            why.push("armor");
        }
        // A cancel window is authored follow-up: what this move is ALLOWED to
        // become, which a plain strike cannot say.
        if tagged(|t| {
            matches!(
                t,
                ambition_entity_catalog::WindowTag::Cancelable { .. }
            )
        }) {
            why.push("cancelable");
        }
        // A technique the volume fires ON CONTACT — the conditional sibling of an
        // `Effect` event, and it was uncounted because it hangs off the VOLUME
        // rather than off the timeline.
        if mv
            .windows
            .iter()
            .any(|w| w.volumes.iter().any(|v| v.on_hit.is_some()))
        {
            why.push("on-hit technique");
        }
        // ⭐ SET KNOCKBACK. `knockback_growth: Some(0.0)` means the launch is the
        // same at 0% and at 150%, which is the genre's combo-starter and its
        // set-up-a-kill tool. `fixed_knockback` is the verb that authors it.
        if mv.windows.iter().any(|w| {
            w.volumes
                .iter()
                .any(|v| v.knockback_growth == Some(0.0) && v.damage > 0)
        }) {
            why.push("set knockback");
        }
        // A move that LOOPS is a held stance rather than a swing.
        if mv.repeat.is_some() {
            why.push("loop");
        }
        // ⛔⛔ THESE THREE LIVED ONLY IN THE PER-SPECIAL CENSUS BELOW, as its own
        // private `extras` list, and that is how this file came to state one
        // definition of "expressive" and compute another. A charge, a launch
        // impulse and a second hit window are mechanics by anyone's reading;
        // they belong to the SHARED definition, not to one caller's idea of it.
        // Moved here 2026-09-06 so there is exactly one of this number in the
        // file — the same failure a peer found the same day in a planning table
        // that disagreed with the prose six lines above it.
        if mv.smash_charge.is_some() {
            why.push("charge");
        }
        if mv.start_impulse.is_some() {
            why.push("impulse");
        }
        // Two windows that both strike is a move with a rhythm — a jab-jab, a
        // hit that sets up its own second hit. One window is a swing.
        if mv.windows.iter().filter(|w| !w.volumes.is_empty()).count() > 1 {
            why.push("hit windows");
        }
        why
    }

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
                special_ids.iter().any(|id| **id == mv.id)
                    && !expressive_reasons(mv).is_empty()
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

    /// ⭐⭐ THE SAME QUESTION IN THE UNIT JON ASKED IT IN — the SPECIAL, not the
    /// fighter.
    ///
    /// ⛔⛔ THE RATCHET ABOVE SATURATED AND WENT BLIND, and it was being reported
    /// as this campaign's progress while it did. It passes when every fighter has
    /// AT LEAST ONE interesting special, and the roster reached that on
    /// 2026-09-05: 19 of 19, floor 18. ⇒ From that day on, a fighter carrying one
    /// technique and three bare swings scored exactly the same as one whose whole
    /// kit reads, and no authoring could move the number in either direction.
    /// A ratchet at its ceiling is not a ratchet.
    ///
    /// ⇒ Jon's words were *"a lot of characters have boring specials"*. The
    /// SPECIAL is the unit of that sentence, and counting it changes the answer
    /// from "0 plain" to this: measured 2026-09-06, **75 of 88 specials carry a
    /// mechanic and 4 do not** (it opened at 75/13 and nine specials have been
    /// authored since). Both numbers come from `expressive_reasons`, the
    /// one definition in this file, so this test and the per-special census
    /// cannot drift apart the way the census and its own doc comment did.
    ///
    /// ⚠⚠ AND THE ONE THING THIS CANNOT CATCH, SAID PLAINLY BECAUSE IT IS THE
    /// EXACT DEFECT THAT PRODUCED THE WRONG HEADLINE: a ratchet holds a FLOOR, so
    /// it sees the number fall and never sees it INFLATE. Padding
    /// `expressive_reasons` with something cosmetic — which is what `extras` did
    /// to the plain verdict — would raise this count and pass. ⇒ The defence is
    /// not here. It is that there is now exactly ONE definition, sited next to
    /// its own bold sentence about cues, so inflating the number means editing
    /// the paragraph that forbids it.
    ///
    /// ⛔⛔ AND THE FIRST NUMBER THIS TEST SHIPPED WITH WAS 40, WHICH WAS WRONG
    /// IN THE SAME WAY AS THE THING IT WAS WRITTEN TO FIX. It read "48 specials
    /// carry no mechanic" because the definition counted two of `MoveEventKind`'s
    /// four mechanical variants and lumped `Impulse` and `Ranged` in with the
    /// cues — so Alice's `key_exchange`, a 640 px/s committed lunge, was counted
    /// as a move whose whole authoring is a sound. ⇒ A census over a closed enum
    /// must name every variant. Naming all six moved the answer from 48 to 13,
    /// and the correction was found by reading one authored move against the
    /// number rather than by re-reading the census.
    ///
    /// ⭐ POISONED 2026-09-06, one reason at a time, and the counts are the
    /// evidence that each is load-bearing rather than decorative:
    /// technique → 61, the event-kind impulse → 43, both impulse roads → 42,
    /// ranged → 74, charge → 74, and pointing the corpus at a verb prefix
    /// nothing uses → 0. All RED. ⚠ Two attempts had to be re-run before they
    /// meant anything: one pushed an empty string (a non-empty vec of nothing,
    /// still GREEN) and one matched a string that appears twice and edited
    /// neither, printing GREEN for a file it never touched.
    #[test]
    fn the_specials_do_not_get_less_expressive() {
        /// Raised deliberately as specials gain mechanics. ⛔ Raising it is a
        /// decision with a commit behind it; watching it fall silently is the
        /// failure this exists to catch.
        const FLOOR: usize = 84;

        let mut rich: Vec<String> = Vec::new();
        let mut bare: Vec<String> = Vec::new();
        for (fighter, contract) in tables() {
            let special_ids: Vec<&String> = contract
                .verbs
                .iter()
                .filter(|(verb, _)| verb.starts_with("special"))
                .map(|(_, id)| id)
                .collect();
            for mv in &contract.moves {
                if !special_ids.iter().any(|id| **id == mv.id) {
                    continue;
                }
                let named = format!("{fighter}/{}", mv.id);
                if expressive_reasons(mv).is_empty() {
                    bare.push(named);
                } else {
                    rich.push(named);
                }
            }
        }

        // ⭐ NAMED, like its sibling, because "which special next" is the
        // question anybody running this actually has — and unlike its sibling
        // this list is long enough to choose from.
        println!(
            "[expressiveness/special] {} with a mechanic, {} without\n  without: {bare:#?}",
            rich.len(),
            bare.len(),
        );
        assert!(
            rich.len() >= FLOOR,
            "specials carrying a mechanic fell to {} (floor {FLOOR}).\n  \
             without a mechanic: {bare:?}",
            rich.len(),
        );

        /// ⭐⭐ AND A CEILING ON PLAINNESS, WHICH IS THE HALF A FLOOR CANNOT HOLD.
        ///
        /// A floor catches a mechanic being REMOVED. It cannot catch a plain
        /// special being ADDED: a new bare swing raises the roster's total and
        /// leaves `rich` exactly where it was, so `rich >= FLOOR` stays green
        /// while the thing Jon complained about gets worse. ⇒ Both directions or
        /// neither.
        ///
        /// ⛔ FOUR, AND ALL FOUR ARE DELIBERATE — a count, never an allowlist of
        /// names, because a name list absorbs the guard's own weaknesses the
        /// moment somebody adds a row to quiet it. Each of the four states its
        /// reason in its own file: the goblin's `scrap_flail` (*"no technique at
        /// all: it turns its whole body into the swing and hopes"*),
        /// `cellular_pulse` (a migration promise not to retune it),
        /// `performer_the_line` (an explicit refusal of `MoveEventKind::Ranged`)
        /// and `performer_trapdoor_air` (a deliberate FAKE — the same first
        /// frames as the real trapdoor and no door, explicitly no i-frames, whose
        /// whole design is that it does nothing).
        ///
        /// ⇒ Raising this is a design decision with a commit behind it, exactly
        /// like lowering the floor. A plain-strike brawler is legitimate; a
        /// plain-strike brawler nobody decided on is the complaint.
        ///
        /// ⭐ POISONED 2026-09-06 with an ADDITION rather than a removal, because
        /// that is the case the floor cannot see: widening the corpus to every
        /// verb starting with "s" took plainness to 57 and reddened THIS while
        ///  stayed green throughout.
        const PLAIN_CEILING: usize = 4;

        assert!(
            bare.len() <= PLAIN_CEILING,
            "specials with no mechanic rose to {} (ceiling {PLAIN_CEILING}).\n  \
             the four expected are deliberate and say so in their own files; \
             anything else here is a bare swing nobody decided on:\n  {bare:#?}",
            bare.len(),
        );
    }

    // ⛔ THE PER-SPECIAL CENSUS LIVES HERE, in the module that already owned
    // this question, and it did not when it was written: it was a second
    // `mod tests` at the foot of the same file with its own idea of what
    // counts as authoring. Two modules, two definitions, one subject — which
    // is how it came to report the roster's first authored windbox as plain.
    use ambition_entity_catalog::MoveEventKind;

    /// Does this move carry a TECHNIQUE, or is it a hitbox and nothing else?
    ///
    /// ⛔⛔ **EVERY ROAD COUNTS, AND THIS CENSUS HAS ALREADY MISSED ONE ONCE.**
    /// Its previous version read `events[..].kind` only, and reported the tether
    /// grab — a move whose whole point is its capture — as featureless, because
    /// a technique live for a WINDOW hangs off `sustain_effect`. The fix added
    /// the second road by hand and left two more unread: a volume's `on_hit` and
    /// a flow node's `Emit`. A move whose only technique sits in either would be
    /// counted bare today, which is the identical failure one iteration later.
    ///
    /// ⇒ `MoveSpec::effect_refs` is exhaustive by destructure, so a fifth site is
    /// a compile error at the walk instead of a silent gap in every hand-listed
    /// copy of it.
    fn techniques(spec: &ambition_entity_catalog::MoveSpec) -> Vec<String> {
        let mut keys: Vec<String> = spec
            .effect_refs()
            .into_iter()
            .map(|(_site, effect)| effect.key.clone())
            .collect();
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
                // ⛔⛔ CUES ONLY, AND THEY DO NOT COUNT TOWARDS THE VERDICT.
                // This list used to also carry charge / impulse / hit windows and
                // was consulted by the plain test below, which meant a special
                // whose entire authoring was a SOUND counted as not-plain — while
                // `expressive_reasons`, in this same file, says in bold that "a
                // strike with cues is not". The mechanics moved to the shared
                // definition; what is left here is colour for the printout, which
                // is worth keeping because "it at least has a cue" is a genuinely
                // different state from "nothing at all" when you are deciding
                // which special to work on next.
                let mut extras: Vec<String> = Vec::new();
                let cues = spec
                    .events
                    .iter()
                    .filter(|e| {
                        matches!(
                            e.kind,
                            MoveEventKind::Sfx { .. } | MoveEventKind::Vfx { .. }
                        )
                    })
                    .count();
                if cues > 0 {
                    extras.push(format!("{cues} cue(s)"));
                }
                // ⛔ THE PLAIN VERDICT USES THE SHARED DEFINITION, not this
                // test's own idea of authoring. `expressive_reasons` counts a
                // VOLUME REACTION, which this census did not — and so it called
                // the roster's first authored windbox plain.
                let reasons = expressive_reasons(spec);
                if keys.is_empty() {
                    bare_total += 1;
                    // ⛔ THE SHARED DEFINITION AND NOTHING ELSE. `extras` is a
                    // cue count and a cue is not a mechanic.
                    if reasons.is_empty() {
                        plain_total += 1;
                    }
                }
                println!(
                    "{:<22} {:<34} {}",
                    name,
                    id,
                    if !keys.is_empty() {
                        keys.join(", ")
                    } else if !reasons.is_empty() {
                        format!("— no technique, but {}", reasons.join(" + "))
                    } else if extras.is_empty() {
                        "— PLAIN (no technique, no other authoring)".to_string()
                    } else {
                        // Reached only when `reasons` is EMPTY, so this move is
                        // plain and the cue count says how it is plain.
                        format!("— PLAIN (no mechanic; {} only)", extras.join(" + "))
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

#[cfg(test)]
mod stance_coupling {
    use super::tables;
    use ambition_entity_catalog::AttackDir;

    /// ⛔⛔ **THE ONE WIRE BETWEEN THE ATTACK KIT AND MOVEMENT SCORING IS INERT,
    /// AND THAT IS WHY IT CANNOT EXPLAIN A MOVEMENT CHANGE.**
    ///
    /// `generate_options` scores movement with
    /// `movement_options(&view, situation, !lifts.is_empty())` — a single boolean
    /// derived from the ATTACK KIT by `lifting_candidates`. It is the only path
    /// by which what a body can hit with reaches how it decides to move, so it is
    /// the first thing to suspect when a kit change moves a body differently.
    ///
    /// ⇒ MEASURED across the roster: for every shipped fighter, that boolean is
    /// the SAME whether the kit is resolved standing or running. The wire exists
    /// and never fires. ⭐ So a kit change cannot alter movement scoring through
    /// it, and an investigation that stops at "the scorers are coupled" has
    /// stopped at something true and inert.
    ///
    /// ⚠ THIS IS NOT AN ARGUMENT THAT THE COUPLING IS HARMLESS. It is an
    /// argument that it is not firing TODAY, on THIS roster — which is exactly
    /// the kind of fact that rots. A fighter whose only lifting move is a smash
    /// or a tilt would make it fire the moment a run pre-empts that press, and
    /// nothing else in the tree would notice. That is what this guard is for.
    #[test]
    fn no_shipped_fighter_changes_its_lift_availability_with_stance() {
        use ambition_platformer2d::entity_catalog as cat;
        let dirs = [
            AttackDir::Neutral,
            AttackDir::Forward,
            AttackDir::Back,
            AttackDir::Up,
            AttackDir::Down,
        ];

        // The kit the brain would hold in one stance, resolved the way the PRESS
        // ROAD resolves it: a run pre-empts the smash gesture and forces the base
        // to ATTACK, and a special never takes that road at all.
        let kit_lifts = |set: &ambition_entity_catalog::MovesetContract,
                         running: bool|
         -> Vec<String> {
            let mut seen: Vec<String> = Vec::new();
            let mut lifts: Vec<String> = Vec::new();
            for (verb_name, basic_or_smash) in [
                (cat::ATTACK_VERB, true),
                (cat::SMASH_VERB, true),
                (cat::SPECIAL_VERB, false),
            ] {
                for d in dirs {
                    let running_now = running && basic_or_smash;
                    let resolve = if running_now { cat::ATTACK_VERB } else { verb_name };
                    let Some(spec) = set.move_for_attack(resolve, d, true, running_now) else {
                        continue;
                    };
                    if seen.contains(&spec.id) {
                        continue;
                    }
                    seen.push(spec.id.clone());
                    if spec.frame_data().recovery_route.offers_a_way_home() {
                        lifts.push(spec.id.clone());
                    }
                }
            }
            lifts.sort();
            lifts
        };

        let mut fires: Vec<String> = Vec::new();
        let mut walked = 0usize;
        let mut with_lifts = 0usize;
        for (owner, set) in tables() {
            walked += 1;
            let standing = kit_lifts(&set, false);
            let running = kit_lifts(&set, true);
            if !standing.is_empty() {
                with_lifts += 1;
            }
            if standing.is_empty() != running.is_empty() {
                fires.push(format!(
                    "{owner}: standing lifts {standing:?}, running lifts {running:?}"
                ));
            }
        }

        assert!(
            fires.is_empty(),
            "a fighter's LIFT AVAILABILITY now depends on its stance, so the attack \
             kit reaches movement scoring through `!lifts.is_empty()` and a kit \
             change can move the body. Re-derive anything that concluded the \
             coupling was inert:\n  {}",
            fires.join("\n  ")
        );

        // ⛔ ANTI-VACUITY, both halves. A roster nobody walked, and a roster where
        // NOBODY lifts, each satisfy the emptiness above forever — and the second
        // is the one that would creep in, because "no lifts either way" is not the
        // same finding as "the same lifts either way".
        assert!(
            walked >= 15,
            "only {walked} fighters walked; this guard is reporting on almost nothing"
        );
        assert!(
            with_lifts >= 5,
            "only {with_lifts} of {walked} fighters have ANY lifting move standing, \
             so `!lifts.is_empty()` is false almost everywhere and agreeing across \
             stances says nothing"
        );
    }
}

#[cfg(test)]
mod a12_projectile_credit_census {
    use super::tables;

    /// ⭐⭐ CAN A PROJECTILE'S VERDICT EVER REACH A CONNECT CONDITION?
    ///
    /// A12 blocker 4: a damage verdict carrying `attacker_move_instance: None`
    /// is credited to whatever move the attacker is playing NOW
    /// (`moveset::verdict_belongs_to`). A projectile's verdict carries `None` —
    /// three sites in `projectile/systems.rs` hardcode it — so move A can fire a
    /// shot, end, and move B can be credited with `connected_hit` it never
    /// earned, taking an OnHit escape it did not pay for.
    ///
    /// ⛔ THE PREDICATE'S OWN COMMENT DEFENDS ADMITTING `None`, and its reasoning
    /// is sound as far as it goes: *"contact attrition, a hazard, the blast zone
    /// and an ability's own volume all resolve through it and no move claims
    /// them."* It never addresses the projectile, which is the case at issue.
    ///
    /// ⇒ THE FIX HAS TWO HALVES — propagate the instance to the shot, then
    /// require a claim — and the second is much cheaper than the first (the
    /// message gains a field across 43 construction sites). **This census exists
    /// to find out whether the first half is needed at all**, by asking the
    /// decidable version of the question: does any fighter author BOTH a move
    /// that fires a shot AND a move that takes a conditional cancel? If no
    /// fighter does, no projectile verdict can reach a connect condition, and
    /// the cheap half is complete on its own.
    ///
    /// ⚠ THE AUTHORED SET IS A SUPERSET OF THE ADMITTED ONE, which is what makes
    /// this instrument sound for a NEGATIVE result: a pairing absent from
    /// everything authored is absent from everything seated. A POSITIVE result
    /// would have to be re-asked of the composed app, because a character can be
    /// withheld at the admission barrier.
    #[test]
    fn a_shot_and_a_conditional_cancel_never_share_a_fighter() {
        use ambition_entity_catalog::{CancelCondition, MoveEventKind, WindowTag};

        let mut shooters = 0usize;
        let mut confirmers = 0usize;
        let mut both: Vec<String> = Vec::new();

        for (fighter, contract) in tables() {
            let mut shots: Vec<&str> = Vec::new();
            let mut confirms: Vec<&str> = Vec::new();
            for m in &contract.moves {
                if m.events
                    .iter()
                    .any(|ev| matches!(ev.kind, MoveEventKind::Ranged))
                {
                    shots.push(m.id.as_str());
                }
                if m.windows.iter().any(|w| {
                    matches!(
                        w.tag,
                        WindowTag::Cancelable {
                            condition: CancelCondition::OnHit
                                | CancelCondition::OnBlock
                                | CancelCondition::OnWhiff,
                            ..
                        }
                    )
                }) {
                    confirms.push(m.id.as_str());
                }
            }
            if !shots.is_empty() {
                shooters += 1;
            }
            if !confirms.is_empty() {
                confirmers += 1;
            }
            if !shots.is_empty() && !confirms.is_empty() {
                both.push(format!("{fighter}: shots={shots:?} confirms={confirms:?}"));
            }
        }

        // ⚠ ANTI-VACUITY, AND BOTH HALVES ARE LOAD-BEARING. A roster with no
        // shooters, or none with a conditional cancel, would report "no overlap"
        // while measuring nothing — and a guard that requires a confirm to exist
        // already lives in this file, so zero here means this walk is broken
        // rather than that the roster changed.
        assert!(
            shooters >= 1,
            "no authored move fires a shot, so this census walked nothing"
        );
        assert!(
            confirmers >= 1,
            "no authored move takes a conditional cancel, so this census cannot \
             see the pairing it exists to look for"
        );

        println!(
            "[a12] fighters with a shot: {shooters}; with a conditional cancel: \
             {confirmers}; with BOTH: {}",
            both.len()
        );
        for row in &both {
            println!("[a12] {row}");
        }
    }
}
