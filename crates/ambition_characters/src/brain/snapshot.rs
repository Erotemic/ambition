//! `BrainSnapshot` — the read-only view a brain consumes each tick.
//!
//! The snapshot is what every brain backend (player, state-machine,
//! and eventually scripted / remote / RL) sees. Brains write into a
//! mutable [`crate::actor::control::ActorControlFrame`]; the snapshot stays immutable
//! per-tick so the same brain function is safe to call against a
//! single set of inputs (deterministic for tests + replay).
//!
//! Fields are organized by who fills them:
//!
//! - Actor self: position, velocity, facing, ground contact —
//!   read off the actor's own ECS components by the brain-driver
//!   system.
//! - Combat timers: cooldown / windup / active / recover / stun.
//!   Mirror of [`crate::actor::ai::CharacterAiSnapshot`] fields so existing pure
//!   evaluators slot in unchanged.
//! - Target: the actor's current "look at" target (player for
//!   most NPCs/enemies; some bosses target a specific anchor). Filled
//!   from `ActorTarget` per the player-singleton audit.
//! - Per-template inputs: surfaced as `Option`s. The `Wanderer`
//!   brain needs wall-contact info; nobody else does. Construct the
//!   snapshot with these set only when the relevant brain wants them.
//!
//! Add new fields by name when a new brain template needs them; don't grow this into a pile of
//! `Option<…>`s without a real consumer.

use ambition_platformer2d_core as ae;

/// Immutable per-tick brain input. Brains write decisions to
/// `ActorControlFrame`; body-derived facts stay on this single read channel.
#[derive(Clone, Debug)]
pub struct BrainSnapshot {
    /// The capture relationship, as three plain facts. Resolved by the phase
    /// that holds the queries and handed down; a brain never reaches for
    /// `CapturedBy` itself. `pummels_landed` is `0` unless `holding_captive`.
    pub captured: bool,
    /// How long this body has been held, in scaled seconds; `0.0` when
    /// free. A brain's only handle on the hold's progress — and the reason a
    /// captive's struggle can have a CADENCE without the brain keeping a timer
    /// of its own, which would be one more thing a rewind has to restore.
    pub captured_for: f32,
    pub holding_captive: bool,
    pub pummels_landed: u8,
    /// Actor's current world position (px).
    pub actor_pos: ae::Vec2,
    /// Actor's current velocity (px/s).
    pub actor_vel: ae::Vec2,
    /// Actor's current facing: +1 local-right, -1 local-left.
    pub actor_facing: f32,
    /// Direction that defines the controlled actor's local down for human-input
    /// interpretation this tick. Defaults to ordinary screen-down so AI/test
    /// snapshots that do not care about human control remain inert.
    pub control_down: ae::Vec2,
    /// Policy for mapping the raw LOCOMOTION stick into this actor's local frame.
    pub movement_frame_mode: ae::InputFrameMode,
    /// Policy for mapping raw PRECISION-AIM input (blink steer, fire aim) into this
    /// actor's local frame. Defaults to screen-directed via [`ae::ControlFrameModes`].
    pub aim_frame_mode: ae::InputFrameMode,
    /// Whether the actor is grounded (touching a `Solid` / `OneWay`
    /// floor this tick).
    pub actor_on_ground: bool,
    /// Local-side normal of the body's current semantic wall contact. `-1` means
    /// a wall on local-right (its outward normal pushes left), `+1` a wall on
    /// local-left. This is a collision FACT; brains may decide what to do with it.
    pub side_contact_normal: Option<f32>,
    /// Autonomous steering preference authored for simple walkers. Consumed by
    /// Patrol/Wanderer brains, never by movement integration or human control.
    pub turns_at_walls: bool,
    /// Whether this body is a gravity-free free-mover (a flyer: enemy
    /// `is_aerial` / `gravity_scale == 0`, or a `Floating` NPC). When true the
    /// brain steers in 2D via `velocity_target` instead of grounded
    /// `locomotion` + jump. Body-derived truth, populated by the snapshot
    /// builder from the body's gravity scale. Defaults `false` so grounded
    /// brains and inert test snapshots are unaffected.
    pub actor_aerial: bool,
    /// The attacks this body can actually throw (FB4b §13.2).
    ///
    /// L2 scores real moves with real frame data, and the brain cannot reach a
    /// moveset: `ambition_combat` depends on `ambition_characters`, not the
    /// reverse. So the actors-side snapshot builder fills this from the body's
    /// live `ActorMoveset` — body-derived truth arriving through the world-in
    /// port, exactly like [`Self::actor_aerial`].
    ///
    /// Empty by default, which is the honest answer for every non-fighter brain
    /// and for an inert test snapshot: `generate_options` then produces no
    /// attacks and the fighter plays movement only.
    pub attack_kit: Vec<crate::brain::attack_kit::AttackCandidate>,
    /// Which body this is, as the integration layer names it.
    ///
    /// The brain genuinely cannot know: a snapshot is pure body state, and a
    /// body's identity is the host's to assign. So it arrives through the
    /// world-in port like [`Self::attack_kit`] and [`Self::actor_aerial`] — the
    /// snapshot builder knows the id, the brain is merely told it.
    ///
    /// It exists so a published fact can name its SUBJECT. An explanation of
    /// "why did this fighter walk off the stage" is worthless with two fighters
    /// on the stage and no way to tell whose decision is whose.
    ///
    /// read by the instrument, never by a decision. A brain that branched
    /// on its own id would be a brain that behaves differently depending on
    /// which body it woke up in, and every no-cheat property this crate argues
    /// for would be void. `None` — the default, and the honest answer for a
    /// test fixture — publishes an unattributed fact rather than inventing one.
    pub subject: Option<String>,
    /// Whether the actor is alive. State-machine brain templates
    /// emit a neutral frame when `alive == false`; the player brain
    /// (`tick_player_brain`) currently doesn't gate on this — dead
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

