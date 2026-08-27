//! The Perfect Cellular Automaton's signature move, authored as data.
//!
//! the vocabulary is Conway's, because the character is. A cellular
//! automaton does not punch — it applies a rule and the neighbourhood changes.
//! Every move names a pattern, and its shape is the pattern's shape: a still
//! life is a small stable block, an oscillator flips in place, a glider leaves
//! and does not come back, and a generation wipe takes everything in the row.
//! Its FOURTEEN authored effect rows were already rendered and, apart from the
//! pulse's cue, not one of them was named by anything.
//!
//! boss-grade telegraphs, kept. The pulse's 0.40s tell is what makes it
//! fair; the new moves are quicker but every one of them is slower to start than
//! the equivalent on a fighter built for this — it is a boss standing in a
//! platform fighter, and the numbers say so.

use ambition_characters::moveset_authoring::Strike;
use ambition_characters::smash_capture::{
    author_pummel, author_standing_grab, author_throw, capture_beat, grab_shell,
    CaptureAttemptParams, CaptureCues, CapturePummelParams, CaptureThrowParams,
    SmashCaptureRepertoire,
};
use ambition_characters::smash_repertoire::{
    DownSpecial, NeutralSpecial, SmashRepertoire, UpSpecial,
};
use ambition_platformer2d::entity_catalog::{
    ClipBinding, HitVolume, ImpulseMode, MoveEvent, MoveEventKind, MoveSpec, MoveWindow,
    MovesetContract, VolumeShape, WindowTag,
};

use ambition_characters::moveset_authoring::{
    committed_tail, impulse, on_contact, sfx, strike, vfx_at,
};

/// How big a pattern's burst is drawn, as a multiple of the presentation
/// default. A still life is small and a generation wipe is not.
const CELL_FX: f32 = 0.8;
const PATTERN_FX: f32 = 1.15;

