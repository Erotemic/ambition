//! ADR 0010 — time-control authority as data.
//!
//! Gameplay code never mutates [`crate::SandboxSimState::time_scale`]
//! (or any future per-domain clock) directly. Instead it writes a
//! [`ClockScaleRequest`] message naming the [`ClockDomain`] it wants
//! to affect, the requested `scale`, and the [`ClockRequester`] doing
//! the asking. [`apply_clock_scale_requests`] consults the active
//! [`RegimePolicy`] and either grants, denies, rebinds, or broadcasts
//! the request.
//!
//! In the default [`Regime::Solo`] regime — single-player —
//! every requester is granted. The shape of the dispatch is the same
//! as it will be in CoopConsensual / Competitive / RLDeterministic
//! regimes; what changes is only the policy table.
//!
//! See ADR 0010 §Vocabulary, ADR 0011 §Two time-control operations.

use bevy::prelude::*;

use crate::{ClockDomain, SandboxSimState};
use crate::feel::SandboxFeelTuning;
use crate::player::components::{
    PlayerCombatState, PlayerMovementAuthority, PlayerSlot, PrimaryPlayer,
};
use crate::SandboxDevState;

/// ADR 0011 — per-entity proper-time scale.
///
/// An entity with `ProperTimeScale(2.0)` ticks at twice the world's
/// sim rate; an entity with `ProperTimeScale(0.5)` ticks at half.
/// Default (and the SP-everywhere assumption today) is `1.0`, in
/// which case [`crate::WorldTime::entity_dt`] returns sim_dt
/// unchanged.
///
/// This is the seam for two future mechanics:
///
/// - **MP bullet-time** (ADR 0011 §Two time-control operations) —
///   one player boosts their proper time without slowing other
///   players' clocks. `BoostEntityProperTime(p, factor)` sets this
///   component on the player entity.
/// - **Special relativity** (ADR 0011 §Galilean→SR ladder) — a
///   future room metric computes this from velocity via the Lorentz
///   factor `γ(v) = 1 / √(1 − v²/c²)`. The integrator already reads
///   per-entity proper-time scale, so adding SR is a data change.
///
/// Most entities never carry this component; `entity_dt` defaults
/// to `ProperTimeScale::ONE` (i.e., `sim_dt`) when it's missing so
/// the change is invisible until something opts in.
#[derive(Component, Copy, Clone, Debug, PartialEq)]
pub struct ProperTimeScale(pub f32);

impl Default for ProperTimeScale {
    fn default() -> Self {
        Self::ONE
    }
}

impl ProperTimeScale {
    /// The everywhere-default scale. Returned by lookups for entities
    /// that have no [`ProperTimeScale`] component.
    pub const ONE: ProperTimeScale = ProperTimeScale(1.0);

    /// Read the scalar value.
    pub fn value(self) -> f32 {
        self.0
    }

    /// Resolve an `Option<&ProperTimeScale>` lookup, defaulting to
    /// [`Self::ONE`] when missing. Convenience for animator + AI
    /// systems that query `Option<&ProperTimeScale>` so they don't
    /// require every entity to carry the component.
    pub fn or_default(opt: Option<&ProperTimeScale>) -> ProperTimeScale {
        opt.copied().unwrap_or(Self::ONE)
    }
}

/// Who is asking for a clock change. Encoded as data so a policy
/// table can grant/deny based on identity without hard-coding which
/// systems are allowed to touch which clocks.
///
/// ADR 0010 calls this the `requester` field of `ClockScaleRequest`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ClockRequester {
    /// A player ability (today: bullet-time blink) or future player-
    /// triggered time mechanic.
    Player(PlayerSlot),
    /// Developer tools — keyboard slowmo toggle, inspector overrides.
    DevTool,
    /// Scripted cutscene / quest / encounter — narrative authority.
    Scripted,
    /// The engine itself — game-mode pause / suspended-gameplay zeroing.
    Engine,
    /// A boss (or other in-world entity) that has been granted time
    /// authority by a room-scoped policy override. ADR 0010 §Narrative
    /// authority: "the boss got root on the simulator."
    Boss,
}

