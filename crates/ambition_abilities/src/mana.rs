//! The main game's Mana: the resource the held abilities in this crate spend.
//!
//! A body holds Mana only because the experience that built it declared the
//! pool (the Ambition provider declares it on the home body). A body without
//! it cannot pay a Mana price — absence is never free — and nothing here or in
//! the engine hands a body Mana for existing.
//!
//! The declaration itself is authored data in `ambition_entity_catalog::mana`,
//! re-exported here beside the rules that spend it.

use ambition_platformer2d_core::resources::{ActorResources, ResourceLevel};
use ambition_resource_spec::ResourceCost;

pub use ambition_entity_catalog::mana::{MANA, POOL, REGEN_PER_SEC};

/// Pay `amount` Mana from `bank`; `false`, and nothing paid, when the body
/// holds no Mana or not enough of it.
pub fn spend(bank: Option<&mut ActorResources>, amount: f32) -> bool {
    bank.is_some_and(|bank| bank.pay(&[ResourceCost::new(MANA, amount)]))
}

/// A bank holding only this pool, at its declared start — what the Ambition
/// home body is built with, for a fixture that stands in for one.
pub fn bank() -> ActorResources {
    ActorResources::declared(&[POOL])
        .expect("one declared pool is a valid layout")
        .expect("a declared pool is a bank")
}

/// The body's Mana, or `None` when it holds none — which a reader shows as
/// absent, not as an empty pool.
pub fn level(bank: Option<&ActorResources>) -> Option<ResourceLevel> {
    bank.and_then(|bank| bank.level_of(&MANA))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_resource_spec::{ResourceDeclaration, ResourceId, ResourceStart};

    #[test]
    fn a_body_without_mana_cannot_spend_it() {
        assert!(!spend(None, 1.0));
        let mut other = ActorResources::declared(&[ResourceDeclaration::new(
            ResourceId::from_static("test.other"),
            100.0,
            ResourceStart::Full,
        )])
        .expect("valid")
        .expect("declared");
        assert!(!spend(Some(&mut other), 1.0));
        let mut pool = ActorResources::declared(&[POOL]).expect("valid").expect("declared");
        assert!(spend(Some(&mut pool), 30.0));
        assert_eq!(pool.level_of(&MANA).expect("held").current, 70.0);
    }
}
