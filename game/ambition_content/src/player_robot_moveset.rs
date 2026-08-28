//! The player robot's canonical move repertoire — the moves that ARE the
//! protagonist, wherever it is seated.
//!
//! Move/refactor the canonical move data into the reusable Robot character provider and have both
//! compositions reference it."* This is that move.
//!
//! a move states what it IS, never what a mode does with it. Startup,
//! active frames, recovery, hitbox geometry, damage, base launch, growth,
//! landing lag and auto-cancel are properties of the swing. Percent, stocks,
//! blast zones, DI and the strength of knockback growth are the RULESET's, and
//! they are declared per stage (`DeclaredCombatRules`) rather than baked here —
//! which is what lets Ambition read this table as Hollow-Knight combat and
//! Smash read it as a platform fighter.

use ambition_characters::moveset_authoring::Strike;
use ambition_characters::smash_capture::{
    author_pummel, author_standing_grab, author_throw, capture_beat, grab_shell,
    CaptureAttemptParams, CaptureCues, CapturePummelParams, CaptureThrowParams,
    SmashCaptureRepertoire,
};
use ambition_characters::smash_repertoire::{
    DownSpecial, NeutralSpecial, SmashRepertoire, UpSpecial,
};
use ambition_platformer2d::entity_catalog::{
    ClipBinding, EffectRef, HitVolume, MoveEvent, MoveEventKind, MoveGates, MoveSpec, MoveWindow,
    MovesetContract, VolumeShape, WindowTag,
};

// the authoring primitives are SHARED (`moveset_authoring`), so the goblin's
// table below is written with the same `strike` this one is rather than a copy
// of it. They left this file the day a second character authored moves.
use ambition_characters::moveset_authoring::{
    committed_tail, impulse, on_contact, sfx, strike, vfx_at,
};
use ambition_platformer2d::entity_catalog::ImpulseMode;

/// The fighter repertoire, as one authored contract.
///
/// Shared by this demo's three fighters today. That is a content decision, not
/// an architectural one: the moveset rides the CHARACTER, so giving George a
/// heavier one is editing his definition and nothing else.
/// When the robot vanishes. Long enough that the disappearance is a read and
/// the move is punishable on reaction.
const BLINK_AT_S: f32 = 0.14;

/// When the move ends. The tail is the robot re-materialising, which is the
/// half of the animation that makes the arrival readable to the other player.
const BLINK_ENDS_S: f32 = 0.42;