/// See the module doc. Sixteen presses; the pulse is the first of them and the
/// only one that is not new.
pub fn cellular_pulse_moveset() -> MovesetContract {
    let window = |start_s: f32, end_s: f32, tag: WindowTag, volumes: Vec<HitVolume>| MoveWindow {
        start_s,
        end_s,
        tag,
        volumes,
        motion_scale: 1.0,
        sustain_effect: None,
    };

    // VERBATIM, and hand-built rather than passed through `strike`. The
    // helper would give it a slash arc and a different window shape; this move's
    // numbers came off an archetype row and the migration that brought them here
    // promised not to retune them. Everything after it uses the helper.
    let cellular_pulse = MoveSpec {
        display_name: None,
        id: "cellular_pulse".to_string(),
        clip: ClipBinding {
            clip: "special".to_string(),
            fallbacks: vec!["idle".to_string()],
        },
        duration_s: 0.85,
        windows: vec![
            // The tell. Long enough to be READ, which is what makes the
            // punish fair and the move boss-grade rather than merely strong.
            window(0.0, 0.40, WindowTag::Startup, Vec::new()),
            window(
                0.40,
                0.54,
                WindowTag::Active,
                vec![HitVolume {
                    // An ordinary hit, not a gust.
                    shape: VolumeShape::Rect {
                        offset: (30.0, 0.0),
                        half_extents: (34.0, 28.0),
                    },
                    damage: 3,
                    knockback: 140.0,
                    // Flat, exactly as the row authored it — the stage's
                    // ruleset decides whether knockback grows with percent.
                    knockback_growth: None,
                    launch_dir: None,
                    on_hit: None,
                    vfx: None,
                    hit_sfx: None,
                    reaction: None,
                }],
            ),
            window(0.54, 0.85, WindowTag::Recovery, Vec::new()),
        ],
        events: vec![MoveEvent {
            at_s: 0.40,
            kind: MoveEventKind::Sfx {
                cue: "pca.cellular_pulse".to_string(),
            },
        }],
        // the SLOT owns the posture — `SmashRepertoire` sets it from
        // `neutral_special`, so this field is only here because a struct literal
        // has to name every field.
        gates: Default::default(),
        start_impulse: None,
        smash_charge_mult: 1.0,
        charge_gesture: ambition_platformer2d::entity_catalog::ChargeGesture::default(),
        smash_charge: None,
        repeat: None,
        landing_lag_s: None,
        autocancel_after_s: None,
        sprite_spin_hz: None,
        equips: None,
    };

    // ── grounded ─────────────────────────────────────────────────────────────

    // JAB — `still_life`. A block appears where its hand would be and does
    // not change. The fastest thing it owns, and the only one with no telegraph.
    let jab = strike(Strike {
        id: "still_life",
        clip: "jab",
        startup_s: 0.06,
        active_s: 0.06,
        recover_s: 0.15,
        offset: (26.0, 0.0),
        half_extents: (18.0, 16.0),
        damage: 3,
        knockback: 48.0,
        knockback_growth: 1.05,
        launch_dir: None,
        on_hit: None,
    });
    let jab = vfx_at(jab, 0.06, "still_life_lock", (26.0, 0.0), CELL_FX);
    let jab = on_contact(jab, "player.hit");

    // FORWARD TILT — `rule_front`. The rule advances one cell into you.
    // without this the commonest press in the genre fell to the jab.
    let f_tilt = strike(Strike {
        id: "rule_front",
        clip: "attack_side",
        startup_s: 0.09,
        active_s: 0.08,
        recover_s: 0.19,
        offset: (32.0, -2.0),
        half_extents: (22.0, 15.0),
        damage: 6,
        knockback: 74.0,
        knockback_growth: 1.30,
        launch_dir: Some((1.0, -0.28)),
        on_hit: None,
    });
    let f_tilt = vfx_at(f_tilt, 0.09, "rule_front", (32.0, -2.0), PATTERN_FX);
    let f_tilt = on_contact(f_tilt, "player.hit");

    // UP TILT — `cell_birth`. A neighbourhood above it reaches three live
    // neighbours and something is born there. Anti-air.
    let u_tilt = strike(Strike {
        id: "cell_birth",
        clip: "attack_up",
        startup_s: 0.09,
        active_s: 0.08,
        recover_s: 0.20,
        offset: (8.0, -28.0),
        half_extents: (18.0, 22.0),
        damage: 5,
        knockback: 76.0,
        knockback_growth: 1.35,
        launch_dir: Some((0.12, -1.0)),
        on_hit: None,
    });
    let u_tilt = vfx_at(u_tilt, 0.09, "cell_birth", (8.0, -28.0), CELL_FX);
    let u_tilt = on_contact(u_tilt, "player.hit");

    // DOWN TILT — `phase_boundary`. The edge between two rules, at ankle
    // height, where standing on the wrong side of it costs.
    let d_tilt = strike(Strike {
        id: "phase_boundary",
        clip: "attack_down",
        startup_s: 0.08,
        active_s: 0.07,
        recover_s: 0.18,
        offset: (26.0, 14.0),
        half_extents: (22.0, 10.0),
        damage: 4,
        knockback: 56.0,
        knockback_growth: 1.18,
        launch_dir: Some((0.9, -0.35)),
        on_hit: None,
    });
    let d_tilt = vfx_at(d_tilt, 0.08, "phase_boundary", (26.0, 14.0), CELL_FX);
    let d_tilt = on_contact(d_tilt, "player.hit");

    // ── smashes ──────────────────────────────────────────────────────────────
    //
    // it had NONE. Three presses that resolved to nothing at all, on the
    // fighter with the highest health pool on the grid — a boss that could not
    // kill. These are its heaviest reads and they telegraph like the pulse.

    // FORWARD — `generation_wipe`. One step of the rule applied to the whole
    // row in front of it. Everything in that row is in the next generation or it
    // is not.
    let f_smash = strike(Strike {
        id: "generation_wipe",
        clip: "smash_forward",
        startup_s: 0.32,
        active_s: 0.10,
        recover_s: 0.30,
        offset: (40.0, -2.0),
        half_extents: (32.0, 24.0),
        damage: 14,
        knockback: 128.0,
        knockback_growth: 2.25,
        launch_dir: Some((0.95, -0.42)),
        on_hit: None,
    });
    let f_smash = vfx_at(f_smash, 0.32, "generation_wipe", (40.0, -2.0), PATTERN_FX);
    let f_smash = sfx(f_smash, 0.32, "pca.cellular_pulse");
    let f_smash = on_contact(f_smash, "player.hit");

    // UP — `causal_cone_expand`. The light cone of one changed cell, opening
    // upward. Tall and narrow at the base, which is what makes it an anti-air
    // rather than a second forward smash.
    let u_smash = strike(Strike {
        id: "causal_cone_expand",
        clip: "smash_up",
        startup_s: 0.30,
        active_s: 0.11,
        recover_s: 0.30,
        offset: (4.0, -34.0),
        half_extents: (24.0, 34.0),
        damage: 13,
        knockback: 124.0,
        knockback_growth: 2.30,
        launch_dir: Some((0.10, -1.0)),
        on_hit: None,
    });
    let u_smash = vfx_at(
        u_smash,
        0.18,
        "causal_cone_expand",
        (4.0, -34.0),
        PATTERN_FX,
    );
    let u_smash = sfx(u_smash, 0.18, "pca.cellular_pulse");
    let u_smash = on_contact(u_smash, "player.hit");

    // DOWN — `garden_growth`. A garden of Eden has no predecessor: it can
    // only be placed. It places one, either side of itself, along the floor.
    let d_smash = strike(Strike {
        id: "garden_growth",
        clip: "smash_down",
        startup_s: 0.31,
        active_s: 0.10,
        recover_s: 0.32,
        offset: (0.0, 20.0),
        half_extents: (44.0, 13.0),
        damage: 12,
        knockback: 116.0,
        knockback_growth: 2.05,
        launch_dir: Some((0.8, -0.58)),
        on_hit: None,
    });
    let d_smash = vfx_at(d_smash, 0.31, "garden_growth", (-30.0, 18.0), CELL_FX);
    let d_smash = vfx_at(d_smash, 0.31, "garden_growth", (30.0, 18.0), CELL_FX);
    let d_smash = sfx(d_smash, 0.31, "pca.cellular_pulse");
    let d_smash = on_contact(d_smash, "player.hit");

    // ── aerials ──────────────────────────────────────────────────────────────

    // NEUTRAL AIR — `oscillator_pulse`. A blinker, around itself, flipping
    // through both of its states.
    let n_air = strike(Strike {
        id: "oscillator_pulse",
        clip: "air_neutral",
        startup_s: 0.07,
        active_s: 0.10,
        recover_s: 0.18,
        offset: (0.0, 0.0),
        half_extents: (28.0, 24.0),
        damage: 6,
        knockback: 70.0,
        knockback_growth: 1.40,
        launch_dir: Some((0.55, -0.75)),
        on_hit: None,
    });
    let n_air = vfx_at(n_air, 0.07, "oscillator_pulse", (0.0, 0.0), PATTERN_FX);
    let n_air = on_contact(n_air, "player.hit");

    let f_air = strike(Strike {
        id: "glider_cut",
        clip: "air_forward",
        startup_s: 0.09,
        active_s: 0.08,
        recover_s: 0.20,
        offset: (30.0, -4.0),
        half_extents: (22.0, 18.0),
        damage: 8,
        knockback: 92.0,
        knockback_growth: 1.70,
        launch_dir: Some((0.95, -0.45)),
        on_hit: None,
    });
    let f_air = vfx_at(f_air, 0.09, "glider_impact", (30.0, -4.0), CELL_FX);
    let f_air = on_contact(f_air, "player.hit");

    // BACK AIR — `cell_death`. Underpopulation, behind it. The hardest
    // single hit in its aerial game, because it cannot see it coming either.
    let b_air = strike(Strike {
        id: "cell_death",
        clip: "air_back",
        startup_s: 0.10,
        active_s: 0.07,
        recover_s: 0.22,
        offset: (-30.0, -2.0),
        half_extents: (22.0, 18.0),
        damage: 9,
        knockback: 100.0,
        knockback_growth: 1.85,
        launch_dir: Some((-0.95, -0.40)),
        on_hit: None,
    });
    let b_air = vfx_at(b_air, 0.10, "cell_death", (-30.0, -2.0), CELL_FX);
    let b_air = on_contact(b_air, "player.hit");

    // UP AIR — `fixed_point_acquire`. It finds the state that maps to
    // itself, directly overhead, and holds it there.
    let u_air = strike(Strike {
        id: "fixed_point_acquire",
        clip: "air_up",
        startup_s: 0.08,
        active_s: 0.09,
        recover_s: 0.19,
        offset: (2.0, -28.0),
        half_extents: (20.0, 24.0),
        damage: 7,
        knockback: 84.0,
        knockback_growth: 1.65,
        launch_dir: Some((0.08, -1.0)),
        on_hit: None,
    });
    let u_air = vfx_at(u_air, 0.08, "fixed_point_acquire", (2.0, -28.0), CELL_FX);
    let u_air = on_contact(u_air, "player.hit");

    // DOWN AIR — `corruption_seed`. It drops a seed and the rule below it
    // stops being the rule. Straight down and hard.
    let d_air = strike(Strike {
        id: "corruption_seed",
        clip: "air_down",
        startup_s: 0.12,
        active_s: 0.08,
        recover_s: 0.24,
        offset: (2.0, 26.0),
        half_extents: (20.0, 20.0),
        damage: 10,
        knockback: 112.0,
        knockback_growth: 2.05,
        launch_dir: Some((0.0, 1.0)),
        on_hit: None,
    });
    let d_air = vfx_at(d_air, 0.12, "corruption_seed", (2.0, 26.0), CELL_FX);
    let d_air = on_contact(d_air, "player.hit");

    // ── the three specials the pulse was standing in for ─────────────────────

    // SIDE — `glider_launch`. A glider leaves and does not come back. the
    // move DISPLACES the automaton rather than spawning a projectile: its ranged
    // glider already exists on its action set, and a second spawner here would
    // be two authorities on one pattern.
    let side_b = strike(Strike {
        id: "glider_launch",
        clip: "special",
        startup_s: 0.16,
        active_s: 0.10,
        recover_s: 0.28,
        offset: (30.0, 0.0),
        half_extents: (26.0, 20.0),
        damage: 11,
        knockback: 108.0,
        knockback_growth: 1.95,
        launch_dir: Some((0.92, -0.38)),
        on_hit: None,
    });
    let side_b = impulse(side_b, 0.16, (620.0, 0.0), ImpulseMode::Set);
    let side_b = committed_tail(side_b, 0.62, 0.05);
    let side_b = vfx_at(side_b, 0.16, "glider_launch", (30.0, 0.0), PATTERN_FX);
    let side_b = on_contact(side_b, "player.hit");

    // UP — `spaceship_ascent`. THE RECOVERY. A lightweight spaceship
    // translates itself one cell per generation, forever, in whatever direction
    // it was built pointing. This one points up.
    let mut up_b = strike(Strike {
        id: "spaceship_ascent",
        clip: "special",
        startup_s: 0.10,
        active_s: 0.12,
        recover_s: 0.24,
        offset: (0.0, -14.0),
        half_extents: (22.0, 32.0),
        damage: 8,
        knockback: 88.0,
        knockback_growth: 1.70,
        launch_dir: Some((0.12, -1.0)),
        on_hit: None,
    });
    up_b.landing_lag_s = Some(0.32);
    let up_b = impulse(up_b, 0.10, (0.0, -740.0), ImpulseMode::Set);
    let up_b = committed_tail(up_b, 0.56, 0.15);
    let up_b = vfx_at(up_b, 0.10, "causal_cone_expand", (0.0, 16.0), CELL_FX);
    let up_b = vfx_at(up_b, 0.22, "cell_birth", (0.0, -14.0), CELL_FX);
    let up_b = on_contact(up_b, "player.hit");

    // DOWN — `generation_collapse`. It runs the rule BACKWARDS: the cone
    // closes instead of opening, and everything inside it arrives at the same
    // cell. No displacement, the longest tail it has, and grounded-only.
    let down_b = strike(Strike {
        id: "generation_collapse",
        clip: "special",
        startup_s: 0.22,
        active_s: 0.12,
        recover_s: 0.34,
        offset: (0.0, 0.0),
        half_extents: (40.0, 30.0),
        damage: 12,
        knockback: 104.0,
        knockback_growth: 1.85,
        launch_dir: Some((0.7, -0.68)),
        on_hit: None,
    });
    let down_b = committed_tail(down_b, 0.74, 0.0);
    let down_b = vfx_at(down_b, 0.06, "corruption_seed", (0.0, 12.0), CELL_FX);
    let down_b = vfx_at(down_b, 0.22, "causal_cone_collapse", (0.0, 0.0), PATTERN_FX);
    let down_b = sfx(down_b, 0.22, "pca.cellular_pulse");
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
    // DOWN, IN THE AIR — `collapse_dive`. The cone closes downward instead
    // of around it: every cell under it arrives at the same one, and so does it.
    let mut air_down_b = strike(Strike {
        id: "collapse_dive",
        clip: "air_down",
        startup_s: 0.12,
        active_s: 0.10,
        recover_s: 0.26,
        offset: (0.0, 24.0),
        half_extents: (22.0, 22.0),
        damage: 10,
        knockback: 100.0,
        knockback_growth: 1.78,
        launch_dir: Some((0.0, 1.0)),
        on_hit: None,
    });
    air_down_b.landing_lag_s = Some(0.32);
    let air_down_b = impulse(air_down_b, 0.12, (0.0, 1200.0), ImpulseMode::Set);
    let air_down_b = vfx_at(
        air_down_b,
        0.12,
        "causal_cone_collapse",
        (0.0, 20.0),
        CELL_FX,
    );
    let air_down_b = on_contact(air_down_b, "player.hit");
    // CELLULAR'S CAPTURE KIT. TALLER than it is wide, which is the automaton's
    // silhouette. The fastest pummel on the roster for the least damage each, and a
    // near-vertical throw: it does not carry you away, it stacks you.
    // the grab draws `attack`, not `grab`: these sheets publish no `grab` row,
    // and each table's own `every_clip_names_a_row_..._sheet_carries` guard says
    // so. `ClipBinding`'s fallbacks would have covered it at runtime, but a move
    // that NAMES a row nobody publishes is a lie the guard is right to refuse.
    let grab = author_standing_grab(
        grab_shell("cellular_grab", "attack", 0.08, 0.07, 0.19),
        CaptureAttemptParams {
            offset: (12.0, 1.0),
            half_extents: (16.0, 20.0),
            hold_offset: (13.0, 3.0),
        },
    );
    let pummel = author_pummel(
        capture_beat("cellular_pummel", "attack", 0.11),
        0.05,
        CapturePummelParams { damage: 2 },
    );
    let forward_throw = author_throw(
        capture_beat("cellular_fthrow", "attack", 0.22),
        0.11,
        CaptureThrowParams {
            damage: 6,
            knockback: 96.0,
            knockback_growth: 2.6,
            launch_dir: (0.35, -1.0),
        },
    );

    let back_throw = author_throw(
        capture_beat("cellular_bthrow", "attack", 0.24),
        0.12,
        CaptureThrowParams {
            damage: 7,
            knockback: 103.68,
            knockback_growth: 2.73,
            launch_dir: (-1.0, -0.62),
        },
    );

    let up_throw = author_throw(
        capture_beat("cellular_uthrow", "attack", 0.23),
        0.11,
        CaptureThrowParams {
            damage: 6,
            knockback: 99.84,
            knockback_growth: 2.65,
            launch_dir: (0.0, -1.0),
        },
    );

    let down_throw = author_throw(
        capture_beat("cellular_dthrow", "attack", 0.25),
        0.12,
        CaptureThrowParams {
            damage: 4,
            knockback: 71.04,
            knockback_growth: 2.08,
            launch_dir: (0.14, -0.92),
        },
    );

    SmashRepertoire {
        taunt: ambition_characters::moveset_authoring::taunt("cellular_automaton_taunt", 0.9),
        dash_attack: ambition_characters::moveset_authoring::dash_attack(
            "cellular_automaton_dash_attack",
            ambition_characters::moveset_authoring::DashAttackShape::GENRE,
            8,
            92.5,
        ),
        jab,
        forward_tilt: f_tilt,
        up_tilt: u_tilt,
        down_tilt: d_tilt,
        forward_smash: f_smash,
        up_smash: u_smash,
        down_smash: d_smash,
        neutral_air: n_air,
        forward_air: f_air,
        back_air: b_air,
        up_air: u_air,
        down_air: d_air,
        neutral_special: NeutralSpecial::Authored(cellular_pulse),
        side_special: side_b,
        up_special: UpSpecial::Standard(up_b),
        // AUTHORED, at the rule that every fighter in the smash roster have a grab. The
        // transitional `None` is gone: capture was proven on George and the Pirate Admiral, and
        // the whole point of proving it was to stop being the only two.
        //
        // the VALUES are per character on purpose. A roster whose grabs are
        // twelve copies of one number set is one grab wearing twelve names.
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
    use super::*;

    // Fourteen fighters each carried a copy of it: every bound verb names a move
    // this table defines, and the table binds the whole vocabulary. Both are now
    // unwritable defects rather than tested ones. `SmashRepertoire` owns the verb
    // strings, so there is no string in this file to misspell; it is a struct
    // with no `Default` and no private fields, so a missing or renamed slot is a
    // COMPILE error here. What the fourteen copies stood for — that every press
    // is answered, in every posture it is asked in — is checked once, by
    // `ambition_characters::smash_repertoire`, and by the host ratchet
    // `smash_roster_movesets::report_the_smash_kit_every_selectable_fighter_has`.

    /// THE PULSE IS UNTOUCHED. Fifteen moves were written around it and the
    /// one that came off the archetype row still carries the row's numbers —
    /// a 0.40s tell, a 0.14s window, 3 damage, 140 flat knockback.
    #[test]
    fn the_signature_move_still_carries_the_rows_verbatim_numbers() {
        let moveset = cellular_pulse_moveset();
        let pulse = moveset
            .move_by_id("cellular_pulse")
            .expect("the signature move");
        assert_eq!(pulse.duration_s, 0.85);
        let active = pulse
            .windows
            .iter()
            .find(|w| matches!(w.tag, WindowTag::Active))
            .expect("the pulse has an active window");
        assert_eq!((active.start_s, active.end_s), (0.40, 0.54));
        let volume = active.volumes.first().expect("one forward volume");
        assert_eq!(volume.damage, 3);
        assert_eq!(volume.knockback, 140.0);
        assert_eq!(
            volume.knockback_growth, None,
            "the row authors no growth, so the STAGE decides it"
        );
    }

    /// A BOSS TELEGRAPHS. Its heaviest reads start slower than the goblin's
    /// whole jab — the identity claim the module doc makes, pinned against a
    /// fighter built for this stage rather than against a constant.
    #[test]
    fn its_smashes_telegraph_more_than_a_fighters_do() {
        let pca = cellular_pulse_moveset();
        let goblin = crate::goblin_moveset::goblin_moveset();
        let startup = |set: &MovesetContract, id: &str| {
            set.move_by_id(id)
                .unwrap_or_else(|| panic!("{id} exists"))
                .windows
                .iter()
                .find(|w| matches!(w.tag, WindowTag::Active))
                .expect("a strike has an active window")
                .start_s
        };
        assert!(
            startup(&pca, "generation_wipe") > startup(&goblin, "smash_forward"),
            "the automaton's kill move comes out faster than the goblin's, so it \
             is a fighter with a boss's health rather than a boss"
        );
    }
}
