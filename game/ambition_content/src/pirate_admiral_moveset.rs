//! **The Pirate Admiral's repertoire** — a cutlass, and the reach that comes with
//! carrying one.
//!
//! ⭐ **the second adopter removed from `smash_fighter_kit()`** (campaign P3.24),
//! after the goblin. That floor's goal is DELETION and the count only moves when
//! somebody writes a table.
//!
//! ⚠ **the character was already telling us what its moves are.** Its catalog row
//! says `default_action_set: "pirate_pistol"` and the roster comment beside its id
//! reads *"pistol + cutlass"*; its sprite is authored at `collision_scale: 1.6`,
//! the largest of the three fighters with tables. So: a big body with a long blade
//! — slower than the goblin, longer than the robot, and hitting harder than either
//! when it connects.
//!
//! ```text
//!            reach     jab startup   f-smash damage
//!   goblin    22 px       0.04 s          12
//!   robot     26 px       0.05 s          15
//!   admiral   32 px       0.06 s          17
//! ```
//!
//! ⛔ **the PISTOL is not in here, and that is the authority split doing its job.**
//! A ranged verb belongs to the character's `ActionSet` — what this body is
//! CAPABLE of — while this table is what its swings ARE. Putting a shot in the
//! move list would give one press two owners, which is the exact double-ownership
//! `RangedExecution` exists to prevent.

use ambition_platformer2d::entity_catalog::MovesetContract;

use crate::moveset_authoring::{airborne_only, grounded_only, strike};

