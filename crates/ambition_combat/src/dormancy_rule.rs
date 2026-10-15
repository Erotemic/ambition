//! How near an observer must be for a hostile's brain to keep thinking: one
//! game's rule for its own rooms (`room_rules`, field `hostile_wake_radius`).
//! The actor crate applies it (`features::ecs::dormancy`); which bodies may
//! sleep is the engine's decision, not the game's.

/// One game's dormancy rule, declared for the rooms it governs with
/// [`DeclareRulesExt::declare_rules`](crate::scoped_rules::DeclareRulesExt::declare_rules).
/// In a room that no game gave a rule, no brain sleeps.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DormancyRule {
    /// A hostile's brain sleeps while every observer is farther than this, in
    /// world units.
    ///
    /// One number, not a rectangle: the wake test is a distance from a point,
    /// and a screen-shaped region would bake the camera's aspect into the
    /// simulation.
    pub hostile_wake_radius: f32,
}
