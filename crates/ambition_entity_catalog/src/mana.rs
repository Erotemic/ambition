//! The main game's Mana, as authored data: its identity, its pool and its
//! refill rate.
//!
//! Beside [`crate::smash_limit::LIMIT`] for the same reason: a resource is a
//! content declaration, and the crates that READ it (the dev inspector, the
//! harness observation, the HUD) must be able to name it without linking the
//! abilities that spend it. What spends Mana is `ambition_abilities::mana`.

use ambition_resource_spec::{ResourceDeclaration, ResourceId, ResourceStart};

/// The Mana resource identity.
pub const MANA: ResourceId = ResourceId::from_static("mana");

/// The main game's pool: 100 points, starting (and resetting) full.
pub const POOL: ResourceDeclaration = ResourceDeclaration::new(MANA, 100.0, ResourceStart::Full);

/// The main game's refill rate, in Mana per second of simulation — stated by
/// the composition that declares the pool (`PlayerManaRegen`).
pub const REGEN_PER_SEC: f32 = 14.0;
