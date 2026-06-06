//! Authoritative ECS components for a boss actor + the `BossMut` /
//! `BossRef` views the per-tick systems mutate / read in place.
//!
//! Dissolves the legacy `BossRuntime` blob (formerly held inside the
//! monolithic `BossFeature` component) into real ECS state, following
//! the enemy / NPC cluster pattern. The boss carries the shared
//! [`BodyKinematics`] component (pos / vel / size / facing) — the same
//! component the player and enemies/NPCs use. Bosses float and never
//! integrate `vel` themselves (the brain emits a fresh `desired_vel`
//! each tick for `integrate_body`), so a boss simply leaves `vel` at
//! `Vec2::ZERO`. A `&mut BodyKinematics` boss query is kept disjoint
//! from player/enemy ones with `With<BossConfig>` / `Without<BossConfig>`
//! filters (boss / enemy / player are mutually exclusive archetypes).
//! Boss-specific config
//! (identity, spawn anchor, brain, behavior profile) lives in
//! [`BossConfig`]; mutable status (health, liveness, hit-flash,
//! encounter phase, derived sprite metrics) lives in [`BossStatus`].
//!
//! [`BossConfig`] doubles as the *is-a-boss* marker component — every
//! boss entity carries exactly one, no other actor does — so systems
//! that used to filter on `With<BossFeature>` / `Without<BossFeature>`
//! filter on `With<BossConfig>` / `Without<BossConfig>`.

use bevy::ecs::query::QueryData;
use bevy::prelude::Component;

use super::super::bosses::{canonical_boss_id_from, BossBehaviorProfile, BossSpriteMetrics};
use crate::boss_encounter::BossEncounterPhase;
use crate::engine_core as ae;
use crate::engine_core::AabbExt;

pub use super::enemy_clusters::BodyKinematics;

/// Authored configuration + identity for a boss actor. Also serves as
/// the boss marker component (see module docs).
#[derive(Component, Clone, Debug)]
pub struct BossConfig {
    pub id: String,
    pub name: String,
    /// Authored spawn anchor; `reset` restores `kin.pos` to it.
    pub spawn: ae::Vec2,
    pub brain: crate::actor::BossBrain,
    pub behavior: BossBehaviorProfile,
}

/// Mutable per-tick boss status: health, liveness, hit-flash timer,
/// active encounter phase, and sprite-derived body metrics.
#[derive(Component, Clone, Debug)]
pub struct BossStatus {
    pub health: crate::actor::Health,
    pub alive: bool,
    pub hit_flash: f32,
    /// Active encounter phase. Forwarded by `sync_boss_encounter_phase`
    /// from `BossEncounterRegistry`. `Dormant` until the encounter
    /// wakes up. The brain reads this via `BossPatternContext`.
    pub encounter_phase: BossEncounterPhase,
    /// Sprite-driven body metrics — populated by the
    /// `derive_boss_sprite_metrics` system after the SheetRegistry
    /// has loaded. `None` for bosses whose sprite has no `body_metrics`
    /// entry (the legacy `combat_size` path applies).
    pub sprite_metrics: Option<BossSpriteMetrics>,
}

/// Immutable borrow view over the boss clusters. Hosts the read-only
/// geometry/identity helpers ported from `BossRuntime`.
pub struct BossRef<'a> {
    pub kin: &'a BodyKinematics,
    pub config: &'a BossConfig,
    pub status: &'a BossStatus,
}

/// Mutable borrow view over the boss clusters. Hosts the integration /
/// profile-mutation helpers ported from `BossRuntime`.
pub struct BossMut<'a> {
    pub kin: &'a mut BodyKinematics,
    pub config: &'a mut BossConfig,
    pub status: &'a mut BossStatus,
}

impl<'a> BossRef<'a> {
    pub fn is_mockingbird(&self) -> bool {
        self.config.behavior.id == "mockingbird"
            || self.config.name.eq_ignore_ascii_case("mockingbird")
    }

