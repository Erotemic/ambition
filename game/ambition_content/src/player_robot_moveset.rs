//! **The player robot's canonical move repertoire** — the moves that ARE the
//! protagonist, wherever it is seated.
//!
//! ⭐⭐ **authored on the CHARACTER, so both games consume the same swing.** This
//! table was written for the Smash demo's shadow duelists (`smash_duelist_a/b`,
//! Robot art, Smash-only identities), which is exactly the arrangement Jon's
//! redirect §15 rejects: *"Do not copy the current rich Smash moves from
//! `smash_duelist_a` into a second independent Robot definition. Move/refactor
//! the canonical move data into the reusable Robot character provider and have
//! both compositions reference it."* This is that move.
//!
//! ⚠ **a move states what it IS, never what a mode does with it.** Startup,
//! active frames, recovery, hitbox geometry, damage, base launch, growth,
//! landing lag and auto-cancel are properties of the swing. Percent, stocks,
//! blast zones, DI and the strength of knockback growth are the RULESET's, and
//! they are declared per stage (`DeclaredCombatRules`) rather than baked here —
//! which is what lets Ambition read this table as Hollow-Knight combat and
//! Smash read it as a platform fighter.

use ambition_platformer2d::entity_catalog::{
    ClipBinding, EffectRef, HitVolume, MoveEvent, MoveEventKind, MoveGates, MoveSpec, MoveWindow,
    MovesetContract, VolumeShape, WindowTag,
};

// ⭐ the authoring primitives are SHARED (`moveset_authoring`), so the goblin's
// table below is written with the same `strike` this one is rather than a copy
// of it. They left this file the day a second character authored moves.
use crate::moveset_authoring::{airborne_only, grounded_only, strike};

