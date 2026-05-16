//! Animation enum + per-actor animation pickers.
//!
//! `CharacterAnim` is the union of every animation row a character
//! sheet may define; the boss has its own row set, see
//! `boss_sprites::BossAnim`. A sheet doesn't have to define every
//! row — `CharacterSheetSpec::resolve_anim` falls back to `Idle` for
//! any row a sheet doesn't carry, so simple characters can list only
//! their relevant animations.

use ambition_engine as ae;

use crate::player::PlayerAnimState;

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
    /// `Player::ledge_grab` is `Some` and not climbing.
    LedgeGrab = 13,
    /// Slow, deliberate "haul-yourself-up" loop against an overhead grip.
    /// The new robot sheet ships a dedicated row; `pick_player_anim` does
    /// not currently auto-route to it (mantle pop-ups go through
    /// `LedgeGetup` instead), but the variant exists so future climb
    /// gestures can request it.
    LedgeClimb = 14,
    /// Mantle pop-up: arms transition from overhead grip to planted push
    /// to standing in one short burst. Driven by `pick_player_anim` when
    /// `Player::ledge_grab.climbing == true`.
    LedgeGetup = 15,
    /// Pinned against a wall (both hands flat). Distinct from `wall_slide`
    /// (which is the engine's downward-scrape state) — `WallGrab` plays
    /// when the player is wall-clinging but not sliding/climbing.
    WallGrab = 16,
    /// Sustained held-jump glide pose (arms out as balance wings).
    /// Driven by `player.gliding`; distinct from `Fly` (rocket jets) and
    /// the airborne `Fall` row.
    FloatGlide = 17,
    /// Heavy landing — big squash, slow rebound. Triggered when the
    /// landing transition was hit at a high downward speed; consumed by
    /// `pick_player_anim` while `PlayerAnimState::land_anim_timer` is
    /// positive and `land_anim_hard` is set.
    LandHard = 18,
    /// Rising recovery after a (hard) landing. Plays when
    /// `land_anim_timer` is positive but `land_anim_hard` is false, or
    /// during the tail of a hard landing.
    LandRecovery = 19,
    /// Brief animation-only dash pre-roll. Plays for the first ~50ms
    /// after a dash starts so the sprite has a discrete wind-up read
    /// before falling through to the streaking `Dash` row.
    DashStartup = 20,
    /// Grounded side-slash (Marth-style horizontal swing). Drives both
    /// the `Forward` / `Neutral` / `Back` / `DashForward` / `WallOut`
    /// engine attack intents — the four variants share one swing read
    /// since the sprite already flips with the player's facing.
    AttackSide = 21,
    /// Grounded up-tilt — overhead arc.
    AttackUp = 22,
    /// Grounded down-tilt — sweep down to the floor.
    AttackDown = 23,
    /// Aerial neutral spin-slash. No `AttackIntent::AirNeutral` exists
    /// yet, so this row is currently selected by `pick_player_anim` only
    /// when the future intent appears; the row is on the sheet so
    /// designers can iterate the shape regardless.
    AirNeutral = 24,
    /// Aerial forward swing.
    AirForward = 25,
    /// Aerial backward swing (no engine intent yet — placeholder row).
    AirBack = 26,
    /// Aerial down-thrust (spike).
    AirDown = 27,
    /// Aerial up-thrust.
    AirUp = 28,
}

pub(super) fn non_looping(anim: CharacterAnim) -> bool {
    matches!(
        anim,
        CharacterAnim::Slash
            | CharacterAnim::Hit
            | CharacterAnim::Death
            | CharacterAnim::LedgeClimb
            | CharacterAnim::LedgeGetup
            | CharacterAnim::LandHard
            | CharacterAnim::LandRecovery
            | CharacterAnim::DashStartup
            | CharacterAnim::AttackSide
            | CharacterAnim::AttackUp
            | CharacterAnim::AttackDown
            | CharacterAnim::AirNeutral
            | CharacterAnim::AirForward
            | CharacterAnim::AirBack
            | CharacterAnim::AirDown
            | CharacterAnim::AirUp
    )
}

