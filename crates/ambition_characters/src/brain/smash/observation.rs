//! Stage 1 — observation.
//!
//! Snapshots the per-tick view the brain reads downstream. Everything
//! the modes / actions / difficulty stages need has to be here; they
//! never read the world directly. That keeps the brain testable
//! against a hand-built [`ObservationFrame`] and replayable
//! deterministically.

use ambition_engine_core as ae;

use super::super::snapshot::BrainSnapshot;

/// Per-tick read-only view of the world the brain consumes. Layout
/// stays flat (no nested Options inside Options) so the cost of
/// passing it through pure stages is a memcpy.
#[derive(Clone, Copy, Debug)]
#[allow(
    dead_code,
    reason = "every field is read by stage 2-5; rustc cross-module dead-code analysis trips over the pure-function chain"
)]
pub struct ObservationFrame {
    // --- Self ---
    pub self_pos: ae::Vec2,
    pub self_vel: ae::Vec2,
    pub self_facing: f32,
    pub self_on_ground: bool,
    /// This body is a gravity-free flyer — the brain steers 2D
    /// `velocity_target` instead of grounded locomotion + jump.
    pub self_aerial: bool,
    pub self_alive: bool,
    /// Mid-air jumps the actor has left until next landing. Reads
    /// straight off `BrainSnapshot.air_jumps_remaining`. The action
    /// stage uses this to decide whether `SpecificAction::Jump`
    /// fired in the air will actually launch a double-jump.
    pub self_air_jumps_remaining: u8,
    /// True when the actor is mid-windup / mid-active / mid-recover
    /// of an attack. Brains use this to refuse another commit until
    /// the swing finishes.
    pub self_attacking: bool,
    pub attack_cooldown_remaining: f32,
    pub stun_remaining: f32,

    // --- Target ---
    pub target_pos: ae::Vec2,
    pub target_alive: bool,
    /// Signed offset target.x - self_pos.x. Positive = target to the
    /// right. Cached so downstream stages don't recompute.
    pub to_target_x: f32,
    /// Signed offset target.y - self_pos.y. Positive = target below
    /// (engine y grows downward).
    pub to_target_y: f32,
    pub distance_to_target: f32,

    // --- Crowding (anti-clump pressure) ---
    pub crowding: CrowdingSignal,

    // --- Terrain awareness (stub today; populated when ledge data
    // surfaces in BrainSnapshot) ---
    pub terrain: TerrainAwareness,

    // --- Time ---
    pub sim_time: f32,
    pub dt: f32,
}

/// Anti-clump signal. The driver system computes this once per tick
/// per actor and feeds it through [`BrainSnapshot`]; the brain
/// stages just read it.
///
/// Two pressure components are tracked separately so the brain can
/// weigh same-faction crowding stronger than mixed-faction crowding
/// (per the design: 1-2 non-faction near is tolerable; 3+ pushes).
#[derive(Clone, Copy, Debug, Default)]
pub struct CrowdingSignal {
    /// Count of same-faction allies within crowding radius.
    pub same_faction_count: u8,
    /// Count of other-faction characters (including the player)
    /// within crowding radius.
    pub other_faction_count: u8,
    /// Unit-ish direction pointing AWAY from the centroid of
    /// nearby actors. Zero vector when nobody's around.
    pub away_dir: ae::Vec2,
    /// Aggregate pressure in `[0, 1+]`. The mode stage compares
    /// against `SmashCfg.crowding_threshold` to decide
    /// `Reposition`. Same-faction allies contribute more weight
    /// than non-faction characters; non-faction characters only
    /// start contributing at count >= 3.
    pub pressure: f32,
}

impl CrowdingSignal {
    /// Stage-aware pressure aggregation. Same-faction allies are
    /// the dominant signal; non-faction characters only start
    /// to pressure above a count of 2 (a single curious NPC or
    /// the player shouldn't make a goblin sidestep).
    ///
    /// Weight calibration: a single same-faction ally within the
    /// crowding radius already triggers `Reposition` against the
    /// default `SmashCfg::STRIKER_DEFAULT.crowding_threshold = 0.65`
    /// — without this, the 2-goblin encounter case (each actor sees
    /// only 1 nearby ally) never trips the anti-clump pressure and
    /// the pair stacks up identically on the player.
    pub fn compute_pressure(same: u8, other: u8) -> f32 {
        let same_weight = same as f32 * 0.70;
        let other_weight = if other >= 3 {
            (other as f32 - 2.0) * 0.15
        } else {
            0.0
        };
        (same_weight + other_weight).min(2.0)
    }
}

