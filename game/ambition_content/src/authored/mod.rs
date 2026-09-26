//! Every character whose body needs a fact its catalog row cannot state yet,
//! one file each. A character that its row states wholly (its gait, health,
//! policy, contact damage, death, action set, traits) is a row in
//! `character_catalog.ron` and has no file here; see
//! `crate::character_catalog::buildable_only_cast`.
//!
//! A creature's facts and the reasons for them belong with the creature, and a match that long
//! is a file every migration has to edit.
//!
//! Adding a character was becoming *edit the catalog data, remember `BUILDABLE_ONLY_CAST`, add
//! a match arm, maybe touch a roster*. That is a registry with extra steps.
//!
//! [`AUTHORED_CAST`] is the ONE table, and it is the module list the compiler
//! already forces you to keep — a character with a file and no entry does not
//! compile into anything, rather than registering as a body that authors
//! nothing.

use ambition_platformer2d::character::CharacterDefinition;

mod npc_burning_flying_shark;
mod npc_companion_dog;
mod npc_giant_gnu;
mod npc_giant_gnu_hands;
mod npc_pirate_admiral;
mod npc_pirate_raider;
mod npc_puppy_slug;
mod officer;
mod perfect_cellular_automaton;
mod projectile_polygon;

/// Which ids each authoring module speaks for.
///
/// A slice of ids rather than one, because a few creatures are genuinely the
/// same authored body under two names — the two cellular automatons, the two
/// plane swarms, the raider and Iron Mary. Splitting those into duplicate files
/// would be the copy this whole move exists to refuse.
pub(crate) const AUTHORED_CAST: &[(
    &[&str],
    fn(&str, CharacterDefinition) -> CharacterDefinition,
)] = &[
    (
        &["perfect_cellular_automaton", "imperfect_cellular_automaton"],
        perfect_cellular_automaton::author,
    ),
    (&["projectile_polygon"], projectile_polygon::author),
    (&["npc_puppy_slug"], npc_puppy_slug::author),
    (&["npc_companion_dog"], npc_companion_dog::author),
    // the two SNAKE-PLANE swarms left this cast: Mary-O is their
    // one provider now (catalog rows + definitions in `ambition_demo_mary_o`),
    // which retired her standalone build's archetype-row fallback.
    (
        &["npc_burning_flying_shark"],
        npc_burning_flying_shark::author,
    ),
    (&["npc_giant_gnu"], npc_giant_gnu::author),
    (
        &["npc_pirate_raider", "npc_pirate_heavy_iron_mary"],
        npc_pirate_raider::author,
    ),
    (&["npc_giant_gnu_hands"], npc_giant_gnu_hands::author),
    (&["npc_pirate_admiral"], npc_pirate_admiral::author),
    // One of the four easter eggs, each a polygon archetype wearing a different
    // person. The other three (the Director, the Performer, the Medic) are
    // catalog rows; the Officer's gun needs a file.
    (&["officer"], officer::author),
];

/// The authoring for `id`, or `None` for a character this provider does not
/// author a body for.
pub(crate) fn author_for(id: &str) -> Option<fn(&str, CharacterDefinition) -> CharacterDefinition> {
    AUTHORED_CAST
        .iter()
        .find(|(ids, _)| ids.contains(&id))
        .map(|(_, author)| *author)
}

/// Every id any module in this directory authors, in table order.
pub(crate) fn authored_ids() -> impl Iterator<Item = &'static str> {
    AUTHORED_CAST
        .iter()
        .flat_map(|(ids, _)| ids.iter().copied())
}