/// Pick the player's animation from ECS animation state and engine state.
///
/// Priority: hit > blink-in/out > slash > fly > dash > airborne (jump/fall) > run/walk/idle.
/// Free-flight overrides ground/airborne motion because the engine
/// integrator already disables gravity in flight; the visual should
/// reflect the active mode rather than whatever fall/run inertia
/// happens to read.
/// Death is not represented yet — the player respawns instantly today.
/// `BlinkOut` is used while the blink button is held/aiming, and
/// `BlinkIn` is held briefly after a committed blink so VFX/camera have
/// time to sell the arrival.
///
/// `anim` is the authoritative ECS component for presentation timers.
/// `combat` provides `hitstun_timer` (now on `PlayerCombatState`).
/// `blink_cam` provides `blink_in_timer` (now on `PlayerBlinkCameraState`).
/// `attack` is the active swing from `CurrentPlayerAttack`; `None` when idle.
pub fn pick_player_anim(
    anim: &PlayerAnimState,
    combat: &crate::player::PlayerCombatState,
    blink_cam: &crate::player::PlayerBlinkCameraState,
    attack: Option<&crate::PlayerAttackState>,
    player: &ae::Player,
) -> CharacterAnim {
    if combat.hitstun_timer > 0.05 {
        return CharacterAnim::Hit;
    }
    if blink_cam.blink_in_timer > 0.0 {
        return CharacterAnim::BlinkIn;
    }
    if anim.slash_anim_timer > 0.0 {
        return directional_attack_anim(attack);
    }
    if player.blink_aiming || player.blink_hold_active {
        return CharacterAnim::BlinkOut;
    }
    if let Some(ledge) = player.ledge_grab.as_ref() {
        return if ledge.climbing {
            CharacterAnim::LedgeGetup
        } else {
            CharacterAnim::LedgeGrab
        };
    }
    if player.fly_enabled {
        return CharacterAnim::Fly;
    }
    if anim.dash_startup_timer > 0.0 {
        return CharacterAnim::DashStartup;
    }
    if player.dash_timer > 0.0 {
        return CharacterAnim::Dash;
    }
    // Wall pin (held against the wall, neither sliding nor climbing) reads
    // distinct from the engine's downward `wall_slide` integration.
    if !player.on_ground
        && player.wall_clinging
        && !player.wall_climbing
        && player.vel.y.abs() < 40.0
    {
        return CharacterAnim::WallGrab;
    }
    if player.gliding {
        return CharacterAnim::FloatGlide;
    }
    if !player.on_ground {
        // Engine uses top-left coords: vel.y < 0 = moving up.
        if player.vel.y < -10.0 {
            return CharacterAnim::Jump;
        }
        return CharacterAnim::Fall;
    }
    if anim.land_anim_timer > 0.0 {
        return if anim.land_anim_hard {
            CharacterAnim::LandHard
        } else {
            CharacterAnim::LandRecovery
        };
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

/// Map the active player attack intent onto the directional swing rows.
///
/// The engine's `AttackIntent` is finer-grained than the visible swing
/// shapes — multiple intents share one row because the sprite already
/// flips with the player's facing.
fn directional_attack_anim(attack: Option<&crate::PlayerAttackState>) -> CharacterAnim {
    use ae::AttackIntent;
    let Some(attack) = attack else {
        // Defensive fallback: slash_anim_timer is set but no attack
        // state — keep the old side-swing read until the timer drains.
        return CharacterAnim::AttackSide;
    };
    match attack.spec.intent {
        AttackIntent::Up => CharacterAnim::AttackUp,
        AttackIntent::Down => CharacterAnim::AttackDown,
        AttackIntent::AirUp => CharacterAnim::AirUp,
        AttackIntent::AirDown => CharacterAnim::AirDown,
        AttackIntent::AirForward => CharacterAnim::AirForward,
        AttackIntent::AirBack => CharacterAnim::AirBack,
        AttackIntent::Neutral
        | AttackIntent::Forward
        | AttackIntent::Back
        | AttackIntent::DashForward
        | AttackIntent::WallOut => CharacterAnim::AttackSide,
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
