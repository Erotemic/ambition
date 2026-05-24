//! Player ECS components.
//!
//! The player entity carries one of each of these as its frame-to-frame
//! authoritative state. See [`super::bundles::PlayerSimulationBundle`] for
//! the canonical spawn shape.

use ambition_engine as ae;
use bevy::prelude::*;

use crate::input::ControlFrame;

/// Marker for **a player entity** — there may eventually be more than
/// one. Use this when a query wants every player regardless of locality
/// or which slot they occupy.
///
/// The game currently spawns exactly one player, with `PlayerSlot(0)`,
/// [`PrimaryPlayer`], and [`LocalPlayer`] all attached. Systems that
/// want the camera/HUD/dev-tool target should filter on `PrimaryPlayer`
/// (or use the helpers in [`crate::player::queries`]) rather than
/// assuming the only `PlayerEntity` is *the* player.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlayerEntity;

/// Per-player slot identifier. Slot `0` is the local primary player;
/// future co-op / split-screen / network players will use slots
/// `1..=N`. Stored as a `u8` so it can fit comfortably in a HUD
/// label, a save key, or a debug overlay glyph.
///
/// `PlayerSlot` is the canonical "which player?" handle for new
/// player-bearing messages and resources. New player-domain message
/// types (heal, damage, respawn, cosmetic, …) SHOULD carry either an
/// `Entity` or a `PlayerSlot` so they don't silently assume the
/// primary player.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PlayerSlot(pub u8);

impl PlayerSlot {
    /// Slot reserved for the local primary player in single-player
    /// builds and for player 1 in future local-multiplayer modes.
    pub const PRIMARY: PlayerSlot = PlayerSlot(0);

    pub fn index(self) -> u8 {
        self.0
    }
}

/// Marks the player that the camera, HUD, dev tools, and pause menu
/// follow by default. Exactly one entity in the world should carry
/// this component; today every spawned player is also primary.
///
/// Distinct from [`LocalPlayer`] because in a future split-screen
/// build the local players would each be `LocalPlayer` but only one
/// would be `PrimaryPlayer` (e.g. the host's view in a guest-joined
/// session).
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PrimaryPlayer;

/// Marks a player whose input comes from this machine's input devices
/// (keyboard / gamepad / touch). In single-player today the local
/// player is also the primary player. In a future networked build,
/// remote players would have `PlayerEntity` (+ `PlayerSlot`) but not
/// `LocalPlayer`.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LocalPlayer;

/// Per-player input snapshot. Mirrors the single global
/// [`crate::input::ControlFrame`] resource onto the local player
/// entity so simulation systems can move toward reading input from a
/// `Query<&PlayerInputFrame>` rather than `Res<ControlFrame>`. That's
/// the architectural seam multiplayer / netcode work needs:
///
/// - the local primary player's frame is filled by
///   `sync_local_player_input_frame` after the input pipeline writes
///   `Res<ControlFrame>`;
/// - future remote / co-op players would have their own
///   `PlayerInputFrame` populated by a network adapter or a second
///   input device, without competing for the single global resource.
///
/// Today exactly one entity (the local primary player) carries this
/// component, and `Res<ControlFrame>` stays the single writer channel.
/// New simulation systems should prefer this component so they're
/// already shaped for multiple input-bearing players.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct PlayerInputFrame {
    pub frame: ControlFrame,
}

/// Frame-to-frame authoritative player movement state.
///
/// This is the single source of truth for `ae::Player` within the Bevy world.
/// All sandbox systems that read or write player movement/ability state must
/// go through this component.
#[derive(Component, Clone)]
pub struct PlayerMovementAuthority {
    pub player: ae::Player,
}

impl PlayerMovementAuthority {
    pub fn new(player: ae::Player) -> Self {
        Self { player }
    }

    pub fn body(&self) -> PlayerBody {
        PlayerBody::from_player(&self.player)
    }
}

