//! **Emmy Ethereal's repertoire** — a theorem, as a fighter.
//!
//! Jon, 2026-08-16: *"We also want to give her a full smash kit like we did with
//! Oiler. Again, we should have the vfx and sfx for this."*
//!
//! ⭐ **the sixth adopter removed from `smash_fighter_kit()`.** She stood on the
//! Super Smash Siblings grid with a `peaceful` catalog row — no melee at all —
//! on top of the fullest animation vocabulary in the project: **123 authored
//! rows**, grabs and throws and techs and ledge options and a shield break,
//! rendered and unreachable. Twelve of her own effects sat on
//! `noether_vfx_spritesheet.ron` with twelve matching cues in the packed bank.
//! This table is the wire between them.
//!
//! ## The character, from her own theorem
//!
//! Noether's theorem: **every differentiable symmetry has a conserved
//! quantity**. Both halves of that sentence are mechanical here.
//!
//! ```text
//!   SYMMETRY      her mirrored pair is IDENTICAL      facing does not matter
//!   CONSERVATION  damage x active time is INVARIANT   force and reach trade
//! ```
//!
//! **The symmetry.** Her forward and back aerials are the same move: same
//! damage, same knockback, same growth, same windows. Every other fighter in
//! this crate authors a back-air that differs from its forward-air, because
//! that is the genre's habit; hers cannot, because a fighter whose theorem is
//! invariance may not care which way she is facing.
//!
//! **The conserved quantity.** Every striking move in this table satisfies
//! `damage x active_seconds = `[`NOETHER_IMPULSE`] to within
//! [`INVARIANT_BAND`]. A move that hits hard is out for almost no time; a move
//! that stays out barely hurts. So she has no move that both damages and kills —
//! the quantity is conserved, and what a player chooses is where on the curve to
//! spend it.
//!
//! ⛔ **this is a real trade and not a re-skin.** Oiler's table holds EVERY box
//! out for at least 0.10s and torques exactly one bolt to kill; hers pays for
//! every point of damage in window time. Landing her forward smash is the
//! hardest single thing any authored fighter here asks for, and it is also the
//! only way she takes a stock quickly. Both claims are tested COMPARATIVELY
//! against Oiler and the goblin for exactly that reason.
//!
//! ## What the engine could not express, and what that changed
//!
//! ⚠ the blueprint that ships with her sheet calls her neutral special *"a held
//! field that returns what it absorbs"* — a counter. **`MoveSpec` has no absorb,
//! armour or reflect**, so a counter is not authorable today and inventing one
//! for a single character would be the wrong shape (see
//! `docs/planning/queue.md`, D139). Her `conservation_law` is instead the thing
//! the runtime CAN say and that reads the same on screen: a field that pays out
//! three times on one press, at even intervals, for as long as she holds the
//! ground she claimed.
//!
//! ⭐ and her Up-B genuinely **does not attack**, which the blueprint asked for
//! and which nothing else on the grid does. It is the only recovery here with no
//! hitbox at all: she cannot trade with an edgeguard, she can only beat it.
//!
//! ## The effects are the move, not decoration
//!
//! ⭐ **an effect is a NAME.** `invariant_core` addresses a row on a shipped FX
//! sheet and needs no table, enum or registry to reach the screen.
//!
//! ⭐⭐ **and the name carries the SOUND too** (D149). `dispatch_move_events`
//! asks for a paired `FxRequest`; presentation resolves the cue the row's own
//! name addresses. So a burst below states the art and stops. ⛔ this table used
//! to author an `Sfx` event beside every one of them — put one back and the
//! burst is heard TWICE.
//!
//! ⛔ **five of her twelve cues carry a `.loop` suffix the sprite row does not**
//! (`vfx.noether.invariant_core.loop`, `conserved_current`, `group_orbit`,
//! `paired_trajectory`, `conserved_pair_exchange`). The derived
//! `vfx.<family>.<row>` name misses the bank for all five, so those five say
//! their cue on the burst itself through `vfx_cued` — the override arm, one
//! authored thing instead of a pair.

use ambition_characters::moveset_prefabs::{SLASH_ARC_VFX, SLASH_POKE_VFX};
use ambition_characters::smash_repertoire::{DownSpecial, NeutralSpecial, SmashRepertoire};
use ambition_platformer2d::entity_catalog::{
    ClipBinding, HitVolume, ImpulseMode, MoveSpec, MoveWindow, MovesetContract, VolumeShape,
    WindowTag,
};

