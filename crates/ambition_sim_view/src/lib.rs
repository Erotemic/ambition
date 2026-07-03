//! Sim → presentation **read-model vocabulary**: the small, pure-data snapshot
//! types the renderer consumes each frame, extracted from
//! `ambition_gameplay_core` (fable review D3).
//!
//! The simulation crate WRITES these (its per-frame view builders materialize
//! them from live ECS state); the renderer READS them. Homing them here — a
//! leaf crate depending only on `ambition_engine_core` + `bevy`'s ECS derive —
//! is the seam that lets presentation stop depending on the 95k gameplay crate:
//! render names `ambition_sim_view::FeatureView` instead of
//! `ambition_gameplay_core::features::FeatureView`.
//!
//! **Scope rule:** only genuinely presentation-facing, dependency-light *value*
//! types belong here. Live-query views (`ActorSpriteData`, `FeatureViewIndex`)
//! stay in the sim crate — they borrow its ECS components and are not
//! transferable vocabulary.

use ambition_engine_core::Vec2;
use bevy::prelude::Component;

/// The kit-level visual taxonomy for a room feature — what a renderer needs to
/// know to pick a sprite/color/z for any authored feature. The *depiction* is
/// content (a sandbag for `TrainingDummy`, a colored block for `Switch`); the
/// kind itself is generic sim→presentation vocabulary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FeatureVisualKind {
    Hazard,
    Enemy,
    /// A passive practice target (struck to test damage/feedback). Rendered with
    /// a sandbag sprite — the depiction is content; the kit kind is generic.
    TrainingDummy,
    Boss,
    Breakable,
    Chest,
    Pickup,
    Npc,
    /// Latched switch. Renders as a colored block whose color depends
    /// on `FeatureView::switch_on` (red = off, green = on).
    Switch,
}

/// Marker binding a feature visual to its kind + collision size (moved here from
/// the render layer so the mount gameplay can remove it without importing presentation).
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct BoundFeatureKind {
    pub kind: FeatureVisualKind,
    pub collision_size: Vec2,
}

impl BoundFeatureKind {
    pub fn new(kind: FeatureVisualKind, collision: bevy::math::Vec2) -> Self {
        Self {
            kind,
            collision_size: Vec2::new(collision.x, collision.y),
        }
    }

    pub fn matches(&self, kind: FeatureVisualKind, collision_size: Vec2) -> bool {
        self.kind == kind && (self.collision_size - collision_size).length_squared() <= 0.25
    }
}

/// The per-feature render snapshot: everything a renderer needs to draw one
/// authored room feature this frame. Materialized by the sim's view builder.
#[derive(Clone, Copy, Debug)]
pub struct FeatureView {
    pub pos: Vec2,
    pub size: Vec2,
    pub kind: FeatureVisualKind,
    pub visible: bool,
    pub flash: bool,
    /// For `FeatureVisualKind::Switch`: true when the switch reads as
    /// "on" (encounter cleared / reset path armed). Renders green when
    /// true, red when false. Ignored for other kinds.
    pub switch_on: bool,
    /// Z-axis rotation to apply to the rendered sprite, in radians
    /// (Bevy frame; +π/2 is CCW). Non-zero for surface-walking
    /// archetypes that crawl on walls/ceilings; everyone else
    /// reports 0.0 and renders axis-aligned. Uses the engine → Bevy
    /// rotation mapping shared by actor rendering.
    pub rotation_rad: f32,
}

/// Attack-phase timing snapshot (seconds) the renderer reads to time
/// windup/active tints.
#[derive(Clone, Copy, Debug)]
pub struct FeatureCombatTuning {
    pub enemy_attack_windup: f32,
    pub enemy_attack_active: f32,
    pub boss_attack_windup: f32,
    pub boss_attack_active: f32,
}

/// Default attack-phase timings (seconds). Single source of truth, shared by
/// [`FeatureCombatTuning::default`] and `SandboxFeelTuning::default` (which
/// projects them back out via `SandboxFeelTuning::feature_combat_tuning`).
pub const DEFAULT_ENEMY_ATTACK_WINDUP: f32 = 0.36;
pub const DEFAULT_ENEMY_ATTACK_ACTIVE: f32 = 0.20;
pub const DEFAULT_BOSS_ATTACK_WINDUP: f32 = 0.52;
pub const DEFAULT_BOSS_ATTACK_ACTIVE: f32 = 0.32;

impl Default for FeatureCombatTuning {
    fn default() -> Self {
        Self {
            enemy_attack_windup: DEFAULT_ENEMY_ATTACK_WINDUP,
            enemy_attack_active: DEFAULT_ENEMY_ATTACK_ACTIVE,
            boss_attack_windup: DEFAULT_BOSS_ATTACK_WINDUP,
            boss_attack_active: DEFAULT_BOSS_ATTACK_ACTIVE,
        }
    }
}
