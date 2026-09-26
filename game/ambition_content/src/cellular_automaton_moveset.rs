//! The Perfect Cellular Automaton's signature move, authored as data.
//!
//! The vocabulary is Conway's, because the character is. A cellular
//! automaton does not punch: it applies a rule and the neighbourhood changes.
//! Each move names a pattern and has the pattern's shape: a still life is a
//! small stable block, an oscillator flips in place, a glider leaves and does
//! not come back, and a generation wipe takes everything in the row.
//!
//! Boss-grade telegraphs. The pulse's 0.40s tell makes it fair, and every
//! other move starts slower than the same move on a fighter built for this
//! mode: it is a boss standing in a platform fighter.

use ambition_entity_catalog::authoring::Strike;
use ambition_entity_catalog::smash_capture::{
    author_pummel, author_standing_grab, author_throw, capture_beat, grab_shell,
    CaptureAttemptParams, CaptureCues, CapturePummelParams, CaptureThrowParams,
    SmashCaptureRepertoire,
};
use ambition_entity_catalog::smash_repertoire::{
    DownSpecial, NeutralSpecial, SmashRepertoire, UpSpecial,
};
use ambition_entity_catalog::{
    AutolinkVolume, ClipBinding, HitVolume, ImpulseMode, MoveEvent, MoveEventKind, MoveSpec,
    MoveWindow, MovesetContract, VolumeShape, WindowTag,
};

use ambition_entity_catalog::authoring::{
    committed_tail, impulse, multihit, on_contact, sfx, strike, vfx_at, Pulse,
};

/// How big a pattern's burst is drawn, as a multiple of the presentation
/// default. A still life is small and a generation wipe is not.
const CELL_FX: f32 = 0.8;
const PATTERN_FX: f32 = 1.15;