/// ECS-visible player body.
///
/// The full engine `ae::Player` state lives on `PlayerMovementAuthority`; this
/// compact component is the query-friendly body/read model for systems that do
/// not need every movement-internal field.
///
/// Expanded during Chunk 4d of the universal-brain interface
/// roll-out so readers can leave `PlayerMovementAuthority` behind
/// for `PlayerBody`. New fields:
/// - `on_wall`, `wall_clinging`, `wall_climbing` (wall state cluster)
/// - `water_contact`, `climbable_contact` (world-contact cluster)
/// - `dash_timer` (dash cluster — the timer the existing dash-active
///   readers need without owning the entire dash state machine)
/// - `blink_aiming` (blink HUD cluster — true while the aim is held)
///
/// Writers stay on `PlayerMovementAuthority` for now; the sync
/// system [`crate::player::write_player_ecs_components`] mirrors
/// these into `PlayerBody` each frame. Chunk 4e+ migrates readers
/// onto `PlayerBody`.
#[derive(Component, Clone, Copy, Debug, PartialEq, Default)]
pub struct PlayerBody {
    pub pos: ae::Vec2,
    pub vel: ae::Vec2,
    pub size: ae::Vec2,
    pub base_size: ae::Vec2,
    pub facing: f32,
    pub on_ground: bool,
    /// True iff the body is in contact with a wall this tick.
    pub on_wall: bool,
    /// Set while the body is wall-clinging (Hollow-Knight style).
    pub wall_clinging: bool,
    /// Set while the body is actively wall-climbing.
    pub wall_climbing: bool,
    pub fly_enabled: bool,
    pub dash_charges_available: u8,
    pub air_jumps_available: u8,
    /// `> 0` while a dash is mid-execution; the value is the
    /// remaining dash-active seconds.
    pub dash_timer: f32,
    /// True while the blink aim is held (drives the blink HUD).
    pub blink_aiming: bool,
    pub mana_current: f32,
    pub body_mode: ae::BodyMode,
    pub invincible: bool,
    pub dodge_rolling: bool,
    /// True while the shield ability is active (button held, not dashing).
    /// Used by the sandbox to show the bubble visual.
    pub shielding: bool,
    /// True during the parry window: shield is active AND `parry_window_timer > 0`.
    /// Damage checks gate contact damage behind `!parrying`.
    pub parrying: bool,
    /// Most-recent submerged-in-water contact, copied off
    /// `ae::Player::water_contact`. `None` = dry.
    pub water_contact: Option<ae::WaterContact>,
    /// Most-recent climbable-region contact (ladders / vines).
    /// `None` = not in a climbable region this tick.
    pub climbable_contact: Option<ae::ClimbableContact>,
}

impl PlayerBody {
    pub fn from_player(player: &ae::Player) -> Self {
        Self {
            pos: player.pos,
            vel: player.vel,
            size: player.size,
            base_size: player.base_size,
            facing: player.facing,
            on_ground: player.on_ground,
            on_wall: player.on_wall,
            wall_clinging: player.wall_clinging,
            wall_climbing: player.wall_climbing,
            fly_enabled: player.fly_enabled,
            dash_charges_available: player.dash_charges_available,
            air_jumps_available: player.air_jumps_available,
            dash_timer: player.dash_timer,
            blink_aiming: player.blink_aiming,
            mana_current: player.mana.current,
            body_mode: player.body_mode,
            invincible: player.invincible,
            dodge_rolling: player.dodge_roll_timer > 0.0,
            shielding: player.shield_active,
            parrying: player.shield_active && player.parry_window_timer > 0.0,
            water_contact: player.water_contact,
            climbable_contact: player.climbable_contact,
        }
    }

    pub fn aabb(self) -> ae::Aabb {
        ae::Aabb::new(self.pos, self.size * 0.5)
    }
}

/// ECS-owned player health.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlayerHealth {
    pub health: ae::Health,
}

impl PlayerHealth {
    pub fn new(health: ae::Health) -> Self {
        Self { health }
    }

    pub fn current(self) -> i32 {
        self.health.current
    }

    pub fn max(self) -> i32 {
        self.health.max
    }

    pub fn heal(&mut self, amount: i32) {
        self.health.heal(amount);
    }

    pub fn damage(&mut self, amount: i32) -> bool {
        self.health.damage(amount)
    }

    pub fn reset(&mut self) {
        self.health.reset();
    }
}

