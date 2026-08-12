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

use ambition_platformer2d::entity_catalog::MovesetContract;

use crate::moveset_authoring::{airborne_only, grounded_only, strike};

/// See the module doc. Eleven moves, the genre's standard verb map.
pub fn ninja_shadow_oni_leader_moveset() -> MovesetContract {
    let mut moves = Vec::new();

    // ── grounded ─────────────────────────────────────────────────────────────
    //
    // ⭐ the fastest jab in the game, and the shortest. It answers a goblin's
    // jab and beats it — and if the goblin was not there, the oni is standing
    // still for a fifth of a second holding an empty hand.
    let mut jab = strike(
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
    jab.gates = grounded_only();
    moves.push(jab);

    let mut up_tilt = strike(
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
    up_tilt.gates = grounded_only();
    moves.push(up_tilt);

    let mut down_tilt = strike(
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
    down_tilt.gates = grounded_only();
    moves.push(down_tilt);

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
    f_smash.gates = grounded_only();
    f_smash.smash_charge_mult = 1.7;
    moves.push(f_smash);

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
    up_smash.gates = grounded_only();
    up_smash.smash_charge_mult = 1.7;
    moves.push(up_smash);

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
    down_smash.gates = grounded_only();
    down_smash.smash_charge_mult = 1.7;
    moves.push(down_smash);

    // ── aerials ──────────────────────────────────────────────────────────────
    let mut n_air = strike(
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
    n_air.gates = airborne_only();
    moves.push(n_air);

    let mut f_air = strike(
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
    f_air.gates = airborne_only();
    moves.push(f_air);

    let mut b_air = strike(
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
    b_air.gates = airborne_only();
    moves.push(b_air);

    let mut u_air = strike(
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
    u_air.gates = airborne_only();
    moves.push(u_air);

    let mut d_air = strike(
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
    d_air.gates = airborne_only();
    moves.push(d_air);

    let verbs = [
        ("attack", "jab"),
        ("attack_up", "tilt_up"),
        ("attack_down", "tilt_down"),
        ("smash_forward", "smash_forward"),
        ("smash_up", "smash_up"),
        ("smash_down", "smash_down"),
        ("attack_air", "air_neutral"),
        ("attack_air_forward", "air_forward"),
        ("attack_air_back", "air_back"),
        ("attack_air_up", "air_up"),
        ("attack_air_down", "air_down"),
    ]
    .into_iter()
    .map(|(verb, id)| (verb.to_string(), id.to_string()))
    .collect();

    MovesetContract { verbs, moves }
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

    fn active(m: &MoveSpec) -> &MoveWindow {
        m.windows
            .iter()
            .find(|w| matches!(w.tag, WindowTag::Active))
            .expect("a strike has an active window")
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

    /// **Every verb he binds resolves to a move that exists.**
    #[test]
    fn every_bound_verb_names_a_move_that_exists() {
        let moveset = ninja_shadow_oni_leader_moveset();
        let ids: std::collections::BTreeSet<&str> =
            moveset.moves.iter().map(|m| m.id.as_str()).collect();
        for (verb, id) in &moveset.verbs {
            assert!(
                ids.contains(id.as_str()),
                "verb `{verb}` binds move `{id}`, which this table does not define"
            );
        }
        assert_eq!(moveset.verbs.len(), 11);
    }

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

        let longest =
            |set: &MovesetContract| set.moves.iter().map(active_len).fold(0.0f32, f32::max);
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
        for m in &oni.moves {
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
