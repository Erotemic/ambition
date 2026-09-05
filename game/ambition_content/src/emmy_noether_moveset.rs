//! Emmy Ethereal's authored Smash repertoire.
//!
//! Her striking moves trade active duration against damage around
//! [`NOETHER_IMPULSE`], and forward/back aerials intentionally share the same
//! parameters. Her up-special is recovery-only. VFX rows normally derive their
//! cue name; rows whose audio cue uses a `.loop` suffix specify it explicitly with
//! `vfx_cued`.

use ambition_characters::moveset_authoring::Strike;
use ambition_characters::moveset_prefabs::{SLASH_ARC_VFX, SLASH_POKE_VFX};
use ambition_characters::smash_capture::{
    author_pummel, author_standing_grab, author_throw, capture_beat, grab_shell,
    CaptureAttemptParams, CaptureCues, CapturePummelParams, CaptureThrowParams,
    SmashCaptureRepertoire,
};
use ambition_characters::smash_repertoire::{
    DownSpecial, NeutralSpecial, SmashRepertoire, UpSpecial,
};
use ambition_platformer2d::entity_catalog::{
    ClipBinding, HitVolume, ImpulseMode, MoveSpec, MoveWindow, MovesetContract, VolumeShape,
    WindowTag,
};

use ambition_characters::moveset_authoring::{
    committed_tail, impulse, on_contact, strike, strike_tag, vfx_at, vfx_cued,
};

/// How big a burst is, by what kind of move throws it — multiples of the
/// presentation default (`ambition_render::fx::FX_DEFAULT_WORLD_SIZE`, a little
/// under a fighter's height).
///
/// right now we are seeing crazy upscaled vfx"*. A poke is a spark on a
/// knuckle; a smash is the size of the swing; a field is meant to be read as
/// ground you cannot stand on. They are not the same size, and until this week
/// every effect in the project drew at one.
const POKE_FX: f32 = 0.55;
const SWING_FX: f32 = 0.75;
const SMASH_FX: f32 = 1.05;
const FIELD_FX: f32 = 1.30;

/// The conserved quantity: `damage x active_seconds`, in damage-seconds.
///
/// this is the character, not a tuning constant fitted to the numbers after
/// the fact. Retuning Emmy means moving a move ALONG the curve — buy damage with
/// window time, or window time with damage — never off it. A move that broke the
/// invariant would be a move her theorem does not describe.
pub const NOETHER_IMPULSE: f32 = 0.90;

/// How far a move may sit off the invariant. Damage is an integer and time is
/// authored in hundredths, so exact products are not available at every point on
/// the curve; this is the rounding, not a licence.
pub const INVARIANT_BAND: f32 = 0.12;

/// The launcher's growth. The blueprint calls `symmetry_break` *"the moment the
/// invariant stops holding"*, and it is the one move that grows like it.
pub const BREAK_GROWTH: f32 = 3.15;

/// What every other move of hers grows at, at most.
pub const ORDINARY_GROWTH: f32 = 1.95;

/// The rise her ethereal lift commands, in px/s.
///
/// authored as a SPEED and applied with [`ImpulseMode::Set`], for the reason
/// every recovery in this repo is: an Emmy pressing this at terminal velocity
/// gets exactly the climb a standing one does. An additive impulse is weakest
/// precisely when it is the only thing between her and the blast zone.
pub const LIFT_SPEED: f32 = 940.0;

/// When the lift takes hold, and when it lets go.
pub const LIFT_AT_S: f32 = 0.18;
/// not a feel number. Under the engine baseline the lift climbs
/// `LIFT_SPEED^2 / 2g` and takes `LIFT_SPEED / g` to do it; a tail shorter than
/// twice that hands her back above where it found her on every press, which is
/// flight rather than a recovery. `the_lift_is_a_save_and_not_a_flight` holds
/// the arithmetic.
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

/// One further term of the conservation field: the same box, later, at the same
/// strength. The GAP is what makes it rehit — a window that starts after a gap
/// is a box that went away and came back.
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

