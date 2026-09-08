//! Component bundles for the static feature entities the feature layer owns.
//!
//! A pickup or a chest is not an actor. Their bundles share the rendered feature
//! base with actors (`FeatureBaseBundle`, owned by the spawn capability because an
//! actor body is built on it too), but WHAT a pickup or chest is made of is this
//! layer's fact: it owns the systems that collect and open them.

use ambition_combat::components::{ChestFeature, PickupFeature};
use ambition_platformer2d_actor_spawn::actor_bundles::FeatureBaseBundle;
use ambition_platformer2d_core::CenteredAabb;
use bevy::prelude::*;

/// Bundle for pickup feature entities.
#[derive(Bundle)]
pub struct PickupBundle {
    pub base: FeatureBaseBundle,
    pub pickup: PickupFeature,
}

impl PickupBundle {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        aabb: CenteredAabb,
        pickup: ambition_interaction::Pickup,
    ) -> Self {
        Self {
            base: FeatureBaseBundle::new(id, name, aabb),
            pickup: PickupFeature::new(pickup),
        }
    }
}

/// Bundle for chest feature entities.
#[derive(Bundle)]
pub struct ChestBundle {
    pub base: FeatureBaseBundle,
    pub chest: ChestFeature,
}

impl ChestBundle {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        aabb: CenteredAabb,
        chest: ambition_interaction::Chest,
    ) -> Self {
        Self {
            base: FeatureBaseBundle::new(id, name, aabb),
            chest: ChestFeature::new(chest),
        }
    }
}
