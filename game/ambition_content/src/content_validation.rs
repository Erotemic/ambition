//! Cross-content validation for authored sandbox data.
//!
//! This module checks relationships that live *between* content systems: LDtk
//! room links, NPC dialogue ids, quest conditions, encounter/boss ids, and
//! music references. The intent is to catch content typos at startup/test time
//! instead of letting string ids silently fall back or never fire.

use std::collections::{BTreeMap, BTreeSet};

use crate::data::MusicRegistry;
use ambition_encounter::encounter_reward_looted_flag;
use ambition_platformer2d_ldtk::{field_string, LdtkProject};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ContentValidationReport {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl ContentValidationReport {
    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn push_error(&mut self, message: impl Into<String>) {
        self.errors.push(message.into());
    }

    #[allow(dead_code)] // Used by content checks that haven't been wired into startup yet.
    pub fn push_warning(&mut self, message: impl Into<String>) {
        self.warnings.push(message.into());
    }

    pub fn extend_errors<I>(&mut self, messages: I)
    where
        I: IntoIterator<Item = String>,
    {
        self.errors.extend(messages);
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn panic_if_errors(&self) {
        if self.errors.is_empty() {
            return;
        }
        panic!(
            "content graph validation failed:\n{}",
            self.errors
                .iter()
                .map(|error| format!("- {error}"))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}

/// Validate the checked-in sandbox content graph.
#[cfg_attr(not(test), allow(dead_code))]
pub fn validate_embedded_content_graph() -> ContentValidationReport {
    let music = crate::audio_registries::load_music_registry();
    let project = match LdtkProject::load_default_for_dev(&crate::worlds::world_manifest()) {
        Ok(project) => project,
        Err(error) => {
            let mut report = ContentValidationReport::default();
            report.push_error(format!("failed to load embedded LDtk project: {error}"));
            return report;
        }
    };
    let character_catalog = crate::character_catalog::load_catalog();
    validate_content_graph(&music, &project, &character_catalog)
}

/// Validate relationships among the music registry and the LDtk world
/// (room/encounter/boss music references, dialogue, quests, patrols).
pub fn validate_content_graph(
    music: &MusicRegistry,
    project: &LdtkProject,
    character_catalog: &ambition_characters::actor::character_catalog::CharacterCatalog,
) -> ContentValidationReport {
    let mut report = ContentValidationReport::default();

    if let Err(error) = music.validate() {
        report.push_error(format!("music registry invalid: {error}"));
    }

    let ldtk_report = project
        .validate(&ambition_platformer2d_ldtk::LdtkVocabulary::engine());
    report.extend_errors(
        ldtk_report
            .errors
            .into_iter()
            .map(|error| format!("LDtk validation: {error}")),
    );
    report.warnings.extend(
        ldtk_report
            .warnings
            .into_iter()
            .map(|warning| format!("LDtk validation: {warning}")),
    );

    validate_ldtk_room_links(project, &mut report);
    validate_room_music_tracks(project, music, &mut report);
    validate_npc_dialogue_ids(project, character_catalog, &mut report);
    validate_npc_brain_overrides(project, character_catalog, &mut report);
    validate_quest_conditions(project, music, &mut report);
    let boss_catalog = crate::bosses::authored_boss_catalog();
    validate_boss_music_tracks(music, &boss_catalog, &mut report);
    validate_patrol_brain_paths(project, &mut report);

    #[cfg(feature = "audio")]
    validate_adaptive_music_catalog(&mut report);

    report
}

/// Catch the failure mode from intro-v1 polish E: an `EnemySpawn`
/// with `brain: "Patrol:<path_id>"` whose `path_id` doesn't resolve
/// to a `KinematicPath` in the same level. The runtime silently
/// falls back to passive behavior, so the broken patrol is invisible
/// until playtest. Surfacing it as a content-graph warning catches
/// it at `cargo test` time instead.
///
/// ⛔ **it does not MIRROR the resolution rule any more, it ASKS it.** This
/// check used to re-derive both halves — conversion's lookup id and the alias
/// set a reference may be spelled with — and both copies drifted from the
/// originals, in opposite directions: it registered the plain name slug that
/// conversion collapses away, and it dropped the display name and slug entirely
/// whenever a path authored an explicit `id`. So it called a working reference
/// broken (a hard startup abort on shippable content) and, for sandbox's
/// basement patrol, called a broken one fine. It now calls
/// `kinematic_path_lookup_id` for the id conversion WILL produce and
/// `kinematic_path_aliases` for the spellings that id answers to — the same two
/// functions the runtime uses.
///
/// ⭐ **THESE ARE ERRORS NOW, which is what this validator always wanted.** It
/// emitted warnings only because one authored mismatch existed when it was
/// written — sandbox's basement `Patrol:enemy_patrol_a` against a path whose
/// name slugged to `enemy_patrol_path_a` — and its own doc said *"promote to an
/// error once the slugs are aligned"*.
///
/// ⛔⛔ **AND THE SENTENCE AFTER THAT MEASUREMENT WAS WRONG.** "Zero patrol
/// warnings, so the mismatch is gone" was read off a validator that had become
/// the wrong oracle: the brain had been rewritten from `enemy_patrol_a` to
/// `enemy_patrol_path_a` — the spelling THIS check derived — which silenced it
/// while leaving the runtime's own lookup table, which knew only the compacted
/// `enemy_patrol_a`, unable to resolve it. The count was correct and the
/// conclusion was not. The slugs were never aligned; they were aligned to each
/// other in one direction, and the patroller stood still from that day until
/// 2026-08-14. Silence from an oracle that derives its own answer is not
/// evidence about the thing it is meant to be watching.
///
/// ⛔ the failure it catches is silent by construction: a patrol whose path does
/// not resolve falls back to PASSIVE, so the enemy simply stands there and the
/// level looks finished. That is exactly the class of defect a content check
/// should refuse to let ship, not mention.
fn validate_patrol_brain_paths(project: &LdtkProject, report: &mut ContentValidationReport) {
    for level in &project.levels {
        let mut path_ids: BTreeSet<String> = BTreeSet::new();
        for entity in level.all_entity_instances() {
            if entity.identifier != "KinematicPath" {
                continue;
            }
            // The display name conversion resolves, then the id it derives from
            // it, then every spelling that id answers to. Three asks, no copies.
            let name = field_string(entity, "name").unwrap_or_else(|| entity.identifier.clone());
            let lookup_id = ambition_platformer2d_ldtk::kinematic_path_lookup_id(entity, &name);
            path_ids.extend(
                ambition_platformer2d_world::rooms::kinematic_path_aliases(&lookup_id, &name)
                    .filter(|alias| !alias.is_empty())
                    .map(|alias| alias.into_owned()),
            );
        }
        for entity in level.all_entity_instances() {
            if entity.identifier != "EnemySpawn" {
                continue;
            }
            let Some(brain) = field_string(entity, "brain") else {
                continue;
            };
            let Some(path_id) = brain.strip_prefix("Patrol:") else {
                continue;
            };
            let path_id = path_id.trim();
            if path_id.is_empty() {
                report.push_error(format!(
                    "level '{}' EnemySpawn '{}' uses bare brain 'Patrol:' (no path_id); enemy will fall back to passive",
                    level.identifier, entity.iid
                ));
                continue;
            }
            if !path_ids.contains(path_id) {
                report.push_error(format!(
                    "level '{}' EnemySpawn '{}' brain 'Patrol:{}' references no matching KinematicPath (resolved ids: {:?}); enemy will fall back to passive",
                    level.identifier, entity.iid, path_id, path_ids
                ));
            }
        }
    }
}

fn validate_ldtk_room_links(project: &LdtkProject, report: &mut ContentValidationReport) {
    let mut area_level_count: BTreeMap<String, usize> = BTreeMap::new();
    let mut zones_by_area: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut links = Vec::new();

    for level in &project.levels {
        let area = level.active_area();
        *area_level_count.entry(area.clone()).or_default() += 1;
        for entity in level.all_entity_instances() {
            if entity.identifier != "LoadingZone" {
                continue;
            }
            let zone_id = field_string(entity, "id").unwrap_or_else(|| entity.iid.clone());
            if zone_id.trim().is_empty() {
                report.push_error(format!(
                    "level '{}' has LoadingZone '{}' with a blank id",
                    level.identifier, entity.iid
                ));
                continue;
            }
            if !zones_by_area
                .entry(area.clone())
                .or_default()
                .insert(zone_id.clone())
            {
                report.push_error(format!(
                    "active area '{}' has duplicate LoadingZone id '{}'",
                    area, zone_id
                ));
            }
            links.push((
                level.identifier.clone(),
                area.clone(),
                zone_id,
                field_string(entity, "target_room"),
                field_string(entity, "target_zone"),
            ));
        }
    }

    for (level_id, area, zone_id, target_room, target_zone) in links {
        let target_room = target_room
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        let target_zone = target_zone
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        match (target_room, target_zone) {
            (Some(room), Some(zone)) => {
                if !area_level_count.contains_key(&room) {
                    report.push_error(format!(
                        "LoadingZone '{}:{}' targets unknown room '{}'",
                        area, zone_id, room
                    ));
                    continue;
                }
                if !zones_by_area
                    .get(&room)
                    .map(|zones| zones.contains(&zone))
                    .unwrap_or(false)
                {
                    report.push_error(format!(
                        "LoadingZone '{}:{}' targets missing zone '{}:{}'",
                        area, zone_id, room, zone
                    ));
                }
            }
            _ => report.push_error(format!(
                "level '{}' LoadingZone '{}:{}' must author both target_room and target_zone",
                level_id, area, zone_id
            )),
        }
    }
}

fn validate_room_music_tracks(
    project: &LdtkProject,
    music: &MusicRegistry,
    report: &mut ContentValidationReport,
) {
    let valid_tracks = music.tracks.iter().map(|track| track.id.as_str());
    report.extend_errors(
        project
            .music_track_warnings(valid_tracks)
            .into_iter()
            .map(|warning| format!("room music reference: {warning}")),
    );
}

fn validate_npc_dialogue_ids(
    project: &LdtkProject,
    character_catalog: &ambition_characters::actor::character_catalog::CharacterCatalog,
    report: &mut ContentValidationReport,
) {
    let known_ids = crate::dialogue::known_dialogue_ids(character_catalog);
    let known = known_ids
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    for level in &project.levels {
        for entity in level.all_entity_instances() {
            if entity.identifier != "NpcSpawn" {
                continue;
            }
            let Some(dialogue_id) = field_string(entity, "dialogue_id") else {
                continue;
            };
            let dialogue_id = dialogue_id.trim();
            if dialogue_id.is_empty() {
                continue;
            }
            if !known.contains(dialogue_id) {
                report.push_error(format!(
                    "level '{}' NpcSpawn '{}' references unknown dialogue_id '{}'",
                    level.identifier, entity.iid, dialogue_id
                ));
            }
        }
    }
}

/// Validate every authored `NpcSpawn.brain_override` against the *assembled*
/// character catalog — the real content check the resolver's namespace rule was
/// built for, run before any actor spawns.
///
/// For each `NpcSpawn` that names a `character_id`, the pair
/// `(character_id, brain_override)` must resolve through
/// [`CharacterCatalog::validate_brain_override`](ambition_characters::actor::character_catalog::CharacterCatalog::validate_brain_override):
/// the character must exist, and a non-empty override must qualify inside the
/// character's own provider namespace (a fully-qualified preset is used exactly;
/// a raw preset resolves character-provider-local — never a silent cross-provider
/// fallback). An empty/absent override is the character default and always
/// passes.
///
/// An `NpcSpawn` with a `brain_override` but NO `character_id` is a content error
/// on its own: there is no character whose namespace could qualify the override.
/// (An anonymous NPC with neither field is a valid brainless placement and is
/// skipped.)
///
/// This is the production host contract: an unresolved override is rejected here
/// so `resolve_npc_brain` never has to tolerate an unknown preset at spawn time.
///
/// An UNKNOWN `character_id` is tolerated (skipped), because a partial composition
/// — a single-provider host, or this embedded Ambition-only check — legitimately
/// loads a catalog that does not own every character the shared Hall places
/// (`sanic`, `mary_o`, …). The FULL multi-provider host passes the merged catalog,
/// so nothing is skipped there and every Hall character is validated (proven by
/// `app_local_catalog_composition`'s full-host test). A KNOWN character's override
/// is validated in every composition — that is the part `resolve_npc_brain` must
/// never have to tolerate at runtime.
fn validate_npc_brain_overrides(
    project: &LdtkProject,
    character_catalog: &ambition_characters::actor::character_catalog::CharacterCatalog,
    report: &mut ContentValidationReport,
) {
    use ambition_characters::actor::character_catalog::BrainBuildError;
    for level in &project.levels {
        for entity in level.all_entity_instances() {
            if entity.identifier != "NpcSpawn" {
                continue;
            }
            let character_id = field_string(entity, "character_id")
                .map(|id| id.trim().to_string())
                .filter(|id| !id.is_empty());
            let brain_override = field_string(entity, "brain_override")
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty());

            match (character_id, brain_override) {
                // Anonymous placement, no brain authority — nothing to check.
                (None, None) => {}
                // An override with no character to qualify it against.
                (None, Some(brain_override)) => report.push_error(format!(
                    "level '{}' NpcSpawn '{}' has brain_override '{}' but no character_id \
                     (a brain preset can only be qualified inside a character's provider namespace)",
                    level.identifier, entity.iid, brain_override
                )),
                // A catalog-backed NPC: character must exist and the override (if
                // any) must resolve. `validate_brain_override` handles both.
                (Some(character_id), brain_override) => {
                    match character_catalog
                        .validate_brain_override(&character_id, brain_override.as_deref())
                    {
                        Ok(_) => {}
                        // A character owned by a provider not loaded in this
                        // composition — skipped (the full host validates it).
                        Err(BrainBuildError::UnknownCharacter(_)) => {}
                        Err(error) => report.push_error(format!(
                            "level '{}' NpcSpawn '{}': {}",
                            level.identifier, entity.iid, error
                        )),
                    }
                }
            }
        }
    }
}

fn validate_quest_conditions(
    project: &LdtkProject,
    music: &MusicRegistry,
    report: &mut ContentValidationReport,
) {
    let room_ids = active_area_ids(project);
    let encounter_ids = authored_encounter_ids(project);
    let boss_ids = authored_boss_encounter_ids(project);
    let item_ids = authored_pickup_ids(project);
    let known_flags = authored_flag_ids(project);
    let valid_tracks = music
        .tracks
        .iter()
        .map(|track| track.id.as_str())
        .collect::<BTreeSet<_>>();

    // ⚠ **the same book the plugin installs**, read from the prepared pack rather
    // than from a process-global the validator happens to share with whatever
    // App ran first. That sharing is exactly what made the old seam look
    // provider-local while not being.
    let waves =
        ambition_encounter::content_schema::lowered_encounter_waves(crate::pack::prepared())
            .cloned()
            .map(ambition_encounter::EncounterWaveBook);
    let loaded_encounters =
        ambition_platformer2d_actor_monolith::encounter::load_encounter_specs_from_ldtk(
            project,
            &ambition_persistence::save_data::AmbitionGameSaveData::default(),
            waves.as_ref(),
        );
    for (id, spec, _) in loaded_encounters {
        // Exactly-empty, matching `encounter/systems.rs`'s own
        // `!spec.music_track.is_empty()` gate — same reason as the boss phases.
        if !spec.music_track.is_empty() && !valid_tracks.contains(spec.music_track.as_str()) {
            report.push_error(format!(
                "encounter '{}' references unknown music track '{}'",
                id, spec.music_track
            ));
        }
    }

    for spec in crate::quest::default_quest_specs() {
        if spec.steps.is_empty() {
            report.push_error(format!("quest '{}' has no steps", spec.id));
        }
        for (index, step) in spec.steps.iter().enumerate() {
            match &step.condition {
                ambition_persistence::quest::QuestStepCondition::RoomEntered(room) => {
                    if !room_ids.contains(room.as_str()) {
                        report.push_error(format!(
                            "quest '{}'/step {} references unknown room '{}'",
                            spec.id, index, room
                        ));
                    }
                }
                ambition_persistence::quest::QuestStepCondition::EncounterCleared(encounter) => {
                    if !encounter_ids.contains(encounter.as_str()) {
                        report.push_error(format!(
                            "quest '{}'/step {} references unknown encounter '{}'",
                            spec.id, index, encounter
                        ));
                    }
                }
                ambition_persistence::quest::QuestStepCondition::BossDefeated(boss) => {
                    if !boss_ids.contains(boss.as_str()) {
                        report.push_error(format!(
                            "quest '{}'/step {} references unknown authored boss encounter '{}'",
                            spec.id, index, boss
                        ));
                    }
                }
                ambition_persistence::quest::QuestStepCondition::FlagSet(flag) => {
                    if !known_flags.contains(flag.as_str()) {
                        report.push_error(format!(
                            "quest '{}'/step {} references unknown authored flag '{}'",
                            spec.id, index, flag
                        ));
                    }
                }
                ambition_persistence::quest::QuestStepCondition::ItemCollected(item) => {
                    if !item_ids.contains(item.as_str()) {
                        report.push_error(format!(
                            "quest '{}'/step {} references unknown pickup/item id '{}'",
                            spec.id, index, item
                        ));
                    }
                }
                ambition_persistence::quest::QuestStepCondition::NpcTalked(npc) => {
                    // Gameplay emits the runtime NPC object id for NpcTalked. Most current
                    // quests use flags instead, but keep the validator honest for future ones.
                    if !authored_npc_ids(project).contains(npc.as_str()) {
                        report.push_error(format!(
                            "quest '{}'/step {} references unknown NPC id '{}'",
                            spec.id, index, npc
                        ));
                    }
                }
            }
        }
    }
}

/// ⚠ **The content compiler now checks this too, for pack-supplied content.**
/// `boss_encounter` emits a `music_track` reference per non-empty phase field,
/// so an unknown track in Ambition's own encounters is refused at reference
/// resolution — before startup, with the field named.
///
/// This is NOT dead: it reads the ASSEMBLED catalog, so it still covers a
/// provider that contributes bosses WITHOUT shipping a content pack. It can be
/// deleted when every boss-contributing provider goes through the compiler —
/// and not before, because deleting startup validation to make a migration look
/// finished is exactly the overclaiming this campaign keeps having to walk back.
///
/// Two checks that AGREE are redundant; two that disagree are the defect. These
/// two apply the same rule to the same four fields, so the overlap is safe.
fn validate_boss_music_tracks(
    music: &MusicRegistry,
    boss_catalog: &ambition_platformer2d_actor_monolith::boss_encounter::BossCatalog,
    report: &mut ContentValidationReport,
) {
    let tracks = music
        .tracks
        .iter()
        .map(|track| track.id.as_str())
        .collect::<BTreeSet<_>>();
    for spec in
        ambition_platformer2d_actor_monolith::boss_encounter::default_boss_specs(boss_catalog)
    {
        for (field, track) in [
            ("music_intro", spec.music_intro.as_str()),
            ("music_phase1", spec.music_phase1.as_str()),
            ("music_phase2", spec.music_phase2.as_str()),
            ("music_enrage", spec.music_enrage.as_str()),
        ] {
            // Exactly-empty, matching `phase_music`'s own gate: a
            // whitespace-only field is a request the runtime makes and must not
            // be waved through here.
            if !track.is_empty() && !tracks.contains(track) {
                report.push_error(format!(
                    "boss spec '{}' {field} references unknown music track '{}'",
                    spec.id, track
                ));
            }
        }
    }
}

#[cfg(feature = "audio")]
fn validate_adaptive_music_catalog(report: &mut ContentValidationReport) {
    let catalog = crate::music::ambition_music_cue_catalog();
    report.extend_errors(
        catalog
            .validate_references()
            .into_iter()
            .map(|error| format!("adaptive music catalog: {error}")),
    );
}

fn active_area_ids(project: &LdtkProject) -> BTreeSet<String> {
    project
        .levels
        .iter()
        .map(|level| level.active_area())
        .collect()
}

fn authored_encounter_ids(project: &LdtkProject) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    for level in &project.levels {
        let area = level.active_area();
        for entity in level.all_entity_instances() {
            if entity.identifier == "EncounterTrigger" {
                ids.insert(
                    field_string(entity, "id")
                        .map(|id| id.trim().to_string())
                        .filter(|id| !id.is_empty())
                        .unwrap_or_else(|| area.clone()),
                );
            }
        }
    }
    ids
}

fn authored_boss_encounter_ids(project: &LdtkProject) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    for level in &project.levels {
        for entity in level.all_entity_instances() {
            if entity.identifier == "BossSpawn" {
                let name = field_string(entity, "name")
                    .map(|name| name.trim().to_string())
                    .filter(|name| !name.is_empty())
                    .unwrap_or_else(|| entity.iid.clone());
                ids.insert(
                    ambition_platformer2d_actor_monolith::boss_encounter::encounter_id_from_name(
                        &name,
                    ),
                );
            }
        }
    }
    ids
}

fn authored_npc_ids(project: &LdtkProject) -> BTreeSet<String> {
    authored_entity_iids(project, "NpcSpawn")
}

fn authored_pickup_ids(project: &LdtkProject) -> BTreeSet<String> {
    authored_entity_iids(project, "PickupSpawn")
}

fn authored_entity_iids(project: &LdtkProject, identifier: &str) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    for level in &project.levels {
        for entity in level.all_entity_instances() {
            if entity.identifier == identifier {
                ids.insert(entity.iid.clone());
            }
        }
    }
    ids
}

