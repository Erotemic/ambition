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

/// When the thought leaves him.
const BOLT_AT_S: f32 = 0.20;

/// When the move releases him. ⚠ LONGER THAN THE BOLT'S OWN LIFETIME IS WRONG —
/// he must be free before the thought fades, or a whiff pins him through his own
/// punish window with nothing on screen to explain it.
const BOLT_ENDS_S: f32 = 0.46;

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
    crate::special_slots::replace_special(&mut set, "special_down", the_second_draft());
    crate::special_slots::replace_special(&mut set, "special_forward", a_train_of_thought());
    set
}

/// Side special: he sends a thought out and flies it with the stick.
///
/// ⭐⭐ JON'S ASSIGNMENT, 2026-09-05: *"I want the author to have side-b be the
/// pk-thunder style 'mind' attack."* This is that move, and it belongs to him
/// twice over — the archetype's `vector_lunge` was a borrowed poke, and a writer
/// steering a thought around the stage is the one fighter on the roster for whom
/// the fiction is literal.
///
/// ⭐⭐ AND IT NEEDED NO INPUT LEASE, which the campaign plan had as the rung
/// before it. `ActorControlFrame::steer_axis()` already publishes what the
/// PLAYER is holding as distinct from what the BODY may move by — it exists
/// because a rooted move reads `locomotion` as zero — so he keeps his own seat
/// and the bolt reads his live stick. Steering is not possession; only the first
/// was ever wanted here.
///
/// ⛔ FLYING IT INTO HIS OWN BACK IS THE POINT. `self_launch` is what makes this
/// a recovery as well as an attack.
///
/// ⛔⛔ AND HE IS NOT HELPLESS WHILE IT IS OUT — an earlier version of this doc
/// said he was, and the code never agreed. The move roots him to 0.46s; the
/// thought lives 2.2s, and the guard below REQUIRES that gap so a whiff does not
/// pin him through his own punish window. ⇒ For most of the flight he is free.
///
/// ⭐ WHICH MAKES THE COST BETTER THAN THE ONE I THOUGHT I HAD WRITTEN: he is not
/// helpless, he is DIVIDED. One stick walks him and turns the thought, so every
/// step he takes to reposition is a turn he did not choose — and flying it home
/// means walking where the bolt needs him to walk.
fn a_train_of_thought() -> ambition_platformer2d::entity_catalog::MoveSpec {
    let spec = ambition_characters::moveset_authoring::hitless_special(
        "author_train_of_thought",
        "special_forward",
        BOLT_AT_S,
        BOLT_ENDS_S,
    );
    let spec = ambition_characters::smash_bolt::author_steered_bolt(
        spec,
        BOLT_AT_S,
        ambition_characters::smash_bolt::SteeredBoltParams {
            // Slow enough to steer and fast enough to cross a gap.
            speed: 300.0,
            // ⭐ THE NUMBER THAT IS THE MOVE. At 220°/s a full reversal takes
            // most of a second, so a turn genuinely costs distance — too low and
            // it is a straight shot, too high and it is a missile he cannot miss
            // with.
            turn_rate_deg: 220.0,
            // Long enough to go out and come back, and short enough that a
            // whiffed thought is a real punish window.
            lifetime_s: 2.2,
            damage: 8,
            radius: 11.0,
            knockback: 92.0,
            // ⚠ THE RECOVERY'S WHOLE RANGE lives in this one number. Well above
            // the bolt's own speed, because a self-launch that merely matched it
            // would be a slower way to travel than walking.
            self_launch: 700.0,
            // Out at arm's length and a little above, so it leaves visibly
            // rather than budding out of his chest.
            offset: (24.0, -12.0),
        },
    );
    let spec =
        ambition_characters::moveset_authoring::sfx(spec, BOLT_AT_S, "player.attack.charge");
    spec
}

