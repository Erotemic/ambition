//! The Director: the sword archetype's table, wielded with a pen.
//!
//! His rig is the Pointed Polygon's, retargeted: the pen has the arming
//! sword's axis and length, so all 136 archetype clips read correctly on him.
//! His spacing follows from that reach, so his frame data is the archetype's,
//! not a copy that would drift.
//!
//! His own parts are the name and several specials (including the recovery).
//! See [`director_moveset`].

use ambition_entity_catalog::MovesetContract;

/// When the thought leaves him.
const BOLT_AT_S: f32 = 0.20;

/// When the move releases him. It must end before the bolt's lifetime does,
/// or a whiff pins him through his own punish window.
const BOLT_ENDS_S: f32 = 0.46;

/// When he vanishes. Slower than the robot's blink: his is a written edit, not
/// a machine's phase-out.
const TELEPORT_AT_S: f32 = 0.18;

/// When the move ends. The tail is him being drawn back in.
const TELEPORT_ENDS_S: f32 = 0.48;

/// Complete sword-fundamentals repertoire, attributed to the Director.
///
/// His up-B is his own. Jon: *"Mewtwo / Palutena / Zelda style teleports"*, a
/// teleport that looks like an affine transform to a point, with a star
/// flash out and the reverse at the destination.
///
/// The mechanic is the robot's; the look is not. Both author the same
/// `smash.teleport` technique and ledge assist, and differ only in the two
/// effect ids, so the look lives in the params, not in the engine.
///
/// It replaces the archetype's `rising_edge`, which stays the Pointed
/// Polygon's.
///
/// One art row is used twice. `four_point_glint` is the star flash; the
/// converging reverse is not drawn yet, so both ends use the existing row
/// until it is.
pub fn director_moveset() -> MovesetContract {
    let mut set = crate::archetype_moveset::under_own_name(
        crate::pointed_polygon_moveset::pointed_polygon_moveset(),
        &["polygon", "pointed_polygon"],
        "director",
    );
    crate::special_slots::replace_special(&mut set, "special_up", directors_teleport());
    crate::special_slots::replace_special(&mut set, "special_down", the_second_draft());
    crate::special_slots::replace_special(&mut set, "special_forward", a_train_of_thought());

    // His low poke leaves a delayed mark: the revision lands later.
    //
    // It is on his weakest move (`director_tilt_down`, 4 damage) on purpose.
    // On the forward smash it would only make a strong move stronger; here it
    // turns a neutral-game tool into a threat, so the opponent must decide what
    // to do for the next second and a half.
    //
    // 1.4s is a decision, not a late hit: long enough to run, shield, or stand
    // next to him and trade the blast (the detonation is `Environment`, so it
    // hits anyone).
    //
    // No new engine authority: `OnHitEffectMessage` carries the victim,
    // `HitVolume::on_hit` the payload, `DamageBoxEffect` the blast, and the clock
    // is a ruleset component like `PlacedMine`.
    ambition_entity_catalog::smash_mark::mark_move_in(
        &mut set,
        "director_tilt_down",
        ambition_entity_catalog::smash_mark::MarkBodyParams {
            fuse_s: 1.4,
            damage: 6,
            blast_radius: 44.0,
            knockback: 1.2,
        },
    );
    set
}

/// Side special: he sends a thought out and flies it with the stick.
///
/// A PK-Thunder-style "mind" attack. It replaces the archetype's
/// `vector_lunge`; a writer steering a thought fits him.
///
/// No input lease is needed. `ActorControlFrame::steer_axis()` publishes the
/// player's stick separately from what the body may move by (a rooted move
/// reads `locomotion` as zero), so he keeps his seat and the bolt reads his
/// live stick. Steering is not possession.
///
/// Flying it into his own back is a recovery: see `self_launch`.
///
/// He is not helpless while it is out. The move roots him to 0.46s; the
/// thought lives 2.2s, and a test requires that gap. The cost is that one
/// stick walks him and turns the thought, so every step he takes also turns
/// the bolt.
fn a_train_of_thought() -> ambition_entity_catalog::MoveSpec {
    let spec = ambition_entity_catalog::authoring::hitless_special(
        "director_train_of_thought",
        "special_forward",
        BOLT_AT_S,
        BOLT_ENDS_S,
    );
    let spec = ambition_entity_catalog::smash_bolt::author_steered_bolt(
        spec,
        BOLT_AT_S,
        ambition_entity_catalog::smash_bolt::SteeredBoltParams {
            // Slow enough to steer and fast enough to cross a gap.
            speed: 300.0,
            // The key number: at 220°/s a full reversal takes most of a second, so a
            // turn costs distance. Too low is a straight shot; too high is a missile
            // that cannot miss.
            turn_rate_deg: 220.0,
            // Long enough to go out and come back; short enough that a whiff is a
            // real punish window.
            lifetime_s: 2.2,
            damage: 8,
            radius: 11.0,
            // A feel multiplier (see `SteeredBoltParams::knockback`), not raw
            // knockback. In band with the boss blast's 1.6.
            knockback: 1.6,
            // The recovery's whole range. Well above the bolt's speed, or the
            // self-launch would be slower than walking.
            self_launch: 700.0,
            // At arm's length and slightly above, so it visibly leaves.
            offset: (24.0, -12.0),
            // The bolt must be visible: steering it is the mechanic. His own star
            // flash, marked often enough that the path reads as a line.
            trail_vfx: "four_point_glint".to_string(),
            trail_every_s: 0.05,
        },
    );
    let spec =
        ambition_entity_catalog::authoring::sfx(spec, BOLT_AT_S, "player.attack.charge");
    spec
}

