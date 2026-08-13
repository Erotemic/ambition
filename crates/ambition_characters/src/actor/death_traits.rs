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
    /// **What this body splits into on death, if anything.**
    ///
    /// ⭐⭐ **it names the OFFSPRING, and it used to be a bare `bool`** (AC5.4,
    /// D102). The engine's split path carried the answer instead — a literal
    /// `"SmallSkitter"`, later `"npc_puppy_slug"`, compiled into a reusable
    /// platformer that has no business knowing what an Ambition mite divides
    /// into. Any other game linking the engine inherited that creature name, and
    /// changing what a mite becomes meant editing the engine.
    ///
    /// ⇒ a character states what it becomes. `None` is "does not divide", which
    /// is what every body that says nothing gets.
    pub divides_into: Option<String>,
    /// A fast charge stopped dead by a wall destroys this body.
    pub charge_crash_explodes: bool,
    /// Damage never kills — a training dummy with an effectively infinite pool.
    ///
    /// ⚠ **not an on-death consequence; a MORTALITY policy.** Its consumer is
    /// the damage resolver (`damage_apply`), which decides whether a hit kills
    /// at all, so it sits one step before the other four rather than beside
    /// them. Grouped here because it is the same kind of authored character
    /// fact and has the same one consumer family; if this struct ever grows a
    /// second mortality knob, that is the moment to split them.
    pub never_dies: bool,
    /// Whether this body leaves what it is holding at the corpse, as a wieldable
    /// ground item: the "steal the enemy's weapon" rule.
    ///
    /// ⭐ **a POLICY, not an item, and the difference was a bug.** It used to be
    /// `Option<HeldItemSpec>` — the character's INTRINSIC weapon, snapshotted at
    /// construction — so a body that picked up a different weapon still dropped
    /// the one it was authored with. `ambition_combat::held_items` owns the live
    /// answer and its module doc named this exact consumer: *"future item drops
    /// can read the same component without adding archetype-specific Rust
    /// branches."* The drop path now reads it.
    ///
    /// ⇒ the character says WHETHER it drops; the body says WHAT it is holding.
    pub drops_held_item: bool,
}
