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

use ambition_persistence::settings::TextureResolutionScale;

use super::CharacterSpriteAsset;

/// What the sheet table knows about one authored character token.
///
/// The distinction between the last two variants is the point. `None` used to
/// mean both "declared, sheet not decoded yet" and "no such character", so a
/// TYPO and a pending decode were indistinguishable — a misspelled id drew the
/// placeholder rectangle forever and looked exactly like a sheet that had not
/// arrived. A caller that wants to wait can now tell it apart from one that
/// should report a binding failure.
///
/// ⭐ **`Declared` is the NONRESIDENT state, in both directions.** It began life
/// as a one-way "not decoded yet" and is now the state a realization returns to
/// when the active quality tier moves — see
/// [`CharacterSpriteAssets::demote_stale_realizations`]. That is why the
/// declaration outlives the decode: a token whose declaration was consumed on
/// publish had no way back.
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

/// Holds resident spritesheet realizations plus the declarations behind them.
///
/// Both maps are **double-keyed** by catalog id AND authored display name, so
/// presentation can resolve either a stable id or a legacy display label through
/// one lookup without depending on the actor roster module.
///
/// ## Two facts, and they have different lifetimes
///
/// A DECLARATION ("this token names character X, whose sheet the engine knows
/// how to build") is permanent knowledge about content. A REALIZATION ("here are
/// decoded handles for X at tier T") is a physical resource with an owner and an
/// end. Publishing used to CONSUME the declaration, which made the two one fact
/// and left `Ready` as a terminal state — there was no way back to `Declared`,
/// so a resident sheet could never be retired and re-made, which is exactly why
/// a live quality change could not converge.
#[derive(Resource, Default, Clone)]
pub struct CharacterSpriteAssets {
    /// Resident realizations. Double-keyed (see above).
    sheets: HashMap<String, CharacterSpriteAsset>,
    /// Per-prop sprite sheets keyed by the LDtk `Prop.kind` field.
    pub props: HashMap<String, CharacterSpriteAsset>,
    /// **What each token NAMES**: token → catalog id. Permanent.
    ///
    /// Startup declares the whole roster here and decodes none of it; the engine
    /// materializer realizes the ids a session actually demands. That is what
    /// keeps a ~130-sheet decode storm off the boot path without making any one
    /// character privileged. The entry SURVIVES the decode, because it is also
    /// the recipe for re-making the realization after one is retired.
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
            self.sheets.insert(token, asset.clone());
        }
        // A character published without ever being declared (a test fixture, or
        // a host inserting a sheet directly) still resolves by its own id.
        self.sheets.insert(character_id.to_string(), asset);
    }

    /// Publish a realization under ONE explicit token.
    ///
    /// For content that builds its own sheet outside the catalog-declared path
    /// (an intro NPC, a demo's bespoke enemy) and knows exactly which tokens
    /// should resolve to it. Prefer [`Self::publish`] when the character was
    /// declared, so every token it was declared under is covered automatically
    /// rather than by the caller remembering to list them.
    ///
    /// ⚠ this does NOT create a declaration, and that is what keeps such a
    /// realization out of the quality transition: the engine has no recipe for
    /// art it did not build, so retiring it would delete a face with no way to
    /// draw it again. See [`Self::demote_stale_realizations`].
    pub fn publish_under(&mut self, token: &str, asset: CharacterSpriteAsset) {
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

    /// **Is anything resident at a tier that is no longer the active one?**
    ///
    /// Read-only on purpose: the transition below takes `&mut self`, and a
    /// system that took the mutable borrow every frame would mark the whole
    /// asset resource changed every frame.
    pub fn has_stale_realizations(&self, active: TextureResolutionScale) -> bool {
        self.sheets
            .iter()
            .any(|(token, asset)| asset.tier != active && self.declared.contains_key(token))
    }

    /// Every tier a resident realization was made at.
    ///
    /// ⭐ **the invariant this exists for: after a quality transition completes
    /// there is exactly ONE active tier across the live residency set.** More
    /// than one means some body on screen is being drawn from pixels the user
    /// stopped asking for.
    pub fn resident_tiers(&self) -> std::collections::BTreeSet<TextureResolutionScale> {
        self.sheets.values().map(|asset| asset.tier).collect()
    }

    /// **The return edge.** Retire every resident realization that is no longer
    /// at the active tier, leaving its declaration behind, and name the catalog
    /// ids that must therefore be demanded again.
    ///
    /// Dropping the [`CharacterSpriteAsset`] drops its strong `Handle<Image>`;
    /// Bevy frees the image once the last strong handle goes, which is every
    /// clone here plus whatever a live presentation still holds until it
    /// rebinds. ⛔ **there is no evictor and there must not be one** — ownership
    /// does the whole job.
    ///
    /// ⚠ only DECLARED tokens are retired. A realization published without a
    /// declaration came from a host that built it itself
    /// ([`Self::publish_under`]), and the engine has no recipe to remake it, so
    /// retiring it would be a one-way deletion of somebody's art.
    pub fn demote_stale_realizations(
        &mut self,
        active: TextureResolutionScale,
    ) -> std::collections::BTreeSet<String> {
        let stale: Vec<String> = self
            .sheets
            .iter()
            .filter(|(token, asset)| asset.tier != active && self.declared.contains_key(*token))
            .map(|(token, _)| token.clone())
            .collect();
        let mut ids = std::collections::BTreeSet::new();
        for token in stale {
            if let Some(id) = self.declared.get(&token) {
                ids.insert(id.clone());
            }
            self.sheets.remove(&token);
        }
        ids
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
