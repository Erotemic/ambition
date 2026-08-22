//! THE BINDING BOUNDARY MOVED OUT OF THIS CRATE — this is only the door.
//!
//!  it lives in [`ambition_binding`] now — a crate whose entire dependency list
//! is `tracing`, below every domain that resolves anything.
//!
//! A crate that wants the boundary WITHOUT the platformer should depend on `ambition_binding`
//! directly, as `ambition_characters` does.

pub use ambition_binding::*;
