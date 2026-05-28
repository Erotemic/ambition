//! `BrainSnapshot` — the read-only view a brain consumes each tick.
//!
//! The snapshot is what every brain backend (player, state-machine,
//! and eventually scripted / remote / RL) sees. Brains write into a
//! mutable [`crate::actor_control::ActorControlFrame`]; the snapshot stays immutable
//! per-tick so the same brain function is safe to call against a
//! single set of inputs (deterministic for tests + replay).
//!
//! Fields are organized by who fills them:
//!
//! - **Actor self**: position, velocity, facing, ground contact —
//!   read off the actor's own ECS components by the brain-driver
//!   system.
//! - **Combat timers**: cooldown / windup / active / recover / stun.
//!   Mirror of [`crate::character_ai::CharacterAiSnapshot`] fields so existing pure
//!   evaluators slot in unchanged.
//! - **Target**: the actor's current "look at" target (player for
//!   most NPCs/enemies; some bosses target a specific anchor). Filled
//!   from `ActorTarget` per the player-singleton audit.
//! - **Per-template inputs**: surfaced as `Option`s. The `Wanderer`
//!   brain needs wall-contact info; nobody else does. Construct the
//!   snapshot with these set only when the relevant brain wants them.
//!
//! Construction is cheap and explicit — there's no builder pattern
//! because the field set is fixed and small. Add new fields by name
//! when a new brain template needs them; don't grow this into a
//! pile of `Option<…>`s without a real consumer.

use crate::engine_core as ae;

/// What a brain sees this tick. Read-only; brains never mutate the
/// snapshot (they write to `&mut ActorControlFrame` instead).
#[derive(Clone, Copy, Debug)]
pub struct BrainSnapshot {
    /// Actor's current world position (px).
    pub actor_pos: ae::Vec2,
    /// Actor's current velocity (px/s).
    pub actor_vel: ae::Vec2,
    /// Actor's current facing: +1 right, -1 left.
    pub actor_facing: f32,
    /// Whether the actor is grounded (touching a `Solid` / `OneWay`
    /// floor this tick).
    pub actor_on_ground: bool,
    /// Whether the actor is alive. State-machine brain templates
    /// emit a neutral frame when `alive == false`; the player brain
    /// (`Brain::Player`) currently doesn't gate on this — dead
    /// players still translate their input. The integration layer
    /// ignores dead actors regardless.
    pub alive: bool,

    /// Position the actor is "interested in" — typically the player,
    /// but may be a boss anchor, a scripted destination, or the
    /// actor's own position when there's no target.
    pub target_pos: ae::Vec2,
    /// Whether the target is alive. Dead targets typically demote the
    /// brain to Idle/Patrol.
    pub target_alive: bool,

    /// Sim time at the start of this tick (seconds, scaled clock).
    pub sim_time: f32,
    /// Scaled dt for this tick (seconds).
    pub dt: f32,

    // --- Combat timers ---
    /// Cooldown remaining before this actor may begin another attack.
    pub attack_cooldown_remaining: f32,
    /// Time remaining in an active attack windup.
    pub attack_windup_remaining: f32,
    /// Time remaining in an active attack hit window.
    pub attack_active_remaining: f32,
    /// Time remaining in post-attack recovery.
    pub attack_recover_remaining: f32,
    /// Stun remaining (e.g. from a parry / pogo).
    pub stun_remaining: f32,

    // --- Per-template inputs ---
    /// Wall contact this tick. `None` = clear path. The brain-driver
    /// system computes this only for brains that care (currently
    /// `Wanderer`); other actors leave it `None`.
    pub wall_contact: Option<WallContact>,
    /// Per-tick input snapshot for [`crate::brain::Brain::Player`].
    /// `None` for non-player actors. The player-brain-driver system
    /// fills this from the actor entity's `PlayerInputFrame`; the
    /// player brain reads it to populate jump / dash / fire / etc.
    /// edges of the control frame.
    pub player_input: Option<crate::input::ControlFrame>,

    /// Per-tick crowding signal — same-faction + non-faction
    /// nearby-actor counts, the averaged "away" direction, and
    /// aggregate pressure. `None` for brains that don't consult
    /// crowding (today: only the Smash brain). The driver system
    /// only computes this for actors whose brain requests it.
    pub crowding: Option<crate::brain::smash::CrowdingSignal>,
    /// Mid-air jumps the actor has remaining until next landing.
    /// `0` = no double-jump available (must land first). The
    /// Smash brain reads this so an airborne actor can commit a
    /// follow-up jump to chase a high target. Non-jumping brains
    /// can leave this at the default `0`.
    pub air_jumps_remaining: u8,
    /// Per-tick stage / ledge / hazard awareness. `None` for brains
    /// that don't consult terrain. Stub today; populated when the
    /// snapshot builder learns about stage geometry under the
    /// actor.
    pub terrain: Option<crate::brain::smash::TerrainAwareness>,
}