/// What the policy lets a requester do with a domain.
///
/// `Grant` writes the request through. `Deny` drops it. `Rebind`
/// rewrites the request to a different domain (e.g., a CoopConsensual
/// regime might rebind a player's `SimClock` request onto their own
/// `PlayerClock`). `Broadcast` applies the request to every domain in
/// scope (e.g., CoopConsensual sharing a player's bullet-time across
/// all PlayerClocks).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Permission {
    Grant,
    Deny,
    Rebind(ClockDomain),
    Broadcast,
}

/// The active permission table. ADR 0010 §Regimes — adding a regime
/// is a data change, not a code change.
///
/// `Solo` is the SP default: permissive, every request granted.
/// `RLDeterministic` denies all clock-scale requests so training
/// runs and CI use a fixed timestep. `Cinematic` defers player
/// requests while scripted authority holds; useful during cutscenes.
///
/// Future: `CoopConsensual` and `Competitive` (ADR 0010 §Regimes).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Regime {
    Solo,
    RLDeterministic,
    Cinematic,
}

/// Resource carrying the active [`Regime`]. The default is `Solo`
/// — SP is what falls out of the most permissive policy.
#[derive(Resource, Copy, Clone, Debug)]
pub struct RegimePolicy {
    pub regime: Regime,
}

impl Default for RegimePolicy {
    fn default() -> Self {
        Self { regime: Regime::Solo }
    }
}

impl RegimePolicy {
    /// Look up the permission for a given `(requester, domain)`
    /// pair under the active regime. Pure function — no resource
    /// access — so the policy table is unit-testable.
    pub fn permission_for(self, requester: ClockRequester, _domain: ClockDomain) -> Permission {
        match self.regime {
            Regime::Solo => Permission::Grant,
            Regime::RLDeterministic => Permission::Deny,
            Regime::Cinematic => match requester {
                ClockRequester::Scripted | ClockRequester::Engine => Permission::Grant,
                ClockRequester::Player(_) | ClockRequester::DevTool | ClockRequester::Boss => {
                    Permission::Deny
                }
            },
        }
    }
}

/// A request to scale a named clock. Written by gameplay systems
/// that want to bend time (bullet-time, hitstop, cutscene pause,
/// boss freeze) instead of mutating [`SandboxSimState::time_scale`]
/// directly. Consumed by [`apply_clock_scale_requests`].
///
/// `reason` is a short static label for telemetry and debug overlays
/// — keep it grep-able ("bullet_blink", "hitstop", "cinematic_freeze").
#[derive(Message, Copy, Clone, Debug)]
pub struct ClockScaleRequest {
    pub domain: ClockDomain,
    pub scale: f32,
    pub requester: ClockRequester,
    pub reason: &'static str,
}

/// Target scale per-domain — the value [`SandboxSimState::time_scale`]
/// is currently smoothing toward. Written by [`apply_clock_scale_requests`]
/// (the policy-aware sink of [`ClockScaleRequest`] messages) and read
/// by [`smooth_sim_clock_toward_target_system`] (the per-frame ramp).
///
/// Decoupling target from current keeps the message system orthogonal
/// to feel: a one-shot request flips the target; the smoother takes
/// the next N frames to slide there at feel-tuned rates. Both can
/// land in the same frame for snap behavior, or be split for
/// cinematic ramps.
#[derive(Resource, Clone, Copy, Debug)]
pub struct RequestedClockScale {
    /// SimClock target. Default 1.0 (real-time pace). PlayerClock
    /// grants currently collapse onto this field (SP only has one
    /// player); per-player clocks are added when MP regimes land.
    pub sim_clock: f32,
}

impl Default for RequestedClockScale {
    fn default() -> Self {
        Self { sim_clock: 1.0 }
    }
}

