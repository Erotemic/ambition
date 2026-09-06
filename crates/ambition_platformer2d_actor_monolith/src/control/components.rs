//! The control-seam STATE: which controlled bodies are local, plus the gesture
//! state owned by each controller slot.
//!
//! ⛔ `LocalPlayer` MOVED OUT ON 2026-09-06 — it is
//! `ambition_platformer2d_shared_tangle::markers::LocalPlayer` now, beside
//! `ControlledSubject` and `PrimaryPlayer`, which is where a zero-field
//! content-free marker belongs. It still says *this body is driven by an input
//! source on this machine*; what changed is that its placement here was the
//! whole of the `avatar -> control` edge in the kernel's cyclic component, and
//! moving it took that component from twelve modules to eleven.
//!
//! The actual per-tick input authority is [`ambition_characters::control::SlotControls`],
//! keyed by the body's [`PlayerSlot`]; it is deliberately not copied onto the
//! body. `SlotGestures` / `SlotInteractionState` are likewise SLOT-level: a
//! gesture belongs to a controller and follows it onto whatever body it drives.


// Player health is now the unified `ambition_characters::actor::BodyHealth` (the keystone
// collapse of the identical `PlayerHealth` / `ActorHealth` wrappers into one
// body-health component).

#[cfg(test)]
#[path = "multiplayer_smoke_tests.rs"]
mod multiplayer_smoke_tests;
