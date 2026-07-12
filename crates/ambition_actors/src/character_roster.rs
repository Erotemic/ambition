//! The character-roster SEAM: the game installs its
//! `character_catalog.ron` text; this module owns the parse cache + the
//! lookup helpers the non-Bevy call sites use.
//!
//! `ambition_characters::character_catalog` owns the catalog SCHEMA +
//! parser + preset resolver (machinery, content-free);
//! `ambition_content::character_catalog` owns Ambition's actual roster
//! DATA (the RON ships in the content crate, readable by the Python tools).
//! The engine ships no characters (R3.2, the violation-#3 eviction).
//!
//! §5 classification: **content registry** — install-once seam
//! ([`install_character_catalog`], first install wins), immutable after,
//! read from pure non-system code (the LDtk `NpcSpawn` converter, spawn
//! paths, sprite lookups) with no `World` in hand. The Bevy
//! `CharacterCatalog` resource ([`character_roster_plugin`]) always takes
//! precedence when one is available, but the LDtk parser runs without
//! `Res<>` access.

use std::sync::OnceLock;

use ambition_characters::actor::character_catalog::{
    parse_catalog, CharacterCatalogData, CharacterCatalogPlugin,
};

/// Game-installed catalog RON text. First install wins; later calls are
/// ignored (the `install_enemy_roster` seam contract).
static CATALOG_RON_OVERRIDE: OnceLock<&'static str> = OnceLock::new();
static CATALOG: OnceLock<CharacterCatalogData> = OnceLock::new();

/// Install the game's character-catalog RON — the content layer calls this
/// at every sim entry choke point (before any catalog read). First install
/// wins.
pub fn install_character_catalog(catalog_ron: &'static str) {
    let _ = CATALOG_RON_OVERRIDE.set(catalog_ron);
}

/// The content-installed default playable character id (C2). The engine names
/// no specific character: the game owns which catalog row the home box wears
/// with no override (its `PLAYABLE_ROSTER[0]`), and installs it here.
static DEFAULT_CHARACTER_ID: OnceLock<&'static str> = OnceLock::new();

/// Install the default playable character id — the content layer calls this at
/// the same choke point as [`install_character_catalog`]. First install wins.
pub fn install_default_character_id(id: &'static str) {
    let _ = DEFAULT_CHARACTER_ID.set(id);
}

/// The default playable character id the home box wears with no override.
/// Resolved lazily (at spawn, after content install), so the engine never
/// bakes in a content name. When the game hasn't installed one, falls back to
/// the FIRST catalog row — still content-derived, never an engine literal.
pub fn default_character_id() -> &'static str {
    DEFAULT_CHARACTER_ID.get().copied().unwrap_or_else(|| {
        catalog()
            .characters
            .keys()
            .next()
            .map(String::as_str)
            .unwrap_or("")
    })
}

/// The installed catalog RON text (feeds the Bevy plugin + the parse cache).
pub fn catalog_ron() -> &'static str {
    CATALOG_RON_OVERRIDE.get().copied().unwrap_or_else(|| {
        #[cfg(test)]
        {
            // Test fixture = the game's REAL catalog, read cross-crate from
            // ambition_content (the install_enemy_roster fixture pattern).
            include_str!("../../../game/ambition_content/assets/data/character_catalog.ron")
        }
        #[cfg(not(test))]
        {
            panic!(
                "character catalog not installed — the game's content must call \
                 install_character_catalog() before any roster lookup \
                 (AmbitionContentPlugin / the app's sim-entry choke points do)"
            )
        }
    })
}

/// One-time parse cache over the installed catalog so non-Bevy call sites
/// (the LDtk parser, tests, headless tooling) query the roster without
/// re-parsing.
pub fn catalog() -> &'static CharacterCatalogData {
    CATALOG.get_or_init(|| parse_catalog(catalog_ron()))
}

/// Look up the display name for a character id. Returns `None` if
/// the id is not in the roster; callers fall back to the id itself.
pub fn display_name_for_character_id(character_id: &str) -> Option<&'static str> {
    catalog()
        .characters
        .get(character_id)
        .map(|entry| entry.display_name.as_str())
}

/// Reverse of [`display_name_for_character_id`]: resolve a display name back to
/// its catalog `character_id`. This is the gameplay-side mirror of the
/// presentation layer's name → sheet join (`npc_asset_for_name`), and is how a
/// spawned actor earns a uniform sprite identity from the only thing every
/// actor reliably carries — its display name. Returns `None` for a name with no
/// catalog row (a generic enemy that renders from a kind-default sheet).
pub fn character_id_for_display_name(display_name: &str) -> Option<&'static str> {
    catalog()
        .characters
        .iter()
        .find(|(_, entry)| entry.display_name == display_name)
        .map(|(id, _)| id.as_str())
}

