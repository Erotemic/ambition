//! **The primitives a character's move table is written with** — shared, because
//! the second character to author one must not begin by copying the first.
//!
//! ⭐ these came out of `player_robot_moveset.rs` when the goblin needed a real
//! repertoire (Jon's second redirect, P6: `smash_fighter_kit()`'s goal is
//! DELETION, and every character that gains a table removes an adopter). They
//! were never robot-specific — `strike` is *startup, one active window carrying
//! one volume, recovery*, which is the shape of nearly every move in the genre.
//!
//! ⚠ **a move states what it IS, never what a mode does with it.** Startup,
//! active frames, recovery, hitbox geometry, damage, base launch and growth are
//! properties of the swing; percent, stocks, blast zones and DI are the
//! RULESET's. That is what lets one table read as Hollow-Knight combat in one
//! game and a platform fighter in another.

use ambition_platformer2d::entity_catalog::{
    ClipBinding, EffectRef, HitVolume, MoveGates, MoveSpec, MoveWindow, VolumeShape, WindowTag,
};

/// Ground moves are grounded-only so an airborne body falls THROUGH them to its
/// aerials rather than throwing a tilt in mid-air.
pub fn grounded_only() -> MoveGates {
    MoveGates {
        grounded: Some(true),
    }
}

/// Aerials are airborne-only for the mirror reason: a grounded press must not
/// reach a move whose whole design is that landing costs you.
pub fn airborne_only() -> MoveGates {
    MoveGates {
        grounded: Some(false),
    }
}

/// One strike on one timeline: startup, one active window carrying one volume,
/// recovery.
///
/// Every move here is that shape, so the authored differences are the ones that
/// MATTER — how long you are committed, how far it reaches, how hard it throws,
/// and how much of the throw scales with the victim's damage.
#[allow(clippy::too_many_arguments)]
pub fn strike(
    id: &str,
    clip: &str,
    startup_s: f32,
    active_s: f32,
    recover_s: f32,
    offset: (f32, f32),
    half_extents: (f32, f32),
    damage: i32,
    knockback: f32,
    knockback_growth: f32,
    launch_dir: Option<(f32, f32)>,
    // **What LANDING this hit can do beyond damage.** Today exactly one move
    // uses it: the down-air says it is capable of rebounding its attacker, and
    // the RULESET (`DeclaredCombatRules::downward_hit`) decides whether this
    // game takes it up on that or reads the swing as a spike instead.
    on_hit: Option<EffectRef>,
) -> MoveSpec {
    let active_start = startup_s;
    let active_end = startup_s + active_s;
    MoveSpec {
        id: id.to_string(),
        clip: ClipBinding {
            clip: clip.to_string(),
            // ⭐⭐ **THE AUTHORED FALLBACK CHAIN** (sprite redirect P0/P1,
            // 2026-08-11). A move names the exact row it wants — `smash_forward`,
            // `air_back` — and this is what it settles for when a sheet does not
            // have it. Robot v3's new sheet has 132 rows and draws the exact
            // clip; a lean sheet with `attack` draws that; one with only `slash`
            // and `idle` still plays.
            //
            // ⛔ **the structural fallbacks are DIRECTIONAL first** — an up-tilt
            // that cannot find `attack_up` should look like a side swing before
            // it looks like nothing, and `attack_side` is the row every fighter
            // sheet in the repo has had for a year.
            //
            // ⚠ a missing clip must never cost the move its GAMEPLAY: the
            // timeline runs whatever draws.
            fallbacks: vec![
                "attack_side".to_string(),
                "attack".to_string(),
                "slash".to_string(),
                "idle".to_string(),
            ],
        },
        duration_s: active_end + recover_s,
        windows: vec![
            MoveWindow {
                start_s: 0.0,
                end_s: active_start,
                tag: WindowTag::Startup,
                volumes: Vec::new(),
                motion_scale: 1.0,
                sustain_effect: None,
            },
            MoveWindow {
                start_s: active_start,
                end_s: active_end,
                tag: WindowTag::Active,
                volumes: vec![HitVolume {
                    shape: VolumeShape::Rect {
                        offset,
                        half_extents,
                    },
                    damage,
                    knockback,
                    knockback_growth,
                    launch_dir,
                    on_hit,
                    // The blade tag: the move runtime draws the slash from the
                    // SAME spawned volume, so the hitbox and the arc can never
                    // point different ways.
                    vfx: Some("slash_arc".to_string()),
                    hit_sfx: None,
                }],
                motion_scale: 1.0,
                sustain_effect: None,
            },
            MoveWindow {
                start_s: active_end,
                end_s: active_end + recover_s,
                tag: WindowTag::Recovery,
                volumes: Vec::new(),
                motion_scale: 1.0,
                sustain_effect: None,
            },
        ],
        events: Vec::new(),
        gates: MoveGates::default(),
        start_impulse: None,
        smash_charge_mult: 1.0,
        landing_lag_s: None,
        autocancel_after_s: None,
    }
}
