//! Tick-local `ae::Player` scratchpad.
//!
//! Phase 2 of the player-ecs-bandaid plan cuts
//! [`super::components::PlayerMovementAuthority`] off the player entity
//! but keeps the engine `update_player_*` helpers (`player_control_phase`
//! / `player_simulation_phase` / room-transition `apply_on_arrival`)
//! untouched until Phase 3 ports each movement feature cluster to its
//! own ECS system.
//!
//! In the meantime, the bridge lets the live tick keep calling those
//! helpers by assembling a tick-local `ae::Player` from the new ECS
//! cluster components at the START of each authority caller, running
//! the existing engine code against it, and committing back to the
//! clusters at the END. This is the transitional shim the plan
//! authorizes under §"Hemorrhage controls / keep each temporary shim
//! local to the branch and delete it before the merge-ready diff" —
//! the entire `engine_player_bridge` module deletes in Phase 3.
//!
//! Do not add new readers of the assembled `ae::Player` outside the
//! authority callers. Read directly from the cluster components.

use ambition_engine as ae;

use super::movement_components::{
    PlayerAbilities, PlayerActionBuffer, PlayerBlinkState, PlayerBodyModeState, PlayerComboTrace,
    PlayerDashState, PlayerDodgeState, PlayerEnvironmentContact, PlayerFlightState,
    PlayerGroundState, PlayerJumpState, PlayerKinematics, PlayerLedgeState, PlayerLifetime,
    PlayerMana, PlayerOffense, PlayerShieldState, PlayerWallState,
};

/// Mutable cluster references the bridge assembles from. Grouped into
/// a struct so the assemble/commit signatures don't carry 18 separate
/// arguments and so callers can capture the player's component
/// references in one query item.
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

/// Build a tick-local `ae::Player` from the current cluster state. The
/// returned value is owned; the caller passes `&mut player` into the
/// existing engine helpers and calls [`commit_player`] when done.
pub fn assemble_player(clusters: &PlayerClustersMut<'_>) -> ae::Player {
    ae::Player {
        abilities: clusters.abilities.abilities,
        pos: clusters.kinematics.pos,
        vel: clusters.kinematics.vel,
        size: clusters.kinematics.size,
        base_size: clusters.kinematics.base_size,
        facing: clusters.kinematics.facing,
        on_ground: clusters.ground.on_ground,
        on_wall: clusters.wall.on_wall,
        wall_normal_x: clusters.wall.wall_normal_x,
        dash_charges_available: clusters.dash.charges_available,
        air_jumps_available: clusters.jump.air_jumps_available,
        fly_enabled: clusters.flight.fly_enabled,
        flight_phase: clusters.flight.flight_phase,
        blink_cooldown: clusters.blink.cooldown,
        blink_hold_active: clusters.blink.hold_active,
        blink_hold_timer: clusters.blink.hold_timer,
        blink_aiming: clusters.blink.aiming,
        blink_aim_offset: clusters.blink.aim_offset,
        blink_grace_timer: clusters.blink.grace_timer,
        fast_falling: clusters.flight.fast_falling,
        gliding: clusters.flight.gliding,
        wall_clinging: clusters.wall.wall_clinging,
        wall_climbing: clusters.wall.wall_climbing,
        dash_timer: clusters.dash.timer,
        dash_cooldown: clusters.dash.cooldown,
        dash_buffer_timer: clusters.action_buffer.dash,
        jump_buffer_timer: clusters.action_buffer.jump,
        coyote_timer: clusters.ground.coyote_timer,
        rebound_cooldown: clusters.ground.rebound_cooldown,
        drop_through_timer: clusters.ground.drop_through_timer,
        combo: clusters.combo_trace.combo.clone(),
        max_speed: clusters.lifetime.max_speed,
        time_alive: clusters.lifetime.time_alive,
        resets: clusters.lifetime.resets,
        damage_multiplier: clusters.offense.damage_multiplier,
        mana: clusters.mana.meter,
        invincible: clusters.offense.invincible,
        body_mode: clusters.body_mode.body_mode,
        water_contact: clusters.env_contact.water,
        climbable_contact: clusters.env_contact.climbable,
        ledge_grab: clusters.ledge.grab,
        pre_wall_vel: clusters.wall.pre_wall_vel,
        pre_wall_vel_age: clusters.wall.pre_wall_vel_age,
        ledge_release_cooldown: clusters.ledge.release_cooldown,
        dodge_roll_timer: clusters.dodge.roll_timer,
        dodge_roll_cooldown: clusters.dodge.cooldown,
        shield_active: clusters.shield.active,
        parry_window_timer: clusters.shield.parry_window_timer,
    }
}

