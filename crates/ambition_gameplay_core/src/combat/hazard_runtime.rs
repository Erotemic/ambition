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
    pub volume: crate::combat::DamageVolume,
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
    pub(super) fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        aabb: ae::Aabb,
        volume: crate::combat::DamageVolume,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            pos: aabb.center(),
            size: aabb.half_size() * 2.0,
            motion: volume.motion.clone().map(PathMotion::new),
            volume,
            mode: HitMode::Knockback,
        }
    }

    pub(super) fn new_with_paths(
        id: impl Into<String>,
        name: impl Into<String>,
        aabb: ae::Aabb,
        mut volume: crate::combat::DamageVolume,
        paths: &[(String, ambition_engine_core::KinematicPath)],
    ) -> Self {
        if let Some(path_id) = volume
            .path_id
            .as_deref()
            .map(str::trim)
            .filter(|path_id| !path_id.is_empty())
        {
            if let Some((_, path)) = paths.iter().find(|(p_id, _)| p_id == path_id) {
                volume.motion = Some(path.clone());
            }
        }

        let mut hazard = Self::new(id, name, aabb, volume);
        if let Some(start_pos) = hazard.motion.as_ref().and_then(PathMotion::start_pos) {
            hazard.pos = start_pos;
        }
        hazard
    }

    pub(crate) fn update(&mut self, dt: f32) {
        if let Some(motion) = &mut self.motion {
            self.pos = motion.advance(self.pos, dt);
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