/// ECS-authoritative player combat/timer state.
///
/// The four timer fields are written directly by the phase helpers and
/// `world_flow` functions that produce damage/hit/respawn events.
/// `write_player_ecs_components` no longer touches them; it only syncs the
/// `attacking` flag from the per-player `ActivePlayerAttack` component so
/// rendering systems can check attack state without querying the runtime.
#[derive(Component, Clone, Debug, Default, PartialEq)]
pub struct PlayerCombatState {
    /// Presentation flash (damage hit-blink). Decays in `cleanup_timers_system`.
    pub flash_timer: f32,
    /// Hitstop: freezes `time_scale` to 0 while positive. Decays in `input_timer_system`.
    pub hitstop_timer: f32,
    /// Invulnerability window after taking damage. Decays in `input_timer_system`.
    pub damage_invuln_timer: f32,
    /// Partial-control penalty after knockback. Decays in `input_timer_system`.
    pub hitstun_timer: f32,
    /// Mirrored each frame from `ActivePlayerAttack::is_active()`.
    pub attacking: bool,
}

impl PlayerCombatState {
    pub fn vulnerable(&self) -> bool {
        self.damage_invuln_timer <= 0.0
    }

    pub fn reset(&mut self) {
        self.flash_timer = 0.0;
        self.hitstop_timer = 0.0;
        self.damage_invuln_timer = 0.0;
        self.hitstun_timer = 0.0;
        self.attacking = false;
    }
}

/// Per-player active melee swing. `None` when no swing is in progress.
///
/// Authoritative source: set/cleared by `start_attack` / `advance_attack`.
/// `write_player_ecs_components` mirrors `is_some()` into
/// `PlayerCombatState::attacking` each frame so rendering can branch on
/// attack state without a separate query.
///
/// Replaces the global `CurrentPlayerAttack` resource (OVERNIGHT-TODO
/// #17.4 / the multiplayer caveat that used to live in `lib.rs`). Each
/// player entity carries its own attack state, so a future co-op /
/// split-screen build can spawn additional players whose swings tick
/// independently.
#[derive(Component, Clone, Debug, Default)]
pub struct ActivePlayerAttack(pub Option<super::super::PlayerAttackState>);

impl ActivePlayerAttack {
    pub fn is_active(&self) -> bool {
        self.0.is_some()
    }

    pub fn clear(&mut self) {
        self.0 = None;
    }
}

/// ECS-owned player animation signal timers.
///
/// All fields are presentation-only: they gate which sprite row plays and
/// decay independent of gameplay timers like hitstop or invulnerability.
/// Written directly by `cleanup_timers_system` / `start_attack` /
/// `advance_attack`; `animate_player` reads them via `pick_player_anim`.
/// This is the authoritative source — `write_player_ecs_components` does
/// not touch it.
#[derive(Component, Clone, Debug, Default, PartialEq)]
pub struct PlayerAnimState {
    /// Time remaining for the slash animation row.
    pub slash_anim_timer: f32,
    /// Time remaining for the post-touchdown landing pose.
    pub land_anim_timer: f32,
    /// True when the landing was fast enough for the hard-impact row.
    pub land_anim_hard: bool,
    /// Time remaining for the brief dash pre-roll pose.
    pub dash_startup_timer: f32,
    /// Previous frame's `on_ground`; used to detect the touchdown edge.
    pub anim_prev_on_ground: bool,
    /// Previous frame's pre-landing downward velocity; used to grade
    /// hard vs. soft landings.
    pub anim_prev_vel_y: f32,
    /// Previous frame's `dash_timer`; used to detect the dash rising edge.
    pub anim_prev_dash_timer: f32,
}

impl PlayerAnimState {
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// ECS-visible player interaction buffer state.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct PlayerInteractionState {
    /// Counts down after a double-tap-down edge; non-zero means morph-ball
    /// entry is pending for the body-mode driver.
    pub down_tap_timer: f32,
    /// Counts down after a double-tap-up edge; drives door/NPC triggers.
    pub up_tap_timer: f32,
    /// Counts down after `interact_pressed`; keeps the interact signal alive
    /// across frames so the player doesn't need to hold the button until the
    /// door animation completes.
    pub interact_buffer_timer: f32,
    /// Set true by `input_timer_system` when a double-tap-down is detected;
    /// consumed by the body-mode driver after `player_control_system + player_simulation_system`.
    pub double_tap_down_pending: bool,
    /// Set true by `input_timer_system` when a double-tap-up gesture is
    /// detected; consumed (via `mem::take`) by `interaction_input_system`
    /// the same frame to fold it into the hit-stun-gated interact buffer
    /// that drives door / NPC / chest activation.
    pub double_tap_up_pending: bool,
}

