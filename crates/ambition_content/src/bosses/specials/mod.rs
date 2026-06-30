//! Boss special-attack **Techniques** — the content-owned systems that drive
//! each named boss special. A Technique reads the boss's brain signal
//! (`ActorActionMessage::Special`) + its per-boss temporal state, and emits
//! generic `ambition_gameplay_core::effects::Effect`s for the engine to execute. The
//! engine owns no boss-special behavior; it lives here.
//!
//! Each Technique's per-boss state component is content-owned too, attached to
//! every boss via `register_required_components::<BossConfig, _>()` in
//! [`super::AmbitionBossContentPlugin`] — so the machinery lib names no boss
//! technique.
//!
//! Migrated from `ambition_gameplay_core::features::ecs::brain_effects` one Technique
//! at a time. First: the Smirking Behemoth eye beam.
#![allow(unused_imports)]

use bevy::prelude::*;

use ambition_characters::brain::{
    action_set::ActionRequest, ActorActionMessage, BossAttackProfile, BossAttackState,
    SpecialActionSpec,
};
use ambition_engine_core::{self as ae, AabbExt};
use ambition_gameplay_core::actor::{BodyKinematics, PlayerEntity};
use ambition_gameplay_core::effects::{Effect, EffectRequest};
use ambition_gameplay_core::enemy_projectile::EnemyProjectileSpawn;
use ambition_gameplay_core::features::{
    ActorFaction, ActorTarget, BossClusterRef, FeatureSimEntity,
};
use ambition_gameplay_core::WorldTime;

mod echo_fan;
mod eye_beam;
mod gradient_nova;
mod gradient_sentinel;
mod mode_collapse;
mod overflow_flood;
mod seismic_stomp;

pub use echo_fan::*;
pub use eye_beam::*;
pub use gradient_nova::*;
pub use gradient_sentinel::*;
pub use mode_collapse::*;
pub use overflow_flood::*;
pub use seismic_stomp::*;

use ambition_gameplay_core::features::BossConfig;
use ambition_gameplay_core::schedule::CombatSet;
use ambition_gameplay_core::session::game_mode::gameplay_allowed;

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
pub struct BossSpecialContentPlugin;

impl Plugin for BossSpecialContentPlugin {
    fn build(&self, app: &mut App) {
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

        // The 11 Technique systems, hung on the engine's combat extension
        // slot. They read `ActorActionMessage::Special` and emit
        // `SpawnProjectile`/`EffectRequest`; the slot ordering guarantees
        // those land before the executors that drain them. Each only acts
        // during live gameplay. Nested into two tuples to stay under
        // Bevy's 20-element add_systems limit; the Techniques are mutually
        // independent (disjoint per-boss state), so no inter-system order
        // is imposed within the slot.
        app.add_systems(
            Update,
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
