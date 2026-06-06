//! Animation enum + per-actor animation pickers.
//!
//! `CharacterAnim` is the union of every animation row a character
//! sheet may define; the boss has its own row set, see
//! `boss_encounter::sprites::BossAnim`. A sheet doesn't have to define every
//! row — `CharacterSheetSpec::resolve_anim` falls back to `Idle` for
//! any row a sheet doesn't carry, so simple characters can list only
//! their relevant animations.

use crate::engine_core as ae;

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
    /// Compressed crouch pose. Selected by `pick_player_anim` while
    /// `body_mode.body_mode == BodyMode::Crouching` and the player is
    /// not actively walking. Matches the generator's `crouch` row.
    Crouch = 31,
    /// Hands-and-knees crawl. Selected while
    /// `body_mode.body_mode == BodyMode::Crawling`. Matches the
    /// generator's `crouch_walk` row (the renderer reuses the crouch
    /// silhouette with a longer stride).
    Crawl = 32,
    /// Forward slide along the ground (low profile, momentum-carrying).
    /// Selected while `body_mode.body_mode == BodyMode::Sliding`.
    /// Matches the generator's `slide` row.
    Slide = 33,
    /// Ladder / vine climb. Selected while
    /// `body_mode.body_mode == BodyMode::Climbing`. Maps to the
    /// generator's `climb` row (one hand over the other).
    LadderClimb = 34,
    /// Submerged swim stroke. Selected while the player is in water
    /// (`env_contact.water.is_some()`) and the `swim` ability is
    /// enabled. Maps to the generator's `swim` row.
    Swim = 35,
    /// Projectile fire pose — single-frame arm extension at the
    /// release point. Generator row `shoot`. Not yet auto-routed by
    /// `pick_player_anim`; needs a `shoot_anim_timer` on
    /// `PlayerAnimState` set when a projectile spawns.
    Shoot = 36,
    /// Held-projectile charge / aim pose. Generator row `aim`. Not
    /// yet auto-routed; needs the projectile charge state on the
    /// player to surface as a presentation flag.
    Aim = 37,
    /// Held-attack charge pose (heavy/release combo wind-up).
    /// Generator row `charge`. No engine intent maps to this today;
    /// the row exists so designers can iterate the wind-up shape.
    Charge = 38,
    /// Defensive bubble / shield-up pose. Generator row `block`.
    /// Routes from the player's shield-held state once that's mirrored
    /// onto `PlayerAnimState` (today the input is `ControlFrame.
    /// shield_held` + `AbilitySet::shield`).
    Block = 39,
    /// Tumbling dodge roll across the ground — invulnerability frames
    /// during the curl. Generator row `roll`. Selected by
    /// `pick_player_anim` once the dodge-roll timer surfaces on
    /// `PlayerAnimState`; distinct from `LedgeRoll` (the latter is
    /// specifically the ledge-getup variant).
    DodgeRoll = 40,
    /// Wall-jump push-off pose. Generator row `wall_jump`. Distinct
    /// from `Jump` (which is the grounded jump arc) and `WallGrab`
    /// (which is the cling pose). Not yet auto-routed; needs a brief
    /// `wall_jump_anim_timer` armed by the wall-jump op.
    WallJump = 41,
    /// Interaction gesture (talk / open / pickup). Generator row
    /// `interact`. Not yet auto-routed; needs an `interact_anim_timer`
    /// armed by the interact buffer firing.
    Interact = 42,
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
            "idle" | "opening" | "rest" | "front_idle" | "side_idle" | "classic_burst" => {
                Self::Idle
            }
            "walk" | "stable" | "spin" | "side_walk" | "burst_round" => Self::Walk,
            "run" | "closing" | "shockwave" => Self::Run,
            "jump" => Self::Jump,
            "fall" => Self::Fall,
            "slash" | "starburst" => Self::Slash,
            "hit" | "hurt" | "smoke_burst" => Self::Hit,
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
            "crouch" => Self::Crouch,
            // The generator emits `crouch_walk` for the crawl pose.
            "crouch_walk" | "crawl" => Self::Crawl,
            "slide" => Self::Slide,
            "climb" | "ladder_climb" => Self::LadderClimb,
            "swim" => Self::Swim,
            "shoot" => Self::Shoot,
            "aim" => Self::Aim,
            "charge" => Self::Charge,
            "block" | "shield" => Self::Block,
            "roll" | "dodge_roll" => Self::DodgeRoll,
            "wall_jump" => Self::WallJump,
            "interact" => Self::Interact,
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
            // New action poses: one-shot reads that should hold the
            // final frame instead of looping back.
            | CharacterAnim::Shoot
            | CharacterAnim::DodgeRoll
            | CharacterAnim::WallJump
            | CharacterAnim::Interact
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
///
/// Phase 2 migration: the remaining player state (velocity, ground,
/// wall, blink/aim, flight, dash, ledge) comes in as five cluster
/// component references so this helper has no dependency on the
/// legacy `ae::Player` aggregate.
#[allow(clippy::too_many_arguments)]
pub fn pick_player_anim(
    anim: &PlayerAnimState,
    combat: &crate::player::PlayerCombatState,
    blink_cam: &crate::player::PlayerBlinkCameraState,
    attack: Option<&crate::PlayerAttackState>,
    kinematics: &crate::player::BodyKinematics,
    ground: &crate::player::PlayerGroundState,
    wall: &crate::player::PlayerWallState,
    blink: &crate::player::PlayerBlinkState,
    flight: &crate::player::PlayerFlightState,
    dash: &crate::player::PlayerDashState,
    ledge: &crate::player::PlayerLedgeState,
    body_mode: &crate::player::PlayerBodyModeState,
    env_contact: &crate::player::PlayerEnvironmentContact,
    abilities: &crate::player::PlayerAbilities,
    dodge: &crate::player::PlayerDodgeState,
    shield: &crate::player::PlayerShieldState,
) -> CharacterAnim {
    if combat.hitstun_timer > 0.05 {
        return CharacterAnim::Hit;
    }
    // Dodge roll wins over slash / blink-aim — once the player commits
    // to a ground roll the curl-pose carries the i-frames; nothing else
    // should clobber the read until the timer drains.
    //
    // Gated on `ledge.grab.is_none()` because ledge_roll +
    // ledge_getup_attack also set `dodge.roll_timer` (the same timer
    // drives their i-frames). Without the gate, every ledge roll
    // would visibly play the grounded-roll pose instead of the
    // dedicated `LedgeRoll` / `LedgeGetupAttack` rows.
    if dodge.roll_timer > 0.0 && ledge.grab.is_none() {
        return CharacterAnim::DodgeRoll;
    }
    if blink_cam.blink_in_timer > 0.0 {
        return CharacterAnim::BlinkIn;
    }
    // Shield held wins over slash so the bubble-up posture stays
    // legible while the parry window is open.
    if shield.active && abilities.abilities.shield {
        return CharacterAnim::Block;
    }
    // Projectile release fires for ~SHOOT_ANIM_HOLD_SECS, set by the
    // projectile system on spawn. Held above slash so the muzzle-flash
    // pose isn't immediately stomped by a same-frame swing.
    if anim.shoot_anim_timer > 0.0 {
        return CharacterAnim::Shoot;
    }
    if anim.slash_anim_timer > 0.0 {
        return directional_attack_anim(attack);
    }
    // Held charge — only relevant while the player is actually charging
    // a projectile and no other action is in flight. Below slash so a
    // mid-charge swing breaks the aim pose immediately.
    if anim.aim_anim_active {
        return CharacterAnim::Aim;
    }
    // Wall-jump push-off pose. Triggered on the WallJump op edge and
    // held briefly so the kick reads even as the player is already
    // arcing away from the wall. Above the airborne Jump/Fall block.
    if anim.wall_jump_anim_timer > 0.0 {
        return CharacterAnim::WallJump;
    }
    // Interact gesture (door tap, NPC talk, pickup). Brief one-shot
    // held while the interaction commits.
    if anim.interact_anim_timer > 0.0 {
        return CharacterAnim::Interact;
    }
    if blink.aiming || blink.hold_active {
        return CharacterAnim::BlinkOut;
    }
    if let Some(ledge_state) = ledge.grab.as_ref() {
        if !ledge_state.climbing {
            return CharacterAnim::LedgeGrab;
        }
        return match ledge_state.getup_kind {
            ae::LedgeGetupKind::Climb => CharacterAnim::LedgeGetup,
            ae::LedgeGetupKind::Roll => CharacterAnim::LedgeRoll,
            ae::LedgeGetupKind::Attack => CharacterAnim::LedgeGetupAttack,
        };
    }
    if flight.fly_enabled {
        return CharacterAnim::Fly;
    }
    // Submerged + swim-capable overrides ground locomotion. Body shape
    // doesn't change but the stroke pose is distinct from walk/run.
    if env_contact.water.is_some() && abilities.abilities.swim {
        return CharacterAnim::Swim;
    }
    if anim.dash_startup_timer > 0.0 {
        return CharacterAnim::DashStartup;
    }
    if dash.timer > 0.0 {
        return CharacterAnim::Dash;
    }
    // Ladder climb pose: BodyMode::Climbing is set by the body-mode
    // driver when the player is on a climbable contact and pushes
    // up/down. Suppresses gravity; needs its own row distinct from
    // wall-climb on solid blocks.
    if matches!(
        body_mode.body_mode,
        crate::engine_core::player_state::BodyMode::Climbing
    ) {
        return CharacterAnim::LadderClimb;
    }
    // Wall pin (held against the wall, neither sliding nor climbing) reads
    // distinct from the engine's downward `wall_slide` integration.
    if !ground.on_ground
        && wall.wall_clinging
        && !wall.wall_climbing
        && kinematics.vel.y.abs() < 40.0
    {
        return CharacterAnim::WallGrab;
    }
    if flight.gliding {
        return CharacterAnim::FloatGlide;
    }
    if !ground.on_ground {
        // Engine uses top-left coords: vel.y < 0 = moving up.
        if kinematics.vel.y < -10.0 {
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
    // Compact body modes — same shape as the engine collision change,
    // distinct silhouette read. Sliding wins over Crawl/Crouch because
    // it usually carries kinetic momentum and the pose differs.
    use crate::engine_core::player_state::BodyMode;
    match body_mode.body_mode {
        BodyMode::Sliding => return CharacterAnim::Slide,
        BodyMode::Crawling => return CharacterAnim::Crawl,
        BodyMode::Crouching => return CharacterAnim::Crouch,
        _ => {}
    }
    let speed = kinematics.vel.x.abs();
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
    use crate::combat::AttackIntent;
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
    /// World position — resolves this actor's *localized* gravity so the sprite
    /// flips the right way when it's wall-walking / on a flipped-gravity ceiling.
    pub pos: ae::Vec2,
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
    /// World position — see [`EnemyAnimState::pos`] (localized-gravity flip).
    pub pos: ae::Vec2,
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
    /// Bundle of every cluster component `pick_player_anim` reads.
    /// Tests mutate just the fields relevant to the case under test.
    struct PickClusters {
        kinematics: crate::player::BodyKinematics,
        ground: crate::player::PlayerGroundState,
        wall: crate::player::PlayerWallState,
        blink: crate::player::PlayerBlinkState,
        flight: crate::player::PlayerFlightState,
        dash: crate::player::PlayerDashState,
        ledge: crate::player::PlayerLedgeState,
        body_mode: crate::player::PlayerBodyModeState,
        env_contact: crate::player::PlayerEnvironmentContact,
        abilities: crate::player::PlayerAbilities,
        dodge: crate::player::PlayerDodgeState,
        shield: crate::player::PlayerShieldState,
    }

    impl PickClusters {
        fn defaults() -> Self {
            Self {
                kinematics: Default::default(),
                ground: Default::default(),
                wall: Default::default(),
                blink: Default::default(),
                flight: Default::default(),
                dash: Default::default(),
                ledge: Default::default(),
                body_mode: Default::default(),
                env_contact: Default::default(),
                abilities: Default::default(),
                dodge: Default::default(),
                shield: Default::default(),
            }
        }
    }

    fn pick_inputs() -> (
        PlayerAnimState,
        PlayerCombatState,
        PlayerBlinkCameraState,
        PickClusters,
    ) {
        (
            PlayerAnimState::default(),
            PlayerCombatState::default(),
            PlayerBlinkCameraState::default(),
            PickClusters::defaults(),
        )
    }

    fn pick(
        anim: &PlayerAnimState,
        combat: &PlayerCombatState,
        blink_cam: &PlayerBlinkCameraState,
        attack: Option<&crate::PlayerAttackState>,
        c: &PickClusters,
    ) -> CharacterAnim {
        pick_player_anim(
            anim,
            combat,
            blink_cam,
            attack,
            &c.kinematics,
            &c.ground,
            &c.wall,
            &c.blink,
            &c.flight,
            &c.dash,
            &c.ledge,
            &c.body_mode,
            &c.env_contact,
            &c.abilities,
            &c.dodge,
            &c.shield,
        )
    }

    fn hang_state(getup: ae::LedgeGetupKind, climbing: bool) -> ae::LedgeGrabState {
        let contact = ae::LedgeContact {
            wall_normal_x: -1.0,
            anchor: ae::Vec2::new(86.0, 110.0),
            climb_target: ae::Vec2::new(115.0, 77.0),
        };
        let mut state = ae::LedgeGrabState::hanging(contact);
        state.elapsed = 0.1;
        state.climbing = climbing;
        state.getup_kind = getup;
        state
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
            let (anim, combat, blink_cam, mut clusters) = pick_inputs();
            clusters.ledge.grab = Some(hang_state(kind, false));
            assert_eq!(
                pick(&anim, &combat, &blink_cam, None, &clusters),
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
        let (anim, combat, blink_cam, mut clusters) = pick_inputs();
        clusters.ledge.grab = Some(hang_state(ae::LedgeGetupKind::Climb, true));
        assert_eq!(
            pick(&anim, &combat, &blink_cam, None, &clusters),
            CharacterAnim::LedgeGetup,
        );
    }

    /// Roll getup picks the new `LedgeRoll` row.
    #[test]
    fn climbing_with_roll_kind_returns_ledge_roll() {
        let (anim, combat, blink_cam, mut clusters) = pick_inputs();
        clusters.ledge.grab = Some(hang_state(ae::LedgeGetupKind::Roll, true));
        assert_eq!(
            pick(&anim, &combat, &blink_cam, None, &clusters),
            CharacterAnim::LedgeRoll,
        );
    }

    /// Attack getup picks the new `LedgeGetupAttack` row. The
    /// `slash_anim_timer` happens to be 0 here so the regular
    /// directional-attack branch doesn't preempt the ledge branch;
    /// the next test pins that ordering.
    #[test]
    fn climbing_with_attack_kind_returns_ledge_getup_attack() {
        let (anim, combat, blink_cam, mut clusters) = pick_inputs();
        clusters.ledge.grab = Some(hang_state(ae::LedgeGetupKind::Attack, true));
        assert_eq!(
            pick(&anim, &combat, &blink_cam, None, &clusters),
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

    /// Shield-up flag wins over slash / aim. Only fires when
    /// `abilities.shield` is true — otherwise the shield cluster's
    /// `active` flag is unreachable from input.
    #[test]
    fn shield_active_with_ability_returns_block() {
        let (anim, combat, blink_cam, mut clusters) = pick_inputs();
        clusters.abilities.abilities.shield = true;
        clusters.shield.active = true;
        assert_eq!(
            pick(&anim, &combat, &blink_cam, None, &clusters),
            CharacterAnim::Block,
        );
    }

    /// Grounded dodge roll picks `DodgeRoll`, but a roll that fires as
    /// part of a ledge getup keeps the dedicated `LedgeRoll` row. The
    /// engine drives both with the same `dodge.roll_timer`; this pins
    /// the visual gate that picks the right pose for the situation.
    #[test]
    fn dodge_roll_grounded_vs_ledge_getup() {
        // Grounded: no ledge state, just a dodge timer.
        let (anim, combat, blink_cam, mut clusters) = pick_inputs();
        clusters.dodge.roll_timer = 0.2;
        assert_eq!(
            pick(&anim, &combat, &blink_cam, None, &clusters),
            CharacterAnim::DodgeRoll,
        );
        // Ledge roll: same timer set, plus a ledge_grab climbing roll.
        // The ledge-state branch must win.
        let (anim, combat, blink_cam, mut clusters) = pick_inputs();
        clusters.dodge.roll_timer = 0.2;
        clusters.ledge.grab = Some(hang_state(ae::LedgeGetupKind::Roll, true));
        assert_eq!(
            pick(&anim, &combat, &blink_cam, None, &clusters),
            CharacterAnim::LedgeRoll,
        );
    }

    /// Swim row picks when the player is in water AND has the swim
    /// ability. Without the ability the picker falls back to whatever
    /// locomotion the kinematics imply (gravity will fight it but at
    /// least the sprite isn't trying to play a swim row the character
    /// can't actually do).
    #[test]
    fn water_contact_with_swim_ability_returns_swim() {
        let water = ae::WaterContact {
            kind: ae::WaterKind::Clear,
            region_aabb: ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(64.0, 64.0)),
            surface_y: 0.0,
            submersion: 1.0,
            spec: ae::WaterVolumeSpec::default(),
        };
        let (anim, combat, blink_cam, mut clusters) = pick_inputs();
        clusters.abilities.abilities.swim = true;
        clusters.env_contact.water = Some(water);
        assert_eq!(
            pick(&anim, &combat, &blink_cam, None, &clusters),
            CharacterAnim::Swim,
        );
        // Same water contact but no swim ability — picker should NOT
        // return Swim.
        let (anim, combat, blink_cam, mut clusters) = pick_inputs();
        clusters.env_contact.water = Some(water);
        assert_ne!(
            pick(&anim, &combat, &blink_cam, None, &clusters),
            CharacterAnim::Swim,
        );
    }

    /// BodyMode::Climbing picks `LadderClimb` (distinct from the
    /// wall-grab path which is for solid-block wall-cling).
    #[test]
    fn climbing_body_mode_returns_ladder_climb() {
        use crate::engine_core::player_state::BodyMode;
        let (anim, combat, blink_cam, mut clusters) = pick_inputs();
        clusters.body_mode.body_mode = BodyMode::Climbing;
        assert_eq!(
            pick(&anim, &combat, &blink_cam, None, &clusters),
            CharacterAnim::LadderClimb,
        );
    }

    /// Crouching body mode picks `Crouch` from the locomotion
    /// fallback once the airborne / cling / dash branches all fall
    /// through.
    #[test]
    fn crouching_body_mode_returns_crouch() {
        use crate::engine_core::player_state::BodyMode;
        let (mut anim, combat, blink_cam, mut clusters) = pick_inputs();
        clusters.body_mode.body_mode = BodyMode::Crouching;
        clusters.ground.on_ground = true;
        let _ = &mut anim;
        assert_eq!(
            pick(&anim, &combat, &blink_cam, None, &clusters),
            CharacterAnim::Crouch,
        );
    }

    /// `shoot_anim_timer > 0.0` picks the `Shoot` row, and the row
    /// wins over slash so a same-frame swing doesn't immediately stomp
    /// the muzzle-flash pose.
    #[test]
    fn shoot_anim_timer_returns_shoot() {
        let (mut anim, combat, blink_cam, clusters) = pick_inputs();
        anim.shoot_anim_timer = 0.10;
        anim.slash_anim_timer = 0.10;
        assert_eq!(
            pick(&anim, &combat, &blink_cam, None, &clusters),
            CharacterAnim::Shoot,
        );
    }

    /// `aim_anim_active` picks `Aim` only when no higher-priority
    /// state (shoot, slash, shield) is set.
    #[test]
    fn aim_anim_active_returns_aim() {
        let (mut anim, combat, blink_cam, clusters) = pick_inputs();
        anim.aim_anim_active = true;
        assert_eq!(
            pick(&anim, &combat, &blink_cam, None, &clusters),
            CharacterAnim::Aim,
        );
    }

    /// `wall_jump_anim_timer > 0.0` picks `WallJump` even while the
    /// player is airborne moving upward. Pre-poison the result with
    /// the default fall-through so a missed return trips this.
    #[test]
    fn wall_jump_anim_timer_returns_wall_jump_when_airborne() {
        let (mut anim, combat, blink_cam, mut clusters) = pick_inputs();
        anim.wall_jump_anim_timer = 0.15;
        clusters.ground.on_ground = false;
        clusters.kinematics.vel.y = -200.0;
        assert_eq!(
            pick(&anim, &combat, &blink_cam, None, &clusters),
            CharacterAnim::WallJump,
        );
    }

    /// `interact_anim_timer > 0.0` picks `Interact`. Set from
    /// NPC / switch / chest open paths; held briefly while the
    /// interaction commits.
    #[test]
    fn interact_anim_timer_returns_interact() {
        let (mut anim, combat, blink_cam, clusters) = pick_inputs();
        anim.interact_anim_timer = 0.20;
        assert_eq!(
            pick(&anim, &combat, &blink_cam, None, &clusters),
            CharacterAnim::Interact,
        );
    }

    /// `from_name` round-trips all the new row names so the
    /// spritesheet RON parser can resolve them without dropping rows
    /// silently.
    #[test]
    fn from_name_resolves_all_new_action_rows() {
        for (name, expected) in [
            ("crouch", CharacterAnim::Crouch),
            ("crouch_walk", CharacterAnim::Crawl),
            ("crawl", CharacterAnim::Crawl),
            ("slide", CharacterAnim::Slide),
            ("climb", CharacterAnim::LadderClimb),
            ("ladder_climb", CharacterAnim::LadderClimb),
            ("swim", CharacterAnim::Swim),
            ("shoot", CharacterAnim::Shoot),
            ("aim", CharacterAnim::Aim),
            ("charge", CharacterAnim::Charge),
            ("block", CharacterAnim::Block),
            ("shield", CharacterAnim::Block),
            ("roll", CharacterAnim::DodgeRoll),
            ("dodge_roll", CharacterAnim::DodgeRoll),
            ("wall_jump", CharacterAnim::WallJump),
            ("interact", CharacterAnim::Interact),
        ] {
            assert_eq!(
                CharacterAnim::from_name(name),
                Some(expected),
                "from_name({name:?}) should map to {expected:?}",
            );
        }
    }
}