pub fn player_robot_moveset() -> MovesetContract {
    // ── grounded ─────────────────────────────────────────────────────────────
    //
    // The jab is the fast, safe, boring one — it exists to be thrown at nothing
    // and get away with it, which is what makes the smash below a decision.
    let jab = strike(Strike {
        id: "jab",
        clip: "jab",
        startup_s: 0.05,
        active_s: 0.06,
        recover_s: 0.14,
        offset: (26.0, 0.0),
        half_extents: (18.0, 14.0),
        damage: 3,
        knockback: 55.0,
        knockback_growth: 1.10,
        launch_dir: None,
        on_hit: None,
    });

    let up_tilt = strike(Strike {
            id: "tilt_up",
            clip: "attack_up",
            startup_s: 0.07,
            active_s: 0.08,
            recover_s: 0.18,
            offset: (10.0, -30.0),
            half_extents: (20.0, 22.0),
            damage: 5,
            knockback: 70.0,
            knockback_growth: 1.40,
            // Straight up: an anti-air that starts a juggle rather than sending the
            launch_dir:
        // opponent away.
        Some((0.15, -1.0)),
            on_hit: None,
        });

    let down_tilt = strike(Strike {
        id: "tilt_down",
        clip: "attack_down",
        startup_s: 0.06,
        active_s: 0.06,
        recover_s: 0.16,
        offset: (26.0, 16.0),
        half_extents: (20.0, 10.0),
        damage: 4,
        knockback: 60.0,
        knockback_growth: 1.20,
        // A low poke that pops them up into the juggle.
        launch_dir: Some((0.5, -0.85)),
        on_hit: None,
    });

    // ── the smashes ──────────────────────────────────────────────────────────
    //
    // the move the demo did not have. A forward smash is eighteen frames
    // of startup you cannot take back, and the reason anybody accepts that is
    // the launch at the end of it: three times the jab's, growing with the
    // victim's percent, so at 120% it is the thing that ends the stock. The
    // charge multiplier is what a HELD press pays for.
    let mut f_smash = strike(Strike {
            id: "smash_forward",
            clip: "smash_forward",
            startup_s: 0.30,
            active_s: 0.07,
            recover_s: 0.34,
            offset: (40.0, -4.0),
            half_extents: (28.0, 20.0),
            damage: 15,
            knockback: 150.0,
            knockback_growth: 3.00,
            // Slightly upward and away: the classic kill angle. A contact-derived
            launch_dir:
        // direction would send a crouching opponent along the floor instead.
        Some((1.0, -0.42)),
            on_hit: None,
        });
    // A fully-held charge lands 1.7× as hard. `smash_charge_mult` scales damage
    // AND knockback by how far the owner's clock got through the leading
    // Startup window before release, so the commitment and the payoff are the
    // same authored number.
    f_smash.smash_charge_mult = 1.7;

    let mut up_smash = strike(Strike {
        id: "smash_up",
        clip: "smash_up",
        startup_s: 0.26,
        active_s: 0.08,
        recover_s: 0.32,
        offset: (8.0, -38.0),
        half_extents: (24.0, 30.0),
        damage: 14,
        knockback: 140.0,
        knockback_growth: 2.80,
        launch_dir: Some((0.12, -1.0)),
        on_hit: None,
    });
    up_smash.smash_charge_mult = 1.7;

    let mut down_smash = strike(Strike {
        id: "smash_down",
        clip: "smash_down",
        startup_s: 0.22,
        active_s: 0.08,
        recover_s: 0.30,
        offset: (0.0, 18.0),
        half_extents: (40.0, 14.0),
        damage: 12,
        knockback: 130.0,
        knockback_growth: 2.60,
        launch_dir: Some((1.0, -0.25)),
        on_hit: None,
    });
    down_smash.smash_charge_mult = 1.6;

    // ── aerials ──────────────────────────────────────────────────────────────
    //
    // landing lag and auto-cancel are what make an aerial a DECISION, and
    // both were engine features with no adopter. The pair reads: throw this one
    // early in a jump and land clean; throw it late and pay for it.
    let mut n_air = strike(Strike {
        id: "air_neutral",
        clip: "air_neutral",
        startup_s: 0.06,
        active_s: 0.14,
        recover_s: 0.16,
        offset: (14.0, 0.0),
        half_extents: (26.0, 22.0),
        damage: 6,
        knockback: 75.0,
        knockback_growth: 1.50,
        launch_dir: None,
        on_hit: None,
    });
    n_air.landing_lag_s = Some(0.10);
    n_air.autocancel_after_s = Some(0.26);

    let mut f_air = strike(Strike {
        id: "air_forward",
        clip: "air_forward",
        startup_s: 0.09,
        active_s: 0.08,
        recover_s: 0.22,
        offset: (32.0, -4.0),
        half_extents: (22.0, 18.0),
        damage: 9,
        knockback: 105.0,
        knockback_growth: 2.10,
        launch_dir: Some((1.0, -0.35)),
        on_hit: None,
    });
    f_air.landing_lag_s = Some(0.18);
    f_air.autocancel_after_s = Some(0.30);

    let mut b_air = strike(Strike {
        id: "air_back",
        clip: "air_back",
        startup_s: 0.10,
        active_s: 0.07,
        recover_s: 0.24,
        offset: (-32.0, -2.0),
        half_extents: (22.0, 18.0),
        damage: 11,
        knockback: 125.0,
        knockback_growth: 2.50,
        launch_dir: Some((-1.0, -0.38)),
        on_hit: None,
    });
    b_air.landing_lag_s = Some(0.20);
    b_air.autocancel_after_s = Some(0.32);

    let mut u_air = strike(Strike {
        id: "air_up",
        clip: "air_up",
        startup_s: 0.07,
        active_s: 0.09,
        recover_s: 0.20,
        offset: (4.0, -34.0),
        half_extents: (22.0, 24.0),
        damage: 7,
        knockback: 90.0,
        knockback_growth: 1.80,
        launch_dir: Some((0.1, -1.0)),
        on_hit: None,
    });
    u_air.landing_lag_s = Some(0.14);
    u_air.autocancel_after_s = Some(0.28);

    let mut d_air = strike(Strike {
            id: "air_down",
            clip: "air_down",
            startup_s: 0.12,
            active_s: 0.10,
            recover_s: 0.26,
            offset: (6.0, 30.0),
            half_extents: (20.0, 22.0),
            damage: 10,
            knockback: 110.0,
            knockback_growth: 2.20,
            // Straight DOWN — a spike. Offstage this is a stock; onstage it is a
            launch_dir:
        // bounce the opponent has to deal with.
        Some((0.0, 1.0)),
            // the ONE move that can bounce its attacker. Ambition reads this as a
            on_hit:
        // pogo; a platform fighter declares `Spike` and it becomes a kill.
        Some(EffectRef::new(
            ambition_platformer2d::combat::on_hit::POGO_BOUNCE_KEY,
        )),
        });
    // The heaviest lag in the set: a missed spike over the stage should hurt.
    d_air.landing_lag_s = Some(0.28);
    d_air.autocancel_after_s = Some(0.40);

    // PROTAGONIST was 12/16 — no forward tilt, and one special answering all
    // four directions, because the Hadouken arrives from the DERIVED kit (the
    // action set's ranged spec) and nothing had ever authored the other three.
    //
    // authored moves overlay the derived kit, they do not replace it, so
    // the Hadouken stays exactly where it is and keeps `special`. These three
    // take the directions it was standing in for.

    // the forward tilt. Without one the commonest press in the genre falls
    // down the directional chain to the jab — the hole five of the ten authored
    // tables had. A straight servo-driven extension: longer than the jab, slower,
    // and it moves you.
    let f_tilt = strike(Strike {
        id: "tilt_forward",
        clip: "attack_side",
        startup_s: 0.07,
        active_s: 0.07,
        recover_s: 0.17,
        offset: (30.0, -2.0),
        half_extents: (20.0, 14.0),
        damage: 6,
        knockback: 72.0,
        knockback_growth: 1.28,
        launch_dir: Some((1.0, -0.26)),
        on_hit: None,
    });
    let f_tilt = vfx_at(f_tilt, 0.07, "air_slice", (30.0, -2.0), 0.8);
    let f_tilt = sfx(f_tilt, 0.07, "player.directional_primary");
    let f_tilt = on_contact(f_tilt, "player.hit");

    // SIDE — `rocket_dash`. The dash it has at home, spent as one committed
    // pass instead of a movement option. `Set`, so it crosses the same
    // distance whatever it was doing — a recovery mix-up rather than a
    // momentum bonus.
    let side_b = strike(Strike {
        id: "rocket_dash",
        clip: "dash",
        startup_s: 0.12,
        active_s: 0.10,
        recover_s: 0.26,
        offset: (28.0, 0.0),
        half_extents: (24.0, 18.0),
        damage: 10,
        knockback: 104.0,
        knockback_growth: 1.95,
        launch_dir: Some((0.95, -0.38)),
        on_hit: None,
    });
    let side_b = impulse(side_b, 0.12, (660.0, 0.0), ImpulseMode::Set);
    let side_b = committed_tail(side_b, 0.60, 0.05);
    let side_b = vfx_at(side_b, 0.12, "dash_streak", (0.0, 0.0), 1.0);
    let side_b = sfx(side_b, 0.12, "player.dash");
    let side_b = on_contact(side_b, "player.hit");

    // UP — `phase_shift`. THE BLINK, AS A RECOVERY.
    //
    // ⭐⭐ JON'S DESIGN, 2026-08-27: *"The robot has a blink up-b, similar to how
    // it works in ambition in terms of the animation."* At home this body has a
    // held-item blink — a short directional teleport that stops a body-half
    // short of the first solid — and this is that same rule reached from a
    // repertoire slot instead of from an inventory. `blink_target` resolves both,
    // which is the whole reason the technique lives beside it rather than in a
    // game crate.
    //
    // ⛔ IT REPLACES `thruster_climb`, a burst of flight. The two are the same
    // fact stated under two rulesets — at home this body can FLY — and a
    // platform fighter does not get flight; but a thruster burst and a
    // teleport are different mechanics, and Jon asked for the one the robot
    // already owns.
    //
    // ⭐ THE LOOK IS THE PHASE-OUT, which is what "similar to how it works in
    // ambition" means: `teleport_depart` where it left, `teleport_arrive` where
    // it appears. The Author's teleport uses the same technique and a different
    // pair — see `author_moveset`.
    //
    // ⛔ NO HITBOX. A recovery that also struck on both ends would be a
    // recovery you throw at people, and the blink's offensive shockwave belongs
    // to the held item's version of it.
    let up_b = ambition_characters::moveset_authoring::hitless_special(
        "phase_shift",
        "fly",
        BLINK_AT_S,
        BLINK_ENDS_S,
    );
    let up_b = ambition_characters::smash_teleport::author_teleport(
        up_b,
        BLINK_AT_S,
        ambition_characters::smash_teleport::TeleportParams {
            // Aimed, like every recovery: the stick, then straight up.
            behind_nearest_foe: false,
            behind_gap: 0.0,
            // Comparable to a good double jump's height, so it recovers from a
            // real edgeguard and does not cross the stage.
            distance: 210.0,
            // ⭐⭐ THE LEDGE ASSIST. Without it a teleport recovery aimed at a
            // platform edge either lands on it or dies a few pixels under it,
            // and that margin is a stick angle nobody can hold.
            ledge_assist: 44.0,
            // ⭐ INTANGIBLE THROUGH THE VANISH. About seven frames, ending well
            // before the move does — the 0.28s of tail after the transit is what
            // the recovery still costs, and an edgeguarder who reads it still
            // wins. Without this the one frame that decides the stock is the one
            // where the body is nowhere.
            intangible_s: 0.12,
            depart_vfx: "teleport_depart".to_string(),
            arrive_vfx: "teleport_arrive".to_string(),
        },
    );
    let up_b = sfx(up_b, 0.0, "player.attack.charge");
    let up_b = sfx(up_b, BLINK_AT_S, "player.fly.start");

    // DOWN — `stabilizer_slam`. It drops its weight through its stabilizers
    // and the floor answers. Wide, flat, grounded-only, and slow enough that
    // whiffing it is the whole risk.
    let down_b = strike(Strike {
        id: "stabilizer_slam",
        clip: "attack_down",
        startup_s: 0.14,
        active_s: 0.09,
        recover_s: 0.30,
        offset: (0.0, 20.0),
        half_extents: (40.0, 12.0),
        damage: 9,
        knockback: 90.0,
        knockback_growth: 1.55,
        launch_dir: Some((0.75, -0.62)),
        on_hit: None,
    });
    let down_b = committed_tail(down_b, 0.62, 0.0);
    let down_b = vfx_at(down_b, 0.14, "shockwave", (0.0, 20.0), 1.1);
    let down_b = sfx(down_b, 0.14, "player.land.heavy");
    let down_b = vfx_at(down_b, 0.14, "hit_metal", (0.0, 16.0), 0.8);
    let down_b = on_contact(down_b, "player.hit");

    // effect on ground. Think of bowser down b. In the air he just does a
    // downward slam, but on the ground, it causes him to jump in an arc and then
    // slam. Specials can have different effects in different contexts that
    // should be ok, and makes for a richer smash game, although in most cases
    // they shouldn't be context dependent."*
    //
    // a special gated to ONE posture is not answered in the other — the
    // directional chain walks straight past it to the NEUTRAL special, so a
    // player pressing down-B in the air got the neutral-B. `special_air_down`
    // sits ahead of `special_down` in that chain and has the whole time; this is
    // the two-form move it exists for.
    // DOWN, IN THE AIR — `stabilizer_dive`. The same stabilizers, with no
    // floor to put them through: it drives them downward and brings the floor
    // to them.
    let mut air_down_b = strike(Strike {
        id: "stabilizer_dive",
        clip: "air_down",
        startup_s: 0.10,
        active_s: 0.10,
        recover_s: 0.24,
        offset: (0.0, 24.0),
        half_extents: (20.0, 22.0),
        damage: 9,
        knockback: 96.0,
        knockback_growth: 1.72,
        launch_dir: Some((0.0, 1.0)),
        on_hit: None,
    });
    air_down_b.landing_lag_s = Some(0.30);
    let air_down_b = impulse(air_down_b, 0.10, (0.0, 1250.0), ImpulseMode::Set);
    let air_down_b = vfx_at(air_down_b, 0.10, "hit_metal", (0.0, 20.0), 0.9);
    let air_down_b = sfx(air_down_b, 0.10, "player.fast_fall");
    let air_down_b = on_contact(air_down_b, "player.hit");

    // ROBOT'S CAPTURE KIT. The reference body: if a grab feels wrong on the robot
    // it is the mechanic, not the character.
    // the grab draws `attack`, not `grab`: these sheets publish no `grab` row,
    // and each table's own `every_clip_names_a_row_..._sheet_carries` guard says
    // so. `ClipBinding`'s fallbacks would have covered it at runtime, but a move
    // that NAMES a row nobody publishes is a lie the guard is right to refuse.
    let grab = author_standing_grab(
        grab_shell("robot_grab", "attack", 0.07, 0.05, 0.2),
        CaptureAttemptParams {
            offset: (12.0, 1.0),
            half_extents: (20.0, 16.0),
            hold_offset: (13.0, 3.0),
        },
    );
    let pummel = author_pummel(
        capture_beat("robot_pummel", "attack", 0.2),
        0.09,
        CapturePummelParams { damage: 4 },
    );
    let forward_throw = author_throw(
        capture_beat("robot_fthrow", "attack", 0.26),
        0.14,
        CaptureThrowParams {
            damage: 8,
            knockback: 120.0,
            knockback_growth: 2.0,
            launch_dir: (0.85, -0.55),
        },
    );

    let back_throw = author_throw(
        capture_beat("robot_bthrow", "attack", 0.28),
        0.15,
        CaptureThrowParams {
            damage: 9,
            knockback: 129.6,
            knockback_growth: 2.1,
            launch_dir: (-1.0, -0.34),
        },
    );

    let up_throw = author_throw(
        capture_beat("robot_uthrow", "attack", 0.27),
        0.14,
        CaptureThrowParams {
            damage: 8,
            knockback: 124.8,
            knockback_growth: 2.04,
            launch_dir: (0.0, -1.0),
        },
    );

    let down_throw = author_throw(
        capture_beat("robot_dthrow", "attack", 0.29),
        0.15,
        CaptureThrowParams {
            damage: 6,
            knockback: 88.8,
            knockback_growth: 1.6,
            launch_dir: (0.34, -0.92),
        },
    );
    SmashRepertoire {
        taunt: ambition_characters::moveset_authoring::taunt("player_robot_taunt", 0.9),
        dash_attack: ambition_characters::moveset_authoring::dash_attack(
            "player_robot_dash_attack",
            ambition_characters::moveset_authoring::DashAttackShape::GENRE,
            8,
            90.0,
        ),
        jab,
        forward_tilt: f_tilt,
        up_tilt,
        down_tilt,
        forward_smash: f_smash,
        up_smash,
        down_smash,
        neutral_air: n_air,
        forward_air: f_air,
        back_air: b_air,
        up_air: u_air,
        down_air: d_air,
        neutral_special: NeutralSpecial::FromBodyKit {
            because: "the charged Hadouken the robot's own body derives",
        },
        side_special: side_b,
        up_special: UpSpecial::Standard(up_b),
        // AUTHORED, at the rule that every fighter in the smash roster have a grab. The
        // transitional `None` is gone: capture was proven on George and the Pirate Admiral, and
        // the whole point of proving it was to stop being the only two.
        //
        // the VALUES are per character on purpose. A roster whose grabs are
        // twelve copies of one number set is one grab wearing twelve names.
        capture: SmashCaptureRepertoire {
            cues: CaptureCues::GENERIC,
            grab,
            pummel,
            forward_throw,
            back_throw: Some(back_throw),
            up_throw: Some(up_throw),
            down_throw: Some(down_throw),
        },
        down_special: DownSpecial::ByPosture {
            grounded: down_b,
            airborne: air_down_b,
        },
    }
    .into_contract()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Fourteen fighters each carried a copy of it: every bound verb names a move
    // this table defines, and the table binds the whole vocabulary. Both are now
    // unwritable defects rather than tested ones. `SmashRepertoire` owns the verb
    // strings, so there is no string in this file to misspell; it is a struct
    // with no `Default` and no private fields, so a missing or renamed slot is a
    // COMPILE error here. What the fourteen copies stood for — that every press
    // is answered, in every posture it is asked in — is checked once, by
    // `ambition_characters::smash_repertoire`, and by the host ratchet
    // `smash_roster_movesets::report_the_smash_kit_every_selectable_fighter_has`.

    /// The protagonist states its own verbs, so a match stops guessing.
    ///
    /// it authored none, and an unauthored character takes the migration
    /// bridge in `seat_abilities`: the MODE's declared set, stamped on verbatim.
    /// That bridge exists because almost nothing in the repo authors verbs yet,
    /// and it is documented as meant to shrink — this is the first character out
    /// of it, and the right first, because it is the one body both games share.
    ///
    /// `reset` is deliberately absent, and asserting that is the point:
    /// it is a debug affordance, and a character that authored it would hand
    /// every game that seats the robot a way to teleport home.
    ///
    /// `fly` is PRESENT, and my first pass had that wrong — see the note
    /// at the authoring site. It reads like a dev toggle from the player's side
    /// and is not: the robot is a grounded-base hybrid that takes to the air for
    /// vertical space, and the duel arena's exhibition robot uses it.
    #[test]
    fn the_robot_authors_its_verbs_rather_than_taking_a_match_s_word_for_them() {
        let v3 = crate::player_robot_lineage::definition(&crate::player_robot_lineage::V3);
        let verbs = v3.abilities.expect("v3 states what its body can do");
        assert!(verbs.jump && verbs.dash && verbs.attack && verbs.shield && verbs.dodge);
        assert!(verbs.blink, "blinking is what the robot IS");
        assert!(verbs.fly, "the grounded-base hybrid lost its fly toggle");
        assert!(
            !verbs.reset,
            "a debug affordance became part of the character, so every game that \
             seats the robot now receives a way to teleport home"
        );

        // a RETIRED incarnation shares the VERBS and not the MOVES, and
        // the split is the point: v0, v2 and v3 are one robot at three ages, so
        // what its body can do is the lineage's — the duel arena fields v2 and
        // it has to blink and dash like the robot it is. The current frame data
        // is v3's alone, because handing a retired incarnation today's timings
        // would be inventing content rather than migrating it.
        let v2 = crate::player_robot_lineage::definition(&crate::player_robot_lineage::V2);
        assert!(
            v2.abilities.is_some_and(|verbs| verbs.blink && verbs.dash),
            "the exhibition robot lost the verbs its archetype row granted it"
        );
        assert!(
            v2.moveset
                .as_ref()
                .is_some_and(|set| set.move_for_verb("special").is_some()),
            "v2 lost the theorem chain, which is the only proof in the repo that \
             a moveset expresses a multi-hit combo as data"
        );
        assert!(
            v2.moveset
                .as_ref()
                .is_some_and(|set| set.move_for_verb("smash_forward").is_none()),
            "v2 was handed v3's platform-fighter table"
        );
    }

    /// A MOVE CAN BE A COMBO, as data.
    ///
    /// the second hit has to HURT MORE, or the pair is a stutter rather than a
    /// chain.
    #[test]
    fn the_theorem_chain_is_two_hits_on_one_timeline() {
        use ambition_platformer2d::entity_catalog::WindowTag;
        let set = theorem_chain_moveset();
        let mv = set
            .move_for_verb("special")
            .expect("the chain is bound to the special verb");
        assert_eq!(mv.id, "theorem_chain");
        let hits: Vec<i32> = mv
            .windows
            .iter()
            .filter(|w| matches!(w.tag, WindowTag::Active) && !w.volumes.is_empty())
            .map(|w| w.volumes[0].damage)
            .collect();
        assert_eq!(
            hits.len(),
            2,
            "a chain with one Active window is a swing: {hits:?}"
        );
        assert!(
            hits[1] > hits[0],
            "the follow-up does not hit harder than the poke, so the pair is a \
             stutter rather than a chain: {hits:?}"
        );
    }

    /// The robot's projectile has its own look, stated by the character.
    ///
    /// this was `ranged_visual` on the archetype row, and the character-first
    /// constructor wrote an empty string — so a migrated robot fired an
    /// unadorned rock while the archetype road drew the Hadouken.
    #[test]
    fn the_robot_states_what_its_projectile_looks_like() {
        for incarnation in crate::player_robot_lineage::LINEAGE {
            let definition = crate::player_robot_lineage::definition(incarnation);
            assert_eq!(
                definition.ranged_vfx.as_deref(),
                Some("hadouken"),
                "`{}` fires an unadorned projectile",
                incarnation.id
            );
        }
    }

    /// The repertoire is a SMASH table, and it says so in its d-air.
    ///
    /// Same press, same geometry, two readings, and only the mode can choose between them.
    ///
    /// Pinned rather than described, because "the moves are shared and the
    /// ruleset interprets" is a claim that has to survive somebody retuning this
    /// table: the day the spike stops pointing down, whoever changed it should
    /// be told that a game elsewhere reads that direction.
    #[test]
    fn the_down_air_is_a_spike_which_is_what_a_pogo_mode_has_to_reinterpret() {
        let set = player_robot_moveset();
        let d_air = set
            .move_for_verb("attack_air_down")
            .expect("the repertoire binds a down-air");
        let launch = d_air
            .windows
            .iter()
            .flat_map(|window| window.volumes.iter())
            .find_map(|volume| volume.launch_dir)
            .expect("the spike states its direction rather than deriving it");
        assert!(
            launch.1 > 0.0,
            "the d-air stopped pointing down, so it is no longer the spike the \
             pogo mode has to reinterpret: {launch:?}"
        );
    }
}

