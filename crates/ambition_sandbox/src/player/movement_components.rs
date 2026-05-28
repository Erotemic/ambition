//! Authoritative ECS movement-state components for the player entity.
//!
//! These are the Phase 1 shape from
//! [`docs/planning/player-ecs-bandaid-phase0.md`]:
//! every field on `ae::Player` has a destination here. They are spawned
//! via [`super::bundles::PlayerSimulationBundle`] and initialized from
//! the spawning engine `ae::Player`.
//!
//! Phase 1 (this commit) only adds the components. The existing
//! [`super::components::PlayerMovementAuthority`] still owns the
//! frame-to-frame truth; the clusters here are unsynced shadow state
//! that Phase 2 will flip readers/writers onto and Phase 3 will rebuild
//! the movement logic against. We deliberately do NOT mirror them each
//! frame — that would be transition code we're about to delete.
//!
//! Crate-boundary note: each cluster has a `from_player(&ae::Player)`
//! constructor for spawn-time init and a `Default` impl matching the
//! "fresh player" shape (`Player::new_with_abilities`). After Phase 3
//! cuts the authority, `from_player` and the engine `Player` struct
//! both go away in the same commit.

use ambition_engine as ae;
use bevy::prelude::*;

/// Active ability set for this player.
///
/// Spawned from `Player::abilities`. After Phase 3 this becomes the
/// authoritative source; the existing `ActionSet` is derived from it.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct PlayerAbilities {
    pub abilities: ae::AbilitySet,
}

impl PlayerAbilities {
    pub fn new(abilities: ae::AbilitySet) -> Self {
        Self { abilities }
    }

    pub fn from_player(player: &ae::Player) -> Self {
        Self::new(player.abilities)
    }
}

/// Position, velocity, AABB size, and facing direction.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct PlayerKinematics {
    pub pos: ae::Vec2,
    pub vel: ae::Vec2,
    pub size: ae::Vec2,
    pub base_size: ae::Vec2,
    pub facing: f32,
}

impl Default for PlayerKinematics {
    fn default() -> Self {
        let body = ae::default_player_body_size();
        Self {
            pos: ae::Vec2::ZERO,
            vel: ae::Vec2::ZERO,
            size: body,
            base_size: body,
            facing: 1.0,
        }
    }
}

impl PlayerKinematics {
    pub fn from_player(player: &ae::Player) -> Self {
        Self {
            pos: player.pos,
            vel: player.vel,
            size: player.size,
            base_size: player.base_size,
            facing: player.facing,
        }
    }

    pub fn aabb(self) -> ae::Aabb {
        ae::Aabb::new(self.pos, self.size * 0.5)
    }
}

/// Ground contact + airborne grace timers (coyote, drop-through,
/// pogo rebound). All timers count down per frame.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct PlayerGroundState {
    pub on_ground: bool,
    pub coyote_timer: f32,
    pub drop_through_timer: f32,
    pub rebound_cooldown: f32,
}

impl PlayerGroundState {
    pub fn from_player(player: &ae::Player) -> Self {
        Self {
            on_ground: player.on_ground,
            coyote_timer: player.coyote_timer,
            drop_through_timer: player.drop_through_timer,
            rebound_cooldown: player.rebound_cooldown,
        }
    }
}

/// Wall contact + wall-cling / wall-climb state and the pre-wall
/// momentum window the ledge-grab boost reads.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct PlayerWallState {
    pub on_wall: bool,
    pub wall_normal_x: f32,
    pub wall_clinging: bool,
    pub wall_climbing: bool,
    pub pre_wall_vel: ae::Vec2,
    pub pre_wall_vel_age: f32,
}

impl PlayerWallState {
    pub fn from_player(player: &ae::Player) -> Self {
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

/// Jump-cluster state. The jump buffer itself lives on
/// [`PlayerActionBuffer`]; this component owns only the air-jump
/// charge count today. Phase 3 will add apex-hang / jump-sustain
/// fields here.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlayerJumpState {
    pub air_jumps_available: u8,
}

impl PlayerJumpState {
    pub fn from_player(player: &ae::Player) -> Self {
        Self {
            air_jumps_available: player.air_jumps_available,
        }
    }
}

/// Dash-cluster state. The dash buffer lives on [`PlayerActionBuffer`];
/// this component owns the charge count, the active-dash countdown,
/// and the cooldown that gates the next dash start.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct PlayerDashState {
    pub charges_available: u8,
    /// `> 0` while a dash is mid-execution.
    pub timer: f32,
    /// Counts down before a new dash may begin.
    pub cooldown: f32,
}

impl PlayerDashState {
    pub fn from_player(player: &ae::Player) -> Self {
        Self {
            charges_available: player.dash_charges_available,
            timer: player.dash_timer,
            cooldown: player.dash_cooldown,
        }
    }
}

