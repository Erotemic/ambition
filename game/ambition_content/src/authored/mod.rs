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

mod officer;
mod perfect_cellular_automaton;
mod projectile_polygon;

/// Which ids each authoring module speaks for.
///
/// A slice of ids rather than one, because the two cellular automatons are the
/// same authored body under two names. Splitting them into duplicate files
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
