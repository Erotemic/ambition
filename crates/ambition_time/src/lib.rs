//! Reusable time vocabulary + producer for Bevy games (ADR 0010 / 0011).
//!
//! This crate owns the *generic* time machinery extracted from
//! `ambition_sandbox`: the named-clock dt model ([`WorldTime`] +
//! [`ClockDomain`]), the single sim-clock scale ([`ClockState`]), the
//! per-entity proper-time scale ([`ProperTimeScale`]), and a drop-in
//! [`TimePlugin`] that computes [`WorldTime`] from [`ClockState`] × Bevy
//! `Time` once per frame.
//!
//! It is content-free and game-agnostic — a different platformer (or an
//! agent building one) can `app.add_plugins(TimePlugin)` and immediately
//! get bullet-time / hitstop / pause-aware dt accessors. What stays in the
//! host game is the *policy* (who is allowed to bend which clock — the
//! Regime / requester table) and how the scale is *driven* (game-feel
//! ramping), because those are game-specific authority decisions. This
//! crate only owns the vocabulary and the producer.
//!
//! ## Quick start
//!
//! ```ignore
//! app.add_plugins(ambition_time::TimePlugin);
//! // ... anywhere downstream:
//! fn my_system(time: Res<ambition_time::WorldTime>) {
//!     let dt = time.sim_dt(); // scales with bullet-time / hitstop / pause
//! }
//! // ... to bend time, write ClockState::time_scale from your own policy.
//! ```
//!
//! ## The dt model (ADR 0010)
//!
//! Read dt through the typed accessors instead of `Res<Time>::delta_secs()`:
//! - [`WorldTime::sim_dt`] — gameplay state machines + world-anchored
//!   animation. Scales with bullet-time / hitstop / pause.
//! - [`WorldTime::wall_dt`] — real time. UI fades, hot-reload polling,
//!   audio — anything that must NOT freeze with the world.
//! - [`WorldTime::player_dt`] — per-observer cognitive rate. In single-
//!   player it equals `sim_dt`; it's the seam where a future multiplayer
//!   build gives one player's bullet-time a different rate from another's.
//! - [`WorldTime::entity_dt`] — per-entity proper time (ADR 0011), the
//!   seam for special-relativity / per-entity time dilation.

use bevy::prelude::*;

/// A clock observer — the seam for per-player (or per-agent) cognitive
/// clocks (ADR 0011). A game maps its own player-slot type onto this
/// generic index; in single-player every observer collapses to
/// [`ClockObserver::PRIMARY`].
///
/// Kept a plain `u8` newtype (not coupled to any game's player type) so
/// this crate stays content-free. Convert from a host slot type with a
/// `From`/`Into` impl on the host side.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct ClockObserver(pub u8);

impl ClockObserver {
    /// The local primary observer (player 1 / the single-player observer).
    pub const PRIMARY: ClockObserver = ClockObserver(0);

    /// The raw observer index.
    pub fn index(self) -> u8 {
        self.0
    }
}

/// ADR 0010 vocabulary — the named clocks gameplay code can read.
///
/// `SimClock` ticks at the gameplay rate; bullet-time / hitstop /
/// pause scale this. `PlayerClock(observer)` is a per-observer cognitive
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
    PlayerClock(ClockObserver),
    WallClock,
}

/// The current sim-clock scale — the single mutable `f32` the time
/// system owns and the [`WorldTime`] producer reads each frame.
///
/// `1.0` is real-time; `0.0` is fully paused; values in between are
/// hitstop / bullet-time / dev-slowmo. The host game's time-control
/// policy is this resource's only writer (e.g. a feel-tuned smoother
/// ramping it toward a granted target).
///
/// **Multiplayer caveat:** this is **global shared-world** — hitstop,
/// bullet-time, and pause affect the whole party. A future build that
/// wants per-player cognitive rates uses the per-entity
/// [`ProperTimeScale`] / [`ClockDomain::PlayerClock`] seam instead,
/// leaving this resource shared.
#[derive(Resource, Clone, Copy, Debug)]
pub struct ClockState {
    /// `raw_dt * time_scale` is the canonical sim dt. See [`WorldTime`].
    pub time_scale: f32,
}

