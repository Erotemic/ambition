//! **The goblin's repertoire** — the third character in the game to state its own
//! moves, and the first ENEMY to.
//!
//! ⭐ **this exists to remove an adopter from `smash_fighter_kit()`** (campaign
//! P3.24). That floor is one `simple_melee` swipe handed to every seated fighter
//! whose character says nothing, and its goal is DELETION: the count falls by one
//! each time somebody writes a table. The goblin is the cheapest next one —
//! Ambition's own, already on the grid, already authoring its body (170 px/s,
//! 5 HP, 0.70 contact) and its controller policy (the shared `medium_striker`).
//!
//! ⚠ **it is NOT the robot's table with different numbers.** A goblin is small,
//! fast and scrappy: it gets in, it pokes, and its punish window is short because
//! it cannot afford a long one. Against the robot that reads as
//!
//! ```text
//!            reach     jab startup   f-smash damage   b-air
//!   robot     26 px       0.05 s          15          10
//!   goblin    22 px       0.04 s          12          9
//! ```
//!
//! — a shorter, faster, weaker fighter that has to be closer to matter. The
//! differences are the authored ones; the SHAPE is `strike`'s, which is the whole
//! reason that helper was pulled out of the robot's file.
//!
//! ⛔ **the clip names are the standard vocabulary and the fallback chain does the
//! rest.** The goblin sheet does not have 132 rows; `strike` names `smash_forward`
//! and settles for `attack_side`, then `attack`, then `slash`, then `idle`. A
//! missing clip costs the move its picture, never its gameplay.

use ambition_platformer2d::entity_catalog::MovesetContract;

use crate::moveset_authoring::{airborne_only, grounded_only, strike};

