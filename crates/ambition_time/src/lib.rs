//! Reusable time vocabulary + Bevy producer for the named-clock dt model
//! (ADR 0010 / 0011).
//!
//! Host games write [`ClockState::time_scale`]; [`TimePlugin`] converts
//! Bevy `Time` into [`WorldTime`] once per frame. Read dt through typed
//! accessors instead of `Res<Time>::delta_secs()`:
//!
//! - [`WorldTime::sim_dt`] for gameplay state and world-anchored animation.
//! - [`WorldTime::wall_dt`] for UI, audio, hot reload, and debug overlays.
//! - [`WorldTime::player_dt`] for observer cognitive time.
//! - [`WorldTime::entity_dt`] for per-entity proper time.

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

/// ADR 0010 vocabulary — sim time, per-observer cognitive time, and
/// unscaled host wall time.
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

/// **The canonical timeline** (netcode N0.1): the index of the simulation step
/// currently executing, counting from `0`.
///
/// This is the clock that identifies a moment of simulation — not a wall-clock
/// instant and not a rendered frame. N0.2 input streams are keyed by it, N0.4
/// hashes sim state per value of it, and rollback rewinds to one.
///
/// It advances once per sim step in **both** schedule modes: frame-stepped
/// (one step per rendered frame) and fixed-tick (one step per `Time<Fixed>`
/// expenditure, which may be zero or several per frame). It advances even while
/// gameplay is suspended — a paused world still has a timeline; its `sim_dt` is
/// simply zero.
#[derive(Resource, Default, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SimTick(pub u64);

impl SimTick {
    #[inline]
    pub fn get(self) -> u64 {
        self.0
    }
}

/// Advance [`SimTick`] at the head of each sim step.
///
/// The first executed step is tick `0`, so the counter names *the step now
/// running* rather than the number of steps completed — that is the index a
/// recorded input frame and a post-step state hash must agree on. `first_step`
/// is what buys that off-by-one: the head of step 0 must not increment.
pub fn advance_sim_tick(mut tick: ResMut<SimTick>, mut first_step: Local<bool>) {
    if *first_step {
        tick.0 = tick.0.wrapping_add(1);
    } else {
        *first_step = true;
    }
}

/// ADR 0011 — per-entity proper-time scale. `1.0` means
/// [`WorldTime::entity_dt`] returns sim dt unchanged.
///
/// Most entities never carry this component; [`ProperTimeScale::or_default`]
/// returns [`ProperTimeScale::ONE`] for the missing-component case.
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

/// Per-frame dt snapshot. Prefer [`WorldTime::sim_dt`] for gameplay;
/// use wall/player/entity accessors only when the domain matters.
///
/// Legacy fields remain aliases: `raw_dt == wall_dt`,
/// `scaled_dt == sim_dt`. [`TimePlugin`] refreshes this resource each
/// `Update` before downstream systems read it.
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

    /// Dt for observer `slot`'s cognitive clock. In single-player this
    /// currently equals [`Self::sim_dt`].
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

    /// Per-entity proper-time dt: [`Self::sim_dt`] multiplied by
    /// [`ProperTimeScale`] (`1.0` by default).
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
/// refreshes the named-clock dt snapshot every `Update`.
///
/// `init_resource` preserves a host-provided [`ClockState`] inserted
/// before adding the plugin.
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
