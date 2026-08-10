//! **What a character does when it dies** — authored data, not a live component.
//!
//! ⭐ **why this type exists at all, given that it mirrors
//! `ambition_combat::CombatCapabilities` field for field.** A character
//! definition must be able to state that an exploding mite explodes. Stating it
//! by owning the runtime combat component would make the AUTHORING type depend
//! on the runtime ECS layer — and since `ambition_combat` already depends on
//! this crate, moving the definition down here with that field would close a
//! dependency cycle.
//!
//! The crate boundary is the design test, and it answered: if an authored fact
//! needs a runtime type to say it, the fact was modelled at the wrong level.
//! [`CharacterDeathTraits`] is the lower semantic fact; construction LOWERS it
//! into the runtime component.
//!
//! ```text
//! CharacterDefinition.death_traits : CharacterDeathTraits   (authored)
//!         ↓  at construction
//! ambition_combat::CombatCapabilities                       (runtime)
//! ```
//!
//! ⚠ **the mirroring is not duplication, it is a direction.** The runtime
//! component may grow fields nobody authors (state a system computes), and this
//! may grow fields the runtime folds into something else. They are the same
//! today because the five facts happen to be purely authored today.
//!
//! See `docs/planning/character-template-architecture-2026-08-10.md`.

/// The authored on-death behaviour of a character template.
///
/// Every field defaults to "nothing special", so a character that says nothing
/// about dying gets the ordinary death — which is what almost every character
/// wants and why the whole struct is `Option` on a definition.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CharacterDeathTraits {
    /// Detonates at the corpse on death, so a point-blank kill is punished.
    pub explodes_on_death: bool,
    /// Splits into offspring on death.
    pub divides_on_death: bool,
    /// A fast charge stopped dead by a wall destroys this body.
    pub charge_crash_explodes: bool,
    /// Damage never kills — a training dummy with an effectively infinite pool.
    pub never_dies: bool,
    /// A weapon left at the corpse as a wieldable ground item: the "steal the
    /// enemy's weapon" rule.
    pub drops_held_item: Option<crate::brain::HeldItemSpec>,
}
