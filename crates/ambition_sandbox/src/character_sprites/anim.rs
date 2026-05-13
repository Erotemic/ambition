//! Animation enum + per-actor animation pickers.
//!
//! `CharacterAnim` is the union of every animation row a character
//! sheet may define; the boss has its own row set, see
//! `boss_sprites::BossAnim`. A sheet doesn't have to define every
//! row — `CharacterSheetSpec::resolve_anim` falls back to `Idle` for
//! any row a sheet doesn't carry, so simple characters can list only
//! their relevant animations.

use ambition_engine as ae;

use crate::SandboxRuntime;

/// Animation ids that a character sheet may define.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CharacterAnim {
    Idle = 0,
    Walk = 1,
    Run = 2,
    Jump = 3,
    Fall = 4,
    Slash = 5,
    Hit = 6,
    Death = 7,
    BlinkOut = 8,
    BlinkIn = 9,
    Dash = 10,
    /// Free-flight pose (jets / hover). Maps to the generator's
    /// `hover` row — the row we emit when the robot config lists
    /// `hover` after `dash`.
    Fly = 11,
    /// Idle-variant gesture for hostile NPCs (pirate admiral / raider
    /// generators emit a `taunt` row between `slash` and `hurt`).
    /// Not currently produced by `pick_*_anim` — the row exists so
    /// atlas indexing aligns with the PNG even when nothing requests
    /// it, and so future combat-banter systems can pick it up.
    Taunt = 12,
    /// Held hang on a ledge — both arms gripping the ledge top with
    /// the body slumped below. Driven by `pick_player_anim` while
    /// `SandboxRuntime::ledge_grab` is `Some` and not climbing.
    LedgeGrab = 13,
    /// Pull-up transition after ledge grab. The placeholder robot sheet does
    /// not have a dedicated row yet; `CharacterSheetSpec::resolve_anim` falls
    /// back to `LedgeGrab` for this variant until art exists.
    LedgeClimb = 14,
}

pub(super) fn non_looping(anim: CharacterAnim) -> bool {
    matches!(
        anim,
        CharacterAnim::Slash | CharacterAnim::Hit | CharacterAnim::Death | CharacterAnim::LedgeClimb
    )
}

/// Pick the player's animation from runtime state.
///
/// Priority: hit > slash > fly > dash > airborne (jump/fall) > run/walk/idle.
/// Free-flight overrides ground/airborne motion because the engine
/// integrator already disables gravity in flight; the visual should
/// reflect the active mode rather than whatever fall/run inertia
/// happens to read.
/// Death is not represented yet — the player respawns instantly today.
/// `BlinkOut`/`BlinkIn` are not used yet because the runtime doesn't
/// track a per-blink anim window; once a `blink_anim_timer` is added
/// alongside `slash_anim_timer`, this function can switch on it.
pub fn pick_player_anim(runtime: &SandboxRuntime) -> CharacterAnim {
    if runtime.hitstun_timer > 0.05 {
        return CharacterAnim::Hit;
    }
    if runtime.slash_anim_timer > 0.0 {
        return CharacterAnim::Slash;
    }
    if runtime.player.blink_aiming || runtime.player.blink_hold_active {
        return CharacterAnim::BlinkOut;
    }
    if let Some(ledge) = runtime.ledge_grab.as_ref() {
        return if ledge.climbing {
            CharacterAnim::LedgeClimb
        } else {
            CharacterAnim::LedgeGrab
        };
    }
    let player = &runtime.player;
    if player.fly_enabled {
        return CharacterAnim::Fly;
    }
    if player.dash_timer > 0.0 {
        return CharacterAnim::Dash;
    }
    if !player.on_ground {
        // Engine uses top-left coords: vel.y < 0 = moving up.
        if player.vel.y < -10.0 {
            return CharacterAnim::Jump;
        }
        return CharacterAnim::Fall;
    }
    let speed = player.vel.x.abs();
    if speed < 12.0 {
        CharacterAnim::Idle
    } else if speed < 220.0 {
        CharacterAnim::Walk
    } else {
        CharacterAnim::Run
    }
}

/// Snapshot of an enemy's per-frame state used to drive its animation.
#[derive(Clone, Copy, Debug)]
pub struct EnemyAnimState {
    pub vel: ae::Vec2,
    pub facing: f32,
    pub alive: bool,
    pub attack_active: bool,
    pub attack_windup: bool,
    pub hit_flash: bool,
}

pub fn pick_enemy_anim(state: EnemyAnimState) -> CharacterAnim {
    if !state.alive {
        return CharacterAnim::Death;
    }
    if state.hit_flash {
        return CharacterAnim::Hit;
    }
    if state.attack_active || state.attack_windup {
        return CharacterAnim::Slash;
    }
    if state.vel.x.abs() > 8.0 {
        CharacterAnim::Walk
    } else {
        CharacterAnim::Idle
    }
}

/// Snapshot of a peaceful NPC's per-frame state for animation.
///
/// Smaller than `EnemyAnimState` because NPCs don't carry attack /
/// alive state (a hostile NPC is migrated to an `EnemyRuntime`
/// elsewhere; once the migration happens, the entity flows through
/// `pick_enemy_anim` instead).
#[derive(Clone, Copy, Debug)]
pub struct NpcAnimState {
    pub vel: ae::Vec2,
    pub facing: f32,
    pub hit_flash: bool,
}

/// Pick an NPC's animation. Hit-flash flickers `Hit` for a frame
/// after a strike; non-zero horizontal speed plays `Walk`; otherwise
/// `Idle`. Sheets without a Walk row fall back to Idle via
/// `CharacterSheetSpec::resolve_anim`, so a stationary General
/// rendered with the (idle-only) `ABSURD_GENERAL_SHEET` cycles its
/// 8 idle frames the moment a `CharacterAnimator` is attached.
pub fn pick_npc_anim(state: NpcAnimState) -> CharacterAnim {
    if state.hit_flash {
        return CharacterAnim::Hit;
    }
    if state.vel.x.abs() > 8.0 {
        CharacterAnim::Walk
    } else {
        CharacterAnim::Idle
    }
}
