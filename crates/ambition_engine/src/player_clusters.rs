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

/// Mutable cluster references aggregated for the engine
/// `update_player_*_with_clusters` entry points.
///
/// Holding the 18 cluster refs in a struct keeps the entry-point
/// signatures from accumulating 20+ positional parameters and lets
/// sandbox callers build the view from a Bevy query without going
/// through a separate bridge module.
pub struct PlayerClustersMut<'a> {
    pub abilities: &'a PlayerAbilities,
    pub kinematics: &'a mut PlayerKinematics,
    pub ground: &'a mut PlayerGroundState,
    pub wall: &'a mut PlayerWallState,
    pub jump: &'a mut PlayerJumpState,
    pub dash: &'a mut PlayerDashState,
    pub flight: &'a mut PlayerFlightState,
    pub blink: &'a mut PlayerBlinkState,
    pub ledge: &'a mut PlayerLedgeState,
    pub dodge: &'a mut PlayerDodgeState,
    pub shield: &'a mut PlayerShieldState,
    pub body_mode: &'a mut PlayerBodyModeState,
    pub env_contact: &'a mut PlayerEnvironmentContact,
    pub mana: &'a mut PlayerMana,
    pub offense: &'a mut PlayerOffense,
    pub action_buffer: &'a mut PlayerActionBuffer,
    pub lifetime: &'a mut PlayerLifetime,
    pub combo_trace: &'a mut PlayerComboTrace,
}

impl<'a> PlayerClustersMut<'a> {
    /// Build an owned `Player` from the cluster refs. Used by the
    /// engine wrapper entry points that still call into the legacy
    /// `&mut Player` movement helpers (Phase 3 transitional shim;
    /// Phase 3d deletes both ae::Player and this helper).
    pub fn to_player(&self) -> Player {
        Player {
            abilities: self.abilities.abilities,
            pos: self.kinematics.pos,
            vel: self.kinematics.vel,
            size: self.kinematics.size,
            base_size: self.kinematics.base_size,
            facing: self.kinematics.facing,
            on_ground: self.ground.on_ground,
            on_wall: self.wall.on_wall,
            wall_normal_x: self.wall.wall_normal_x,
            dash_charges_available: self.dash.charges_available,
            air_jumps_available: self.jump.air_jumps_available,
            fly_enabled: self.flight.fly_enabled,
            flight_phase: self.flight.flight_phase,
            blink_cooldown: self.blink.cooldown,
            blink_hold_active: self.blink.hold_active,
            blink_hold_timer: self.blink.hold_timer,
            blink_aiming: self.blink.aiming,
            blink_aim_offset: self.blink.aim_offset,
            blink_grace_timer: self.blink.grace_timer,
            fast_falling: self.flight.fast_falling,
            gliding: self.flight.gliding,
            wall_clinging: self.wall.wall_clinging,
            wall_climbing: self.wall.wall_climbing,
            dash_timer: self.dash.timer,
            dash_cooldown: self.dash.cooldown,
            dash_buffer_timer: self.action_buffer.dash,
            jump_buffer_timer: self.action_buffer.jump,
            coyote_timer: self.ground.coyote_timer,
            rebound_cooldown: self.ground.rebound_cooldown,
            drop_through_timer: self.ground.drop_through_timer,
            combo: self.combo_trace.combo.clone(),
            max_speed: self.lifetime.max_speed,
            time_alive: self.lifetime.time_alive,
            resets: self.lifetime.resets,
            damage_multiplier: self.offense.damage_multiplier,
            mana: self.mana.meter,
            invincible: self.offense.invincible,
            body_mode: self.body_mode.body_mode,
            water_contact: self.env_contact.water,
            climbable_contact: self.env_contact.climbable,
            ledge_grab: self.ledge.grab,
            pre_wall_vel: self.wall.pre_wall_vel,
            pre_wall_vel_age: self.wall.pre_wall_vel_age,
            ledge_release_cooldown: self.ledge.release_cooldown,
            dodge_roll_timer: self.dodge.roll_timer,
            dodge_roll_cooldown: self.dodge.cooldown,
            shield_active: self.shield.active,
            parry_window_timer: self.shield.parry_window_timer,
        }
    }

    /// Assemble a tick-local `Player` from these cluster refs, hand
    /// it to the closure, and commit the result back. Use this from
    /// any sandbox system that still needs to call an engine helper
    /// taking `&mut Player`.
    ///
    /// Phase 3 transitional shim — Phase 3d removes both `ae::Player`
    /// and this helper once every engine helper is cluster-native.
    pub fn with_player_scratchpad<R, F: FnOnce(&mut Player) -> R>(&mut self, f: F) -> R {
        let mut player = self.to_player();
        let r = f(&mut player);
        self.write_from_player(player);
        r
    }