    /// This actor's own health as a fraction of its max in `[0, 1]`. The Smash
    /// brain watches this for DROPS (damage taken) to trigger a regroup — it backs
    /// off and resets after taking a beating instead of trading forever. `1.0`
    /// (full health) for snapshots that don't care.
    pub health_fraction: f32,

    /// Sim time at the start of this tick (seconds, scaled clock).
    pub sim_time: f32,
    /// Scaled dt for this tick (seconds).
    pub dt: f32,

    /// The controlled body's ground-run capability in px/s — "the fastest this
    /// character can run". AI brains that think in absolute speeds turn that into
    /// normalized intent with [`Self::locomotion_for`], so any per-spawn speed
    /// jitter rides along *as intent* rather than as a varying capability. The
    /// integration half scales back by the same capability, so velocity is exact
    /// without the simulation ever branching on actor type. Player-style brains
    /// write an already-normalized stick and ignore this.
    pub max_run_speed: f32,

    /// The movement law this body actually plays under, for brains that
    /// PREDICT rather than steer.
    ///
    /// Body-derived truth arriving through the world-in port, exactly like
    /// [`Self::actor_aerial`] and [`Self::attack_kit`].
    ///
    /// this is NOT a second [`Self::max_run_speed`], and the two answer
    /// different questions. `max_run_speed` is the throttle scale the caller
    /// wants this body's locomotion intent expressed against — deliberately `0`
    /// for a body whose integrator ignores it, and a boss's flight speed for a
    /// body that flies. This is what the body's own tuning says, for predicting
    /// where it will BE.
    ///
    /// `None` — the default — means "no authored law reached this snapshot", and
    /// a predictor falls back to the engine's canonical defaults.
    pub movement_tuning: Option<ae::MovementTuning>,

    /// Which movement VERBS this body owns, beside the law that says what
    /// they are worth.
    ///
    /// The pair is what the movement kernel needs to be driven at all, and a
    /// brain that wants to know *"could I still get back from there"* asks the
    /// kernel rather than answering it — see
    /// [`crate::brain::fighter::recovery::RecoveryLens`]. A body with an unspent
    /// air jump, a wall it can cling to or a ledge it can catch gets a different
    /// answer from the same position, and no list in the brain has to be kept in
    /// step for that to be true.
    ///
    /// not a capability list for the brain to interpret. [`Self::actor_aerial`]
    /// and `SelfView`'s `burst` exist precisely because a driver re-deriving the
    /// kernel's precedence rules is the failure mode; this field is never read to
    /// decide what to press, only handed to the kernel that owns the question.
    ///
    /// `None` — the default — means no kit reached this snapshot, and every
    /// consumer degrades to not asking.
    pub abilities: Option<ae::AbilitySet>,

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