/// Stage / ledge / hazard awareness. Stubs today so the API surface
/// is locked in for the next slice — ledges + drop-offs land when
/// the snapshot builder learns about `Solid` block geometry
/// underneath the actor.
#[derive(Clone, Copy, Debug, Default)]
pub struct TerrainAwareness {
    /// True when the actor is suspended over a gap with no platform
    /// below within fall range (off-stage).
    pub off_stage: bool,
    /// Distance to the nearest stage edge (px). `f32::MAX` = no
    /// edge nearby / unknown.
    pub nearest_ledge_distance: f32,
}

/// Build an `ObservationFrame` from a `BrainSnapshot`. Pure — no
/// Bevy world access. The driver system populates the snapshot's
/// extension fields (`crowding`, eventual `terrain`); this function
/// just packs them into the flat shape downstream stages read.
pub fn observe(snap: &BrainSnapshot) -> ObservationFrame {
    let to_target = snap.target_pos - snap.actor_pos;
    let distance = to_target.length();
    let self_attacking = snap.attack_windup_remaining > 0.0
        || snap.attack_active_remaining > 0.0
        || snap.attack_recover_remaining > 0.0;
    ObservationFrame {
        self_pos: snap.actor_pos,
        self_vel: snap.actor_vel,
        self_facing: snap.actor_facing,
        self_on_ground: snap.actor_on_ground,
        self_aerial: snap.actor_aerial,
        self_alive: snap.alive,
        self_air_jumps_remaining: snap.air_jumps_remaining,
        self_attacking,
        attack_cooldown_remaining: snap.attack_cooldown_remaining,
        stun_remaining: snap.stun_remaining,
        target_pos: snap.target_pos,
        target_alive: snap.target_alive,
        to_target_x: to_target.x,
        to_target_y: to_target.y,
        distance_to_target: distance,
        crowding: snap.crowding.unwrap_or_default(),
        terrain: snap.terrain.unwrap_or_default(),
        sim_time: snap.sim_time,
        dt: snap.dt,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_pressure_single_ally_triggers_default_threshold() {
        // 1 ally → 0.70, above STRIKER_DEFAULT.crowding_threshold = 0.65.
        // This is the load-bearing case for the 2-goblin encounter
        // (each actor sees exactly one nearby ally).
        let p = CrowdingSignal::compute_pressure(1, 0);
        assert!(p > 0.65, "got {p}");
    }

    #[test]
    fn compute_pressure_two_allies_passes_default_threshold() {
        // 2 allies → 1.40, well above threshold.
        let p = CrowdingSignal::compute_pressure(2, 0);
        assert!(p > 0.65, "got {p}");
    }

    #[test]
    fn compute_pressure_non_faction_count_floor() {
        // 2 non-faction characters alone shouldn't pressure.
        assert_eq!(CrowdingSignal::compute_pressure(0, 2), 0.0);
        // 3 non-faction → starts to pressure: (3-2) * 0.15 = 0.15.
        assert!(
            (CrowdingSignal::compute_pressure(0, 3) - 0.15).abs() < f32::EPSILON,
            "got {}",
            CrowdingSignal::compute_pressure(0, 3)
        );
    }

    #[test]
    fn compute_pressure_caps_at_2_0() {
        assert!(CrowdingSignal::compute_pressure(10, 10) <= 2.0);
    }

    #[test]
    fn observe_packs_distance_correctly() {
        let mut snap = BrainSnapshot::idle();
        snap.actor_pos = ae::Vec2::new(100.0, 50.0);
        snap.target_pos = ae::Vec2::new(160.0, 130.0);
        let obs = observe(&snap);
        assert_eq!(obs.to_target_x, 60.0);
        assert_eq!(obs.to_target_y, 80.0);
        let expected = (60.0_f32 * 60.0 + 80.0 * 80.0).sqrt();
        assert!((obs.distance_to_target - expected).abs() < 1e-3);
    }

    #[test]
    fn observe_self_attacking_when_any_attack_timer_active() {
        let mut snap = BrainSnapshot::idle();
        snap.attack_windup_remaining = 0.1;
        assert!(observe(&snap).self_attacking);
        snap.attack_windup_remaining = 0.0;
        snap.attack_active_remaining = 0.05;
        assert!(observe(&snap).self_attacking);
        snap.attack_active_remaining = 0.0;
        snap.attack_recover_remaining = 0.2;
        assert!(observe(&snap).self_attacking);
        snap.attack_recover_remaining = 0.0;
        assert!(!observe(&snap).self_attacking);
    }
}
