//! Loaded character spritesheet handles shared by loaders and renderers.
//!
//! That shape made the engine know four of Ambition's characters by name, so a second provider's
//! protagonist could not be a hot-path character however it was authored, and it made "this id has
//! no sheet" and "there is no such id" the same answer (`None`). One map, keyed by what content
//! declares, answers both questions honestly.

use std::collections::HashMap;

use bevy::prelude::*;

use ambition_persistence::settings::TextureResolutionScale;

use super::CharacterSpriteAsset;

/// What the sheet table knows about one authored character token.
///
/// `Declared` is the NONRESIDENT state, in both directions. It began life
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

// Diagnostics want the STATE named, not the atlas dumped.
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
/// ## Two facts, and they have different lifetimes
///
/// A DECLARATION ("this token names character X, whose sheet the engine knows how to build") is
/// permanent knowledge about content. A REALIZATION ("here are decoded handles for X at tier T") is
/// a physical resource with an owner and an end.
#[derive(Resource, Default, Clone)]
pub struct CharacterSpriteAssets {
    /// Resident realizations. Double-keyed (see above).
    sheets: HashMap<String, CharacterSpriteAsset>,
    /// Per-prop sprite sheets keyed by the LDtk `Prop.kind` field.
    pub props: HashMap<String, CharacterSpriteAsset>,
    /// What each token NAMES: token → catalog id. Permanent.
    ///
    /// Startup declares the whole roster here and decodes none of it; the engine
    /// materializer realizes the ids a session actually demands. That is what
    /// keeps a ~130-sheet decode storm off the boot path without making any one
    /// character privileged. The entry SURVIVES the decode, because it is also
    /// the recipe for re-making the realization after one is retired.
    declared: HashMap<String, String>,
    /// Tokens whose realization was RETIRED, and the tier it was retired from.
    ///
    /// ⛔⛔ WITHOUT THIS, `Declared` ALIASES TWO DIFFERENT FACTS. Retiring drops
    /// the token from `sheets` and deliberately leaves `declared` standing (it is
    /// the recipe), so "never materialized" and "materialized, then retired by a
    /// quality change" become the same observable state — which is exactly what
    /// [`CharacterSheetState::Declared`]'s own doc says it means, and exactly
    /// what the placeholder-rectangle warning could not tell apart. It reported
    /// *"nothing demanded it"* for both, and that warning fired 111 times on one
    /// Hall reveal, so its diagnosis was evidence for a cause nobody checked.
    ///
    /// ⚠ It is a TRACE, not state anything decides on. Nothing reads it to choose
    /// what to load; it exists so a report can say which of the two happened.
    /// Cleared the moment the token is resident again, so a re-realized character
    /// stops being described by a retirement it recovered from.
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
    /// (an intro NPC, a demo's bespoke enemy) and knows exactly which tokens
    /// should resolve to it. Prefer [`Self::publish`] when the character was
    /// declared, so every token it was declared under is covered automatically
    /// rather than by the caller remembering to list them.
    ///
    /// this does NOT create a declaration, and that is what keeps such a
    /// realization out of the quality transition: the engine has no recipe for
    /// art it did not build, so retiring it would delete a face with no way to
    /// draw it again. See [`Self::demote_stale_realizations`].
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

    /// The tier a token's realization was RETIRED from, if it ever had one.
    ///
    /// The half of [`CharacterSheetState::Declared`] the state itself cannot
    /// carry: `Some` means the sheet was decoded and then dropped by a quality
    /// transition, `None` means nothing has ever realized it. Both are
    /// `Declared`, and a report that does not ask this cannot tell them apart —
    /// which is how *"nothing demanded it"* came to be printed for characters
    /// that had been demanded and served.
    ///
    /// ⚠ `None` is also the answer for a token that was never declared, because
    /// only declared tokens are retired. Ask [`Self::sheet_state`] first: this
    /// question only means anything about a `Declared` one.
    pub fn retired_tier(&self, token: &str) -> Option<TextureResolutionScale> {
        self.retired.get(token).copied()
    }

    /// Is anything resident whose REQUEST is no longer the active one?
    ///
    /// `requested_tier`, deliberately — the question is "has the active
    /// setting been answered for everybody", not "what is in memory". A sheet
    /// with no baked variant answers `Half` with full-resolution pixels, and
    /// asking [`CharacterSpriteAsset::resolved_tier`] here would call it stale
    /// forever: the transition below would retire it, remake it identically, and
    /// do that again next frame. See [`CharacterSpriteAsset::requested_tier`].
    ///
    /// Read-only on purpose: the transition below takes `&mut self`, and a
    /// system that took the mutable borrow every frame would mark the whole
    /// asset resource changed every frame.
    pub fn has_stale_realizations(&self, active: TextureResolutionScale) -> bool {
        self.has_stale_realizations_outside(active, active)
    }

