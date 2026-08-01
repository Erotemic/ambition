//! The log as a Bevy resource.
//!
//! ⚠ **this is the SOUND way to record from an ECS**, and the thread-local sink
//! is not: Bevy runs systems across worker threads, so a system publishing
//! through the sink publishes into nothing (see
//! [`crate::facts_lost_offthread`]). A system takes `ResMut<CausalRecording>`.
//!
//! It lives here rather than in a host crate so that ANY domain can publish
//! without depending on a host — which is the property that lets an explanation
//! survive a composition with movement and no combat.

use bevy_ecs::prelude::Resource;

use crate::log::CausalLog;

/// The app's causal log.
#[derive(Resource, Default)]
pub struct CausalRecording(pub CausalLog);

impl std::ops::Deref for CausalRecording {
    type Target = CausalLog;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for CausalRecording {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