/// The Author's counter: you land the blow, and it is revised out of the scene.
///
/// ⭐⭐ JON'S ASSIGNMENT, 2026-09-05: *"Swordies will get a counter."* The Author
/// IS the sword archetype — this module's own first line says so — and until now
/// the counter's only authored customers were demo stand-ins. This puts it on a
/// fighter somebody picks.
///
/// ⭐⭐ AND THE RESPONSE IS HIS TELEPORT, WHICH MAKES THE MOVE HIS RATHER THAN A
/// SECOND COPY OF GEORGE'S. `riposte` answers a parry with a GRAB; this answers
/// with `smash.teleport` in its **ambush** mode — `behind_nearest_foe`, which
/// arrives on the far side of the foe. ⇒ You commit to a swing, and he is behind
/// you. That is a sentence being rewritten while you are still in it, and it is
/// the same technique as his up-B with one flag flipped.
///
/// ⛔⛔ `behind_nearest_foe: true` HAD NO AUTHORED CUSTOMER ANYWHERE IN THE TREE.
/// The ambush arrival, its foe selection and its facing rule were all built and
/// nothing used them — the same state Sing's engine was in this morning. ⇒ A
/// capability with no customer is one nobody can tell is broken, so this is
/// worth more than the move it adds.
///
/// ⚠ HE ABSORBS SHOTS RATHER THAN RETURNING THEM, and that is the deliberate
/// difference from `riposte`, whose note argues reflection is "a reward the crowd
/// can see". An author does not throw your sentence back at you; he deletes it.
/// It also keeps the move from being strictly better than George's: reposition
/// OR reflection, not both.
fn the_second_draft() -> ambition_platformer2d::entity_catalog::MoveSpec {
    ambition_platformer2d::characters::smash_counter::counter_move(
        "author_second_draft",
        "special",
        // Faster to open than the riposte and shorter-lived: he is not blocking,
        // he is noticing.
        0.05,
        0.14,
        0.42,
        ambition_platformer2d::characters::smash_counter::CounterParams {
            // A HEARTBEAT, not a duration — `parry_window_timer` decays and the
            // stance re-arms it every live frame. Three ticks of slack at 60Hz.
            window_s: 0.05,
            response: ambition_platformer2d::characters::smash_teleport::TELEPORT.to_string(),
            response_params: ambition_platformer2d::entity_catalog::ParamValue::from_typed(
                &ambition_platformer2d::characters::smash_teleport::TeleportParams {
                    behind_nearest_foe: true,
                    // From the foe's EDGE, so he arrives the same distance behind
                    // a small body and a large one.
                    behind_gap: 26.0,
                    // ⛔ A RANGE, NOT A LEASH — the field's own doc: a foe beyond
                    // this is not a target and the teleport REFUSES rather than
                    // carrying him partway and landing him in front. Whoever just
                    // got parried is by definition within a melee reach of him,
                    // so this only has to cover that.
                    distance: 180.0,
                    // ⛔ ZERO, which the field's doc names as what a teleport that
                    // is not a recovery wants. He is arriving behind somebody, not
                    // climbing back on stage, and a ledge grabbing that arrival
                    // would take the punish away.
                    ledge_assist: 0.0,
                    // Through the swing he just answered, and no longer.
                    intangible_s: 0.14,
                    // His own two rows, the same glint his up-B uses.
                    depart_vfx: "four_point_glint".to_string(),
                    arrive_vfx: "four_point_glint".to_string(),
                },
            )
            .expect("the second draft's teleport params serialize"),
            absorbs_projectiles: true,
        },
    )
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
    // whether the move RUNS THE EXECUTOR — not whose move it is. The Performer's
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

    /// ⛔⛔ HE MUST BE FREE BEFORE THE THOUGHT FADES. The move's whole risk is
    /// the commitment while the bolt is out — but if the commitment OUTLASTS the
    /// bolt, a whiff pins him through his own punish window with nothing on
    /// screen explaining why he cannot move. ⇒ A relationship between two
    /// authored numbers, so the guard states the relationship rather than either
    /// number.
    #[test]
    fn his_thought_outlives_the_move_that_threw_it() {
        let set = author_moveset();
        assert_eq!(
            set.verbs.get("special_forward").map(String::as_str),
            Some("author_train_of_thought"),
            "his side-B must be the steered thought"
        );
        let move_spec = set
            .moves
            .iter()
            .find(|m| m.id == "author_train_of_thought")
            .expect("…and the move it names must be in the table");
        let bolt: ambition_platformer2d::characters::smash_bolt::SteeredBoltParams = move_spec
            .events
            .iter()
            .find_map(|event| match &event.kind {
                ambition_platformer2d::entity_catalog::MoveEventKind::Effect(effect)
                    if effect.key
                        == ambition_platformer2d::characters::smash_bolt::STEERED_BOLT =>
                {
                    effect.params.hydrate().ok()
                }
                _ => None,
            })
            .expect("his side-B throws a bolt");
        assert!(
            bolt.lifetime_s > move_spec.duration_s,
            "the thought fades at {}s inside a {}s move, so he is pinned watching \
             nothing",
            bolt.lifetime_s,
            move_spec.duration_s,
        );

        // ⭐ AND IT CARRIES HIM. `self_launch` is what makes this a recovery
        // rather than a slow projectile, and it must beat the bolt's own speed —
        // a self-launch that merely matched it would be a worse way to travel
        // than walking.
        assert!(
            bolt.self_launch > bolt.speed,
            "the thunder jacket ({}) is slower than the bolt ({})",
            bolt.self_launch,
            bolt.speed,
        );

        // ⛔ AND THE ARCHETYPE'S LUNGE IS GONE rather than left unreachable.
        assert!(
            !set.moves.iter().any(|m| m.id == "author_vector_lunge"),
            "the displaced lunge is still in the table"
        );
    }

    /// ⛔⛔ THE COUNTER IS HIS, AND THE AMBUSH IS THE PART WORTH GUARDING. A test
    /// that only found a `smash.counter` on his down-B would pass against a
    /// second copy of George's riposte — the whole reason this move is the
    /// Author's is that its response is a teleport that arrives BEHIND you.
    #[test]
    fn the_authors_counter_answers_by_arriving_behind_whoever_swung() {
        let set = author_moveset();
        assert_eq!(
            set.verbs.get("special_down").map(String::as_str),
            Some("author_second_draft"),
            "his down-B must be the counter"
        );
        let counter = set
            .moves
            .iter()
            .find(|m| m.id == "author_second_draft")
            .expect("…and the move it names must be in the table");

        let params: ambition_platformer2d::characters::smash_counter::CounterParams = counter
            .windows
            .iter()
            .filter_map(|window| window.sustain_effect.as_ref())
            .find(|effect| {
                effect.key == ambition_platformer2d::characters::smash_counter::COUNTER
            })
            .expect("the move holds a counter stance")
            .params
            .hydrate()
            .expect("counter params hydrate");

        assert_eq!(
            params.response,
            ambition_platformer2d::characters::smash_teleport::TELEPORT,
            "his counter must answer with the teleport, not a grab"
        );
        let teleport: ambition_platformer2d::characters::smash_teleport::TeleportParams =
            params.response_params.hydrate().expect("teleport params hydrate");
        assert!(
            teleport.behind_nearest_foe,
            "the response is an AIMED teleport, so he answers a parry by leaving \
             rather than by arriving behind the swing"
        );
        // ⛔ NOT A RECOVERY. `ledge_assist` above zero would let a ledge catch the
        // ambush arrival and take the punish away — and would quietly hand him a
        // second recovery on his down-B.
        assert_eq!(teleport.ledge_assist, 0.0);
        assert!(
            params.absorbs_projectiles,
            "he deletes shots rather than returning them — the difference from \
             the riposte, and what stops this being strictly better than it"
        );
    }

    /// ⛔ THE DOWN SWAP TOOK THE GROUNDED HALF AND LEFT THE AERIAL ONE. His
    /// borrowed down-special is `DownSpecial::ByPosture`, so it is TWO moves on
    /// two verbs; replacing `special_down` must not take his air-down with it.
    #[test]
    fn the_counter_displaced_the_ground_low_arc_and_spared_the_falling_edge() {
        let set = author_moveset();
        assert!(
            !set.moves.iter().any(|m| m.id == "author_low_arc"),
            "the displaced grounded down-B is still in the table, where every \
             census that walks `moves` reports it as part of his kit"
        );
        assert!(
            set.moves.iter().any(|m| m.id == "author_falling_edge"),
            "his AERIAL down-special went with it — that verb was never replaced"
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
