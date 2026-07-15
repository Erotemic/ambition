//! Boss special-attack **Techniques** — the content-owned systems that drive
//! each named boss special. A Technique reads the boss's brain signal
//! (`ActorActionMessage::Special`) + its per-boss temporal state, and emits
//! generic `ambition_vfx::Effect`s for the engine to execute. The
//! engine owns no boss-special behavior; it lives here.
//!
//! Each Technique's per-boss state component is content-owned too, attached to
//! every boss via `register_required_components::<BossConfig, _>()` in
//! [`super::AmbitionBossContentPlugin`] — so the machinery lib names no boss
//! technique.
//!
//! Migrated from `ambition_actors::features::ecs::brain_effects` one Technique
//! at a time. First: the Smirking Behemoth eye beam.

use bevy::prelude::*;

mod echo_fan;
mod eye_beam;
mod gradient_nova;
mod gradient_sentinel;
mod mode_collapse;
mod overflow_flood;
mod seismic_stomp;

// Curated re-export of each Technique's public surface: the per-boss state
// component (attached via required components + snapshot-registered) and the
// `spawn_*_from_special_messages` system. Nothing outside this module consumes
// them today — the hub feeds this file's plugin (below) and `snapshot::register`
// — but they are the Techniques' genuine public API, so an explicit `pub use`
// (not a glob) states it without re-globbing each submodule's private imports.
pub use echo_fan::{spawn_echo_fan_from_special_messages, EchoFanState};
pub use eye_beam::{spawn_eye_beam_from_special_messages, EyeBeamState};
pub use gradient_nova::{spawn_gradient_nova_from_special_messages, ExplodingGradientState};
pub use gradient_sentinel::{
    spawn_gnu_apple_rain_from_special_messages,
    spawn_gradient_cascade_minions_from_special_messages, spawn_minima_trap_from_special_messages,
    spawn_overfit_volley_from_special_messages, spawn_saddle_point_from_special_messages,
    AppleRainSpawnState, GradientCascadeState, MinimaTrapState, OverfitVolleyState,
    SaddlePointState,
};
pub use mode_collapse::{spawn_mode_collapse_converge_from_special_messages, ModeCollapseState};
pub use overflow_flood::{spawn_overflow_flood_from_special_messages, OverflowState};
pub use seismic_stomp::{spawn_seismic_stomp_from_special_messages, SeismicStompState};

use ambition_actors::features::BossConfig;
use ambition_platformer_primitives::schedule::gameplay_allowed;
use ambition_platformer_primitives::schedule::CombatSet;
use ambition_platformer_primitives::schedule::SimScheduleExt;

/// Installs the named per-boss special-attack Techniques as a single
/// self-contained content domain unit.
///
/// It owns both halves of the boss-special wiring that the engine
/// deliberately names nothing of:
///
/// 1. **State attachment** — each Technique's per-boss temporal state is
///    attached to every boss (`BossConfig`) via required components, so a
///    boss spawned anywhere carries the state its Technique needs.
/// 2. **Schedule** — each Technique system runs in
///    [`CombatSet::ContentSpecials`], the engine's combat extension slot.
///    The slot's position in the combat chain (after the enemy-action
///    consumers, before the effect/projectile executors that drain a
///    Technique's `SpawnProjectile`/`EffectRequest` output) is configured
///    by the app's `CombatSchedulePlugin`.
///
/// Installed by [`super::AmbitionBossContentPlugin`].
mod snapshot;

pub struct BossSpecialContentPlugin;

impl Plugin for BossSpecialContentPlugin {
    fn build(&self, app: &mut App) {
        let sim = app.sim_schedule();
        // Per-boss Technique state, attached to every boss via required
        // components (registered at plugin-build time, before any boss
        // spawns). The machinery lib's spawn names no boss Technique.
        app.register_required_components::<BossConfig, EyeBeamState>();
        app.register_required_components::<BossConfig, AppleRainSpawnState>();
        app.register_required_components::<BossConfig, OverfitVolleyState>();
        app.register_required_components::<BossConfig, MinimaTrapState>();
        app.register_required_components::<BossConfig, SaddlePointState>();
        app.register_required_components::<BossConfig, GradientCascadeState>();
        app.register_required_components::<BossConfig, ModeCollapseState>();
        app.register_required_components::<BossConfig, ExplodingGradientState>();
        app.register_required_components::<BossConfig, OverflowState>();
        app.register_required_components::<BossConfig, SeismicStompState>();
        app.register_required_components::<BossConfig, EchoFanState>();

        // N3.1: this crate owns eleven pieces of sim state, and no crate below it can
        // name them. `init_resource` rather than a plugin-order assumption — either
        // this plugin or `SnapshotRegistryPlugin` may build first, and both are
        // additive. (A silent `if let Some(..)` here registered NOTHING for one
        // commit, because this plugin builds first. Silence is not a fallback.)
        app.init_resource::<ambition_runtime::snapshot::SnapshotRegistry>();
        let mut registry = app
            .world_mut()
            .resource_mut::<ambition_runtime::snapshot::SnapshotRegistry>();
        snapshot::register(&mut registry);

        // The 11 Technique systems, hung on the engine's combat extension
        // slot. They read `ActorActionMessage::Special` and emit
        // `SpawnProjectile`/`EffectRequest`; the slot ordering guarantees
        // those land before the executors that drain them. Each only acts
        // during live gameplay. Nested into two tuples to stay under
        // Bevy's 20-element add_systems limit; the Techniques are mutually
        // independent (disjoint per-boss state), so no inter-system order
        // is imposed within the slot.
        app.add_systems(
            sim,
            (
                spawn_gnu_apple_rain_from_special_messages,
                spawn_overfit_volley_from_special_messages,
                spawn_eye_beam_from_special_messages,
                spawn_mode_collapse_converge_from_special_messages,
                spawn_gradient_nova_from_special_messages,
                spawn_overflow_flood_from_special_messages,
                spawn_seismic_stomp_from_special_messages,
                spawn_echo_fan_from_special_messages,
                spawn_minima_trap_from_special_messages,
                spawn_saddle_point_from_special_messages,
                spawn_gradient_cascade_minions_from_special_messages,
            )
                .run_if(gameplay_allowed)
                .in_set(CombatSet::ContentSpecials),
        );
    }
}