impl PlayerInteractionState {
    /// Advance timers and detect a double-tap-down edge. Returns `true` when
    /// two taps arrive within `window` seconds.
    pub fn register_down_tap(&mut self, down_pressed: bool, frame_dt: f32, window: f32) -> bool {
        self.down_tap_timer = (self.down_tap_timer - frame_dt).max(0.0);
        if !down_pressed {
            return false;
        }
        if self.down_tap_timer > 0.0 {
            self.down_tap_timer = 0.0;
            true
        } else {
            self.down_tap_timer = window;
            false
        }
    }

    /// Advance timers and detect a double-tap-up edge. Returns `true` when
    /// two taps arrive within `window` seconds.
    pub fn register_up_tap(&mut self, up_pressed: bool, frame_dt: f32, window: f32) -> bool {
        self.up_tap_timer = (self.up_tap_timer - frame_dt).max(0.0);
        if !up_pressed {
            return false;
        }
        if self.up_tap_timer > 0.0 {
            self.up_tap_timer = 0.0;
            true
        } else {
            self.up_tap_timer = window;
            false
        }
    }

    /// Update the interact buffer and return whether the buffer is live.
    pub fn buffered_interact(&mut self, pressed: bool, frame_dt: f32, window: f32) -> bool {
        self.interact_buffer_timer = (self.interact_buffer_timer - frame_dt).max(0.0);
        if pressed {
            self.interact_buffer_timer = window;
        }
        self.interact_buffer_timer > 0.0
    }

    pub fn buffered(self) -> bool {
        self.interact_buffer_timer > 0.0
    }

    pub fn clear(&mut self) {
        self.interact_buffer_timer = 0.0;
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// Camera easing and blink-in presentation state. Authoritative ECS component;
/// written by `cleanup_timers_system`, `load_room`, and `handle_player_events`
/// (blink path). Read by the camera follow system and the sprite animator.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct PlayerBlinkCameraState {
    /// Counts down from `blink_in_duration` to 0 after a blink; the camera
    /// and animator use this to play the arrival ease-in.
    pub blink_in_timer: f32,
    /// Set to `BLINK_IN_ANIM_TIME` when a blink fires; used to normalise
    /// `blink_in_timer` into a 0..1 progress value.
    pub blink_in_duration: f32,
    /// World-space camera position at the moment the blink fired; the camera
    /// eases from here toward the new player position.
    pub blink_camera_from: ambition_engine::Vec2,
    /// Blink destination in world space (set alongside `blink_camera_from`
    /// for future use; not yet consumed by the camera easing path).
    pub blink_camera_to: ambition_engine::Vec2,
    /// Positive while the camera should snap (not ease) to the player position.
    /// Set on door transitions; zero on edge exits to allow scroll effects.
    pub camera_snap_timer: f32,
}

impl Default for PlayerBlinkCameraState {
    fn default() -> Self {
        Self {
            blink_in_timer: 0.0,
            blink_in_duration: 0.0,
            blink_camera_from: ambition_engine::Vec2::ZERO,
            blink_camera_to: ambition_engine::Vec2::ZERO,
            camera_snap_timer: 0.0,
        }
    }
}

impl PlayerBlinkCameraState {
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// Sandbox-only scratch flag: whether the player was riding a moving
/// platform last simulation frame. Used by `player_simulation_phase`'s
/// diagnostic debug log that prints riding-state transitions while
/// chasing the "glitchy platform behavior" repro.
///
/// Lives on the player ECS entity rather than on `ae::Player` because
/// moving platforms are a sandbox concept — the engine controller is
/// platform-agnostic. Auto-resets to `false` because the field has no
/// meaningful initial value; on a player reset we don't need to carry
/// the previous riding state across.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlayerPlatformRideState {
    pub was_riding: bool,
}

/// Per-player "last known safe spot" used by hazard knockback and
/// debug respawn helpers. Authored separately from the engine
/// `ae::Player::pos` so reset paths and trace recorders can read a
/// value that was deliberately gated by `SafePositionContext`
/// rather than the raw frame-to-frame position.
///
/// Replaces `SandboxSimState::last_safe_player_pos`
/// (OVERNIGHT-TODO #17.9). The old resource field implicitly meant
/// "the primary player's safe spot" — a future co-op build wants
/// per-player anchors so a second player can hazard-fail without
/// the first player's safe spot moving.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct PlayerSafetyState {
    /// Last grounded, gameplay-safe position the safety gate
    /// approved (see `crate::remember_safe_player_position`). The
    /// hazard / OOB respawn path warps the player here.
    pub last_safe_pos: ae::Vec2,
}