/// See the module doc. Eleven moves, the genre's standard verb map.
pub fn goblin_moveset() -> MovesetContract {
    let mut moves = Vec::new();

    // ── grounded ─────────────────────────────────────────────────────────────
    //
    // Faster than the robot's and weaker, which is the whole character in one
    // move: the goblin's jab is the thing it throws while walking into you.
    let mut jab = strike(
        "jab",
        "jab",
        0.04,
        0.05,
        0.12,
        (22.0, 0.0),
        (16.0, 13.0),
        2,
        45.0,
        1.05,
        None,
        None,
    );
    jab.gates = grounded_only();
    moves.push(jab);

    // An upward poke that beats a shorthop. Small volume — it is an
    // anti-air, not a wall.
    let mut up_tilt = strike(
        "tilt_up",
        "attack_up",
        0.07,
        0.07,
        0.16,
        (10.0, -24.0),
        (16.0, 18.0),
        4,
        70.0,
        1.30,
        Some((0.15, -1.0)),
        None,
    );
    up_tilt.gates = grounded_only();
    moves.push(up_tilt);

    // Low and forward: the goblin's ground game is knee height, which is where a
    // small body's reach actually is.
    let mut down_tilt = strike(
        "tilt_down",
        "attack_down",
        0.06,
        0.06,
        0.15,
        (22.0, 12.0),
        (18.0, 10.0),
        3,
        50.0,
        1.15,
        Some((1.0, -0.25)),
        None,
    );
    down_tilt.gates = grounded_only();
    moves.push(down_tilt);

    // ── smashes ──────────────────────────────────────────────────────────────
    //
    // ⚠ committed and NOT safe. The goblin's kill move costs it 0.30s of
    // recovery against a body that only has 5 HP to trade with — throwing this
    // and missing is how a goblin dies, which is what makes landing it exciting
    // rather than routine.
    let mut f_smash = strike(
        "smash_forward",
        "smash_forward",
        0.28,
        0.06,
        0.30,
        (34.0, -2.0),
        (24.0, 18.0),
        12,
        135.0,
        2.85,
        Some((1.0, -0.40)),
        None,
    );
    f_smash.gates = grounded_only();
    f_smash.smash_charge_mult = 1.7;
    moves.push(f_smash);

    let mut up_smash = strike(
        "smash_up",
        "smash_up",
        0.24,
        0.07,
        0.28,
        (6.0, -30.0),
        (20.0, 26.0),
        11,
        140.0,
        2.70,
        Some((0.0, -1.0)),
        None,
    );
    up_smash.gates = grounded_only();
    up_smash.smash_charge_mult = 1.7;
    moves.push(up_smash);

    // Both sides, low — the goblin's answer to being surrounded, and its
    // ledge-guard.
    let mut down_smash = strike(
        "smash_down",
        "smash_down",
        0.22,
        0.08,
        0.32,
        (0.0, 14.0),
        (32.0, 12.0),
        10,
        125.0,
        2.55,
        Some((0.9, -0.55)),
        None,
    );
    down_smash.gates = grounded_only();
    down_smash.smash_charge_mult = 1.7;
    moves.push(down_smash);

    // ── aerials ──────────────────────────────────────────────────────────────
    let mut n_air = strike(
        "air_neutral",
        "air_neutral",
        0.05,
        0.10,
        0.13,
        (0.0, 0.0),
        (22.0, 20.0),
        4,
        60.0,
        1.25,
        None,
        None,
    );
    n_air.gates = airborne_only();
    moves.push(n_air);

    let mut f_air = strike(
        "air_forward",
        "air_forward",
        0.08,
        0.07,
        0.16,
        (26.0, -2.0),
        (20.0, 16.0),
        6,
        85.0,
        1.70,
        Some((1.0, -0.30)),
        None,
    );
    f_air.gates = airborne_only();
    moves.push(f_air);

    // ⭐ the goblin's best kill option, and it faces the wrong way — the classic
    // trade. Committing to a back-air means committing to not looking at them.
    let mut b_air = strike(
        "air_back",
        "air_back",
        0.10,
        0.06,
        0.20,
        (-28.0, 0.0),
        (20.0, 16.0),
        9,
        120.0,
        2.40,
        Some((-1.0, -0.35)),
        None,
    );
    b_air.gates = airborne_only();
    moves.push(b_air);

    let mut u_air = strike(
        "air_up",
        "air_up",
        0.06,
        0.08,
        0.14,
        (2.0, -26.0),
        (18.0, 20.0),
        5,
        80.0,
        1.80,
        Some((0.0, -1.0)),
        None,
    );
    u_air.gates = airborne_only();
    moves.push(u_air);

    // Straight down and hard. ⚠ no `on_hit` rebound: the robot's down-air says it
    // is capable of bouncing its attacker and this one does not, because a goblin
    // that could pogo off a body would out-recover a character built around
    // recovery being its problem.
    let mut d_air = strike(
        "air_down",
        "air_down",
        0.11,
        0.07,
        0.22,
        (4.0, 24.0),
        (18.0, 18.0),
        8,
        110.0,
        2.10,
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

    /// **Every verb the goblin binds resolves to a move that exists.**
    ///
    /// ⛔ a verb bound to a missing id is silence at the press — the runtime looks
    /// the move up, finds nothing, and the button does nothing at all. It is the
    /// one defect in a move table that cannot be seen by reading it.
    #[test]
    fn every_bound_verb_names_a_move_that_exists() {
        let moveset = goblin_moveset();
        let ids: std::collections::BTreeSet<&str> =
            moveset.moves.iter().map(|m| m.id.as_str()).collect();
        for (verb, id) in &moveset.verbs {
            assert!(
                ids.contains(id.as_str()),
                "verb `{verb}` binds move `{id}`, which this table does not define"
            );
        }
        assert_eq!(
            moveset.verbs.len(),
            11,
            "the standard ground+air vocabulary is eleven verbs; a smaller table \
             means a press somewhere falls through to nothing"
        );
    }

    /// **The goblin is not the robot with different numbers.**
    ///
    /// ⭐ the point of a per-character table is that the characters differ, and a
    /// table copied wholesale would pass every other test in this file. This pins
    /// the identity the module doc claims: shorter reach, faster jab, weaker kill.
    #[test]
    fn the_goblin_is_shorter_faster_and_weaker_than_the_robot() {
        let goblin = goblin_moveset();
        let robot = crate::player_robot_moveset::player_robot_moveset();
        let find = |set: &MovesetContract, id: &str| {
            set.moves
                .iter()
                .find(|m| m.id == id)
                .unwrap_or_else(|| panic!("{id} exists"))
                .clone()
        };

        let (g_jab, r_jab) = (find(&goblin, "jab"), find(&robot, "jab"));
        let startup = |m: &ambition_platformer2d::entity_catalog::MoveSpec| {
            m.windows
                .iter()
                .find(|w| {
                    matches!(
                        w.tag,
                        ambition_platformer2d::entity_catalog::WindowTag::Active
                    )
                })
                .expect("a strike has an active window")
                .start_s
        };
        assert!(
            startup(&g_jab) < startup(&r_jab),
            "the goblin's jab comes out faster"
        );

        let reach = |m: &ambition_platformer2d::entity_catalog::MoveSpec| {
            m.windows
                .iter()
                .flat_map(|w| w.volumes.iter())
                .map(|v| match v.shape {
                    ambition_platformer2d::entity_catalog::VolumeShape::Rect {
                        offset,
                        half_extents,
                    } => offset.0.abs() + half_extents.0,
                    _ => 0.0,
                })
                .fold(0.0f32, f32::max)
        };
        assert!(
            reach(&g_jab) < reach(&r_jab),
            "and it reaches less far, which is what makes it have to get close"
        );

        let damage = |m: &ambition_platformer2d::entity_catalog::MoveSpec| {
            m.windows
                .iter()
                .flat_map(|w| w.volumes.iter())
                .map(|v| v.damage)
                .max()
                .unwrap_or(0)
        };
        assert!(
            damage(&find(&goblin, "smash_forward")) < damage(&find(&robot, "smash_forward")),
            "and its kill move hits softer — a small fighter trades reach and \
             power for speed, or it is just the robot in a different sheet"
        );
    }
}
