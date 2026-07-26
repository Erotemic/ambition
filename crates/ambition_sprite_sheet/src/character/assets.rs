//! Loaded character spritesheet handles shared by loaders and renderers.
//!
//! **No character id appears in this file.** It used to hold four typed slots —
//! `player`, `robot`, `goblin`, `sandbag` — and `asset_for_character_id` was a
//! hardcoded `match` on those names falling through to a map. That shape made the
//! engine know four of Ambition's characters by name, so a second provider's
//! protagonist could not be a hot-path character however it was authored, and it
//! made "this id has no sheet" and "there is no such id" the same answer (`None`).
//! One map, keyed by what content declares, answers both questions honestly.

use std::collections::HashMap;

use bevy::prelude::*;

use super::CharacterSpriteAsset;

/// What the sheet table knows about one authored character token.
///
/// The distinction between the last two variants is the point. `None` used to
/// mean both "declared, sheet not decoded yet" and "no such character", so a
/// TYPO and a pending decode were indistinguishable — a misspelled id drew the
/// placeholder rectangle forever and looked exactly like a sheet that had not
/// arrived. A caller that wants to wait can now tell it apart from one that
/// should report a binding failure.
#[derive(Clone, Copy)]
pub enum CharacterSheetState<'a> {
    /// Decoded and ready to draw.
    Ready(&'a CharacterSpriteAsset),
    /// Content declares this character and named a sheet, but nothing has
    /// materialized it yet. Transient, and resolvable by demanding it.
    Declared { character_id: &'a str },
    /// No content declares this token under any key. A typo, or a character
    /// from a provider that is not loaded. Never resolves by waiting.
    Unknown,
}

impl CharacterSheetState<'_> {
    /// True only for a decoded sheet.
    pub fn is_ready(&self) -> bool {
        matches!(self, Self::Ready(_))
    }

    /// True only for a token no content declares. The interesting predicate: it
    /// is the one that means "report a binding failure" rather than "wait".
    pub fn is_unknown(&self) -> bool {
        matches!(self, Self::Unknown)
    }

    /// The catalog id behind a declared token, if it is declared.
    pub fn declared_character_id(&self) -> Option<&str> {
        match self {
            Self::Declared { character_id } => Some(character_id),
            _ => None,
        }
    }
}

// Hand-written so the enum does not force `Debug` onto the whole sheet-spec tree
// it points at. Diagnostics want the STATE named, not the atlas dumped.
impl std::fmt::Debug for CharacterSheetState<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ready(_) => write!(f, "Ready"),
            Self::Declared { character_id } => write!(f, "Declared({character_id})"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Holds decoded spritesheet handles plus the declared-but-not-yet-decoded set.
///
/// Both maps are **double-keyed** by catalog id AND authored display name, so
/// presentation can resolve either a stable id or a legacy display label through
/// one lookup without depending on the actor roster module.
#[derive(Resource, Default, Clone)]
pub struct CharacterSpriteAssets {
    /// Materialized sheets. Double-keyed (see above).
    sheets: HashMap<String, CharacterSpriteAsset>,
    /// Per-prop sprite sheets keyed by the LDtk `Prop.kind` field.
    pub props: HashMap<String, CharacterSpriteAsset>,
    /// Declared characters whose sheets are NOT decoded yet: token → catalog id.
    /// Startup declares the whole roster here and decodes none of it; the engine
    /// materializer promotes the ids a session actually demands. That is what
    /// keeps a ~130-sheet decode storm off the boot path without making any one
    /// character privileged.
    declared: HashMap<String, String>,
}

impl CharacterSpriteAssets {
    /// Declare a character's sheet without decoding it, under every token that
    /// should resolve to it (its catalog id and its display name).
    pub fn declare(&mut self, character_id: &str, display_name: &str) {
        self.declared
            .insert(character_id.to_string(), character_id.to_string());
        self.declared
            .insert(display_name.to_string(), character_id.to_string());
    }

    /// Publish a decoded sheet under every token currently declared for
    /// `character_id`, plus the id itself, and clear those declarations.
    pub fn publish(&mut self, character_id: &str, asset: CharacterSpriteAsset) {
        let tokens: Vec<String> = self
            .declared
            .iter()
            .filter(|(_, declared_id)| declared_id.as_str() == character_id)
            .map(|(token, _)| token.clone())
            .collect();
        for token in tokens {
            self.declared.remove(&token);
            self.sheets.insert(token, asset.clone());
        }
        // A character published without ever being declared (a test fixture, or
        // a host inserting a sheet directly) still resolves by its own id.
        self.sheets.insert(character_id.to_string(), asset);
    }

    /// Publish a decoded sheet under ONE explicit token.
    ///
    /// For content that builds its own sheet outside the catalog-declared path
    /// (an intro NPC, a demo's bespoke enemy) and knows exactly which tokens
    /// should resolve to it. Prefer [`Self::publish`] when the character was
    /// declared, so every token it was declared under is covered automatically
    /// rather than by the caller remembering to list them.
    pub fn publish_under(&mut self, token: &str, asset: CharacterSpriteAsset) {
        self.declared.remove(token);
        self.sheets.insert(token.to_string(), asset);
    }

    /// Every declared-but-undecoded catalog id, deduplicated.
    pub fn declared_character_ids(&self) -> std::collections::BTreeSet<&str> {
        self.declared.values().map(String::as_str).collect()
    }

    /// True when `character_id` is declared and still awaiting a decode.
    pub fn is_declared(&self, character_id: &str) -> bool {
        self.declared.contains_key(character_id)
    }

    /// THE lookup. `token` is either a stable catalog id or an authored display
    /// name; the table is double-keyed so both reach the same sheet.
    ///
    /// This replaced three methods (`asset_for_character_id`,
    /// `npc_asset_for_name`, `asset_for_authored_character`) that had become the
    /// same map lookup wearing different names — the sort of drift where two of
    /// them quietly stop agreeing.
    pub fn sheet(&self, token: &str) -> Option<&CharacterSpriteAsset> {
        self.sheets.get(token)
    }

    /// The full answer, distinguishing a pending decode from an unknown id.
    pub fn sheet_state(&self, token: &str) -> CharacterSheetState<'_> {
        if let Some(asset) = self.sheets.get(token) {
            return CharacterSheetState::Ready(asset);
        }
        match self.declared.get(token) {
            Some(character_id) => CharacterSheetState::Declared { character_id },
            None => CharacterSheetState::Unknown,
        }
    }

    /// Pick a prop spritesheet by its registry key.
    pub fn prop_asset_for_kind(&self, kind: &str) -> Option<&CharacterSpriteAsset> {
        self.props.get(kind)
    }

    /// Number of decoded sheet tokens. Diagnostics and censuses only.
    pub fn ready_token_count(&self) -> usize {
        self.sheets.len()
    }
}