/// Info about a wall the actor pressed against this tick.
#[derive(Clone, Copy, Debug)]
pub struct WallContact {
    /// Outward-pointing normal of the wall surface in actor-local
    /// terms. `(-1, 0)` = wall to the right (actor moving right hit
    /// it); `(+1, 0)` = wall to the left. Same convention as
    /// `EnemyRuntime::surface_normal`.
    pub normal: ae::Vec2,
    /// Whether the wall is climbable (has a `Climbable` overlap or
    /// is a non-solid surface). Drives the `Wanderer` climb-vs-
    /// reverse decision.
    pub is_climbable: bool,
}

impl BrainSnapshot {
    /// Build a minimal snapshot — useful for tests where most fields
    /// are inert. Callers can `..BrainSnapshot::idle()` and override
    /// the fields that matter for the test.
    pub fn idle() -> Self {
        Self {
            actor_pos: ae::Vec2::ZERO,
            actor_vel: ae::Vec2::ZERO,
            actor_facing: 1.0,
            actor_on_ground: true,
            alive: true,
            target_pos: ae::Vec2::ZERO,
            target_alive: true,
            sim_time: 0.0,
            dt: 1.0 / 60.0,
            attack_cooldown_remaining: 0.0,
            attack_windup_remaining: 0.0,
            attack_active_remaining: 0.0,
            attack_recover_remaining: 0.0,
            stun_remaining: 0.0,
            wall_contact: None,
            player_input: None,
            crowding: None,
            terrain: None,
            air_jumps_remaining: 0,
        }
    }

    /// Build the engine-side AI snapshot from this brain snapshot
    /// plus per-template aggro/attack ranges. The state-machine
    /// brain templates use the existing
    /// [`crate::character_ai::evaluate_character_ai_output`] for their idle / patrol
    /// / chase / attack decisions; this helper threads the fields
    /// through without copy-pasting in each tick fn.
    pub fn to_character_ai_snapshot(
        self,
        aggro_radius: f32,
        attack_range: f32,
        patrol_enabled: bool,
    ) -> crate::character_ai::CharacterAiSnapshot {
        crate::character_ai::CharacterAiSnapshot {
            actor_pos: self.actor_pos,
            player_pos: self.target_pos,
            aggro_radius,
            attack_range,
            attack_windup_remaining: self.attack_windup_remaining,
            attack_active_remaining: self.attack_active_remaining,
            attack_recover_remaining: self.attack_recover_remaining,
            stun_remaining: self.stun_remaining,
            alive: self.alive,
            patrol_enabled,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_snapshot_has_inert_timers() {
        let s = BrainSnapshot::idle();
        assert_eq!(s.attack_cooldown_remaining, 0.0);
        assert_eq!(s.attack_windup_remaining, 0.0);
        assert_eq!(s.attack_active_remaining, 0.0);
        assert_eq!(s.attack_recover_remaining, 0.0);
        assert_eq!(s.stun_remaining, 0.0);
        assert!(s.wall_contact.is_none());
        assert!(s.alive);
        assert!(s.player_input.is_none(), "idle snapshot has no input");
    }

    #[test]
    fn snapshot_player_input_roundtrips() {
        // Snapshot must round-trip player_input correctly so the
        // player brain reads the same values the driver set.
        let mut input = crate::input::ControlFrame::default();
        input.axis_x = 0.6;
        input.jump_pressed = true;
        let mut snap = BrainSnapshot::idle();
        snap.player_input = Some(input);
        let extracted = snap.player_input.expect("player_input set");
        assert_eq!(extracted.axis_x, 0.6);
        assert!(extracted.jump_pressed);
    }

    #[test]
    fn to_character_ai_snapshot_handles_negative_inputs_without_panic() {
        // Defensive: negative aggro / attack ranges should pass
        // through to the engine evaluator (which clamps via
        // .max(0.0)). Pin that the helper doesn't try to
        // pre-validate or panic.
        let s = BrainSnapshot::idle();
        let ai = s.to_character_ai_snapshot(-10.0, -5.0, false);
        assert_eq!(ai.aggro_radius, -10.0);
        assert_eq!(ai.attack_range, -5.0);
        assert!(!ai.patrol_enabled);
    }

    #[test]
    fn to_character_ai_snapshot_threads_timers() {
        let mut s = BrainSnapshot::idle();
        s.attack_windup_remaining = 0.25;
        s.attack_recover_remaining = 0.1;
        s.stun_remaining = 0.5;
        let ai = s.to_character_ai_snapshot(100.0, 24.0, true);
        assert_eq!(ai.attack_windup_remaining, 0.25);
        assert_eq!(ai.attack_recover_remaining, 0.1);
        assert_eq!(ai.stun_remaining, 0.5);
        assert_eq!(ai.aggro_radius, 100.0);
        assert_eq!(ai.attack_range, 24.0);
        assert!(ai.patrol_enabled);
    }
}
