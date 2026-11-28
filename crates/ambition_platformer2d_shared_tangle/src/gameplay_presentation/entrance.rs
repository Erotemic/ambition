use bevy::prelude::Component;

/// Selects how the renderer introduces a new runtime actor.
///
/// Add this component to the body when you spawn a wave mob, staged enemy,
/// or post-boss NPC. The renderer shows these actors immediately when this
/// component is absent.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActorEntrance {
    /// Increase the opacity of the whole actor over a short time.
    FadeIn,
}
