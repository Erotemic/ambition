//! **Every character this provider AUTHORS, one file each.**
//!
//! ⭐⭐ **A CHARACTER'S BODY LIVES BESIDE ITS MOVESET, NOT IN A CENTRAL MATCH.**
//! This was one 850-line `match id` in `character_catalog.rs` — nineteen arms,
//! each a different creature's vitals, locomotion, abilities and autonomous
//! policy, and each carrying the migration note explaining what archetype row it
//! replaced. A creature's facts and the reasons for them belong with the
//! creature, and a match that long is a file every migration has to edit.
//!
//! ⛔ **the shape this refuses is the one D73 was written to avoid**: the
//! archetype table is nearly gone (`character_archetypes.ron` is down to
//! `combatant` and `medium_striker`), but its AUTHORITY does not evaporate with
//! it — it moves into whatever central structure grows to replace it. Adding a
//! character was becoming *edit the catalog data, remember
//! `BUILDABLE_ONLY_CAST`, add a match arm, maybe touch a roster*. That is a
//! registry with extra steps.
//!
//! ⚠ [`AUTHORED_CAST`] is the ONE table, and it is the module list the compiler
//! already forces you to keep — a character with a file and no entry does not
//! compile into anything, rather than registering as a body that authors
//! nothing.

use ambition_platformer2d_actor_monolith::character_runtime::CharacterDefinition;

mod goblin;
mod hall_humanoids;
mod npc_ai_slop;
mod npc_burning_flying_shark;
mod npc_carl_stargan;
mod npc_dividing_mite;
mod npc_exploding_mite;
mod npc_giant_gnu;
mod npc_goblin_brute;
mod npc_giant_gnu_hands;
mod npc_lab_raider;
mod npc_ninja_shadow_oni_leader;
mod npc_pirate_admiral;
mod npc_pirate_crew;
mod npc_pirate_raider;
mod npc_puppy_slug;
mod npc_salvage_guard;
mod perfect_cellular_automaton;
mod sandbag;
mod sandbag_infinite;
mod special_patent_clerk;
mod stochastic_parrot;

/// **Which ids each authoring module speaks for.**
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
    (&["npc_exploding_mite"], npc_exploding_mite::author),
    (&["npc_dividing_mite"], npc_dividing_mite::author),
    (&["npc_puppy_slug"], npc_puppy_slug::author),
    (&["stochastic_parrot"], stochastic_parrot::author),
    // ⭐ the two SNAKE-PLANE swarms left this cast 2026-08-13: Mary-O is their
    // one provider now (catalog rows + definitions in `ambition_demo_mary_o`),
    // which retired her standalone build's archetype-row fallback.
    (&["npc_ai_slop"], npc_ai_slop::author),
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
    (&["sandbag"], sandbag::author),
    (&["sandbag_infinite"], sandbag_infinite::author),
    (&["special_patent_clerk"], special_patent_clerk::author),
    (
        &["npc_ninja_shadow_oni_leader"],
        npc_ninja_shadow_oni_leader::author,
    ),
    (&["npc_pirate_admiral"], npc_pirate_admiral::author),
    (&["npc_lab_raider"], npc_lab_raider::author),
    (&["npc_salvage_guard"], npc_salvage_guard::author),
    // AC4: the six pirates and Carl Stargan, whose bodies were the whole of
    // `REGISTERED_WITHOUT_A_BODY`. Jon's 2026-08-13 rulings unblocked both — see
    // each module's doc for which decision it consumes.
    (
        &[
            "npc_pirate_cutlass_viper",
            "npc_pirate_heavy_broadside_bess",
            "npc_pirate_heavy_salt_annet",
            "npc_pirate_lookout",
            "npc_pirate_navigator",
            "npc_pirate_quartermaster",
        ],
        npc_pirate_crew::author,
    ),
    (&["npc_carl_stargan"], npc_carl_stargan::author),
    // AC5: the last four characters that could not build their own body. Each
    // was missing only locomotion — see the module doc.
    (
        &["npc_alice", "npc_bob", "npc_noether", "npc_oiler"],
        hall_humanoids::author,
    ),
    (&["goblin"], goblin::author),
    // AC6/D102: Jon's 2026-08-13 casting of `large_brute` as a real character.
    (&["npc_goblin_brute"], npc_goblin_brute::author),
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