    // --- BossPattern inputs ---
    // The three fields a `BossPattern` brain needs beyond the shared
    // `actor_pos`/`target_pos`/`dt`: its encounter phase, the world bounds for the
    // soft movement clamp, and the front-wall probe. Filled by the boss tick
    // system; `None`/`ZERO` for every non-boss body, which no other brain reads.
    /// Boss encounter phase this tick (drives pattern selection + the
    /// `is_attacking()` gate). `None` for non-boss bodies.
    pub boss_encounter_phase: Option<crate::brain::boss_pattern::BossEncounterPhase>,
    /// World size (px) for the BossPattern movement soft-clamp. `ZERO` for
    /// non-bosses (the clamp is inert at zero extent).
    pub world_size: ae::Vec2,
    /// Distance to the first blocking wall in the boss's approach lane; `None` =
    /// clear (or non-boss).
    pub front_wall_clearance: Option<f32>,
    /// Per-tick input snapshot for [`crate::brain::tick_player_brain`].
    /// `None` for non-player actors. The player-brain driver fills this directly
    /// from the [`SlotControls`](crate::control::SlotControls) entry named by the
    /// body's `DrivingParticipant(slot)`; no input frame is copied onto the entity.
    pub player_input: Option<ambition_platformer2d_core::ControlFrame>,

    /// Per-tick crowding signal — same-faction + non-faction
    /// nearby-actor counts, the averaged "away" direction, and
    /// aggregate pressure. Smash uses this for brawler spacing;
    /// flying state-machine brains use it to avoid stacking in
    /// the air.
    pub crowding: Option<crate::brain::smash::CrowdingSignal>,
    /// `0` = no double-jump available (must land first). The Smash brain reads this so an
    /// airborne actor can commit a follow-up jump to chase a high target. Non-jumping brains
    /// can leave this at the default `0`.
    pub air_jumps_remaining: u8,
    /// Per-tick stage / ledge / hazard awareness. `None` for brains
    /// that don't consult terrain. Stub today; populated when the
    /// snapshot builder learns about stage geometry under the
    /// actor.
    pub terrain: Option<crate::brain::smash::TerrainAwareness>,
}

impl BrainSnapshot {
    /// Build a minimal snapshot — useful for tests where most fields
    /// are inert. Callers can `..BrainSnapshot::idle()` and override
    /// the fields that matter for the test.
    pub fn idle() -> Self {
        Self {
            captured: false,
            captured_for: 0.0,
            holding_captive: false,
            pummels_landed: 0,
            actor_pos: ae::Vec2::ZERO,
            actor_vel: ae::Vec2::ZERO,
            actor_facing: 1.0,
            control_down: ae::Vec2::new(0.0, 1.0),
            movement_frame_mode: ae::ControlFrameModes::default().movement,
            aim_frame_mode: ae::ControlFrameModes::default().aim,
            actor_on_ground: true,
            side_contact_normal: None,
            turns_at_walls: false,
            actor_aerial: false,
            attack_kit: Vec::new(),
            subject: None,
            alive: true,
            target_pos: ae::Vec2::ZERO,
            target_alive: true,
            health_fraction: 1.0,
            sim_time: 0.0,
            dt: 1.0 / 60.0,
            max_run_speed: 120.0,
            movement_tuning: None,
            abilities: None,
            attack_cooldown_remaining: 0.0,
            attack_windup_remaining: 0.0,
            attack_active_remaining: 0.0,
            attack_recover_remaining: 0.0,
            stun_remaining: 0.0,
            boss_encounter_phase: None,
            world_size: ae::Vec2::ZERO,
            front_wall_clearance: None,
            player_input: None,
            crowding: None,
            terrain: None,
            air_jumps_remaining: 0,
        }
    }

