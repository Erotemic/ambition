//! `HazardRuntime`: the per-hazard runtime blob (id/name/pos/size, its
//! `DamageVolume`, optional patrol `PathMotion`, and resolve `HitMode`) carried
//! by LDtk-entity hazards. Tile-grid hazards never reach here. The per-frame
//! tick lives in [`hazards`](super::hazards); this module just holds the type
//! and its constructors. Re-exported via `pub use hazard_runtime::*`.

use super::*;

#[derive(Clone, Debug)]
pub struct HazardRuntime {
    pub id: String,
    pub name: String,
    pub pos: ae::Vec2,
    pub size: ae::Vec2,
    pub volume: crate::DamageVolume,
    pub motion: Option<PathMotion>,
    /// How a hit should resolve. Tile-grid hazards (`BlockKind::Hazard`)
    /// run through the engine's reset-to-spawn path and never reach
    /// `HazardRuntime`, so the LDtk-entity hazards we *do* handle here
    /// default to `Knockback`. Authors can still pick `SafeRespawn`
    /// per-volume when an entity hazard is meant to bounce the player
    /// back to safety (e.g. lava pits).
    pub mode: HitMode,
}

impl HazardRuntime {
    /// A hazard at `aabb`. A hazard with a motion path is placed at the
    /// path's seed, not at `aabb`'s center: a prescribed mover's pose and
    /// phase start together. A path that cannot be prepared refuses the hazard.
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        aabb: ae::Aabb,
        volume: crate::DamageVolume,
    ) -> Result<Self, ambition_platformer2d_core::PathFault> {
        let motion = volume.motion.as_ref().map(PathMotion::new).transpose()?;
        Ok(Self {
            id: id.into(),
            name: name.into(),
            pos: motion.as_ref().map_or(aabb.center(), PathMotion::position),
            size: aabb.half_size() * 2.0,
            motion,
            volume,
            mode: HitMode::Knockback,
        })
    }

    pub fn new_with_paths(
        id: impl Into<String>,
        name: impl Into<String>,
        aabb: ae::Aabb,
        mut volume: crate::DamageVolume,
        paths: &[(String, ambition_platformer2d_core::KinematicPath)],
    ) -> Result<Self, ambition_platformer2d_core::PathFault> {
        if let Some(path_id) = volume.path_id.as_deref() {
            if let Some(path) = ambition_platformer2d_core::resolve_kinematic_path(paths, path_id) {
                volume.motion = Some(path.clone());
            }
        }

        Self::new(id, name, aabb, volume)
    }

    pub fn update(&mut self, dt: f32) {
        if let Some(motion) = &mut self.motion {
            self.pos = motion.step(dt);
        }
    }

    pub fn active(&self) -> bool {
        self.volume.enabled
    }

    pub fn aabb(&self) -> ae::Aabb {
        ae::Aabb::new(self.pos, self.size * 0.5)
    }
}

/// ECS wrapper for an authored hazard: the live runtime + its spawn
/// position (for room reset).
#[derive(Component, Clone, Debug)]
pub struct HazardFeature {
    pub hazard: HazardRuntime,
    pub spawn: ae::Vec2,
}

impl HazardFeature {
    pub fn new(hazard: HazardRuntime) -> Self {
        let spawn = hazard.pos;
        Self { hazard, spawn }
    }
}
