//! The wave-mob spawn REQUEST vocabulary.
//!
//! ⭐ THIS LIVES IN THE ENCOUNTER DOMAIN, not the actor kernel (moved
//! 2026-09-03). A wave mob is described BY an encounter and constructed BY the
//! kernel: the description is this crate's vocabulary and the body assembly is
//! the kernel's, which is the orchestration/construction line the decomposition
//! doctrine draws. While the type lived in the kernel, an encounter could not
//! say what it wanted spawned without naming
//! `ambition_platformer2d_actor_monolith` — so an ADAPTER inside the kernel had
//! to say it on the domain's behalf.
//!
//! ⛔ It carries NO kernel types and never should: `String`, `&str`,
//! `ambition_entity_catalog::placements::CharacterBrain` and
//! `ambition_platformer2d_core::Vec2`, all from crates below both sides. That is
//! what makes it movable at all.

use ambition_platformer2d_core as ae;

/// One encounter wave mob, as the wave director describes it.
///
///  the three questions a body's identity answers, and they are separate.
/// The vocabulary is deliberately [`ambition_platformer2d_world::rooms::EnemySpawnSpec`]'s, the
/// neighbouring spawn path, so the two structs read against each other:
///
/// | question | here | `EnemySpawnSpec` |
/// |---|---|---|
/// | what it LOOKS LIKE | `character` | `character_id` |
/// | what it DOES | `brain` | `brain` |
/// | which BODY | `id` | the authored placement's own id |
///
///  a struct rather than five more positional arguments, because the
/// interesting value here is `character: None` — and a bare `None` in argument
/// position 8 tells a reader nothing about which of three questions was
/// declined.
pub struct EncounterMobSeed<'a> {
    /// WHICH BODY. Minted per spawn by the wave director
    /// (`encounter:<trigger>:w<wave>:<n>`) so ids never collide across attempts,
    /// and the key the encounter's own `FeatureId` liveness refresh looks a mob
    /// up by.  never the character: two goblins in one wave are two bodies.
    pub id: String,
    /// WHAT IT LOOKS LIKE. A catalog character id — art only, exactly as far
    /// as [`ambition_platformer2d_world::rooms::EnemySpawnSpec::character_id`] reaches: the sheet, the
    /// sprite-derived collision box, hurt feedback, and the display label its
    /// banners and barks are keyed by.  it does NOT select the catalog's
    /// `default_brain` or `default_action_set` — `brain` below does that, and
    /// whether an enemy IS a character or merely WEARS one is an open design
    /// question, not something this field quietly answers.
    ///
    /// `None` is the older road and stays open: an encounter assembled from LDtk
    /// `EnemySpawn` markers that name no `character_id` has no character to give.
    pub character: Option<&'a str>,
    /// WHAT IT DOES. The roster archetype key, as
    /// `CharacterBrain::Custom(kind)` — health, speed, reach, melee/ranged kit.
    pub brain: ambition_entity_catalog::placements::CharacterBrain,
    /// Spawn centre, world space.
    pub pos: ae::Vec2,
    /// Body size. A HINT: a named character resizes to its authored sprite's
    /// collision, the same as a peaceful NPC of that character.
    pub size: ae::Vec2,
}
