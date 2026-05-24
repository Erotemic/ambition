//! Animation enum + per-actor animation pickers.
//!
//! `CharacterAnim` is the union of every animation row a character
//! sheet may define; the boss has its own row set, see
//! `boss_encounter::sprites::BossAnim`. A sheet doesn't have to define every
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
    /// Smash-Bros style ledge roll: tumble onto the platform with
    /// invulnerability frames. Selected by `pick_player_anim` when
    /// `ledge_grab.climbing && getup_kind == Roll`.
    LedgeRoll = 29,
    /// Ledge getup attack: swing onto the platform with an active
    /// hitbox. Selected by `pick_player_anim` when
    /// `ledge_grab.climbing && getup_kind == Attack`. The slash op
    /// fires at the start of the transition (engine side); the
    /// sprite should peak the swing mid-animation so visual + hitbox
    /// read as a single beat.
    LedgeGetupAttack = 30,
}

impl CharacterAnim {
    /// Map a generator-emitted row name (e.g. the lowercase strings in
    /// `*_spritesheet.ron`'s `rows[*].animation` field) to its enum
    /// variant. Returns `None` for names the runtime doesn't have a
    /// variant for — the row is silently dropped from the sheet spec.
    ///
    /// Accepted aliases:
    /// - `hurt` ↔ `Hit` (the goblin / pirate generators emit `hurt`,
    ///   but the runtime ECS animation picker uses `Hit`).
    /// - `hover` ↔ `Fly` (robot generator emits `hover` for the
    ///   jet-flight pose).
    /// - `opening` ↔ `Idle`, `stable` / `spin` ↔ `Walk`,
    ///   `closing` ↔ `Run` (interdimensional gate portal / ring sheets
    ///   borrow `CharacterAnim` slots for their phase-machine rows;
    ///   see `GATE_PORTAL_SHEET` / `GATE_RING_SHEET` docstrings in
    ///   `sheets.rs` for the runtime mapping).
    pub fn from_name(name: &str) -> Option<Self> {
        // Lowercase + strip nothing; we want exact matches against the
        // generator output strings.
        Some(match name {
            // `rest` (boss-encounter sheets), `front_idle` / `side_idle`
            // (girdle's facing-split sheet) — alias to Idle so the
            // catalog can pull every character in. A fully typed
            // CharacterAnim::Rest can land later if a consumer
            // distinguishes them.
            "idle" | "opening" | "rest" | "front_idle" | "side_idle" => Self::Idle,
            "walk" | "stable" | "spin" | "side_walk" => Self::Walk,
            "run" | "closing" => Self::Run,
            "jump" => Self::Jump,
            "fall" => Self::Fall,
            "slash" => Self::Slash,
            "hit" | "hurt" => Self::Hit,
            "death" => Self::Death,
            "blink_out" => Self::BlinkOut,
            "blink_in" => Self::BlinkIn,
            "dash" => Self::Dash,
            "fly" | "hover" => Self::Fly,
            "taunt" => Self::Taunt,
            "ledge_grab" => Self::LedgeGrab,
            "ledge_climb" => Self::LedgeClimb,
            "ledge_getup" => Self::LedgeGetup,
            "wall_grab" => Self::WallGrab,
            "float_glide" => Self::FloatGlide,
            "land_hard" => Self::LandHard,
            "land_recovery" => Self::LandRecovery,
            "dash_startup" => Self::DashStartup,
            "attack_side" => Self::AttackSide,
            "attack_up" => Self::AttackUp,
            "attack_down" => Self::AttackDown,
            "air_neutral" => Self::AirNeutral,
            "air_forward" => Self::AirForward,
            "air_back" => Self::AirBack,
            "air_down" => Self::AirDown,
            "air_up" => Self::AirUp,
            "ledge_roll" => Self::LedgeRoll,
            "ledge_getup_attack" => Self::LedgeGetupAttack,
            _ => return None,
        })
    }
}

