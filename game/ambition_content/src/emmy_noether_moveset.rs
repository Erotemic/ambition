//! Emmy Ethereal's authored Smash repertoire.
//!
//! Her striking moves trade active duration against damage around
//! [`NOETHER_IMPULSE`], and forward/back aerials intentionally share the same
//! parameters. Her up-special is recovery-only. VFX rows normally derive their
//! cue name; rows whose audio cue uses a `.loop` suffix specify it explicitly with
//! `vfx_cued`.

use ambition_entity_catalog::authoring::Strike;
use ambition_entity_catalog::authoring::{SLASH_ARC_VFX, SLASH_POKE_VFX};
use ambition_entity_catalog::smash_capture::{
    author_pummel, author_standing_grab, author_throw, capture_beat, grab_shell,
    CaptureAttemptParams, CaptureCues, CapturePummelParams, CaptureThrowParams,
    SmashCaptureRepertoire,
};
use ambition_entity_catalog::smash_repertoire::{
    DownSpecial, NeutralSpecial, SmashRepertoire, UpSpecial,
};
use ambition_entity_catalog::{
    ClipBinding, HitVolume, ImpulseMode, MoveSpec, MoveWindow, MovesetContract, VolumeShape,
    WindowTag,
};

use ambition_entity_catalog::authoring::{
    committed_tail, impulse, on_contact, strike, strike_tag, vfx_at, vfx_cued,
};

/// Burst size by the kind of move that throws it, as multiples of the
/// presentation default (`ambition_render::fx::FX_DEFAULT_WORLD_SIZE`, a little
/// under a fighter's height).
///
/// A poke is a spark on a knuckle; a smash is the size of the swing; a field
/// reads as ground you cannot stand on. They must not share one size.
const POKE_FX: f32 = 0.55;
const SWING_FX: f32 = 0.75;
const SMASH_FX: f32 = 1.05;
const FIELD_FX: f32 = 1.30;

/// The conserved quantity: `damage x active_seconds`, in damage-seconds.
///
/// This is the character, not a fitted tuning constant. Retune Emmy by moving
/// a move along the curve (trade damage for window time, or the reverse),
/// never off it.
pub const NOETHER_IMPULSE: f32 = 0.90;

/// How far a move may sit off the invariant. Damage is an integer and time is
/// authored in hundredths, so exact products are not always possible; this is
/// the rounding, not a licence.
pub const INVARIANT_BAND: f32 = 0.12;

/// The launcher's growth. The blueprint calls `symmetry_break` *"the moment
/// the invariant stops holding"*, and it is the one move that grows like it.
pub const BREAK_GROWTH: f32 = 3.15;

/// What every other move of hers grows at, at most.
pub const ORDINARY_GROWTH: f32 = 1.95;

/// The rise her ethereal lift commands, in px/s.
///
/// A speed applied with [`ImpulseMode::Set`], so a falling Emmy climbs as far
/// as a standing one. An additive impulse is weakest exactly when it is all
/// that stands between her and the blast zone.
pub const LIFT_SPEED: f32 = 940.0;

/// When the lift takes hold, and when it lets go.
pub const LIFT_AT_S: f32 = 0.18;
/// Not a feel number. Under the engine baseline the lift climbs
/// `LIFT_SPEED^2 / 2g` in `LIFT_SPEED / g`; a tail shorter than twice that
/// returns her higher on every press, which is flight.
/// `the_lift_is_a_save_and_not_a_flight` checks the arithmetic.
pub const LIFT_ENDS_S: f32 = 1.16;

/// The active seconds a move keeps a box in the world, summed over its windows.
pub fn total_active_s(spec: &MoveSpec) -> f32 {
    spec.windows
        .iter()
        .filter(|w| matches!(w.tag, WindowTag::Active) && !w.volumes.is_empty())
        .map(|w| (w.end_s - w.start_s).max(0.0))
        .sum()
}

pub fn conserved_impulse(spec: &MoveSpec) -> f32 {
    let damage = spec
        .windows
        .iter()
        .flat_map(|w| w.volumes.iter())
        .map(|v| v.damage as f32)
        .fold(0.0_f32, f32::max);
    damage * total_active_s(spec)
}

/// One further term of the conservation field: the same box, later, at the
/// same strength. The gap makes it rehit: a window after a gap is a box that
/// went away and came back.
fn field_term(start_s: f32, end_s: f32) -> MoveWindow {
    let mut term = MoveWindow {
        start_s,
        end_s,
        tag: WindowTag::Active,
        volumes: Vec::new(),
        motion_scale: 1.0,
        sustain_effect: None,
    };
    term.volumes.push(HitVolume {
        // An ordinary hit, not a gust.
        shape: VolumeShape::Rect {
            offset: (0.0, 18.0),
            half_extents: (34.0, 20.0),
        },
        damage: 3,
        knockback: 58.0,
        knockback_growth: Some(1.35),
        launch_dir: Some((0.2, -0.9)),
        on_hit: None,
        vfx: Some(SLASH_POKE_VFX.to_string()),
        hit_sfx: None,
        reaction: None,
    });
    term
}

