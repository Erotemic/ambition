//! **Mary-O's repertoire, for the stage she visits rather than the one she
//! lives on.**
//!
//! ⭐ **written 2026-08-16** (Jon: *"Let's complete the kit for all characters,
//! authoring new moves when we need to."*). A census of the smash grid found her
//! at **0/16**: no table, no action set, and no unarmed floor reaching her body,
//! so every press was silence. She was one of four in that state.
//!
//! ⛔⛔ **AND IT CHANGES NOTHING IN HER OWN GAME**, which is the only reason this
//! file is allowed to exist. Her catalog row authors `abilities: Some([RunJump])`
//! — *"Mary-O Classic is deliberately only the run/jump floor"* — and `RunJump`
//! does not include `attack`. A move table is *what the attack IS*; the ability
//! is *whether this body may attack at all*, and hers says no. So these sixteen
//! moves are unreachable at home and reach a body the moment a stage GRANTS the
//! verb, which the smash stage does (`MatchAbilities::levelled`).
//!
//! ⇒ that split is what makes "a classic platformer protagonist on a fighting
//! grid" expressible instead of a contradiction. Her jump physics, her one-hit
//! death and her two on-screen buttons are untouched.
//!
//! ## The character
//!
//! She is a platformer, so her kit is FEET. Every heavy press in it is a stomp,
//! a drop or a jump that landed on somebody, and the two moves that are not —
//! the fireball pair — are the power-up her own game is built around. Light,
//! quick, and the weakest kill power on the grid: what she has instead is the
//! best down-air in it, because falling on things is the genre she comes from.

use ambition_platformer2d::characters::moveset_authoring::{
    airborne_only, committed_tail, either_posture, grounded_only, impulse, on_contact, sfx, strike,
    vfx_at,
};
use ambition_platformer2d::entity_catalog::{ImpulseMode, MovesetContract};

const STEP_FX: f32 = 0.8;
const FLAME_FX: f32 = 1.1;

