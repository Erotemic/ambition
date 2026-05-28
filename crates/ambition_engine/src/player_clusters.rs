//! Authoritative player cluster types.
//!
//! Phase 3 of the player-ecs-bandaid plan moves the cluster fields out
//! of the monolithic `ae::Player` aggregate and into individual
//! cluster structs. These structs are plain value types (no Bevy
//! `Component` derive); the sandbox newtype-wraps each one with
//! `#[derive(Component, Deref, DerefMut)]` for ECS storage.
//!
//! Engine movement helpers (Phase 3b) consume these cluster types
//! directly instead of taking `&mut Player`. The
//! [`super::movement::Player`] aggregate continues to exist for
//! cross-tick scratchpad usage and the few engine tests that still
//! construct a whole player; it will go away in Phase 3d once nothing
//! references it.

use crate::abilities::AbilitySet;
use crate::ledge_grab::LedgeGrabState;
use crate::movement::{ComboMark, Player, BLINK_DISTANCE};
use crate::player_state::{BodyMode, ResourceMeter};
use crate::world::{ClimbableContact, WaterContact};
use crate::Vec2;

/// Active ability set for this player.
#[derive(bevy_ecs::component::Component, Clone, Copy, Debug, Default)]
pub struct PlayerAbilities {
    pub abilities: AbilitySet,
}

impl PlayerAbilities {
    pub fn new(abilities: AbilitySet) -> Self {
        Self { abilities }
    }

    pub fn from_player(player: &Player) -> Self {
        Self::new(player.abilities)
    }
}

/// Position, velocity, AABB size, and facing direction.
#[derive(bevy_ecs::component::Component, Clone, Copy, Debug, PartialEq)]
pub struct PlayerKinematics {
    pub pos: Vec2,
    pub vel: Vec2,
    pub size: Vec2,
    pub base_size: Vec2,
    pub facing: f32,
}

impl Default for PlayerKinematics {
    fn default() -> Self {
        let body = crate::movement::default_player_body_size();
        Self {
            pos: Vec2::ZERO,
            vel: Vec2::ZERO,
            size: body,
            base_size: body,
            facing: 1.0,
        }
    }
}

impl PlayerKinematics {
    pub fn from_player(player: &Player) -> Self {
        Self {
            pos: player.pos,
            vel: player.vel,
            size: player.size,
            base_size: player.base_size,
            facing: player.facing,
        }
    }

    pub fn aabb(self) -> crate::Aabb {
        crate::Aabb::new(self.pos, self.size * 0.5)
    }
}

impl PlayerGroundState {
    pub fn from_player(player: &Player) -> Self {
        Self {
            on_ground: player.on_ground,
            coyote_timer: player.coyote_timer,
            drop_through_timer: player.drop_through_timer,
            rebound_cooldown: player.rebound_cooldown,
        }
    }
}

impl PlayerWallState {
    pub fn from_player(player: &Player) -> Self {
        Self {
            on_wall: player.on_wall,
            wall_normal_x: player.wall_normal_x,
            wall_clinging: player.wall_clinging,
            wall_climbing: player.wall_climbing,
            pre_wall_vel: player.pre_wall_vel,
            pre_wall_vel_age: player.pre_wall_vel_age,
        }
    }
}

impl PlayerJumpState {
    pub fn from_player(player: &Player) -> Self {
        Self {
            air_jumps_available: player.air_jumps_available,
        }
    }
}

impl PlayerDashState {
    pub fn from_player(player: &Player) -> Self {
        Self {
            charges_available: player.dash_charges_available,
            timer: player.dash_timer,
            cooldown: player.dash_cooldown,
        }
    }
}

impl PlayerFlightState {
    pub fn from_player(player: &Player) -> Self {
        Self {
            fly_enabled: player.fly_enabled,
            flight_phase: player.flight_phase,
            gliding: player.gliding,
            fast_falling: player.fast_falling,
        }
    }
}

/// Ground contact + airborne grace timers (coyote, drop-through,
/// pogo rebound).
#[derive(bevy_ecs::component::Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct PlayerGroundState {
    pub on_ground: bool,
    pub coyote_timer: f32,
    pub drop_through_timer: f32,
    pub rebound_cooldown: f32,
}

