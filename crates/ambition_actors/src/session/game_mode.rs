//! Compatibility facade for Ambition's coarse gameplay/session state.
//!
//! The canonical `GameMode` vocabulary lives in
//! [`ambition_platformer_primitives::schedule`] next to the runtime schedule
//! labels. This facade remains so actor-internal modules and older callers can
//! be repointed incrementally without owning the type here.

pub use ambition_platformer_primitives::schedule::{
    gameplay_allowed, gameplay_suspended, GameMode,
};
