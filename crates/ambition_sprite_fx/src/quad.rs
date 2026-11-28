use bevy::math::Vec2;
use bevy::transform::components::Transform;

/// Get opposite quad corners relative to the sprite's normalized anchor.
pub fn anchored_quad_corners(size: Vec2, anchor: Vec2) -> (Vec2, Vec2) {
    let half = Vec2::splat(0.5);
    ((-half - anchor) * size, (half - anchor) * size)
}

/// Place a centered unit mesh over a sprite. Apply the sprite's scale to
/// the mesh size and the offset from the sprite anchor.
pub fn place_anchored_quad(base: &Transform, anchor: Vec2, size: Vec2) -> Transform {
    Transform {
        translation: base.transform_point((-anchor * size).extend(0.0)),
        scale: (size * base.scale.truncate()).extend(base.scale.z),
        ..*base
    }
}