/// The Director's counter: you land the blow, and it is revised out of the scene.
///
/// Swordies get a counter, and the Director is the sword archetype.
///
/// The response is his teleport in ambush mode (`behind_nearest_foe`), which
/// arrives on the far side of the foe: you swing, and he is behind you. It is
/// his up-B's technique with one flag changed, and it differs from George's
/// `riposte` (which grabs).
///
/// This is the first authored user of `behind_nearest_foe: true`, so it also
/// exercises the ambush arrival, foe selection and facing rule.
///
/// He absorbs shots instead of returning them, unlike `riposte`: an author
/// deletes your sentence. This also keeps the move from being strictly better
/// than George's: reposition or reflection, not both.
fn the_second_draft() -> ambition_entity_catalog::MoveSpec {
    ambition_entity_catalog::smash_counter::counter_move(
        "director_second_draft",
        "special",
        // Faster to open than the riposte and shorter: he is noticing, not
        // blocking.
        0.05,
        0.14,
        0.42,
        ambition_entity_catalog::smash_counter::CounterParams {
            // A heartbeat, not a duration: `parry_window_timer` decays, and the stance
            // re-arms it every live frame. Three ticks of slack at 60Hz.
            window_s: 0.05,
            // Its own answer, as every counter but the clerk's is.
            answers_the_attacker: false,
            response: ambition_entity_catalog::smash_teleport::TELEPORT.to_string(),
            response_params: ambition_entity_catalog::ParamValue::from_typed(
                &ambition_entity_catalog::smash_teleport::TeleportParams {
                    behind_nearest_foe: true,
                    // From the foe's edge, so he lands the same distance behind a small body
                    // and a large one.
                    behind_gap: 26.0,
                    // A range, not a leash: a foe beyond this is not a target, and the
                    // teleport refuses instead of landing him in front. A parried attacker is
                    // within melee reach.
                    distance: 180.0,
                    // Zero: this is not a recovery. A ledge catching the arrival would take the
                    // punish away.
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

/// The Director's recovery: he edits himself out and back in somewhere else.
fn directors_teleport() -> ambition_entity_catalog::MoveSpec {
    let spec = ambition_entity_catalog::authoring::hitless_special(
        "director_revision",
        "special_up",
        TELEPORT_AT_S,
        TELEPORT_ENDS_S,
    );
    let mut spec = spec;
    spec.display_name = Some("Revision".to_string());
    let spec = ambition_entity_catalog::smash_teleport::author_teleport(
        spec,
        TELEPORT_AT_S,
        ambition_entity_catalog::smash_teleport::TeleportParams {
            // Aimed, like every recovery: any direction given between the press and
            // the transit at `TELEPORT_AT_S`, straight up if none. The startup is the
            // aim window. See `TeleportParams::behind_nearest_foe`.
            behind_nearest_foe: false,
            behind_gap: 0.0,
            // Further than the robot's, and slower to come out.
            distance: 250.0,
            // Same ledge assist radius as the robot: it belongs to recovering onto a
            // stage, not to either fighter.
            ledge_assist: 44.0,
            // Same intangible window as the robot, for the same reason.
            intangible_s: 0.12,
            depart_vfx: "four_point_glint".to_string(),
            arrive_vfx: "four_point_glint".to_string(),
        },
    );
    let spec = ambition_entity_catalog::authoring::sfx(spec, 0.0, "player.attack.charge");
    // No authored blink cue. `apply_authored_teleports` emits `PLAYER_BLINK`
    // at the transit for every authored teleport, so an event here would play
    // it twice. The rule is whether the move runs the teleport executor: moves
    // that do not (the Performer's trap, Alice's side-B impulse) choose their
    // own cue; any move authored through `author_teleport` must not carry one.
    // Wrap through the slot so `gates.recovery` is stamped: this is inserted
    // after `SmashRepertoire::into_contract`, and an up-B that costs nothing is
    // flight.
    ambition_entity_catalog::smash_repertoire::UpSpecial::Standard(spec).into_spec()
}

#[cfg(test)]
mod tests {

    /// The swap is complete: the verb points at the new move, the move is in the
    /// table, and the archetype's rise is removed, not left unreachable.
    #[test]
    fn the_director_recovers_by_teleporting_and_the_archetypes_rise_is_gone() {
        let set = crate::authored_movesets::shipped("director");
        assert_eq!(
            set.verbs.get("special_up").map(String::as_str),
            Some("director_revision"),
            "his up-B must be the teleport"
        );
        assert!(
            set.moves.iter().any(|m| m.id == "director_revision"),
            "…and the move it names must be in the table"
        );
        assert!(
            !set.moves.iter().any(|m| m.id == "director_rising_edge"),
            "…and the archetype's spinning rise must not be left behind \
             unreachable, where every census that walks `moves` reports it as \
             part of his kit"
        );
    }

    /// He must be free before the thought fades, or a whiff pins him through his
    /// own punish window. The guard states the relationship between the two
    /// numbers, not either number.
    #[test]
    fn his_thought_outlives_the_move_that_threw_it() {
        let set = crate::authored_movesets::shipped("director");
        assert_eq!(
            set.verbs.get("special_forward").map(String::as_str),
            Some("director_train_of_thought"),
            "his side-B must be the steered thought"
        );
        let move_spec = set
            .moves
            .iter()
            .find(|m| m.id == "director_train_of_thought")
            .expect("…and the move it names must be in the table");
        let bolt: ambition_entity_catalog::smash_bolt::SteeredBoltParams = move_spec
            .events
            .iter()
            .find_map(|event| match &event.kind {
                ambition_entity_catalog::MoveEventKind::Effect(effect)
                    if effect.key
                        == ambition_entity_catalog::smash_bolt::STEERED_BOLT =>
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

        // It carries him: `self_launch` must beat the bolt's speed, or it would be
        // slower than walking.
        assert!(
            bolt.self_launch > bolt.speed,
            "the thunder jacket ({}) is slower than the bolt ({})",
            bolt.self_launch,
            bolt.speed,
        );

        // The archetype's lunge is removed, not left unreachable.
        assert!(
            !set.moves.iter().any(|m| m.id == "director_vector_lunge"),
            "the displaced lunge is still in the table"
        );
    }

    /// The counter is his: the response must be a teleport that arrives behind
    /// the attacker. A check for `smash.counter` alone would pass a copy of
    /// George's riposte.
    #[test]
    fn the_directors_counter_answers_by_arriving_behind_whoever_swung() {
        let set = crate::authored_movesets::shipped("director");
        assert_eq!(
            set.verbs.get("special_down").map(String::as_str),
            Some("director_second_draft"),
            "his down-B must be the counter"
        );
        let counter = set
            .moves
            .iter()
            .find(|m| m.id == "director_second_draft")
            .expect("…and the move it names must be in the table");

        let params: ambition_entity_catalog::smash_counter::CounterParams = counter
            .windows
            .iter()
            .filter_map(|window| window.sustain_effect.as_ref())
            .find(|effect| {
                effect.key == ambition_entity_catalog::smash_counter::COUNTER
            })
            .expect("the move holds a counter stance")
            .params
            .hydrate()
            .expect("counter params hydrate");

        assert_eq!(
            params.response,
            ambition_entity_catalog::smash_teleport::TELEPORT,
            "his counter must answer with the teleport, not a grab"
        );
        let teleport: ambition_entity_catalog::smash_teleport::TeleportParams =
            params.response_params.hydrate().expect("teleport params hydrate");
        assert!(
            teleport.behind_nearest_foe,
            "the response is an AIMED teleport, so he answers a parry by leaving \
             rather than by arriving behind the swing"
        );
        // Not a recovery: a positive `ledge_assist` would let a ledge catch the
        // ambush arrival, and would give him a second recovery.
        assert_eq!(teleport.ledge_assist, 0.0);
        assert!(
            params.absorbs_projectiles,
            "he deletes shots rather than returning them — the difference from \
             the riposte, and what stops this being strictly better than it"
        );
    }

    /// The down swap took the grounded half and left the aerial one. His
    /// borrowed down-special is `DownSpecial::ByPosture` (two moves on two
    /// verbs), so replacing `special_down` must keep his air-down.
    #[test]
    fn the_counter_displaced_the_ground_low_arc_and_spared_the_falling_edge() {
        let set = crate::authored_movesets::shipped("director");
        // Name the id the borrowed table currently provides, not a historical one;
        // a renamed id would make this check pass without testing anything.
        assert!(
            !set.moves.iter().any(|m| m.id == "director_riposte"),
            "the displaced grounded down-B is still in the table, where every \
             census that walks `moves` reports it as part of his kit"
        );
        assert!(
            set.moves.iter().any(|m| m.id == "director_falling_edge"),
            "his AERIAL down-special went with it — that verb was never replaced"
        );
    }

    /// It is still a recovery. `UpSpecial::Standard` sets `gates.recovery` on
    /// the move it lowers; a replacement inserted after that must carry the cost,
    /// or he gets an unlimited teleport.
    #[test]
    fn the_replacement_still_spends_the_airtimes_recovery() {
        let set = crate::authored_movesets::shipped("director");
        let up = set
            .moves
            .iter()
            .find(|m| m.id == "director_revision")
            .expect("his up-B is in the table");
        assert_ne!(
            up.gates.recovery,
            ambition_entity_catalog::RecoveryUse::None,
            "an up-B that costs nothing is flight"
        );
    }
}
