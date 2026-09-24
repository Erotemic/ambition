//! Patent Clerk's authored Smash repertoire.
//!
//! The kit is a heavyweight/control-oriented set of strikes and recoveries. The
//! MASS/ENERGY/MOVING/AT REST classification and reference-frame mechanics remain
//! character/system abilities rather than move-table effects. Character-specific VFX
//! cue overrides live here with the authored moves.

use ambition_entity_catalog::authoring::Strike;
use ambition_entity_catalog::smash_capture::{
    author_pummel, author_standing_grab, author_throw, capture_beat, grab_shell,
    CaptureAttemptParams, CaptureCues, CapturePummelParams, CaptureThrowParams,
    SmashCaptureRepertoire,
};
use ambition_entity_catalog::smash_repertoire::{
    DownSpecial, NeutralSpecial, SmashRepertoire, UpSpecial,
};
use ambition_entity_catalog::{ImpulseMode, MoveSpec, MovesetContract};

use ambition_entity_catalog::authoring::{
    committed_tail, fixed_knockback, impulse, on_contact, strike, vfx_at, vfx_cued,
};

const STAMP_FX: f32 = 0.55;
const SWING_FX: f32 = 0.80;
const PROOF_FX: f32 = 1.35;

/// The rise `elevator_thought` commands: the equivalence principle as a
/// recovery. A speed applied with [`ImpulseMode::Set`], so a falling clerk
/// climbs as far as a standing one.
pub const ELEVATOR_SPEED: f32 = 920.0;
pub const ELEVATOR_AT_S: f32 = 0.22;
/// Not a feel number: the tail must outlast the arc, or repeated presses gain
/// height (flight). `the_elevator_is_a_save_and_not_a_flight` checks it.
pub const ELEVATOR_ENDS_S: f32 = 1.18;

/// The least steering any window of a move leaves its owner.
pub fn tightest_lock(spec: &MoveSpec) -> f32 {
    spec.windows
        .iter()
        .map(|w| w.motion_scale)
        .fold(f32::MAX, f32::min)
}