    pub fn is_gnu_ton(&self) -> bool {
        self.config.behavior.id == "gnu_ton"
            || self.config.name.eq_ignore_ascii_case("gnu_ton")
            || self.config.name.eq_ignore_ascii_case("gnu-ton")
    }

    pub fn render_size(&self) -> ae::Vec2 {
        self.kin.size
    }

    /// Multi-part bosses (GNU-ton) expose a `combat_size` distinct from
    /// the sprite `size`; that's the size collision and volumes use.
    pub fn combat_size(&self) -> ae::Vec2 {
        self.config.behavior.combat_size.unwrap_or(self.kin.size)
    }

    /// World offset from `kin.pos` to the body's bounding-AABB center.
    /// Non-zero for bosses whose sprite metadata reports an off-center
    /// body bbox; `ZERO` otherwise.
    pub fn combat_offset(&self) -> ae::Vec2 {
        self.status
            .sprite_metrics
            .as_ref()
            .map(|m| m.combat_offset)
            .unwrap_or(ae::Vec2::ZERO)
    }

    pub fn aabb(&self) -> ae::Aabb {
        ae::Aabb::new(
            self.kin.pos + self.combat_offset(),
            self.combat_size() * 0.5,
        )
    }

    /// World-space anchor for a combat-banter speech bubble. For GNU-ton
    /// the scholar sits on the right shoulder.
    pub fn bark_anchor(&self) -> ae::Vec2 {
        if self.is_gnu_ton() {
            let half_h = self.combat_size().y * 0.5;
            ae::Vec2::new(self.kin.pos.x + 38.0, self.kin.pos.y - half_h * 0.55 - 18.0)
        } else {
            let half_h = self.combat_size().y * 0.5;
            ae::Vec2::new(self.kin.pos.x, self.kin.pos.y - half_h - 20.0)
        }
    }
}

impl<'a> BossMut<'a> {
    /// Reborrow as an immutable view to reach the read-only helpers.
    pub fn as_ref(&self) -> BossRef<'_> {
        BossRef {
            kin: self.kin,
            config: self.config,
            status: self.status,
        }
    }

    pub fn is_gnu_ton(&self) -> bool {
        self.as_ref().is_gnu_ton()
    }

    pub fn combat_size(&self) -> ae::Vec2 {
        self.as_ref().combat_size()
    }

    pub fn aabb(&self) -> ae::Aabb {
        self.as_ref().aabb()
    }

    pub fn bark_anchor(&self) -> ae::Vec2 {
        self.as_ref().bark_anchor()
    }

    pub fn render_size(&self) -> ae::Vec2 {
        self.as_ref().render_size()
    }

    pub fn apply_behavior_profile(&mut self, behavior: BossBehaviorProfile) {
        self.config.behavior = behavior;
    }

    /// Integrate the boss body using the brain-emitted `desired_vel`.
    /// **Integration only** — the brain owns the policy decision and
    /// writes `ActorControl` upstream; this just collision-resolves the
    /// desired velocity into a position change. Bosses float (gravity =
    /// 0, max_fall_speed = 0); multi-part bosses collide against
    /// `combat_size`, not the sprite `size`.
    pub fn integrate_body(&mut self, world: &ae::World, desired_vel: ae::Vec2, dt: f32) {
        if !self.status.alive || dt <= 0.0 {
            return;
        }
        let mut body = crate::kinematic::KinematicBody {
            pos: self.kin.pos,
            vel: desired_vel,
            size: self.combat_size(),
            on_ground: false,
            facing: self.kin.facing,
        };
        crate::kinematic::step_kinematic(
            &mut body,
            world,
            crate::kinematic::KinematicTuning {
                gravity: 0.0,
                max_fall_speed: 0.0,
                gravity_sign: 1.0,
            },
            crate::kinematic::KinematicInputs {
                drop_through: false,
            },
            dt,
        );
        self.kin.pos = body.pos;
        self.kin.facing = if body.facing.abs() > 0.001 {
            body.facing.signum()
        } else {
            self.kin.facing
        };
        self.status.hit_flash = (self.status.hit_flash - dt).max(0.0);
    }
}

