//! **George Booul's repertoire** — the excluded middle, as a fighter.
//!
//! ⭐ **this demo's own character finally swings like itself.** All three
//! registered fighters took [`crate::moveset::fighter_moveset`] verbatim, and
//! that file says so in its own doc: *"Shared by this demo's three fighters
//! today. That is a content decision, not an architectural one: the moveset
//! rides the CHARACTER, so giving George a heavier one is editing his definition
//! and nothing else."* This is that edit, and nothing else changed.
//!
//! ⚠ **the other two keep the shared table on purpose.** `smash_duelist_a/b` are
//! STAND-INS for `player_robot_v3`/`v2` — characters whose canonical repertoire
//! lives on the real Robot provider (redirect §15) and reaches them the moment a
//! host composes it. Authoring a third robot table here would be the copy that
//! redirect exists to forbid. George is the one fighter this demo actually owns.
//!
//! ## The character, from his own row
//!
//! > *"Either you are on the stage or you are not."*
//!
//! A logician, and the heaviest body on the grid (`knockback_weight: 1.35`). The
//! line is the law of the excluded middle, and it is the whole table:
//!
//! ```text
//!                    startup      damage
//!   jab, n-air, u-air   0.05–0.07    3–4      the pokes
//!            ↑ nothing at all lives in here ↑
//!   everything else     0.16–0.40   11–21     the commitments
//! ```
//!
//! George has three fast options and eight slow ones and **no middle** — not
//! even his tilts, which for every other fighter in the genre are the safe
//! in-between. A fighter with no medium answer has to decide, every exchange,
//! which of the two things he is doing; that is what a body built out of a
//! disjunction plays like.
//!
//! ⛔ **not the shared table renumbered**, and the tests below are comparative
//! for exactly that reason: heavier than it on every smash, faster nowhere, and
//! carrying a startup GAP the shared table does not have.

use ambition_platformer2d::entity_catalog::MovesetContract;

use crate::moveset::{airborne_only, grounded_only, strike};

/// The widest startup a POKE may have, and the narrowest a COMMITMENT may have.
///
/// ⚠ these are the character, not tuning constants that happen to bracket the
/// numbers: the gap between them is the excluded middle, and the guard asserts
/// no move lands inside it. Retuning George means moving a move to one side or
/// the other, never into the band.
const POKE_MAX_STARTUP_S: f32 = 0.08;
const COMMIT_MIN_STARTUP_S: f32 = 0.15;

