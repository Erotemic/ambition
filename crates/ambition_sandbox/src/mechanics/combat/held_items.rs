//! ECS-owned held item capability for actors.
//!
//! The item component is the durable answer to "what is this actor holding?".
//! Brain/action builders may derive an `ActionSet` from it, projectile visuals can
//! route by its id, and future item drops can read the same component without
//! adding archetype-specific Rust branches.

use bevy::prelude::Component;

/// Runtime component attached to actors that are visibly / mechanically holding
/// an item. The spec is data-authored in `enemy_archetypes.ron` and cloned onto
/// the actor when it spawns or changes state.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct HeldItem {
    pub spec: crate::brain::HeldItemSpec,
}

impl HeldItem {
    pub fn new(spec: crate::brain::HeldItemSpec) -> Self {
        Self { spec }
    }

    pub fn id(&self) -> &str {
        self.spec.id.as_str()
    }
}