    /// A realization is stale when its tier is BELOW `floor` (drawn from pixels
    /// too small for where it is shown) or ABOVE `ceiling` (bigger than the
    /// user's setting asks for). One tier for both is the old exact rule.
    ///
    /// The room a character stands in sets the floor
    /// (`room_sprite_tier_cap`: a gallery of 132-px pedestals needs Quarter),
    /// the user's setting sets the ceiling. A Full sheet standing in a gallery
    /// is merely oversampled and is KEPT — demoting it on every hall entry and
    /// re-decoding it on every exit would be churn for nothing; a Quarter sheet
    /// carried out into a Full room is too small and goes.
    pub fn has_stale_realizations_outside(
        &self,
        floor: TextureResolutionScale,
        ceiling: TextureResolutionScale,
    ) -> bool {
        self.sheets.iter().any(|(token, asset)| {
            (asset.requested_tier < floor || asset.requested_tier > ceiling)
                && self.declared.contains_key(token)
        })
    }

    /// Every tier that is PHYSICALLY resident — the tiers the decoded bytes
    /// came from, not the tiers that were asked for.
    ///
    /// the invariant this exists for: after a quality transition completes
    /// there is exactly ONE active tier across the live residency set. More
    /// than one means some body on screen is being drawn from pixels the user
    /// stopped asking for.
    ///
    /// two tiers here is therefore NOT by itself a convergence failure: a
    /// fallback is a permanent, correct disagreement. Ask
    /// [`Self::has_stale_realizations`] whether the transition has settled.
    pub fn resident_tiers(&self) -> std::collections::BTreeSet<TextureResolutionScale> {
        self.sheets
            .values()
            .map(|asset| asset.resolved_tier)
            .collect()
    }

    /// Dropping the [`CharacterSpriteAsset`] drops its strong `Handle<Image>`;
    /// Bevy frees the image once the last strong handle goes, which is every
    /// clone here plus whatever a live presentation still holds until it
    /// rebinds. there is no evictor and there must not be one — ownership
    /// does the whole job.
    ///
    /// only DECLARED tokens are retired.
    ///
    /// staleness is [`CharacterSpriteAsset::requested_tier`] — same reason as
    /// [`Self::has_stale_realizations`], and this is the half where getting it
    /// wrong costs an `asset_server.load` every frame rather than a wrong
    /// number.
    pub fn demote_stale_realizations(
        &mut self,
        active: TextureResolutionScale,
    ) -> std::collections::BTreeSet<String> {
        self.demote_stale_realizations_outside(active, active)
    }

    /// [`Self::demote_stale_realizations`] with the floor/ceiling rule of
    /// [`Self::has_stale_realizations_outside`].
    pub fn demote_stale_realizations_outside(
        &mut self,
        floor: TextureResolutionScale,
        ceiling: TextureResolutionScale,
    ) -> std::collections::BTreeSet<String> {
        let stale: Vec<String> = self
            .sheets
            .iter()
            .filter(|(token, asset)| {
                (asset.requested_tier < floor || asset.requested_tier > ceiling)
                    && self.declared.contains_key(*token)
            })
            .map(|(token, _)| token.clone())
            .collect();
        let mut ids = std::collections::BTreeSet::new();
        for token in stale {
            if let Some(id) = self.declared.get(&token) {
                ids.insert(id.clone());
            }
            // The tier it HELD, not the tier that was asked for: a report saying
            // "retired from Full" is describing pixels that existed.
            if let Some(asset) = self.sheets.remove(&token) {
                self.retired.insert(token, asset.resolved_tier);
            }
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

    /// Every RESIDENT token and the realization it resolves to.
    ///
    /// added because its absence made a whole class of test unwritable
    /// . `ready_token_count` gives a number, and
    /// `declared_character_ids` is this set's COMPLEMENT — it filters to tokens
    /// with NO resident sheet, exactly as `is_declared` says. So a test asking
    /// *"after a quality change, does each resident token still resolve to the
    /// same character's file?"* had no way to enumerate its own subject, and
    /// reaching for `declared_character_ids` instead yields a tautology: every
    /// id in it is guaranteed to have no sheet.
    ///
    /// read-only and order-free. `sheets` is a `HashMap`, so a caller that
    /// needs determinism must collect and sort — this deliberately does not
    /// impose an order it would then have to promise.
    pub fn resident_sheets(&self) -> impl Iterator<Item = (&str, &CharacterSpriteAsset)> {
        self.sheets
            .iter()
            .map(|(token, asset)| (token.as_str(), asset))
    }
}