impl PlayerSafetyState {
    pub fn new(initial: ae::Vec2) -> Self {
        Self {
            last_safe_pos: initial,
        }
    }
}

#[cfg(test)]
mod multiplayer_smoke_tests {
    use super::*;
    use crate::player::PrimaryPlayerOnly;
    use ambition_engine as ae;

    fn dummy_attack_spec() -> ae::AttackSpec {
        // Construct via the live `attack_spec` builder; a minimal Player
        // is enough — only the `intent` field is meaningful for these
        // tests, and the builder gives us a well-formed spec with
        // non-zero timings so the `PlayerAttackState::done()` path
        // doesn't short-circuit.
        let world = ae::World::new(
            "smoke",
            ae::Vec2::new(1000.0, 1000.0),
            ae::Vec2::new(100.0, 900.0),
            vec![],
        );
        let player = ae::Player::new_with_abilities(world.spawn, ae::AbilitySet::sandbox_all());
        ae::attack_spec(&player, ae::AttackIntent::Forward)
    }

    /// Two player entities each carry their own `ActivePlayerAttack`,
    /// so a swing on one player does not silently affect the other.
    /// Regression guard for the old shared-resource shape — if a
    /// future patch turns `ActivePlayerAttack` back into a global
    /// `Resource`, this test stops being meaningful and should fail
    /// loudly when it tries to read two values.
    #[test]
    fn two_players_have_independent_active_attacks() {
        let mut app = App::new();
        let p1 = app
            .world_mut()
            .spawn((
                PlayerEntity,
                PlayerSlot(0),
                PrimaryPlayer,
                ActivePlayerAttack::default(),
            ))
            .id();
        let p2 = app
            .world_mut()
            .spawn((PlayerEntity, PlayerSlot(1), ActivePlayerAttack::default()))
            .id();

        // Start an attack on player 1 only.
        let attack_spec = dummy_attack_spec();
        app.world_mut()
            .entity_mut(p1)
            .get_mut::<ActivePlayerAttack>()
            .expect("p1 has the component")
            .0 = Some(crate::PlayerAttackState::new(attack_spec));

        let p1_attack = app
            .world()
            .entity(p1)
            .get::<ActivePlayerAttack>()
            .expect("p1 has the component");
        let p2_attack = app
            .world()
            .entity(p2)
            .get::<ActivePlayerAttack>()
            .expect("p2 has the component");

        assert!(p1_attack.is_active(), "p1 should be mid-attack");
        assert!(
            !p2_attack.is_active(),
            "p2's attack must not pick up p1's swing — that's the whole \
             point of moving CurrentPlayerAttack onto the player entity \
             (OVERNIGHT-TODO #17.4)"
        );
    }

    /// Two players each carry their own `PlayerSafetyState`; updating
    /// one player's safe position must not move the other player's
    /// anchor (OVERNIGHT-TODO #17.9).
    #[test]
    fn two_players_have_independent_safety_anchors() {
        let mut app = App::new();
        let p1_initial = ae::Vec2::new(100.0, 100.0);
        let p2_initial = ae::Vec2::new(500.0, 500.0);
        let p1 = app
            .world_mut()
            .spawn((
                PlayerEntity,
                PlayerSlot(0),
                PrimaryPlayer,
                PlayerSafetyState::new(p1_initial),
            ))
            .id();
        let p2 = app
            .world_mut()
            .spawn((
                PlayerEntity,
                PlayerSlot(1),
                PlayerSafetyState::new(p2_initial),
            ))
            .id();

        app.world_mut()
            .entity_mut(p1)
            .get_mut::<PlayerSafetyState>()
            .unwrap()
            .last_safe_pos = ae::Vec2::new(999.0, 999.0);

        assert_eq!(
            app.world()
                .entity(p1)
                .get::<PlayerSafetyState>()
                .unwrap()
                .last_safe_pos,
            ae::Vec2::new(999.0, 999.0)
        );
        assert_eq!(
            app.world()
                .entity(p2)
                .get::<PlayerSafetyState>()
                .unwrap()
                .last_safe_pos,
            p2_initial,
            "p2's anchor must not pick up p1's update — that's the whole \
             point of moving last_safe_player_pos onto the player entity"
        );
    }

