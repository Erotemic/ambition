//! **The Shadow Oni Leader's repertoire** — the counter-puncher, written from
//! his own barks.
//!
//! ⭐ **the fourth adopter removed from the generic floor** (campaign P3.24), and
//! the first authored from a character's VOICE rather than from a
//! `gameplay_description`. His catalog row carries no design note; it carries
//! five lines, and they are a design note:
//!
//! > *"Your form is loud."* · *"A warning: one breath left."* · *"The shadow
//! > answers."* · *"I permit your gaze. Note the word 'permit'."* · *"A leader's
//! > hardest order is the one obeyed instantly."*
//!
//! Three of those decide this table and none of them is mine:
//!
//! * *the shadow ANSWERS* → he does not open. He replies, and he replies
//!   **faster than anybody on the grid** — the quickest startups in the game.
//! * *one breath left* → and the reply is a single instant. His **active
//!   windows are the shortest in the game**: you have to be exactly there,
//!   exactly then, or the answer passes through empty air.
//! * *the order obeyed instantly* → an order cannot be recalled. Every one of
//!   his moves carries **recovery of more than three times its own active
//!   window**, which is the longest commitment-to-payoff ratio on the grid.
//!
//! ```text
//!               startup   active   recovery      the trade
//!   goblin       0.04      0.05      0.12         fast and cheap
//!   oni leader   0.03      0.04      0.20         faster, and it COSTS
//! ```
//!
//! ⛔ **this is a different AXIS, not a fifth set of numbers.** The four tables
//! that exist vary reach, damage and speed together — a goblin is smaller and
//! quicker and weaker, an admiral longer and slower and harder. The oni varies
//! the SHAPE of a swing: same reach band as a striker, and a window you either
//! meet or miss. A character whose numbers only slide along the existing axis is
//! the previous table renumbered, which is what the comparative tests below
//! exist to refuse.
//!
//! ⚠ **the ninja fantasy is NOT in here** — no teleport, no clone, no smoke.
//! Those are abilities and techniques; a moveset says what a hit IS. Giving him
//! a vanish as a move window would be the wholesale-migration failure mode
//! wearing a content commit.

use ambition_characters::smash_capture::{
    author_pummel, author_standing_grab, author_throw, capture_beat, grab_shell,
    CaptureAttemptParams, CapturePummelParams, CaptureThrowParams, SmashCaptureRepertoire,
};
use ambition_characters::smash_repertoire::{DownSpecial, NeutralSpecial, SmashRepertoire};
use ambition_platformer2d::entity_catalog::MovesetContract;

use ambition_characters::moveset_authoring::{
    committed_tail, impulse, on_contact, sfx, strike, vfx_at,
};
use ambition_platformer2d::entity_catalog::ImpulseMode;