/// THEOREM CHAIN — the robot's two-hit signature, a light poke into a
/// heavier follow-up on ONE timeline.
///
/// the only proof in the repo that a moveset expresses multi-hit combos as DATA across
/// characters rather than as a boss one-off.
///
/// v2's, not v3's. The duel arena fields Robot v2 against the PCA, and v3
/// carries the platform-fighter table instead. Two incarnations of one robot
/// with different repertoires is what a lineage IS — the same reason v0 and v2
/// keep their own silhouettes.
pub fn theorem_chain_moveset() -> MovesetContract {
    let volume = |offset: (f32, f32), half_extents: (f32, f32), damage: i32, knockback: f32| {
        HitVolume {
            // An ordinary hit, not a gust.
            shape: VolumeShape::Rect {
                offset,
                half_extents,
            },
            damage,
            knockback,
            // Flat, exactly as the row authored it.
            knockback_growth: None,
            launch_dir: None,
            on_hit: None,
            vfx: None,
            hit_sfx: None,
            reaction: None,
        }
    };
    let window = |start_s: f32, end_s: f32, tag: WindowTag, volumes: Vec<HitVolume>| MoveWindow {
        start_s,
        end_s,
        tag,
        volumes,
        motion_scale: 1.0,
        sustain_effect: None,
    };
    MovesetContract {
        verbs: [("special".to_string(), "theorem_chain".to_string())]
            .into_iter()
            .collect(),
        moves: vec![MoveSpec {
            display_name: None,
            id: "theorem_chain".to_string(),
            clip: ClipBinding {
                clip: "special".to_string(),
                fallbacks: vec!["slash".to_string(), "idle".to_string()],
            },
            duration_s: 0.72,
            windows: vec![
                window(0.0, 0.14, WindowTag::Startup, Vec::new()),
                // The light poke.
                window(
                    0.14,
                    0.22,
                    WindowTag::Active,
                    vec![volume((30.0, 0.0), (26.0, 22.0), 2, 90.0)],
                ),
                window(0.22, 0.36, WindowTag::Recovery, Vec::new()),
                // the SECOND Active window on the SAME timeline — the whole
                // point. A combo that needed two moves and a cancel would prove
                // the runtime can chain presses, not that a move can be a combo.
                window(
                    0.36,
                    0.46,
                    WindowTag::Active,
                    vec![volume((36.0, 0.0), (30.0, 24.0), 3, 160.0)],
                ),
                window(0.46, 0.72, WindowTag::Recovery, Vec::new()),
            ],
            events: vec![
                MoveEvent {
                    at_s: 0.14,
                    kind: MoveEventKind::Sfx {
                        cue: "player.theorem_1".to_string(),
                    },
                },
                MoveEvent {
                    at_s: 0.36,
                    kind: MoveEventKind::Sfx {
                        cue: "player.theorem_2".to_string(),
                    },
                },
            ],
            gates: MoveGates::default(),
            start_impulse: None,
            smash_charge_mult: 1.0,
            charge_gesture: ambition_platformer2d::entity_catalog::ChargeGesture::default(),
            smash_charge: None,
            repeat: None,
            landing_lag_s: None,
            autocancel_after_s: None,
            sprite_spin_hz: None,
            equips: None,
        }],
    }
}

