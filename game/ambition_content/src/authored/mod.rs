//! Every character this provider AUTHORS, one file each.
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

use ambition_characters::actor::definition::CharacterDefinition;

mod actor;
mod author;
mod goblin;
mod hall_humanoids;
mod medic;
mod npc_ai_slop;
mod npc_alice;
mod npc_bob;
mod npc_burning_flying_shark;
mod npc_carl_stargan;
mod npc_dividing_mite;
mod npc_emmy_noether;
mod npc_exploding_mite;
mod npc_giant_gnu;
mod npc_giant_gnu_hands;
mod npc_goblin_brute;
mod npc_kernel_guide;
mod npc_lab_raider;
mod npc_ninja_shadow_oni_leader;
mod npc_oiler;
mod npc_pirate_admiral;
mod npc_pirate_crew;
mod npc_pirate_raider;
mod npc_puppy_slug;
mod npc_salvage_guard;
mod officer;
mod perfect_cellular_automaton;
mod pointed_polygon;
mod projectile_polygon;
mod pugnacious_polygon;
mod sandbag;
mod sandbag_infinite;
mod special_patent_clerk;
mod stochastic_parrot;

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
    (&["pointed_polygon"], pointed_polygon::author),
    (&["projectile_polygon"], projectile_polygon::author),
    (&["pugnacious_polygon"], pugnacious_polygon::author),
    (&["npc_exploding_mite"], npc_exploding_mite::author),
    (&["npc_dividing_mite"], npc_dividing_mite::author),
    (&["npc_puppy_slug"], npc_puppy_slug::author),
    (&["stochastic_parrot"], stochastic_parrot::author),
    // the two SNAKE-PLANE swarms left this cast: Mary-O is their
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
    // The hub NPC that arrived for a different reason than everyone else here —
    // an IDENTITY rather than a moveset. See its module doc.
    (&["npc_kernel_guide"], npc_kernel_guide::author),
    (&["npc_salvage_guard"], npc_salvage_guard::author),
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
    // AC5: the last characters that could not build their own body. Alice and Bob left that
    // file the day they grew repertoires — the same rule, the third and fourth time this week.
    // Same walk, same health, sixteen new answers each.
    (&["npc_alice"], npc_alice::author),
    (&["npc_bob"], npc_bob::author),
    (&["npc_emmy_noether"], npc_emmy_noether::author),
    // Oiler left that file the day he grew a repertoire, which is
    // the rule its own doc states. Same walk, same health, sixteen new answers.
    (&["npc_oiler"], npc_oiler::author),
    (&["goblin"], goblin::author),
    // THE FOUR EASTER EGGS. Each is a polygon archetype wearing a different
    // person, and their entries say only what differs from it — see their
    // modules, and `crate::archetype_moveset` for why they borrow the table
    // rather than copying or sharing it.
    //
    // ⚠ Two of the four are hand-drawn rather than faceted, and neither has
    // gameplay rules for her own specials yet: the Medic's ADRENALINE / FIELD
    // DRESSING pair and the Actor's trap door and flyline exist as CLIPS and
    // hit volumes in the sprite repository and as nothing here. They borrow the
    // archetype's specials until someone writes what they cost.
    (&["author"], author::author),
    (&["officer"], officer::author),
    (&["actor"], actor::author),
    (&["medic"], medic::author),
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