/// See the module doc. Sixteen presses.
pub fn mary_o_moveset() -> MovesetContract {
    let mut moves = Vec::new();

    // **JAB — `hop_kick`.** A quick boot. She does not have a punch.
    let mut jab = strike(
        "hop_kick",
        "jab",
        0.05,
        0.05,
        0.13,
        (22.0, 2.0),
        (16.0, 12.0),
        3,
        46.0,
        1.05,
        None,
        None,
    );
    jab.gates = grounded_only();
    let jab = vfx_at(jab, 0.05, "poof_small", (22.0, 2.0), STEP_FX);
    let jab = on_contact(jab, "player.hit");
    moves.push(jab);

    // **FORWARD TILT — `sweep_kick`.** Low and out, at the height a goomba is.
    let mut f_tilt = strike(
        "sweep_kick",
        "attack_side",
        0.07,
        0.07,
        0.16,
        (28.0, 6.0),
        (20.0, 13.0),
        5,
        66.0,
        1.24,
        Some((1.0, -0.30)),
        None,
    );
    f_tilt.gates = grounded_only();
    let f_tilt = vfx_at(f_tilt, 0.07, "skid_puff", (28.0, 6.0), STEP_FX);
    let f_tilt = on_contact(f_tilt, "player.hit");
    moves.push(f_tilt);

    // **UP TILT — `header`.** The brick-breaking motion, aimed at a person.
    let mut u_tilt = strike(
        "header",
        "attack_up",
        0.07,
        0.07,
        0.16,
        (4.0, -26.0),
        (16.0, 20.0),
        5,
        72.0,
        1.30,
        Some((0.10, -1.0)),
        None,
    );
    u_tilt.gates = grounded_only();
    let u_tilt = vfx_at(u_tilt, 0.07, "four_point_glint", (4.0, -26.0), STEP_FX);
    let u_tilt = on_contact(u_tilt, "player.hit");
    moves.push(u_tilt);

    // **DOWN TILT — `slide`.** The crouch-slide, which in her game is how you
    // get under things.
    let mut d_tilt = strike(
        "slide",
        "attack_down",
        0.06,
        0.07,
        0.16,
        (26.0, 14.0),
        (22.0, 9.0),
        4,
        52.0,
        1.14,
        Some((0.95, -0.28)),
        None,
    );
    d_tilt.gates = grounded_only();
    let d_tilt = vfx_at(d_tilt, 0.06, "skid_puff", (26.0, 14.0), STEP_FX);
    let d_tilt = on_contact(d_tilt, "player.hit");
    moves.push(d_tilt);

    // **FORWARD SMASH — `shell_kick`.** She kicks something that is not there
    // and it goes anyway. ⚠ the weakest forward smash on the grid: she is the
    // lightest fighter on it and the numbers say so rather than a comment.
    let mut f_smash = strike(
        "shell_kick",
        "smash_forward",
        0.16,
        0.09,
        0.28,
        (34.0, 2.0),
        (26.0, 18.0),
        11,
        116.0,
        2.10,
        Some((0.95, -0.40)),
        None,
    );
    f_smash.gates = grounded_only();
    let f_smash = vfx_at(f_smash, 0.16, "starburst", (34.0, 2.0), FLAME_FX);
    let f_smash = sfx(f_smash, 0.16, "player.attack.charge");
    let f_smash = on_contact(f_smash, "player.hit");
    moves.push(f_smash);

    // **UP SMASH — `block_punch`.** Straight up into where the block would be.
    let mut u_smash = strike(
        "block_punch",
        "smash_up",
        0.15,
        0.10,
        0.27,
        (2.0, -32.0),
        (20.0, 30.0),
        11,
        114.0,
        2.15,
        Some((0.08, -1.0)),
        None,
    );
    u_smash.gates = grounded_only();
    let u_smash = vfx_at(u_smash, 0.15, "starburst", (2.0, -32.0), STEP_FX);
    let u_smash = on_contact(u_smash, "player.hit");
    moves.push(u_smash);

    // **DOWN SMASH — `ground_pound`.** Both feet, once, hard.
    let mut d_smash = strike(
        "ground_pound",
        "smash_down",
        0.17,
        0.09,
        0.30,
        (0.0, 20.0),
        (36.0, 12.0),
        10,
        108.0,
        1.95,
        Some((0.8, -0.55)),
        None,
    );
    d_smash.gates = grounded_only();
    let d_smash = vfx_at(d_smash, 0.17, "shockwave", (0.0, 20.0), FLAME_FX);
    let d_smash = vfx_at(d_smash, 0.17, "landing_puff", (0.0, 22.0), STEP_FX);
    let d_smash = on_contact(d_smash, "player.hit");
    moves.push(d_smash);

    // **NEUTRAL AIR — `tumble`.** The somersault her jump already does.
    let mut n_air = strike(
        "tumble",
        "air_neutral",
        0.06,
        0.11,
        0.15,
        (0.0, 0.0),
        (24.0, 22.0),
        6,
        66.0,
        1.36,
        Some((0.55, -0.75)),
        None,
    );
    n_air.gates = airborne_only();
    let n_air = vfx_at(n_air, 0.06, "wind_curl", (0.0, 0.0), STEP_FX);
    let n_air = on_contact(n_air, "player.hit");
    moves.push(n_air);

    // **FORWARD AIR — `drop_kick`.** Both feet, forward and down.
    let mut f_air = strike(
        "drop_kick",
        "air_forward",
        0.08,
        0.08,
        0.17,
        (28.0, 2.0),
        (21.0, 17.0),
        8,
        90.0,
        1.68,
        Some((0.9, -0.42)),
        None,
    );
    f_air.gates = airborne_only();
    let f_air = vfx_at(f_air, 0.08, "poof_small", (28.0, 2.0), STEP_FX);
    let f_air = on_contact(f_air, "player.hit");
    moves.push(f_air);

    // **BACK AIR — `mule_kick`.** She does not turn round.
    let mut b_air = strike(
        "mule_kick",
        "air_back",
        0.09,
        0.06,
        0.19,
        (-28.0, 0.0),
        (21.0, 16.0),
        9,
        98.0,
        1.82,
        Some((-0.95, -0.38)),
        None,
    );
    b_air.gates = airborne_only();
    let b_air = vfx_at(b_air, 0.09, "poof_small", (-28.0, 0.0), STEP_FX);
    let b_air = on_contact(b_air, "player.hit");
    moves.push(b_air);

    // **UP AIR — `flip_kick`.** Over the top, at whatever is above her.
    let mut u_air = strike(
        "flip_kick",
        "air_up",
        0.07,
        0.08,
        0.16,
        (2.0, -26.0),
        (19.0, 22.0),
        7,
        82.0,
        1.60,
        Some((0.08, -1.0)),
        None,
    );
    u_air.gates = airborne_only();
    let u_air = vfx_at(u_air, 0.07, "wind_curl", (2.0, -26.0), STEP_FX);
    let u_air = on_contact(u_air, "player.hit");
    moves.push(u_air);

    // **DOWN AIR — `stomp`. THE MOVE SHE COMES FROM.** The hardest down-air on
    // the grid, because landing on things is the entire genre she is a
    // protagonist of. Everything else in her kit is light; this is not.
    let mut d_air = strike(
        "stomp",
        "air_down",
        0.09,
        0.09,
        0.20,
        (0.0, 26.0),
        (20.0, 20.0),
        12,
        122.0,
        2.15,
        Some((0.0, 1.0)),
        None,
    );
    d_air.gates = airborne_only();
    let d_air = vfx_at(d_air, 0.09, "landing_puff", (0.0, 26.0), STEP_FX);
    let d_air = sfx(d_air, 0.09, "player.land");
    let d_air = on_contact(d_air, "player.hit");
    moves.push(d_air);

    // **NEUTRAL — `fireball`.** The power-up her game is built around, thrown
    // rather than carried. ⚠ a swung volume rather than a spawned projectile:
    // spawning one would be a second authority on a pattern her own game already
    // owns.
    let mut n_b = strike(
        "fireball",
        "attack",
        0.14,
        0.10,
        0.26,
        (32.0, 4.0),
        (24.0, 16.0),
        9,
        96.0,
        1.72,
        Some((0.9, -0.45)),
        None,
    );
    n_b.gates = either_posture();
    let n_b = committed_tail(n_b, 0.56, 0.20);
    let n_b = vfx_at(n_b, 0.14, "ember_wisp", (32.0, 4.0), FLAME_FX);
    let n_b = sfx(n_b, 0.14, "player.directional_special");
    let n_b = on_contact(n_b, "player.hit");
    moves.push(n_b);

    // **SIDE — `cape_spin`.** She turns once and whatever was next to her is on
    // the other side of the argument.
    let mut side_b = strike(
        "cape_spin",
        "attack_side",
        0.12,
        0.10,
        0.24,
        (26.0, 0.0),
        (24.0, 20.0),
        9,
        100.0,
        1.85,
        Some((0.92, -0.40)),
        None,
    );
    side_b.gates = either_posture();
    let side_b = impulse(side_b, 0.12, (520.0, 0.0), ImpulseMode::Set);
    let side_b = committed_tail(side_b, 0.54, 0.20);
    let side_b = vfx_at(side_b, 0.12, "wind_curl", (0.0, 0.0), FLAME_FX);
    let side_b = on_contact(side_b, "player.hit");
    moves.push(side_b);

    // **UP — `spring_jump`. THE RECOVERY**, and it is a JUMP, which is the only
    // shape her recovery could honestly take. High and cheap to land: the
    // lightest fighter's way home.
    let mut up_b = strike(
        "spring_jump",
        "attack_up",
        0.06,
        0.12,
        0.16,
        (0.0, -12.0),
        (18.0, 30.0),
        7,
        82.0,
        1.58,
        Some((0.10, -1.0)),
        None,
    );
    up_b.gates = either_posture();
    up_b.landing_lag_s = Some(0.20);
    let up_b = impulse(up_b, 0.06, (0.0, -800.0), ImpulseMode::Set);
    let up_b = committed_tail(up_b, 0.46, 0.30);
    let up_b = vfx_at(up_b, 0.06, "landing_puff", (0.0, 18.0), STEP_FX);
    let up_b = sfx(up_b, 0.06, "player.double_jump");
    let up_b = on_contact(up_b, "player.hit");
    moves.push(up_b);

    // **DOWN — `pipe_drop`.** She goes down and the floor objects. Grounded-only
    // — the joke needs a pipe to be standing on.
    let mut down_b = strike(
        "pipe_drop",
        "attack_down",
        0.16,
        0.09,
        0.30,
        (0.0, 18.0),
        (30.0, 13.0),
        9,
        92.0,
        1.68,
        Some((0.7, -0.62)),
        None,
    );
    down_b.gates = grounded_only();
    let down_b = committed_tail(down_b, 0.62, 0.0);
    let down_b = vfx_at(down_b, 0.16, "smoke_puff", (0.0, 18.0), FLAME_FX);
    let down_b = on_contact(down_b, "player.hit");
    moves.push(down_b);

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
    // **DOWN, IN THE AIR — `pipe_dive`.** No pipe up here, so she brings the
    // drop instead of the pipe. The genre's own move, and the one press of hers
    // that should never have been missing in the air.
    let mut air_down_b = strike(
        "pipe_dive",
        "air_down",
        0.09,
        0.10,
        0.22,
        (0.0, 24.0),
        (19.0, 21.0),
        10,
        104.0,
        1.82,
        Some((0.0, 1.0)),
        None,
    );
    air_down_b.gates = airborne_only();
    air_down_b.landing_lag_s = Some(0.24);
    let air_down_b = impulse(air_down_b, 0.09, (0.0, 1250.0), ImpulseMode::Set);
    let air_down_b = vfx_at(air_down_b, 0.09, "smoke_puff", (0.0, 22.0), STEP_FX);
    let air_down_b = on_contact(air_down_b, "player.hit");
    moves.push(air_down_b);

    let verbs = [
        ("attack", "hop_kick"),
        ("attack_forward", "sweep_kick"),
        ("attack_up", "header"),
        ("attack_down", "slide"),
        ("smash_forward", "shell_kick"),
        ("smash_up", "block_punch"),
        ("smash_down", "ground_pound"),
        ("attack_air", "tumble"),
        ("attack_air_forward", "drop_kick"),
        ("attack_air_back", "mule_kick"),
        ("attack_air_up", "flip_kick"),
        ("attack_air_down", "stomp"),
        ("special", "fireball"),
        ("special_forward", "cape_spin"),
        ("special_up", "spring_jump"),
        ("special_down", "pipe_drop"),
        ("special_air_down", "pipe_dive"),
    ]
    .into_iter()
    .map(|(verb, id)| (verb.to_string(), id.to_string()))
    .collect();

    MovesetContract { verbs, moves }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **Every verb she binds resolves to a move that exists.**
    #[test]
    fn every_bound_verb_names_a_move_that_exists() {
        let moveset = mary_o_moveset();
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
            17,
            "sixteen presses, and the down-B answers in BOTH postures"
        );
    }

    /// **THE STOMP IS HER HARDEST HIT**, which is the identity claim the module
    /// doc makes and the one a retune would quietly lose.
    #[test]
    fn the_down_air_hits_harder_than_anything_else_she_has() {
        let moveset = mary_o_moveset();
        let damage = |id: &str| {
            moveset
                .move_by_id(id)
                .unwrap_or_else(|| panic!("{id} exists"))
                .windows
                .iter()
                .flat_map(|w| w.volumes.iter())
                .map(|v| v.damage)
                .max()
                .unwrap_or(0)
        };
        let stomp = damage("stomp");
        for other in ["shell_kick", "block_punch", "ground_pound", "fireball"] {
            assert!(
                stomp >= damage(other),
                "`{other}` hits harder than the stomp, so the platformer \
                 protagonist's best move is no longer falling on somebody"
            );
        }
    }
}