/// Free-flight, glide, and fast-fall flags + the idle hover-bob phase.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct PlayerFlightState {
    pub fly_enabled: bool,
    pub flight_phase: f32,
    pub gliding: bool,
    pub fast_falling: bool,
}

impl PlayerFlightState {
    pub fn from_player(player: &ae::Player) -> Self {
        Self {
            fly_enabled: player.fly_enabled,
            flight_phase: player.flight_phase,
            gliding: player.gliding,
            fast_falling: player.fast_falling,
        }
    }
}

/// Blink cluster: cooldown, hold-to-aim state, precision aim offset,
/// and the post-blink grace timer.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct PlayerBlinkState {
    pub cooldown: f32,
    pub hold_active: bool,
    pub hold_timer: f32,
    pub aiming: bool,
    pub aim_offset: ae::Vec2,
    pub grace_timer: f32,
}

impl Default for PlayerBlinkState {
    fn default() -> Self {
        Self {
            cooldown: 0.0,
            hold_active: false,
            hold_timer: 0.0,
            aiming: false,
            aim_offset: ae::Vec2::new(ae::BLINK_DISTANCE, 0.0),
            grace_timer: 0.0,
        }
    }
}

impl PlayerBlinkState {
    pub fn from_player(player: &ae::Player) -> Self {
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

/// Engine-owned ledge hang / pull-up state + the re-grab cooldown.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct PlayerLedgeState {
    pub grab: Option<ae::LedgeGrabState>,
    pub release_cooldown: f32,
}

impl PlayerLedgeState {
    pub fn from_player(player: &ae::Player) -> Self {
        Self {
            grab: player.ledge_grab,
            release_cooldown: player.ledge_release_cooldown,
        }
    }
}

/// Dodge-roll i-frame timer + cooldown.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct PlayerDodgeState {
    pub roll_timer: f32,
    pub cooldown: f32,
}

impl PlayerDodgeState {
    pub fn from_player(player: &ae::Player) -> Self {
        Self {
            roll_timer: player.dodge_roll_timer,
            cooldown: player.dodge_roll_cooldown,
        }
    }
}

/// Shield/parry cluster: `active` while the shield button is held,
/// `parry_window_timer` counts down from `MovementTuning::parry_window_time`
/// each time the shield is reactivated.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct PlayerShieldState {
    pub active: bool,
    pub parry_window_timer: f32,
}

impl PlayerShieldState {
    pub fn from_player(player: &ae::Player) -> Self {
        Self {
            active: player.shield_active,
            parry_window_timer: player.parry_window_timer,
        }
    }

    /// True iff the shield is up AND inside the parry window. Damage
    /// gating reads this.
    pub fn parrying(self) -> bool {
        self.active && self.parry_window_timer > 0.0
    }
}

/// Authoritative body-shape stance (`Standing` / `Crouching` /
/// `MorphBall` / `Climbing` / …).
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlayerBodyModeState {
    pub body_mode: ae::BodyMode,
}

impl PlayerBodyModeState {
    pub fn from_player(player: &ae::Player) -> Self {
        Self {
            body_mode: player.body_mode,
        }
    }
}

/// Per-frame world-contact cluster: water + climbable region overlap.
/// Movement integration reads these to gate swim physics and ladder
/// motion.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct PlayerEnvironmentContact {
    pub water: Option<ae::WaterContact>,
    pub climbable: Option<ae::ClimbableContact>,
}

impl PlayerEnvironmentContact {
    pub fn from_player(player: &ae::Player) -> Self {
        Self {
            water: player.water_contact,
            climbable: player.climbable_contact,
        }
    }
}

/// Generic spendable meter the player draws on for charge attacks /
/// special abilities. Backs the engine `ResourceMeter`'s `try_spend`,
/// `refill`, `tick_regen` / `tick_decay`.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct PlayerMana {
    pub meter: ae::ResourceMeter,
}

impl Default for PlayerMana {
    fn default() -> Self {
        Self {
            meter: ae::ResourceMeter::new(100.0, 0.0, 0.0),
        }
    }
}

impl PlayerMana {
    pub fn from_player(player: &ae::Player) -> Self {
        Self { meter: player.mana }
    }
}

/// Offensive scaling knobs (damage multiplier today; future crit /
/// armor-pierce / charge-attack damage scales attach here so the
/// damage gate stays a single-component read). Also carries the
/// `invincible` flag the dev-tools "godmode" toggle writes — the
/// ledger floated putting that on `PlayerHealth` but a separate
/// component bundle keeps `PlayerHealth`'s public shape (which other
/// tests pin) untouched.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
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