/// **The fighter repertoire**, as one authored contract.
///
/// Shared by this demo's three fighters today. That is a content decision, not
/// an architectural one: the moveset rides the CHARACTER, so giving George a
/// heavier one is editing his definition and nothing else.
pub fn player_robot_moveset() -> MovesetContract {
    let mut moves = Vec::new();

    // ── grounded ─────────────────────────────────────────────────────────────
    //
    // The jab is the fast, safe, boring one — it exists to be thrown at nothing
    // and get away with it, which is what makes the smash below a decision.
    let mut jab = strike(
        "jab",
        "attack",
        0.05,
        0.06,
        0.14,
        (26.0, 0.0),
        (18.0, 14.0),
        3,
        55.0,
        1.10,
        None,
        None,
    );
    jab.gates = grounded_only();
    moves.push(jab);

    let mut up_tilt = strike(
        "tilt_up",
        "attack",
        0.07,
        0.08,
        0.18,
        (10.0, -30.0),
        (20.0, 22.0),
        5,
        70.0,
        1.40,
        // Straight up: an anti-air that starts a juggle rather than sending the
        // opponent away.
        Some((0.15, -1.0)),
        None,
    );
    up_tilt.gates = grounded_only();
    moves.push(up_tilt);

    let mut down_tilt = strike(
        "tilt_down",
        "attack",
        0.06,
        0.06,
        0.16,
        (26.0, 16.0),
        (20.0, 10.0),
        4,
        60.0,
        1.20,
        // A low poke that pops them up into the juggle.
        Some((0.5, -0.85)),
        None,
    );
    down_tilt.gates = grounded_only();
    moves.push(down_tilt);

    // ── the smashes ──────────────────────────────────────────────────────────
    //
    // ⭐ **the move the demo did not have.** A forward smash is eighteen frames
    // of startup you cannot take back, and the reason anybody accepts that is
    // the launch at the end of it: three times the jab's, growing with the
    // victim's percent, so at 120% it is the thing that ends the stock. The
    // charge multiplier is what a HELD press pays for.
    let mut f_smash = strike(
        "smash_forward",
        "attack",
        0.30,
        0.07,
        0.34,
        (40.0, -4.0),
        (28.0, 20.0),
        15,
        150.0,
        3.00,
        // Slightly upward and away: the classic kill angle. A contact-derived
        // direction would send a crouching opponent along the floor instead.
        Some((1.0, -0.42)),
        None,
    );
    f_smash.gates = grounded_only();
    // A fully-held charge lands 1.7× as hard. `smash_charge_mult` scales damage
    // AND knockback by how far the owner's clock got through the leading
    // Startup window before release, so the commitment and the payoff are the
    // same authored number.
    f_smash.smash_charge_mult = 1.7;
    moves.push(f_smash);

    let mut up_smash = strike(
        "smash_up",
        "attack",
        0.26,
        0.08,
        0.32,
        (8.0, -38.0),
        (24.0, 30.0),
        14,
        140.0,
        2.80,
        Some((0.12, -1.0)),
        None,
    );
    up_smash.gates = grounded_only();
    up_smash.smash_charge_mult = 1.7;
    moves.push(up_smash);

    let mut down_smash = strike(
        "smash_down",
        "attack",
        0.22,
        0.08,
        0.30,
        (0.0, 18.0),
        (40.0, 14.0),
        12,
        130.0,
        2.60,
        // Low and outward — the edge-guarding smash, not a launcher.
        Some((1.0, -0.25)),
        None,
    );
    down_smash.gates = grounded_only();
    down_smash.smash_charge_mult = 1.6;
    moves.push(down_smash);

    // ── aerials ──────────────────────────────────────────────────────────────
    //
    // ⭐ **landing lag and auto-cancel are what make an aerial a DECISION**, and
    // both were engine features with no adopter. The pair reads: throw this one
    // early in a jump and land clean; throw it late and pay for it.
    let mut n_air = strike(
        "air_neutral",
        "attack",
        0.06,
        0.14,
        0.16,
        (14.0, 0.0),
        (26.0, 22.0),
        6,
        75.0,
        1.50,
        None,
        None,
    );
    n_air.gates = airborne_only();
    n_air.landing_lag_s = Some(0.10);
    n_air.autocancel_after_s = Some(0.26);
    moves.push(n_air);

    let mut f_air = strike(
        "air_forward",
        "attack",
        0.09,
        0.08,
        0.22,
        (32.0, -4.0),
        (22.0, 18.0),
        9,
        105.0,
        2.10,
        Some((1.0, -0.35)),
        None,
    );
    f_air.gates = airborne_only();
    f_air.landing_lag_s = Some(0.18);
    f_air.autocancel_after_s = Some(0.30);
    moves.push(f_air);

    let mut b_air = strike(
        "air_back",
        "attack",
        0.10,
        0.07,
        0.24,
        (-32.0, -2.0),
        (22.0, 18.0),
        11,
        125.0,
        2.50,
        // Backwards and slightly up: the strongest aerial, and the one you have
        // to turn around for.
        Some((-1.0, -0.38)),
        None,
    );
    b_air.gates = airborne_only();
    b_air.landing_lag_s = Some(0.20);
    b_air.autocancel_after_s = Some(0.32);
    moves.push(b_air);

    let mut u_air = strike(
        "air_up",
        "attack",
        0.07,
        0.09,
        0.20,
        (4.0, -34.0),
        (22.0, 24.0),
        7,
        90.0,
        1.80,
        Some((0.1, -1.0)),
        None,
    );
    u_air.gates = airborne_only();
    u_air.landing_lag_s = Some(0.14);
    u_air.autocancel_after_s = Some(0.28);
    moves.push(u_air);

    let mut d_air = strike(
        "air_down",
        "attack",
        0.12,
        0.10,
        0.26,
        (6.0, 30.0),
        (20.0, 22.0),
        10,
        110.0,
        2.20,
        // Straight DOWN — a spike. Offstage this is a stock; onstage it is a
        // bounce the opponent has to deal with.
        Some((0.0, 1.0)),
        // ⭐ the ONE move that can bounce its attacker. Ambition reads this as a
        // pogo; a platform fighter declares `Spike` and it becomes a kill.
        Some(EffectRef::new(
            ambition_platformer2d::combat::on_hit::POGO_BOUNCE_KEY,
        )),
    );
    d_air.gates = airborne_only();
    // The heaviest lag in the set: a missed spike over the stage should hurt.
    d_air.landing_lag_s = Some(0.28);
    d_air.autocancel_after_s = Some(0.40);
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

    /// **Every verb the robot binds resolves to a move that exists.**
    #[test]
    fn every_authored_verb_resolves() {
        let set = player_robot_moveset();
        for (verb, id) in &set.verbs {
            assert!(
                set.move_by_id(id).is_some(),
                "verb `{verb}` names move `{id}`, which is not in the contract"
            );
        }
        assert_eq!(set.verbs.len(), 11, "the full directional repertoire");
    }

    /// **The protagonist states its own verbs, so a match stops guessing.**
    ///
    /// ⛔ **it authored none**, and an unauthored character takes the migration
    /// bridge in `seat_abilities`: the MODE's declared set, stamped on verbatim.
    /// That bridge exists because almost nothing in the repo authors verbs yet,
    /// and it is documented as meant to shrink — this is the first character out
    /// of it, and the right first, because it is the one body both games share.
    ///
    /// ⚠ **`reset` is deliberately absent**, and asserting that is the point:
    /// it is a debug affordance, and a character that authored it would hand
    /// every game that seats the robot a way to teleport home.
    ///
    /// ⚠ **`fly` is PRESENT, and my first pass had that wrong** — see the note
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

        // ⚠ **a RETIRED incarnation shares the VERBS and not the MOVES**, and
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

    /// **A MOVE CAN BE A COMBO, as data.**
    ///
    /// ⭐ **the claim the `player_robot` archetype row existed to make**, moved
    /// here with it (ledger D83): TWO Active windows on ONE timeline, so the
    /// system expresses a multi-hit combo as authored data rather than as a
    /// chain of presses the runtime happens to allow. A combo that needed two
    /// moves and a cancel would prove something else entirely.
    ///
    /// ⚠ the second hit has to HURT MORE, or the pair is a stutter rather than a
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

    /// **The robot's projectile has its own look, stated by the character.**
    ///
    /// ⛔ this was `ranged_visual` on the archetype row, and the character-first
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

    /// **The repertoire is a SMASH table, and it says so in its d-air.**
    ///
    /// ⛔ **this is the row that blocks attaching it to the protagonist**
    /// (ledger D82). `air_down` launches straight DOWN — a spike, which ends a
    /// stock offstage — while Ambition's down-air is a POGO that bounces the
    /// ATTACKER up off whatever it hits. Same press, same geometry, two
    /// readings, and only the mode can choose between them (Jon's redirect §16).
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

/// **THEOREM CHAIN — the robot's two-hit signature**, a light poke into a
/// heavier follow-up on ONE timeline.
///
/// ⭐ **the only proof in the repo that a moveset expresses multi-hit combos as
/// DATA across characters** rather than as a boss one-off. It lived on the
/// `player_robot` archetype row (ledger D83), which is the last thing keeping
/// eighty lines of enemy-archetype alive; here it is a character's move, on the
/// incarnation the exhibition duel actually fields.
///
/// ⚠ **v2's, not v3's.** The duel arena fields Robot v2 against the PCA, and v3
/// carries the platform-fighter table instead. Two incarnations of one robot
/// with different repertoires is what a lineage IS — the same reason v0 and v2
/// keep their own silhouettes.
pub fn theorem_chain_moveset() -> MovesetContract {
    let volume = |offset: (f32, f32), half_extents: (f32, f32), damage: i32, knockback: f32| {
        HitVolume {
            shape: VolumeShape::Rect {
                offset,
                half_extents,
            },
            damage,
            knockback,
            // Flat, exactly as the row authored it. A migration that added growth
            // on the way would be a retune wearing a migration's commit.
            knockback_growth: 0.0,
            launch_dir: None,
            on_hit: None,
            vfx: None,
            hit_sfx: None,
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
                // ⭐ the SECOND Active window on the SAME timeline — the whole
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
            landing_lag_s: None,
            autocancel_after_s: None,
        }],
    }
}