/// Drain pending [`ClockScaleRequest`] messages, run them through
/// the active [`RegimePolicy`], and store the granted scales in
/// [`RequestedClockScale`].
///
/// This system DOES NOT mutate [`SandboxSimState::time_scale`]
/// directly — that's the smoother's job. The split exists so the
/// policy table sees every requester (auditability, telemetry) and
/// so multiple per-frame requesters can land in a sensible order
/// before the smoother converts the resulting target into the
/// current frame's time_scale.
///
/// Multiple requests in one frame: later requests win for the same
/// domain. SP wins on whatever the last grant says; deterministic
/// because the message order is the schedule order.
pub fn apply_clock_scale_requests(
    mut requests: MessageReader<ClockScaleRequest>,
    policy: Res<RegimePolicy>,
    mut target: ResMut<RequestedClockScale>,
) {
    for req in requests.read() {
        let permission = policy.permission_for(req.requester, req.domain);
        match permission {
            Permission::Grant => write_target(&mut target, req.domain, req.scale),
            Permission::Deny => continue,
            Permission::Rebind(other) => write_target(&mut target, other, req.scale),
            Permission::Broadcast => {
                // SP today has one player + one sim clock; broadcast
                // collapses to a SimClock write. CoopConsensual will
                // fan out to every PlayerClock here.
                write_target(&mut target, ClockDomain::SimClock, req.scale);
            }
        }
    }
}

fn write_target(target: &mut RequestedClockScale, domain: ClockDomain, scale: f32) {
    match domain {
        ClockDomain::SimClock => target.sim_clock = scale,
        // ADR 0011 §Two time-control operations — in SP the
        // "boost-player-proper-time" path collapses onto SimClock
        // (one observer, one frame). When MP lands, this arm
        // diverges into per-PlayerClock targets.
        ClockDomain::PlayerClock(_) => target.sim_clock = scale,
        ClockDomain::WallClock => { /* wall clock is never scaled */ }
    }
}

/// Read the primary player's state and the dev tools, decide what
/// SimClock scale should be in effect this frame, and fire one
/// [`ClockScaleRequest`].
///
/// Priority matches the historic [`crate::update_time_scale`]
/// ladder so behavior is preserved:
///
/// 1. hitstop active → 0.0   (Engine requester — the engine took
///    authority on the player's behalf)
/// 2. blink aiming → bullet_time_scale  (Player requester —
///    the "bullet_blink" verb)
/// 3. blink hold active → blink_hold_slow_scale  (Player —
///    "blink_hold_slow")
/// 4. dev_state.slowmo → debug_slowmo_scale  (DevTool —
///    inspector-driven)
/// 5. otherwise → 1.0  (Engine — restoring real-time pace)
///
/// ADR 0011 §"Two time-control operations" note: in SP, the
/// "slow sim" (Operation 1) and "boost player proper time"
/// (Operation 2) are observationally equivalent for one observer.
/// We implement Operation 1 here because it's the simpler write.
/// Step 3's per-entity `ProperTimeScale` component + `entity_dt`
/// accessor are the seam where future MP / RL regimes diverge.
pub fn emit_player_time_intent_system(
    primary: Query<(&PlayerMovementAuthority, &PlayerCombatState), With<PrimaryPlayer>>,
    dev_state: Res<SandboxDevState>,
    feel: Res<SandboxFeelTuning>,
    mut writer: MessageWriter<ClockScaleRequest>,
) {
    let Ok((authority, combat)) = primary.single() else { return };
    let player = &authority.player;
    let (scale, requester, reason) = if combat.hitstop_timer > 0.0 {
        (0.0, ClockRequester::Engine, "hitstop")
    } else if player.blink_aiming {
        (
            feel.bullet_time_scale,
            ClockRequester::Player(PlayerSlot::PRIMARY),
            "bullet_blink",
        )
    } else if player.blink_hold_active {
        (
            feel.blink_hold_slow_scale,
            ClockRequester::Player(PlayerSlot::PRIMARY),
            "blink_hold_slow",
        )
    } else if dev_state.slowmo {
        (feel.debug_slowmo_scale, ClockRequester::DevTool, "dev_slowmo")
    } else {
        (1.0, ClockRequester::Engine, "default")
    };
    writer.write(ClockScaleRequest {
        domain: ClockDomain::SimClock,
        scale,
        requester,
        reason,
    });
}