/// See the module doc. Sixteen moves: the genre's standard verb map, and every
/// clip is a row her rig publishes.
pub fn emmy_noether_moveset() -> MovesetContract {
    // ── the ground game ──────────────────────────────────────────────────────
    //
    // Every clip below is one of her authored rows. Where the sheet has no row
    // for a genre verb (no `attack_side`, no `smash_forward`), the move takes the
    // signature row that draws that idea (`generator_strike` for the forward
    // swing, `symmetry_break` for the smash), not a clip name that would fall
    // through to `idle`.

    // Five damage held out for nearly a fifth of a second: the cheap end of the
    // curve, and the easiest thing she has to land.
    let jab = strike(Strike {
        id: "jab",
        clip: "jab",
        startup_s: 0.05,
        active_s: 0.18,
        recover_s: 0.14,
        offset: (26.0, -6.0),
        half_extents: (16.0, 13.0),
        damage: 5,
        knockback: 44.0,
        knockback_growth: 1.05,
        launch_dir: None,
        on_hit: None,
    });
    let jab = strike_tag(jab, SLASH_POKE_VFX);
    let jab = vfx_at(jab, 0.05, "generator_steps", (26.0, -6.0), POKE_FX);

    // The committed swing the blueprint calls *"her fastest way to say no"*.
    // Nine damage buys a tenth of a second.
    let mut f_tilt = strike(Strike {
        id: "tilt_forward",
        clip: "generator_strike",
        startup_s: 0.10,
        active_s: 0.10,
        recover_s: 0.20,
        offset: (32.0, -4.0),
        half_extents: (22.0, 16.0),
        damage: 9,
        knockback: 76.0,
        knockback_growth: 1.55,
        launch_dir: Some((1.0, -0.30)),
        on_hit: None,
    });
    f_tilt.start_impulse = Some((130.0, 0.0));
    let f_tilt = vfx_at(f_tilt, 0.10, "generator_steps", (32.0, -4.0), SWING_FX);
    let f_tilt = vfx_at(f_tilt, 0.13, "symmetry_axis_snap", (32.0, -4.0), POKE_FX);
    let f_tilt = on_contact(f_tilt, "player.hit");

    let up_tilt = strike(Strike {
        id: "tilt_up",
        clip: "attack_up",
        startup_s: 0.09,
        active_s: 0.15,
        recover_s: 0.18,
        offset: (6.0, -26.0),
        half_extents: (17.0, 23.0),
        damage: 6,
        knockback: 70.0,
        knockback_growth: 1.50,
        launch_dir: Some((0.1, -1.0)),
        on_hit: None,
    });
    let up_tilt = vfx_cued(
        up_tilt,
        0.09,
        "group_orbit",
        (6.0, -26.0),
        SWING_FX,
        "vfx.noether.group_orbit.loop",
    );
    let up_tilt = on_contact(up_tilt, "player.hit");

    let down_tilt = strike(Strike {
        id: "tilt_down",
        clip: "attack_down",
        startup_s: 0.08,
        active_s: 0.15,
        recover_s: 0.18,
        offset: (24.0, 15.0),
        half_extents: (22.0, 11.0),
        damage: 6,
        knockback: 66.0,
        knockback_growth: 1.45,
        launch_dir: Some((0.9, -0.35)),
        on_hit: None,
    });
    let down_tilt = vfx_cued(
        down_tilt,
        0.08,
        "conserved_current",
        (24.0, 15.0),
        SWING_FX,
        "vfx.noether.conserved_current.loop",
    );
    let down_tilt = on_contact(down_tilt, "player.hit");

    // ── the smashes: the expensive end of the curve ──────────────────────────

    // The launcher. Fifteen damage for six hundredths of a second: the
    // narrowest window on the roster, and her only move that grows like a kill
    // move.
    let mut f_smash = strike(Strike {
        id: "smash_forward",
        clip: "symmetry_break",
        startup_s: 0.20,
        active_s: 0.06,
        recover_s: 0.34,
        offset: (36.0, -8.0),
        half_extents: (26.0, 22.0),
        damage: 15,
        knockback: 124.0,
        knockback_growth: BREAK_GROWTH,
        launch_dir: Some((1.0, -0.55)),
        on_hit: None,
    });
    f_smash.smash_charge_mult = 1.85;
    let f_smash = strike_tag(f_smash, SLASH_ARC_VFX);
    // The tell sits on her, not on the box: it is the wind-up, and the box does
    // not exist yet.
    let f_smash = vfx_at(f_smash, 0.02, "symmetry_axis_snap", (0.0, -10.0), SWING_FX);
    let f_smash = vfx_at(
        f_smash,
        0.20,
        "broken_symmetry_shards",
        (36.0, -8.0),
        SMASH_FX,
    );
    let f_smash = on_contact(f_smash, "player.hit");

    let mut up_smash = strike(Strike {
        id: "smash_up",
        clip: "smash_up",
        startup_s: 0.17,
        active_s: 0.075,
        recover_s: 0.30,
        offset: (2.0, -34.0),
        half_extents: (20.0, 28.0),
        damage: 12,
        knockback: 112.0,
        knockback_growth: ORDINARY_GROWTH,
        launch_dir: Some((0.0, -1.0)),
        on_hit: None,
    });
    up_smash.smash_charge_mult = 1.70;
    let up_smash = vfx_cued(
        up_smash,
        0.17,
        "group_orbit",
        (2.0, -34.0),
        SMASH_FX,
        "vfx.noether.group_orbit.loop",
    );
    let up_smash = on_contact(up_smash, "player.hit");

    // The down smash is the up smash reflected: same damage, window and growth,
    // opposite launch.
    let mut down_smash = strike(Strike {
        id: "smash_down",
        clip: "smash_down",
        startup_s: 0.17,
        active_s: 0.075,
        recover_s: 0.30,
        offset: (0.0, 20.0),
        half_extents: (34.0, 14.0),
        damage: 12,
        knockback: 112.0,
        knockback_growth: ORDINARY_GROWTH,
        launch_dir: Some((0.0, 1.0)),
        on_hit: None,
    });
    down_smash.smash_charge_mult = 1.70;
    let down_smash = vfx_cued(
        down_smash,
        0.17,
        "conserved_current",
        (0.0, 20.0),
        SMASH_FX,
        "vfx.noether.conserved_current.loop",
    );
    let down_smash = on_contact(down_smash, "player.hit");

    // ── the air game ─────────────────────────────────────────────────────────

    let mut n_air = strike(Strike {
        id: "air_neutral",
        clip: "air_neutral",
        startup_s: 0.08,
        active_s: 0.11,
        recover_s: 0.20,
        offset: (0.0, -6.0),
        half_extents: (26.0, 24.0),
        damage: 8,
        knockback: 78.0,
        knockback_growth: 1.60,
        launch_dir: Some((0.5, -0.75)),
        on_hit: None,
    });
    n_air.landing_lag_s = Some(0.16);
    n_air.autocancel_after_s = Some(0.30);
    let n_air = vfx_cued(
        n_air,
        0.08,
        "group_orbit",
        (0.0, -6.0),
        SWING_FX,
        "vfx.noether.group_orbit.loop",
    );
    let n_air = on_contact(n_air, "player.hit");

    // A fighter whose theorem is invariance does not get to care which way she is facing.
    let [f_air, b_air] = [
        ("air_forward", "air_forward", 1.0_f32),
        ("air_back", "air_back", -1.0_f32),
    ]
    .map(|(id, clip, dir_x)| {
        let mut aerial = strike(Strike {
            id: id,
            clip: clip,
            startup_s: 0.10,
            active_s: 0.10,
            recover_s: 0.22,
            offset: (28.0 * dir_x, -4.0),
            half_extents: (22.0, 18.0),
            damage: 9,
            knockback: 84.0,
            knockback_growth: 1.70,
            launch_dir: Some((dir_x, -0.45)),
            on_hit: None,
        });
        aerial.landing_lag_s = Some(0.18);
        aerial.autocancel_after_s = Some(0.32);
        let aerial = vfx_cued(
            aerial,
            0.10,
            "paired_trajectory",
            (28.0 * dir_x, -4.0),
            SWING_FX,
            "vfx.noether.paired_trajectory.loop",
        );
        on_contact(aerial, "player.hit")
    });

    let mut up_air = strike(Strike {
        id: "air_up",
        clip: "air_up",
        startup_s: 0.08,
        active_s: 0.15,
        recover_s: 0.19,
        offset: (2.0, -28.0),
        half_extents: (18.0, 24.0),
        damage: 6,
        knockback: 72.0,
        knockback_growth: 1.55,
        launch_dir: Some((0.0, -1.0)),
        on_hit: None,
    });
    up_air.landing_lag_s = Some(0.14);
    up_air.autocancel_after_s = Some(0.28);
    let up_air = vfx_at(up_air, 0.08, "equivalence_bridge", (2.0, -28.0), SWING_FX);
    let up_air = on_contact(up_air, "player.hit");

    // The spike. Ten damage buys the second-narrowest window she has.
    let mut d_air = strike(Strike {
        id: "air_down",
        clip: "air_down",
        startup_s: 0.12,
        active_s: 0.09,
        recover_s: 0.24,
        offset: (0.0, 24.0),
        half_extents: (18.0, 26.0),
        damage: 10,
        knockback: 96.0,
        knockback_growth: 1.80,
        launch_dir: Some((0.0, 1.0)),
        on_hit: None,
    });
    d_air.landing_lag_s = Some(0.26);
    d_air.autocancel_after_s = Some(0.34);
    let d_air = vfx_at(d_air, 0.12, "ether_cancel", (0.0, 24.0), SWING_FX);
    let d_air = on_contact(d_air, "player.hit");

    // ── THE FOUR SPECIALS ────────────────────────────────────────────────────

    // Neutral: `conservation_law`. She claims a piece of ground and it keeps
    // paying: three identical terms at even intervals. What the field returns
    // does not decay, which is the conservation idea as a move.
    //
    // The engine has a counter technique (`smash.counter`, with
    // `CounterParams { response, absorbs_projectiles }`), and her down-B uses it.
    // The neutral stays a field on purpose; a later design call can overrule
    // that. See `docs/planning/engine/expressive-move-capabilities.md`.
    let n_b = strike(Strike {
            id: "conservation_law",
            clip: "conservation_law",
            startup_s: 0.16,
            active_s: 0.10,
            // Long enough to contain the two further terms below (they end at
            recover_s:
        // 0.66). Windows pushed after construction do not extend the move; the
        // builder's `debug_assert` checks this.
        0.44,
            offset: (0.0, 18.0),
            half_extents: (34.0, 20.0),
            damage: 3,
            knockback: 58.0,
            knockback_growth: 1.35,
            launch_dir: Some((0.2, -0.9)),
            on_hit: None,
        });
    let mut n_b = strike_tag(n_b, SLASH_POKE_VFX);
    // Even gaps, identical terms — the invariant holding, three times.
    n_b.windows.push(field_term(0.36, 0.46));
    n_b.windows.push(field_term(0.56, 0.66));
    debug_assert!(
        n_b.duration_s >= 0.66,
        "the last term of the field must fit inside the move"
    );
    let n_b = vfx_cued(
        n_b,
        0.0,
        "invariant_core",
        (0.0, 0.0),
        SWING_FX,
        "vfx.noether.invariant_core.loop",
    );
    let n_b = vfx_cued(
        n_b,
        0.16,
        "conserved_pair_exchange",
        (0.0, 18.0),
        FIELD_FX,
        "vfx.noether.conserved_pair_exchange.loop",
    );
    let n_b = vfx_at(n_b, 0.56, "conservation_transfer", (0.0, 18.0), FIELD_FX);
    let n_b = vfx_at(n_b, 0.66, "proof_complete", (0.0, 0.0), SMASH_FX);
    let n_b = on_contact(n_b, "player.hit");

    // Side: `symmetry_shift`. A lateral displacement that keeps her facing.
    //
    // The impulse is negative, and that is the move. Body-local x runs toward
    // her facing, so a negative one carries her away from what she faces without
    // turning her: the blueprint's *"reposition without conceding the
    // neutral"*. It buys distance and keeps the threat pointed where it was.
    let side_b = strike(Strike {
        id: "symmetry_shift",
        clip: "symmetry_shift",
        startup_s: 0.14,
        active_s: 0.225,
        recover_s: 0.28,
        offset: (18.0, -2.0),
        half_extents: (24.0, 20.0),
        damage: 4,
        knockback: 62.0,
        knockback_growth: 1.40,
        launch_dir: Some((-0.6, -0.55)),
        on_hit: None,
    });
    let side_b = impulse(side_b, 0.14, (-640.0, 0.0), ImpulseMode::Set);
    let side_b = committed_tail(side_b, 0.62, 0.55);
    let side_b = vfx_at(side_b, 0.14, "equivalence_bridge", (18.0, -2.0), SWING_FX);
    // The trail she leaves behind her: the half of a retreat a watcher must
    // see.
    let side_b = vfx_cued(
        side_b,
        0.30,
        "paired_trajectory",
        (34.0, -2.0),
        SWING_FX,
        "vfx.noether.paired_trajectory.loop",
    );
    let side_b = on_contact(side_b, "player.hit");

    // Up: `ethereal_lift`. The recovery, and it does not attack.
    //
    // The blueprint asked for this: *"Rises, does not attack: the traversal
    // motif, not a second offensive option"*. Other recoveries carry a box, so an
    // edgeguard is a trade. Hers is pure traversal: she cannot trade with the
    // edgeguarder, only beat them. The cost is deliberate, and it is why her side
    // special buys distance, not damage.
    let mut up_b = MoveSpec {
        display_name: None,
        id: "ethereal_lift".to_string(),
        // The same structural fallback chain every `strike` authors: a recovery
        // that cannot find its row must still run.
        clip: ClipBinding {
            clip: "ethereal_lift".to_string(),
            fallbacks: vec!["jump".to_string(), "fall".to_string(), "idle".to_string()],
        },
        duration_s: LIFT_ENDS_S,
        windows: vec![MoveWindow {
            start_s: 0.0,
            end_s: LIFT_AT_S,
            tag: WindowTag::Startup,
            volumes: Vec::new(),
            motion_scale: 1.0,
            sustain_effect: None,
        }],
        events: Vec::new(),
        // The slot owns the posture (`SmashRepertoire` sets it from `up_special`);
        // a struct literal must name every field.
        gates: Default::default(),
        start_impulse: None,
        smash_charge_mult: 1.0,
        charge_gesture: ambition_entity_catalog::ChargeGesture::default(),
        smash_charge: None,
        repeat: None,
        landing_lag_s: Some(0.24),
        autocancel_after_s: None,
        sprite_spin_hz: None,
        equips: None,
        flow: None,
    };
    up_b.windows.push(MoveWindow {
        start_s: LIFT_AT_S,
        end_s: LIFT_ENDS_S,
        tag: WindowTag::Recovery,
        volumes: Vec::new(),
        // Enough authority to choose where she lands and none to cancel: the same
        // helpless tail every recovery pays.
        motion_scale: 0.14,
        sustain_effect: None,
    });
    let up_b = impulse(up_b, LIFT_AT_S, (0.0, -LIFT_SPEED), ImpulseMode::Set);
    let up_b = vfx_cued(
        up_b,
        0.04,
        "invariant_core",
        (0.0, 6.0),
        SWING_FX,
        "vfx.noether.invariant_core.loop",
    );
    let up_b = vfx_cued(
        up_b,
        LIFT_AT_S,
        "group_orbit",
        (0.0, 0.0),
        SMASH_FX,
        "vfx.noether.group_orbit.loop",
    );
    let up_b = vfx_cued(
        up_b,
        0.62,
        "conserved_current",
        (0.0, 20.0),
        SWING_FX,
        "vfx.noether.conserved_current.loop",
    );

    // Down: `invariant_field`, the counter her blueprint asked for, built on the
    // shared `smash.counter` technique.
    //
    // The response is the theorem: a symmetry implies a conserved quantity, so
    // energy put in is not destroyed. Her answer to being struck is
    // `smash.vitality`: she keeps it. No two counters on the roster are alike.
    //
    // She absorbs, not reflects. Returning the shot would be conservation of
    // momentum, which George's riposte already does. One conservation law per
    // move.
    //
    // The cost: the displaced move was a low wide poke at `damage: 6`, her
    // cheapest special, and `conservation_law` still claims ground. The kit loses
    // a duplicate and gains a defensive option.
    //
    // The heal is small on purpose. A parry is already a full punish window;
    // three points is a reason to take the read, not to turtle.
    let down_b = ambition_entity_catalog::smash_counter::counter_move(
        "invariant_field",
        // Her own clip: the art is a field closing, which is still what the move
        // looks like.
        "invariant_field",
        0.14,
        // The old active window, kept as the stance. 0.15s (nine frames at 60Hz) is
        // a real read; the 0.05s windows elsewhere would be a guess.
        0.15,
        0.30,
        ambition_entity_catalog::smash_counter::CounterParams {
            // A heartbeat, not a duration: `parry_window_timer` decays and the stance
            // re-arms it every live frame.
            window_s: 0.05,
            // Its own answer, as every counter but the clerk's is.
            answers_the_attacker: false,
            response: ambition_entity_catalog::smash_vitality::VITALITY.to_string(),
            response_params: ambition_entity_catalog::ParamValue::from_typed(
                &ambition_entity_catalog::smash_vitality::VitalityParams {
                    change: 3,
                    // Ignored by a restore: the floor bounds a price, and this is a gain.
                    // Set at the type's default.
                    floor: 1,
                    // Her own rows. `conserved_current` is the effect her neutral and her
                    // recovery both use for the theorem.
                    vfx: "conserved_current".to_string(),
                    sfx: "player.attack.charge".to_string(),
                },
            )
            .expect("the invariant field's vitality params serialize"),
            absorbs_projectiles: true,
        },
    );
    let down_b = vfx_cued(
        down_b,
        0.14,
        "invariant_core",
        (14.0, 20.0),
        FIELD_FX,
        "vfx.noether.invariant_core.loop",
    );
    let down_b = vfx_cued(
        down_b,
        0.22,
        "conserved_current",
        (14.0, 20.0),
        FIELD_FX,
        "vfx.noether.conserved_current.loop",
    );
    let down_b = on_contact(down_b, "player.hit");

    // Down-B has two forms, like Bowser's: a slam in the air, an arc and slam
    // on the ground. Context-dependent specials are acceptable, though most
    // should not be.
    //
    // A special gated to one posture is not answered in the other: the
    // directional chain falls through to the neutral special.
    // `special_air_down` comes before `special_down` in that chain.
    // Down, in the air. The grounded form needs a floor; this one brings the
    // symmetry down with her.
    let mut air_down_b = strike(Strike {
        id: "falling_invariant",
        clip: "air_down",
        startup_s: 0.10,
        active_s: 0.09,
        recover_s: 0.24,
        offset: (0.0, 23.0),
        half_extents: (21.0, 21.0),
        damage: 10,
        knockback: 98.0,
        knockback_growth: 1.74,
        launch_dir: Some((0.0, 1.0)),
        on_hit: None,
    });
    air_down_b.landing_lag_s = Some(0.28);
    let air_down_b = impulse(air_down_b, 0.10, (0.0, 1200.0), ImpulseMode::Set);
    // This table's rule: every burst is heard.
    let air_down_b = vfx_cued(
        air_down_b,
        0.10,
        "conserved_current",
        (0.0, 20.0),
        FIELD_FX,
        "vfx.noether.conserved_current.loop",
    );
    let air_down_b = on_contact(air_down_b, "player.hit");

    // Emmy's capture kit. The steepest growth after the automaton: weak early,
    // decisive late. What her throw takes out of you returns with interest at high
    // percent. Her sheet ships `grab`, `grab_hold` and `grab_release`, so the kit
    // draws those rows.
    let grab = author_standing_grab(
        grab_shell("emmy_grab", "grab", 0.07, 0.06, 0.21),
        CaptureAttemptParams {
            offset: (12.0, 1.0),
            half_extents: (20.0, 15.0),
            hold_offset: (13.0, 3.0),
        },
    );
    let pummel = author_pummel(
        capture_beat("emmy_pummel", "grab_hold", 0.2),
        0.09,
        CapturePummelParams { damage: 4 },
    );
    let forward_throw = author_throw(
        capture_beat("emmy_fthrow", "grab_release", 0.28),
        0.15,
        CaptureThrowParams {
            damage: 9,
            knockback: 108.0,
            knockback_growth: 2.5,
            launch_dir: (0.6, -0.8),
        },
    );

    let back_throw = author_throw(
        capture_beat("emmy_bthrow", "grab_release", 0.3),
        0.16,
        CaptureThrowParams {
            damage: 10,
            knockback: 116.64,
            knockback_growth: 2.62,
            launch_dir: (-1.0, -0.5),
        },
    );

    let up_throw = author_throw(
        capture_beat("emmy_uthrow", "grab_release", 0.29),
        0.15,
        CaptureThrowParams {
            damage: 9,
            knockback: 112.32,
            knockback_growth: 2.55,
            launch_dir: (0.0, -1.0),
        },
    );

    let down_throw = author_throw(
        capture_beat("emmy_dthrow", "grab_release", 0.31),
        0.16,
        CaptureThrowParams {
            damage: 7,
            knockback: 79.92,
            knockback_growth: 2.0,
            launch_dir: (0.24, -0.92),
        },
    );
    let repertoire = SmashRepertoire {
        taunt: ambition_entity_catalog::authoring::taunt("emmy_noether_taunt", 0.9),
        dash_attack: ambition_entity_catalog::authoring::dash_attack(
            "emmy_noether_dash_attack",
            ambition_entity_catalog::authoring::DashAttackShape::GENRE,
            11,
            95.0,
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
        up_air,
        down_air: d_air,
        neutral_special: NeutralSpecial::Authored(n_b),
        side_special: side_b,
        up_special: UpSpecial::Standard(up_b),
        // Every smash fighter has a grab. The values are per character on purpose.
        capture: SmashCaptureRepertoire {
            // Her own sheet's effects: her kit test requires every effect to come off
            // it, so a shared `classic_burst` would fail.
            cues: CaptureCues {
                reach: "symmetry_axis_snap",
                impact: "conservation_transfer",
                release: "equivalence_bridge",
            },
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
    .into_contract();

    // The invariant is checked where it is authored, the one place that holds
    // the whole table.
    debug_assert!(
        repertoire
            .moves
            .iter()
            .filter(|m| conserved_impulse(m) > 0.0)
            .all(|m| (conserved_impulse(m) - NOETHER_IMPULSE).abs() <= INVARIANT_BAND),
        "a Noether move left the conservation curve"
    );

    repertoire
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_entity_catalog::MoveEventKind;

    fn find(set: &MovesetContract, id: &str) -> MoveSpec {
        set.moves
            .iter()
            .find(|m| m.id == id)
            .unwrap_or_else(|| panic!("{id} exists"))
            .clone()
    }

    fn growth(m: &MoveSpec) -> f32 {
        m.windows
            .iter()
            .flat_map(|w| w.volumes.iter())
            .filter_map(|v| v.knockback_growth)
            .fold(0.0f32, f32::max)
    }

    fn damage(m: &MoveSpec) -> i32 {
        m.windows
            .iter()
            .flat_map(|w| w.volumes.iter())
            .map(|v| v.damage)
            .fold(0, i32::max)
    }

    fn knockback(m: &MoveSpec) -> f32 {
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

    /// The symmetry, with every other fighter as the negative control.
    ///
    /// Her forward and back aerials must be the same move except for direction.
    /// Oiler's and the goblin's pairs differ, which makes this a claim about Emmy,
    /// not about aerials in general.
    #[test]
    fn her_forward_and_back_aerials_are_the_same_move() {
        let set = crate::authored_movesets::shipped("npc_emmy_noether");
        let f = find(&set, "air_forward");
        let b = find(&set, "air_back");
        assert_eq!(damage(&f), damage(&b), "same damage");
        assert_eq!(knockback(&f), knockback(&b), "same knockback");
        assert_eq!(growth(&f), growth(&b), "same growth");
        assert_eq!(f.duration_s, b.duration_s, "same clock");
        assert_eq!(
            total_active_s(&f),
            total_active_s(&b),
            "same time in the world"
        );

        // The negative control: nobody else on the grid is symmetric.
        let oiler = crate::authored_movesets::shipped("npc_oiler");
        let of = find(&oiler, "air_forward");
        let ob = find(&oiler, "air_back");
        assert!(
            damage(&of) != damage(&ob) || knockback(&of) != knockback(&ob),
            "Oiler's aerial pair differs — if it stopped differing this test \
             would stop being about Emmy"
        );
    }

    /// The conserved quantity.
    ///
    /// Every striking move sits on `damage x active_seconds =` [`NOETHER_IMPULSE`]
    /// within [`INVARIANT_BAND`]. The band is not vacuous: Oiler's table, authored
    /// from the opposite idea, must miss it.
    #[test]
    fn every_strike_she_throws_conserves_the_same_quantity() {
        let set = crate::authored_movesets::shipped("npc_emmy_noether");
        let mut striking = 0;
        for m in &set.moves {
            let impulse = conserved_impulse(m);
            if impulse == 0.0 {
                continue;
            }
            striking += 1;
            assert!(
                (impulse - NOETHER_IMPULSE).abs() <= INVARIANT_BAND,
                "`{}` is at {impulse:.3} damage-seconds, off the curve at \
                 {NOETHER_IMPULSE} +/- {INVARIANT_BAND}",
                m.id
            );
        }
        assert!(striking >= 14, "only {striking} moves strike at all");

        let oiler = crate::authored_movesets::shipped("npc_oiler");
        let off_curve = oiler
            .moves
            .iter()
            .map(conserved_impulse)
            .filter(|i| *i > 0.0 && (i - NOETHER_IMPULSE).abs() > INVARIANT_BAND)
            .count();
        assert!(
            off_curve >= 6,
            "only {off_curve} of Oiler's moves miss Emmy's curve, so the curve is \
             a description of fighters in general rather than of her"
        );
    }

    /// The launcher is the one move whose growth leaves the ordinary band: the
    /// blueprint's *"the moment the invariant stops holding"*.
    #[test]
    fn exactly_one_move_grows_like_a_kill_move() {
        let set = crate::authored_movesets::shipped("npc_emmy_noether");
        let loud: Vec<&str> = set
            .moves
            .iter()
            .filter(|m| growth(m) > ORDINARY_GROWTH)
            .map(|m| m.id.as_str())
            .collect();
        assert_eq!(loud, ["smash_forward"], "one break, and it is the break");
        assert_eq!(growth(&find(&set, "smash_forward")), BREAK_GROWTH);
    }

    /// Her recovery does not attack, and nothing else on the grid is like it.
    ///
    /// The blueprint asked for this. The negative control is Oiler's geyser,
    /// which carries a box.
    #[test]
    fn her_recovery_carries_no_hitbox_and_that_is_unusual() {
        let set = crate::authored_movesets::shipped("npc_emmy_noether");
        let lift = find(&set, "ethereal_lift");
        assert!(
            lift.windows.iter().all(|w| w.volumes.is_empty()),
            "the ethereal lift is a traversal, not a second offensive option"
        );
        assert_eq!(conserved_impulse(&lift), 0.0);

        let oiler = crate::authored_movesets::shipped("npc_oiler");
        let geyser = find(&oiler, "oil_geyser");
        assert!(
            geyser.windows.iter().any(|w| !w.volumes.is_empty()),
            "Oiler's recovery hits — if it stopped, hers would no longer be the \
             one that gives that up"
        );
    }

    /// The lift is a save, not flight, held by arithmetic, not by a cooldown,
    /// like Oiler's geyser.
    #[test]
    fn the_lift_is_a_save_and_not_a_flight() {
        // Engine baseline gravity, the same number the geyser's guard uses.
        const G: f32 = 2200.0;
        let climb_s = LIFT_SPEED / G;
        assert!(
            LIFT_ENDS_S >= 2.0 * climb_s,
            "the lift ends at {LIFT_ENDS_S}s but its own arc takes {:.2}s up and \
             the same down, so repeated presses would gain height",
            climb_s
        );
        let set = crate::authored_movesets::shipped("npc_emmy_noether");
        let lift = find(&set, "ethereal_lift");
        assert!(
            lift.windows
                .iter()
                .all(|w| !matches!(w.tag, WindowTag::Cancelable { .. })),
            "a cancelable window would let her re-press before the arc is spent"
        );
    }

    /// The side special buys distance BACKWARD without turning her round.
    #[test]
    fn the_symmetry_shift_retreats_without_conceding_the_facing() {
        let set = crate::authored_movesets::shipped("npc_emmy_noether");
        let shift = find(&set, "symmetry_shift");
        let displacement = shift
            .events
            .iter()
            .find_map(|e| match &e.kind {
                MoveEventKind::Impulse { local, .. } => Some(*local),
                _ => None,
            })
            .expect("the shift commands a displacement");
        assert!(
            displacement.0 < 0.0,
            "body-local +x is her facing, so a retreat that keeps the facing has \
             to be negative; got {displacement:?}"
        );
    }

    // Burst sound is guarded by `a_paired_burst_is_heard_exactly_once`
    // (`src/moveset_sound.rs`). It drives these tables through the real
    // dispatcher and fan-out and counts what reaches the SFX channel, so it
    // catches both silence and double-play.

    /// The art is hers, and it all ships.
    ///
    /// The oracle is the art: `is_authored_effect` reads the rows out of the
    /// baked manifests, so this asks what the renderer will ask.
    #[test]
    fn the_kit_looks_like_emmy_and_the_art_all_ships() {
        let set = crate::authored_movesets::shipped("npc_emmy_noether");
        let mut effects = std::collections::BTreeSet::new();
        // Collect problems across every move, then assert once, so one run reports
        // every move that references a renamed effect.
        let mut problems: Vec<String> = Vec::new();
        for m in &set.moves {
            problems.extend(m.presentation_problems(
                ambition_platformer2d::sprite_sheet::fx::is_authored_effect,
            ));
            for ev in &m.events {
                if let MoveEventKind::Vfx { effect, .. } = &ev.kind {
                    effects.insert(effect.clone());
                }
            }
        }
        // Before the palette checks below: a renamed effect makes those fail too,
        // with a message about breadth, not the rename.
        assert!(problems.is_empty(), "{problems:?}");
        assert_eq!(
            effects.len(),
            12,
            "all twelve of her rendered rows are bound, and nothing else is: \
             {effects:?}"
        );
        for effect in &effects {
            let authored = ambition_platformer2d::sprite_sheet::fx::authored_effect(effect)
                .unwrap_or_else(|| panic!("`{effect}` ships"));
            assert!(
                authored.sheet.contains("noether"),
                "`{effect}` is drawn off `{}`, which is not Emmy's sheet",
                authored.sheet
            );
        }
    }

    /// Every clip she names is a row her rig actually publishes.
    ///
    /// The fallback chain makes a missing clip silent: the move still runs,
    /// drawn as `idle`. Right at runtime, wrong for authoring, so the table is
    /// checked against the sheet here.
    #[test]
    fn every_clip_names_a_row_her_sheet_carries() {
        let set = crate::authored_movesets::shipped("npc_emmy_noether");
        let record =
            ambition_platformer2d::sprite_sheet::character::sheets::record_for_sheet_key("noether")
                .expect("Emmy's sheet is baked into the registry");
        let rows: std::collections::BTreeSet<&str> =
            record.rows.iter().map(|r| r.animation.as_str()).collect();
        for m in &set.moves {
            assert!(
                rows.contains(m.clip.clip.as_str()),
                "`{}` draws `{}`, which her sheet does not publish — it would \
                 fall down the chain to `idle`",
                m.id,
                m.clip.clip
            );
        }
    }

    /// The theorem is the move: the answer to being struck is that the energy is
    /// kept. A test that only found a counter would pass against one that
    /// answered with a grab.
    #[test]
    fn her_field_answers_a_blow_by_conserving_it() {
        let set = crate::authored_movesets::shipped("npc_emmy_noether");
        let field = set
            .moves
            .iter()
            .find(|m| m.id == "invariant_field")
            .expect("her grounded down special");

        let params: ambition_entity_catalog::smash_counter::CounterParams = field
            .windows
            .iter()
            .filter_map(|w| w.sustain_effect.as_ref())
            .find(|e| e.key == ambition_entity_catalog::smash_counter::COUNTER)
            .expect("the field holds a counter stance")
            .params
            .hydrate()
            .expect("counter params hydrate");

        assert_eq!(
            params.response,
            ambition_entity_catalog::smash_vitality::VITALITY,
            "she must answer by keeping the energy, not by grabbing or leaving"
        );
        let gain: ambition_entity_catalog::smash_vitality::VitalityParams = params
            .response_params
            .hydrate()
            .expect("vitality params hydrate");
        assert!(
            gain.change > 0,
            "a conservation law that COSTS her health is the opposite of the move: {}",
            gain.change
        );
        // Small: a parry is already a full punish window.
        assert!(
            gain.change <= 5,
            "the heal is worth turtling for: {}",
            gain.change
        );

        // She absorbs, not reflects: returning the shot is conservation of
        // momentum, which is George's riposte.
        assert!(
            params.absorbs_projectiles,
            "she returns shots, which is the other fighter's law"
        );

        // The stance is a real read: her old active window was 0.15s (nine
        // frames). The 0.05s stances elsewhere are for fighters who answer fast.
        let stance = field
            .windows
            .iter()
            .find(|w| w.sustain_effect.is_some())
            .expect("a stance window");
        assert!(
            stance.end_s - stance.start_s >= 0.12,
            "the stance is {}s, which is a guess rather than a read",
            stance.end_s - stance.start_s
        );
    }
}
