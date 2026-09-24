//! Loaded character spritesheet handles shared by loaders and renderers.
//!
//! One map, keyed by the tokens that content declares, holds every character
//! sheet. The engine knows no character by name, and "this id has no sheet" is
//! different from "no such id" (see [`CharacterSheetState`]).

use std::collections::HashMap;

use bevy::prelude::*;

use ambition_persistence::settings::TextureResolutionScale;

use super::CharacterSpriteAsset;

/// What the sheet table knows about one authored character token.
///
/// `Declared` is the nonresident state. A token is `Declared` before its first
/// decode, and an unworn realization returns to `Declared` when the quality tier
/// changes (see [`CharacterSpriteAssets::retire_realizations`]; a worn one is
/// replaced in place). So the declaration must outlive the decode.
#[derive(Clone, Copy)]
pub enum CharacterSheetState<'a> {
    /// Decoded and ready to draw.
    Ready(&'a CharacterSpriteAsset),
    /// Content declares this character and named a sheet, but nothing has
    /// materialized it yet — either it never has, or its realization was
    /// retired by a quality change. Resolvable by demanding it.
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

    /// True only for a token no content declares. This means "report a binding
    /// failure", not "wait".
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

// Diagnostics name the state; they do not dump the atlas.
impl std::fmt::Debug for CharacterSheetState<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ready(_) => write!(f, "Ready"),
            Self::Declared { character_id } => write!(f, "Declared({character_id})"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Holds resident spritesheet realizations plus the declarations behind them.
///
/// Both maps are double-keyed by catalog id AND authored display name, so
/// presentation can resolve either a stable id or a legacy display label through
/// one lookup without depending on the actor roster module.
///
/// ## Declarations and realizations have different lifetimes
///
/// A declaration ("this token names character X") is permanent knowledge about
/// content. A realization ("decoded handles for X at tier T") is a resource
/// with an owner and an end.
#[derive(Resource, Default, Clone)]
pub struct CharacterSpriteAssets {
    /// Resident realizations. Double-keyed (see above).
    sheets: HashMap<String, CharacterSpriteAsset>,
    /// Per-prop sprite sheets keyed by the LDtk `Prop.kind` field.
    pub props: HashMap<String, CharacterSpriteAsset>,
    /// What each token names: token to catalog id. Permanent.
    ///
    /// Startup declares the whole roster and decodes none of it. The engine
    /// materializer realizes only the ids a session demands, which keeps a large
    /// decode off the boot path. The entry survives the decode because it is the
    /// recipe to make the realization again after retirement.
    declared: HashMap<String, String>,
    /// Tokens whose realization was retired, and the tier it was retired from.
    ///
    /// Retiring removes the token from `sheets` but keeps it in `declared`. So
    /// without this map, "never materialized" and "retired by a quality change"
    /// are the same `Declared` state, and the placeholder warning cannot tell
    /// them apart.
    ///
    /// This is a trace only. Nothing reads it to decide what to load. It is
    /// cleared when the token is resident again.
    retired: HashMap<String, TextureResolutionScale>,
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

    /// Publish a realization under every token declared for `character_id`,
    /// plus the id itself. The declarations stay: see the type docs.
    pub fn publish(&mut self, character_id: &str, asset: CharacterSpriteAsset) {
        let tokens: Vec<String> = self
            .declared
            .iter()
            .filter(|(_, declared_id)| declared_id.as_str() == character_id)
            .map(|(token, _)| token.clone())
            .collect();
        for token in tokens {
            self.retired.remove(&token);
            self.sheets.insert(token, asset.clone());
        }
        // A character published without ever being declared (a test fixture, or
        // a host inserting a sheet directly) still resolves by its own id.
        self.retired.remove(character_id);
        self.sheets.insert(character_id.to_string(), asset);
    }

    /// Publish a realization under ONE explicit token.
    ///
    /// For content that builds its own sheet outside the catalog-declared path
    /// (an intro NPC, a demo enemy) and knows which tokens resolve to it.
    /// Prefer [`Self::publish`] for a declared character, so every declared
    /// token is covered.
    ///
    /// This does not create a declaration. That keeps the realization out of the
    /// quality transition: the engine has no recipe for art it did not build, so
    /// it cannot draw it again after retirement. See
    /// [`Self::retire_realizations`].
    pub fn publish_under(&mut self, token: &str, asset: CharacterSpriteAsset) {
        self.retired.remove(token);
        self.sheets.insert(token.to_string(), asset);
    }

    /// Every declared catalog id with no resident realization, deduplicated.
    pub fn declared_character_ids(&self) -> std::collections::BTreeSet<&str> {
        self.declared
            .iter()
            .filter(|(token, _)| !self.sheets.contains_key(token.as_str()))
            .map(|(_, id)| id.as_str())
            .collect()
    }

    /// True when `character_id` is declared and has no resident realization.
    pub fn is_declared(&self, character_id: &str) -> bool {
        self.declared.contains_key(character_id) && !self.sheets.contains_key(character_id)
    }

    /// The catalog id a token names, declared or resident.
    pub fn character_id_for(&self, token: &str) -> Option<&str> {
        self.declared.get(token).map(String::as_str)
    }

    /// The tier a token's realization was retired from, if any.
    ///
    /// [`CharacterSheetState::Declared`] cannot carry this. `Some` means the
    /// sheet was decoded and then dropped by a quality transition. `None` means
    /// nothing has realized it.
    ///
    /// `None` is also the answer for an undeclared token, because only declared
    /// tokens are retired. Ask [`Self::sheet_state`] first; this answer is only
    /// useful for a `Declared` token.
    pub fn retired_tier(&self, token: &str) -> Option<TextureResolutionScale> {
        self.retired.get(token).copied()
    }

    /// The character id a token was declared under, whether or not a sheet is
    /// resident for it.
    pub fn declared_id(&self, token: &str) -> Option<&str> {
        self.declared.get(token).map(String::as_str)
    }

    /// Resident realizations whose requested tier is not `active`, as
    /// `(token, character id)` pairs. A quality transition acts on these.
    /// Compare `requested_tier`, not [`CharacterSpriteAsset::resolved_tier`]: a
    /// sheet with no baked variant answers `Half` with full-resolution pixels,
    /// and its resolved tier would stay stale forever.
    pub fn stale_realizations(&self, active: TextureResolutionScale) -> Vec<(String, String)> {
        self.sheets
            .iter()
            .filter(|(_, asset)| asset.requested_tier != active)
            .filter_map(|(token, _)| {
                self.declared
                    .get(token)
                    .map(|id| (token.clone(), id.clone()))
            })
            .collect()
    }

    /// Drop these resident realizations (declared tokens only), keeping their
    /// declarations so a later demand can realize them again. Returns the
    /// character ids that lost a realization.
    ///
    /// Do not use this for a sheet that a live body draws. A quality transition
    /// demands those again and [`Self::publish`] replaces them in place, so the
    /// body keeps its sheet until the new texture is ready. Retiring a sheet in
    /// use shows the placeholder rectangle until the reload completes.
    pub fn retire_realizations(
        &mut self,
        tokens: impl IntoIterator<Item = String>,
    ) -> std::collections::BTreeSet<String> {
        self.retire_tokens(tokens.into_iter().collect())
    }

    /// Every tier that is physically resident: the tiers the decoded bytes came
    /// from, not the requested tiers.
    ///
    /// After a quality transition completes, there should be one active tier.
    /// More than one means some body draws pixels the user no longer asks for.
    ///
    /// Two tiers here is not by itself a failure: a fallback is a permanent,
    /// correct difference. Ask [`Self::stale_realizations`] whether the
    /// transition has settled.
    pub fn resident_tiers(&self) -> std::collections::BTreeSet<TextureResolutionScale> {
        self.sheets
            .values()
            .map(|asset| asset.resolved_tier)
            .collect()
    }

    /// Retire every declared realization whose character id is not in `keep`,
    /// and return the retired ids. This is the room-exit half of residency.
    ///
    /// A realization belongs to a room that places the character, a body that
    /// wears it, or a neighbour that prefetch decodes for. The caller that
    /// commits a room transition names that set. This does not change tiers: the
    /// tier is the user's setting everywhere, and no room may lower it.
    pub fn retire_realizations_except(
        &mut self,
        keep: &std::collections::BTreeSet<String>,
    ) -> std::collections::BTreeSet<String> {
        let unowned: Vec<String> = self
            .sheets
            .keys()
            .filter(|token| {
                self.declared
                    .get(*token)
                    .is_some_and(|id| !keep.contains(id))
            })
            .cloned()
            .collect();
        self.retire_tokens(unowned)
    }

    fn retire_tokens(&mut self, tokens: Vec<String>) -> std::collections::BTreeSet<String> {
        let mut ids = std::collections::BTreeSet::new();
        for token in tokens {
            if let Some(id) = self.declared.get(&token) {
                ids.insert(id.clone());
            }
            // Record the tier it held, not the requested tier.
            if let Some(asset) = self.sheets.remove(&token) {
                self.retired.insert(token, asset.resolved_tier);
            }
        }
        ids
    }

    /// The main lookup. `token` is a stable catalog id or an authored display
    /// name; the table is double-keyed so both reach the same sheet.
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

    /// Every resident token and the realization it resolves to.
    ///
    /// `declared_character_ids` is the complement (tokens with no resident
    /// sheet), so tests that check resident tokens after a quality change need
    /// this.
    ///
    /// Order is not defined (`sheets` is a `HashMap`). Sort if you need
    /// determinism.
    pub fn resident_sheets(&self) -> impl Iterator<Item = (&str, &CharacterSpriteAsset)> {
        self.sheets
            .iter()
            .map(|(token, asset)| (token.as_str(), asset))
    }
}