    /// Write the post-tick `Player` back to the cluster refs.
    pub fn write_from_player(&mut self, player: Player) {
        self.kinematics.pos = player.pos;
        self.kinematics.vel = player.vel;
        self.kinematics.size = player.size;
        self.kinematics.base_size = player.base_size;
        self.kinematics.facing = player.facing;

        self.ground.on_ground = player.on_ground;
        self.ground.coyote_timer = player.coyote_timer;
        self.ground.drop_through_timer = player.drop_through_timer;
        self.ground.rebound_cooldown = player.rebound_cooldown;

        self.wall.on_wall = player.on_wall;
        self.wall.wall_normal_x = player.wall_normal_x;
        self.wall.wall_clinging = player.wall_clinging;
        self.wall.wall_climbing = player.wall_climbing;
        self.wall.pre_wall_vel = player.pre_wall_vel;
        self.wall.pre_wall_vel_age = player.pre_wall_vel_age;

        self.jump.air_jumps_available = player.air_jumps_available;

        self.dash.charges_available = player.dash_charges_available;
        self.dash.timer = player.dash_timer;
        self.dash.cooldown = player.dash_cooldown;

        self.flight.fly_enabled = player.fly_enabled;
        self.flight.flight_phase = player.flight_phase;
        self.flight.gliding = player.gliding;
        self.flight.fast_falling = player.fast_falling;

        self.blink.cooldown = player.blink_cooldown;
        self.blink.hold_active = player.blink_hold_active;
        self.blink.hold_timer = player.blink_hold_timer;
        self.blink.aiming = player.blink_aiming;
        self.blink.aim_offset = player.blink_aim_offset;
        self.blink.grace_timer = player.blink_grace_timer;

        self.ledge.grab = player.ledge_grab;
        self.ledge.release_cooldown = player.ledge_release_cooldown;

        self.dodge.roll_timer = player.dodge_roll_timer;
        self.dodge.cooldown = player.dodge_roll_cooldown;

        self.shield.active = player.shield_active;
        self.shield.parry_window_timer = player.parry_window_timer;

        self.body_mode.body_mode = player.body_mode;

        self.env_contact.water = player.water_contact;
        self.env_contact.climbable = player.climbable_contact;

        self.mana.meter = player.mana;

        self.offense.damage_multiplier = player.damage_multiplier;
        self.offense.invincible = player.invincible;

        self.action_buffer.jump = player.jump_buffer_timer;
        self.action_buffer.dash = player.dash_buffer_timer;

        self.lifetime.time_alive = player.time_alive;
        self.lifetime.resets = player.resets;
        self.lifetime.max_speed = player.max_speed;

        self.combo_trace.combo = player.combo;
    }
}

/// Bevy query data that matches [`PlayerClustersMut`]. Use in a system
/// signature as `Query<PlayerClusterQueryData, ...>` and call
/// [`PlayerClusterQueryDataItem::as_clusters_mut`] to borrow the view.
#[derive(bevy_ecs::query::QueryData)]
#[query_data(mutable)]
pub struct PlayerClusterQueryData {
    pub abilities: &'static PlayerAbilities,
    pub kinematics: &'static mut PlayerKinematics,
    pub ground: &'static mut PlayerGroundState,
    pub wall: &'static mut PlayerWallState,
    pub jump: &'static mut PlayerJumpState,
    pub dash: &'static mut PlayerDashState,
    pub flight: &'static mut PlayerFlightState,
    pub blink: &'static mut PlayerBlinkState,
    pub ledge: &'static mut PlayerLedgeState,
    pub dodge: &'static mut PlayerDodgeState,
    pub shield: &'static mut PlayerShieldState,
    pub body_mode: &'static mut PlayerBodyModeState,
    pub env_contact: &'static mut PlayerEnvironmentContact,
    pub mana: &'static mut PlayerMana,
    pub offense: &'static mut PlayerOffense,
    pub action_buffer: &'static mut PlayerActionBuffer,
    pub lifetime: &'static mut PlayerLifetime,
    pub combo_trace: &'static mut PlayerComboTrace,
}

impl<'w, 's> PlayerClusterQueryDataItem<'w, 's> {
    /// Borrow the query item as a [`PlayerClustersMut`].
    pub fn as_clusters_mut<'a>(&'a mut self) -> PlayerClustersMut<'a>
    where
        'w: 'a,
        's: 'a,
    {
        PlayerClustersMut {
            abilities: &*self.abilities,
            kinematics: &mut *self.kinematics,
            ground: &mut *self.ground,
            wall: &mut *self.wall,
            jump: &mut *self.jump,
            dash: &mut *self.dash,
            flight: &mut *self.flight,
            blink: &mut *self.blink,
            ledge: &mut *self.ledge,
            dodge: &mut *self.dodge,
            shield: &mut *self.shield,
            body_mode: &mut *self.body_mode,
            env_contact: &mut *self.env_contact,
            mana: &mut *self.mana,
            offense: &mut *self.offense,
            action_buffer: &mut *self.action_buffer,
            lifetime: &mut *self.lifetime,
            combo_trace: &mut *self.combo_trace,
        }
    }
}

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
