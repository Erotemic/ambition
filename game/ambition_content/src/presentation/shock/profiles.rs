//! The bone atlas and the body layouts it can draw.

use std::{collections::BTreeMap, io};

use ambition_characters::actor::character_catalog::{CharacterBodyKind, CharacterCatalogEntry};
use bevy::{
    asset::{io::Reader, AssetLoader, LoadContext},
    prelude::*,
};
use serde::Deserialize;

pub(super) const ATLAS: &str = "content://ambition_shared/sprites/shock_bones.shock.ron";
const IMAGE: &str = "content://ambition_shared/sprites/shock_bones.png";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Layout {
    Biped,
    Quadruped,
    SkullOnly,
}

pub(super) struct Profile {
    pub skull: u32,
    pub layout: Layout,
}

#[derive(Asset, TypePath)]
pub(super) struct BoneAtlas {
    #[dependency]
    pub image: Handle<Image>,
    spec: AtlasSpec,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AtlasSpec {
    skulls: BTreeMap<String, u32>,
    chargen: BTreeMap<String, String>,
}

impl BoneAtlas {
    pub(super) fn profile(&self, entry: &CharacterCatalogEntry) -> Option<Profile> {
        self.spec.profile(entry)
    }
}

impl AtlasSpec {
    fn profile(&self, entry: &CharacterCatalogEntry) -> Option<Profile> {
        let has = |name| entry.tags.iter().any(|tag| tag == name);
        if [
            "boneless",
            "construct",
            "elemental",
            "ghost",
            "slime",
            "robot",
            "automaton",
            "mechanical",
            "synthetic",
            "insect",
            "insectoid",
            "plant",
            "spirit",
        ]
        .into_iter()
        .any(has)
        {
            return None;
        }
        let chargen = entry
            .manifest_target()
            .and_then(|sheet| self.chargen.get(sheet.rsplit('/').next()?));
        if let Some(species) = chargen {
            return Some(Profile {
                skull: *self.skulls.get(species)?,
                layout: Layout::Biped,
            });
        }

        let species = entry.tags.iter().find_map(|tag| {
            let name = match tag.as_str() {
                "wolf" | "coyote" | "fox" | "hound" => "dog",
                "goat" | "sheep" => "ram",
                "crocodile" => "alligator",
                _ => tag,
            };
            self.skulls.contains_key(name).then_some(name)
        });
        let land_animal = matches!(
            species,
            Some("alligator" | "bear" | "cat" | "dog" | "horse" | "rabbit" | "ram")
        );
        let bird = matches!(species, Some("bird" | "finch" | "owl" | "pigeon"));
        let humanoid = ["anthropomorphic", "humanoid", "townsfolk", "story", "npc"]
            .into_iter()
            .any(has);
        let wide = matches!(
            entry.body_kind,
            CharacterBodyKind::Wide | CharacterBodyKind::Crawler
        );

        let layout = if has("quadruped") || (land_animal && !humanoid && (has("animal") || wide)) {
            Layout::Quadruped
        } else if humanoid
            || (entry.body_kind == CharacterBodyKind::Standard && !bird && !has("animal"))
        {
            Layout::Biped
        } else {
            Layout::SkullOnly
        };
        let species = match layout {
            Layout::Biped => species.unwrap_or("human"),
            _ => species?,
        };
        Some(Profile {
            skull: *self.skulls.get(species)?,
            layout,
        })
    }
}

#[derive(Default, TypePath)]
pub(super) struct BoneAtlasLoader;

impl AssetLoader for BoneAtlasLoader {
    type Asset = BoneAtlas;
    type Settings = ();
    type Error = io::Error;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _: &(),
        context: &mut LoadContext<'_>,
    ) -> Result<BoneAtlas, io::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let spec: AtlasSpec = ron::de::from_bytes(&bytes).map_err(io::Error::other)?;
        if !spec.skulls.contains_key("human")
            || spec
                .chargen
                .values()
                .any(|species| !spec.skulls.contains_key(species))
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "A shock profile has no skull cell",
            ));
        }
        Ok(BoneAtlas {
            image: context.load(IMAGE),
            spec,
        })
    }

    fn extensions(&self) -> &[&str] {
        &["shock.ron"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(sheet: &str, tags: &[&str]) -> CharacterCatalogEntry {
        ron::from_str(&format!(
            r#"(
                display_name: "Shock fixture",
                manifest: "sprites/{sheet}_spritesheet.ron",
                tier: MainHall,
                body_kind: Standard,
                default_action_set: "peaceful",
                tags: {tags:?},
            )"#
        ))
        .unwrap()
    }

    #[test]
    fn one_skull_can_fit_two_body_layouts() {
        let atlas = AtlasSpec {
            skulls: BTreeMap::from([("human".into(), 0), ("dog".into(), 13)]),
            chargen: BTreeMap::from([("foreman".into(), "dog".into())]),
        };
        let foreman = atlas.profile(&entry("foreman", &["townsfolk"])).unwrap();
        let hound = atlas
            .profile(&entry("hound", &["dog", "quadruped"]))
            .unwrap();
        assert_eq!(foreman.skull, hound.skull);
        assert_eq!(foreman.layout, Layout::Biped);
        assert_eq!(hound.layout, Layout::Quadruped);
        assert!(atlas.profile(&entry("ghost", &["dog", "ghost"])).is_none());
    }
}
