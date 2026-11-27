//! A squashed body's picture (crushing C6): drawn flat along the axis its
//! pinch squashed, and a little wider across it, so a flattened body reads as
//! flattened and not as a smaller one. Its box is the simulation's; this is
//! only how it is drawn, read from [`SquashedViews`].

use bevy::prelude::*;

use ambition_platformer2d_shared_tangle::lifecycle::PlayerVisual;
use ambition_sim_view::squash_view::SquashedViews;

use super::primitives::FeatureVisual;

/// How much wider a body is drawn across its squash: the square root of how
/// flat it is, so a body squashed to a quarter is drawn twice as wide.
fn drawn_scale(ratio: Vec2) -> Vec2 {
    let ratio = ratio.max(Vec2::splat(0.05));
    Vec2::new(ratio.x / ratio.y.sqrt(), ratio.y / ratio.x.sqrt())
}

/// Scale each squashed body's picture, after the bodies are placed.
pub(crate) fn present_squashed_bodies(
    squashed: Res<SquashedViews>,
    mut players: Query<(Entity, &mut Transform), With<PlayerVisual>>,
    mut features: Query<(&FeatureVisual, &mut Transform), Without<PlayerVisual>>,
) {
    if squashed.is_empty() {
        return;
    }
    for (entity, mut transform) in &mut players {
        if let Some(ratio) = squashed.by_body.get(&entity) {
            let scale = drawn_scale(*ratio);
            transform.scale.x *= scale.x;
            transform.scale.y *= scale.y;
        }
    }
    if squashed.by_feature.is_empty() {
        return;
    }
    for (visual, mut transform) in &mut features {
        if let Some(ratio) = squashed.by_feature.get(visual.id.as_str()) {
            let scale = drawn_scale(*ratio);
            transform.scale.x *= scale.x;
            transform.scale.y *= scale.y;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A body at rest is drawn as it is; one squashed to a quarter of its
    /// height is drawn a quarter as tall and twice as wide.
    #[test]
    fn a_squashed_body_is_drawn_flat_and_wide() {
        assert_eq!(drawn_scale(Vec2::ONE), Vec2::ONE);
        let flat = drawn_scale(Vec2::new(1.0, 0.25));
        assert!((flat.x - 2.0).abs() < 1.0e-5 && (flat.y - 0.25).abs() < 1.0e-5, "{flat:?}");
    }

    /// The pass scales a squashed player's picture and a squashed actor's
    /// visual (found by its feature id), and leaves the others alone.
    #[test]
    fn squashed_pictures_are_scaled_and_others_are_not() {
        let mut app = App::new();
        app.add_systems(Update, present_squashed_bodies);
        let player = app.world_mut().spawn((PlayerVisual, Transform::default())).id();
        let slug_visual = app.world_mut().spawn((FeatureVisual { id: "slug".into() }, Transform::default())).id();
        let other_visual = app.world_mut().spawn((FeatureVisual { id: "other".into() }, Transform::default())).id();
        let mut squashed = SquashedViews::default();
        squashed.by_body.insert(player, Vec2::new(1.0, 0.25));
        squashed.by_feature.insert("slug".into(), Vec2::new(1.0, 0.5));
        app.insert_resource(squashed);
        app.update();
        let scale = |app: &App, entity| app.world().get::<Transform>(entity).unwrap().scale;
        assert!((scale(&app, player).y - 0.25).abs() < 1.0e-5, "{:?}", scale(&app, player));
        assert!((scale(&app, slug_visual).y - 0.5).abs() < 1.0e-5, "{:?}", scale(&app, slug_visual));
        assert_eq!(scale(&app, other_visual), Vec3::ONE);
    }
}
