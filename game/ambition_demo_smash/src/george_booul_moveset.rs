//! George Booul's authored fighter repertoire.
//!
//! George is a heavy commitment fighter with three fast pokes and otherwise
//! slow, high-damage attacks; the startup gap between those groups is part of
//! his character contract and is guarded by comparative tests. The demo owns
//! George's table; stand-in robot fighters continue to use their provider-owned
//! repertoires.

use ambition_entity_catalog::authoring::Strike;
use ambition_entity_catalog::smash_repertoire::{
    DownSpecial, NeutralSpecial, SmashRepertoire, UpSpecial,
};
use ambition_entity_catalog::{CancelCondition, ImpulseMode, MovesetContract};

use crate::moveset::{feel, Feel};
use ambition_entity_catalog::authoring::{cancelable, committed_tail, impulse, on_hit, strike};

/// The rise George's Up-B commands, engine units per second against gravity.
///
/// A speed applied with `ImpulseMode::Set`, which makes it a recovery: a body
/// falling at terminal velocity gets the same climb as one at rest. An additive
/// impulse would be weakest when George needs it most.
pub(crate) const ASCENT_SPEED: f32 = 1020.0;

/// When it arrives: a visible windup, and the number the recovery probe plans
/// around.
pub(crate) const ASCENT_AT_S: f32 = 0.18;

/// When the move lets go. `the_ascent_is_a_save_and_not_a_flight` guards the
/// arithmetic.
pub(crate) const ASCENT_ENDS_S: f32 = 1.15;

/// The widest startup a POKE may have, and the narrowest a COMMITMENT may have.
///
/// These define the character. The gap between them is the excluded middle,
/// and the guard asserts no move lands inside it. Retune by moving a move to
/// one side, never into the band.
const POKE_MAX_STARTUP_S: f32 = 0.08;
const COMMIT_MIN_STARTUP_S: f32 = 0.15;

/// Where a smash freezes: four frames into its windup, for every move.
///
/// Authored, not derived. The engine fallback `CHARGE_POSE_FRACTION` makes the
/// pose a fraction of the windup, so a slow smash would hold later than a fast
/// one. The hold must be on the first frames of the animation, before the
/// frames with hitboxes.
///
/// It must be inside the leading startup and before the first active window.
/// Every windup here is at least 0.22s. `CatalogError::ChargeHoldOutsideWindup`
/// refuses a pose that is not.
const CHARGE_POSE_AT_S: f32 = 4.0 / 60.0;

