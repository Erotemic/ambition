//! Per-frame named-clock dt snapshots (ADR 0010 / 0011).
//!
//! Extracted from `lib.rs` during the themed-module reorg. Public path is
//! preserved by re-exports at the crate root.

use bevy::prelude::{Res, ResMut, Resource, Time};

use crate::player::components::PlayerSlot;
use crate::time::time_control::ProperTimeScale;
use crate::SandboxSimState;

/// ADR 0010 vocabulary — the named clocks gameplay code can read.
///
/// `SimClock` ticks at the gameplay rate; bullet-time / hitstop /
/// pause scale this. `PlayerClock(slot)` is a per-player cognitive
/// rate (ADR 0011) and is what multiplayer-coherent time abilities
/// rebind. `WallClock` is the host's real time, never scaled —
/// used by UI fades, hot-reload polling, audio.
///
/// In single-player today every PlayerClock equals SimClock, so
/// the operationally-equivalent SP path "slow sim" and MP-correct
/// path "boost player proper time" are observationally identical.
/// See ADR 0011 §"Two time-control operations".
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ClockDomain {
    SimClock,
    PlayerClock(PlayerSlot),
    WallClock,
}

/// Per-frame dt snapshots keyed by [`ClockDomain`].
///
/// Use the typed accessors instead of `Res<Time>::delta_secs()`:
///
/// - [`WorldTime::sim_dt`] — gameplay state machines + world-anchored
///   animation. Scales with bullet-time / hitstop / pause.
/// - [`WorldTime::player_dt`] — per-player cognitive rate. In SP this
///   equals `sim_dt`; in future MP it's the seam where one player's
///   bullet-time doesn't slow the other's world.
/// - [`WorldTime::wall_dt`] — real time. UI fades, hot-reload polling,
///   debug overlays — anything that must NOT freeze with the world.
///
/// Default `sim_dt` for new code; reach for the others only when you
/// can articulate why ([feedback-time-domains]).
///
/// The legacy fields [`WorldTime::raw_dt`] / [`WorldTime::scaled_dt`]
/// remain as aliases (`raw_dt == wall_dt`, `scaled_dt == sim_dt`) so
/// existing callers keep compiling. Migrate to the accessors at
/// touch time; the fields are slated for removal in a follow-up.
///
/// Refreshed once per Update via [`refresh_world_time`] before
/// every system that reads it; the resource is always one frame
/// fresh by construction.
#[derive(Resource, Default, Debug, Clone, Copy)]
pub struct WorldTime {
    /// Wall-clock dt from Bevy's `Time` resource. Unscaled — for
    /// UI / debug only. Legacy alias for [`WorldTime::wall_dt`].
    pub raw_dt: f32,
    /// `raw_dt * SandboxSimState::time_scale`. The canonical
    /// dt for gameplay + world-anchored animation timers. Zero
    /// while paused (`time_scale == 0`). Legacy alias for
    /// [`WorldTime::sim_dt`].
    pub scaled_dt: f32,
}

impl WorldTime {
    /// Dt for the gameplay sim clock — bullet-time / hitstop / pause
    /// scale this. Canonical choice for world-anchored timers,
    /// animation, AI ticks, and any gameplay state machine.
    #[inline]
    pub fn sim_dt(&self) -> f32 {
        self.scaled_dt
    }

    /// Dt for the host's wall clock — never scaled. Use for UI
    /// fades, hot-reload polling, debug overlays, audio buses;
    /// anything that must keep ticking when the world freezes.
    #[inline]
    pub fn wall_dt(&self) -> f32 {
        self.raw_dt
    }

    /// Dt for player `slot`'s cognitive clock (ADR 0011). In the
    /// single-player Solo regime every `PlayerClock` equals
    /// `SimClock`, so this is observationally identical to
    /// [`Self::sim_dt`] today. The accessor is the seam where a
    /// future CoopConsensual / Competitive regime can give each
    /// player a distinct rate without the call sites changing.
    #[inline]
    pub fn player_dt(&self, _slot: PlayerSlot) -> f32 {
        // SP regime: every PlayerClock == SimClock.
        // ADR 0010 §Regimes — Solo is permissive; multi-observer
        // regimes (future) will diverge here.
        self.sim_dt()
    }

    /// Dt for an arbitrary [`ClockDomain`]. Prefer the typed
    /// accessors above for known domains; this exists for systems
    /// that take a domain as data (the regime-policy dispatch).
    #[inline]
    pub fn dt_for(&self, domain: ClockDomain) -> f32 {
        match domain {
            ClockDomain::SimClock => self.sim_dt(),
            ClockDomain::PlayerClock(slot) => self.player_dt(slot),
            ClockDomain::WallClock => self.wall_dt(),
        }
    }

