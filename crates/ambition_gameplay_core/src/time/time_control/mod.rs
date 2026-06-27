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

use crate::actor::BodyCombat;
use crate::player::components::{PlayerSlot, PrimaryPlayer};
use crate::time::clock_state::ClockState;
use crate::time::feel::SandboxFeelTuning;
use crate::ClockDomain;
use crate::SandboxDevState;

/// ADR 0011 — per-entity proper-time scale. Now lives in the reusable
/// `ambition_time` crate (Stage 18 T1b); re-exported here so the historic
/// `crate::time::time_control::ProperTimeScale` path keeps resolving.
pub use ambition_time::ProperTimeScale;

/// Who is asking for a clock change. Encoded as data so a policy
/// table can grant/deny based on identity without hard-coding which
/// systems are allowed to touch which clocks.
///
/// ADR 0010 calls this the `requester` field of `ClockScaleRequest`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[allow(dead_code)] // `Scripted` + `Boss` are reserved per ADR 0010 (narrative authority).
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
#[allow(dead_code)] // `Rebind` + `Broadcast` are reserved per ADR 0010 (CoopConsensual / Regimes).
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
#[allow(dead_code)] // RLDeterministic + Cinematic regimes reserved per ADR 0010.
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
        Self {
            regime: Regime::Solo,
        }
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
    /// Telemetry/debug label only — not read by `apply_clock_scale_requests`.
    /// Kept on the request so traces and the debug overlay can attribute a
    /// scale change to its source without an additional out-of-band lookup.
    #[allow(dead_code)]
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
    primary: Query<(&crate::actor::BodyBlinkState, &BodyCombat), With<PrimaryPlayer>>,
    dev_state: Res<SandboxDevState>,
    feel: Res<SandboxFeelTuning>,
    mut writer: MessageWriter<ClockScaleRequest>,
) {
    let Ok((blink, combat)) = primary.single() else {
        return;
    };
    let (scale, requester, reason) = if combat.hitstop_timer > 0.0 {
        (0.0, ClockRequester::Engine, "hitstop")
    } else if blink.aiming {
        (
            feel.bullet_time_scale,
            ClockRequester::Player(PlayerSlot::PRIMARY),
            "bullet_blink",
        )
    } else if blink.hold_active {
        (
            feel.blink_hold_slow_scale,
            ClockRequester::Player(PlayerSlot::PRIMARY),
            "blink_hold_slow",
        )
    } else if dev_state.slowmo {
        (
            feel.debug_slowmo_scale,
            ClockRequester::DevTool,
            "dev_slowmo",
        )
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
    mut clock: ResMut<ClockState>,
) {
    let frame_dt = time.delta_secs();
    let rate = if target.sim_clock < clock.time_scale {
        feel.time_ramp_down_rate
    } else {
        feel.time_ramp_up_rate
    };
    clock.time_scale = crate::move_toward(clock.time_scale, target.sim_clock, rate * frame_dt);
}

#[cfg(test)]
mod tests;