impl Default for ClockState {
    fn default() -> Self {
        Self { time_scale: 1.0 }
    }
}

/// ADR 0011 — per-entity proper-time scale.
///
/// An entity with `ProperTimeScale(2.0)` ticks at twice the world's
/// sim rate; an entity with `ProperTimeScale(0.5)` ticks at half.
/// Default is `1.0`, in which case [`WorldTime::entity_dt`] returns
/// sim_dt unchanged.
///
/// This is the seam for per-entity time dilation: a future room metric
/// could compute it from velocity via the Lorentz factor
/// `γ(v) = 1 / √(1 − v²/c²)`. The integrator already reads per-entity
/// proper-time scale, so adding special relativity is a data change.
///
/// Most entities never carry this component; [`ProperTimeScale::or_default`]
/// returns `ONE` (i.e., `sim_dt`) when it's missing so the change is
/// invisible until something opts in.
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

/// Per-frame dt snapshots keyed by [`ClockDomain`].
///
/// Use the typed accessors instead of `Res<Time>::delta_secs()`:
///
/// - [`WorldTime::sim_dt`] — gameplay state machines + world-anchored
///   animation. Scales with bullet-time / hitstop / pause.
/// - [`WorldTime::player_dt`] — per-observer cognitive rate. In SP this
///   equals `sim_dt`; in future MP it's the seam where one player's
///   bullet-time doesn't slow the other's world.
/// - [`WorldTime::wall_dt`] — real time. UI fades, hot-reload polling,
///   debug overlays — anything that must NOT freeze with the world.
///
/// Default `sim_dt` for new code; reach for the others only when you
/// can articulate why.
///
/// The legacy fields [`WorldTime::raw_dt`] / [`WorldTime::scaled_dt`]
/// remain as aliases (`raw_dt == wall_dt`, `scaled_dt == sim_dt`) so
/// existing callers keep compiling.
///
/// Refreshed once per Update via [`refresh_world_time`] before
/// every system that reads it; the resource is always one frame
/// fresh by construction.
#[derive(Resource, Default, Debug, Clone, Copy)]
pub struct WorldTime {
    /// Wall-clock dt from Bevy's `Time` resource. Unscaled — for
    /// UI / debug only. Legacy alias for [`WorldTime::wall_dt`].
    pub raw_dt: f32,
    /// `raw_dt * ClockState::time_scale`. The canonical
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