impl PlayerOffense {
    pub fn from_player(player: &ae::Player) -> Self {
        Self {
            damage_multiplier: player.damage_multiplier,
            invincible: player.invincible,
        }
    }
}

/// Lifetime + diagnostic counters that the trace recorder and RL
/// observation builder read every frame. None of these influence
/// gameplay simulation; they exist so the `time_alive` / `resets` /
/// `max_speed` columns on `AgentObservation` and the trace JSON keep
/// reporting truth after the engine `ae::Player` aggregate is gone.
///
/// `resets` increments via [`crate::reset_player`]; `time_alive`
/// accumulates per sim tick; `max_speed` is the per-life velocity
/// magnitude high-watermark.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct PlayerLifetime {
    pub time_alive: f32,
    pub resets: u32,
    pub max_speed: f32,
}

impl PlayerLifetime {
    pub fn from_player(player: &ae::Player) -> Self {
        Self {
            time_alive: player.time_alive,
            resets: player.resets,
            max_speed: player.max_speed,
        }
    }
}

/// Symbolic operation trace ("J o D o D"), preserved across the
/// engine-Player tick scratchpad so the HUD combo readout doesn't
/// blank every frame. The engine helper `update_player_*` appends
/// `MovementOp` entries via `Player::record`; the bridge mirrors
/// them onto / off of this component.
///
/// Holds at most 18 marks (matching the engine cap). Non-Copy
/// because of the inner `Vec`; clone is cheap (≤ 18 entries).
/// No `PartialEq` because `ae::ComboMark` doesn't impl it.
#[derive(Component, Clone, Debug, Default)]
pub struct PlayerComboTrace {
    pub combo: Vec<ae::ComboMark>,
}

impl PlayerComboTrace {
    pub fn from_player(player: &ae::Player) -> Self {
        Self {
            combo: player.combo.clone(),
        }
    }

    /// Symbolic combo string (the HUD readout). Empty trace returns
    /// `-` so the HUD field always renders something.
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

/// Generic ECS-owned action buffer.
///
/// Each field is a remaining-time buffer for the named action: > 0
/// means a press is alive and waiting to fire on the next legal frame.
/// Today only `jump` and `dash` are populated (matching the existing
/// `ae::Player::jump_buffer_timer` and `dash_buffer_timer`). The
/// `attack`, `pogo`, `projectile`, and `blink` slots exist so the
/// follow-up wave that lands attack/pogo/projectile/blink buffering
/// can populate them without adding more one-off timers — see
/// plan §"Action buffers" and §"Immediate follow-up / Generic action
/// buffering".
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct PlayerActionBuffer {
    pub jump: f32,
    pub dash: f32,
    pub attack: f32,
    pub pogo: f32,
    pub projectile: f32,
    pub blink: f32,
}

impl PlayerActionBuffer {
    pub fn from_player(player: &ae::Player) -> Self {
        Self {
            jump: player.jump_buffer_timer,
            dash: player.dash_buffer_timer,
            attack: 0.0,
            pogo: 0.0,
            projectile: 0.0,
            blink: 0.0,
        }
    }

    /// Record a button press by arming the named slot for `window`
    /// seconds. Idempotent within the window — repeated presses keep
    /// extending the freshness.
    pub fn press_jump(&mut self, window: f32) {
        self.jump = window;
    }

    pub fn press_dash(&mut self, window: f32) {
        self.dash = window;
    }

    /// Consume the jump buffer iff a press is alive; returns true on
    /// consume. Use this inside the jump-fire decision to honor a
    /// recent press.
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

    /// Advance every slot by `dt`, clamping to 0.
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

#[cfg(test)]
mod tests {
    use super::*;