fn authored_flag_ids(project: &LdtkProject) -> BTreeSet<String> {
    let mut flags = BTreeSet::from([
        "met_any_hub_npc".to_string(),
        "test_switch_toggled".to_string(),
        crate::quest::PIRATE_TREASURE_REWARD_FLAG.to_string(),
    ]);
    for level in &project.levels {
        for entity in level.all_entity_instances() {
            if entity.identifier == "NpcSpawn" {
                if let Some(dialogue_id) = field_string(entity, "dialogue_id") {
                    let dialogue_id = dialogue_id.trim();
                    if !dialogue_id.is_empty() {
                        flags.insert(format!("npc_{dialogue_id}_talked"));
                    }
                }
            }
            if entity.identifier == "EncounterTrigger" {
                let encounter_id = field_string(entity, "id")
                    .map(|id| id.trim().to_string())
                    .filter(|id| !id.is_empty())
                    .unwrap_or_else(|| level.active_area());
                flags.insert(format!("encounter_{encounter_id}_reward_dropped"));
                flags.insert(encounter_reward_looted_flag(&encounter_id));
            }
            if entity.identifier == "Switch" {
                if let Some(id) = field_string(entity, "id") {
                    let id = id.trim();
                    if !id.is_empty() {
                        flags.insert(format!("switch_{id}_used"));
                    }
                }
            }
            // PickupSpawn entities with `kind: "flag:<id>"` set the
            // named flag in save state when collected. Mirror the
            // runtime parse rule in `world/ldtk_world/fields.rs::parse_pickup_kind`
            // so quest steps that depend on a story-flag pickup
            // validate without needing the flag listed elsewhere.
            if entity.identifier == "PickupSpawn" {
                if let Some(kind) = field_string(entity, "kind") {
                    if let Some(flag) = kind.trim().strip_prefix("flag:") {
                        if !flag.is_empty() {
                            flags.insert(flag.to_string());
                        }
                    }
                }
            }
        }
    }
    for boss in authored_boss_encounter_ids(project) {
        flags.insert(format!("encounter_{boss}_reward_dropped"));
        flags.insert(encounter_reward_looted_flag(&boss));
    }
    flags
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_content_graph_validates() {
        let report = validate_embedded_content_graph();
        report.panic_if_errors();
    }

    /// **⛔ THE PATROL CHECK MUST HAVE SOMETHING TO CHECK** — proved against a
    /// fixture it owns, not against the shipped game.
    ///
    /// `validate_patrol_brain_paths` is an ERROR now, and an error no entity can
    /// trigger is a check that cannot fail. This used to establish that by
    /// counting `Patrol:` brains in the embedded project — which made the
    /// validator's own credibility depend on Ambition continuing to author at
    /// least one patrolling enemy forever. That is a content-design choice, and
    /// nothing should stop a designer from making a different one.
    ///
    /// So the non-vacuity proof is a synthetic level carrying all three cases at
    /// once: one patrol that resolves, one that names a path nobody authored,
    /// and one bare `Patrol:`. `embedded_content_graph_validates` above
    /// separately proves the shipped game is clean.
    #[test]
    fn the_patrol_check_errors_on_a_broken_reference_and_stays_quiet_on_a_good_one() {
        use ambition_platformer2d_ldtk::{
            LdtkEntityInstance, LdtkFieldInstance, LdtkLayerInstance, LdtkLevel,
        };

        fn field(identifier: &str, value: &str) -> LdtkFieldInstance {
            LdtkFieldInstance {
                identifier: identifier.to_string(),
                value: serde_json::Value::String(value.to_string()),
                real_editor_values: Vec::new(),
            }
        }
        fn entity(
            iid: &str,
            identifier: &str,
            fields: Vec<LdtkFieldInstance>,
        ) -> LdtkEntityInstance {
            LdtkEntityInstance {
                iid: iid.to_string(),
                identifier: identifier.to_string(),
                pivot: Vec::new(),
                px: [0, 0],
                width: 16,
                height: 16,
                field_instances: fields,
            }
        }

        let project = LdtkProject {
            json_version: "1.5.3".to_string(),
            levels: vec![LdtkLevel {
                identifier: "patrol_fixture".to_string(),
                iid: "level-iid".to_string(),
                world_x: 0,
                world_y: 0,
                px_wid: 640,
                px_hei: 480,
                field_instances: Vec::new(),
                layer_instances: vec![LdtkLayerInstance {
                    identifier: "Ambition".to_string(),
                    layer_type: "Entities".to_string(),
                    c_wid: 40,
                    c_hei: 30,
                    grid_size: 16,
                    entity_instances: vec![
                        // The path everyone below is judged against. Named, not
                        // id'd, so the slug rule is exercised too.
                        entity(
                            "path",
                            "KinematicPath",
                            vec![field("name", "Lab Patrol Line")],
                        ),
                        entity(
                            "good",
                            "EnemySpawn",
                            vec![field("brain", "Patrol:lab_patrol_line")],
                        ),
                        entity(
                            "missing",
                            "EnemySpawn",
                            vec![field("brain", "Patrol:nobody_authored_this")],
                        ),
                        entity("bare", "EnemySpawn", vec![field("brain", "Patrol:")]),
                        // Not a patrol at all: must not be judged by this check.
                        entity("passive", "EnemySpawn", vec![field("brain", "Passive")]),
                    ],
                    int_grid_csv: Vec::new(),
                    grid_tiles: Vec::new(),
                }],
            }],
        };

        let mut report = ContentValidationReport::default();
        validate_patrol_brain_paths(&project, &mut report);

        assert_eq!(
            report.errors.len(),
            2,
            "exactly the broken reference and the bare prefix must error: {:?}",
            report.errors,
        );
        assert!(
            report
                .errors
                .iter()
                .any(|e| e.contains("nobody_authored_this")),
            "a patrol naming a path nobody authored must be an error: {:?}",
            report.errors,
        );
        assert!(
            report.errors.iter().any(|e| e.contains("bare brain")),
            "a bare 'Patrol:' must be an error: {:?}",
            report.errors,
        );
        // ⚠ by SUBJECT, not by path id: the missing-reference message lists the
        // level's resolved ids, so a naive `contains("lab_patrol_line")` matches
        // the error about a DIFFERENT entity.
        assert!(
            !report.errors.iter().any(|e| e.contains("'good'")),
            "the patrol that RESOLVES must not be reported — a check that fails \
             on everything is as useless as one that fails on nothing: {:?}",
            report.errors,
        );
        assert!(
            !report.errors.iter().any(|e| e.contains("'passive'")),
            "a non-patrol brain is not this check's business: {:?}",
            report.errors,
        );
    }

    /// **THE INVARIANT: this validator accepts exactly the spellings the
    /// runtime resolves — no more, and no fewer.**
    ///
    /// ⛔ it did neither, on the very path that shipped. Sandbox's basement
    /// authors `enemy patrol path A` with no `id`, so conversion used to derive
    /// the COMPACTED id `enemy_patrol_a` (its own slug rule collapsed `_path_`)
    /// while the placement references the raw slug `enemy_patrol_path_a`. This
    /// validator knew only the raw slug and the binding sweep knew both, so both
    /// oracles passed — and the runtime's own table knew only the compacted id,
    /// so the gallery's patroller stood still. Fewer than the runtime is a hard
    /// startup abort on shippable content; more is a dead demo nobody is told
    /// about.
    ///
    /// ⭐ **the compacting slug rule is now DELETED**, so this fixture's path is
    /// simply `enemy_patrol_path_a` — the spelling the placement always used.
    /// `enemy_patrol_a` therefore joins the typo as a spelling nothing accepts,
    /// and that is the assertion guarding the deletion: if it resolves again, a
    /// second id-minting authority is back.
    #[test]
    fn a_patrol_reference_is_judged_by_the_runtime_alias_set() {
        use ambition_platformer2d_ldtk::{
            LdtkEntityInstance, LdtkFieldInstance, LdtkLayerInstance, LdtkLevel,
        };

        fn field(identifier: &str, value: &str) -> LdtkFieldInstance {
            LdtkFieldInstance {
                identifier: identifier.to_string(),
                value: serde_json::Value::String(value.to_string()),
                real_editor_values: Vec::new(),
            }
        }
        fn entity(
            iid: &str,
            identifier: &str,
            fields: Vec<LdtkFieldInstance>,
        ) -> LdtkEntityInstance {
            LdtkEntityInstance {
                iid: iid.to_string(),
                identifier: identifier.to_string(),
                pivot: Vec::new(),
                px: [0, 0],
                width: 16,
                height: 16,
                field_instances: fields,
            }
        }

        let project = LdtkProject {
            json_version: "1.5.3".to_string(),
            levels: vec![LdtkLevel {
                identifier: "alias_fixture".to_string(),
                iid: "level-iid".to_string(),
                world_x: 0,
                world_y: 0,
                px_wid: 640,
                px_hei: 480,
                field_instances: Vec::new(),
                layer_instances: vec![LdtkLayerInstance {
                    identifier: "Ambition".to_string(),
                    layer_type: "Entities".to_string(),
                    c_wid: 40,
                    c_hei: 30,
                    grid_size: 16,
                    entity_instances: vec![
                        // The shipped shape: named, never id'd — so its id is
                        // derived from the name, by the ONE remaining slug rule.
                        entity(
                            "path",
                            "KinematicPath",
                            vec![field("name", "enemy patrol path A")],
                        ),
                        entity(
                            "by_raw_slug",
                            "EnemySpawn",
                            vec![field("brain", "Patrol:enemy_patrol_path_a")],
                        ),
                        entity(
                            "by_compacted_id",
                            "EnemySpawn",
                            vec![field("brain", "Patrol:enemy_patrol_a")],
                        ),
                        entity(
                            "by_display_name",
                            "EnemySpawn",
                            vec![field("brain", "Patrol:enemy patrol path A")],
                        ),
                        entity(
                            "by_typo",
                            "EnemySpawn",
                            vec![field("brain", "Patrol:enemy_patrol_b")],
                        ),
                    ],
                    int_grid_csv: Vec::new(),
                    grid_tiles: Vec::new(),
                }],
            }],
        };

        let mut report = ContentValidationReport::default();
        validate_patrol_brain_paths(&project, &mut report);

        assert!(
            !report.errors.iter().any(|e| e.contains("'by_raw_slug'")),
            "the raw name slug resolves at runtime and must not abort startup: {:?}",
            report.errors,
        );
        assert!(
            !report
                .errors
                .iter()
                .any(|e| e.contains("'by_display_name'")),
            "the path's display name is a spelling the runtime resolves: {:?}",
            report.errors,
        );
        // ...and the poison: a spelling NOTHING accepts is still an error, so
        // this is a shared alias set rather than a check that stopped checking.
        assert!(
            report.errors.iter().any(|e| e.contains("'by_typo'")),
            "a patrol naming no path at all must still error: {:?}",
            report.errors,
        );
        // ⛔ THE DELETION'S GUARD: `enemy_patrol_a` is what the converter's own
        // slug rule used to mint for this name. Nothing references it, nothing
        // should resolve it, and if it starts resolving again then a second
        // id-minting authority has come back and the alias sets will drift
        // again — which is the failure that left a patroller standing still.
        assert!(
            report
                .errors
                .iter()
                .any(|e| e.contains("'by_compacted_id'")),
            "the DELETED slug rule's spelling must resolve for nobody: {:?}",
            report.errors,
        );
    }

    #[test]
    fn validates_ldtk_loading_zone_targets() {
        let music = crate::audio_registries::load_music_registry();
        let project = LdtkProject::load_default_for_dev(&crate::worlds::world_manifest())
            .expect("embedded LDtk loads");
        let character_catalog = crate::character_catalog::load_catalog();
        let report = validate_content_graph(&music, &project, &character_catalog);
        assert!(
            report
                .errors
                .iter()
                .all(|error| !error.contains("LoadingZone")),
            "loading zone validation failed: {:?}",
            report.errors
        );
    }

    #[test]
    fn quest_boss_conditions_point_at_authored_bosses() {
        let project = LdtkProject::load_default_for_dev(&crate::worlds::world_manifest())
            .expect("embedded LDtk loads");
        let boss_ids = authored_boss_encounter_ids(&project);
        assert!(boss_ids.contains("clockwork_warden"));
        for spec in crate::quest::default_quest_specs() {
            for step in &spec.steps {
                if let ambition_persistence::quest::QuestStepCondition::BossDefeated(id) =
                    &step.condition
                {
                    assert!(
                        boss_ids.contains(id.as_str()),
                        "quest '{}' references boss '{}' not authored in LDtk; authored bosses: {:?}",
                        spec.id,
                        id,
                        boss_ids
                    );
                }
            }
        }
    }
}