/// Wall contact + wall-cling / wall-climb state and the pre-wall
/// momentum window the ledge-grab boost reads.
#[derive(bevy_ecs::component::Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct PlayerWallState {
    pub on_wall: bool,
    pub wall_normal_x: f32,
    pub wall_clinging: bool,
    pub wall_climbing: bool,
    pub pre_wall_vel: Vec2,
    pub pre_wall_vel_age: f32,
}

/// Jump-cluster state. The jump buffer itself lives on
/// [`PlayerActionBuffer`]; this component owns only the air-jump
/// charge count today.
#[derive(bevy_ecs::component::Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlayerJumpState {
    pub air_jumps_available: u8,
}

/// Dash-cluster state. The dash buffer lives on [`PlayerActionBuffer`];
/// this owns the charge count, the active-dash countdown, and the
/// cooldown.
#[derive(bevy_ecs::component::Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct PlayerDashState {
    pub charges_available: u8,
    pub timer: f32,
    pub cooldown: f32,
}

/// Free-flight, glide, and fast-fall flags + the idle hover-bob phase.
#[derive(bevy_ecs::component::Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct PlayerFlightState {
    pub fly_enabled: bool,
    pub flight_phase: f32,
    pub gliding: bool,
    pub fast_falling: bool,
}

/// Blink cluster: cooldown, hold-to-aim state, precision aim offset,
/// and the post-blink grace timer.
#[derive(bevy_ecs::component::Component, Clone, Copy, Debug, PartialEq)]
pub struct PlayerBlinkState {
    pub cooldown: f32,
    pub hold_active: bool,
    pub hold_timer: f32,
    pub aiming: bool,
    pub aim_offset: Vec2,
    pub grace_timer: f32,
}

impl Default for PlayerBlinkState {
    fn default() -> Self {
        Self {
            cooldown: 0.0,
            hold_active: false,
            hold_timer: 0.0,
            aiming: false,
            aim_offset: Vec2::new(BLINK_DISTANCE, 0.0),
            grace_timer: 0.0,
        }
    }
}

/// Engine-owned ledge hang / pull-up state + the re-grab cooldown.
#[derive(bevy_ecs::component::Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct PlayerLedgeState {
    pub grab: Option<LedgeGrabState>,
    pub release_cooldown: f32,
}

/// Dodge-roll i-frame timer + cooldown.
#[derive(bevy_ecs::component::Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct PlayerDodgeState {
    pub roll_timer: f32,
    pub cooldown: f32,
}

/// Shield/parry cluster.
#[derive(bevy_ecs::component::Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct PlayerShieldState {
    pub active: bool,
    pub parry_window_timer: f32,
}

impl PlayerBlinkState {
    pub fn from_player(player: &Player) -> Self {
        Self {
            cooldown: player.blink_cooldown,
            hold_active: player.blink_hold_active,
            hold_timer: player.blink_hold_timer,
            aiming: player.blink_aiming,
            aim_offset: player.blink_aim_offset,
            grace_timer: player.blink_grace_timer,
        }
    }
}

impl PlayerLedgeState {
    pub fn from_player(player: &Player) -> Self {
        Self {
            grab: player.ledge_grab,
            release_cooldown: player.ledge_release_cooldown,
        }
    }
}

impl PlayerDodgeState {
    pub fn from_player(player: &Player) -> Self {
        Self {
            roll_timer: player.dodge_roll_timer,
            cooldown: player.dodge_roll_cooldown,
        }
    }
}

impl PlayerShieldState {
    pub fn from_player(player: &Player) -> Self {
        Self {
            active: player.shield_active,
            parry_window_timer: player.parry_window_timer,
        }
    }

    pub fn parrying(self) -> bool {
        self.active && self.parry_window_timer > 0.0
    }
}

impl PlayerBodyModeState {
    pub fn from_player(player: &Player) -> Self {
        Self {
            body_mode: player.body_mode,
        }
    }
}