#[derive(QueryData)]
#[query_data(mutable)]
pub struct BossClusterQueryData {
    pub kin: &'static mut BodyKinematics,
    pub config: &'static mut BossConfig,
    pub status: &'static mut BossStatus,
}

impl<'w, 's> BossClusterQueryDataItem<'w, 's> {
    pub fn as_boss_mut<'a>(&'a mut self) -> BossMut<'a>
    where
        'w: 'a,
        's: 'a,
    {
        BossMut {
            kin: &mut self.kin,
            config: &mut self.config,
            status: &mut self.status,
        }
    }

    /// Immutable view of the same components — for read-only helpers
    /// (`aabb`, `combat_size`, `from_ref`, …) on a mutable boss query.
    pub fn as_boss_ref<'a>(&'a self) -> BossRef<'a>
    where
        'w: 'a,
        's: 'a,
    {
        BossRef {
            kin: &self.kin,
            config: &self.config,
            status: &self.status,
        }
    }
}

#[derive(QueryData)]
pub struct BossClusterRef {
    pub kin: &'static BodyKinematics,
    pub config: &'static BossConfig,
    pub status: &'static BossStatus,
}

impl<'w, 's> BossClusterRefItem<'w, 's> {
    pub fn as_boss_ref(&self) -> BossRef<'_> {
        BossRef {
            kin: self.kin,
            config: self.config,
            status: self.status,
        }
    }
}

/// Owned aggregate for spawn construction / non-ECS callers (tests,
/// the gnu_ton encounter setup). Mirrors the enemy/NPC scratch.
#[derive(Clone, Debug)]
pub struct BossClusterScratch {
    pub kin: BodyKinematics,
    pub config: BossConfig,
    pub status: BossStatus,
}

impl BossClusterScratch {
    /// Build the boss clusters directly from spawn inputs — the
    /// cluster-native replacement for `BossRuntime::new`.
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        aabb: ae::Aabb,
        brain: crate::actor::BossBrain,
    ) -> Self {
        let name = name.into();
        // Behavior lookup prefers the brain's `PhaseScript:` id over the
        // LDtk display name, so a "System Boss" room whose brain is
        // `PhaseScript:clockwork_warden` still resolves to the
        // clockwork_warden / Gradient Sentinel profile.
        let canonical_id = canonical_boss_id_from(&name, &brain);
        let center = aabb.center();
        Self {
            kin: BodyKinematics {
                pos: center,
                // Bosses float; the brain emits a fresh `desired_vel` each
                // tick (consumed by `integrate_body`), so `vel` is never
                // integrated and stays `ZERO`.
                vel: ae::Vec2::ZERO,
                size: aabb.half_size() * 2.0,
                facing: 1.0,
            },
            config: BossConfig {
                id: id.into(),
                name,
                spawn: center,
                brain,
                behavior: BossBehaviorProfile::for_authored_boss(&canonical_id),
            },
            status: BossStatus {
                health: crate::actor::Health::new(18),
                alive: true,
                hit_flash: 0.0,
                encounter_phase: BossEncounterPhase::Dormant,
                sprite_metrics: None,
            },
        }
    }

    pub fn as_mut(&mut self) -> BossMut<'_> {
        BossMut {
            kin: &mut self.kin,
            config: &mut self.config,
            status: &mut self.status,
        }
    }

    pub fn as_ref(&self) -> BossRef<'_> {
        BossRef {
            kin: &self.kin,
            config: &self.config,
            status: &self.status,
        }
    }

    /// The three authoritative components as a spawnable Bundle.
    pub fn into_components(self) -> (BodyKinematics, BossConfig, BossStatus) {
        (self.kin, self.config, self.status)
    }
}