/// Resolve a catalog `character_id` into its authored default [`Brain`], using
/// `spawn_world_x` as the patrol/anchor center. This is the data-driven join
/// that lets a placed NPC's behavior come from its catalog row (e.g. the lively
/// `Aerial` flyer) instead of a hardcoded Patrol/StandStill. Returns `None` for
/// an unknown id or a missing preset.
pub fn default_brain_for_character_id(
    character_id: &str,
    spawn_world_x: f32,
) -> Option<ambition_characters::brain::Brain> {
    let entry = catalog().characters.get(character_id)?;
    let preset = catalog().brain_presets.get(&entry.default_brain)?;
    Some(ambition_characters::actor::character_catalog::brain_from_preset(preset, spawn_world_x))
}

/// Resolve a catalog `character_id` into its authored default [`ActionSet`]
/// (its combat-capability bundle: melee / ranged / special / locomotion). The
/// presentation-agnostic mirror of [`default_brain_for_character_id`] — a body
/// earns its moveset from the same catalog row that names its sprite and brain.
/// Returns `None` for an unknown id or a missing preset.
pub fn default_action_set_for_character_id(
    character_id: &str,
) -> Option<ambition_characters::brain::ActionSet> {
    let entry = catalog().characters.get(character_id)?;
    let preset = catalog()
        .action_set_presets
        .get(&entry.default_action_set)?;
    Some(ambition_characters::actor::character_catalog::action_set_from_preset(preset))
}

/// Pick a bark line for a character id + situation, rotated by `rotation`
/// (so repeated barks cycle the pool). Reads the character's catalog `barks`
/// pools — the single source of truth for its voice. Returns `None` when the
/// id is unknown or its pool for that situation is empty, so callers can fall
/// back to the legacy bark tables during the catalog-population transition.
pub fn bark_line_for_character_id(
    character_id: &str,
    situation: ambition_characters::actor::character_catalog::BarkSituation,
    rotation: u32,
) -> Option<&'static str> {
    catalog()
        .characters
        .get(character_id)?
        .barks
        .pick(situation, rotation)
}

/// The Hall-of-Characters dialogue node id authored for a character id, if
/// any. The hall generator reads this to populate each pedestal's
/// `dialogue_id`; the dialogue validator folds it into the known-id set.
pub fn hall_dialogue_id_for_character_id(character_id: &str) -> Option<&'static str> {
    catalog()
        .characters
        .get(character_id)?
        .hall_dialogue_id
        .as_deref()
}

/// The declared source of a catalog row's PLAYABLE kit.
///
/// This deliberately returns `None` for an unknown id instead of folding it into
/// either enum variant. The wear seam must distinguish three cases:
///
/// - known [`PlayableKitSource::Authored`] row → the referenced preset must win;
/// - known [`PlayableKitSource::HostCode`] row → rebuild from host abilities;
/// - unknown id → the explicit unknown-character fallback.
///
/// Keeping "unknown" separate also prevents a malformed authored row (known row,
/// missing preset) from silently receiving the host protagonist's privileged kit.
pub fn playable_kit_source_for_character_id(
    character_id: &str,
) -> Option<ambition_characters::actor::character_catalog::PlayableKitSource> {
    catalog()
        .characters
        .get(character_id)
        .map(|entry| entry.playable_kit)
}

/// Compatibility helper for call sites that only need the yes/no question.
/// Prefer [`playable_kit_source_for_character_id`] at resolution boundaries so
/// unknown ids remain distinguishable from authored rows.
pub fn playable_kit_is_host_code(character_id: &str) -> bool {
    playable_kit_source_for_character_id(character_id)
        == Some(ambition_characters::actor::character_catalog::PlayableKitSource::HostCode)
}

/// The authored surface-momentum params for a character id, hydrated into the
/// serde-free kernel struct (Q21 / S2). `Some` means the character's body opts
/// into `MotionModel::SurfaceMomentum` — the surface-follower solver. Returns
/// `None` for an unknown id or a row that authors no `momentum` field (the
/// axis-swept default). This is the single lookup both the player-wear seam
/// ([`crate::avatar::apply_worn_motion_model`]) and the actor spawn path read.
pub fn momentum_params_for_character_id(
    character_id: &str,
) -> Option<ambition_engine_core::surface::MomentumParams> {
    catalog()
        .characters
        .get(character_id)?
        .momentum
        .as_ref()
        .map(|spec| spec.to_kernel())
}

/// The catalog `body_kind` for a character id, if present. `Floating` means the
/// actor is gravity-free (a flyer): the spawn zeroes its `gravity_scale` so the
/// brain's full 2D `desired_vel` drives flight.
pub fn body_kind_for_character_id(
    character_id: &str,
) -> Option<ambition_characters::actor::character_catalog::CharacterBodyKind> {
    catalog()
        .characters
        .get(character_id)
        .map(|entry| entry.body_kind)
}

/// The catalog plugin pre-loaded with the installed roster. The install
/// must precede this constructor (the app's sim-resources plugin installs
/// content immediately before adding it).
pub fn character_roster_plugin() -> CharacterCatalogPlugin {
    CharacterCatalogPlugin {
        catalog_ron: catalog_ron(),
    }
}

#[cfg(test)]
mod tests;