#[cfg(test)]
mod clip_binding_tests {
    /// EVERY CANONICAL ROBOT MOVE ASKS FOR ITS OWN ROW.
    ///
    /// sprite redirect P1. All eleven passed `"attack"` as their clip, so a
    /// 132-row sheet drew ONE animation for a jab, three smashes and five
    /// aerials. The gameplay was already distinct; only the picture was not.
    ///
    /// this asserts the REQUEST, not the drawing. Whether a row exists is
    /// a question about a particular sheet and belongs to
    /// `SheetRecord::first_bound_row`; what a character ASKS FOR is a fact about
    /// the character, and it is the half that was missing.
    #[test]
    fn every_canonical_move_names_its_own_clip() {
        let moveset = super::player_robot_moveset();
        for (id, clip) in [
            ("jab", "jab"),
            ("tilt_up", "attack_up"),
            ("tilt_down", "attack_down"),
            ("smash_forward", "smash_forward"),
            ("smash_up", "smash_up"),
            ("smash_down", "smash_down"),
            ("air_neutral", "air_neutral"),
            ("air_forward", "air_forward"),
            ("air_back", "air_back"),
            ("air_up", "air_up"),
            ("air_down", "air_down"),
        ] {
            let spec = moveset
                .moves
                .iter()
                .find(|m| m.id == id)
                .unwrap_or_else(|| panic!("the robot authors no move `{id}`"));
            assert_eq!(
                spec.clip.clip, clip,
                "`{id}` asks for `{}` — a sheet that draws eleven distinct moves \
                 will draw one",
                spec.clip.clip
            );
            // and the chain must still reach a sheet that has none of them.
            assert!(
                spec.clip.fallbacks.iter().any(|f| f == "idle"),
                "`{id}` can fall all the way through to nothing"
            );
        }
    }
}