    /// Per-entity proper-time dt (ADR 0011). Multiplies [`Self::sim_dt`]
    /// by the entity's [`crate::time::time_control::ProperTimeScale`] —
    /// `1.0` by default, so callers that pass
    /// [`crate::time::time_control::ProperTimeScale::ONE`] (the missing-
    /// component case) get the same `sim_dt` value as before.
    ///
    /// Pattern: animator + AI systems query `Option<&ProperTimeScale>`
    /// alongside the entity's other components and feed the result
    /// through [`crate::time::time_control::ProperTimeScale::or_default`]
    /// before calling `entity_dt`. SP gameplay is unchanged because no
    /// entity sets the component today.
    #[inline]
    pub fn entity_dt(&self, scale: ProperTimeScale) -> f32 {
        self.sim_dt() * scale.value()
    }
}

/// Refresh [`WorldTime`] from `Time × SandboxSimState::time_scale`.
/// Registered early in the Update schedule so every downstream
/// system sees a current value.
pub fn refresh_world_time(
    time: Res<Time>,
    sim_state: Res<SandboxSimState>,
    mut world_time: ResMut<WorldTime>,
) {
    let raw = time.delta_secs();
    world_time.raw_dt = raw;
    world_time.scaled_dt = raw * sim_state.time_scale;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::components::PlayerSlot;
    use crate::time::time_control::ProperTimeScale;

    /// Sim regime: SP grants every PlayerClock the SimClock rate, so
    /// player_dt(slot) is observationally identical to sim_dt(). This
    /// is the seam where future MP / RL regimes diverge — until they
    /// do, the SP path stays one-line.
    #[test]
    fn sp_player_clock_equals_sim_clock() {
        let wt = WorldTime {
            raw_dt: 1.0 / 60.0,
            scaled_dt: 1.0 / 240.0,
        };
        assert_eq!(wt.sim_dt(), 1.0 / 240.0);
        assert_eq!(wt.player_dt(PlayerSlot::PRIMARY), wt.sim_dt());
        assert_eq!(wt.player_dt(PlayerSlot(7)), wt.sim_dt());
    }

    /// Wall clock is never scaled. UI fades / hot-reload polling must
    /// keep ticking when the world freezes.
    #[test]
    fn wall_dt_ignores_sim_scale() {
        let wt = WorldTime {
            raw_dt: 1.0 / 60.0,
            scaled_dt: 0.0,
        };
        assert_eq!(wt.wall_dt(), 1.0 / 60.0);
        assert_eq!(wt.sim_dt(), 0.0);
    }

    /// `dt_for(ClockDomain)` is the data-driven dispatch used by the
    /// regime policy. Each domain routes to its typed accessor.
    #[test]
    fn dt_for_dispatches_by_domain() {
        let wt = WorldTime {
            raw_dt: 1.0 / 60.0,
            scaled_dt: 1.0 / 480.0,
        };
        assert_eq!(wt.dt_for(ClockDomain::SimClock), wt.sim_dt());
        assert_eq!(wt.dt_for(ClockDomain::WallClock), wt.wall_dt());
        assert_eq!(
            wt.dt_for(ClockDomain::PlayerClock(PlayerSlot::PRIMARY)),
            wt.player_dt(PlayerSlot::PRIMARY),
        );
    }

    /// Legacy fields remain as aliases — `raw_dt == wall_dt` and
    /// `scaled_dt == sim_dt`. Existing call sites keep compiling.
    #[test]
    fn legacy_fields_alias_new_accessors() {
        let wt = WorldTime {
            raw_dt: 0.016,
            scaled_dt: 0.004,
        };
        assert_eq!(wt.raw_dt, wt.wall_dt());
        assert_eq!(wt.scaled_dt, wt.sim_dt());
    }

    /// ADR 0011 — per-entity proper time. The default scale 1.0
    /// collapses entity_dt to sim_dt; non-1.0 scales independently
    /// stretch or shrink the entity's tick. SP today doesn't set
    /// the component, so every entity tickts at sim_dt — Galilean
    /// behavior unchanged.
    #[test]
    fn entity_dt_default_one_equals_sim_dt() {
        let wt = WorldTime {
            raw_dt: 0.016,
            scaled_dt: 0.008,
        };
        assert_eq!(wt.entity_dt(ProperTimeScale::ONE), wt.sim_dt());
    }

    #[test]
    fn entity_dt_scales_sim_dt_by_proper_time() {
        let wt = WorldTime {
            raw_dt: 0.016,
            scaled_dt: 0.008,
        };
        assert!((wt.entity_dt(ProperTimeScale(2.0)) - 0.016).abs() < 1e-7);
        assert!((wt.entity_dt(ProperTimeScale(0.5)) - 0.004).abs() < 1e-7);
    }
}