/// See the module doc. Sixteen moves, the genre's standard verb map.
pub fn patent_clerk_moveset() -> MovesetContract {
    // ── grounded ─────────────────────────────────────────────────────────────
    //
    // The slowest jab in the game, on purpose. 0.08s is long enough that a
    // goblin can walk in, hit twice and leave.
    let jab = strike(Strike {
        id: "jab",
        clip: "jab",
        startup_s: 0.08,
        active_s: 0.08,
        recover_s: 0.20,
        offset: (28.0, 0.0),
        half_extents: (22.0, 18.0),
        damage: 5,
        knockback: 60.0,
        knockback_growth: 1.10,
        launch_dir: None,
        on_hit: None,
    });
    let jab = vfx_at(jab, 0.08, "stamp_at_rest", (28.0, 0.0), STAMP_FX);

    // A controller, not a killer: it pops them straight up, too weak to finish
    // anyone, so the next moves happen above a body that cannot walk away.
    let up_tilt = strike(Strike {
        id: "tilt_up",
        clip: "attack_up",
        startup_s: 0.10,
        active_s: 0.09,
        recover_s: 0.20,
        offset: (8.0, -30.0),
        half_extents: (22.0, 26.0),
        damage: 6,
        knockback: 75.0,
        knockback_growth: 1.15,
        launch_dir: Some((0.0, -1.0)),
        on_hit: None,
    });
    let up_tilt = vfx_cued(
        up_tilt,
        0.10,
        "proper_time_tick",
        (8.0, -30.0),
        SWING_FX,
        "vfx.patent_clerk.proper_time_tick.loop",
    );
    let up_tilt = on_contact(up_tilt, "player.hit");

    // The other half of the setup: along the floor, almost no vertical, toward
    // the ledge where he wants everyone.
    let down_tilt = strike(Strike {
        id: "tilt_down",
        clip: "attack_down",
        startup_s: 0.09,
        active_s: 0.08,
        recover_s: 0.19,
        offset: (28.0, 14.0),
        half_extents: (24.0, 10.0),
        damage: 5,
        knockback: 70.0,
        knockback_growth: 1.20,
        launch_dir: Some((1.0, -0.10)),
        on_hit: None,
    });
    let down_tilt = vfx_cued(
        down_tilt,
        0.09,
        "simultaneity_slice",
        (28.0, 14.0),
        SWING_FX,
        "vfx.patent_clerk.simultaneity_slice.loop",
    );
    let down_tilt = on_contact(down_tilt, "player.hit");

    // ── smashes: the FINISHERS ───────────────────────────────────────────────
    //
    // The hardest hits in the game on the longest commitments. A body this slow
    // gets about one per stock, so it must end things.
    let mut f_smash = strike(Strike {
        id: "smash_forward",
        clip: "smash_forward",
        startup_s: 0.38,
        active_s: 0.08,
        recover_s: 0.42,
        offset: (42.0, -4.0),
        half_extents: (30.0, 24.0),
        damage: 19,
        knockback: 175.0,
        knockback_growth: 3.30,
        launch_dir: Some((1.0, -0.45)),
        on_hit: None,
    });
    f_smash.smash_charge_mult = 1.7;
    let f_smash = vfx_at(f_smash, 0.06, "stamp_mass", (0.0, -8.0), STAMP_FX);
    let f_smash = vfx_at(
        f_smash,
        0.38,
        "mass_energy_exchange",
        (42.0, -4.0),
        PROOF_FX,
    );
    let f_smash = vfx_at(f_smash, 0.44, "stamp_energy", (42.0, -4.0), STAMP_FX);
    let f_smash = on_contact(f_smash, "player.hit");

    let mut up_smash = strike(Strike {
        id: "smash_up",
        clip: "smash_up",
        startup_s: 0.34,
        active_s: 0.10,
        recover_s: 0.38,
        offset: (6.0, -36.0),
        half_extents: (26.0, 32.0),
        damage: 17,
        knockback: 170.0,
        knockback_growth: 6.10,
        launch_dir: Some((0.0, -1.0)),
        on_hit: None,
    });
    up_smash.smash_charge_mult = 1.7;
    let up_smash = vfx_at(up_smash, 0.34, "light_cone", (6.0, -36.0), PROOF_FX);
    let up_smash = on_contact(up_smash, "player.hit");

    let mut down_smash = strike(Strike {
        id: "smash_down",
        clip: "smash_down",
        startup_s: 0.32,
        active_s: 0.11,
        recover_s: 0.40,
        offset: (0.0, 16.0),
        half_extents: (42.0, 13.0),
        damage: 15,
        knockback: 155.0,
        knockback_growth: 2.85,
        launch_dir: Some((0.95, -0.45)),
        on_hit: None,
    });
    down_smash.smash_charge_mult = 1.7;
    let down_smash = vfx_at(down_smash, 0.32, "clock_desync", (0.0, 16.0), SWING_FX);
    let down_smash = on_contact(down_smash, "player.hit");

    // ── aerials ──────────────────────────────────────────────────────────────
    //
    // Big and slow in the air too, with one exception below.
    let n_air = strike(Strike {
        id: "air_neutral",
        clip: "air_neutral",
        startup_s: 0.09,
        active_s: 0.14,
        recover_s: 0.18,
        offset: (0.0, 0.0),
        half_extents: (28.0, 26.0),
        damage: 7,
        knockback: 75.0,
        knockback_growth: 1.30,
        launch_dir: None,
        on_hit: None,
    });
    let n_air = vfx_at(
        n_air,
        0.09,
        "relative_velocity_arrows",
        (0.0, 0.0),
        SWING_FX,
    );
    let n_air = on_contact(n_air, "player.hit");

    let f_air = strike(Strike {
        id: "air_forward",
        clip: "air_forward",
        startup_s: 0.13,
        active_s: 0.09,
        recover_s: 0.22,
        offset: (32.0, -2.0),
        half_extents: (26.0, 20.0),
        damage: 10,
        knockback: 105.0,
        knockback_growth: 1.95,
        launch_dir: Some((1.0, -0.28)),
        on_hit: None,
    });
    let f_air = vfx_at(f_air, 0.13, "stamp_moving", (32.0, -2.0), SWING_FX);
    let f_air = on_contact(f_air, "player.hit");

    let b_air = strike(Strike {
        id: "air_back",
        clip: "air_back",
        startup_s: 0.15,
        active_s: 0.08,
        recover_s: 0.26,
        offset: (-34.0, 0.0),
        half_extents: (26.0, 20.0),
        damage: 12,
        knockback: 145.0,
        knockback_growth: 2.60,
        launch_dir: Some((-1.0, -0.35)),
        on_hit: None,
    });
    let b_air = vfx_at(b_air, 0.15, "stamp_at_rest", (-34.0, 0.0), SWING_FX);
    let b_air = on_contact(b_air, "player.hit");

    let u_air = strike(Strike {
        id: "air_up",
        clip: "air_up",
        startup_s: 0.10,
        active_s: 0.10,
        recover_s: 0.19,
        offset: (2.0, -32.0),
        half_extents: (22.0, 26.0),
        damage: 8,
        knockback: 95.0,
        knockback_growth: 1.90,
        launch_dir: Some((0.0, -1.0)),
        on_hit: None,
    });
    let u_air = vfx_at(u_air, 0.10, "light_cone", (2.0, -32.0), SWING_FX);
    let u_air = on_contact(u_air, "player.hit");

    // The exception, and where "at rest" shows up as a swing: it stops dead and
    // drops. Straight down, no drift, the heaviest spike in the game.
    let d_air = strike(Strike {
        id: "air_down",
        clip: "air_down",
        startup_s: 0.16,
        active_s: 0.09,
        recover_s: 0.28,
        offset: (4.0, 30.0),
        half_extents: (22.0, 22.0),
        damage: 13,
        knockback: 140.0,
        knockback_growth: 2.40,
        launch_dir: Some((0.0, 1.0)),
        on_hit: None,
    });
    let d_air = vfx_at(d_air, 0.16, "mass_energy_exchange", (4.0, 30.0), SWING_FX);
    let d_air = on_contact(d_air, "player.hit");

    // A forward tilt, so the most common press does not fall down the chain to
    // the jab. A margin correction: he reaches out and rewrites what you just
    // did.
    let f_tilt = strike(Strike {
        id: "tilt_forward",
        clip: "margin_correction",
        startup_s: 0.12,
        active_s: 0.09,
        recover_s: 0.22,
        offset: (34.0, -4.0),
        half_extents: (24.0, 16.0),
        damage: 7,
        knockback: 82.0,
        knockback_growth: 1.35,
        launch_dir: Some((1.0, -0.30)),
        on_hit: None,
    });
    let f_tilt = vfx_at(f_tilt, 0.12, "stamp_moving", (34.0, -4.0), SWING_FX);
    let f_tilt = on_contact(f_tilt, "player.hit");

    // NEUTRAL — `light_argument`. The speed of light is the same in every
    // frame: no impulse, no drift, a fixed cone that does not care what he was
    // doing when he threw it.
    let n_b = strike(Strike {
        id: "light_argument",
        clip: "light_argument",
        startup_s: 0.22,
        active_s: 0.12,
        recover_s: 0.32,
        offset: (36.0, -6.0),
        half_extents: (30.0, 20.0),
        damage: 11,
        knockback: 108.0,
        knockback_growth: 2.05,
        launch_dir: Some((0.9, -0.45)),
        on_hit: None,
    });
    // Set knockback: "the same in every frame". `fixed_knockback` zeroes the
    // growth on every volume, so the launch is the same at 0% and 150%.
    //
    // That makes it his set-up, not his finisher: it never kills, but it sends
    // them where he wants at any percent. The trade is real: he gives up a
    // neutral-B that scaled into a kill move.
    let n_b = fixed_knockback(n_b);
    let n_b = committed_tail(n_b, 0.66, 0.0);
    let n_b = vfx_at(n_b, 0.22, "light_cone", (36.0, -6.0), PROOF_FX);
    let n_b = on_contact(n_b, "player.hit");

    // Side: `reference_frame`. He declares a frame and moves in it. The impulse
    // fires on the active frame, not the press, and the tail is fully locked, so
    // the pass goes as far as it was going: no take-backs.
    //
    // It displaces him only. The reference-frame mechanic the module header
    // excludes stays out.
    let side_b = strike(Strike {
        id: "reference_frame",
        clip: "reference_frame",
        startup_s: 0.20,
        active_s: 0.11,
        recover_s: 0.28,
        offset: (30.0, 0.0),
        half_extents: (26.0, 22.0),
        damage: 11,
        knockback: 112.0,
        knockback_growth: 2.10,
        launch_dir: Some((0.9, -0.40)),
        on_hit: None,
    });
    let side_b = impulse(side_b, 0.20, (640.0, 0.0), ImpulseMode::Set);
    // Super armour: he goes through you. `MovePlayback` republishes <!-- cite-ok: records the deleted boolean `ArmorPolicy` replaced -->
    // `BodyCombat::armored` from the live window each tick, and `hit_reaction`
    // gates the launch on `!combat.armored`.
    //
    // It fits: a pass with "no take-backs" should not be cancelled by any jab,
    // and his theme is mass in motion.
    //
    // It is not invulnerability. He takes all the damage but not the launch,
    // hitstun or recoil lock. Counterplay: chip him out of it, or grab him.
    //
    // The pass only: `0.20..0.31`, from the impulse to the end of the active
    // window. His startup and locked tail stay punishable. This adds a window;
    // nothing else changes.
    let side_b = ambition_entity_catalog::authoring::armor(side_b, 0.20, 0.31);
    let side_b = committed_tail(side_b, 0.66, 0.0);
    let side_b = vfx_cued(
        side_b,
        0.20,
        "reference_frame_grid",
        (0.0, 0.0),
        PROOF_FX,
        "vfx.patent_clerk.reference_frame_grid.loop",
    );
    let side_b = vfx_at(
        side_b,
        0.34,
        "relative_velocity_arrows",
        (30.0, 0.0),
        SWING_FX,
    );
    let side_b = on_contact(side_b, "player.hit");

    // Up: `elevator_thought`, the recovery: a man in a rising lift cannot tell
    // it from gravity. He does not jump; his frame accelerates.
    let mut up_b = strike(Strike {
        id: "elevator_thought",
        clip: "elevator_thought",
        startup_s: ELEVATOR_AT_S,
        active_s: 0.12,
        recover_s: 0.20,
        offset: (0.0, 14.0),
        half_extents: (20.0, 32.0),
        damage: 9,
        knockback: 92.0,
        knockback_growth: 1.85,
        launch_dir: Some((0.0, -1.0)),
        on_hit: None,
    });
    // A heavyweight landing out of the lift pays for it. Offstage it costs
    // nothing.
    up_b.landing_lag_s = Some(0.34);
    let up_b = impulse(
        up_b,
        ELEVATOR_AT_S,
        (0.0, -ELEVATOR_SPEED),
        ImpulseMode::Set,
    );
    let up_b = committed_tail(up_b, ELEVATOR_ENDS_S, 0.0);
    let up_b = vfx_at(up_b, 0.06, "elevator_frame", (0.0, 0.0), PROOF_FX);
    let up_b = vfx_cued(
        up_b,
        ELEVATOR_AT_S,
        "proper_time_tick",
        (0.0, 10.0),
        SWING_FX,
        "vfx.patent_clerk.proper_time_tick.loop",
    );
    let up_b = on_contact(up_b, "player.hit");

    // DOWN — `synchronize_clocks`. Two clocks, one slice: a wide flat window
    // on the floor either side of him, and the stamp that says it is settled.
    let down_b = strike(Strike {
        id: "synchronize_clocks",
        clip: "synchronize_clocks",
        startup_s: 0.20,
        active_s: 0.13,
        recover_s: 0.32,
        offset: (0.0, 20.0),
        half_extents: (44.0, 12.0),
        damage: 8,
        knockback: 86.0,
        knockback_growth: 1.60,
        launch_dir: Some((0.8, -0.55)),
        on_hit: None,
    });
    let down_b = committed_tail(down_b, 0.65, 0.0);
    let down_b = vfx_at(down_b, 0.20, "clock_sync", (-30.0, 18.0), SWING_FX);
    let down_b = vfx_at(down_b, 0.26, "clock_desync", (30.0, 18.0), SWING_FX);
    let down_b = vfx_at(down_b, 0.33, "known_result_stamp", (0.0, 4.0), STAMP_FX);
    let down_b = on_contact(down_b, "player.hit");
    // A counter that desyncs the attacker's clock: the art has always drawn
    // `clock_sync` on one side and `clock_desync` on the other.
    //
    // The stance rides the windup, not the whole move. `live_counter_stance`
    // asks which window is live, so a stance over `0.0..0.20` is open while he
    // draws the slice, and the strike after is unchanged. Swinging into the
    // windup desyncs you; waiting gets an ordinary down-special.
    //
    // `Attacker`: this counter acts on the body that swung (`ParriedBodyHit`
    // carries it). Every other counter on the roster acts on its owner.
    //
    // Roster decision #19, Jon's to overrule: 0.35 for 0.45s. Long enough to
    // punish, short enough to be a read, not a stun. It slows their moves,
    // hurtbox resolution and animation, not their walking (see
    // `smash_time_dilation`).
    let down_b = {
        let mut spec = down_b;
        spec.windows.push(ambition_entity_catalog::MoveWindow {
            start_s: 0.0,
            end_s: 0.20,
            tag: ambition_entity_catalog::WindowTag::Active,
            volumes: Vec::new(),
            motion_scale: 1.0,
            sustain_effect: Some(ambition_entity_catalog::EffectRef {
                key: ambition_entity_catalog::smash_counter::COUNTER.to_string(),
                params: ambition_entity_catalog::ParamValue::from_typed(
                    &ambition_entity_catalog::smash_counter::CounterParams {
                        window_s: 0.05,
                        answers_the_attacker: true,
                        response: ambition_entity_catalog::smash_time_dilation::TIME_DILATION
                            .to_string(),
                        response_params: ambition_entity_catalog::ParamValue::from_typed(
                            &ambition_entity_catalog::smash_time_dilation::TimeDilationParams {
                                scale: 0.35,
                                seconds: 0.45,
                            },
                        )
                        .expect("the clerk's dilation params serialize"),
                        absorbs_projectiles: false,
                    },
                )
                .expect("the clerk's counter params serialize"),
            }),
        });
        spec
    };

    // Down-B has two forms, like Bowser's: a slam in the air, an arc and slam
    // on the ground. Context-dependent specials are acceptable, though most
    // should not be.
    //
    // A special gated to one posture is not answered in the other: the
    // directional chain falls through to the neutral special.
    // `special_air_down` comes before `special_down` in that chain.
    // Down, in the air: `falling_simultaneity`. He brings the slice down with
    // him.
    let mut air_down_b = strike(Strike {
        id: "falling_simultaneity",
        clip: "air_down",
        startup_s: 0.12,
        active_s: 0.10,
        recover_s: 0.26,
        offset: (0.0, 24.0),
        half_extents: (22.0, 22.0),
        damage: 10,
        knockback: 98.0,
        knockback_growth: 1.72,
        launch_dir: Some((0.0, 1.0)),
        on_hit: None,
    });
    air_down_b.landing_lag_s = Some(0.32);
    let air_down_b = impulse(air_down_b, 0.12, (0.0, 1250.0), ImpulseMode::Set);
    let air_down_b = vfx_at(air_down_b, 0.12, "clock_sync", (0.0, 20.0), SWING_FX);
    let air_down_b = on_contact(air_down_b, "player.hit");

    // Clerk's capture kit: unassuming and competent, which is the character.
    // The grab draws `attack`, not `grab`: these sheets publish no `grab` row,
    // and each table's clip guard refuses unpublished rows.
    let grab = author_standing_grab(
        grab_shell("clerk_grab", "attack", 0.07, 0.05, 0.21),
        CaptureAttemptParams {
            offset: (12.0, 1.0),
            half_extents: (18.0, 15.0),
            hold_offset: (13.0, 3.0),
        },
    );
    let pummel = author_pummel(
        capture_beat("clerk_pummel", "attack", 0.19),
        0.08,
        CapturePummelParams { damage: 3 },
    );
    let forward_throw = author_throw(
        capture_beat("clerk_fthrow", "attack", 0.26),
        0.13,
        CaptureThrowParams {
            damage: 8,
            knockback: 110.0,
            knockback_growth: 2.2,
            launch_dir: (0.8, -0.6),
        },
    );

    let back_throw = author_throw(
        capture_beat("clerk_bthrow", "attack", 0.28),
        0.14,
        CaptureThrowParams {
            damage: 9,
            knockback: 118.8,
            knockback_growth: 2.31,
            launch_dir: (-1.0, -0.37),
        },
    );

    let up_throw = author_throw(
        capture_beat("clerk_uthrow", "attack", 0.27),
        0.13,
        CaptureThrowParams {
            damage: 8,
            knockback: 114.4,
            knockback_growth: 2.24,
            launch_dir: (0.0, -1.0),
        },
    );

    let down_throw = author_throw(
        capture_beat("clerk_dthrow", "attack", 0.29),
        0.14,
        CaptureThrowParams {
            damage: 6,
            knockback: 81.4,
            knockback_growth: 1.76,
            launch_dir: (0.32, -0.92),
        },
    );
    SmashRepertoire {
        taunt: ambition_entity_catalog::authoring::taunt("patent_clerk_taunt", 0.9),
        dash_attack: ambition_entity_catalog::authoring::dash_attack(
            "patent_clerk_dash_attack",
            ambition_entity_catalog::authoring::DashAttackShape::GENRE,
            9,
            102.5,
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
        neutral_special: NeutralSpecial::Authored(n_b),
        side_special: side_b,
        up_special: UpSpecial::Standard(up_b),
        // Every smash fighter has a grab. The values are per character on purpose.
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

    /// The same launch for every observer. `fixed_knockback` zeroes the growth on
    /// every volume of `light_argument`.
    ///
    /// Asserts zero, not "small": `growth < 1.0` would pass a move that still
    /// scales.
    #[test]
    fn the_light_argument_launches_the_same_at_every_percent() {
        let set = patent_clerk_moveset();
        let argument = set
            .move_by_id("light_argument")
            .expect("light_argument exists");
        let volumes: Vec<_> = argument
            .windows
            .iter()
            .flat_map(|w| w.volumes.iter())
            .collect();
        assert!(!volumes.is_empty(), "it is still a strike");
        for volume in volumes {
            assert_eq!(
                volume.knockback_growth,
                Some(0.0),
                "a frame-dependent launch is the one thing the postulate forbids"
            );
        }
    }
    /// The clerk's counter desyncs the attacker's clock.
    ///
    /// The `answers_the_attacker` assertion is the key: a Witch-Time that slowed
    /// its own caster would be a self-inflicted stun.
    #[test]
    fn the_clerks_windup_desyncs_the_clock_of_whoever_swings_into_it() {
        use ambition_entity_catalog::smash_counter::{CounterParams, COUNTER};
        use ambition_entity_catalog::smash_time_dilation::{
            TimeDilationParams, TIME_DILATION,
        };
        let set = super::patent_clerk_moveset();
        let down = set
            .moves
            .iter()
            .find(|m| m.id == "synchronize_clocks")
            .expect("his down-B is in the table");
        let stance = down
            .windows
            .iter()
            .find_map(|w| {
                let effect = w.sustain_effect.as_ref()?;
                (effect.key == COUNTER).then(|| effect.params.hydrate::<CounterParams>())
            })
            .expect("his down-B holds no counter stance, so `clock_desync` is art again")
            .expect("the stance's params hydrate");

        assert!(
            stance.answers_the_attacker,
            "the stance answers its OWNER — a Witch-Time that slows its own caster \
             is a self-inflicted stun"
        );
        assert_eq!(
            stance.response, TIME_DILATION,
            "the stance answers with {} rather than a dilation",
            stance.response
        );
        let dilation = stance
            .response_params
            .hydrate::<TimeDilationParams>()
            .expect("the dilation params hydrate");
        assert!(
            dilation.problems().is_empty(),
            "the authored dilation is not one: {:?}",
            dilation.problems()
        );
        // A read, not a stun: long enough to punish, short enough that the slowed
        // fighter is still playing.
        assert!(
            dilation.seconds > 0.2 && dilation.seconds < 1.0,
            "a {}s slow is not a read", dilation.seconds
        );
    }

    use super::*;
    use ambition_entity_catalog::{MoveSpec, WindowTag};

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

    // Verb binding is checked by construction: `SmashRepertoire` owns the verb
    // strings and is a struct with no `Default`, so a missing slot is a compile
    // error. Coverage in every posture is checked by
    // `ambition_entity_catalog::smash_repertoire` and by
    // `smash_roster_movesets::report_the_smash_kit_every_selectable_fighter_has`.

    /// Heavyweight with finishers: the table must mean it.
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

    /// Controller: the tilts set up and do not finish. A tilt as strong as a
    /// smash would make the smash pointless, so the gap is asserted.
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

    /// Armour covers the crossing and nothing on either side. Armour over the
    /// whole move would make his startup and locked tail unpunishable, which is
    /// the price the pass must pay.
    #[test]
    fn his_pass_is_armoured_only_while_he_is_crossing() {
        let set = patent_clerk_moveset();
        let pass = find(&set, "reference_frame");
        let armor: Vec<(f32, f32)> = pass
            .windows
            .iter()
            .filter(|w| matches!(w.tag, WindowTag::Armor))
            .map(|w| (w.start_s, w.end_s))
            .collect();
        assert_eq!(armor.len(), 1, "expected exactly one armour window: {armor:?}");
        let (start, end) = armor[0];

        // The impulse fires at 0.20 and the active window ends at 0.31.
        assert!(
            (start - 0.20).abs() < 1e-4 && (end - 0.31).abs() < 1e-4,
            "armour runs {start}s..{end}s, not the pass"
        );
        // The startup is still punishable.
        assert!(start > 0.0, "armour covers his wind-up");
        // The locked tail is still a free hit: `committed_tail` runs the move to
        // 0.66s, so armour ending at 0.31 leaves a third of a second.
        assert!(
            end < pass.duration_s,
            "armour runs to {end}s on a {}s move, so his recovery is covered too",
            pass.duration_s
        );
    }

    /// He is still hit: armour is not i-frames. An added `Invuln` window would
    /// pass the test above.
    #[test]
    fn the_armoured_pass_grants_no_invulnerability() {
        let set = patent_clerk_moveset();
        let pass = find(&set, "reference_frame");
        assert!(
            !pass
                .windows
                .iter()
                .any(|w| matches!(w.tag, WindowTag::Invuln)),
            "the pass carries i-frames, so he is not taking the hit at all"
        );
    }
}