/// See the module doc. Sixteen moves, the genre's standard verb map plus four
/// specials.
// `knockback_growth: 6.28` is a tuned launch value, not an attempt at TAU.
#[allow(clippy::approx_constant)]
pub fn george_booul_moveset() -> MovesetContract {
    // ── the three pokes ──────────────────────────────────────────────────────
    //
    // Everything George throws quickly is nearly harmless. The pokes exist so
    // that not committing is a legal option.
    let jab = strike(Strike {
        id: "jab",
        clip: "attack",
        startup_s: 0.05,
        active_s: 0.05,
        recover_s: 0.15,
        offset: (26.0, 0.0),
        half_extents: (18.0, 14.0),
        damage: 3,
        knockback: 50.0,
        knockback_growth: 1.05,
        launch_dir: None,
        on_hit: None,
    });
    // The one route across the gap, open only on contact. `OnHit`: a
    // whiffed jab cancels into nothing, so the smash route is a reward for
    // connecting, not a smash with a jab's startup. The window covers the
    // active frames and the recovery.
    //
    // The string comes first: `jab2` answers the undirected follow-up (a
    // neutral re-press or a held button); the smash route needs a directed
    // press. The string is `Always` and the route is `OnHit`; one window
    // cannot hold both conditions, so each has its own window.
    let jab = cancelable(jab, 0.05, 0.25, &["jab2"], CancelCondition::Always);
    let jab = cancelable(
        jab,
        0.05,
        0.25,
        &["smash", "special"],
        CancelCondition::OnHit,
    );
    // Shielded, the jab buys a grab (`OnBlock`): a blocked jab is when the
    // defender is committed to shield, and a grab beats that.
    //
    // Third in the list, not first: the chain takes the first successor it
    // can resolve by move id, so `jab2` must stay first or a grab would answer
    // every held button. `grab` is a verb here; it resolves to `george_grab`.
    let jab = cancelable(jab, 0.05, 0.25, &["grab"], CancelCondition::OnBlock);
    let jab = feel(jab, Feel::Poke);

    let mut n_air = strike(Strike {
        id: "air_neutral",
        clip: "attack",
        startup_s: 0.06,
        active_s: 0.12,
        recover_s: 0.18,
        offset: (0.0, 0.0),
        half_extents: (26.0, 24.0),
        damage: 4,
        knockback: 65.0,
        knockback_growth: 1.20,
        launch_dir: None,
        on_hit: None,
    });
    n_air.landing_lag_s = Some(0.16);
    n_air.autocancel_after_s = Some(0.24);
    let n_air = feel(n_air, Feel::Poke);

    let mut u_air = strike(Strike {
        id: "air_up",
        clip: "attack",
        startup_s: 0.07,
        active_s: 0.09,
        recover_s: 0.19,
        offset: (2.0, -32.0),
        half_extents: (20.0, 24.0),
        damage: 4,
        knockback: 70.0,
        knockback_growth: 1.35,
        launch_dir: Some((0.0, -1.0)),
        on_hit: None,
    });
    u_air.landing_lag_s = Some(0.16);
    u_air.autocancel_after_s = Some(0.26);
    let u_air = feel(u_air, Feel::Launcher);

    // ── the tilts, which for George are commitments ──────────────────────────
    //
    // For most fighters a tilt is the safe middle option. George has none:
    // his up-tilt starts more than twice as late as the shared table's and hits
    // more than twice as hard.
    let up_tilt = strike(Strike {
        id: "tilt_up",
        clip: "attack",
        startup_s: 0.16,
        active_s: 0.09,
        recover_s: 0.26,
        offset: (10.0, -30.0),
        half_extents: (24.0, 28.0),
        damage: 11,
        knockback: 130.0,
        knockback_growth: 2.20,
        launch_dir: Some((0.1, -1.0)),
        on_hit: None,
    });
    let up_tilt = feel(up_tilt, Feel::Launcher);

    let down_tilt = strike(Strike {
        id: "tilt_down",
        clip: "attack",
        startup_s: 0.17,
        active_s: 0.08,
        recover_s: 0.28,
        offset: (30.0, 14.0),
        half_extents: (26.0, 11.0),
        damage: 11,
        knockback: 135.0,
        knockback_growth: 2.30,
        launch_dir: Some((1.0, -0.20)),
        on_hit: None,
    });
    let down_tilt = feel(down_tilt, Feel::Launcher);

    // ── the smashes ──────────────────────────────────────────────────────────
    //
    // The slowest and hardest smashes here, on a body that survives longest.
    // Fair only because he can never throw one unseen.
    let f_smash = strike(Strike {
        id: "smash_forward",
        clip: "attack",
        startup_s: 0.40,
        active_s: 0.08,
        recover_s: 0.46,
        offset: (46.0, -4.0),
        half_extents: (32.0, 24.0),
        damage: 21,
        knockback: 185.0,
        knockback_growth: 3.45,
        launch_dir: Some((1.0, -0.44)),
        on_hit: None,
    });
    let mut f_smash = ambition_entity_catalog::authoring::charge(
        f_smash,
        ambition_entity_catalog::authoring::Charge {
            hold_at_s: CHARGE_POSE_AT_S,
            max_hold_s: ambition_entity_catalog::SmashChargeSpec::DEFAULT_MAX_HOLD_S,
            stores: false,
            roots: true,
            sustain: ambition_entity_catalog::ChargeSustain::WhileHeld,
            gesture: ambition_entity_catalog::ChargeGesture::Smash,
            multiplier: 1.7,
        },
    );
    // Tip and base. The volume above is the tip, authored first, so a body
    // reached by both takes the tip. This is the base: the same commitment at
    // the wrong distance, which hurts but does not kill. The list order is the
    // priority.
    for window in f_smash
        .windows
        .iter_mut()
        .filter(|w| matches!(w.tag, ambition_entity_catalog::WindowTag::Active))
    {
        let tip = window.volumes[0].clone();
        window.volumes.push(ambition_entity_catalog::HitVolume {
            shape: ambition_entity_catalog::VolumeShape::Rect {
                // Inboard of the tip and overlapping it, so a body between
                // the two is reached by both.
                offset: (16.0, -4.0),
                half_extents: (18.0, 24.0),
            },
            damage: 11,
            knockback: 82.0,
            knockback_growth: Some(82.0 * crate::SMASH_KNOCKBACK_GROWTH),
            // Flatter and weaker: a base hit leaves them beside you.
            launch_dir: Some((1.0, -0.16)),
            ..tip
        });
    }
    let f_smash = feel(f_smash, Feel::Heavy);

    let up_smash = strike(Strike {
        id: "smash_up",
        clip: "attack",
        startup_s: 0.36,
        active_s: 0.10,
        recover_s: 0.42,
        offset: (6.0, -38.0),
        half_extents: (26.0, 34.0),
        damage: 19,
        knockback: 178.0,
        knockback_growth: 6.28,
        launch_dir: Some((0.0, -1.0)),
        on_hit: None,
    });
    let up_smash = ambition_entity_catalog::authoring::charge(
        up_smash,
        ambition_entity_catalog::authoring::Charge {
            hold_at_s: CHARGE_POSE_AT_S,
            max_hold_s: ambition_entity_catalog::SmashChargeSpec::DEFAULT_MAX_HOLD_S,
            stores: false,
            roots: true,
            sustain: ambition_entity_catalog::ChargeSustain::WhileHeld,
            gesture: ambition_entity_catalog::ChargeGesture::Smash,
            multiplier: 1.7,
        },
    );
    let up_smash = feel(up_smash, Feel::Heavy);

    let down_smash = strike(Strike {
        id: "smash_down",
        clip: "attack",
        startup_s: 0.34,
        active_s: 0.11,
        recover_s: 0.44,
        offset: (0.0, 16.0),
        half_extents: (44.0, 13.0),
        damage: 17,
        knockback: 165.0,
        knockback_growth: 3.46,
        launch_dir: Some((0.95, -0.45)),
        on_hit: None,
    });
    let down_smash = ambition_entity_catalog::authoring::charge(
        down_smash,
        ambition_entity_catalog::authoring::Charge {
            hold_at_s: CHARGE_POSE_AT_S,
            max_hold_s: ambition_entity_catalog::SmashChargeSpec::DEFAULT_MAX_HOLD_S,
            stores: false,
            roots: true,
            sustain: ambition_entity_catalog::ChargeSustain::WhileHeld,
            gesture: ambition_entity_catalog::ChargeGesture::Smash,
            multiplier: 1.7,
        },
    );
    let down_smash = feel(down_smash, Feel::Heavy);

    // ── the committed aerials ────────────────────────────────────────────────
    //
    // Three of his five aerials are on the slow side of the gap, with landing
    // lag to match. Jumping is another decision, not an escape.
    let mut f_air = strike(Strike {
        id: "air_forward",
        clip: "attack",
        startup_s: 0.18,
        active_s: 0.09,
        recover_s: 0.28,
        offset: (34.0, -2.0),
        half_extents: (26.0, 20.0),
        damage: 12,
        knockback: 140.0,
        knockback_growth: 2.35,
        launch_dir: Some((1.0, -0.30)),
        on_hit: None,
    });
    f_air.landing_lag_s = Some(0.24);
    f_air.autocancel_after_s = Some(0.34);
    let f_air = feel(f_air, Feel::Poke);

    let mut b_air = strike(Strike {
        id: "air_back",
        clip: "attack",
        startup_s: 0.20,
        active_s: 0.08,
        recover_s: 0.30,
        offset: (-36.0, 0.0),
        half_extents: (26.0, 20.0),
        damage: 14,
        knockback: 155.0,
        knockback_growth: 2.75,
        launch_dir: Some((-1.0, -0.36)),
        on_hit: None,
    });
    b_air.landing_lag_s = Some(0.26);
    b_air.autocancel_after_s = Some(0.36);
    let b_air = feel(b_air, Feel::Heavy);

    // The heaviest landing lag on the grid: a missed spike over the stage is a
    // free smash for whoever stands under it.
    let mut d_air = strike(Strike {
        id: "air_down",
        clip: "attack",
        startup_s: 0.22,
        active_s: 0.09,
        recover_s: 0.32,
        offset: (6.0, 32.0),
        half_extents: (22.0, 22.0),
        damage: 15,
        knockback: 150.0,
        knockback_growth: 2.55,
        launch_dir: Some((0.0, 1.0)),
        on_hit: None,
    });
    d_air.landing_lag_s = Some(0.34);
    d_air.autocancel_after_s = Some(0.44);
    let d_air = feel(d_air, Feel::Dive);

    // ── the forward tilt ─────────────────────────────────────────────────────
    //
    // Without it, a grounded forward press fell through to the jab. A stride
    // into a shoulder: still a commitment.
    let mut f_tilt = strike(Strike {
        id: "tilt_forward",
        clip: "attack",
        startup_s: 0.18,
        active_s: 0.08,
        recover_s: 0.27,
        offset: (36.0, -2.0),
        half_extents: (28.0, 18.0),
        damage: 12,
        knockback: 140.0,
        knockback_growth: 2.80,
        launch_dir: Some((1.0, -0.28)),
        on_hit: None,
    });
    // A short stride, additive: it adds to any run George brought, so it
    // covers more ground out of a dash.
    f_tilt.start_impulse = Some((190.0, 0.0));
    let f_tilt = feel(f_tilt, Feel::Heavy);

    // ── the specials ─────────────────────────────────────────────────────────
    //
    // `special` / `special_forward` / `special_up` / `special_down` resolve
    // through the same directional chain as every attack. Three are ordinary
    // strikes; the up-B needed `MoveEventKind::Impulse`.

    // Neutral: `bivalence`. Two active windows on one timeline: an early weak
    // pop and a late strong throw. Which half you take depends on when you
    // stood next to him.
    let mut bivalence = strike(Strike {
        id: "bivalence",
        clip: "special",
        startup_s: 0.30,
        active_s: 0.07,
        recover_s: 0.34,
        offset: (0.0, -6.0),
        half_extents: (36.0, 30.0),
        damage: 11,
        knockback: 160.0,
        knockback_growth: 2.00,
        launch_dir: Some((0.2, -1.0)),
        on_hit: None,
    });
    // No `smash_charge_mult`: a Special never takes the smash gesture, so the
    // multiplier could never be earned. Its old 1.6 is baked into the numbers
    // (damage 7→11 and 13→21, knockback 100→160 and 170→272), which matches
    // what the runtime paid. Without it George lost his recovery situations
    // (`the_cpu_throws_its_authored_recovery_during_a_match`). Whether this
    // should be his damage is open in
    // docs/planning/awaiting-maintainer-decision.md.
    //
    // The second half is a window, not a second move: same press, same clock,
    // harder answer.
    {
        let end = bivalence.duration_s;
        bivalence.windows.push(ambition_entity_catalog::MoveWindow {
            start_s: 0.42,
            end_s: 0.50,
            tag: ambition_entity_catalog::WindowTag::Active,
            volumes: vec![ambition_entity_catalog::HitVolume {
                // An ordinary hit, not a gust.
                shape: ambition_entity_catalog::VolumeShape::Circle {
                    offset: (0.0, -4.0),
                    radius: 46.0,
                },
                damage: 21,
                knockback: 272.0,
                knockback_growth: Some(3.40),
                launch_dir: Some((0.85, -0.55)),
                on_hit: None,
                vfx: Some("slash_arc".to_string()),
                hit_sfx: None,
                reaction: None,
            }],
            motion_scale: 0.25,
            sustain_effect: None,
        });
        debug_assert!(end >= 0.50, "the second window must fit inside the move");
    }
    let bivalence = feel(bivalence, Feel::Special);

    // The Limit payoff, on the same press. A full Limit meter turns the
    // neutral special into the version below; an empty one gives the usual
    // `bivalence`. Built from shipped parts: `meter_cost`, `when_refused` and
    // `afford_meter` (as the goblin's charged dive).
    //
    // It costs the whole meter. Against `JONS_BASELINE` (cap 60), blocking alone
    // needs 60 blocks, damage taken 30 hits, the clock 120 seconds. A mixed
    // sixty-second exchange (ten hits taken, ten dealt, eight blocks) scores
    // 68, so blocking contributes to the payoff.
    //
    // Clone before the buff: cloned after, the fallback would be the expensive
    // move and the meter would buy nothing.
    let bivalence_unmetered = {
        let mut spec = bivalence.clone();
        spec.id = "bivalence_unmetered".to_string();
        spec
    };
    let bivalence = ambition_entity_catalog::MoveSpec {
        gates: ambition_entity_catalog::MoveGates {
            // Exactly the cap (`LimitMeterFill::JONS_BASELINE.cap`):
            // `afford_meter` refuses anything less.
            costs: vec![ambition_resource_spec::ResourceCost::new(
                ambition_entity_catalog::smash_limit::LIMIT,
                60.0,
            )],
            // Bound to no verb, like the goblin's fallback: `move_by_id`
            // searches every move, so it needs only an id and a place in
            // `moves`.
            when_refused: Some(bivalence_unmetered.id.clone()),
            ..bivalence.gates.clone()
        },
        ..bivalence
    };
    // The payoff is the same strike made decisive, not a second move: the two
    // windows read the same, and only the numbers change.
    let bivalence = {
        let mut spec = bivalence;
        for window in &mut spec.windows {
            for volume in &mut window.volumes {
                volume.damage = (volume.damage as f32 * 1.5).round() as i32;
                volume.knockback *= 1.35;
            }
        }
        spec
    };

    // Side: `modus_ponens`. A travelling body-check. The burst is `Set`, so it
    // replaces George's motion with one committed direction, and the tail
    // cannot be steered. Offstage it is a horizontal recovery, and a way to
    // die, because it also erases any drift.
    let side_b = strike(Strike {
        id: "modus_ponens",
        clip: "special",
        startup_s: 0.20,
        active_s: 0.12,
        recover_s: 0.30,
        offset: (40.0, 0.0),
        half_extents: (30.0, 20.0),
        damage: 14,
        knockback: 160.0,
        knockback_growth: 3.20,
        launch_dir: Some((1.0, -0.30)),
        on_hit: None,
    });
    // No hop: the up-B is George's way home. A content decision.
    //
    // Known gap: `lifting_candidates` filters on `lift_speed > 0`, so the
    // recovery search never offers this horizontal move. Closing that means
    // the search proposing every displacing move, a search-cost decision.
    // `the_ascent_commands_its_rise_and_advertises_it` asserts nothing else in
    // George's table lifts.
    let side_b = impulse(side_b, 0.20, (760.0, 0.0), ImpulseMode::Set);
    let side_b = committed_tail(side_b, 0.74, 0.0);
    let side_b = feel(side_b, Feel::Special);

    // Up: `excluded_middle`, the recovery. The rise is commanded (`Set`) at
    // `ASCENT_AT_S` after a windup (`MoveEventKind::Impulse`), so a falling
    // George gets the same climb as a standing one. `start_impulse` fires at
    // the press and adds, so it cannot express this.
    //
    // Not flight: with no `Cancelable` window he cannot re-press until the
    // move ends, and the move outlasts its arc (`ASCENT_ENDS_S`), so repeated
    // use loses height. Held by a test; no rollback state.
    //
    // The hit is weak on purpose: a way home, not a kill move.
    let mut up_b = strike(Strike {
        id: "excluded_middle",
        clip: "special",
        startup_s: ASCENT_AT_S,
        active_s: 0.14,
        recover_s: 0.16,
        offset: (2.0, -30.0),
        half_extents: (24.0, 34.0),
        damage: 6,
        knockback: 95.0,
        knockback_growth: 1.90,
        launch_dir: Some((0.05, -1.0)),
        on_hit: None,
    });
    // Landing out of the ascent costs, so onstage it is a bad panic button.
    up_b.landing_lag_s = Some(0.28);
    let up_b = impulse(up_b, ASCENT_AT_S, (0.0, -ASCENT_SPEED), ImpulseMode::Set);
    // The helpless tail: `0.15` lets George nudge his landing and nothing
    // more, so an edgeguard is possible.
    let up_b = committed_tail(up_b, ASCENT_ENDS_S, 0.15);
    let up_b = feel(up_b, Feel::Recovery);

    // Down: `reductio`. A commanded plunge with pogo on contact: connect and
    // George bounces back up, so it can happen twice in a row. Offstage it
    // takes a stock from whoever is wrong about who is above whom.
    let mut down_b = strike(Strike {
        id: "reductio",
        clip: "special",
        startup_s: 0.16,
        active_s: 0.24,
        recover_s: 0.20,
        offset: (4.0, 30.0),
        half_extents: (24.0, 26.0),
        damage: 16,
        knockback: 150.0,
        knockback_growth: 3.00,
        launch_dir: Some((0.0, 1.0)),
        on_hit: None,
    });
    down_b.landing_lag_s = Some(0.36);
    let down_b = impulse(down_b, 0.16, (0.0, 1500.0), ImpulseMode::Set);
    let down_b = on_hit(
        down_b,
        ambition_platformer2d::characters::technique::POGO_BOUNCE_KEY,
    );
    let down_b = feel(down_b, Feel::Dive);

    // Down, on the ground: `reductio_ad_absurdum`. A short arc up, then the
    // same plunge (`reductio`'s numbers). The kit census asks specials in both
    // postures, so it must find this.
    let ground_down_b = strike(Strike {
        id: "reductio_ad_absurdum",
        clip: "special",
        startup_s: 0.34,
        active_s: 0.24,
        recover_s: 0.22,
        offset: (4.0, 30.0),
        half_extents: (24.0, 26.0),
        damage: 16,
        knockback: 150.0,
        knockback_growth: 3.00,
        launch_dir: Some((0.0, 1.0)),
        on_hit: None,
    });
    // The arc is an `Add`: `strike` derives `lift_speed` only from `Set`
    // impulses, and `excluded_middle` must be the only move that advertises a
    // way home. As a `Set`, the CPU would press down-B offstage and die (the
    // test below catches it). The move is grounded-only, so there is no
    // momentum to compose with.
    let ground_down_b = impulse(ground_down_b, 0.10, (200.0, -620.0), ImpulseMode::Add);
    let ground_down_b = impulse(ground_down_b, 0.34, (0.0, 1500.0), ImpulseMode::Set);
    let ground_down_b = on_hit(
        ground_down_b,
        ambition_platformer2d::characters::technique::POGO_BOUNCE_KEY,
    );
    let ground_down_b = committed_tail(ground_down_b, 0.86, 0.10);
    let ground_down_b = feel(ground_down_b, Feel::Dive);

    // ── The hold ────────────────────────────────────────────────────────────
    //
    // The sixteen slots above stay in Rust: they are composed from `strike`,
    // `impulse`, `on_hit`, `committed_tail` and `feel`, and the
    // `debug_assert` below states a law about the whole table.
    let capture = crate::smash_pack::capture_kit(crate::SMASH_GEORGE_BOOUL);

    let repertoire = SmashRepertoire {
        taunt: ambition_entity_catalog::authoring::taunt("george_booul_taunt", 0.9),

        // George's dash attack is a commitment. His law
        // (`no_move_lives_between_the_pokes_and_the_commitments`) keeps the
        // fast half weak (pokes top out at 5 damage), so a 14-damage move gets
        // `COMMIT_MIN_STARTUP_S` instead of the genre's 0.05.
        dash_attack: ambition_entity_catalog::authoring::dash_attack(
            "george_booul_dash_attack",
            ambition_entity_catalog::authoring::DashAttackShape {
                startup_s: COMMIT_MIN_STARTUP_S,
                ..ambition_entity_catalog::authoring::DashAttackShape::GENRE
            },
            14,
            175.0,
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
        neutral_special: NeutralSpecial::Authored(bivalence),
        side_special: side_b,
        up_special: UpSpecial::Standard(up_b),
        capture,
        down_special: DownSpecial::ByPosture {
            grounded: ground_down_b,
            airborne: down_b,
        },
    }
    .into_contract();

    // The jab string, from `jab_string_continuations`. A chain is a cancel
    // table over ordinary moves, not a verb, so the continuations join the
    // table directly. Do not copy them here.
    let mut repertoire = repertoire;
    repertoire
        .moves
        .extend(crate::moveset::jab_string_continuations());
    // The unmetered fallback: carried, but no input reaches it. With an empty
    // meter, `accepted_or_variant` finds it by id.
    repertoire.moves.push(bivalence_unmetered);

    // Check the poke/commitment split where it is authored, not only in
    // tests: this is the last place that knows both numbers.
    debug_assert!(
        repertoire.moves.iter().all(|m| {
            let startup = m
                .windows
                .iter()
                .find(|w| matches!(w.tag, ambition_entity_catalog::WindowTag::Active))
                .map_or(0.0, |w| w.start_s);
            startup <= POKE_MAX_STARTUP_S || startup >= COMMIT_MIN_STARTUP_S
        }),
        "a George move landed between the pokes and the commitments"
    );

    repertoire
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_entity_catalog::{MoveSpec, WindowTag};

    fn find(set: &MovesetContract, id: &str) -> MoveSpec {
        set.moves
            .iter()
            .find(|m| m.id == id)
            .unwrap_or_else(|| panic!("{id} exists"))
            .clone()
    }

    /// The tell before a move becomes dangerous, or `None` for a move with no
    /// dangerous moment. Pummels and throws have no Active window: their
    /// target was selected when the capture began.
    fn startup(m: &MoveSpec) -> Option<f32> {
        m.windows
            .iter()
            .find(|w| matches!(w.tag, WindowTag::Active))
            .map(|w| w.start_s)
    }

    fn damage(m: &MoveSpec) -> i32 {
        m.windows
            .iter()
            .flat_map(|w| w.volumes.iter())
            .map(|v| v.damage)
            .max()
            .unwrap_or(0)
    }

    // `SmashRepertoire` owns the verb strings and has no `Default` or private
    // fields, so a missing or renamed slot is a compile error here. That every
    // press is answered in every posture is checked by
    // `ambition_entity_catalog::smash_repertoire` and the host ratchet
    // `smash_roster_movesets::report_the_smash_kit_every_selectable_fighter_has`.

    /// The excluded middle, as an assertion: every move is a poke or a
    /// commitment, and the band between them is empty. A move in the band would
    /// be a reasonable tilt and would make George somebody else.
    #[test]
    fn no_move_lives_between_the_pokes_and_the_commitments() {
        let george = george_booul_moveset();

        // Exempt by name: six moves have no tell and reach for nobody (a
        // pummel and four throws, whose target is already selected, and the
        // taunt). Pinning the list means a strike that lost its Active window
        // fails here. The grab is not exempt: it reaches, so it has a tell.
        let mut telless: Vec<&str> = george
            .moves
            .iter()
            .filter(|m| startup(m).is_none())
            .map(|m| m.id.as_str())
            .collect();
        telless.sort_unstable();
        assert_eq!(
            telless,
            vec![
                "george_booul_taunt",
                "george_bthrow",
                "george_dthrow",
                "george_fthrow",
                "george_pummel",
                "george_uthrow",
            ],
            "the set of moves with no Active window changed"
        );

        for m in &george.moves {
            let Some(s) = startup(m) else { continue };
            assert!(
                s <= POKE_MAX_STARTUP_S || s >= COMMIT_MIN_STARTUP_S,
                "`{}` starts at {s}s, inside the band this fighter does not have \
                 ({POKE_MAX_STARTUP_S}..{COMMIT_MIN_STARTUP_S})",
                m.id
            );
        }

        // The two halves also differ by payoff, not only timing. Pinned by
        // name like the tell exemption, so a smash that lost its volumes
        // fails instead of becoming the softest commitment.
        let mut payless: Vec<&str> = george
            .moves
            .iter()
            .filter(|m| startup(m).is_some() && damage(m) == 0)
            .map(|m| m.id.as_str())
            .collect();
        payless.sort_unstable();
        assert_eq!(
            payless,
            // The running grab is derived by the capture kit from George's
            // grab, so the startup band also constrains a derived move. If the
            // grab starts near `POKE_MAX_STARTUP_S`, the derived wind-up can
            // land in the band, and the assertion above reports it.
            vec!["george_grab", "george_grab_dash"],
            "the set of moves that reach and deal no damage changed"
        );

        let (pokes, commits): (Vec<_>, Vec<_>) = george
            .moves
            .iter()
            // A move with no tell is neither poke nor commitment, and a move
            // with no damage is outside this claim.
            .filter(|m| startup(m).is_some() && damage(m) > 0)
            .partition(|m| startup(m).unwrap_or_default() <= POKE_MAX_STARTUP_S);
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

        // The poison: the shared table has a real middle (tilts at
        // 0.06–0.07, aerials at 0.09, 0.10, 0.12). If this passed for both
        // tables, the band would describe nothing.
        let shared = crate::moveset::fighter_moveset();
        assert!(
            shared.moves.iter().any(|m| {
                startup(m).is_some_and(|s| s > POKE_MAX_STARTUP_S && s < COMMIT_MIN_STARTUP_S)
            }),
            "the shared repertoire is supposed to HAVE a middle; if it does not, \
             this whole test is asserting a property of the threshold rather \
             than a property of George"
        );
    }

    /// Comparative, as for the goblin and the admiral: a table copied and
    /// renumbered would pass every other test here.
    #[test]
    fn george_commits_longer_and_hits_harder_than_the_shared_repertoire() {
        let george = george_booul_moveset();
        let shared = crate::moveset::fighter_moveset();
        for id in ["smash_forward", "smash_up", "smash_down"] {
            let (g, s) = (find(&george, id), find(&shared, id));
            // `expect`, not a filter: these are strikes, so a missing Active
            // window is a defect.
            let (gs, ss) = (
                startup(&g).expect("a smash has an active window"),
                startup(&s).expect("a smash has an active window"),
            );
            assert!(gs > ss, "`{id}`: the heavy commits longer ({gs} vs {ss})");
            assert!(
                damage(&g) > damage(&s),
                "`{id}`: and is paid for it ({} vs {})",
                damage(&g),
                damage(&s)
            );
        }

        // And nowhere is he faster; otherwise he would just be stronger.
        // The count stops the filter from emptying the loop.
        let mut compared = 0;
        for m in &george.moves {
            let Some(s) = shared.moves.iter().find(|other| other.id == m.id) else {
                continue;
            };
            compared += 1;
            // Both are shared-table strikes; `None` means one lost its Active
            // window.
            let (gs, ss) = (
                startup(m).expect("a shared-table move has an active window"),
                startup(s).expect("a shared-table move has an active window"),
            );
            assert!(
                gs >= ss,
                "`{}` is quicker than the shared table's ({gs} vs {ss})",
                m.id
            );
        }
        assert!(
            compared >= 11,
            "only {compared} moves were comparable; the two tables have stopped \
             overlapping and this test is asserting nothing"
        );
    }
    // ── the specials ─────────────────────────────────────────────────────────

    /// The ascent is a save, not a flight.
    ///
    /// This lets the Up-B exist with no cooldown, no per-airtime counter and no
    /// rollback state. With no `Cancelable` window the body cannot re-press
    /// while the move plays, and the move outlasts its arc, so one full cycle
    /// cannot gain height.
    #[test]
    fn the_ascent_is_a_save_and_not_a_flight() {
        let g = ambition_platformer2d::engine_core::DEFAULT_TUNING.gravity;
        let to_apex = ASCENT_SPEED / g;
        let tail = ASCENT_ENDS_S - ASCENT_AT_S;
        assert!(
            tail > 2.0 * to_apex,
            "the ascent climbs for {to_apex:.3}s and is handed back {tail:.3}s \
             after the burst; anything at or under {:.3}s returns George higher \
             than it found him, every press, which is flight",
            2.0 * to_apex
        );
        // The windup is real: a recovery with no tell is a free escape.
        assert!(ASCENT_AT_S >= COMMIT_MIN_STARTUP_S);
        // Landing out of it costs, so it is a bad panic button on the stage.
        let up_b = find(&george_booul_moveset(), "excluded_middle");
        assert!(up_b.landing_lag_s.unwrap_or(0.0) > 0.0);
    }

    /// The rise is commanded (`Set`), not added.
    ///
    /// Under `ImpulseMode::Add` a falling George would get only what was left
    /// over. `lift_speed` is derived from `Set` impulses only, so this also
    /// asserts that the brain and the recovery probe can see the move.
    #[test]
    fn the_ascent_commands_its_rise_and_advertises_it() {
        use ambition_entity_catalog::{ImpulseMode, MoveEventKind};
        let up_b = find(&george_booul_moveset(), "excluded_middle");
        let burst = up_b
            .events
            .iter()
            .find_map(|e| match &e.kind {
                MoveEventKind::Impulse { local, mode } => Some((e.at_s, *local, *mode)),
                _ => None,
            })
            .expect("the recovery special displaces its owner");
        assert_eq!(burst.2, ImpulseMode::Set);
        assert!(burst.1 .1 < 0.0, "the burst must point AGAINST gravity");
        assert_eq!(burst.0, ASCENT_AT_S);

        // The derived affordance the brain and recovery probe read. At zero
        // the CPU cannot see its recovery.
        let frames = up_b.frame_data();
        assert_eq!(frames.lift_speed, ASCENT_SPEED);
        assert_eq!(frames.lift_at_s, ASCENT_AT_S);

        // The poison: nothing else advertises a lift, or the assertion above
        // would tell a policy layer nothing.
        let table = george_booul_moveset();
        let others: Vec<&str> = table
            .moves
            .iter()
            .filter(|m| m.id != "excluded_middle" && m.frame_data().lift_speed > 0.0)
            .map(|m| m.id.as_str())
            .collect();
        assert!(
            others.is_empty(),
            "these moves also claim to be ways home: {others:?}"
        );
    }

    /// Four specials, four mechanisms: not rotated or mirrored clones of one
    /// base melee. One commands a rise, one a plunge that rebounds off what it
    /// hits, one an unsteerable horizontal charge, and one lands twice on one
    /// press.
    #[test]
    fn the_four_specials_are_four_different_mechanisms() {
        use ambition_entity_catalog::{ImpulseMode, MoveEventKind, WindowTag};
        let set = george_booul_moveset();
        let commanded = |id: &str| -> Option<(f32, f32)> {
            find(&set, id).events.iter().find_map(|e| match &e.kind {
                MoveEventKind::Impulse {
                    local,
                    mode: ImpulseMode::Set,
                } => Some(*local),
                _ => None,
            })
        };
        // Up: a rise only.
        let up = commanded("excluded_middle").expect("the Up-B displaces");
        assert!(up.1 < 0.0 && up.0 == 0.0);
        // Down: a plunge that rebounds off a body.
        let down = commanded("reductio").expect("the dive displaces");
        assert!(down.1 > 0.0);
        assert!(find(&set, "reductio")
            .windows
            .iter()
            .flat_map(|w| w.volumes.iter())
            .any(|v| v.on_hit.is_some()));
        // Side: a horizontal charge with an unsteerable tail.
        let side = commanded("modus_ponens").expect("the side special travels");
        assert!(side.0 > 0.0);
        assert!(
            find(&set, "modus_ponens")
                .windows
                .iter()
                .any(|w| matches!(w.tag, WindowTag::Recovery) && w.motion_scale == 0.0),
            "a charge you can steer out of is not a commitment"
        );
        // Neutral: no displacement; it lands twice instead.
        assert!(commanded("bivalence").is_none());
        assert_eq!(
            find(&set, "bivalence")
                .windows
                .iter()
                .filter(|w| matches!(w.tag, WindowTag::Active))
                .count(),
            2,
            "the neutral special's whole idea is the second window"
        );
    }

    /// Every press a body can make reaches a move, in both postures.
    #[test]
    fn both_postures_reach_at_least_eight_distinct_moves() {
        use ambition_entity_catalog::AttackDir;
        let set = george_booul_moveset();
        let reachable = |grounded: bool| -> std::collections::BTreeSet<String> {
            let mut ids = std::collections::BTreeSet::new();
            for base in ["attack", "smash", "special"] {
                for dir in [
                    AttackDir::Neutral,
                    AttackDir::Forward,
                    AttackDir::Back,
                    AttackDir::Up,
                    AttackDir::Down,
                ] {
                    if let Some(m) = set.move_for_directional_verb(base, dir, grounded) {
                        ids.insert(m.id.clone());
                    }
                }
            }
            ids
        };
        let on_ground = reachable(true);
        let airborne = reachable(false);
        assert!(
            on_ground.len() >= 8,
            "a grounded George reaches only {:?}",
            on_ground
        );
        assert!(
            airborne.len() >= 8,
            "an airborne George reaches only {:?}",
            airborne
        );
        // The recovery is reachable from both postures, so it can be practised
        // on stage.
        assert!(on_ground.contains("excluded_middle"));
        assert!(airborne.contains("excluded_middle"));
        // The forward press does not fall through to the jab.
        assert_eq!(
            set.move_for_directional_verb("attack", AttackDir::Forward, true)
                .map(|m| m.id.as_str()),
            Some("tilt_forward")
        );
    }

    /// The feedback is differentiated and resolvable. Two claims in one test
    /// because they fail together: identical sounds give no feedback, and an
    /// effect no shipped spritesheet carries never plays.
    #[test]
    fn important_moves_sound_and_look_like_themselves() {
        use ambition_entity_catalog::MoveEventKind;
        let set = george_booul_moveset();
        let mut effects = std::collections::BTreeSet::new();
        let mut cues = std::collections::BTreeSet::new();
        // Accumulate problems across every move, then assert once, so one run
        // lists every move that references a renamed effect.
        let mut problems: Vec<String> = Vec::new();
        for m in &set.moves {
            problems.extend(m.presentation_problems(
                ambition_platformer2d::sprite_sheet::fx::is_authored_effect,
            ));
            for ev in &m.events {
                match &ev.kind {
                    MoveEventKind::Vfx { effect, .. } => {
                        effects.insert(effect.clone());
                    }
                    MoveEventKind::Sfx { cue } => {
                        cues.insert(cue.clone());
                    }
                    _ => {}
                }
            }
        }
        // Before the palette checks below, which also fail on a renamed
        // effect with a less useful message.
        //
        // The message names the other cause. `fx::is_authored_effect` reads a
        // table that `ambition_sprite_sheet`'s `build.rs` bakes at compile time
        // from the generated, gitignored `assets/sprites`. If they were never
        // rendered, the table is empty and every move is listed. The count is
        // the diagnosis.
        assert!(
            problems.is_empty(),
            "{} move(s) name an unknown cosmetic effect.\n{problems:?}\n\
             ⇒ IF NEARLY EVERY MOVE IS LISTED, the baked FX sheet table is EMPTY \
             and this is not a content bug: the sheets are generated and \
             gitignored. Rebuild them, then re-run:\n\
             \x20   ./scripts/regen/sprites.sh          # or one target: --target george_booul_vfx\n\
             \x20   scripts/setup/generated_content.sh  # everything, fonts included\n\
             \x20   find crates/ambition_platformer2d_actor_monolith/assets/sprites \\\n\
             \x20        -name '*_spritesheet.ron' | wc -l   # 0 means the bake is empty\n\
             ⇒ IF ONLY ONE OR TWO ARE LISTED, a sheet row really was renamed or \
             removed and this moveset still names the old row.",
            problems.len()
        );
        assert!(
            effects.len() >= 4,
            "a jab, a smash, a launcher, a special and a recovery cannot all \
             look the same: {effects:?}"
        );
        assert!(cues.len() >= 3, "{cues:?}");
        // The recovery activating has its own burst.
        let up_b = find(&set, "excluded_middle");
        assert!(up_b.events.iter().any(|e| matches!(
            &e.kind,
            MoveEventKind::Vfx { effect, .. } if effect == "classic_burst"
        )));
        // A heavy landing sounds different from a poke landing.
        let heavy_hit = |id: &str| -> Option<String> {
            find(&set, id)
                .windows
                .iter()
                .flat_map(|w| w.volumes.iter())
                .find_map(|v| v.hit_sfx.clone())
        };
        assert_ne!(heavy_hit("smash_forward"), heavy_hit("jab"));
        assert!(heavy_hit("smash_forward").is_some());
        assert!(heavy_hit("jab").is_none(), "a jab does not clang");
    }

    /// The jab's two cancels are different promises. The string continues on
    /// a whiff (`Always`); the route across George's gap rewards connecting
    /// (`OnHit`). Windows are read by what they name, not by their order.
    #[test]
    fn the_jab_strings_on_a_whiff_and_opens_the_commitments_only_when_it_lands() {
        use ambition_entity_catalog::{CancelCondition, WindowTag};
        let jab = find(&george_booul_moveset(), "jab");
        let cancels: Vec<(Vec<String>, CancelCondition)> = jab
            .windows
            .iter()
            .filter_map(|w| match &w.tag {
                WindowTag::Cancelable { into, condition } => Some((into.clone(), *condition)),
                _ => None,
            })
            .collect();
        let named = |target: &str| {
            cancels
                .iter()
                .find(|(into, _)| into.iter().any(|t| t == target))
                .unwrap_or_else(|| panic!("no cancel window names `{target}`"))
        };
        assert_eq!(
            named("jab2").1,
            CancelCondition::Always,
            "a whiffed jab must still string"
        );
        let route = named("smash");
        assert_eq!(
            route.1,
            CancelCondition::OnHit,
            "George's route across the gap is bought by connecting"
        );
        assert!(route.0.iter().any(|t| t == "special"));
        // The string is named first: the chain takes the first successor it
        // can resolve by move id, so an undirected follow-up reaches `jab2`,
        // not a smash.
        assert_eq!(
            cancels[0].0.first().map(String::as_str),
            Some("jab2"),
            "the string has to be the first thing the jab nominates"
        );
    }

    /// What George leaves unanswered is the genre's shape, not a gap.
    ///
    /// The sibling guard in `moveset.rs` pins the stand-in's silent presses.
    /// Both enumerate every `(base, direction, stance)` press, because
    /// `move_for_directional_verb` falls back to the base verb.
    ///
    /// George is silent on seven presses, all `smash`: no neutral smash, no back
    /// smash, and no aerial smashes. That matches the genre, which uses the
    /// `attack` family in the air. The stand-in's silent presses are specials,
    /// which the genre has. See `awaiting-maintainer-decision.md`.
    ///
    /// The two halves are different claims:
    ///
    /// - The `smash` set is structural: `SmashRepertoire` has only
    ///   `forward_smash` / `up_smash` / `down_smash`, and aerials answer `attack`
    ///   presses. This arm is a schema guard.
    /// - The special arm is authored. George authors four specials (neutral,
    ///   side, up, down); the back press falls through to the neutral one.
    #[test]
    fn the_presses_george_leaves_unanswered_are_the_ones_the_genre_lacks() {
        use ambition_entity_catalog::AttackDir;
        let set = george_booul_moveset();
        let dirs = [
            ("neutral", AttackDir::Neutral),
            ("forward", AttackDir::Forward),
            ("up", AttackDir::Up),
            ("down", AttackDir::Down),
            ("back", AttackDir::Back),
        ];

        let mut silent: Vec<String> = Vec::new();
        for base in ["attack", "smash", "special"] {
            for (dir_name, dir) in dirs {
                for (stance, grounded) in [("ground", true), ("air", false)] {
                    if set.move_for_directional_verb(base, dir, grounded).is_none() {
                        silent.push(format!("{base}_{dir_name}_{stance}"));
                    }
                }
            }
        }

        // The load-bearing half: George answers all ten attack and all ten
        // special presses. If a special falls silent, the maintainer decision
        // changes shape.
        let non_smash: Vec<&String> = silent.iter().filter(|p| !p.starts_with("smash_")).collect();
        assert!(
            non_smash.is_empty(),
            "George stopped answering a non-`smash` press: {non_smash:?}. The roster question in `awaiting-maintainer-decision.md` rests on George answering all ten specials while the stand-ins answer two."
        );

        // The seven are exactly the genre's missing presses, asserted as a
        // set so a swap cannot pass by keeping the count.
        let mut got = silent.clone();
        got.sort();
        let mut want = vec![
            "smash_neutral_ground",
            "smash_neutral_air",
            "smash_back_ground",
            "smash_back_air",
            "smash_forward_air",
            "smash_up_air",
            "smash_down_air",
        ];
        want.sort();
        assert_eq!(
            got,
            want,
            "George's silent presses moved. Gaining one is likely good news (an authored move) and losing one is a regression; either way the claim in `smash-parity-inventory.md` wants re-deriving, not editing to match."
        );
    }

    /// A shielded jab buys a grab (`OnBlock`): a blocked jab is when the
    /// defender is committed to shield. Without it the `OnHit` route is closed,
    /// because nothing connected.
    ///
    /// It must not widen the other two: the string stays `Always` and the
    /// route stays `OnHit`. `MoveContact` carries three facts for this.
    #[test]
    fn a_shielded_jab_buys_george_a_grab() {
        use ambition_entity_catalog::{CancelCondition, WindowTag};
        let george = george_booul_moveset();
        let jab = find(&george, "jab");
        let blocked: Vec<&Vec<String>> = jab
            .windows
            .iter()
            .filter_map(|w| match &w.tag {
                WindowTag::Cancelable { into, condition }
                    if *condition == CancelCondition::OnBlock =>
                {
                    Some(into)
                }
                _ => None,
            })
            .collect();
        assert_eq!(
            blocked.len(),
            1,
            "the jab should nominate exactly one on-block continuation"
        );
        assert!(
            blocked[0].iter().any(|t| t == "grab"),
            "a shielded jab buys a GRAB — the option that beats the shield that \
             just ate it; got {:?}",
            blocked[0]
        );

        // The name must resolve: `grab` is a verb, not a move id, so the
        // contract's verb map must bind it.
        let targets = george.cancel_targets(blocked[0]);
        assert!(
            !targets.is_empty(),
            "`grab` named a continuation nothing in George's table answers to; \
             the window would open onto nothing"
        );
    }
}

#[cfg(test)]
mod limit_payoff_tests {
    use super::george_booul_moveset;

    fn spec(id: &str) -> ambition_entity_catalog::MoveSpec {
        george_booul_moveset()
            .move_by_id(id)
            .unwrap_or_else(|| panic!("the contract carries `{id}`"))
            .clone()
    }

    fn top_damage(spec: &ambition_entity_catalog::MoveSpec) -> i32 {
        spec.windows
            .iter()
            .flat_map(|w| w.volumes.iter())
            .map(|v| v.damage)
            .max()
            .expect("the special hits")
    }

    /// An `id` with no move behind it is a dead button. `when_refused` resolves
    /// with `move_by_id` against the carried moves; a fallback never pushed into
    /// `moves` resolves to `None`, so neutral-B on an empty meter does nothing.
    /// The id is a `String`, so the compiler cannot check it.
    #[test]
    fn the_metered_special_falls_back_to_a_move_the_contract_actually_carries() {
        let payoff = spec("bivalence");
        let fallback_id = payoff
            .gates
            .when_refused
            .clone()
            .expect("the metered special names a fallback");
        assert!(
            george_booul_moveset().move_by_id(&fallback_id).is_some(),
            "`bivalence` falls back to `{fallback_id}`, which the contract does \
             not carry — on an empty meter the press resolves to nothing and the \
             button is dead"
        );
    }

    /// The order is the mechanic: a fallback cloned after the buff would be
    /// the expensive move, and the meter would buy nothing while every other
    /// test passes. This compares the two specs one press can give.
    #[test]
    fn a_full_meter_buys_a_strictly_harder_answer_from_the_same_press() {
        let payoff = spec("bivalence");
        let unmetered = spec("bivalence_unmetered");
        assert!(
            top_damage(&payoff) > top_damage(&unmetered),
            "the metered neutral special ({}) must hit harder than the one an \
             empty meter gives ({}), or blocking all match bought nothing",
            top_damage(&payoff),
            top_damage(&unmetered),
        );
        assert!(
            unmetered.gates.costs.is_empty(),
            "the fallback must be free; a priced fallback is refused by the same \
             affordance that refused the payoff, and the press dies"
        );
    }

    /// The price is the cap: `afford_meter` refuses anything less. A lower
    /// price would be a different mechanic (a chargeable resource).
    #[test]
    fn the_price_is_the_whole_meter() {
        assert_eq!(
            spec("bivalence").gates.costs,
            vec![ambition_resource_spec::ResourceCost::new(
                ambition_entity_catalog::smash_limit::LIMIT,
                ambition_entity_catalog::smash_limit::LimitMeterFill::JONS_BASELINE.cap,
            )],
            "the payoff must cost exactly the match's Limit cap, in Limit"
        );
    }

    /// The fallback is carried, not pressable. With a verb, a player could
    /// reach the cheap version directly.
    #[test]
    fn the_fallback_is_bound_to_no_input() {
        assert!(
            !george_booul_moveset()
                .verbs
                .iter()
                .any(|(_, id)| id == "bivalence_unmetered"),
            "the unmetered fallback has been bound to an input; it is reachable \
             only by refusal, or the meter buys nothing"
        );
    }
}
