//! **The Patent Clerk's repertoire** — the heavyweight, written from the row's own
//! `gameplay_description` rather than from taste.
//!
//! ⭐ **the third adopter removed from `smash_fighter_kit()`** (campaign P3.24).
//!
//! ⚠ **the character had already been designed and nobody had read it back.** Its
//! catalog row carries a `gameplay_description` in full:
//!
//! > *A high-mastery heavyweight controller who classifies bodies as MASS,
//! > ENERGY, MOVING, or AT REST; manipulates relative velocity and local reference
//! > frames; and turns careful observation into unusually strong parries and
//! > finishers.*
//!
//! Three words in that decide this table — **heavyweight**, **controller**,
//! **finishers** — and they are not mine:
//!
//! * *heavyweight* → the slowest startups in the game and the largest volumes.
//!   Every swing here is a commitment; the clerk does not poke.
//! * *controller* → the tilts SET UP rather than kill. The down-tilt sends along
//!   the floor, the up-tilt pops straight up: both leave the opponent somewhere
//!   the clerk chose, which is what a controller trades damage for.
//! * *finishers* → and then the smashes are the hardest in the game, because a
//!   body this slow only ever gets the one.
//!
//! ⛔ **the CLASSIFICATION mechanic is NOT in here** — MASS/ENERGY/MOVING/AT REST,
//! the reference-frame manipulation, the accelerating-elevator recovery, the
//! synchronized clocks. Those are systems, not swings. A moveset says what a hit
//! IS; a mechanic that changes what a body's velocity MEANS belongs to the
//! character's abilities and the ruleset. Writing them as move windows would be
//! the wholesale-migration failure mode wearing a content commit.

use ambition_platformer2d::entity_catalog::MovesetContract;

use crate::moveset_authoring::{airborne_only, grounded_only, strike};

