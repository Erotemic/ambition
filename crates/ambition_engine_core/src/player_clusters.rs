//! Authoritative player cluster types.
//!
//! Each Bevy `Component` carries one tightly related slice of player state
//! (kinematics, ground contact, dash timers, …). Together they form the
//! authoritative player aggregate.
//!
//! [`PlayerClustersMut`] is a struct-of-`&mut` view assembled from a
//! `Query<PlayerClusterQueryData, …>::as_clusters_mut()` call; every
//! engine entry point in `crate::movement` takes one.
//! Tests that need a non-ECS scratchpad construct
//! [`PlayerClusterScratch::new_with_abilities`] and re-borrow via
//! `PlayerClusterScratch::as_mut`.

use crate::abilities::AbilitySet;
use crate::ledge_grab::LedgeGrabState;
use crate::movement::{ComboMark, BLINK_DISTANCE};
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
    pub kinematics: &'a mut BodyKinematics,
    pub base_size: &'a mut PlayerBaseSize,
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

/// Bevy query data that matches [`PlayerClustersMut`]. Use in a system
/// signature as `Query<PlayerClusterQueryData, ...>` and call
/// [`PlayerClusterQueryDataItem::as_clusters_mut`] to borrow the view.
#[derive(bevy_ecs::query::QueryData)]
#[query_data(mutable)]
pub struct PlayerClusterQueryData {
    pub abilities: &'static PlayerAbilities,
    pub kinematics: &'static mut BodyKinematics,
    pub base_size: &'static mut PlayerBaseSize,
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
            base_size: &mut *self.base_size,
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
}

/// Position, velocity, AABB size, and facing direction of a body.
///
/// The foundational body state every controllable body in the platformer
/// shares: the player, enemies/NPCs, and bosses all carry one. It replaces
/// the three historical parallel types (`PlayerKinematics`,
/// `ActorKinematics`, `BossKinematics`) so any code that operates on "a body"
/// (orientation, transit, vortex, brain effects, …) holds ONE query instead
/// of branching across three.
///
/// The player shares this unified component with enemies / NPCs / bosses. The
/// player-only "base / standing body size" lives separately on
/// [`PlayerBaseSize`] so the shared component stays minimal.
///
/// ## Query-conflict discipline
///
/// Because player, enemy, and boss entities all carry `BodyKinematics`, any
/// single system that holds more than one `&mut BodyKinematics` query (or a
/// `&mut` query alongside another that can alias the same entity) must make the
/// queries provably disjoint with marker filters (player / enemy / boss are
/// mutually exclusive archetypes). Handle the conflict with filters, never by
/// re-splitting the component.
///
/// Bosses float and never integrate `vel` themselves (the brain emits a fresh
/// `desired_vel` each tick for `integrate_body`), so a boss simply leaves
/// `vel` at [`Vec2::ZERO`].
#[derive(bevy_ecs::component::Component, Clone, Copy, Debug, PartialEq)]
pub struct BodyKinematics {
    pub pos: Vec2,
    pub vel: Vec2,
    pub size: Vec2,
    pub facing: f32,
}

impl Default for BodyKinematics {
    /// Player-flavored default (the only `::default()` callers are player
    /// spawn helpers): a default-sized body at the origin, at rest, facing
    /// right. Matches the pre-unification `PlayerKinematics::default`.
    fn default() -> Self {
        let body = crate::movement::default_player_body_size();
        Self {
            pos: Vec2::ZERO,
            vel: Vec2::ZERO,
            size: body,
            facing: 1.0,
        }
    }
}

impl BodyKinematics {
    /// The body's world-space AABB (centered on `pos`, half-extents `size/2`).
    pub fn aabb(self) -> crate::Aabb {
        crate::Aabb::new(self.pos, self.size * 0.5)
    }

    /// The body's AABB ORIENTED to its gravity/acceleration frame: width<->height
    /// swap under sideways gravity (the body lies along the wall), so the collision
    /// footprint matches the gravity-rotated sprite. Identity under down/up gravity,
    /// so vertical-gravity play is byte-identical to [`Self::aabb`].
    pub fn aabb_oriented(self, gravity_dir: crate::Vec2) -> crate::Aabb {
        let half = crate::AccelerationFrame::new(gravity_dir).to_world_half(self.size * 0.5);
        crate::Aabb::new(self.pos, half)
    }
}

/// The player's authored *standing* body size — the baseline the morph /
/// crouch / slide stances and the sprite-scale math read from. Player-only;
/// it is deliberately NOT part of the shared [`BodyKinematics`] (enemies and
/// bosses have no stance-baseline concept), so it rides in its own component
/// alongside the rest of the player clusters.
#[derive(bevy_ecs::component::Component, Clone, Copy, Debug, PartialEq)]
pub struct PlayerBaseSize {
    pub base_size: Vec2,
}