/// See the module doc. Sixteen moves: the genre's standard verb map, every clip
/// a row her rig actually publishes.
pub fn emmy_noether_moveset() -> MovesetContract {
    // ── the ground game ──────────────────────────────────────────────────────
    //
    // every clip below is one of her 123 authored rows. Where the sheet has
    // no row for a genre verb (she has no `attack_side` and no `smash_forward`),
    // the move takes the SIGNATURE row that draws that idea — `generator_strike`
    // for the committed forward swing, `symmetry_break` for the smash — rather
    // than a clip name that would fall down the structural chain to `idle`.

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

    // The launcher. Fifteen damage for six hundredths of a second — the
    // narrowest window any authored fighter here asks a player to hit, and the
    // only move of hers that grows like a kill move.
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
    // The tell sits on HER, not on the box — it is the wind-up, and the box
    // does not exist yet.
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

    // the down smash is the up smash REFLECTED: same damage, same window, same
    // growth, opposite launch. Her table is meant to look like this.
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

    // NEUTRAL — `conservation_law`. She claims a piece of ground and it keeps
    // paying. Three terms at even intervals, identical every time: what the field
    // returns does not decay, which is the conservation idea stated as a move.
    //
    // ⛔⛔ THIS NOTE RECORDED A DEFERRAL AND THE DEFERRAL HAS EXPIRED. It read:
    // *"not the counter the sheet's blueprint imagined — `MoveSpec` has no
    // absorb or reflect, and inventing one for one character would be the wrong
    // shape."* Both halves were true when written and neither is true now.
    //
    // ⭐ AS OF 2026-09-05 THE ENGINE HAS BOTH, and not as one character's
    // invention: `smash.counter` carries `CounterParams { response,
    // absorbs_projectiles }`, where the response is an ARBITRARY technique and
    // the flag chooses absorb-versus-reflect. Three fighters author one — George
    // answers a parry with a grab, the Author with an ambush teleport, the Shadow
    // Oni with a sleep pulse — so the "wrong shape" objection is answered by the
    // thing being shared rather than bespoke.
    //
    // ⚠ HER MOVE IS DELIBERATELY LEFT AS THE FIELD, and that is a decision to be
    // overruled rather than an oversight. `conservation_law` is richly authored —
    // three even terms and four cues — and swapping it out is a design call on a
    // fighter whose blueprint I have not read. ⇒ What is fixed here is the false
    // sentence: somebody reading this file should learn that the counter is
    // AVAILABLE, not that the engine refuses it. See
    // `docs/planning/demos/campaigns/expressive-moves-2026-09-05.md`.
    //
    // ⭐ AND THE MOVE BELOW IS STILL WORTH ITS OWN DEFENCE: three terms at even
    // intervals, identical every time, is the conservation idea stated AS A MOVE
    // — which a counter would not be.
    let n_b = strike(Strike {
            id: "conservation_law",
            clip: "conservation_law",
            startup_s: 0.16,
            active_s: 0.10,
            // long enough to CONTAIN the two further terms below (they end at
            recover_s:
        // 0.66) — the builder's own `debug_assert` caught this at 0.34 and it is
        // the reason the assert is there: windows pushed after construction do
        // not extend the move.
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

    // SIDE — `symmetry_shift`. A lateral displacement that keeps her facing.
    //
    // the impulse is NEGATIVE, and that is the move. Body-local x runs
    // toward her facing, so a negative one carries her AWAY from what she is
    // looking at without turning her round — the blueprint's *"reposition without
    // conceding the neutral"*. Every other displacing special in this repo
    // commits you to the direction you are moving; this one buys distance and
    // keeps the threat pointed where it was.
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
    // The trail she leaves BEHIND her, which is the half of a retreat a watcher
    // needs to see.
    let side_b = vfx_cued(
        side_b,
        0.30,
        "paired_trajectory",
        (34.0, -2.0),
        SWING_FX,
        "vfx.noether.paired_trajectory.loop",
    );
    let side_b = on_contact(side_b, "player.hit");

    // UP — `ethereal_lift`. THE RECOVERY, AND IT DOES NOT ATTACK.
    //
    // the blueprint asked for exactly this — *"Rises, does not attack — the
    // traversal motif, not a second offensive option"* — and nothing else on the
    // grid is shaped like it. Every other authored recovery here carries a box,
    // so an edgeguard is a trade you might win by pressing anyway. Hers is a pure
    // traversal: she cannot trade with the person waiting for her, only beat
    // them. That is a real cost, deliberately paid, and it is why her side
    // special buys distance rather than damage.
    let mut up_b = MoveSpec {
        display_name: None,
        id: "ethereal_lift".to_string(),
        // The same structural fallback chain every `strike` authors, because a
        // recovery that cannot find its row must still RUN — the timeline is the
        // move, the drawing is not.
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
        // the SLOT owns the posture — `SmashRepertoire` sets it from
        // `up_special`; this field is only here because a struct literal has to
        // name every field.
        gates: Default::default(),
        start_impulse: None,
        smash_charge_mult: 1.0,
        charge_gesture: ambition_platformer2d::entity_catalog::ChargeGesture::default(),
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
        // Enough authority to choose where she lands and none to cancel — the
        // same helpless tail every recovery here pays.
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

    // DOWN — `invariant_field`. ⭐⭐ IT IS THE COUNTER HER BLUEPRINT ASKED FOR,
    // and the note that recorded why she could not have one expired this
    // morning: *"`MoveSpec` has no absorb or reflect, and inventing one for one
    // character would be the wrong shape."* Both halves are answered —
    // `smash.counter` carries an ARBITRARY response and an absorb flag, and four
    // fighters now author one, so it is shared rather than bespoke.
    //
    // ⭐⭐ AND THE RESPONSE IS THE THEOREM. Noether's is that a symmetry implies
    // a CONSERVED QUANTITY: put energy in and it is not destroyed. So the answer
    // to being struck is `smash.vitality` — she keeps it. ⇒ Four counters on this
    // roster and no two alike: George grabs, the Author arrives behind you, the
    // Shadow Oni puts you out, and she is simply better off for having been hit.
    //
    // ⛔ SHE ABSORBS RATHER THAN REFLECTS, which is the same idea and not a
    // second one. Returning the shot would be conservation of MOMENTUM — George's
    // riposte already does that — and this move is about the energy going
    // nowhere. One conservation law per move.
    //
    // ⚠ WHAT IT COSTS HER, stated rather than buried: the displaced field was a
    // low wide poke at `damage: 6`, the CHEAPEST special she owns, and she keeps
    // a second ground-claiming field in `conservation_law`. ⇒ So the kit loses a
    // duplicate and gains a defensive option, which is why this slot rather than
    // the neutral her blueprint named — that one is three even terms and four
    // cues, and displacing it would cost her the move that states her idea.
    //
    // ⚠ THE HEAL IS SMALL ON PURPOSE. A parry is already a full punish window;
    // three points is a reason to take the read, not a reason to turtle.
    let down_b = ambition_characters::smash_counter::counter_move(
        "invariant_field",
        // Her own clip, kept: the art is a field closing and that is still what
        // the move looks like.
        "invariant_field",
        0.14,
        // The old ACTIVE window, kept as the stance. 0.15s is a real read at
        // 60Hz — nine frames — where the 0.05s windows elsewhere on this roster
        // would be a guess.
        0.15,
        0.30,
        ambition_characters::smash_counter::CounterParams {
            // A heartbeat, not a duration: `parry_window_timer` decays and the
            // stance re-arms it every live frame.
            window_s: 0.05,
            response: ambition_characters::smash_vitality::VITALITY.to_string(),
            response_params: ambition_platformer2d::entity_catalog::ParamValue::from_typed(
                &ambition_characters::smash_vitality::VitalityParams {
                    change: 3,
                    // ⓘ IGNORED BY A RESTORE — the floor only bounds a PRICE, and
                    // this is a gain. Stated at the type's own default rather
                    // than left to look meaningful.
                    floor: 1,
                    // ⭐ HER OWN ROWS. `conserved_current` is the effect her
                    // neutral and her recovery both use for the theorem, which is
                    // exactly what this is: the quantity that did not go away.
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

    // effect on ground. Think of bowser down b. In the air he just does a
    // downward slam, but on the ground, it causes him to jump in an arc and then
    // slam. Specials can have different effects in different contexts that
    // should be ok, and makes for a richer smash game, although in most cases
    // they shouldn't be context dependent."*
    //
    // a special gated to ONE posture is not answered in the other — the
    // directional chain walks straight past it to the NEUTRAL special, so a
    // player pressing down-B in the air got the neutral-B. `special_air_down`
    // sits ahead of `special_down` in that chain and has the whole time; this is
    // the two-form move it exists for.
    // DOWN, IN THE AIR. The grounded form needs a floor; this one brings
    // the symmetry down with her.
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
    // this table's own rule: every burst is heard. The conserved current comes
    // down with her.
    let air_down_b = vfx_cued(
        air_down_b,
        0.10,
        "conserved_current",
        (0.0, 20.0),
        FIELD_FX,
        "vfx.noether.conserved_current.loop",
    );
    let air_down_b = on_contact(air_down_b, "player.hit");

    // EMMY'S CAPTURE KIT. The steepest growth after the automaton: weak early,
    // decisive late. A conservation joke that is also a real property — what her throw
    // takes out of you is returned with interest at high percent.
    // her sheet ships the whole grab family — `grab`, `grab_hold`, `grab_release` — so the capture kit draws the rows it was drawn for.
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
        taunt: ambition_characters::moveset_authoring::taunt("emmy_noether_taunt", 0.9),
        dash_attack: ambition_characters::moveset_authoring::dash_attack(
            "emmy_noether_dash_attack",
            ambition_characters::moveset_authoring::DashAttackShape::GENRE,
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
        // AUTHORED, at the rule that every fighter in the smash roster have a grab. The
        // transitional `None` is gone: capture was proven on George and the Pirate Admiral, and
        // the whole point of proving it was to stop being the only two.
        //
        // the VALUES are per character on purpose. A roster whose grabs are
        // twelve copies of one number set is one grab wearing twelve names.
        capture: SmashCaptureRepertoire {
            // the axis she snaps you onto, the transfer each pummel makes, and the bridge she throws you across — her kit guards that every effect comes off her
            // own sheet, and a shared `classic_burst` would violate it.
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

    // the invariant is checked WHERE IT IS AUTHORED. A move edited off the
    // curve stops being Emmy's before anything else notices, and this is the last
    // place that holds the whole table at once.
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
    use ambition_platformer2d::entity_catalog::MoveEventKind;

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

    // Fourteen fighters each carried a copy of it: every bound verb names a move
    // this table defines, and the table binds the whole vocabulary. Both are now
    // unwritable defects rather than tested ones. `SmashRepertoire` owns the verb
    // strings, so there is no string in this file to misspell; it is a struct
    // with no `Default` and no private fields, so a missing or renamed slot is a
    // COMPILE error here. What the fourteen copies stood for — that every press
    // is answered, in every posture it is asked in — is checked once, by
    // `ambition_characters::smash_repertoire`, and by the host ratchet
    // `smash_roster_movesets::report_the_smash_kit_every_selectable_fighter_has`.

    /// THE SYMMETRY, AS AN ASSERTION — and the poison is every other fighter.
    ///
    /// Her forward and back aerials must be the same move in everything but the
    /// direction they point. the second half is what makes this a claim about
    /// EMMY rather than about aerials in general: Oiler's pair and the goblin's
    /// pair differ, so a table that accidentally satisfied the first assertion
    /// would still be saying something true only of her.
    #[test]
    fn her_forward_and_back_aerials_are_the_same_move() {
        let set = emmy_noether_moveset();
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

        // the poison: nobody else on the grid is symmetric, so this is a
        // statement about her and not about the genre.
        let oiler = crate::oiler_moveset::oiler_moveset();
        let of = find(&oiler, "air_forward");
        let ob = find(&oiler, "air_back");
        assert!(
            damage(&of) != damage(&ob) || knockback(&of) != knockback(&ob),
            "Oiler's aerial pair differs — if it stopped differing this test \
             would stop being about Emmy"
        );
    }

    /// THE CONSERVED QUANTITY, AS AN ASSERTION.
    ///
    /// Every striking move sits on `damage x active_seconds =` [`NOETHER_IMPULSE`]
    /// within [`INVARIANT_BAND`]. and the band is not wide enough to be
    /// vacuous: the same measurement over Oiler's table — a fighter authored
    /// from the opposite idea — has to miss it, or this asserts nothing.
    #[test]
    fn every_strike_she_throws_conserves_the_same_quantity() {
        let set = emmy_noether_moveset();
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

        let oiler = crate::oiler_moveset::oiler_moveset();
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

    /// The launcher is the ONE move whose growth leaves the ordinary band —
    /// the blueprint's *"the moment the invariant stops holding"*.
    #[test]
    fn exactly_one_move_grows_like_a_kill_move() {
        let set = emmy_noether_moveset();
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
    /// The blueprint asked for exactly this. the poison is Oiler's geyser,
    /// which DOES carry a box — so this is a statement about a deliberate trade
    /// and not about recoveries generally.
    #[test]
    fn her_recovery_carries_no_hitbox_and_that_is_unusual() {
        let set = emmy_noether_moveset();
        let lift = find(&set, "ethereal_lift");
        assert!(
            lift.windows.iter().all(|w| w.volumes.is_empty()),
            "the ethereal lift is a traversal, not a second offensive option"
        );
        assert_eq!(conserved_impulse(&lift), 0.0);

        let oiler = crate::oiler_moveset::oiler_moveset();
        let geyser = find(&oiler, "oil_geyser");
        assert!(
            geyser.windows.iter().any(|w| !w.volumes.is_empty()),
            "Oiler's recovery hits — if it stopped, hers would no longer be the \
             one that gives that up"
        );
    }

    /// The lift is a save, not flight — held by arithmetic rather than by a
    /// cooldown, exactly as Oiler's geyser is.
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
        let set = emmy_noether_moveset();
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
        let set = emmy_noether_moveset();
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

    // A burst carries its own sound now: `dispatch_move_events` asks for a paired `FxRequest`
    // and presentation resolves the cue the effect's name addresses.
    //
    // what guards it instead, and it is a stronger claim than this test
    // made: `a_paired_burst_is_heard_exactly_once` (`src/moveset_sound.rs`)
    // drives these very tables through the real dispatcher and the real fan-out
    // and counts what reaches the SFX channel — so it catches the silence this
    // test caught AND the double-play this test could not, having been written
    // to require the second half of the pair.

    /// THE ART IS HERS, AND IT ALL SHIPS.
    ///
    /// the oracle is the ART — `is_authored_effect` reads the rows out of the
    /// baked manifests — so this asks exactly what the renderer will ask.
    #[test]
    fn the_kit_looks_like_emmy_and_the_art_all_ships() {
        let set = emmy_noether_moveset();
        let mut effects = std::collections::BTreeSet::new();
        for m in &set.moves {
            for problem in
                m.presentation_problems(ambition_platformer2d::sprite_sheet::fx::is_authored_effect)
            {
                panic!("{problem}");
            }
            for ev in &m.events {
                if let MoveEventKind::Vfx { effect, .. } = &ev.kind {
                    effects.insert(effect.clone());
                }
            }
        }
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
    /// the structural fallback chain means a missing clip is SILENT: the move
    /// still runs, drawn as `idle`. That is the right runtime behaviour and the
    /// wrong authoring outcome, so the table is checked against the sheet here.
    #[test]
    fn every_clip_names_a_row_her_sheet_carries() {
        let set = emmy_noether_moveset();
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

    /// ⛔⛔ THE THEOREM IS THE MOVE: a symmetry implies a CONSERVED QUANTITY, so
    /// the answer to being struck is that the energy is kept. A test that only
    /// found a counter would pass against one that answered with a grab — which
    /// would be George's move on her fighter, and would say nothing.
    #[test]
    fn her_field_answers_a_blow_by_conserving_it() {
        let set = emmy_noether_moveset();
        let field = set
            .moves
            .iter()
            .find(|m| m.id == "invariant_field")
            .expect("her grounded down special");

        let params: ambition_platformer2d::characters::smash_counter::CounterParams = field
            .windows
            .iter()
            .filter_map(|w| w.sustain_effect.as_ref())
            .find(|e| e.key == ambition_platformer2d::characters::smash_counter::COUNTER)
            .expect("the field holds a counter stance")
            .params
            .hydrate()
            .expect("counter params hydrate");

        assert_eq!(
            params.response,
            ambition_platformer2d::characters::smash_vitality::VITALITY,
            "she must answer by keeping the energy, not by grabbing or leaving"
        );
        let gain: ambition_platformer2d::characters::smash_vitality::VitalityParams =
            params.response_params.hydrate().expect("vitality params hydrate");
        assert!(
            gain.change > 0,
            "a conservation law that COSTS her health is the opposite of the move: {}",
            gain.change
        );
        // ⛔ SMALL. A parry is already a full punish window; this is a reason to
        // take the read, not a reason to turtle.
        assert!(gain.change <= 5, "the heal is worth turtling for: {}", gain.change);

        // ⭐ SHE ABSORBS RATHER THAN REFLECTS — the same conservation idea, not a
        // second one. Returning the shot is conservation of MOMENTUM and is
        // already George's riposte.
        assert!(
            params.absorbs_projectiles,
            "she returns shots, which is the other fighter's law"
        );

        // ⛔ AND THE STANCE IS A REAL READ. Her old ACTIVE window was 0.15s —
        // nine frames — and a counter inherits it; the 0.05s stances elsewhere on
        // the roster are for fighters who answer fast, not for a field.
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