/// See the module doc. Eleven moves, the genre's standard verb map.
pub fn ninja_shadow_oni_leader_moveset() -> MovesetContract {
    // ── grounded ─────────────────────────────────────────────────────────────
    //
    // ⭐ the fastest jab in the game, and the shortest. It answers a goblin's
    // jab and beats it — and if the goblin was not there, the oni is standing
    // still for a fifth of a second holding an empty hand.
    let jab = strike(
        "jab",
        "jab",
        0.03,
        0.04,
        0.20,
        (24.0, 0.0),
        (17.0, 13.0),
        3,
        50.0,
        1.05,
        None,
        None,
    );

    let up_tilt = strike(
        "tilt_up",
        "attack_up",
        0.05,
        0.04,
        0.24,
        (10.0, -26.0),
        (17.0, 20.0),
        5,
        72.0,
        1.30,
        Some((0.1, -1.0)),
        None,
    );

    let down_tilt = strike(
        "tilt_down",
        "attack_down",
        0.05,
        0.04,
        0.22,
        (24.0, 13.0),
        (19.0, 10.0),
        4,
        58.0,
        1.18,
        Some((1.0, -0.22)),
        None,
    );

    // ── smashes ──────────────────────────────────────────────────────────────
    //
    // ⚠ **the FASTEST smashes on the grid and the most punishing to miss**, which
    // is the whole character in one pair of numbers. Everybody else's kill move
    // is slow to start; his is slow to *end*. A goblin that eats it was caught
    // reacting; a goblin that saw it coming gets 0.44s to answer.
    let mut f_smash = strike(
        "smash_forward",
        "smash_forward",
        0.20,
        0.05,
        0.44,
        (36.0, -2.0),
        (26.0, 19.0),
        16,
        150.0,
        3.05,
        Some((1.0, -0.42)),
        None,
    );
    f_smash.smash_charge_mult = 1.7;

    let mut up_smash = strike(
        "smash_up",
        "smash_up",
        0.18,
        0.05,
        0.42,
        (6.0, -32.0),
        (22.0, 28.0),
        15,
        148.0,
        2.90,
        Some((0.0, -1.0)),
        None,
    );
    up_smash.smash_charge_mult = 1.7;

    let mut down_smash = strike(
        "smash_down",
        "smash_down",
        0.16,
        0.06,
        0.46,
        (0.0, 15.0),
        (34.0, 12.0),
        13,
        132.0,
        2.70,
        Some((0.95, -0.50)),
        None,
    );
    down_smash.smash_charge_mult = 1.7;

    // ── aerials ──────────────────────────────────────────────────────────────
    let n_air = strike(
        "air_neutral",
        "air_neutral",
        0.04,
        0.06,
        0.22,
        (0.0, 0.0),
        (23.0, 21.0),
        5,
        66.0,
        1.28,
        None,
        None,
    );

    let f_air = strike(
        "air_forward",
        "air_forward",
        0.06,
        0.05,
        0.24,
        (28.0, -2.0),
        (21.0, 17.0),
        8,
        98.0,
        1.85,
        Some((1.0, -0.30)),
        None,
    );

    let b_air = strike(
        "air_back",
        "air_back",
        0.07,
        0.04,
        0.28,
        (-30.0, 0.0),
        (21.0, 17.0),
        11,
        128.0,
        2.45,
        Some((-1.0, -0.36)),
        None,
    );

    let u_air = strike(
        "air_up",
        "air_up",
        0.04,
        0.05,
        0.22,
        (2.0, -28.0),
        (19.0, 21.0),
        6,
        84.0,
        1.75,
        Some((0.0, -1.0)),
        None,
    );

    let d_air = strike(
        "air_down",
        "air_down",
        0.08,
        0.05,
        0.30,
        (5.0, 26.0),
        (19.0, 19.0),
        10,
        118.0,
        2.20,
        Some((0.0, 1.0)),
        None,
    );

    // ── 2026-08-16: THE FIVE THAT WERE MISSING ───────────────────────────────
    //
    // Jon: *"Let's complete the kit for all characters … We can invent whatever
    // special we want for oni and goblin … I think oni has a bunch of sfx and
    // vfx ready for it."* He does: fourteen authored rows on his own FX sheet,
    // and this table had never named ONE of them. Every effect below is his.
    //
    // ⭐ **the axis holds through all five.** Fastest to start, shortest active
    // window, recovery of more than three times it — a special that opened
    // slowly or lingered would be a different character wearing the mask.

    // ⛔ **the forward tilt, which fell down the chain to the jab.** `missed_
    // answer_cut` is the row for it: the answer that goes through where you were.
    let f_tilt = strike(
        "tilt_forward",
        "attack_side",
        0.04,
        0.04,
        0.22,
        (30.0, -2.0),
        (20.0, 13.0),
        5,
        66.0,
        1.22,
        Some((1.0, -0.28)),
        None,
    );
    let f_tilt = vfx_at(f_tilt, 0.04, "missed_answer_cut", (30.0, -2.0), 0.9);
    let f_tilt = sfx(f_tilt, 0.04, "enemy.shadow_oni.slash");
    let f_tilt = on_contact(f_tilt, "player.hit");

    // **NEUTRAL — `shadow_answer`.** The bark, as a move: *"The shadow answers."*
    // The shortest active window on the grid, four frames of eye-flash telegraph
    // in front of it, and a recovery you pay whether or not it landed.
    let n_b = strike(
        "shadow_answer",
        "attack",
        0.06,
        0.04,
        0.34,
        (26.0, -4.0),
        (26.0, 22.0),
        12,
        112.0,
        2.05,
        Some((0.9, -0.50)),
        None,
    );
    let n_b = committed_tail(n_b, 0.62, 0.0);
    let n_b = vfx_at(n_b, 0.02, "oni_eye_flash", (0.0, -10.0), 0.8);
    let n_b = sfx(n_b, 0.02, "enemy.shadow_oni.alert");
    let n_b = vfx_at(n_b, 0.06, "shadow_answer_slash", (26.0, -4.0), 1.15);
    let n_b = sfx(n_b, 0.06, "enemy.shadow_oni.slash");
    let n_b = on_contact(n_b, "player.hit");

    // **SIDE — `iaijutsu`.** The draw and the cut are one motion, so the impulse
    // and the active window are the same instant. He crosses the distance
    // already having swung.
    let side_b = strike(
        "iaijutsu",
        "attack_side",
        0.05,
        0.05,
        0.30,
        (34.0, 0.0),
        (30.0, 16.0),
        11,
        106.0,
        1.95,
        Some((0.95, -0.35)),
        None,
    );
    let side_b = impulse(side_b, 0.05, (700.0, 0.0), ImpulseMode::Set);
    let side_b = committed_tail(side_b, 0.58, 0.0);
    let side_b = vfx_at(side_b, 0.01, "silent_step", (0.0, 14.0), 0.8);
    let side_b = vfx_at(side_b, 0.05, "iaijutsu_glint", (34.0, 0.0), 1.0);
    let side_b = sfx(side_b, 0.05, "enemy.shadow_oni.slash");
    let side_b = on_contact(side_b, "player.hit");

    // **UP — `smoke_fold`. THE RECOVERY**, and the reason this batch is not
    // cosmetic: with no special at all he had a double jump and nothing else.
    // He does not climb — he leaves, and arrives. ⭐ the hit is on the ARRIVAL,
    // not the departure, so covering the spot he left is not a punish.
    let mut up_b = strike(
        "smoke_fold",
        "attack_up",
        0.05,
        0.05,
        0.26,
        (0.0, -8.0),
        (20.0, 30.0),
        8,
        84.0,
        1.60,
        Some((0.15, -1.0)),
        None,
    );
    up_b.landing_lag_s = Some(0.30);
    let up_b = impulse(up_b, 0.05, (0.0, -780.0), ImpulseMode::Set);
    let up_b = committed_tail(up_b, 0.52, 0.0);
    let up_b = vfx_at(up_b, 0.0, "blink_depart", (0.0, 0.0), 1.0);
    let up_b = sfx(up_b, 0.0, "enemy.shadow_oni.vanish");
    let up_b = vfx_at(up_b, 0.05, "smoke_fold", (0.0, 4.0), 1.1);
    let up_b = sfx(up_b, 0.05, "faction.ninja.smoke_poof");
    let up_b = vfx_at(up_b, 0.10, "blink_arrive", (0.0, -8.0), 1.0);
    let up_b = on_contact(up_b, "player.hit");

    // **DOWN — `command_seal`.** *"A leader's hardest order is the one obeyed
    // instantly."* He plants a seal and the ring closes on it: no displacement,
    // no reach, and the longest tail he owns. The order is given; standing there
    // while it is obeyed is the cost.
    let down_b = strike(
        "command_seal",
        "attack_down",
        0.06,
        0.05,
        0.36,
        (0.0, 6.0),
        (38.0, 26.0),
        10,
        98.0,
        1.75,
        Some((0.65, -0.70)),
        None,
    );
    let down_b = committed_tail(down_b, 0.70, 0.0);
    let down_b = vfx_at(down_b, 0.0, "command_seal", (0.0, 10.0), 1.0);
    let down_b = sfx(down_b, 0.0, "enemy.shadow_oni.alert");
    let down_b = vfx_at(down_b, 0.06, "counter_ring", (0.0, 6.0), 1.2);
    let down_b = sfx(down_b, 0.06, "faction.ninja.parry_flash");
    let down_b = on_contact(down_b, "player.hit");

    // ── 2026-08-16: THE OTHER POSTURE ────────────────────────────────────────
    //
    // Jon: *"A down-b that has special airborne properties should also have an
    // effect on ground. Think of bowser down b. In the air he just does a
    // downward slam, but on the ground, it causes him to jump in an arc and then
    // slam. Specials can have different effects in different contexts that
    // should be ok, and makes for a richer smash game, although in most cases
    // they shouldn't be context dependent."*
    //
    // ⛔ a special gated to ONE posture is not answered in the other — the
    // directional chain walks straight past it to the NEUTRAL special, so a
    // player pressing down-B in the air got the neutral-B. `special_air_down`
    // sits ahead of `special_down` in that chain and has the whole time; this is
    // the two-form move it exists for.
    // **DOWN, IN THE AIR — `falling_seal`.** The order is given on the way
    // down. Same seal, no floor to plant it on, so it closes around him as he
    // drops — and he arrives with it.
    let mut air_down_b = strike(
        "falling_seal",
        "air_down",
        0.05,
        0.05,
        0.28,
        (0.0, 22.0),
        (24.0, 22.0),
        9,
        92.0,
        1.70,
        Some((0.0, 1.0)),
        None,
    );
    air_down_b.landing_lag_s = Some(0.28);
    let air_down_b = impulse(air_down_b, 0.05, (0.0, 1250.0), ImpulseMode::Set);
    let air_down_b = vfx_at(air_down_b, 0.0, "command_seal", (0.0, 0.0), 0.9);
    let air_down_b = vfx_at(air_down_b, 0.05, "counter_ring", (0.0, 18.0), 1.0);
    let air_down_b = sfx(air_down_b, 0.05, "faction.ninja.parry_flash");
    let air_down_b = on_contact(air_down_b, "player.hit");

    // **ONI'S CAPTURE KIT.** The FASTEST grab in the game and the longest recovery
    // behind it — a read, not a poke. Whiffing this is the punish window his whole kit
    // is balanced around.
    // ⚠ the grab draws `attack`, not `grab`: these sheets publish no `grab` row,
    // and each table's own `every_clip_names_a_row_..._sheet_carries` guard says
    // so. `ClipBinding`'s fallbacks would have covered it at runtime, but a move
    // that NAMES a row nobody publishes is a lie the guard is right to refuse.
    let grab = author_standing_grab(
        grab_shell("oni_grab", "attack", 0.05, 0.04, 0.26),
        CaptureAttemptParams {
            offset: (12.0, 1.0),
            half_extents: (21.0, 16.0),
            hold_offset: (13.0, 3.0),
        },
    );
    let pummel = author_pummel(
        capture_beat("oni_pummel", "attack", 0.15),
        0.06,
        CapturePummelParams { damage: 3 },
    );
    let forward_throw = author_throw(
        capture_beat("oni_fthrow", "attack", 0.25),
        0.12,
        CaptureThrowParams {
            damage: 9,
            knockback: 126.0,
            knockback_growth: 2.0,
            launch_dir: (0.8, -0.6),
        },
    );
    SmashRepertoire {
        taunt: ambition_characters::moveset_authoring::taunt("ninja_shadow_oni_leader_taunt", 0.9),
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
        neutral_special: NeutralSpecial::Authored(n_b),
        side_special: side_b,
        up_special: up_b,
        // ⭐ **AUTHORED 2026-08-19, at Jon's ask that every fighter in the smash
        // roster have a grab.** The transitional `None` is gone: capture was
        // proven on George and the Pirate Admiral, and the whole point of
        // proving it was to stop being the only two.
        //
        // ⚠ the VALUES are per character on purpose. A roster whose grabs are
        // twelve copies of one number set is one grab wearing twelve names.
        capture: SmashCaptureRepertoire {
            grab,
            pummel,
            forward_throw,
            // ⛔ back/up/down stay `None` and that is still the authored answer,
            // not an omission: an unauthored throw does NOTHING rather than
            // falling back to a pummel, which tells a player this fighter has
            // none instead of telling them it has a bad one.
            back_throw: None,
            up_throw: None,
            down_throw: None,
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
    use ambition_platformer2d::entity_catalog::{MoveSpec, MoveWindow, WindowTag};

    fn find(set: &MovesetContract, id: &str) -> MoveSpec {
        set.moves
            .iter()
            .find(|m| m.id == id)
            .unwrap_or_else(|| panic!("{id} exists"))
            .clone()
    }

    /// ⚠ **only a STRIKE has one.** A pummel and a throw are timelines with no
    /// Active window at all, so the swing tests below iterate `strikes()` rather
    /// than every move the contract carries — asking a throw for its startup is
    /// asking about a window it never has.
    fn active(m: &MoveSpec) -> &MoveWindow {
        m.windows
            .iter()
            .find(|w| matches!(w.tag, WindowTag::Active))
            .expect("a strike has an active window")
    }

    /// Every move that actually swings.
    fn strikes(set: &MovesetContract) -> Vec<&MoveSpec> {
        set.moves
            .iter()
            .filter(|m| m.windows.iter().any(|w| matches!(w.tag, WindowTag::Active)))
            .collect()
    }

    fn startup(m: &MoveSpec) -> f32 {
        active(m).start_s
    }

    fn active_len(m: &MoveSpec) -> f32 {
        let w = active(m);
        w.end_s - w.start_s
    }

    fn recovery(m: &MoveSpec) -> f32 {
        m.duration_s - active(m).end_s
    }

    // ⭐⭐ **RETIRED 2026-08-16 — `every_bound_verb_names_a_move_that_exists`.**
    //
    // Fourteen fighters each carried a copy of it: every bound verb names a move
    // this table defines, and the table binds the whole vocabulary. Both are now
    // unwritable defects rather than tested ones. `SmashRepertoire` owns the verb
    // strings, so there is no string in this file to misspell; it is a struct
    // with no `Default` and no private fields, so a missing or renamed slot is a
    // COMPILE error here. What the fourteen copies stood for — that every press
    // is answered, in every posture it is asked in — is checked once, by
    // `ambition_characters::smash_repertoire`, and by the host ratchet
    // `smash_roster_movesets::report_the_smash_kit_every_selectable_fighter_has`.

    /// **THE SHADOW ANSWERS: he is quicker to start than the quickest body that
    /// already had a table, and quicker to finish answering than any of them.**
    ///
    /// Comparative against the GOBLIN, which is the fast one — measuring against
    /// the admiral or the clerk would make "fast" mean "not a heavyweight" and
    /// prove nothing.
    #[test]
    fn he_answers_faster_and_for_less_time_than_the_goblin() {
        let oni = ninja_shadow_oni_leader_moveset();
        let goblin = crate::goblin_moveset::goblin_moveset();

        assert!(
            startup(&find(&oni, "jab")) < startup(&find(&goblin, "jab")),
            "the shadow answers first"
        );

        let longest = |set: &MovesetContract| {
            strikes(set)
                .into_iter()
                .map(|m| active_len(m))
                .fold(0.0f32, f32::max)
        };
        assert!(
            longest(&oni) < longest(&goblin),
            "and his widest window is still narrower than the goblin's ({} vs {}) \
             — one breath, and you are either in it or you are not",
            longest(&oni),
            longest(&goblin)
        );
    }

    /// **THE ORDER OBEYED INSTANTLY CANNOT BE RECALLED: every move recovers for
    /// more than three times its own active window.**
    ///
    /// ⭐ this is the axis, and it is what stops the table being the goblin's
    /// with smaller numbers. A fighter can be given fast startups by typing
    /// smaller floats; a fighter whose every swing costs more than triple what it
    /// buys has a different relationship to committing.
    ///
    /// ⛔ **the poison is that the GOBLIN must fail this**, or the ratio is a
    /// property of `strike`'s shape rather than a property of him.
    #[test]
    fn every_swing_costs_more_than_three_times_the_moment_it_buys() {
        let oni = ninja_shadow_oni_leader_moveset();
        // ⚠ SWINGS only. The claim is about what a swing costs, and a pummel or
        // a throw is not a swing — it holds no window to be three times longer
        // than. ⛔ the count is the zero floor: a filter that removed everything
        // would satisfy this loop by iterating nothing.
        let swings = strikes(&oni);
        assert!(
            swings.len() >= 16,
            "only {} of his moves swing at all — this is being asserted over a \
             population that shrank",
            swings.len()
        );
        for m in swings {
            assert!(
                recovery(m) > active_len(m) * 3.0,
                "`{}` recovers {}s for an active window of {}s — under 3x, which \
                 is a swing he could throw casually",
                m.id,
                recovery(m),
                active_len(m)
            );
        }

        let goblin = crate::goblin_moveset::goblin_moveset();
        assert!(
            goblin
                .moves
                .iter()
                .any(|m| recovery(m) <= active_len(m) * 3.0),
            "the goblin is supposed to have cheap swings; if every table passes \
             this, the ratio describes `strike` rather than the oni leader"
        );
    }

    /// **And he is not simply BETTER.** The fast answer is paid for: his kill
    /// move commits longer after the fact than the admiral's does, and the
    /// admiral is the slow one.
    #[test]
    fn his_kill_move_commits_longer_than_the_admirals() {
        let oni = ninja_shadow_oni_leader_moveset();
        let admiral = crate::pirate_admiral_moveset::pirate_admiral_moveset();
        let (o, a) = (find(&oni, "smash_forward"), find(&admiral, "smash_forward"));
        assert!(
            startup(&o) < startup(&a),
            "he starts his finisher first ({} vs {})",
            startup(&o),
            startup(&a)
        );
        assert!(
            recovery(&o) > recovery(&a),
            "and stands in it longer afterwards ({} vs {}) — the reply is instant \
             and the price is paid at the other end",
            recovery(&o),
            recovery(&a)
        );
    }
}
