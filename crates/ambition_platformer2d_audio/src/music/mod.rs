//! Game facts select one owned music goal.
mod intent;
#[cfg(test)]
mod tests;
pub use intent::{compute_music_intent, release_narrative_music_on_room_change};