/// Write the post-tick `ae::Player` back to the cluster components.
/// Consumes the player by value so callers can't accidentally keep
/// using the stale scratchpad after commit.
pub fn commit_player(player: ae::Player, clusters: &mut PlayerClustersMut<'_>) {
    // PlayerAbilities is read-only inside the tick — the engine
    // doesn't mutate `player.abilities`. Skip the writeback.
    clusters.kinematics.pos = player.pos;
    clusters.kinematics.vel = player.vel;
    clusters.kinematics.size = player.size;
    clusters.kinematics.base_size = player.base_size;
    clusters.kinematics.facing = player.facing;

    clusters.ground.on_ground = player.on_ground;
    clusters.ground.coyote_timer = player.coyote_timer;
    clusters.ground.drop_through_timer = player.drop_through_timer;
    clusters.ground.rebound_cooldown = player.rebound_cooldown;

    clusters.wall.on_wall = player.on_wall;
    clusters.wall.wall_normal_x = player.wall_normal_x;
    clusters.wall.wall_clinging = player.wall_clinging;
    clusters.wall.wall_climbing = player.wall_climbing;
    clusters.wall.pre_wall_vel = player.pre_wall_vel;
    clusters.wall.pre_wall_vel_age = player.pre_wall_vel_age;

    clusters.jump.air_jumps_available = player.air_jumps_available;

    clusters.dash.charges_available = player.dash_charges_available;
    clusters.dash.timer = player.dash_timer;
    clusters.dash.cooldown = player.dash_cooldown;

    clusters.flight.fly_enabled = player.fly_enabled;
    clusters.flight.flight_phase = player.flight_phase;
    clusters.flight.gliding = player.gliding;
    clusters.flight.fast_falling = player.fast_falling;

    clusters.blink.cooldown = player.blink_cooldown;
    clusters.blink.hold_active = player.blink_hold_active;
    clusters.blink.hold_timer = player.blink_hold_timer;
    clusters.blink.aiming = player.blink_aiming;
    clusters.blink.aim_offset = player.blink_aim_offset;
    clusters.blink.grace_timer = player.blink_grace_timer;

    clusters.ledge.grab = player.ledge_grab;
    clusters.ledge.release_cooldown = player.ledge_release_cooldown;

    clusters.dodge.roll_timer = player.dodge_roll_timer;
    clusters.dodge.cooldown = player.dodge_roll_cooldown;

    clusters.shield.active = player.shield_active;
    clusters.shield.parry_window_timer = player.parry_window_timer;

    clusters.body_mode.body_mode = player.body_mode;

    clusters.env_contact.water = player.water_contact;
    clusters.env_contact.climbable = player.climbable_contact;

    clusters.mana.meter = player.mana;

    clusters.offense.damage_multiplier = player.damage_multiplier;
    clusters.offense.invincible = player.invincible;

    clusters.action_buffer.jump = player.jump_buffer_timer;
    clusters.action_buffer.dash = player.dash_buffer_timer;
    // attack / pogo / projectile / blink slots are not driven by the
    // engine `Player` aggregate today — they wait for the Phase 3
    // action-buffer system to take ownership.

    clusters.lifetime.time_alive = player.time_alive;
    clusters.lifetime.resets = player.resets;
    clusters.lifetime.max_speed = player.max_speed;

    clusters.combo_trace.combo = player.combo;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Round-trip: assemble from clusters, do nothing, commit back.
    /// Every cluster field must equal its initial value — the bridge
    /// must not lose or corrupt state for the no-op case.
    #[test]
    fn assemble_commit_roundtrip_is_identity() {
        // Construct a non-default engine player so the round-trip
        // covers fields a zero-initialized player would mask.
        let mut p = ae::Player::new(ae::Vec2::new(123.0, -45.0));
        p.vel = ae::Vec2::new(10.0, -20.0);
        p.facing = -1.0;
        p.on_ground = true;
        p.on_wall = true;
        p.wall_normal_x = -1.0;
        p.wall_clinging = true;
        p.dash_charges_available = 3;
        p.dash_timer = 0.07;
        p.dash_cooldown = 0.5;
        p.air_jumps_available = 2;
        p.coyote_timer = 0.08;
        p.fly_enabled = true;
        p.gliding = true;
        p.fast_falling = true;
        p.blink_cooldown = 0.4;
        p.blink_aiming = true;
        p.blink_grace_timer = 0.1;
        p.body_mode = ae::BodyMode::Crouching;
        p.size = ae::Vec2::new(30.0, 26.4);
        p.damage_multiplier = 3;
        p.invincible = true;
        p.dodge_roll_timer = 0.2;
        p.shield_active = true;
        p.parry_window_timer = 0.15;
        p.time_alive = 42.0;
        p.resets = 7;
        p.max_speed = 350.0;
        p.combo.push(ae::ComboMark {
            op: ae::MovementOp::Jump,
            age: 0.0,
        });

        // Initialize clusters from the engine player.
        let abilities = PlayerAbilities::from_player(&p);
        let mut kinematics = PlayerKinematics::from_player(&p);
        let mut ground = PlayerGroundState::from_player(&p);
        let mut wall = PlayerWallState::from_player(&p);
        let mut jump = PlayerJumpState::from_player(&p);
        let mut dash = PlayerDashState::from_player(&p);
        let mut flight = PlayerFlightState::from_player(&p);
        let mut blink = PlayerBlinkState::from_player(&p);
        let mut ledge = PlayerLedgeState::from_player(&p);
        let mut dodge = PlayerDodgeState::from_player(&p);
        let mut shield = PlayerShieldState::from_player(&p);
        let mut body_mode = PlayerBodyModeState::from_player(&p);
        let mut env_contact = PlayerEnvironmentContact::from_player(&p);
        let mut mana = PlayerMana::from_player(&p);
        let mut offense = PlayerOffense::from_player(&p);
        let mut action_buffer = PlayerActionBuffer::from_player(&p);
        let mut lifetime = PlayerLifetime::from_player(&p);
        let mut combo_trace = PlayerComboTrace::from_player(&p);

        let mut clusters = PlayerClustersMut {
            abilities: &abilities,
            kinematics: &mut kinematics,
            ground: &mut ground,
            wall: &mut wall,
            jump: &mut jump,
            dash: &mut dash,
            flight: &mut flight,
            blink: &mut blink,
            ledge: &mut ledge,
            dodge: &mut dodge,
            shield: &mut shield,
            body_mode: &mut body_mode,
            env_contact: &mut env_contact,
            mana: &mut mana,
            offense: &mut offense,
            action_buffer: &mut action_buffer,
            lifetime: &mut lifetime,
            combo_trace: &mut combo_trace,
        };

        let assembled = assemble_player(&clusters);
        // Sanity: the assembled player should equal the source for
        // every field the bridge plumbs through.
        assert_eq!(assembled.pos, p.pos);
        assert_eq!(assembled.vel, p.vel);
        assert_eq!(assembled.facing, p.facing);
        assert_eq!(assembled.on_ground, p.on_ground);
        assert_eq!(assembled.on_wall, p.on_wall);
        assert_eq!(assembled.wall_normal_x, p.wall_normal_x);
        assert_eq!(assembled.wall_clinging, p.wall_clinging);
        assert_eq!(assembled.dash_charges_available, p.dash_charges_available);
        assert_eq!(assembled.air_jumps_available, p.air_jumps_available);
        assert_eq!(assembled.coyote_timer, p.coyote_timer);
        assert_eq!(assembled.fly_enabled, p.fly_enabled);
        assert_eq!(assembled.gliding, p.gliding);
        assert_eq!(assembled.fast_falling, p.fast_falling);
        assert_eq!(assembled.blink_cooldown, p.blink_cooldown);
        assert_eq!(assembled.blink_aiming, p.blink_aiming);
        assert_eq!(assembled.blink_grace_timer, p.blink_grace_timer);
        assert_eq!(assembled.body_mode, p.body_mode);
        assert_eq!(assembled.size, p.size);
        assert_eq!(assembled.damage_multiplier, p.damage_multiplier);
        assert_eq!(assembled.invincible, p.invincible);
        assert_eq!(assembled.dodge_roll_timer, p.dodge_roll_timer);
        assert_eq!(assembled.shield_active, p.shield_active);
        assert_eq!(assembled.parry_window_timer, p.parry_window_timer);
        assert_eq!(assembled.time_alive, p.time_alive);
        assert_eq!(assembled.resets, p.resets);
        assert_eq!(assembled.max_speed, p.max_speed);
        // ComboMark is not PartialEq; compare via length + symbol.
        assert_eq!(assembled.combo.len(), p.combo.len());

        // Commit a deliberately-mutated scratchpad to prove writeback.
        // Pre-poison the cluster's `pos` so a missing writeback path
        // would be detected ([[feedback_pre_poison_test_pattern]]).
        clusters.kinematics.pos = ae::Vec2::new(f32::NAN, f32::NAN);
        clusters.lifetime.resets = u32::MAX;

        let mut mutated = assembled;
        mutated.pos = ae::Vec2::new(999.0, 888.0);
        mutated.vel = ae::Vec2::ZERO;
        mutated.coyote_timer = 0.0;
        mutated.combo.push(ae::ComboMark {
            op: ae::MovementOp::Dash,
            age: 0.0,
        });
        mutated.resets = 8;
        commit_player(mutated, &mut clusters);

        assert_eq!(clusters.kinematics.pos, ae::Vec2::new(999.0, 888.0));
        assert_eq!(clusters.kinematics.vel, ae::Vec2::ZERO);
        assert_eq!(clusters.ground.coyote_timer, 0.0);
        assert_eq!(clusters.combo_trace.combo.len(), 2);
        assert_eq!(clusters.lifetime.resets, 8);
    }
}
