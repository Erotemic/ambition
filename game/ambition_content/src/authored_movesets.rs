//! Every moveset this crate authors, in one list.
//!
//! This is not the selectable cast. Some fighters (Mary-O, Sanic) keep their
//! tables in their own crates, and this crate cannot see them. A hand-kept
//! list can also fall behind silently when a fighter is added.
//!
//! The cast's authority is `SmashRoster::assemble` against a live
//! `PreparedCharacterRegistry`, then each prepared character's
//! `kit.projectable_moveset()`. That needs an app;
//! `a_recovery_mount_cannot_be_deleted_by_one_hit` uses it.
//!
//! Use this list for questions about moves this crate authors (for example
//! `moveset_sound`).

use ambition_entity_catalog::MovesetContract;

/// The move table the game ships for `character`, read from the content pack.
///
/// A fighter's tests read this, so they guard the file the game plays and not
/// a copy of it.
pub fn shipped(character: &str) -> MovesetContract {
    ambition_characters::moveset_content_schema::lowered_movesets(crate::pack::prepared())
        .and_then(|table| table.get(character))
        .cloned()
        .unwrap_or_else(|| panic!("the shipped pack carries no move table for `{character}`"))
}

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
        ("director", crate::director_moveset::director_moveset()),
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
    /// A fifth of the shipped smash platform's 480px width, the same as the
    /// smash demo's ceiling. It is stated against the stage, not the body.
    const ORDINARY_REACH_PX: f32 = 96.0;

    /// What a declared TETHER may reach instead.
    ///
    /// A third of the platform: a tether may surprise, but must not cover the
    /// stage.
    const TETHER_REACH_PX: f32 = 160.0;

    /// The grabs allowed past [`ORDINARY_REACH_PX`], and why.
    ///
    /// An allowlist, not a raised ceiling. Raising the ceiling to 160 would let
    /// every fighter grow a tether without review. Naming each exception makes it
    /// a reviewed fact.
    ///
    /// Name only the authored grab. `author_standing_grab` derives the running
    /// variant by cloning the standing grab's windows, so a tether standing grab
    /// is also a tether dash grab. Listing `…_grab_dash` separately would let the
    /// two drift.
    const TETHERS: &[&str] = &[
        // The ranged fighter. Its identity is built around distance, so its grab is
        // a tether.
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
    /// moveset this crate authors.
    ///
    /// The smash demo's
    /// `no_grab_this_demo_authors_reaches_further_than_the_stage_allows` covers
    /// only the demo's own movesets, because `ambition_demo_smash` does not
    /// depend on `ambition_content`. This test covers the movesets in
    /// `tables()`.
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
        // Population floor. This crate authors several standing grabs; finding none
        // means the capture key, the window shape or `tables()` changed.
        assert!(
            seen >= 3,
            "only {seen} authored capture attempt(s) were found across \
             {} movesets, so this guard is measuring nothing rather than passing",
            super::tables().len(),
        );
        // The allowlist must be live: an entry naming a removed move is an invisible
        // permission. Each tether contributes two moves (itself and the derived
        // running variant), so a lower count also catches a lost derivation.
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

    /// Every shipped flow still runs the trace it was authored for (A12).
    ///
    /// `ambition_combat`'s flow tests use hand-built graphs, and `problems()` is
    /// structural only (dangling edges, unreachable `Finish`). This runs each
    /// shipped flow through the interpreter.
    ///
    /// Every road of every flow is checked, because these are all `Wait`-shaped.
    /// The timeout road (the whiff) must emit nothing: an `on_timeout` pointed at
    /// the `Emit` instead of `Finish` would give a free grab on a miss.
    ///
    /// The expected table is exact and must cover the discovered set, so a new
    /// authored flow fails this test until its trace is written down.
    #[test]
    fn every_shipped_flow_still_runs_the_trace_it_was_authored_for() {
        use ambition_combat::moveset::{advance_move_playback, MoveEventMessage, MovePlayback};
        use ambition_entity_catalog::MoveEventKind;
        use bevy::prelude::*;

        // A road is a contact state, not a boolean. The oni's flow branches on
        // `Blocked` after waiting on `Overlapped`, so it has three roads.
        // `MoveContact::overlapped` is `landed || connected || blocked`, so a blocked
        // road needs no separate landed flag.
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
                // The goblin's tackle: wait on the connect, grab, finish. A landed charge
                // grabs; a whiff must stay empty-handed.
                "headlong_charge",
                vec![
                    (CONNECTED, vec![grab]),
                    (WHIFFED, vec![]),
                    // A blocked charge must not grab. Waiting on the overlap would give the
                    // goblin a grab for running into a shield; this asserts the move's
                    // comment.
                    (BLOCKED, vec![]),
                ],
            ),
            (
                // The oni leader's iaijutsu: wait on the overlap, branch on the guard,
                // teleport behind them, finish.
                "iaijutsu",
                vec![
                    (BLOCKED, vec![teleport]),
                    // He does not escape a hit that landed: the branch's `otherwise` is
                    // `Finish`.
                    (CONNECTED, vec![]),
                    (WHIFFED, vec![]),
                ],
            ),
        ]
        .into_iter()
        .collect();

        /// What the flow emitted, in order.
        ///
        /// Use a `MessageReader` in a system, not a cursor per tick. A fresh cursor
        /// starts at the oldest buffered message, and bevy keeps messages for two
        /// frames, so each emission would be counted twice. Only the roads that emit
        /// can catch that, which is why the exact trace is pinned.
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
            // The contact fact the flow branches on, set up front in the same fields
            // the strike seam and damage road write in production.
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

        // Ratchet in both directions: a flow with no expectation is not covered,
        // and an expectation for a move with no flow is stale.
        let want: std::collections::BTreeSet<String> =
            expected.keys().map(|k| (*k).to_string()).collect();
        assert_eq!(
            found, want,
            "the set of shipped flows and the set this test pins have drifted \
             apart — a new authored flow needs its trace written down here, and a \
             removed one needs its line taken out"
        );
        // Anti-vacuity: an empty roster would satisfy both assertions above.
        assert!(
            found.len() >= 2,
            "this crate authors fewer than two flows, so the walk found almost \
             nothing and the guard is reporting on an empty population"
        );
    }

    /// Every held item a move creates has an art entry. Without one,
    /// `HeldItemArt` resolves to the placeholder quad.
    ///
    /// The scan is structural: `ParamValue` is a `ron::Value`, so this walks
    /// every authored effect's params for a field named `item_id`, whatever the
    /// technique. A new move is covered automatically.
    ///
    /// This checks registration only, not that the PNG exists. Sprites are
    /// generated and gitignored, so a file check would fail on a fresh checkout.
    /// `scripts/check_published_sheets_are_present.py` checks presence.
    #[test]
    fn every_held_item_a_move_creates_has_art() {
        use bevy::prelude::App;

        // What the roster asks for: every `item_id` any authored effect names.
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

        // Use `MoveSpec::effect_refs`, which is exhaustive by destructure. It covers
        // all sites (window sustains, events, volume `on_hit`, flow `Emit`), and a
        // new site is a compile error there.
        let mut wanted: std::collections::BTreeSet<String> = Default::default();
        for (_, contract) in tables() {
            for mv in &contract.moves {
                for (_site, effect) in mv.effect_refs() {
                    wanted.extend(item_ids_in(&effect.params));
                }
            }
        }

        // Anti-vacuity: a renamed field would make the walk find nothing.
        assert!(
            !wanted.is_empty(),
            "no authored effect names an `item_id`, so this guard is comparing \
             an empty set against the manifest"
        );

        // What the game draws.
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

    /// An authored portal rise must land inside the visible stage.
    ///
    /// Alice's up-B once opened its exit 320 px above her, past the smash
    /// ruleset's 240 px ceiling blast margin, so it was outside the playable box.
    ///
    /// The bound belongs to `ambition_demo_smash::CEILING_BLAST_MARGIN_PX`, which
    /// this crate cannot depend on, so it is repeated here. That constant's doc
    /// points at this test, but nothing forces them to agree.
    ///
    /// The rise value itself (150) is tuning and is not pinned.
    #[test]
    fn an_authored_portal_rise_stays_inside_the_stage() {
        use ambition_entity_catalog::smash_portal::{PortalPairParams, PORTAL_PAIR};

        /// `ambition_demo_smash::CEILING_BLAST_MARGIN_PX`, repeated because this
        /// crate is below the ruleset and cannot read it.
        const CEILING_BLAST_MARGIN_PX: f32 = 240.0;

        // Scan the timeline's events, not window sustains: `author_portal_pair`
        // attaches the pair as a `MoveEvent` at a time.
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

        // Anti-vacuity: no portal pair means the roster dropped the move.
        assert!(
            checked >= 1,
            "no shipped move authors a portal pair, so this guard is checking an \
             empty set"
        );
    }

    /// Every cancel target resolves, and the roster uses the conditional cancel.
    ///
    /// A `Cancelable` window's `into` list shares one namespace: literal move
    /// ids, verbs, and the classes `cancel_class_names()` derives from
    /// `cancel_names_for`. A name in none of these is dead: the window opens, the
    /// press is looked up, and nothing answers.
    ///
    /// The second half requires at least one customer of
    /// `CancelCondition::OnHit` / `OnWhiff` / `OnBlock`, so the capability is
    /// exercised.
    #[test]
    fn every_cancel_target_resolves_and_a_confirm_is_authored() {
        use ambition_entity_catalog::{
            base_verb_of, cancel_class_names, cancel_names_for, CancelCondition, WindowTag,
        };

        let mut dead: Vec<String> = Vec::new();
        let mut confirms = 0usize;
        let mut windows = 0usize;
        for (fighter, contract) in tables() {
            let ids: std::collections::BTreeSet<&str> =
                contract.moves.iter().map(|m| m.id.as_str()).collect();
            // The class names come from `cancel_class_names()`, derived from
            // `cancel_names_for`, so this guard and the catalog's validator ask one
            // question. It includes `smash`, `grab` and `taunt`.
            //
            // Do not seed this with the raw bound verbs (`contract.verbs.keys()`).
            // `trigger_moveset_moves` asks `cancel_names_for(base_verb_of(verb), ..)`,
            // so a `special_forward` press offers `["special"]` and never the
            // directional spelling.
            let mut verbs: std::collections::BTreeSet<&str> = Default::default();
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
                        let known = cancel_class_names().contains(&target.as_str())
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
        // Anti-vacuity: a roster with no cancel windows would pass forever.
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

    /// Every named refusal variant resolves to a move that exists.
    ///
    /// `MoveGates::when_refused` holds a move id, and an unknown id falls back to
    /// "no fallback" so a typo cannot crash a match. The failure is then a dead
    /// button, so it needs a guard.
    ///
    /// The variant need not own a verb. The goblin's uncharged dive is reachable
    /// only through this field, so the check is membership in `moves` (what
    /// `move_by_id` searches).
    ///
    /// Checked across the whole crate so a variant on any fighter is covered.
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
        // Anti-vacuity: this would also pass if the last authored variant were
        // deleted.
        assert!(
            named >= 1,
            "no move in any shipped roster authors `when_refused`, so this guard \
             is validating an empty set"
        );
    }

    /// Every authored flow in every shipped roster validates.
    ///
    /// `TechniqueFlow::problems()` exists because each failure is silent at
    /// runtime: a transition past the end, no reachable `Finish`, a `Wait` that
    /// never times out, a cycle, a stranded node. Each gives a move that does only
    /// part of its job.
    ///
    /// The per-fighter tests cover only their own flow; this covers the whole
    /// crate.
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
        // Anti-vacuity: a census that walks no flows passes forever.
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
    /// One definition for two censuses:
    /// `the_roster_does_not_get_less_expressive` (counts fighters, ratchets) and
    /// `the_census_of_specials_that_carry_no_technique` (lists specials). Their
    /// grains differ on purpose; what counts as expressive must not. For example,
    /// volume reactions count, so `officer_disperse` (a windbox) is expressive.
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
        // Two damaging volumes in one window is a sweetspot: `StrikeRank` is the
        // move's reading order, and the strike seam takes the first that reaches.
        // Count damaging volumes only; `wake` appends a no-damage windbox.
        if mv
            .windows
            .iter()
            .any(|w| w.volumes.iter().filter(|v| v.damage > 0).count() > 1)
        {
            why.push("sweetspot");
        }
        // `VolumeReaction::{Autolink, Windbox}` change what a hit does (a gather,
        // a shove), which a plain strike cannot express.
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
        // `MoveEventKind` has six variants: `Sfx` and `Vfx` are cosmetic;
        // `Effect`, `GravityModifier`, `Ranged` and `Impulse` are mechanics (for
        // example Alice's `key_exchange` is an `Impulse` lunge). Name every variant
        // so a seventh is a compile error here.
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
        // The same rule for the other closed sets (`WindowTag`, `HitVolume`,
        // `MoveSpec` fields): enumerate them so the definition is complete by
        // construction. A change here is an instrument change, not an authoring
        // change; report the two separately.
        //
        // Match exhaustively in a closure, so three invulnerable windows report
        // "invuln" once and a new `WindowTag` is a compile error.
        let tagged = |wanted: fn(&ambition_entity_catalog::WindowTag) -> bool| {
            mv.windows.iter().any(|w| wanted(&w.tag))
        };
        // Invincibility frames: a move you can throw through something.
        if tagged(|t| {
            matches!(t, ambition_entity_catalog::WindowTag::Invuln)
        }) {
            why.push("invuln");
        }
        // Super armour: you get hit and you swing anyway.
        if tagged(|t| matches!(t, ambition_entity_catalog::WindowTag::Armor)) {
            why.push("armor");
        }
        // A cancel window is authored follow-up: what this move may become.
        if tagged(|t| {
            matches!(
                t,
                ambition_entity_catalog::WindowTag::Cancelable { .. }
            )
        }) {
            why.push("cancelable");
        }
        // A technique the volume fires on contact: the conditional sibling of an
        // `Effect` event, attached to the volume, not the timeline.
        if mv
            .windows
            .iter()
            .any(|w| w.volumes.iter().any(|v| v.on_hit.is_some()))
        {
            why.push("on-hit technique");
        }
        // Set knockback: `knockback_growth: Some(0.0)` gives the same launch at 0%
        // and 150% (combo starter, kill setup). Authored by `fixed_knockback`.
        if mv.windows.iter().any(|w| {
            w.volumes
                .iter()
                .any(|v| v.knockback_growth == Some(0.0) && v.damage > 0)
        }) {
            why.push("set knockback");
        }
        // A move that loops is a held stance, not a swing.
        if mv.repeat.is_some() {
            why.push("loop");
        }
        // Charge, launch impulse and a second hit window are mechanics, so they
        // belong to this shared definition.
        if mv.smash_charge.is_some() {
            why.push("charge");
        }
        if mv.start_impulse.is_some() {
            why.push("impulse");
        }
        // Two striking windows give a move a rhythm (jab-jab, a hit that sets up
        // its own second hit). One window is a swing.
        if mv.windows.iter().filter(|w| !w.volumes.is_empty()).count() > 1 {
            why.push("hit windows");
        }
        why
    }

    /// How many fighters have a special that does something a strike cannot.
    ///
    /// A special is expressive when `expressive_reasons` is non-empty (a
    /// technique, stance, flow, gravity regime, and so on). A strike with cues is
    /// not.
    ///
    /// This is a ratchet, not a target. A plain-strike brawler is valid; going
    /// backwards silently is not. It holds the floor and prints the ranking.
    ///
    /// It walks this crate's tables, which are not the smash grid. `tables()`
    /// includes `theorem_chain`, Robot v2's duel-arena moveset, which is plain on
    /// purpose. Check which composition an entry belongs to before reading the
    /// plain list as "boring smash fighters".
    #[test]
    fn the_roster_does_not_get_less_expressive() {

        /// The floor, raised on purpose as fighters gain techniques.
        const FLOOR: usize = 18;

        let mut expressive: Vec<&str> = Vec::new();
        let mut plain: Vec<&str> = Vec::new();
        for (fighter, contract) in tables() {
            // Specials only.
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

        // Print the names every run, not only on failure: they answer "who is
        // next".
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

    /// The same question per special, not per fighter.
    ///
    /// The per-fighter ratchet saturated (every fighter has at least one
    /// expressive special), so it no longer moves. This counts specials, using the
    /// same `expressive_reasons` definition.
    ///
    /// A floor cannot catch inflation: adding something cosmetic to
    /// `expressive_reasons` would raise the count and pass. The defence is that
    /// there is one definition, next to the rule that cues do not count.
    ///
    /// Each reason was checked by removing it and seeing this test fail.
    #[test]
    fn the_specials_do_not_get_less_expressive() {
        /// Raised on purpose as specials gain mechanics. A fall is the failure this
        /// catches.
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

        // Print the names, as the sibling does: "which special next".
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

        /// A ceiling on plainness, which a floor cannot hold. A new bare special
        /// raises the total and leaves `rich` unchanged, so the floor stays green.
        ///
        /// A count, not a name allowlist: a name list grows to silence the guard.
        /// Each current plain special states its reason in its own file: the goblin's
        /// `scrap_flail`, `cellular_pulse` (not to be retuned during migration),
        /// `performer_the_line` (refuses `MoveEventKind::Ranged`) and
        /// `performer_trapdoor_air` (a deliberate feint).
        ///
        /// Raising this is a design decision, like lowering the floor.
        const PLAIN_CEILING: usize = 4;

        assert!(
            bare.len() <= PLAIN_CEILING,
            "specials with no mechanic rose to {} (ceiling {PLAIN_CEILING}).\n  \
             the four expected are deliberate and say so in their own files; \
             anything else here is a bare swing nobody decided on:\n  {bare:#?}",
            bare.len(),
        );
    }

    // The per-special census lives in this module so it uses the same
    // definition as the other census.
    use ambition_entity_catalog::MoveEventKind;

    /// Does this move carry a technique, or is it a hitbox and nothing else?
    ///
    /// Every site counts. `MoveSpec::effect_refs` is exhaustive by destructure
    /// (events, window `sustain_effect`, volume `on_hit`, flow `Emit`), so a new
    /// site is a compile error.
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

    /// Census of specials by technique: a special with no technique is a hitbox
    /// on another button.
    ///
    /// It asserts only that it measured something. There is no correct number of
    /// bare specials. Run with `--nocapture` to read the roster.
    ///
    /// "Bare" means no technique, not "boring". The Perfect Cellular Automaton's
    /// down-B has no technique but is a three-pulse `multihit` with autolink
    /// volumes and a telegraph. So the census also reports what else a special
    /// carries; a special with neither is the one worth reading.
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
                // Cues only, for the printout; they do not count toward the verdict.
                // "Has a cue" is still a useful state when choosing what to work on.
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
                // The plain verdict uses the shared definition, which counts volume
                // reactions.
                let reasons = expressive_reasons(spec);
                if keys.is_empty() {
                    bare_total += 1;
                    // Shared definition only: a cue is not a mechanic.
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
                        // Reached only when `reasons` is empty; the cue count says how it is
                        // plain.
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

    /// The one link from the attack kit to movement scoring is inert on this
    /// roster.
    ///
    /// `generate_options` scores movement with
    /// `movement_options(&view, situation, !lifts.is_empty())`, a boolean from the
    /// attack kit via `lifting_candidates`. For every shipped fighter it is the
    /// same whether the kit is resolved standing or running, so a kit change
    /// cannot change movement scoring through it.
    ///
    /// That can change. A fighter whose only lifting move is a smash or a tilt
    /// would make it fire when a run pre-empts that press. This guard catches
    /// that.
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

        // The kit in one stance, resolved as the press road resolves it: a run
        // pre-empts the smash gesture and forces the base to attack; a special never
        // takes that road.
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

        // Anti-vacuity, both halves: a roster nobody walked, and a roster where
        // nobody lifts. "No lifts either way" is a different finding from "the same
        // lifts either way".
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

    /// Can a projectile's verdict reach a connect condition?
    ///
    /// A12 blocker 4: a damage verdict with `attacker_move_instance: None` is
    /// credited to the move the attacker is playing now
    /// (`moveset::verdict_belongs_to`). Projectile verdicts carry `None` (three
    /// sites in `projectile/systems.rs`), so move A can fire a shot and move B can
    /// be credited with a `connected_hit` and take an OnHit escape.
    ///
    /// The full fix propagates the instance to the shot (expensive: 43
    /// construction sites) and then requires a claim. This census checks if the
    /// first half is needed: does any fighter author both a shooting move and a
    /// conditional cancel? If not, no projectile verdict reaches a connect
    /// condition.
    ///
    /// The authored set is a superset of the admitted one, so a negative result
    /// is sound. A positive result must be re-checked on the composed app.
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

        // Anti-vacuity, both halves. Another guard in this file requires a confirm
        // to exist, so zero here means the walk is broken.
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

/// The character ids each table above is the moveset for.
///
/// The keys of [`tables`] are file names, not identities, and many differ
/// from the cast id (`alice` vs `npc_alice`, `patent_clerk` vs
/// `special_patent_clerk`, and so on). `cellular_automaton` is one table for
/// two ids. A content file keyed by a file name is looked up by character id
/// in `authored_intrinsics`, misses, and is silently ignored.
///
/// A name that exists elsewhere is not necessarily a cast id:
/// `ninja_shadow_oni_leader` is a sheet id (in
/// `sprites_0_25x/ninja_shadow_oni_leader_actor.ron`), not a cast id. <!-- cite-ok: an asset path relative to the content assets root, not a repo path -->
///
/// This cannot be derived: the old link was `.with_moveset(...)` in each
/// creature's file, and moving tables to content removed it.
/// `the_table_character_map_covers_every_table` and
/// `every_character_the_move_section_names_is_one_this_game_builds` keep it
/// in step.
///
/// Two ids for one table is real: the two cellular automatons are one
/// authored body under two names (see `authored::AUTHORED_CAST`).
///
/// Absent on purpose:
/// * `player_robot`: the catalog has rows (`player_robot_v3`,
///   `player_robot_fable`, `player_robot_v2`), but
///   `player_robot_lineage::register` builds them with `definition_from` and
///   never calls `authored_intrinsics`, and `register_declared_cast` skips
///   lineage ids. `the_lineage_never_reaches_the_authored_intrinsics_seam`
///   pins that.
/// * `theorem_chain`: an archetype table no catalog row names.
pub const TABLE_CHARACTERS: &[(&str, &[&str])] = &[
    ("alice", &["npc_alice"]),
    ("bob", &["npc_bob"]),
    ("carl_stargan", &["npc_carl_stargan"]),
    (
        "cellular_automaton",
        &["perfect_cellular_automaton", "imperfect_cellular_automaton"],
    ),
    ("goblin", &["goblin"]),
    ("ninja_shadow_oni_leader", &["npc_ninja_shadow_oni_leader"]),
    ("emmy_noether", &["npc_emmy_noether"]),
    ("oiler", &["npc_oiler"]),
    ("patent_clerk", &["special_patent_clerk"]),
    ("pirate_admiral", &["npc_pirate_admiral"]),
    ("director", &["director"]),
    ("medic", &["medic"]),
    ("officer", &["officer"]),
    ("performer", &["performer"]),
    ("pointed_polygon", &["pointed_polygon"]),
    ("projectile_polygon", &["projectile_polygon"]),
    ("pugnacious_polygon", &["pugnacious_polygon"]),
];

/// The character ids this table is the moveset for, or `None` for a table no
/// cast id claims.
pub fn characters_for(table: &str) -> Option<&'static [&'static str]> {
    TABLE_CHARACTERS
        .iter()
        .find(|(name, _)| *name == table)
        .map(|(_, ids)| *ids)
}

#[cfg(test)]
mod table_character_tests {
    use super::*;

    /// The two lists cannot drift. A table with no entry in
    /// [`TABLE_CHARACTERS`] cannot be migrated; an entry for a removed table maps
    /// nothing.
    #[test]
    fn the_table_character_map_covers_every_table() {
        let tables: std::collections::BTreeSet<&str> =
            tables().iter().map(|(name, _)| *name).collect();
        let mapped: std::collections::BTreeSet<&str> =
            TABLE_CHARACTERS.iter().map(|(name, _)| *name).collect();
        assert!(tables.len() >= 10, "{} table(s) is not the roster", tables.len());

        // The two deliberate absences are named, not filtered, so a third one
        // fails.
        let unmapped: Vec<&str> = tables.difference(&mapped).copied().collect();
        assert_eq!(
            unmapped,
            vec!["player_robot", "theorem_chain"],
            "a table has no character mapping. Add it to `TABLE_CHARACTERS`, or \
             say here why it has no cast id — those two do, in this module's docs"
        );
        let orphans: Vec<&str> = mapped.difference(&tables).copied().collect();
        assert!(
            orphans.is_empty(),
            "`TABLE_CHARACTERS` maps {orphans:?}, which `tables()` no longer carries"
        );
    }

    /// Every mapped id is a character this game builds, so the map cannot name a
    /// plausible id nobody registers.
    #[test]
    fn every_mapped_character_is_one_this_game_builds() {
        let buildable: std::collections::BTreeSet<&str> =
            crate::character_catalog::buildable_cast().collect();
        assert!(
            buildable.len() >= 20,
            "{} buildable character(s) — not the cast",
            buildable.len()
        );
        let strangers: Vec<&str> = TABLE_CHARACTERS
            .iter()
            .flat_map(|(_, ids)| ids.iter().copied())
            .filter(|id| !buildable.contains(id))
            .collect();
        assert!(
            strangers.is_empty(),
            "`TABLE_CHARACTERS` names {strangers:?}, which this game builds no \
             character for"
        );
    }
}

#[cfg(test)]
mod offer_census {

    /// PROBE: how far out does an authored hit region START?
    ///
    /// The option layer admits a move when the opponent lies between the near
    /// and far sides of its region, with `ADMISSION_SLACK_PX` slack on each. If
    /// every authored box starts within the slack, the near test never refuses
    /// anything, and "the region contains them" means only "the region reaches
    /// them". Check this before tuning.
    #[test]
    #[ignore = "PROBE, print-only: where each authored hit region begins"]
    fn probe_how_far_out_an_authored_region_begins() {
        let mut deep: Vec<(String, f32, f32)> = Vec::new();
        let mut total = 0usize;
        for (table, set) in super::tables() {
            for m in &set.moves {
                let Some(coverage) = m.frame_data().coverage else {
                    continue;
                };
                total += 1;
                // Straight ahead, against a point target: the near side with nothing
                // forgiven.
                let Some((near, far)) = coverage.span_toward((1.0, 0.0), (0.0, 0.0)) else {
                    continue;
                };
                if near > 0.0 {
                    deep.push((format!("{table}/{}", m.id), near, far));
                }
            }
        }
        deep.sort_by(|a, b| b.1.total_cmp(&a.1));
        for (id, near, far) in deep.iter().take(20) {
            println!("[near] {id:<48} begins {near:>6.1} ends {far:>6.1}");
        }
        println!(
            "[near] {} of {total} authored hit regions begin away from the body; \
             deepest {:.1}px",
            deep.len(),
            deep.first().map(|d| d.1).unwrap_or(0.0),
        );
    }
    /// PROBE: which roster moves offer the attack scorer nothing to price?
    ///
    /// Print-only. `MoveFrameData::coverage` is `None` for four different kinds
    /// of move (a counter, a buff, a projectile launcher, a pure-motion
    /// recovery), and the option scorer treated them as one.
    ///
    /// The membership rule must match `generate_options`' `(None, None)` arm
    /// (including `hazard_reach`). Check it by widening the arm and watching this
    /// number fall.
    #[test]
    #[ignore = "PROBE, print-only: the roster's hitless, shoveless, motionless moves"]
    fn probe_the_moves_that_offer_the_attack_scorer_nothing() {
        let mut total = 0usize;
        let mut silent = 0usize;
        for (table, set) in super::tables() {
            for m in &set.moves {
                total += 1;
                let f = m.frame_data();
                if f.coverage.is_none() && f.push_coverage.is_none() && f.hazard.is_none() {
                    silent += 1;
                    let mut keys: Vec<&str> = m
                        .events
                        .iter()
                        .filter_map(|e| match &e.kind {
                            ambition_entity_catalog::MoveEventKind::Effect(effect) => {
                                Some(effect.key.as_str())
                            }
                            _ => None,
                        })
                        .chain(
                            m.windows
                                .iter()
                                .filter_map(|w| w.sustain_effect.as_ref())
                                .map(|e| e.key.as_str()),
                        )
                        .collect();
                    keys.sort_unstable();
                    keys.dedup();
                    println!(
                        "[offer] {table}/{:<34} route={:?} lift={:>4.0} keys={keys:?}",
                        m.id,
                        f.recovery_route,
                        f.lift_speed,
                    );
                }
            }
        }
        println!("[offer] {silent} of {total} authored moves offer the attack scorer nothing");
    }
}
