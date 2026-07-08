//! Loaded character spritesheet handles shared by loaders and renderers.

use std::collections::HashMap;

use bevy::prelude::*;

use super::CharacterSpriteAsset;

/// Holds optional spritesheet handles. A missing PNG produces a `None` (or absent
/// map entry); callers fall back to colored rectangles.
#[derive(Resource, Default, Clone)]
pub struct CharacterSpriteAssets {
    /// Player-specific compact robot sheet. Preferred for the controlled body;
    /// falls back to `robot` when missing.
    pub player: Option<CharacterSpriteAsset>,
    /// Base robot sheet.
    pub robot: Option<CharacterSpriteAsset>,
    pub goblin: Option<CharacterSpriteAsset>,
    pub sandbag: Option<CharacterSpriteAsset>,
    /// Per-NPC and per-character sheets. The actor-side loader intentionally
    /// double-keys this map by both display name and catalog id so render can
    /// resolve either an authored display label or a stable character id without
    /// depending on the actor roster module.
    pub npcs: HashMap<String, CharacterSpriteAsset>,
    /// Per-prop sprite sheets keyed by the LDtk `Prop.kind` field.
    pub props: HashMap<String, CharacterSpriteAsset>,
}

impl CharacterSpriteAssets {
    /// Generic fallback sheet for an actor that resolved no named sprite.
    pub fn actor_fallback_asset(
        &self,
        is_sandbag: bool,
        fighting: bool,
    ) -> Option<&CharacterSpriteAsset> {
        if is_sandbag {
            self.sandbag.as_ref().or(self.goblin.as_ref())
        } else if fighting {
            self.goblin.as_ref()
        } else {
            None
        }
    }

    /// Pick a character spritesheet by authored display name.
    pub fn npc_asset_for_name(&self, name: &str) -> Option<&CharacterSpriteAsset> {
        self.npcs.get(name)
    }

    /// Pick a prop spritesheet by its registry key.
    pub fn prop_asset_for_kind(&self, kind: &str) -> Option<&CharacterSpriteAsset> {
        self.props.get(kind)
    }

    /// Resolve the loaded sprite asset for a catalog `character_id`.
    pub fn asset_for_character_id(&self, character_id: &str) -> Option<&CharacterSpriteAsset> {
        match character_id {
            "player" => self.player.as_ref().or(self.robot.as_ref()),
            "robot" => self.robot.as_ref(),
            "goblin" => self.goblin.as_ref(),
            "sandbag" => self.sandbag.as_ref(),
            _ => self.npcs.get(character_id),
        }
    }
}
