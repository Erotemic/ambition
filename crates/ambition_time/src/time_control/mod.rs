//! Time-control authority as data.
//!
//! Gameplay emits [`ClockScaleRequest`] or [`ClockResetRequest`] instead of
//! mutating [`ClockState::time_scale`] directly. [`apply_clock_scale_requests`]
//! applies the active [`RegimePolicy`] to each request. [`Regime::Solo`] grants
//! every requester; other regimes change the policy table rather than the callers.

use bevy::prelude::*;

use crate::{ClockDomain, ClockObserver, ClockState};

/// Identity of the requester, used by [`RegimePolicy`] to authorize clock changes.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[allow(dead_code)] // Reserved for narrative and boss clock authority.
pub enum ClockRequester {
    /// A player ability (today: bullet-time blink) or future player-
    /// triggered time mechanic.
    Player(ClockObserver),
    /// Developer tools — keyboard slowmo toggle, inspector overrides.
    DevTool,
    /// Scripted cutscene / quest / encounter — narrative authority.
    Scripted,
    /// The engine itself — game-mode pause / suspended-gameplay zeroing.
    Engine,
    /// An in-world entity granted clock authority by room-scoped policy.
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
#[allow(dead_code)] // Some policy variants are not selected by current hosts.
pub enum Permission {
    Grant,
    Deny,
    Rebind(ClockDomain),
    Broadcast,
}

/// Active clock permission policy.
///
/// `Solo` grants every request; `RLDeterministic` denies scale changes; `Cinematic`
/// defers player requests while scripted authority holds.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[allow(dead_code)] // Non-solo regimes are not selected by current hosts.
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
/// boss freeze) instead of mutating [`ClockState::time_scale`]
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
    /// scale change to its source without an additional out-of-band lookup;
    /// [`report_sim_clock_changes`] is what prints it, which is why a
    /// `[sim-clock]` line can name the cause without a provenance system.
    pub reason: &'static str,
}

/// Request an immediate reset of a named clock to neutral scale.
/// Separate from [`ClockScaleRequest`] so reset/respawn/transition can bypass smoothing.
#[derive(Message, Copy, Clone, Debug)]
pub struct ClockResetRequest {
    pub domain: ClockDomain,
    pub requester: ClockRequester,
    /// Telemetry/debug label only. Keep labels short and grep-able — a
    /// `[sim-clock]` line prints it verbatim as the cause of a clock change.
    pub reason: &'static str,
}

impl ClockResetRequest {
    /// Snap the sim clock back to real-time pace (`1.0`).
    pub const fn sim_clock(requester: ClockRequester, reason: &'static str) -> Self {
        Self {
            domain: ClockDomain::SimClock,
            requester,
            reason,
        }
    }
}

/// Target scale per-domain — the value [`ClockState::time_scale`]
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

/// Apply granted clock-scale requests to [`RequestedClockScale`].
///
/// Multiple requests reduce by `min`, so the strongest slowdown wins independently
/// of schedule or query order. A frame with no granted request leaves the target
/// unchanged; callers use [`ClockResetRequest`] to request a return to normal speed.
/// Fast-forward requests are not currently modeled by this reduction.
pub fn apply_clock_scale_requests(
    mut requests: MessageReader<ClockScaleRequest>,
    policy: Res<RegimePolicy>,
    mut target: ResMut<RequestedClockScale>,
) {
    let mut strongest: Option<f32> = None;
    for req in requests.read() {
        let domain = match policy.permission_for(req.requester, req.domain) {
            Permission::Grant => req.domain,
            Permission::Deny => continue,
            Permission::Rebind(other) => other,
            // Solo has one player clock, so broadcast currently collapses to `SimClock`.
            Permission::Broadcast => ClockDomain::SimClock,
        };
        match domain {
            // Solo has one player clock, so this currently targets `SimClock`.
            ClockDomain::SimClock | ClockDomain::PlayerClock(_) => {
                strongest = Some(strongest.map_or(req.scale, |held: f32| held.min(req.scale)));
            }
            ClockDomain::WallClock => { /* wall clock is never scaled */ }
        }
    }
    if let Some(scale) = strongest {
        target.sim_clock = scale;
    }
}

/// Drain pending [`ClockResetRequest`] messages through the same policy table as
/// scale requests, then snap the granted clock domain back to neutral. This is
/// the sole write owner for reset/respawn/transition `time_scale = 1.0` behavior.
pub fn apply_clock_reset_requests(
    mut requests: MessageReader<ClockResetRequest>,
    policy: Res<RegimePolicy>,
    mut target: ResMut<RequestedClockScale>,
    mut clock: ResMut<ClockState>,
) {
    for req in requests.read() {
        let permission = policy.permission_for(req.requester, req.domain);
        match permission {
            Permission::Grant => reset_domain(&mut target, &mut clock, req.domain),
            Permission::Deny => continue,
            Permission::Rebind(other) => reset_domain(&mut target, &mut clock, other),
            Permission::Broadcast => reset_domain(&mut target, &mut clock, ClockDomain::SimClock),
        }
    }
}

fn reset_domain(target: &mut RequestedClockScale, clock: &mut ClockState, domain: ClockDomain) {
    match domain {
        ClockDomain::SimClock | ClockDomain::PlayerClock(_) => {
            target.sim_clock = 1.0;
            clock.time_scale = 1.0;
        }
        ClockDomain::WallClock => { /* wall clock is never scaled */ }
    }
}
/// While gameplay is suspended, force both live and requested sim-clock scale to
/// zero so presentation animations freeze and the smoother cannot ramp up next
/// frame. Gameplay mode leaves scale control to the normal time-control pipeline.
///
/// The host schedule runs this FIRST (under `run_if(gameplay_suspended)`), before
/// `refresh_world_time` snapshots the scale — otherwise `WorldTime::scaled_dt`
/// stays non-zero on the first suspended frame and presentation systems tick once
/// after pause. The ordering lives in the app's
/// `register_player_input_systems`; the logic is body-generic time control and
/// lives here.
pub fn apply_suspended_time_scale_system(
    mut clock: ResMut<ClockState>,
    mut target: ResMut<RequestedClockScale>,
) {
    clock.time_scale = 0.0;
    target.sim_clock = 0.0;
}

#[cfg(test)]
mod tests;