/// See the module doc. Eleven moves, the genre's standard verb map.
pub fn pirate_admiral_moveset() -> MovesetContract {
    let mut moves = Vec::new();

    // ── grounded ─────────────────────────────────────────────────────────────
    //
    // Even the jab is a blade: it starts slower than the goblin's whole punish
    // window and reaches half a body further.
    let mut jab = strike(
        "jab",
        "jab",
        0.06,
        0.07,
        0.16,
        (30.0, 0.0),
        (22.0, 14.0),
        4,
        55.0,
        1.10,
        None,
        None,
    );
    jab.gates = grounded_only();
    moves.push(jab);

    // A rising cutlass arc. Wide, because a sword's up-tilt covers the space in
    // front of the shoulder as well as above it.
    let mut up_tilt = strike(
        "tilt_up",
        "attack_up",
        0.09,
        0.08,
        0.19,
        (14.0, -28.0),
        (22.0, 24.0),
        6,
        80.0,
        1.35,
        Some((0.25, -1.0)),
        None,
    );
    up_tilt.gates = grounded_only();
    moves.push(up_tilt);

    // A low sweep along the deck. Long, shallow, and it sends them along the
    // ground rather than up — the setup, not the finish.
    let mut down_tilt = strike(
        "tilt_down",
        "attack_down",
        0.08,
        0.07,
        0.18,
        (30.0, 14.0),
        (24.0, 9.0),
        5,
        60.0,
        1.20,
        Some((1.0, -0.18)),
        None,
    );
    down_tilt.gates = grounded_only();
    moves.push(down_tilt);

    // ── smashes ──────────────────────────────────────────────────────────────
    //
    // ⚠ the slowest kill move of the three tables, and the hardest. An admiral
    // who commits to a full cutlass swing has decided the exchange is worth 0.38s
    // of standing still afterwards.
    let mut f_smash = strike(
        "smash_forward",
        "smash_forward",
        0.34,
        0.08,
        0.38,
        (44.0, -4.0),
        (30.0, 20.0),
        17,
        160.0,
        3.10,
        Some((1.0, -0.42)),
        None,
    );
    f_smash.gates = grounded_only();
    f_smash.smash_charge_mult = 1.7;
    moves.push(f_smash);

    let mut up_smash = strike(
        "smash_up",
        "smash_up",
        0.30,
        0.09,
        0.34,
        (8.0, -34.0),
        (24.0, 30.0),
        15,
        155.0,
        2.95,
        Some((0.0, -1.0)),
        None,
    );
    up_smash.gates = grounded_only();
    up_smash.smash_charge_mult = 1.7;
    moves.push(up_smash);

    // Both sides at deck height — the boarding-action answer to being flanked.
    let mut down_smash = strike(
        "smash_down",
        "smash_down",
        0.28,
        0.10,
        0.36,
        (0.0, 16.0),
        (40.0, 12.0),
        13,
        140.0,
        2.70,
        Some((0.9, -0.50)),
        None,
    );
    down_smash.gates = grounded_only();
    down_smash.smash_charge_mult = 1.7;
    moves.push(down_smash);

    // ── aerials ──────────────────────────────────────────────────────────────
    let mut n_air = strike(
        "air_neutral",
        "air_neutral",
        0.07,
        0.12,
        0.16,
        (0.0, 0.0),
        (26.0, 22.0),
        6,
        70.0,
        1.30,
        None,
        None,
    );
    n_air.gates = airborne_only();
    moves.push(n_air);

    let mut f_air = strike(
        "air_forward",
        "air_forward",
        0.11,
        0.08,
        0.20,
        (32.0, -2.0),
        (24.0, 18.0),
        9,
        100.0,
        1.90,
        Some((1.0, -0.30)),
        None,
    );
    f_air.gates = airborne_only();
    moves.push(f_air);

    let mut b_air = strike(
        "air_back",
        "air_back",
        0.13,
        0.07,
        0.24,
        (-34.0, 0.0),
        (24.0, 18.0),
        11,
        135.0,
        2.55,
        Some((-1.0, -0.35)),
        None,
    );
    b_air.gates = airborne_only();
    moves.push(b_air);

    let mut u_air = strike(
        "air_up",
        "air_up",
        0.08,
        0.09,
        0.17,
        (2.0, -30.0),
        (20.0, 24.0),
        7,
        90.0,
        1.85,
        Some((0.0, -1.0)),
        None,
    );
    u_air.gates = airborne_only();
    moves.push(u_air);

    // ⭐ a real spike: point-down cutlass, straight into the blast zone. ⚠ no
    // `on_hit` rebound, same as the goblin's — the robot is the only body that
    // says it can bounce off what it hits, and that is a property of the
    // character rather than of down-airs.
    let mut d_air = strike(
        "air_down",
        "air_down",
        0.14,
        0.08,
        0.26,
        (6.0, 28.0),
        (20.0, 20.0),
        11,
        130.0,
        2.30,
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
    use ambition_platformer2d::entity_catalog::{MoveSpec, VolumeShape, WindowTag};

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

    fn reach(m: &MoveSpec) -> f32 {
        m.windows
            .iter()
            .flat_map(|w| w.volumes.iter())
            .map(|v| match v.shape {
                VolumeShape::Rect {
                    offset,
                    half_extents,
                } => offset.0.abs() + half_extents.0,
                _ => 0.0,
            })
            .fold(0.0f32, f32::max)
    }

    fn damage(m: &MoveSpec) -> i32 {
        m.windows
            .iter()
            .flat_map(|w| w.volumes.iter())
            .map(|v| v.damage)
            .max()
            .unwrap_or(0)
    }

    /// **Every verb the admiral binds resolves to a move that exists.**
    ///
    /// ⛔ a verb bound to a missing id is silence at the press: the runtime looks
    /// the move up, finds nothing, and the button does nothing at all.
    #[test]
    fn every_bound_verb_names_a_move_that_exists() {
        let moveset = pirate_admiral_moveset();
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

    /// **Three tables, three fighters, one ORDERING** — and it is checked against
    /// the other two rather than against literals.
    ///
    /// ⭐ this is what stops a repertoire being the previous one renumbered. The
    /// claim in every module doc here is comparative (*shorter*, *slower*,
    /// *harder*), so the test has to be comparative too; pinning the admiral's
    /// numbers alone would go green on a table that had quietly become the
    /// goblin's.
    ///
    /// ⚠ it also means retuning ANY of the three has to keep the ordering true or
    /// say why, which is the point: these are the characters' relationships to
    /// each other, not three independent piles of numbers.
    #[test]
    fn the_admiral_is_longer_slower_and_heavier_than_the_other_two() {
        let admiral = pirate_admiral_moveset();
        let goblin = crate::goblin_moveset::goblin_moveset();
        let robot = crate::player_robot_moveset::player_robot_moveset();

        let jabs = |set: &MovesetContract| {
            let jab = find(set, "jab");
            (reach(&jab), startup(&jab))
        };
        let (a_reach, a_startup) = jabs(&admiral);
        let (r_reach, r_startup) = jabs(&robot);
        let (g_reach, g_startup) = jabs(&goblin);

        assert!(
            a_reach > r_reach && r_reach > g_reach,
            "reach orders admiral > robot > goblin (got {a_reach}, {r_reach}, {g_reach})"
        );
        assert!(
            a_startup > r_startup && r_startup > g_startup,
            "and startup orders the same way — the longer blade is the slower one \
             (got {a_startup}, {r_startup}, {g_startup})"
        );

        let smash = |set: &MovesetContract| damage(&find(set, "smash_forward"));
        assert!(
            smash(&admiral) > smash(&robot) && smash(&robot) > smash(&goblin),
            "and the kill move pays for the commitment: {} > {} > {}",
            smash(&admiral),
            smash(&robot),
            smash(&goblin)
        );
    }
}