impl PlayerEnvironmentContact {
    pub fn from_player(player: &Player) -> Self {
        Self {
            water: player.water_contact,
            climbable: player.climbable_contact,
        }
    }
}

impl PlayerMana {
    pub fn from_player(player: &Player) -> Self {
        Self { meter: player.mana }
    }
}

impl PlayerOffense {
    pub fn from_player(player: &Player) -> Self {
        Self {
            damage_multiplier: player.damage_multiplier,
            invincible: player.invincible,
        }
    }
}

impl PlayerLifetime {
    pub fn from_player(player: &Player) -> Self {
        Self {
            time_alive: player.time_alive,
            resets: player.resets,
            max_speed: player.max_speed,
        }
    }
}

impl PlayerComboTrace {
    pub fn from_player(player: &Player) -> Self {
        Self {
            combo: player.combo.clone(),
        }
    }
}

/// Authoritative body-shape stance.
#[derive(bevy_ecs::component::Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlayerBodyModeState {
    pub body_mode: BodyMode,
}

/// Per-frame world-contact cluster: water + climbable region overlap.
#[derive(bevy_ecs::component::Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct PlayerEnvironmentContact {
    pub water: Option<WaterContact>,
    pub climbable: Option<ClimbableContact>,
}

/// Generic spendable meter the player draws on for charge attacks /
/// special abilities.
#[derive(bevy_ecs::component::Component, Clone, Copy, Debug, PartialEq)]
pub struct PlayerMana {
    pub meter: ResourceMeter,
}

impl Default for PlayerMana {
    fn default() -> Self {
        Self {
            meter: ResourceMeter::new(100.0, 0.0, 0.0),
        }
    }
}

/// Offensive scaling knobs.
#[derive(bevy_ecs::component::Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlayerOffense {
    pub damage_multiplier: i32,
    pub invincible: bool,
}

impl Default for PlayerOffense {
    fn default() -> Self {
        Self {
            damage_multiplier: 1,
            invincible: false,
        }
    }
}

/// Generic ECS-owned action buffer.
#[derive(bevy_ecs::component::Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct PlayerActionBuffer {
    pub jump: f32,
    pub dash: f32,
    pub attack: f32,
    pub pogo: f32,
    pub projectile: f32,
    pub blink: f32,
}

impl PlayerActionBuffer {
    pub fn from_player(player: &Player) -> Self {
        Self {
            jump: player.jump_buffer_timer,
            dash: player.dash_buffer_timer,
            attack: 0.0,
            pogo: 0.0,
            projectile: 0.0,
            blink: 0.0,
        }
    }

    pub fn press_jump(&mut self, window: f32) {
        self.jump = window;
    }

    pub fn press_dash(&mut self, window: f32) {
        self.dash = window;
    }

    pub fn consume_jump(&mut self) -> bool {
        if self.jump > 0.0 {
            self.jump = 0.0;
            true
        } else {
            false
        }
    }

    pub fn consume_dash(&mut self) -> bool {
        if self.dash > 0.0 {
            self.dash = 0.0;
            true
        } else {
            false
        }
    }

    pub fn tick(&mut self, dt: f32) {
        for slot in [
            &mut self.jump,
            &mut self.dash,
            &mut self.attack,
            &mut self.pogo,
            &mut self.projectile,
            &mut self.blink,
        ] {
            *slot = (*slot - dt).max(0.0);
        }
    }
}

/// Lifetime + diagnostic counters.
#[derive(bevy_ecs::component::Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct PlayerLifetime {
    pub time_alive: f32,
    pub resets: u32,
    pub max_speed: f32,
}

/// Symbolic operation trace ("J o D o D"), preserved across the
/// engine-Player tick scratchpad so the HUD combo readout doesn't
/// blank every frame.
#[derive(bevy_ecs::component::Component, Clone, Debug, Default)]
pub struct PlayerComboTrace {
    pub combo: Vec<ComboMark>,
}

impl PlayerComboTrace {
    pub fn symbols(&self) -> String {
        if self.combo.is_empty() {
            return "-".to_string();
        }
        self.combo
            .iter()
            .map(|m| m.op.symbol())
            .collect::<Vec<_>>()
            .join(" o ")
    }
}