    /// `from_player` round-trips every field a fresh engine `Player`
    /// has set, so spawn-time init does not silently drop state.
    /// Guards Phase 2: when the authority cut moves writers off the
    /// aggregate, anything the engine `Player::new` populates must
    /// land on these components or get explicitly deleted via the
    /// ledger — not lost.
    #[test]
    fn from_player_matches_fresh_engine_player() {
        let p = ae::Player::new(ae::Vec2::new(42.0, 17.0));

        let kin = PlayerKinematics::from_player(&p);
        assert_eq!(kin.pos, p.pos);
        assert_eq!(kin.vel, p.vel);
        assert_eq!(kin.size, p.size);
        assert_eq!(kin.base_size, p.base_size);
        assert_eq!(kin.facing, p.facing);

        let ground = PlayerGroundState::from_player(&p);
        assert_eq!(ground.on_ground, p.on_ground);
        assert_eq!(ground.coyote_timer, p.coyote_timer);

        let wall = PlayerWallState::from_player(&p);
        assert_eq!(wall.on_wall, p.on_wall);
        assert_eq!(wall.wall_normal_x, p.wall_normal_x);
        assert_eq!(wall.pre_wall_vel, p.pre_wall_vel);

        let jump = PlayerJumpState::from_player(&p);
        assert_eq!(jump.air_jumps_available, p.air_jumps_available);

        let dash = PlayerDashState::from_player(&p);
        assert_eq!(dash.charges_available, p.dash_charges_available);
        assert_eq!(dash.timer, p.dash_timer);
        assert_eq!(dash.cooldown, p.dash_cooldown);

        let flight = PlayerFlightState::from_player(&p);
        assert_eq!(flight.fly_enabled, p.fly_enabled);
        assert_eq!(flight.gliding, p.gliding);

        let blink = PlayerBlinkState::from_player(&p);
        assert_eq!(blink.cooldown, p.blink_cooldown);
        assert_eq!(blink.aim_offset, p.blink_aim_offset);
        assert_eq!(blink.grace_timer, p.blink_grace_timer);

        let ledge = PlayerLedgeState::from_player(&p);
        assert_eq!(ledge.grab, p.ledge_grab);
        assert_eq!(ledge.release_cooldown, p.ledge_release_cooldown);

        let dodge = PlayerDodgeState::from_player(&p);
        assert_eq!(dodge.roll_timer, p.dodge_roll_timer);
        assert_eq!(dodge.cooldown, p.dodge_roll_cooldown);

        let shield = PlayerShieldState::from_player(&p);
        assert_eq!(shield.active, p.shield_active);
        assert_eq!(shield.parry_window_timer, p.parry_window_timer);
        assert!(!shield.parrying());

        let body_mode = PlayerBodyModeState::from_player(&p);
        assert_eq!(body_mode.body_mode, p.body_mode);

        let env = PlayerEnvironmentContact::from_player(&p);
        assert_eq!(env.water, p.water_contact);
        assert_eq!(env.climbable, p.climbable_contact);

        let mana = PlayerMana::from_player(&p);
        assert_eq!(mana.meter, p.mana);

        let offense = PlayerOffense::from_player(&p);
        assert_eq!(offense.damage_multiplier, p.damage_multiplier);
        assert_eq!(offense.invincible, p.invincible);

        let lifetime = PlayerLifetime::from_player(&p);
        assert_eq!(lifetime.time_alive, p.time_alive);
        assert_eq!(lifetime.resets, p.resets);
        assert_eq!(lifetime.max_speed, p.max_speed);

        let combo = PlayerComboTrace::from_player(&p);
        assert_eq!(combo.combo.len(), p.combo.len());
        assert_eq!(combo.symbols(), "-");

        let buf = PlayerActionBuffer::from_player(&p);
        assert_eq!(buf.jump, p.jump_buffer_timer);
        assert_eq!(buf.dash, p.dash_buffer_timer);
    }

    /// Action buffer's press/consume cycle is single-use.
    /// Pre-poisons the slot to a non-zero sentinel before press so an
    /// early-return without write would still trip the consume check —
    /// matches the [[feedback_pre_poison_test_pattern]] guidance.
    #[test]
    fn action_buffer_jump_press_consume_cycle() {
        let mut buf = PlayerActionBuffer {
            jump: -1.0, // poison: not the value press_jump should leave
            ..Default::default()
        };
        buf.press_jump(0.15);
        assert_eq!(buf.jump, 0.15);
        assert!(buf.consume_jump());
        assert_eq!(buf.jump, 0.0);
        assert!(!buf.consume_jump(), "second consume must report empty");
    }

    /// `tick` clamps each slot at 0 — a slot can't go negative and
    /// later reappear when it next gets armed.
    #[test]
    fn action_buffer_tick_clamps_at_zero() {
        let mut buf = PlayerActionBuffer {
            jump: 0.05,
            dash: 0.10,
            ..Default::default()
        };
        buf.tick(0.5);
        assert_eq!(buf.jump, 0.0);
        assert_eq!(buf.dash, 0.0);
        // After clamp, a fresh press still arms cleanly.
        buf.press_dash(0.2);
        assert_eq!(buf.dash, 0.2);
    }

    /// `PlayerKinematics::aabb` uses half-extents matching
    /// `ae::Player::aabb`. The two must stay equivalent until Phase 3
    /// deletes the engine aggregate; otherwise the live authority
    /// path (which still uses `Player::aabb`) and any new code that
    /// adopts the component path would compute different volumes.
    #[test]
    fn kinematics_aabb_matches_engine_player_aabb() {
        let p = ae::Player::new(ae::Vec2::new(123.0, -45.0));
        let kin = PlayerKinematics::from_player(&p);
        assert_eq!(p.aabb(), kin.aabb());
    }
}