    /// Acceleration frame that defines this actor's local side/down axes.
    pub fn acceleration_frame(&self) -> ae::AccelerationFrame {
        ae::AccelerationFrame::new(self.control_down)
    }

    /// Turn a desired *local* velocity (px/s, body-local axes) into normalized
    /// locomotion intent for [`crate::actor::control::ActorControlFrame::locomotion`]:
    /// `desired / max_run_speed`, clamped-safe against a zero capability. This is
    /// how a brain that reasons in absolute speeds (patrol/chase, with per-spawn
    /// jitter) expresses intent so the integrator can scale it back by the same
    /// capability — no actor-type branch downstream.
    pub fn locomotion_for(&self, desired_local_velocity: ae::LocalAxes) -> ae::LocalAxes {
        if self.max_run_speed > 1e-3 {
            desired_local_velocity / self.max_run_speed
        } else {
            ae::LocalAxes::ZERO
        }
    }

    /// Vector from the actor to its current target in actor-local coordinates.
    /// `x` is local side/right; `y` is toward the actor's feet/down.
    pub fn target_delta_local(&self) -> ae::LocalAxes {
        ae::LocalAxes::from_vec(
            self.acceleration_frame()
                .to_local(self.target_pos - self.actor_pos),
        )
    }

    /// Actor velocity in actor-local coordinates. Brains that make body-relative
    /// movement decisions should prefer this over reading world `x/y` directly.
    pub fn actor_vel_local(&self) -> ae::LocalAxes {
        ae::LocalAxes::from_vec(self.acceleration_frame().to_local(self.actor_vel))
    }

    /// Build the engine-side AI snapshot from this brain snapshot
    /// plus per-template aggro/attack ranges. The state-machine
    /// brain templates use the existing
    /// [`crate::actor::ai::evaluate_character_ai_output`] for their idle / patrol
    /// / chase / attack decisions; this helper threads the fields
    /// through without copy-pasting in each tick fn.
    pub fn to_character_ai_snapshot(
        &self,
        aggro_radius: f32,
        attack_range: f32,
        patrol_enabled: bool,
    ) -> crate::actor::ai::CharacterAiSnapshot {
        // The low-level evaluator only needs distance and `x` sign. Feed it the
        // actor-relative target vector so `direction_side` means local side/right,
        // not raw world X. Direct tests of `CharacterAiSnapshot` can still pass
        // world-like coordinates; the brain seam normalizes live actors here.
        crate::actor::ai::CharacterAiSnapshot {
            actor_pos: ae::Vec2::ZERO,
            player_pos: self.target_delta_local().vec(),
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
        assert!(s.alive);
        assert!(s.player_input.is_none(), "idle snapshot has no input");
        assert_eq!(s.control_down, ae::Vec2::new(0.0, 1.0));
        assert_eq!(s.movement_frame_mode, ae::InputFrameMode::ScreenRelative);
        assert_eq!(s.aim_frame_mode, ae::InputFrameMode::ScreenRelative);
    }

    #[test]
    fn snapshot_player_input_roundtrips() {
        // Snapshot must round-trip player_input correctly so the
        // player brain reads the same values the driver set.
        let mut input = ambition_platformer2d_core::ControlFrame::default();
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

    #[test]
    fn local_snapshot_vectors_are_c4_equivalent() {
        let local_target = ae::Vec2::new(80.0, -24.0);
        let local_vel = ae::Vec2::new(-12.0, 33.0);
        for down in [
            ae::Vec2::new(0.0, 1.0),
            ae::Vec2::new(1.0, 0.0),
            ae::Vec2::new(0.0, -1.0),
            ae::Vec2::new(-1.0, 0.0),
        ] {
            let frame = ae::AccelerationFrame::new(down);
            let mut s = BrainSnapshot::idle();
            s.control_down = down;
            s.actor_pos = ae::Vec2::new(100.0, 200.0);
            s.target_pos = s.actor_pos + frame.to_world(local_target);
            s.actor_vel = frame.to_world(local_vel);
            assert_eq!(
                s.target_delta_local(),
                ae::LocalAxes::from_vec(local_target)
            );
            assert_eq!(s.actor_vel_local(), ae::LocalAxes::from_vec(local_vel));
        }
    }
}