/// See the module doc. Eleven moves, the genre's standard verb map.
pub fn patent_clerk_moveset() -> MovesetContract {
    let mut moves = Vec::new();

    // ── grounded ─────────────────────────────────────────────────────────────
    //
    // ⚠ the slowest jab in the game, and it is supposed to be. A heavyweight's
    // fast option is still a decision — 0.08s is long enough that a goblin can
    // walk into it, hit twice and leave.
    let mut jab = strike(
        "jab",
        "jab",
        0.08,
        0.08,
        0.20,
        (28.0, 0.0),
        (22.0, 18.0),
        5,
        60.0,
        1.10,
        None,
        None,
    );
    jab.gates = grounded_only();
    moves.push(jab);

    // CONTROLLER, not killer: it pops them straight up, at a launch too weak to
    // finish anybody. What it buys is the next four moves happening above a body
    // that cannot walk away.
    let mut up_tilt = strike(
        "tilt_up",
        "attack_up",
        0.10,
        0.09,
        0.20,
        (8.0, -30.0),
        (22.0, 26.0),
        6,
        75.0,
        1.15,
        Some((0.0, -1.0)),
        None,
    );
    up_tilt.gates = grounded_only();
    moves.push(up_tilt);

    // The other half of the setup: along the floor, almost no vertical. They end
    // up at the ledge, which is where the clerk wants everybody.
    let mut down_tilt = strike(
        "tilt_down",
        "attack_down",
        0.09,
        0.08,
        0.19,
        (28.0, 14.0),
        (24.0, 10.0),
        5,
        70.0,
        1.20,
        Some((1.0, -0.10)),
        None,
    );
    down_tilt.gates = grounded_only();
    moves.push(down_tilt);

    // ── smashes: the FINISHERS ───────────────────────────────────────────────
    //
    // ⚠ the hardest hits in the game, on the longest commitments in the game. A
    // body this slow gets one of these per stock if it is playing well, so it has
    // to be the one that ends things.
    let mut f_smash = strike(
        "smash_forward",
        "smash_forward",
        0.38,
        0.08,
        0.42,
        (42.0, -4.0),
        (30.0, 24.0),
        19,
        175.0,
        3.30,
        Some((1.0, -0.45)),
        None,
    );
    f_smash.gates = grounded_only();
    f_smash.smash_charge_mult = 1.7;
    moves.push(f_smash);

    let mut up_smash = strike(
        "smash_up",
        "smash_up",
        0.34,
        0.10,
        0.38,
        (6.0, -36.0),
        (26.0, 32.0),
        17,
        170.0,
        3.15,
        Some((0.0, -1.0)),
        None,
    );
    up_smash.gates = grounded_only();
    up_smash.smash_charge_mult = 1.7;
    moves.push(up_smash);

    let mut down_smash = strike(
        "smash_down",
        "smash_down",
        0.32,
        0.11,
        0.40,
        (0.0, 16.0),
        (42.0, 13.0),
        15,
        155.0,
        2.85,
        Some((0.95, -0.45)),
        None,
    );
    down_smash.gates = grounded_only();
    down_smash.smash_charge_mult = 1.7;
    moves.push(down_smash);

    // ── aerials ──────────────────────────────────────────────────────────────
    //
    // Big and slow in the air too, with one exception below.
    let mut n_air = strike(
        "air_neutral",
        "air_neutral",
        0.09,
        0.14,
        0.18,
        (0.0, 0.0),
        (28.0, 26.0),
        7,
        75.0,
        1.30,
        None,
        None,
    );
    n_air.gates = airborne_only();
    moves.push(n_air);

    let mut f_air = strike(
        "air_forward",
        "air_forward",
        0.13,
        0.09,
        0.22,
        (32.0, -2.0),
        (26.0, 20.0),
        10,
        105.0,
        1.95,
        Some((1.0, -0.28)),
        None,
    );
    f_air.gates = airborne_only();
    moves.push(f_air);

    let mut b_air = strike(
        "air_back",
        "air_back",
        0.15,
        0.08,
        0.26,
        (-34.0, 0.0),
        (26.0, 20.0),
        12,
        145.0,
        2.60,
        Some((-1.0, -0.35)),
        None,
    );
    b_air.gates = airborne_only();
    moves.push(b_air);

    let mut u_air = strike(
        "air_up",
        "air_up",
        0.10,
        0.10,
        0.19,
        (2.0, -32.0),
        (22.0, 26.0),
        8,
        95.0,
        1.90,
        Some((0.0, -1.0)),
        None,
    );
    u_air.gates = airborne_only();
    moves.push(u_air);

    // ⭐ the exception, and the one place *AT REST* shows up as a swing: it stops
    // dead and drops. Straight down, no drift, the heaviest spike in the game.
    let mut d_air = strike(
        "air_down",
        "air_down",
        0.16,
        0.09,
        0.28,
        (4.0, 30.0),
        (22.0, 22.0),
        13,
        140.0,
        2.40,
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

    fn launch(m: &MoveSpec) -> f32 {
        m.windows
            .iter()
            .flat_map(|w| w.volumes.iter())
            .map(|v| v.knockback)
            .fold(0.0f32, f32::max)
    }

    /// **Every verb the clerk binds resolves to a move that exists.**
    #[test]
    fn every_bound_verb_names_a_move_that_exists() {
        let moveset = patent_clerk_moveset();
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

    /// **The row said HEAVYWEIGHT and FINISHERS, and the table has to mean it.**
    ///
    /// Measured against the admiral — the next-heaviest body with a table — so the
    /// claim stays comparative. A heavyweight that is merely *big numbers* is a
    /// balance patch; a heavyweight is slower AND harder than the thing below it.
    #[test]
    fn the_clerk_is_slower_and_hits_harder_than_the_admiral() {
        let clerk = patent_clerk_moveset();
        let admiral = crate::pirate_admiral_moveset::pirate_admiral_moveset();

        assert!(
            startup(&find(&clerk, "jab")) > startup(&find(&admiral, "jab")),
            "the heaviest body has the slowest fast option"
        );
        assert!(
            startup(&find(&clerk, "smash_forward")) > startup(&find(&admiral, "smash_forward")),
            "and the longest commitment on its kill move"
        );
        assert!(
            damage(&find(&clerk, "smash_forward")) > damage(&find(&admiral, "smash_forward")),
            "which is what it is paid for"
        );
    }

    /// **CONTROLLER: the tilts set up, they do not finish.**
    ///
    /// ⭐ the word in the row that is easiest to lose while writing numbers. A
    /// tilt that launches as hard as a smash makes the smash pointless and the
    /// character a brawler — so the gap between them IS the design, and it is
    /// asserted rather than remembered.
    #[test]
    fn the_tilts_set_up_and_the_smashes_finish() {
        let clerk = patent_clerk_moveset();
        let strongest_tilt = ["tilt_up", "tilt_down"]
            .into_iter()
            .map(|id| launch(&find(&clerk, id)))
            .fold(0.0f32, f32::max);
        let weakest_smash = ["smash_forward", "smash_up", "smash_down"]
            .into_iter()
            .map(|id| launch(&find(&clerk, id)))
            .fold(f32::MAX, f32::min);
        assert!(
            strongest_tilt * 2.0 < weakest_smash,
            "a controller's tilts must be worth less than half its finishers \
             ({strongest_tilt} vs {weakest_smash}), or the finisher is decoration"
        );
    }
}
