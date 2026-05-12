use bevy::prelude::*;

/// Marker/config component for a spawned parallax background layer.
///
/// `factor` is the fraction of camera motion visible as background drift:
///
/// * `0.0`: locked to the camera, like a skybox.
/// * `0.5`: moves half as fast as world tiles.
/// * `1.0`: behaves like normal world geometry.
#[derive(Component, Clone, Copy, Debug)]
pub struct ParallaxLayer {
    pub factor: Vec2,
    pub offset: Vec2,
    pub z: f32,
}

impl ParallaxLayer {
    pub const fn new(factor: Vec2, offset: Vec2, z: f32) -> Self {
        Self { factor, offset, z }
    }
}

/// Convert a camera translation into the world-space transform for a layer.
///
/// A factor of zero follows the camera exactly, so the layer is screen-locked.
/// A factor of one does not follow the camera, so the layer is world-locked.
pub fn parallax_layer_translation(camera: Vec3, layer: ParallaxLayer) -> Vec3 {
    Vec3::new(
        camera.x * (1.0 - layer.factor.x) + layer.offset.x,
        camera.y * (1.0 - layer.factor.y) + layer.offset.y,
        layer.z,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_factor_tracks_camera_like_skybox() {
        let camera = Vec3::new(100.0, -50.0, 999.0);
        let layer = ParallaxLayer::new(Vec2::ZERO, Vec2::new(3.0, 4.0), -50.0);
        let out = parallax_layer_translation(camera, layer);
        assert_eq!(out, Vec3::new(103.0, -46.0, -50.0));
    }

    #[test]
    fn one_factor_is_world_locked() {
        let camera = Vec3::new(100.0, -50.0, 999.0);
        let layer = ParallaxLayer::new(Vec2::ONE, Vec2::new(3.0, 4.0), -50.0);
        let out = parallax_layer_translation(camera, layer);
        assert_eq!(out, Vec3::new(3.0, 4.0, -50.0));
    }

    #[test]
    fn mid_factor_drifts_slower_than_world() {
        let camera = Vec3::new(100.0, 80.0, 999.0);
        let layer = ParallaxLayer::new(Vec2::splat(0.25), Vec2::ZERO, -40.0);
        let out = parallax_layer_translation(camera, layer);
        assert_eq!(out, Vec3::new(75.0, 60.0, -40.0));
    }
}