/// Smooth [`SandboxSimState::time_scale`] toward
/// [`RequestedClockScale::sim_clock`] at feel-tuned rates.
///
/// Replaces the imperative `crate::update_time_scale` helper. The
/// asymmetric ramp (`time_ramp_down_rate` when decelerating,
/// `time_ramp_up_rate` when accelerating) preserves the "snap into
/// bullet-time, breathe back to normal" feel the imperative version
/// shipped.
pub fn smooth_sim_clock_toward_target_system(
    target: Res<RequestedClockScale>,
    feel: Res<SandboxFeelTuning>,
    time: Res<Time>,
    mut sim_state: ResMut<SandboxSimState>,
) {
    let frame_dt = time.delta_secs();
    let rate = if target.sim_clock < sim_state.time_scale {
        feel.time_ramp_down_rate
    } else {
        feel.time_ramp_up_rate
    };
    sim_state.time_scale =
        crate::move_toward(sim_state.time_scale, target.sim_clock, rate * frame_dt);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::components::PlayerSlot;

    #[test]
    fn proper_time_scale_default_is_one() {
        let pts = ProperTimeScale::default();
        assert_eq!(pts, ProperTimeScale::ONE);
        assert_eq!(pts.value(), 1.0);
    }

    #[test]
    fn proper_time_scale_or_default_falls_back_to_one() {
        let some = ProperTimeScale(2.5);
        assert_eq!(ProperTimeScale::or_default(Some(&some)).value(), 2.5);
        assert_eq!(ProperTimeScale::or_default(None), ProperTimeScale::ONE);
    }

    #[test]
    fn solo_regime_grants_every_requester_every_domain() {
        let policy = RegimePolicy { regime: Regime::Solo };
        for requester in [
            ClockRequester::Player(PlayerSlot::PRIMARY),
            ClockRequester::DevTool,
            ClockRequester::Scripted,
            ClockRequester::Engine,
            ClockRequester::Boss,
        ] {
            for domain in [
                ClockDomain::SimClock,
                ClockDomain::PlayerClock(PlayerSlot::PRIMARY),
                ClockDomain::WallClock,
            ] {
                assert_eq!(
                    policy.permission_for(requester, domain),
                    Permission::Grant,
                    "Solo must grant {:?} -> {:?}", requester, domain,
                );
            }
        }
    }

    #[test]
    fn rl_deterministic_denies_every_request() {
        let policy = RegimePolicy { regime: Regime::RLDeterministic };
        assert_eq!(
            policy.permission_for(
                ClockRequester::Player(PlayerSlot::PRIMARY),
                ClockDomain::SimClock,
            ),
            Permission::Deny,
        );
        assert_eq!(
            policy.permission_for(ClockRequester::Scripted, ClockDomain::SimClock),
            Permission::Deny,
        );
    }

    #[test]
    fn cinematic_grants_scripted_denies_player() {
        let policy = RegimePolicy { regime: Regime::Cinematic };
        assert_eq!(
            policy.permission_for(ClockRequester::Scripted, ClockDomain::SimClock),
            Permission::Grant,
        );
        assert_eq!(
            policy.permission_for(
                ClockRequester::Player(PlayerSlot::PRIMARY),
                ClockDomain::SimClock,
            ),
            Permission::Deny,
        );
        // Engine retains authority during cinematics so pause /
        // suspended-gameplay still works.
        assert_eq!(
            policy.permission_for(ClockRequester::Engine, ClockDomain::SimClock),
            Permission::Grant,
        );
    }

    /// End-to-end: build a minimal app, fire a request, run the
    /// system, observe the target change. This is the canonical
    /// regression check that the dispatch pipeline is wired.
    /// SandboxSimState::time_scale is touched by the SMOOTHER
    /// (smooth_sim_clock_toward_target_system), not by this system.
    #[test]
    fn solo_grant_writes_requested_clock_scale() {
        let mut app = App::new();
        app.add_message::<ClockScaleRequest>()
            .insert_resource(RegimePolicy::default())
            .insert_resource(RequestedClockScale::default())
            .add_systems(Update, apply_clock_scale_requests);

        app.world_mut().write_message(ClockScaleRequest {
            domain: ClockDomain::SimClock,
            scale: 0.125,
            requester: ClockRequester::Player(PlayerSlot::PRIMARY),
            reason: "bullet_blink_test",
        });

        app.update();

        let target = app.world().resource::<RequestedClockScale>();
        assert!((target.sim_clock - 0.125).abs() < 1e-6);
    }

    #[test]
    fn rl_regime_denies_blocks_the_scale_change() {
        let mut app = App::new();
        app.add_message::<ClockScaleRequest>()
            .insert_resource(RegimePolicy { regime: Regime::RLDeterministic })
            .insert_resource(RequestedClockScale::default())
            .add_systems(Update, apply_clock_scale_requests);

        app.world_mut().write_message(ClockScaleRequest {
            domain: ClockDomain::SimClock,
            scale: 0.125,
            requester: ClockRequester::Player(PlayerSlot::PRIMARY),
            reason: "denied_test",
        });

        app.update();

        let target = app.world().resource::<RequestedClockScale>();
        assert!(
            (target.sim_clock - 1.0).abs() < 1e-6,
            "RL must keep sim-clock target at 1.0",
        );
    }

    /// Wall clock is by definition unscaled. A grant targeting it is
    /// a no-op so the host's real-time tick keeps advancing.
    #[test]
    fn wall_clock_grant_does_not_mutate_target() {
        let mut app = App::new();
        app.add_message::<ClockScaleRequest>()
            .insert_resource(RegimePolicy::default())
            .insert_resource(RequestedClockScale::default())
            .add_systems(Update, apply_clock_scale_requests);

        app.world_mut().write_message(ClockScaleRequest {
            domain: ClockDomain::WallClock,
            scale: 0.25,
            requester: ClockRequester::DevTool,
            reason: "wall_noop_test",
        });

        app.update();

        let target = app.world().resource::<RequestedClockScale>();
        assert!((target.sim_clock - 1.0).abs() < 1e-6);
    }

    /// End-to-end: a SimClock grant + the smoother together actually
    /// move time_scale. After ~10 frames at default ramp rates the
    /// time_scale should be well below 1.0 (heading toward 0.125).
    #[test]
    fn smoother_ramps_sim_state_time_scale_toward_target() {
        let mut app = App::new();
        app.add_message::<ClockScaleRequest>()
            .insert_resource(RegimePolicy::default())
            .insert_resource(RequestedClockScale::default())
            .insert_resource(SandboxSimState::default())
            .insert_resource(SandboxFeelTuning::default())
            .insert_resource(Time::<()>::default())
            .add_systems(
                Update,
                (apply_clock_scale_requests, smooth_sim_clock_toward_target_system).chain(),
            );

        // Pump a fixed 16ms tick into Bevy's Time so the smoother
        // sees a non-zero frame_dt.
        let frame = std::time::Duration::from_millis(16);
        for _ in 0..30 {
            app.world_mut().write_message(ClockScaleRequest {
                domain: ClockDomain::SimClock,
                scale: 0.125,
                requester: ClockRequester::Player(PlayerSlot::PRIMARY),
                reason: "ramp_test",
            });
            app.world_mut().resource_mut::<Time>().advance_by(frame);
            app.update();
        }

        let sim = app.world().resource::<SandboxSimState>();
        // SandboxFeelTuning::time_ramp_down_rate is 6.0 (units/s).
        // 30 frames * 16ms = 480ms => deltas of ~2.88 units, way
        // past the (1.0 -> 0.125) gap of 0.875.
        assert!(
            (sim.time_scale - 0.125).abs() < 1e-4,
            "expected sim time_scale ~= 0.125 after ramp; got {}",
            sim.time_scale,
        );
    }
}
