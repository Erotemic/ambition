//! Actor-sim item adapters.
//!
//! The reusable item catalog, shop primitives, and inventory UI state live in
//! `ambition_items` (E8); the PHYSICAL life of a touched collectible lives in
//! `ambition_world_items` (D33, 2026-09-02).
//!
//! ⛔⛔ THIS PARAGRAPH USED TO READ *"the pickup/throw/projectile steppers stay
//! here because they mutate actor bodies, gravity, portals, abilities, and hit
//! events"*, AND THAT SENTENCE WAS LOAD-BEARING SOMEWHERE ELSE: the owner doc
//! quoted it to defend the whole items domain against a carve, concluding a
//! generic hook design was needed first. Measured 2026-09-02, the conclusion is
//! wrong and one clause is simply false.
//!
//! What `pickup` touches, and where each thing lives:
//!
//! ```text
//! bodies    ambition_platformer2d_core::BodyKinematics          BELOW the kernel
//! gravity   shared_tangle::gravity::{GravityCtx, apply_world_forces}   BELOW
//! portals   ambition_portal2d::PortalGun  (#[cfg(feature = "portal")]) BELOW
//! abilities ambition_characters::brain::{ActionSet, HeldItemSpec, …}   BELOW
//! hit events  — zero occurrences of `hit_event` or `HitEvent` anywhere
//! ```
//!
//! ⇒ It reaches all of them through crates BELOW this one, so they are not
//! reasons the code must live here. The 1,344 lines of `pickup/mod.rs` outside
//! its plugin block and `restore_custody_to_checkpoint` name NOTHING in this
//! crate — no `crate::<module>::` path and no `super::`, checked in all three
//! forms.
//!
//! ⚠ WHERE THE OLD SENTENCE WAS TRUE IS THE PLUGIN, not the steppers — and it
//! is no longer true even there. `ItemPickupSimulationPlugin` used to name
//! `crate::abilities::{ranged, traversal, thrown}` and `crate::ability_cooldown`:
//! its NEIGHBOURS' systems, being placed in a schedule. The abilities carve
//! (D33, 2026-09-03) took all of them to `ambition_abilities`, which now
//! configures `ThrownItemEffects` and `WieldedAbilities` end to end. ⇒ The
//! plugin's remaining reach is `crate::shrine`, `crate::items::match_spawn` and
//! the puppy-slug gun, plus the three-variant chain — the one edge that must
//! stay here because it orders sets owned by two other crates.


pub mod conditions;
pub mod match_spawn;
pub mod narrative;
pub mod persist;
pub mod pickup;
/// `wallet.can_afford` — the purse's own published question.
pub mod wallet_conditions;

// ⭐ `world_item` AND `item_motion` LEFT THIS CRATE (D33, 2026-09-02) and are
// `ambition_world_items` now — the physical life of a touched collectible: where
// it is, whether it is moving, and that touching it collects it. Nothing is
// re-exported here on purpose. A re-export would keep this module as the
// discovery path for something it no longer owns, and the point of the carve is
// that a consumer names the crate that has the code.
//
// ⭐ AND THE SIBLING FOLLOWED (D33, 2026-09-03): the PRESSED pickup is
// `ambition_held_items` now. What `pickup` keeps is the KERNEL's — the plugin
// that chains the three `ItemPickupSet` variants and attaches the shrine, the
// gun and the match spawn to the domain's `HeldItemStep`s, the checkpoint
// policy over the domain's components, and `minted_horizon`. Same rule: nothing
// re-exported; games reach the domain through the facade's `held_items`.