    /// With two `PlayerEntity` actors spawned, a `Query<...,
    /// PrimaryPlayerOnly>` resolves to exactly one entity. Together
    /// with the next test (which checks generic `With<PlayerEntity>`
    /// queries see both), this pins the invariant the audit calls
    /// out: only one player carries the `PrimaryPlayer` marker, so
    /// camera/HUD/input systems can keep using `.single()` safely
    /// while combat/hazard systems iterate.
    #[test]
    fn primary_player_query_resolves_with_two_players_spawned() {
        let mut app = App::new();
        app.world_mut()
            .spawn((PlayerEntity, PlayerSlot(0), PrimaryPlayer));
        app.world_mut().spawn((PlayerEntity, PlayerSlot(1)));

        let mut q = app
            .world_mut()
            .query_filtered::<Entity, PrimaryPlayerOnly>();
        let primaries: Vec<Entity> = q.iter(app.world()).collect();
        assert_eq!(
            primaries.len(),
            1,
            "exactly one entity must carry both PlayerEntity and PrimaryPlayer; \
             camera/HUD systems rely on this for `.single()` correctness"
        );
    }

    /// Generic `With<PlayerEntity>` queries see every spawned player,
    /// even the non-primary one. This is the half of the architectural
    /// promise that lets hazards/projectiles/pickups iterate over all
    /// players in B-bucket systems (audit doc §B).
    #[test]
    fn player_entity_query_iterates_all_spawned_players() {
        let mut app = App::new();
        app.world_mut()
            .spawn((PlayerEntity, PlayerSlot(0), PrimaryPlayer));
        app.world_mut().spawn((PlayerEntity, PlayerSlot(1)));
        app.world_mut().spawn((PlayerEntity, PlayerSlot(2)));

        let mut q = app
            .world_mut()
            .query_filtered::<&PlayerSlot, With<PlayerEntity>>();
        let mut slots: Vec<u8> = q.iter(app.world()).map(|s| s.0).collect();
        slots.sort_unstable();
        assert_eq!(slots, vec![0, 1, 2]);
    }

    /// `ActivePlayerAttack::clear` zeroes the attack on its own
    /// entity without touching sibling players.
    #[test]
    fn clear_is_per_entity() {
        let mut app = App::new();
        let attack_spec = dummy_attack_spec();
        let p1 = app
            .world_mut()
            .spawn((
                PlayerEntity,
                PlayerSlot(0),
                ActivePlayerAttack(Some(crate::PlayerAttackState::new(attack_spec.clone()))),
            ))
            .id();
        let p2 = app
            .world_mut()
            .spawn((
                PlayerEntity,
                PlayerSlot(1),
                ActivePlayerAttack(Some(crate::PlayerAttackState::new(attack_spec))),
            ))
            .id();

        app.world_mut()
            .entity_mut(p1)
            .get_mut::<ActivePlayerAttack>()
            .unwrap()
            .clear();

        assert!(!app
            .world()
            .entity(p1)
            .get::<ActivePlayerAttack>()
            .unwrap()
            .is_active());
        assert!(
            app.world()
                .entity(p2)
                .get::<ActivePlayerAttack>()
                .unwrap()
                .is_active(),
            "clearing p1's attack must not touch p2's component"
        );
    }