/// See the module doc. Eleven moves, the genre's standard verb map.
pub fn george_booul_moveset() -> MovesetContract {
    let mut moves = Vec::new();

    // ── the three pokes ──────────────────────────────────────────────────────
    //
    // Everything George owns that comes out quickly is also nearly harmless. He
    // is not paid for these; they exist so that "not committing" is a legal
    // move rather than standing still.
    let mut jab = strike(
        "jab",
        "attack",
        0.05,
        0.05,
        0.15,
        (26.0, 0.0),
        (18.0, 14.0),
        3,
        50.0,
        1.05,
        None,
    );
    jab.gates = grounded_only();
    moves.push(jab);

    let mut n_air = strike(
        "air_neutral",
        "attack",
        0.06,
        0.12,
        0.18,
        (0.0, 0.0),
        (26.0, 24.0),
        4,
        65.0,
        1.20,
        None,
    );
    n_air.gates = airborne_only();
    n_air.landing_lag_s = Some(0.16);
    n_air.autocancel_after_s = Some(0.24);
    moves.push(n_air);

    let mut u_air = strike(
        "air_up",
        "attack",
        0.07,
        0.09,
        0.19,
        (2.0, -32.0),
        (20.0, 24.0),
        4,
        70.0,
        1.35,
        Some((0.0, -1.0)),
    );
    u_air.gates = airborne_only();
    u_air.landing_lag_s = Some(0.16);
    u_air.autocancel_after_s = Some(0.26);
    moves.push(u_air);

    // ── the tilts, which for George are COMMITMENTS ──────────────────────────
    //
    // ⭐ this is the single most character-defining pair in the table. A tilt is
    // the genre's safe middle option everywhere else — the poke you throw when
    // you do not want to decide. George does not have one. His up-tilt starts
    // more than twice as late as the shared table's and hits more than twice as
    // hard, which is the same trade every one of his slow moves makes.
    let mut up_tilt = strike(
        "tilt_up",
        "attack",
        0.16,
        0.09,
        0.26,
        (10.0, -30.0),
        (24.0, 28.0),
        11,
        130.0,
        2.20,
        Some((0.1, -1.0)),
    );
    up_tilt.gates = grounded_only();
    moves.push(up_tilt);

    let mut down_tilt = strike(
        "tilt_down",
        "attack",
        0.17,
        0.08,
        0.28,
        (30.0, 14.0),
        (26.0, 11.0),
        11,
        135.0,
        2.30,
        Some((1.0, -0.20)),
    );
    down_tilt.gates = grounded_only();
    moves.push(down_tilt);

    // ── the smashes ──────────────────────────────────────────────────────────
    //
    // ⚠ the slowest and hardest in this composition, on a body that already
    // survives longest. That is deliberate and it is the risk: a heavyweight
    // who also lands the biggest hits is only fair because he can never throw
    // one without being seen doing it.
    let mut f_smash = strike(
        "smash_forward",
        "attack",
        0.40,
        0.08,
        0.46,
        (46.0, -4.0),
        (32.0, 24.0),
        21,
        185.0,
        3.45,
        Some((1.0, -0.44)),
    );
    f_smash.gates = grounded_only();
    f_smash.smash_charge_mult = 1.7;
    moves.push(f_smash);

    let mut up_smash = strike(
        "smash_up",
        "attack",
        0.36,
        0.10,
        0.42,
        (6.0, -38.0),
        (26.0, 34.0),
        19,
        178.0,
        3.30,
        Some((0.0, -1.0)),
    );
    up_smash.gates = grounded_only();
    up_smash.smash_charge_mult = 1.7;
    moves.push(up_smash);

    let mut down_smash = strike(
        "smash_down",
        "attack",
        0.34,
        0.11,
        0.44,
        (0.0, 16.0),
        (44.0, 13.0),
        17,
        165.0,
        3.05,
        Some((0.95, -0.45)),
    );
    down_smash.gates = grounded_only();
    down_smash.smash_charge_mult = 1.7;
    moves.push(down_smash);

    // ── the committed aerials ────────────────────────────────────────────────
    //
    // Three of his five aerials are on the slow side of the gap, with landing
    // lag to match. Jumping is not an escape for this body; it is another
    // decision.
    let mut f_air = strike(
        "air_forward",
        "attack",
        0.18,
        0.09,
        0.28,
        (34.0, -2.0),
        (26.0, 20.0),
        12,
        140.0,
        2.35,
        Some((1.0, -0.30)),
    );
    f_air.gates = airborne_only();
    f_air.landing_lag_s = Some(0.24);
    f_air.autocancel_after_s = Some(0.34);
    moves.push(f_air);

    let mut b_air = strike(
        "air_back",
        "attack",
        0.20,
        0.08,
        0.30,
        (-36.0, 0.0),
        (26.0, 20.0),
        14,
        155.0,
        2.75,
        Some((-1.0, -0.36)),
    );
    b_air.gates = airborne_only();
    b_air.landing_lag_s = Some(0.26);
    b_air.autocancel_after_s = Some(0.36);
    moves.push(b_air);

    // The heaviest landing lag on the grid. A missed spike over the stage is a
    // free smash for whoever is standing under it — which, for a fighter whose
    // whole table is commitments, is the correct punishment.
    let mut d_air = strike(
        "air_down",
        "attack",
        0.22,
        0.09,
        0.32,
        (6.0, 32.0),
        (22.0, 22.0),
        15,
        150.0,
        2.55,
        Some((0.0, 1.0)),
    );
    d_air.gates = airborne_only();
    d_air.landing_lag_s = Some(0.34);
    d_air.autocancel_after_s = Some(0.44);
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

    // ⭐ **the disjunction is checked WHERE IT IS AUTHORED**, not only in the
    // test module. These two numbers are the character; a move edited into the
    // band between them stops being George's before anything else notices, and
    // the builder is the last place that still knows both halves at once.
    debug_assert!(
        moves.iter().all(|m| {
            let startup = m
                .windows
                .iter()
                .find(|w| {
                    matches!(
                        w.tag,
                        ambition_platformer2d::entity_catalog::WindowTag::Active
                    )
                })
                .map_or(0.0, |w| w.start_s);
            startup <= POKE_MAX_STARTUP_S || startup >= COMMIT_MIN_STARTUP_S
        }),
        "a George move landed between the pokes and the commitments"
    );

    MovesetContract { verbs, moves }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d::entity_catalog::{MoveSpec, WindowTag};

    fn find(set: &MovesetContract, id: &str) -> MoveSpec {
        set.moves
            .iter()
            .find(|m| m.id == id)
            .unwrap_or_else(|| panic!("{id} exists"))
            .clone()
    }

    fn startup(m: &MoveSpec) -> f32 {
        m.windows
            .iter()
            .find(|w| matches!(w.tag, WindowTag::Active))
            .expect("a strike has an active window")
            .start_s
    }

    fn damage(m: &MoveSpec) -> i32 {
        m.windows
            .iter()
            .flat_map(|w| w.volumes.iter())
            .map(|v| v.damage)
            .max()
            .unwrap_or(0)
    }

    /// **Every verb George binds resolves to a move that exists.**
    ///
    /// ⛔ a verb bound to a missing id is silence at the press: the runtime looks
    /// the move up, finds nothing, and the button does nothing at all.
    #[test]
    fn every_bound_verb_names_a_move_that_exists() {
        let moveset = george_booul_moveset();
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

    /// **THE EXCLUDED MIDDLE, AS AN ASSERTION.**
    ///
    /// ⭐ the claim the module doc makes and the one thing that cannot survive a
    /// careless retune: every move is a poke or a commitment, and the band
    /// between them is empty. A move that drifted into it would be a perfectly
    /// reasonable tilt and would quietly make George somebody else.
    #[test]
    fn no_move_lives_between_the_pokes_and_the_commitments() {
        let george = george_booul_moveset();
        for m in &george.moves {
            let s = startup(m);
            assert!(
                s <= POKE_MAX_STARTUP_S || s >= COMMIT_MIN_STARTUP_S,
                "`{}` starts at {s}s, inside the band this fighter does not have \
                 ({POKE_MAX_STARTUP_S}..{COMMIT_MIN_STARTUP_S})",
                m.id
            );
        }

        // ⭐ and the two halves are separated by PAYOFF, not only by timing —
        // otherwise "slow" would just mean "slow", and the disjunction would be
        // about the clock rather than about the decision.
        let (pokes, commits): (Vec<_>, Vec<_>) = george
            .moves
            .iter()
            .partition(|m| startup(m) <= POKE_MAX_STARTUP_S);
        let hardest_poke = pokes.iter().map(|m| damage(m)).max().expect("pokes exist");
        let softest_commit = commits
            .iter()
            .map(|m| damage(m))
            .min()
            .expect("commitments exist");
        assert!(
            hardest_poke < softest_commit,
            "the fast half must be the weak half ({hardest_poke} vs {softest_commit})"
        );

        // ⛔ **the poison.** The shared table has a real middle — its tilts sit
        // at 0.06–0.07 and its aerials climb through 0.09, 0.10, 0.12 — so if
        // this assertion ever passed for BOTH tables, the band would be
        // describing nothing.
        let shared = crate::moveset::fighter_moveset();
        assert!(
            shared.moves.iter().any(|m| {
                let s = startup(m);
                s > POKE_MAX_STARTUP_S && s < COMMIT_MIN_STARTUP_S
            }),
            "the shared repertoire is supposed to HAVE a middle; if it does not, \
             this whole test is asserting a property of the threshold rather \
             than a property of George"
        );
    }

    /// **The heavy is slower AND harder than the table he replaced** — on every
    /// smash, measured against it rather than against a literal.
    ///
    /// ⚠ comparative for the same reason the goblin's and the admiral's tests
    /// are: a table copied wholesale and renumbered would pass every other test
    /// in this file.
    #[test]
    fn george_commits_longer_and_hits_harder_than_the_shared_repertoire() {
        let george = george_booul_moveset();
        let shared = crate::moveset::fighter_moveset();
        for id in ["smash_forward", "smash_up", "smash_down"] {
            let (g, s) = (find(&george, id), find(&shared, id));
            assert!(
                startup(&g) > startup(&s),
                "`{id}`: the heavy commits longer ({} vs {})",
                startup(&g),
                startup(&s)
            );
            assert!(
                damage(&g) > damage(&s),
                "`{id}`: and is paid for it ({} vs {})",
                damage(&g),
                damage(&s)
            );
        }

        // And nowhere is he FASTER. A heavyweight that also had the quicker
        // option somewhere would just be stronger.
        for m in &george.moves {
            let s = find(&shared, &m.id);
            assert!(
                startup(m) >= startup(&s),
                "`{}` is quicker than the shared table's ({} vs {})",
                m.id,
                startup(m),
                startup(&s)
            );
        }
    }
}