pub(super) fn non_looping(anim: CharacterAnim) -> bool {
    matches!(
        anim,
        CharacterAnim::Slash
            | CharacterAnim::Hit
            | CharacterAnim::Death
            | CharacterAnim::LedgeClimb
            | CharacterAnim::LedgeGetup
            | CharacterAnim::LedgeRoll
            | CharacterAnim::LedgeGetupAttack
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
/// `attack` is the active swing from `player::ActivePlayerAttack`; `None` when idle.
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
        if !ledge.climbing {
            return CharacterAnim::LedgeGrab;
        }
        return match ledge.getup_kind {
            ae::LedgeGetupKind::Climb => CharacterAnim::LedgeGetup,
            ae::LedgeGetupKind::Roll => CharacterAnim::LedgeRoll,
            ae::LedgeGetupKind::Attack => CharacterAnim::LedgeGetupAttack,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::{PlayerBlinkCameraState, PlayerCombatState};

    /// Build a player + the three default state inputs that
    /// `pick_player_anim` consumes. Tests then mutate just the
    /// fields relevant to the case under test.
    fn pick_inputs() -> (
        PlayerAnimState,
        PlayerCombatState,
        PlayerBlinkCameraState,
        ae::Player,
    ) {
        (
            PlayerAnimState::default(),
            PlayerCombatState::default(),
            PlayerBlinkCameraState::default(),
            ae::Player::new(ae::Vec2::ZERO),
        )
    }

    fn hang_state(getup: ae::LedgeGetupKind, climbing: bool) -> ae::LedgeGrabState {
        ae::LedgeGrabState {
            contact: ae::LedgeContact {
                wall_normal_x: -1.0,
                anchor: ae::Vec2::new(86.0, 110.0),
                climb_target: ae::Vec2::new(115.0, 77.0),
            },
            elapsed: 0.1,
            climbing,
            getup_kind: getup,
            climb_elapsed: 0.0,
            momentum_at_grab: ae::Vec2::ZERO,
        }
    }

    /// While hanging (not climbing), the picker returns the static
    /// `LedgeGrab` row regardless of getup_kind. The hang is the
    /// pre-action state; the getup kind is only meaningful once
    /// the player commits.
    #[test]
    fn hang_returns_ledge_grab_regardless_of_getup_kind() {
        for kind in [
            ae::LedgeGetupKind::Climb,
            ae::LedgeGetupKind::Roll,
            ae::LedgeGetupKind::Attack,
        ] {
            let (anim, combat, blink, mut player) = pick_inputs();
            player.ledge_grab = Some(hang_state(kind, false));
            assert_eq!(
                pick_player_anim(&anim, &combat, &blink, None, &player),
                CharacterAnim::LedgeGrab,
                "hang with kind {:?} must read as LedgeGrab",
                kind,
            );
        }
    }

    /// Climb is the default getup; picker should return the
    /// `LedgeGetup` row (the existing mantle pop-up animation).
    #[test]
    fn climbing_with_climb_kind_returns_ledge_getup() {
        let (anim, combat, blink, mut player) = pick_inputs();
        player.ledge_grab = Some(hang_state(ae::LedgeGetupKind::Climb, true));
        assert_eq!(
            pick_player_anim(&anim, &combat, &blink, None, &player),
            CharacterAnim::LedgeGetup,
        );
    }

    /// Roll getup picks the new `LedgeRoll` row.
    #[test]
    fn climbing_with_roll_kind_returns_ledge_roll() {
        let (anim, combat, blink, mut player) = pick_inputs();
        player.ledge_grab = Some(hang_state(ae::LedgeGetupKind::Roll, true));
        assert_eq!(
            pick_player_anim(&anim, &combat, &blink, None, &player),
            CharacterAnim::LedgeRoll,
        );
    }

    /// Attack getup picks the new `LedgeGetupAttack` row. The
    /// `slash_anim_timer` happens to be 0 here so the regular
    /// directional-attack branch doesn't preempt the ledge branch;
    /// the next test pins that ordering.
    #[test]
    fn climbing_with_attack_kind_returns_ledge_getup_attack() {
        let (anim, combat, blink, mut player) = pick_inputs();
        player.ledge_grab = Some(hang_state(ae::LedgeGetupKind::Attack, true));
        assert_eq!(
            pick_player_anim(&anim, &combat, &blink, None, &player),
            CharacterAnim::LedgeGetupAttack,
        );
    }

    /// The non-looping list must include the two new ledge rows so
    /// `CharacterAnimator` doesn't keep cycling their frames after
    /// the engine transition completes. Regression guard against
    /// adding new variants and forgetting the `non_looping` entry.
    #[test]
    fn new_ledge_rows_are_non_looping() {
        assert!(non_looping(CharacterAnim::LedgeRoll));
        assert!(non_looping(CharacterAnim::LedgeGetupAttack));
        // Sanity: the prior LedgeGetup also stays non-looping.
        assert!(non_looping(CharacterAnim::LedgeGetup));
    }

    /// `from_name` round-trips the new row names so the spritesheet
    /// RON parser can resolve `"ledge_roll"` / `"ledge_getup_attack"`
    /// from the generator output without dropping them silently.
    #[test]
    fn from_name_resolves_new_ledge_rows() {
        assert_eq!(
            CharacterAnim::from_name("ledge_roll"),
            Some(CharacterAnim::LedgeRoll),
        );
        assert_eq!(
            CharacterAnim::from_name("ledge_getup_attack"),
            Some(CharacterAnim::LedgeGetupAttack),
        );
    }
}
