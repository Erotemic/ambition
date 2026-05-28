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
    pub mode: PlayerDamageMode,
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
            mode: PlayerDamageMode::Knockback,
        }
    }

    pub(super) fn new_with_paths(
        id: impl Into<String>,
        name: impl Into<String>,
        aabb: ae::Aabb,
        mut volume: crate::combat::DamageVolume,
        paths: &[(String, crate::actor::KinematicPath)],
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
