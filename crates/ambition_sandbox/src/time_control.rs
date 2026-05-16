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
use crate::player::components::PlayerSlot;

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

/// Drain pending [`ClockScaleRequest`] messages, run them through
/// the active [`RegimePolicy`], and apply the granted scales.
///
/// Today this only mutates [`SandboxSimState::time_scale`] for
/// `SimClock` grants. PlayerClock + per-entity proper-time
/// destinations land in ADR 0011 step 3. WallClock is never
/// scaled by definition — grants targeting it are a no-op.
///
/// The system reads requests as a stream so multiple requesters
/// in one frame are processed in order; later requests win for the
/// same domain. The smoother in [`crate::update_time_scale`] still
/// runs every frame and shapes the final ramp; this system is the
/// data-driven seam between "who asked" and "what changes".
pub fn apply_clock_scale_requests(
    mut requests: MessageReader<ClockScaleRequest>,
    policy: Res<RegimePolicy>,
    mut sim_state: ResMut<SandboxSimState>,
) {
    for req in requests.read() {
        let permission = policy.permission_for(req.requester, req.domain);
        match permission {
            Permission::Grant => write_scale(&mut sim_state, req.domain, req.scale),
            Permission::Deny => continue,
            Permission::Rebind(other) => write_scale(&mut sim_state, other, req.scale),
            Permission::Broadcast => {
                // SP today has one player + one sim clock; broadcast
                // collapses to a SimClock write. CoopConsensual will
                // fan out to every PlayerClock here.
                write_scale(&mut sim_state, ClockDomain::SimClock, req.scale);
            }
        }
    }
}

fn write_scale(sim_state: &mut SandboxSimState, domain: ClockDomain, scale: f32) {
    match domain {
        ClockDomain::SimClock => sim_state.time_scale = scale,
        // ADR 0011 step 3 wires per-player + per-entity clocks; until
        // then the SP regime collapses these onto SimClock so today's
        // call sites see identical behavior.
        ClockDomain::PlayerClock(_) => sim_state.time_scale = scale,
        ClockDomain::WallClock => { /* wall clock is never scaled */ }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::components::PlayerSlot;

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
    /// system, observe the time_scale change. This is the canonical
    /// regression check that the dispatch pipeline is wired.
    #[test]
    fn solo_grant_writes_sim_clock_scale() {
        let mut app = App::new();
        app.add_message::<ClockScaleRequest>()
            .insert_resource(RegimePolicy::default())
            .insert_resource(SandboxSimState::default())
            .add_systems(Update, apply_clock_scale_requests);

        app.world_mut().write_message(ClockScaleRequest {
            domain: ClockDomain::SimClock,
            scale: 0.125,
            requester: ClockRequester::Player(PlayerSlot::PRIMARY),
            reason: "bullet_blink_test",
        });

        app.update();

        let sim = app.world().resource::<SandboxSimState>();
        assert!((sim.time_scale - 0.125).abs() < 1e-6);
    }

    #[test]
    fn rl_regime_denies_blocks_the_scale_change() {
        let mut app = App::new();
        app.add_message::<ClockScaleRequest>()
            .insert_resource(RegimePolicy { regime: Regime::RLDeterministic })
            .insert_resource(SandboxSimState::default())
            .add_systems(Update, apply_clock_scale_requests);

        app.world_mut().write_message(ClockScaleRequest {
            domain: ClockDomain::SimClock,
            scale: 0.125,
            requester: ClockRequester::Player(PlayerSlot::PRIMARY),
            reason: "denied_test",
        });

        app.update();

        let sim = app.world().resource::<SandboxSimState>();
        assert!((sim.time_scale - 1.0).abs() < 1e-6, "RL must keep sim clock at 1.0");
    }

    /// Wall clock is by definition unscaled. A grant targeting it is
    /// a no-op so the host's real-time tick keeps advancing.
    #[test]
    fn wall_clock_grant_does_not_mutate_sim_state() {
        let mut app = App::new();
        app.add_message::<ClockScaleRequest>()
            .insert_resource(RegimePolicy::default())
            .insert_resource(SandboxSimState::default())
            .add_systems(Update, apply_clock_scale_requests);

        app.world_mut().write_message(ClockScaleRequest {
            domain: ClockDomain::WallClock,
            scale: 0.25,
            requester: ClockRequester::DevTool,
            reason: "wall_noop_test",
        });

        app.update();

        let sim = app.world().resource::<SandboxSimState>();
        assert!((sim.time_scale - 1.0).abs() < 1e-6);
    }
}
