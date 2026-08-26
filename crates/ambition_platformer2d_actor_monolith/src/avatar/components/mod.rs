//! Player POLICY components — the state that is genuinely slot-0's, not any
//! body's.
//!
//! The control seam (`LocalPlayer`, `SlotControls`, the slot gesture state)
//! left for `crate::control` / `ambition_characters::brain` in the S5/S6 fold; the body vocabulary
//! (`BodyAnimFacts`, `BodyMelee`) left for `crate::actor`. What remains is
//! camera easing and respawn safety — decisions about the local human's
//! experience, which no other body has.


// Re-export generic player markers from the platformer runtime.
pub use ambition_platformer2d_shared_tangle::markers::{PlayerEntity, PrimaryPlayer};
// Stable facade for the player-slot marker used by brain/player code.
pub use ambition_characters::control::PlayerSlot;

// ⛔ A `///` WITH NOTHING OF ITS OWN TO DOCUMENT. This line survived the wallet's
// move and then silently attached itself to whatever item came next — which was
// `PlayerSafetyState`, an unrelated component. It only became visible when that
// moved out too. A doc comment left behind by a deletion documents the next
// thing, not nothing.
// The body's coin/credits wallet is now `ambition_characters::actor::BodyWallet` (body
// vocabulary — players AND currency-dropping NPCs carry it).

// Player combat/timer state is now the unified `ambition_characters::actor::BodyCombat` (the
// keystone collapse of `BodyCombat` + the actor read-model into one body
// combat component). The player fills the reaction-timer fields; the actor fills
// the status/attack fields.

// Camera easing and blink-in presentation state is now
// `ambition_platformer2d_shared_tangle::camera_ease::PlayerBlinkCameraState`. It
// carried no actor-domain type — four `f32`s and two `Vec2`s — and every reader
// outside this crate (`ambition_sim_view`'s pose/camera snapshots, the runtime's
// reset / room-transition / rollback paths) sits ABOVE this crate, so owning it
// here made the actor crate a way-station on an edge that never needed it.
// Named from its owner; deliberately NOT re-exported.

// ⛔ `PlayerSafetyState` LEFT WITH ITS MECHANIC, 2026-08-26 — to
// `shared_tangle::safe_position`, beside the gate that writes it and the
// cooldown that gates that. The comment above had already written this move
// down for `PlayerBlinkCameraState`, for the same reason and in the same words.