/// THE ROBOT'S CANONICAL REPERTOIRE — what actions it intrinsically HAS.
///
/// This lived in `default_player_action_set(abilities)`, a Rust function that built the
/// protagonist's kit from scratch on every call and gated it by the live `AbilitySet` in the same
/// expression. Two different questions shared one body of code: *what actions does this character
/// have* (a character fact, and the only one on this list) and *which of them are unlocked right
/// now* (runtime progression). `ActionSet::gated_by` is the second question's general form, so this
/// is free to be the first question's plain answer.
pub fn player_robot_action_set() -> ambition_characters::brain::ActionSet {
    use ambition_characters::brain::{
        ActionSet, MeleeActionSpec, MoveStyleSpec, RangedActionSpec, SpecialActionSpec, SwipeSpec,
    };
    ActionSet {
        melee: Some(MeleeActionSpec::Swipe(SwipeSpec {
            windup_s: 0.0,
            active_s: 0.10,
            recover_s: 0.18,
            damage: 1,
            reach_px: 36.0,
        })),
        // The Hadouken. HOW it fires — hold to build, release — is
        // `ranged_execution: ChargedProjectile` on the definition, not a property
        // of this slot: the slot says the robot throws something, the execution
        // says the throw charges.
        ranged: Some(RangedActionSpec::bolt(600.0, 1)),
        move_style: MoveStyleSpec::Walk,
        special: Some(SpecialActionSpec::Special("bubble_shield".to_string())),
    }
}