    /// Dt for observer `slot`'s cognitive clock (ADR 0011). In the
    /// single-player Solo regime every `PlayerClock` equals
    /// `SimClock`, so this is observationally identical to
    /// [`Self::sim_dt`] today. The accessor is the seam where a
    /// future CoopConsensual / Competitive regime can give each
    /// observer a distinct rate without the call sites changing.
    #[inline]
    pub fn player_dt(&self, _slot: ClockObserver) -> f32 {
        // SP regime: every PlayerClock == SimClock.
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
    /// by the entity's [`ProperTimeScale`] — `1.0` by default, so callers
    /// that pass [`ProperTimeScale::ONE`] (the missing-component case) get
    /// the same `sim_dt` value as before.
    ///
    /// Pattern: animator + AI systems query `Option<&ProperTimeScale>`
    /// alongside the entity's other components and feed the result
    /// through [`ProperTimeScale::or_default`] before calling `entity_dt`.
    #[inline]
    pub fn entity_dt(&self, scale: ProperTimeScale) -> f32 {
        self.sim_dt() * scale.value()
    }
}

/// Refresh [`WorldTime`] from `Time × ClockState::time_scale`.
/// Registered early in the Update schedule (by [`TimePlugin`]) so every
/// downstream system sees a current value.
pub fn refresh_world_time(
    time: Res<Time>,
    clock: Res<ClockState>,
    mut world_time: ResMut<WorldTime>,
) {
    let raw = time.delta_secs();
    world_time.raw_dt = raw;
    world_time.scaled_dt = raw * clock.time_scale;
}

/// Drop-in time producer: installs [`ClockState`] + [`WorldTime`] and
/// runs [`refresh_world_time`] each `Update` so downstream systems read
/// a one-frame-fresh [`WorldTime`].
///
/// The host game writes [`ClockState::time_scale`] from its own
/// time-control policy (bullet-time / hitstop / pause); this plugin only
/// converts that scale × Bevy `Time` into the named-clock dt snapshot.
///
/// `add_plugins(TimePlugin)` is idempotent on the resources via
/// `init_resource`, so a host that wants to seed a non-default
/// [`ClockState`] can `insert_resource` it before adding the plugin.
#[derive(Default)]
pub struct TimePlugin;

impl Plugin for TimePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ClockState>()
            .init_resource::<WorldTime>()
            .add_systems(Update, refresh_world_time);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sp_player_clock_equals_sim_clock() {
        let wt = WorldTime {
            raw_dt: 1.0 / 60.0,
            scaled_dt: 1.0 / 240.0,
        };
        assert_eq!(wt.sim_dt(), 1.0 / 240.0);
        assert_eq!(wt.player_dt(ClockObserver::PRIMARY), wt.sim_dt());
        assert_eq!(wt.player_dt(ClockObserver(7)), wt.sim_dt());
    }

    #[test]
    fn wall_dt_ignores_sim_scale() {
        let wt = WorldTime {
            raw_dt: 1.0 / 60.0,
            scaled_dt: 0.0,
        };
        assert_eq!(wt.wall_dt(), 1.0 / 60.0);
        assert_eq!(wt.sim_dt(), 0.0);
    }

    #[test]
    fn dt_for_dispatches_by_domain() {
        let wt = WorldTime {
            raw_dt: 1.0 / 60.0,
            scaled_dt: 1.0 / 480.0,
        };
        assert_eq!(wt.dt_for(ClockDomain::SimClock), wt.sim_dt());
        assert_eq!(wt.dt_for(ClockDomain::WallClock), wt.wall_dt());
        assert_eq!(
            wt.dt_for(ClockDomain::PlayerClock(ClockObserver::PRIMARY)),
            wt.player_dt(ClockObserver::PRIMARY),
        );
    }

    #[test]
    fn legacy_fields_alias_new_accessors() {
        let wt = WorldTime {
            raw_dt: 0.016,
            scaled_dt: 0.004,
        };
        assert_eq!(wt.raw_dt, wt.wall_dt());
        assert_eq!(wt.scaled_dt, wt.sim_dt());
    }

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

    #[test]
    fn proper_time_scale_default_is_one() {
        assert_eq!(ProperTimeScale::default(), ProperTimeScale::ONE);
        assert_eq!(ProperTimeScale::default().value(), 1.0);
    }

    #[test]
    fn proper_time_scale_or_default_falls_back_to_one() {
        let some = ProperTimeScale(2.5);
        assert_eq!(ProperTimeScale::or_default(Some(&some)).value(), 2.5);
        assert_eq!(ProperTimeScale::or_default(None), ProperTimeScale::ONE);
    }

    #[test]
    fn time_plugin_refreshes_world_time_from_clock_state() {
        let mut app = App::new();
        app.insert_resource(ClockState { time_scale: 0.5 });
        app.add_plugins(TimePlugin);
        app.insert_resource(Time::<()>::default());
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(16));
        app.update();
        let wt = app.world().resource::<WorldTime>();
        assert!(wt.raw_dt > 0.0);
        assert!((wt.sim_dt() - wt.wall_dt() * 0.5).abs() < 1e-7);
    }
}
