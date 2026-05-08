use super::*;

#[derive(Clone, Debug)]
pub struct HazardRuntime {
    pub id: String,
    pub name: String,
    pub pos: ae::Vec2,
    pub size: ae::Vec2,
    pub volume: ae::DamageVolume,
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
    pub(super) fn new(object: &ae::RoomObject, volume: ae::DamageVolume) -> Self {
        Self {
            id: object.id.clone(),
            name: object.name.clone(),
            pos: object.aabb.center(),
            size: object.aabb.half_size() * 2.0,
            motion: volume.motion.clone().map(PathMotion::new),
            volume,
            mode: PlayerDamageMode::Knockback,
        }
    }

    pub(super) fn update(&mut self, dt: f32) {
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