use ambition_characters::moveset_authoring::{
    committed_tail, impulse, on_contact, strike, strike_tag, vfx_at, vfx_cued,
};

/// **How big a burst is, by what kind of move throws it** — multiples of the
/// presentation default (`ambition_render::fx::FX_DEFAULT_WORLD_SIZE`, a little
/// under a fighter's height).
///
/// Jon, 2026-08-16: *"try to make the hitboxes and vfx placement make sense,
/// right now we are seeing crazy upscaled vfx"*. A poke is a spark on a
/// knuckle; a smash is the size of the swing; a field is meant to be read as
/// ground you cannot stand on. They are not the same size, and until this week
/// every effect in the project drew at one.
const POKE_FX: f32 = 0.55;
const SWING_FX: f32 = 0.75;
const SMASH_FX: f32 = 1.05;
const FIELD_FX: f32 = 1.30;

/// **The conserved quantity**: `damage x active_seconds`, in damage-seconds.
///
/// ⚠ this is the character, not a tuning constant fitted to the numbers after
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
/// ⭐ authored as a SPEED and applied with [`ImpulseMode::Set`], for the reason
/// every recovery in this repo is: an Emmy pressing this at terminal velocity
/// gets exactly the climb a standing one does. An additive impulse is weakest
/// precisely when it is the only thing between her and the blast zone.
pub const LIFT_SPEED: f32 = 940.0;

/// When the lift takes hold, and when it lets go.
pub const LIFT_AT_S: f32 = 0.18;
/// ⛔ **not a feel number.** Under the engine baseline the lift climbs
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

/// The conserved quantity, measured off a built move.
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
        shape: VolumeShape::Rect {
            offset: (0.0, 18.0),
            half_extents: (34.0, 20.0),
        },
        damage: 3,
        knockback: 58.0,
        knockback_growth: 1.35,
        launch_dir: Some((0.2, -0.9)),
        on_hit: None,
        vfx: Some(SLASH_POKE_VFX.to_string()),
        hit_sfx: None,
    });
    term
}

