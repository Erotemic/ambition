//! Boss special-attack **Techniques** — the content-owned systems that drive
//! each named boss special. A Technique reads the boss's brain signal
//! (`ActorActionMessage::Special`) + its per-boss temporal state, and emits
//! generic `ambition_sandbox::effects::Effect`s for the engine to execute. The
//! engine owns no boss-special behavior; it lives here.
//!
//! Each Technique's per-boss state component is content-owned too, attached to
//! every boss via `register_required_components::<BossConfig, _>()` in
//! [`super::AmbitionBossContentPlugin`] — so the machinery lib names no boss
//! technique.
//!
//! Migrated from `ambition_sandbox::features::ecs::brain_effects` one Technique
//! at a time. First: the Smirking Behemoth eye beam.
#![allow(unused_imports)]

use bevy::prelude::*;

use ambition_sandbox::brain::{
    action_set::ActionRequest, ActorActionMessage, BossAttackProfile, BossAttackState,
    SpecialActionSpec,
};
use ambition_sandbox::effects::{Effect, EffectRequest};
use ambition_sandbox::enemy_projectile::EnemyProjectileSpawn;
use ambition_sandbox::engine_core::{self as ae, AabbExt};
use ambition_sandbox::features::{ActorFaction, ActorTarget, BossClusterRef, FeatureSimEntity};
use ambition_sandbox::player::{BodyKinematics, PlayerEntity};
use ambition_sandbox::projectile::ProjectileFaction;
use ambition_sandbox::WorldTime;

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
