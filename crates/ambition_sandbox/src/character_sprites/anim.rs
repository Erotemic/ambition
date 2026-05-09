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
}

pub(super) fn non_looping(anim: CharacterAnim) -> bool {
    matches!(
        anim,
        CharacterAnim::Slash | CharacterAnim::Hit | CharacterAnim::Death
    )
}

/// Pick the player's animation from runtime state.
///
/// Priority: hit > slash > dash > airborne (jump/fall) > run/walk/idle.
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
    let player = &runtime.player;
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