    /// A `PlayerHealRequested` carrying `target: Some(p2)` heals p2,
    /// not the primary p1. Pins the OVERNIGHT-TODO #17.6 bridge —
    /// pickups now route heals to the player who actually overlapped
    /// the heart instead of always to primary.
    #[test]
    fn targeted_heal_routes_to_named_entity_not_primary() {
        use crate::player::{apply_player_heal_requests, PlayerHealRequested, PlayerHealth};

        let mut app = App::new();
        app.add_message::<PlayerHealRequested>();
        app.add_systems(Update, apply_player_heal_requests);

        let p1 = app
            .world_mut()
            .spawn((
                PlayerEntity,
                PlayerSlot(0),
                PrimaryPlayer,
                PlayerHealth::new(ae::Health {
                    current: 1,
                    max: 5,
                    invulnerable: false,
                }),
            ))
            .id();
        let p2 = app
            .world_mut()
            .spawn((
                PlayerEntity,
                PlayerSlot(1),
                PlayerHealth::new(ae::Health {
                    current: 1,
                    max: 5,
                    invulnerable: false,
                }),
            ))
            .id();

        app.world_mut()
            .resource_mut::<bevy::ecs::message::Messages<PlayerHealRequested>>()
            .write(PlayerHealRequested::for_target(2, p2));
        app.update();

        let p1_health = app.world().entity(p1).get::<PlayerHealth>().unwrap();
        let p2_health = app.world().entity(p2).get::<PlayerHealth>().unwrap();
        assert_eq!(p1_health.current(), 1, "primary must not pick up p2's heal");
        assert_eq!(p2_health.current(), 3, "p2 must be healed by 2");
    }

    /// `PlayerHealRequested::new` (target = None) keeps legacy
    /// behavior: heal lands on the primary player. Pins the
    /// backwards-compatible path so cutscene/quest heals don't
    /// silently break when other code starts using `for_target`.
    #[test]
    fn untargeted_heal_routes_to_primary() {
        use crate::player::{apply_player_heal_requests, PlayerHealRequested, PlayerHealth};

        let mut app = App::new();
        app.add_message::<PlayerHealRequested>();
        app.add_systems(Update, apply_player_heal_requests);

        let p1 = app
            .world_mut()
            .spawn((
                PlayerEntity,
                PlayerSlot(0),
                PrimaryPlayer,
                PlayerHealth::new(ae::Health {
                    current: 1,
                    max: 5,
                    invulnerable: false,
                }),
            ))
            .id();
        let p2 = app
            .world_mut()
            .spawn((
                PlayerEntity,
                PlayerSlot(1),
                PlayerHealth::new(ae::Health {
                    current: 1,
                    max: 5,
                    invulnerable: false,
                }),
            ))
            .id();

        app.world_mut()
            .resource_mut::<bevy::ecs::message::Messages<PlayerHealRequested>>()
            .write(PlayerHealRequested::new(3));
        app.update();

        let p1_health = app.world().entity(p1).get::<PlayerHealth>().unwrap();
        let p2_health = app.world().entity(p2).get::<PlayerHealth>().unwrap();
        assert_eq!(p1_health.current(), 4, "primary picks up untargeted heal");
        assert_eq!(p2_health.current(), 1, "p2 not touched by untargeted heal");
    }

    /// Two players each carry their own `PlayerInputFrame`; mutating
    /// one player's input frame must not propagate to the other.
    /// Pins the multiplayer-readiness invariant for the per-player
    /// input migration (OVERNIGHT-TODO #17.5) — if a future patch
    /// turns the input frame back into a `Resource`, this test will
    /// stop being meaningful and should fail loudly.
    #[test]
    fn two_players_have_independent_input_frames() {
        let mut app = App::new();
        let p1 = app
            .world_mut()
            .spawn((
                PlayerEntity,
                PlayerSlot(0),
                PrimaryPlayer,
                PlayerInputFrame::default(),
            ))
            .id();
        let p2 = app
            .world_mut()
            .spawn((PlayerEntity, PlayerSlot(1), PlayerInputFrame::default()))
            .id();

        // Mutate p1's input frame only.
        app.world_mut()
            .entity_mut(p1)
            .get_mut::<PlayerInputFrame>()
            .unwrap()
            .frame
            .interact_pressed = true;

        let p1_input = app.world().entity(p1).get::<PlayerInputFrame>().unwrap();
        let p2_input = app.world().entity(p2).get::<PlayerInputFrame>().unwrap();
        assert!(
            p1_input.frame.interact_pressed,
            "p1's input frame should reflect the just-written press"
        );
        assert!(
            !p2_input.frame.interact_pressed,
            "p2's input frame must not pick up p1's press — per-player input \
             only makes sense if the components are independent"
        );
    }
}
