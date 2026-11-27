//! Which bodies a pinch has squashed, and how flat (crushing C4/C6): the read
//! model presentation draws a flattened body from, keyed by the body (a
//! player's picture is its body) and by its feature id (an actor's picture is
//! a visual of that id). Rebuilt each frame; empty when nothing is squashed.

use std::collections::HashMap;

use bevy::prelude::*;

use ambition_combat::components::FeatureId;

/// Each squashed body's size over its rest size, per axis (1 at rest).
#[derive(Resource, Default, Clone, Debug)]
pub struct SquashedViews {
    pub by_body: HashMap<Entity, Vec2>,
    pub by_feature: HashMap<String, Vec2>,
}

impl SquashedViews {
    pub fn is_empty(&self) -> bool {
        self.by_body.is_empty()
    }
}

pub fn rebuild_squashed_views(
    mut view: ResMut<SquashedViews>,
    bodies: Query<(Entity, &ambition_platformer2d_core::crush::Squash, &ambition_platformer2d_core::BodyKinematics, Option<&FeatureId>)>,
) {
    if bodies.is_empty() && view.is_empty() {
        return;
    }
    view.by_body.clear();
    view.by_feature.clear();
    for (entity, squash, kin, feature) in &bodies {
        let ratio = squash.ratio(kin.canonical_size);
        let ratio = Vec2::new(ratio.x, ratio.y);
        view.by_body.insert(entity, ratio);
        if let Some(feature) = feature {
            view.by_feature.insert(feature.0.clone(), ratio);
        }
    }
}