/// See the module doc. Sixteen presses; the pulse is the first and the only
/// one carried over from the archetype row.
pub fn cellular_pulse_moveset() -> MovesetContract {
    let window = |start_s: f32, end_s: f32, tag: WindowTag, volumes: Vec<HitVolume>| MoveWindow {
        start_s,
        end_s,
        tag,
        volumes,
        motion_scale: 1.0,
        sustain_effect: None,
    };

    // Verbatim and hand-built, not passed through `strike`: the helper would add
    // a slash arc and a different window shape, and these numbers came from an
    // archetype row that must not be retuned. Every later move uses the helper.
    let cellular_pulse = MoveSpec {
        display_name: None,
        id: "cellular_pulse".to_string(),
        clip: ClipBinding {
            clip: "special".to_string(),
            fallbacks: vec!["idle".to_string()],
        },
        duration_s: 0.85,
        windows: vec![
            // The tell. Long enough to be read, which makes the punish fair and the
            // move boss-grade.
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
                    // Flat, as the row authored it: the stage's ruleset decides whether
                    // knockback grows with percent.
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
        // The slot owns the posture (`SmashRepertoire` sets it from
        // `neutral_special`); a struct literal must name every field.
        gates: Default::default(),
        start_impulse: None,
        smash_charge_mult: 1.0,
        charge_gesture: ambition_entity_catalog::ChargeGesture::default(),
        smash_charge: None,
        repeat: None,
        landing_lag_s: None,
        autocancel_after_s: None,
        sprite_spin_hz: None,
        equips: None,
        flow: None,
    };

    // ── grounded ─────────────────────────────────────────────────────────────

    // Jab: `still_life`. A block appears where its hand would be and does not
    // change. Its fastest move, and the only one with no telegraph.
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

    // Forward tilt: `rule_front`. The rule advances one cell into you.
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

    // Up tilt: `cell_birth`. A neighbourhood above it reaches three live
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

    // Down tilt: `phase_boundary`. The edge between two rules, at ankle
    // height.
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
    // Its heaviest reads, telegraphed like the pulse.

    // Forward: `generation_wipe`. One step of the rule applied to the whole row
    // in front of it.
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
        knockback_growth: 3.17,
        launch_dir: Some((0.95, -0.42)),
        on_hit: None,
    });
    let f_smash = vfx_at(f_smash, 0.32, "generation_wipe", (40.0, -2.0), PATTERN_FX);
    let f_smash = sfx(f_smash, 0.32, "pca.cellular_pulse");
    let f_smash = on_contact(f_smash, "player.hit");

    // Up: `causal_cone_expand`. The light cone of one changed cell, opening
    // upward. Tall and narrow at the base, so it is an anti-air, not a second
    // forward smash.
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
        knockback_growth: 5.91,
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

    // Down: `garden_growth`. A garden of Eden has no predecessor: it can only
    // be placed. It places one on each side of itself, along the floor.
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
        knockback_growth: 2.75,
        launch_dir: Some((0.8, -0.58)),
        on_hit: None,
    });
    let d_smash = vfx_at(d_smash, 0.31, "garden_growth", (-30.0, 18.0), CELL_FX);
    let d_smash = vfx_at(d_smash, 0.31, "garden_growth", (30.0, 18.0), CELL_FX);
    let d_smash = sfx(d_smash, 0.31, "pca.cellular_pulse");
    let d_smash = on_contact(d_smash, "player.hit");

    // ── aerials ──────────────────────────────────────────────────────────────

    // Neutral air: `oscillator_pulse`. A blinker around itself, flipping
    // through both states.
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

    // Back air: `cell_death`. Underpopulation, behind it. The hardest single
    // hit in its aerial game.
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

    // Up air: `fixed_point_acquire`. It finds the state that maps to itself,
    // directly overhead, and holds it.
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

    // Down air: `corruption_seed`. It drops a seed and the rule below it stops
    // being the rule. Straight down and hard.
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

    // Side: `glider_launch`. A glider leaves and does not come back. The move
    // displaces the automaton instead of spawning a projectile: its ranged glider
    // is already on its action set, and a second spawner would be two authorities
    // on one pattern.
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

    // Up: `spaceship_ascent`, the recovery. A lightweight spaceship moves one
    // cell per generation in the direction it was built pointing. This one points
    // up.
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

    // Down: `generation_collapse`. It runs the rule backwards: the cone closes,
    // and everything inside it arrives at the same cell. No displacement, its
    // longest tail, grounded-only.
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
    // The collapse converges. `VolumeReaction::Autolink` holds the victim near
    // the attacker instead of launching it away, and `multihit` puts holding
    // pulses in front of a finisher.
    //
    // The anchor's x is zero, and that is required: `autolink_anchor_world`
    // mirrors the anchor with the attacker's facing, so a non-zero x would make
    // the gather point depend on facing.
    //
    // The finisher still launches: the generations collapse to one cell, then
    // that cell resolves. The pulses are chip (2 damage each), so the ending pays
    // for the move.
    let down_b = multihit(
        down_b,
        3,
        Pulse {
            // Centred and a little wider than the finisher: the closing cone is bigger
            // than the cell it closes on.
            offset: (0.0, 0.0),
            half_extents: (46.0, 34.0),
            damage: 2,
            // Separated windows. The re-hit rule refuses a contiguous track, so
            // touching windows would land only once.
            active_s: 0.035,
            gap_s: 0.030,
            autolink: AutolinkVolume {
                anchor: (0.0, -6.0),
                // It does not move, so the gather comes entirely from the correction.
                carry: 0.0,
                pull: 20.0,
                max_speed: 900.0,
            },
        },
    );
    let down_b = committed_tail(down_b, 0.74, 0.0);
    let down_b = vfx_at(down_b, 0.06, "corruption_seed", (0.0, 12.0), CELL_FX);
    let down_b = vfx_at(down_b, 0.22, "causal_cone_collapse", (0.0, 0.0), PATTERN_FX);
    let down_b = sfx(down_b, 0.22, "pca.cellular_pulse");
    let down_b = on_contact(down_b, "player.hit");

    // Down-B has two forms, like Bowser's: a slam in the air, an arc and slam
    // on the ground. Context-dependent specials are acceptable, though most
    // should not be.
    //
    // A special gated to one posture is not answered in the other: the
    // directional chain falls through to the neutral special.
    // `special_air_down` comes before `special_down` in that chain.
    // Down, in the air: `collapse_dive`. The cone closes downward: every cell
    // under it arrives at the same one, and so does it.
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
    // Cellular's capture kit. Taller than it is wide, like the automaton. The
    // fastest pummel on the roster for the least damage each, and a near-vertical
    // throw: it stacks you instead of carrying you away.
    // The grab draws `attack`, not `grab`: these sheets publish no `grab` row,
    // and each table's clip guard refuses unpublished rows.
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
        taunt: ambition_entity_catalog::authoring::taunt("cellular_automaton_taunt", 0.9),
        dash_attack: ambition_entity_catalog::authoring::dash_attack(
            "cellular_automaton_dash_attack",
            ambition_entity_catalog::authoring::DashAttackShape::GENRE,
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
    use super::*;

    // Verb binding is checked by construction: `SmashRepertoire` owns the verb
    // strings and is a struct with no `Default`, so a missing slot is a compile
    // error. Coverage in every posture is checked by
    // `ambition_entity_catalog::smash_repertoire` and by
    // `smash_roster_movesets::report_the_smash_kit_every_selectable_fighter_has`.

    /// The pulse keeps the archetype row's numbers: a 0.40s tell, a 0.14s window,
    /// 3 damage, 140 flat knockback.
    #[test]
    fn the_signature_move_still_carries_the_rows_verbatim_numbers() {
        let moveset = crate::authored_movesets::shipped("perfect_cellular_automaton");
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

    /// A boss telegraphs: its heaviest reads start slower than the goblin's whole
    /// jab. Pinned against a fighter built for this stage, not a constant.
    #[test]
    fn its_smashes_telegraph_more_than_a_fighters_do() {
        let pca = crate::authored_movesets::shipped("perfect_cellular_automaton");
        let goblin = crate::authored_movesets::shipped("goblin");
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

    /// The collapse must converge. Its only mechanic was once a strike with
    /// `launch_dir: (0.7, -0.68)`, which throws victims away; this test keeps the
    /// move gathering.
    #[test]
    fn the_generation_collapse_gathers_before_it_launches() {
        let set = crate::authored_movesets::shipped("perfect_cellular_automaton");
        let collapse = set
            .moves
            .iter()
            .find(|m| m.id == "generation_collapse")
            .expect("it has a grounded down special");

        let holds: Vec<ambition_entity_catalog::AutolinkVolume> = collapse
            .windows
            .iter()
            .flat_map(|w| w.volumes.iter())
            .filter_map(|v| match v.reaction {
                Some(ambition_entity_catalog::VolumeReaction::Autolink(a)) => {
                    Some(a)
                }
                _ => None,
            })
            .collect();
        assert!(
            holds.len() >= 2,
            "a cone that closes has more than one generation: {} holding pulse(s)",
            holds.len()
        );

        // The anchor's x is zero: `autolink_anchor_world` mirrors it with the
        // attacker's facing, so a non-zero x would make the gather point depend on
        // facing.
        for hold in &holds {
            assert_eq!(
                hold.anchor.0, 0.0,
                "the gather point moves with facing, so the collapse is a poke"
            );
            assert!(hold.pull > 0.0, "a hold that does not pull holds nothing");
        }

        // The finisher still launches. A move that only gathered would never let
        // go.
        assert!(
            collapse
                .windows
                .iter()
                .flat_map(|w| w.volumes.iter())
                .any(|v| v.reaction.is_none() && v.damage > 0),
            "nothing in the collapse launches, so it gathers forever"
        );
    }
}
