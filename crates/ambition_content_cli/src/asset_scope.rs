//! Export room and art bindings from the same lowered values the runtime reads.

use ambition_content_pack::{lowered_rules, PreparedContentPack};
use ambition_platformer2d_ldtk::{LdtkProject, LdtkVocabulary};
use ambition_platformer2d_world::world_manifest::{AuthoredWorldManifest, WORLD_MANIFEST_SCHEMA};
use serde_json::{json, Value};
use std::collections::BTreeMap;

pub fn describe(pack: &PreparedContentPack) -> Value {
    let mut rooms = BTreeMap::new();
    let mut failures = Vec::new();
    if let Some(worlds) = lowered_rules::<AuthoredWorldManifest>(pack, WORLD_MANIFEST_SCHEMA) {
        for world in &worlds.worlds {
            let path = pack.root.join("worlds").join(&world.file);
            let result = LdtkProject::load_from_path(&path).and_then(|project|
                project.to_room_set_with_entry(&worlds.entry_room, &LdtkVocabulary::engine())
                    .map_err(|errors| errors.join("\n")));
            match result {
                Ok(set) => {
                    for room in &set.rooms {
                        let theme = ambition_sprite_sheet::game_assets::ParallaxTheme::named_by_room_metadata(&room.metadata)
                            .map(|theme| theme.key());
                        rooms.insert(room.id.clone(), json!({"source": path, "world": world.id, "spec": room, "theme": theme}));
                    }
                }
                Err(error) => failures.push(json!({"source": path, "error": error})),
            }
        }
    }
    let held = ambition_items::content_schema::held_item_art::lowered_held_item_art(pack)
        .map(|rows| rows.iter().map(|(id, row)| (id.clone(), row.path.clone())).collect::<BTreeMap<_, _>>());
    let props = lowered_rules::<ambition_sprite_sheet::prop_sheets::PropSheetRows>(pack, "prop_sheets")
        .map(|rows| rows.iter().map(|(id, row)| (id.clone(), row.target(id).to_owned())).collect::<BTreeMap<_, _>>());
    let platforms = ambition_sprite_sheet::prop_sheets::lowered_platform_looks(pack)
        .map(|rows| rows.iter().map(|(id, row)| (id.clone(), row.sheet.as_ref().unwrap_or(id).clone())).collect::<BTreeMap<_, _>>());
    let bosses = lowered_rules::<ambition_boss_encounter::BossArtKeys>(pack, "boss_art_keys")
        .map(|keys| &keys.sprite_filenames);
    let world_items = ambition_items::content_schema::world_item_art::lowered_world_item_art(pack)
        .map(|rows| rows.iter().map(|(id, row)| (id.clone(), row.path.clone())).collect::<BTreeMap<_, _>>());
    let encounters = ambition_encounter::content_schema::lowered_encounter_waves(pack)
        .map(|rows| rows.iter().map(|(id, waves)| (id.clone(), waves.iter()
            .flat_map(|wave| &wave.mobs).filter_map(|mob| mob.character.clone()).collect::<Vec<_>>()))
            .collect::<BTreeMap<_, _>>());
    let music = ambition_audio::content_schema::lowered_music_registry(pack)
        .map(|registry| registry.tracks.iter().map(|track| (track.id.clone(), track.resolved_asset_path()))
            .collect::<BTreeMap<_, _>>());
    json!({"rooms": rooms, "room_failures": failures, "held_items": held, "props": props,
           "bosses": bosses, "platforms": platforms, "world_items": world_items, "encounters": encounters, "music": music})
}