impl Default for PlayerBaseSize {
    fn default() -> Self {
        Self {
            base_size: crate::movement::default_player_body_size(),
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
/// [`PlayerActionBuffer`]; this component owns the air-jump charge
/// count plus the transient ladder-jump boost / ladder drop-through
/// timers today.
#[derive(bevy_ecs::component::Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct PlayerJumpState {
    pub air_jumps_available: u8,
    pub ladder_jump_boost: f32,
    pub ladder_drop_through_timer: f32,
    pub ladder_drop_through_hold_lock: bool,
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

/// Re-grab lockout applied when a hit knocks the player off a ledge, so they
/// fall with the knockback instead of instantly re-latching.
pub const LEDGE_KNOCK_OFF_COOLDOWN: f32 = 0.35;

impl PlayerLedgeState {
    /// Drop any active ledge grab because the player was hit, arming a brief
    /// re-grab lockout. Returns true if the player was actually hanging (so the
    /// caller can react). A no-op when not grabbing.
    pub fn knock_off_on_hit(&mut self) -> bool {
        if self.grab.take().is_some() {
            self.release_cooldown = self.release_cooldown.max(LEDGE_KNOCK_OFF_COOLDOWN);
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod ledge_knock_off_tests {
    use super::*;
    use crate::ledge_grab::LedgeContact;

    fn hanging() -> LedgeGrabState {
        LedgeGrabState::hanging(LedgeContact {
            wall_normal_x: 1.0,
            anchor: Vec2::ZERO,
            climb_target: Vec2::ZERO,
        })
    }

    #[test]
    fn getting_hit_knocks_the_player_off_a_ledge_grab() {
        let mut ledge = PlayerLedgeState {
            grab: Some(hanging()),
            release_cooldown: 0.0,
        };
        assert!(
            ledge.knock_off_on_hit(),
            "was hanging → reports knocked off"
        );
        assert!(
            ledge.grab.is_none(),
            "ledge grab cleared so the player falls"
        );
        assert!(
            ledge.release_cooldown >= LEDGE_KNOCK_OFF_COOLDOWN,
            "re-grab lockout armed"
        );
    }

    #[test]
    fn knock_off_is_a_noop_when_not_grabbing() {
        let mut ledge = PlayerLedgeState::default();
        assert!(!ledge.knock_off_on_hit());
        assert!(ledge.grab.is_none());
        assert_eq!(
            ledge.release_cooldown, 0.0,
            "no lockout when nothing to drop"
        );
    }
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

impl PlayerShieldState {
    pub fn parrying(self) -> bool {
        self.active && self.parry_window_timer > 0.0
    }
}

/// Reset a live player back to spawn while preserving the
/// `PlayerAbilities` and incrementing the lifetime reset counter. The
/// combo trace is wiped and a fresh `MovementOp::Reset` mark is pushed.
pub fn reset_player_clusters(clusters: &mut PlayerClustersMut<'_>, spawn: Vec2) {
    use crate::movement::{default_player_body_size, ComboMark, MovementOp, DEFAULT_TUNING};

    let new_resets = clusters.lifetime.resets + 1;
    let abilities = clusters.abilities.abilities;
    let body = default_player_body_size();
    let dash_charges = abilities.dash_charge_count();
    let air_jumps = abilities.air_jump_count(DEFAULT_TUNING.air_jumps);

    *clusters.kinematics = BodyKinematics {
        pos: spawn,
        vel: Vec2::ZERO,
        size: body,
        facing: 1.0,
    };
    *clusters.base_size = PlayerBaseSize { base_size: body };
    *clusters.ground = PlayerGroundState::default();
    *clusters.wall = PlayerWallState::default();
    *clusters.jump = PlayerJumpState {
        air_jumps_available: air_jumps,
        ladder_jump_boost: 0.0,
        ladder_drop_through_timer: 0.0,
        ladder_drop_through_hold_lock: false,
    };
    *clusters.dash = PlayerDashState {
        charges_available: dash_charges,
        ..Default::default()
    };
    *clusters.flight = PlayerFlightState::default();
    *clusters.blink = PlayerBlinkState::default();
    *clusters.ledge = PlayerLedgeState::default();
    *clusters.dodge = PlayerDodgeState::default();
    *clusters.shield = PlayerShieldState::default();
    *clusters.body_mode = PlayerBodyModeState::default();
    *clusters.env_contact = PlayerEnvironmentContact::default();
    *clusters.mana = PlayerMana::default();
    *clusters.offense = PlayerOffense::default();
    *clusters.action_buffer = PlayerActionBuffer::default();
    *clusters.lifetime = PlayerLifetime {
        resets: new_resets,
        ..Default::default()
    };
    clusters.combo_trace.combo.clear();
    clusters.combo_trace.combo.push(ComboMark {
        op: MovementOp::Reset,
        age: 0.0,
    });
}

/// Refresh the dash charge count and air-jump count from the active
/// `PlayerAbilities` + the caller's tuning.
pub fn refresh_movement_resources_clusters(
    abilities: &PlayerAbilities,
    dash: &mut PlayerDashState,
    jump: &mut PlayerJumpState,
    tuning: crate::movement::MovementTuning,
) {
    dash.charges_available = abilities.abilities.dash_charge_count();
    jump.air_jumps_available = abilities.abilities.air_jump_count(tuning.air_jumps);
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

/// Owned bag of all 18 player cluster components, used by unit tests
/// and the non-ECS call sites that need to assemble a
/// `PlayerClustersMut` without a Bevy entity. Construct via
/// [`PlayerClusterScratch::new_with_abilities`] and re-borrow as a view
/// via [`PlayerClusterScratch::as_mut`].
#[derive(Clone, Debug)]
pub struct PlayerClusterScratch {
    pub abilities: PlayerAbilities,
    pub kinematics: BodyKinematics,
    pub base_size: PlayerBaseSize,
    pub ground: PlayerGroundState,
    pub wall: PlayerWallState,
    pub jump: PlayerJumpState,
    pub dash: PlayerDashState,
    pub flight: PlayerFlightState,
    pub blink: PlayerBlinkState,
    pub ledge: PlayerLedgeState,
    pub dodge: PlayerDodgeState,
    pub shield: PlayerShieldState,
    pub body_mode: PlayerBodyModeState,
    pub env_contact: PlayerEnvironmentContact,
    pub mana: PlayerMana,
    pub offense: PlayerOffense,
    pub action_buffer: PlayerActionBuffer,
    pub lifetime: PlayerLifetime,
    pub combo_trace: PlayerComboTrace,
}

impl PlayerClusterScratch {
    /// Build a `PlayerClusterScratch` for a fresh player at `spawn`
    /// with the given `AbilitySet` — same defaults as
    /// `Player::new_with_abilities` but without materializing the
    /// monolithic `Player` aggregate.
    pub fn new_with_abilities(spawn: Vec2, abilities: crate::abilities::AbilitySet) -> Self {
        use crate::movement::{default_player_body_size, BLINK_DISTANCE, DEFAULT_TUNING};
        let body = default_player_body_size();
        let dash_charges = abilities.dash_charge_count();
        let air_jumps = abilities.air_jump_count(DEFAULT_TUNING.air_jumps);
        Self {
            abilities: PlayerAbilities { abilities },
            kinematics: BodyKinematics {
                pos: spawn,
                vel: Vec2::ZERO,
                size: body,
                facing: 1.0,
            },
            base_size: PlayerBaseSize { base_size: body },
            ground: PlayerGroundState::default(),
            wall: PlayerWallState::default(),
            jump: PlayerJumpState {
                air_jumps_available: air_jumps,
                ladder_jump_boost: 0.0,
                ladder_drop_through_timer: 0.0,
                ladder_drop_through_hold_lock: false,
            },
            dash: PlayerDashState {
                charges_available: dash_charges,
                timer: 0.0,
                cooldown: 0.0,
            },
            flight: PlayerFlightState::default(),
            blink: PlayerBlinkState {
                cooldown: 0.0,
                hold_active: false,
                hold_timer: 0.0,
                aiming: false,
                aim_offset: Vec2::new(BLINK_DISTANCE, 0.0),
                grace_timer: 0.0,
            },
            ledge: PlayerLedgeState::default(),
            dodge: PlayerDodgeState::default(),
            shield: PlayerShieldState::default(),
            body_mode: PlayerBodyModeState::default(),
            env_contact: PlayerEnvironmentContact::default(),
            mana: PlayerMana {
                meter: ResourceMeter::new(100.0, 0.0, 0.0),
            },
            offense: PlayerOffense {
                damage_multiplier: 1,
                invincible: false,
            },
            action_buffer: PlayerActionBuffer::default(),
            lifetime: PlayerLifetime::default(),
            combo_trace: PlayerComboTrace::default(),
        }
    }

    pub fn as_mut(&mut self) -> PlayerClustersMut<'_> {
        PlayerClustersMut {
            abilities: &self.abilities,
            kinematics: &mut self.kinematics,
            base_size: &mut self.base_size,
            ground: &mut self.ground,
            wall: &mut self.wall,
            jump: &mut self.jump,
            dash: &mut self.dash,
            flight: &mut self.flight,
            blink: &mut self.blink,
            ledge: &mut self.ledge,
            dodge: &mut self.dodge,
            shield: &mut self.shield,
            body_mode: &mut self.body_mode,
            env_contact: &mut self.env_contact,
            mana: &mut self.mana,
            offense: &mut self.offense,
            action_buffer: &mut self.action_buffer,
            lifetime: &mut self.lifetime,
            combo_trace: &mut self.combo_trace,
        }
    }
}