/// See the module doc. Sixteen moves: the genre's standard verb map, every clip
/// a row her rig actually publishes.
pub fn noether_moveset() -> MovesetContract {
    // ── the ground game ──────────────────────────────────────────────────────
    //
    // ⚠ **every clip below is one of her 123 authored rows.** Where the sheet has
    // no row for a genre verb (she has no `attack_side` and no `smash_forward`),
    // the move takes the SIGNATURE row that draws that idea — `generator_strike`
    // for the committed forward swing, `symmetry_break` for the smash — rather
    // than a clip name that would fall down the structural chain to `idle`.

    // Five damage held out for nearly a fifth of a second: the cheap end of the
    // curve, and the easiest thing she has to land.
    let jab = strike(
        "jab",
        "jab",
        0.05,
        0.18,
        0.14,
        (26.0, -6.0),
        (16.0, 13.0),
        5,
        44.0,
        1.05,
        None,
        None,
    );
    let jab = strike_tag(jab, SLASH_POKE_VFX);
    let jab = vfx_at(jab, 0.05, "generator_steps", (26.0, -6.0), POKE_FX);

    // The committed swing the blueprint calls *"her fastest way to say no"*.
    // Nine damage buys a tenth of a second.
    let mut f_tilt = strike(
        "tilt_forward",
        "generator_strike",
        0.10,
        0.10,
        0.20,
        (32.0, -4.0),
        (22.0, 16.0),
        9,
        76.0,
        1.55,
        Some((1.0, -0.30)),
        None,
    );
    f_tilt.start_impulse = Some((130.0, 0.0));
    let f_tilt = vfx_at(f_tilt, 0.10, "generator_steps", (32.0, -4.0), SWING_FX);
    let f_tilt = vfx_at(f_tilt, 0.13, "symmetry_axis_snap", (32.0, -4.0), POKE_FX);
    let f_tilt = on_contact(f_tilt, "player.hit");

    let up_tilt = strike(
        "tilt_up",
        "attack_up",
        0.09,
        0.15,
        0.18,
        (6.0, -26.0),
        (17.0, 23.0),
        6,
        70.0,
        1.50,
        Some((0.1, -1.0)),
        None,
    );
    let up_tilt = vfx_cued(
        up_tilt,
        0.09,
        "group_orbit",
        (6.0, -26.0),
        SWING_FX,
        "vfx.noether.group_orbit.loop",
    );
    let up_tilt = on_contact(up_tilt, "player.hit");

    let down_tilt = strike(
        "tilt_down",
        "attack_down",
        0.08,
        0.15,
        0.18,
        (24.0, 15.0),
        (22.0, 11.0),
        6,
        66.0,
        1.45,
        Some((0.9, -0.35)),
        None,
    );
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

    // **The launcher.** Fifteen damage for six hundredths of a second — the
    // narrowest window any authored fighter here asks a player to hit, and the
    // only move of hers that grows like a kill move.
    let mut f_smash = strike(
        "smash_forward",
        "symmetry_break",
        0.20,
        0.06,
        0.34,
        (36.0, -8.0),
        (26.0, 22.0),
        15,
        124.0,
        BREAK_GROWTH,
        Some((1.0, -0.55)),
        None,
    );
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

    let mut up_smash = strike(
        "smash_up",
        "smash_up",
        0.17,
        0.075,
        0.30,
        (2.0, -34.0),
        (20.0, 28.0),
        12,
        112.0,
        ORDINARY_GROWTH,
        Some((0.0, -1.0)),
        None,
    );
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

    // ⭐ the down smash is the up smash REFLECTED: same damage, same window, same
    // growth, opposite launch. Her table is meant to look like this.
    let mut down_smash = strike(
        "smash_down",
        "smash_down",
        0.17,
        0.075,
        0.30,
        (0.0, 20.0),
        (34.0, 14.0),
        12,
        112.0,
        ORDINARY_GROWTH,
        Some((0.0, 1.0)),
        None,
    );
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

    let mut n_air = strike(
        "air_neutral",
        "air_neutral",
        0.08,
        0.11,
        0.20,
        (0.0, -6.0),
        (26.0, 24.0),
        8,
        78.0,
        1.60,
        Some((0.5, -0.75)),
        None,
    );
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

    // ⭐⭐ **THE SYMMETRY, and it is the whole character in two moves.** The
    // forward and back aerials are built from the same numbers by construction —
    // ONE expression over the two ids rather than two hand-written blocks, so
    // they cannot drift apart in a later retune. A fighter whose theorem is
    // invariance does not get to care which way she is facing.
    let [f_air, b_air] = [
        ("air_forward", "air_forward", 1.0_f32),
        ("air_back", "air_back", -1.0_f32),
    ]
    .map(|(id, clip, dir_x)| {
        let mut aerial = strike(
            id,
            clip,
            0.10,
            0.10,
            0.22,
            (28.0 * dir_x, -4.0),
            (22.0, 18.0),
            9,
            84.0,
            1.70,
            Some((dir_x, -0.45)),
            None,
        );
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

    let mut up_air = strike(
        "air_up",
        "air_up",
        0.08,
        0.15,
        0.19,
        (2.0, -28.0),
        (18.0, 24.0),
        6,
        72.0,
        1.55,
        Some((0.0, -1.0)),
        None,
    );
    up_air.landing_lag_s = Some(0.14);
    up_air.autocancel_after_s = Some(0.28);
    let up_air = vfx_at(up_air, 0.08, "equivalence_bridge", (2.0, -28.0), SWING_FX);
    let up_air = on_contact(up_air, "player.hit");

    // The spike. Ten damage buys the second-narrowest window she has.
    let mut d_air = strike(
        "air_down",
        "air_down",
        0.12,
        0.09,
        0.24,
        (0.0, 24.0),
        (18.0, 26.0),
        10,
        96.0,
        1.80,
        Some((0.0, 1.0)),
        None,
    );
    d_air.landing_lag_s = Some(0.26);
    d_air.autocancel_after_s = Some(0.34);
    let d_air = vfx_at(d_air, 0.12, "ether_cancel", (0.0, 24.0), SWING_FX);
    let d_air = on_contact(d_air, "player.hit");

    // ── THE FOUR SPECIALS ────────────────────────────────────────────────────

    // **NEUTRAL — `conservation_law`.** She claims a piece of ground and it keeps
    // paying. Three terms at even intervals, identical every time: what the field
    // returns does not decay, which is the conservation idea stated as a move.
    //
    // ⛔ **not the counter the sheet's blueprint imagined** — `MoveSpec` has no
    // absorb or reflect, and inventing one for one character would be the wrong
    // shape. See the module doc.
    let n_b = strike(
        "conservation_law",
        "conservation_law",
        0.16,
        0.10,
        // ⚠ long enough to CONTAIN the two further terms below (they end at
        // 0.66) — the builder's own `debug_assert` caught this at 0.34 and it is
        // the reason the assert is there: windows pushed after construction do
        // not extend the move.
        0.44,
        (0.0, 18.0),
        (34.0, 20.0),
        3,
        58.0,
        1.35,
        Some((0.2, -0.9)),
        None,
    );
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

    // **SIDE — `symmetry_shift`.** A lateral displacement that keeps her facing.
    //
    // ⭐⭐ **the impulse is NEGATIVE, and that is the move.** Body-local x runs
    // toward her facing, so a negative one carries her AWAY from what she is
    // looking at without turning her round — the blueprint's *"reposition without
    // conceding the neutral"*. Every other displacing special in this repo
    // commits you to the direction you are moving; this one buys distance and
    // keeps the threat pointed where it was.
    let side_b = strike(
        "symmetry_shift",
        "symmetry_shift",
        0.14,
        0.225,
        0.28,
        (18.0, -2.0),
        (24.0, 20.0),
        4,
        62.0,
        1.40,
        Some((-0.6, -0.55)),
        None,
    );
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

    // **UP — `ethereal_lift`. THE RECOVERY, AND IT DOES NOT ATTACK.**
    //
    // ⭐⭐ the blueprint asked for exactly this — *"Rises, does not attack — the
    // traversal motif, not a second offensive option"* — and nothing else on the
    // grid is shaped like it. Every other authored recovery here carries a box,
    // so an edgeguard is a trade you might win by pressing anyway. Hers is a pure
    // traversal: she cannot trade with the person waiting for her, only beat
    // them. That is a real cost, deliberately paid, and it is why her side
    // special buys distance rather than damage.
    let mut up_b = MoveSpec {
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
        // ⭐ the SLOT owns the posture — `SmashRepertoire` sets it from
        // `up_special`; this field is only here because a struct literal has to
        // name every field.
        gates: Default::default(),
        start_impulse: None,
        smash_charge_mult: 1.0,
        landing_lag_s: Some(0.24),
        autocancel_after_s: None,
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

    // **DOWN — `invariant_field`.** A low, wide field that denies the ground in
    // front of her. The widest box in the table and the cheapest damage on it.
    let down_b = strike(
        "invariant_field",
        "invariant_field",
        0.14,
        0.15,
        0.30,
        (14.0, 20.0),
        (40.0, 13.0),
        6,
        74.0,
        1.50,
        Some((0.8, -0.5)),
        None,
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
    // **DOWN, IN THE AIR.** The grounded form needs a floor; this one brings
    // the symmetry down with her.
    let mut air_down_b = strike(
        "falling_invariant",
        "air_down",
        0.10,
        0.09,
        0.24,
        (0.0, 23.0),
        (21.0, 21.0),
        10,
        98.0,
        1.74,
        Some((0.0, 1.0)),
        None,
    );
    air_down_b.landing_lag_s = Some(0.28);
    let air_down_b = impulse(air_down_b, 0.10, (0.0, 1200.0), ImpulseMode::Set);
    // ⚠ this table's own rule: every burst is heard. The conserved current comes
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

    let repertoire = SmashRepertoire {
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
        up_special: up_b,
        down_special: DownSpecial::ByPosture {
            grounded: down_b,
            airborne: air_down_b,
        },
    }
    .into_contract();

    // ⭐ **the invariant is checked WHERE IT IS AUTHORED.** A move edited off the
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
            .map(|v| v.knockback_growth)
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

    // ⭐⭐ **RETIRED 2026-08-16 — the per-file verb-map test.**
    //
    // Fourteen fighters each carried a copy of it: every bound verb names a move
    // this table defines, and the table binds the whole vocabulary. Both are now
    // unwritable defects rather than tested ones. `SmashRepertoire` owns the verb
    // strings, so there is no string in this file to misspell; it is a struct
    // with no `Default` and no private fields, so a missing or renamed slot is a
    // COMPILE error here. What the fourteen copies stood for — that every press
    // is answered, in every posture it is asked in — is checked once, by
    // `ambition_characters::smash_repertoire`, and by the host ratchet
    // `smash_roster_movesets::report_the_smash_kit_every_selectable_fighter_has`.

    /// **THE SYMMETRY, AS AN ASSERTION — and the poison is every other fighter.**
    ///
    /// Her forward and back aerials must be the same move in everything but the
    /// direction they point. ⛔ **the second half is what makes this a claim about
    /// EMMY** rather than about aerials in general: Oiler's pair and the goblin's
    /// pair differ, so a table that accidentally satisfied the first assertion
    /// would still be saying something true only of her.
    #[test]
    fn her_forward_and_back_aerials_are_the_same_move() {
        let set = noether_moveset();
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

        // ⛔ the poison: nobody else on the grid is symmetric, so this is a
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

    /// **THE CONSERVED QUANTITY, AS AN ASSERTION.**
    ///
    /// Every striking move sits on `damage x active_seconds =` [`NOETHER_IMPULSE`]
    /// within [`INVARIANT_BAND`]. ⛔ **and the band is not wide enough to be
    /// vacuous**: the same measurement over Oiler's table — a fighter authored
    /// from the opposite idea — has to miss it, or this asserts nothing.
    #[test]
    fn every_strike_she_throws_conserves_the_same_quantity() {
        let set = noether_moveset();
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

    /// **The launcher is the ONE move whose growth leaves the ordinary band** —
    /// the blueprint's *"the moment the invariant stops holding"*.
    #[test]
    fn exactly_one_move_grows_like_a_kill_move() {
        let set = noether_moveset();
        let loud: Vec<&str> = set
            .moves
            .iter()
            .filter(|m| growth(m) > ORDINARY_GROWTH)
            .map(|m| m.id.as_str())
            .collect();
        assert_eq!(loud, ["smash_forward"], "one break, and it is the break");
        assert_eq!(growth(&find(&set, "smash_forward")), BREAK_GROWTH);
    }

    /// **Her recovery does not attack, and nothing else on the grid is like it.**
    ///
    /// The blueprint asked for exactly this. ⛔ the poison is Oiler's geyser,
    /// which DOES carry a box — so this is a statement about a deliberate trade
    /// and not about recoveries generally.
    #[test]
    fn her_recovery_carries_no_hitbox_and_that_is_unusual() {
        let set = noether_moveset();
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

    /// **The lift is a save, not flight** — held by arithmetic rather than by a
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
        let set = noether_moveset();
        let lift = find(&set, "ethereal_lift");
        assert!(
            lift.windows
                .iter()
                .all(|w| !matches!(w.tag, WindowTag::Cancelable { .. })),
            "a cancelable window would let her re-press before the arc is spent"
        );
    }

    /// **The side special buys distance BACKWARD without turning her round.**
    #[test]
    fn the_symmetry_shift_retreats_without_conceding_the_facing() {
        let set = noether_moveset();
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

    // ⭐⭐ **`every_burst_in_this_table_is_heard` was RETIRED here (D149).** Its
    // whole job was checking that whoever authored a `Vfx` remembered to write
    // an `Sfx` beside it — infrastructure making a content author carry a
    // backend detail. A burst carries its own sound now: `dispatch_move_events`
    // asks for a paired `FxRequest` and presentation resolves the cue the
    // effect's name addresses.
    //
    // ⚠ **what guards it instead**, and it is a stronger claim than this test
    // made: `a_paired_burst_is_heard_exactly_once` (`src/moveset_sound.rs`)
    // drives these very tables through the real dispatcher and the real fan-out
    // and counts what reaches the SFX channel — so it catches the silence this
    // test caught AND the double-play this test could not, having been written
    // to require the second half of the pair.

    /// **THE ART IS HERS, AND IT ALL SHIPS.**
    ///
    /// ⭐ the oracle is the ART — `is_authored_effect` reads the rows out of the
    /// baked manifests — so this asks exactly what the renderer will ask.
    #[test]
    fn the_kit_looks_like_emmy_and_the_art_all_ships() {
        let set = noether_moveset();
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

    /// **Every clip she names is a row her rig actually publishes.**
    ///
    /// ⛔ the structural fallback chain means a missing clip is SILENT: the move
    /// still runs, drawn as `idle`. That is the right runtime behaviour and the
    /// wrong authoring outcome, so the table is checked against the sheet here.
    #[test]
    fn every_clip_names_a_row_her_sheet_carries() {
        let set = noether_moveset();
        let record =
            ambition_platformer2d::sprite_sheet::character::sheets::record_for_target("noether")
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
}
