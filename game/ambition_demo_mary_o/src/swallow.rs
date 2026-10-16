//! The Laden Swallow: the bird that stays over the player and lets coconuts go.
//!
//! Its row is the flyer the planes are (`Aerial` with no attack range), which
//! flies to a place over the body it sees and never dives. Its action set has
//! one ranged action, the coconut: a shot with gravity and no launch speed to
//! speak of. Its brain says the one thing the row can not say: when to let one
//! go, and that it goes straight down. The brain is an extension module
//! (`ambition_demo_mary_o_modules::laden_swallow`), so an edit to it does not
//! compile this crate.
//!
//! A swallow is squashed from above as a walker is
//! (`ai_slop::bounce_squash_ai_slop`).

use ambition_platformer2d::entity_catalog::placements::CharacterBrain;

/// The brain key a level placement carries beside `character_id`: the key
/// its module binds.
pub use ambition_demo_mary_o_modules::laden_swallow::BRAIN_KEY as LADEN_SWALLOW_BRAIN_KEY;
/// The catalog id. The sheet is the African swallow's.
pub const LADEN_SWALLOW_CHARACTER_ID: &str = "laden_swallow";
/// The `ProjectileVisualId` its coconut authors (`provider.rs` registers it).
pub const COCONUT_VISUAL: &str = "mary_o_coconut";

/// Is this body a Laden Swallow?
pub fn is_laden_swallow_brain(brain: &CharacterBrain) -> bool {
    matches!(brain, CharacterBrain::Custom(key) if key == LADEN_SWALLOW_BRAIN_KEY)
}
