//! **The primitives a character's move table is written with** — shared, because
//! the second character to author one must not begin by copying the first.
//!
//! ⭐⭐ **IT MOVED DOWN HERE ON 2026-08-16, and the reason is the same one that
//! moved `build_actor_moveset`**: it lived in `ambition_content`, so a character
//! belonging to any OTHER provider could not use it. Mary-O and Sanic are
//! registered by their own demos, both were 0/16 on the smash grid, and neither
//! demo depends on Ambition's content crate — so completing their kits meant
//! either a fourth copy of these helpers or moving the one copy to a crate
//! everybody already has. `ambition_characters` is where the character model
//! lives and where `moveset_prefabs` already derives a table from an action set;
//! authoring one by hand belongs beside it.
//!
//! ⚠ `ambition_demo_smash` still carries its OWN fork of these (`crate::moveset`
//! in that crate, with a `Feel` tag this one has no concept of). Unifying it is
//! its own change and would expose what the fork hides; it is not this one.
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

use crate::moveset_prefabs::SLASH_ARC_VFX;
use ambition_entity_catalog::{
    ClipBinding, EffectRef, HitVolume, ImpulseMode, MoveEvent, MoveEventKind, MoveGates, MoveSpec,
    MoveWindow, VolumeShape, WindowTag,
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

/// **A move that answers its button from the ground OR the air.** The specials
/// are the moves that do this: a recovery that could only be pressed with your
/// feet down is not a recovery.
pub fn either_posture() -> MoveGates {
    MoveGates { grounded: None }
}

fn event(mut m: MoveSpec, at_s: f32, kind: MoveEventKind) -> MoveSpec {
    m.events.push(MoveEvent { at_s, kind });
    m
}

/// **A TIMED SELF-DISPLACEMENT.**
///
/// ⭐ [`ImpulseMode::Set`] COMMANDS a velocity; [`ImpulseMode::Add`] contributes
/// to one. The difference is the difference between a recovery and a hop: a body
/// falling at terminal velocity gets exactly the same result from a `Set` as a
/// standing one does, and the worst possible result from an `Add`. It is also
/// what the catalog's `lift_speed` / `lift_side` derivation keys on — an `Add`
/// states no speed, so no static reader may claim one for it.
///
/// `local` is body-local: `+x` toward facing, `+y` toward the feet, so a rise is
/// a NEGATIVE second component.
pub fn impulse(m: MoveSpec, at_s: f32, local: (f32, f32), mode: ImpulseMode) -> MoveSpec {
    event(m, at_s, MoveEventKind::Impulse { local, mode })
}

/// **A CUE AT A MOMENT.** The move's own timeline is where its sound lives, so a
/// windup you can hear and a swing you can hear are two events and not two
/// systems.
pub fn sfx(m: MoveSpec, at_s: f32, cue: &str) -> MoveSpec {
    event(
        m,
        at_s,
        MoveEventKind::Sfx {
            cue: cue.to_string(),
        },
    )
}

/// **A BURST AT A MOMENT.**
///
/// ⚠ `effect` is the NAME of a row on one of the shipped FX spritesheets
/// (`ambition_sprite_sheet::fx` — 189 of them). `MoveSpec::presentation_problems`
/// refuses a name no sheet carries, and the renderer counts it as a miss rather
/// than playing nothing quietly.
pub fn vfx(m: MoveSpec, at_s: f32, effect: &str) -> MoveSpec {
    event(
        m,
        at_s,
        MoveEventKind::Vfx {
            effect: effect.to_string(),
            at: (0.0, 0.0),
            scale: 1.0,
        },
    )
}

/// **A burst that says WHERE and HOW BIG**, in the same body-local numbers the
/// move's strike volumes use.
///
/// Jon, 2026-08-16: *"try to make the hitboxes and vfx placement make sense,
/// right now we are seeing crazy upscaled vfx and very tiny hitboxes"*. Plain
/// [`vfx`] draws at the owner's centre at the presentation default size, which
/// is the right identity for an effect ABOUT the fighter (a transformation, a
/// shield) and the wrong one for a swing: the spark belongs on the box.
///
/// ⭐ pass a volume's own `offset` as `at` and the two cannot disagree.
pub fn vfx_at(m: MoveSpec, at_s: f32, effect: &str, at: (f32, f32), scale: f32) -> MoveSpec {
    event(
        m,
        at_s,
        MoveEventKind::Vfx {
            effect: effect.to_string(),
            at,
            scale,
        },
    )
}

/// **WHAT LANDING THIS MOVE SOUNDS LIKE**, applied to every volume it throws.
/// Contact feedback belongs to the volume because only the volume knows it
/// connected.
pub fn on_contact(mut m: MoveSpec, cue: &str) -> MoveSpec {
    for volume in m.windows.iter_mut().flat_map(|w| w.volumes.iter_mut()) {
        volume.hit_sfx = Some(cue.to_string());
    }
    m
}

/// **HOW THE SWING ITSELF IS DRAWN** — the strike-presentation tag every volume
/// this move throws carries.
///
/// ⛔ **this is NOT an FX-sheet row name.** `HitVolume::vfx` is a two-word
/// vocabulary ([`SLASH_ARC_VFX`] /
/// [`SLASH_POKE_VFX`](ambition_characters::moveset_prefabs::SLASH_POKE_VFX))
/// that the move runtime
/// reads twice: it picks the arc-vs-jab shape drawn out of the spawned volume,
/// and it is the flag that makes a volume prefer the sprite manifest's authored
/// hit polygon for this move's clip over the synthetic box. A sheet row name put
/// here would silently take a move off both paths. Per-move ART is a
/// [`vfx`] EVENT; this is how the SWING reads.
///
/// [`strike`] tags every volume `slash_arc`, which is right for a committed
/// swing and wrong for a poke — so this exists for the pokes.
pub fn strike_tag(mut m: MoveSpec, tag: &str) -> MoveSpec {
    for volume in m.windows.iter_mut().flat_map(|w| w.volumes.iter_mut()) {
        volume.vfx = Some(tag.to_string());
    }
    m
}

/// **A TAIL THE BODY CANNOT STEER OUT OF.** Extends the move to `to_s` with a
/// Recovery window whose `motion_scale` damps the owner's steering — the genre's
/// "you are committed now", authored rather than hardcoded, and enforced
/// body-side so it binds a CPU and a human identically.
pub fn committed_tail(mut m: MoveSpec, to_s: f32, motion_scale: f32) -> MoveSpec {
    let from = m.duration_s;
    if to_s <= from {
        return m;
    }
    m.windows.push(MoveWindow {
        start_s: from,
        end_s: to_s,
        tag: WindowTag::Recovery,
        volumes: Vec::new(),
        motion_scale,
        sustain_effect: None,
    });
    m.duration_s = to_s;
    m
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
                    // point different ways. ⚠ a POKE wants the other tag — see
                    // [`strike_tag`].
                    vfx: Some(SLASH_ARC_VFX.to_string()),
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
