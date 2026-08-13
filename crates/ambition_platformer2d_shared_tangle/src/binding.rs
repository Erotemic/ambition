//! **THE BINDING BOUNDARY MOVED OUT OF THIS CRATE — this is only the door.**
//!
//! ⛔⛔ **it was legal here and it was still the wrong floor.** The absence
//! contracts passed: nothing forbids `ambition_characters` depending on
//! `ambition_platformer2d_shared_tangle`. But the boundary's whole
//! implementation is generic Rust — `PhantomData`, `Arc`, a `BTreeMap`, one line
//! of `tracing` — and this crate is **~18k lines across 51 files** of platformer
//! lifecycle, camera, transit, schedules, hotkeys and Bevy state/input. So the
//! canonical character domain sat on top of a grab-bag it used FOUR names from,
//! and an edit to any of those 51 files invalidated it. That is exactly the
//! compile topology the monolith carve is supposed to improve, and legality is
//! not the test (GPT 5.6 review of `1579ab3`, finding 3).
//!
//! ⇒ it lives in [`ambition_binding`] now — a crate whose entire dependency list
//! is `tracing`, below every domain that resolves anything.
//!
//! ⚠ **this re-export is deliberate and is not a shim to delete on sight.** Nine
//! files across `ambition_render`, `ambition_sprite_sheet`, the actor monolith
//! and this crate itself spell the path `shared_tangle::binding::…`; churning
//! them would be a rename campaign, not an architecture change, and the edge
//! that mattered is the one `ambition_characters` no longer has. A crate that
//! wants the boundary WITHOUT the platformer should depend on `ambition_binding`
//! directly, as `ambition_characters` does.

pub use ambition_binding::*;
