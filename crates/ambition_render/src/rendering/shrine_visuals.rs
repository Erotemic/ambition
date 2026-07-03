//! Shrine visuals (was the render tail of ambition_gameplay_core::shrine): the obelisk
//! sprite sync + activation-pulse animation. Reads the sim shrine state
//! (HealShrine, ShrineActivationPulse) from ambition_gameplay_core.

use super::sheet_atlas::{
    atlas_layout_from_record, row_duration, row_frame_count, row_start_index,
};
use ambition_gameplay_core::shrine::{HealShrine, ShrineActivationPulse};
use ambition_sprite_sheet::{SheetRecord, SheetRegistry};
use bevy::prelude::*;
use bevy::{image::TextureAtlas, image::TextureAtlasLayout};
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

/// Marks the shrine's visual.
#[derive(Component)]
pub struct ShrineVisual;

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ShrineVisualKey(u64);

#[derive(Component, Clone, Copy, Debug, Default)]
pub struct ShrineVisualAnim {
    mode: ShrineVisualMode,
    frame: usize,
    elapsed: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum ShrineVisualMode {
    #[default]
    Idle,
    Activate,
}

#[derive(Component, Clone, Copy, Debug)]
pub struct ShrineVisualAtlas {
    idle_start: usize,
    idle_frame_count: usize,
    idle_duration: f32,
    activate_start: usize,
    activate_frame_count: usize,
    activate_duration: f32,
}

#[derive(Clone)]
pub enum ShrineVisualSource {
    Flat(Handle<Image>),
    Atlas {
        image: Handle<Image>,
        layout: Handle<TextureAtlasLayout>,
        idle_start: usize,
        idle_frame_count: usize,
        idle_duration: f32,
        activate_start: usize,
        activate_frame_count: usize,
        activate_duration: f32,
    },
}

/// Draw each shrine as its obelisk prop sprite so the player reads it as a
/// "rest here" landmark. The shrine now uses the authored
/// `sprites/shrine_spritesheet.png` sheet (with a flat `sprites/props/shrine.png`
/// fallback), and is scaled to the shrine's collision footprint so its base
/// sits at the floor.
pub fn sync_shrine_visual(
    mut commands: Commands,
    world: Res<ambition_gameplay_core::RoomGeometry>,
    asset_server: Res<AssetServer>,
    sheet_registry: Option<Res<SheetRegistry>>,
    mut atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    mut visual_source: Local<Option<ShrineVisualSource>>,
    mut visual_cache: Local<HashMap<u64, Entity>>,
    mut transforms: Query<&mut Transform>,
    mut sprites: Query<&mut Sprite>,
    visuals: Query<(Entity, &ShrineVisualKey)>,
    shrines: Query<&HealShrine>,
) {
    let source = shrine_visual_source(
        &asset_server,
        sheet_registry.as_ref().map(|registry| &**registry),
        &mut atlas_layouts,
        &mut visual_source,
    );

    let mut present = HashSet::new();
    for (entity, key) in &visuals {
        visual_cache.insert(key.0, entity);
    }

    for shrine in &shrines {
        let key = shrine_visual_key(shrine);
        present.insert(key);
        let translation = ambition_engine_core::config::world_to_bevy(&world.0, shrine.pos, 8.0);

        if let Some(&entity) = visual_cache.get(&key) {
            let mut matched = true;
            if let Ok(mut transform) = transforms.get_mut(entity) {
                transform.translation = translation;
            } else {
                matched = false;
            }
            if let Ok(mut sprite) = sprites.get_mut(entity) {
                sprite.custom_size = Some(shrine.half_extent * 2.0);
            } else {
                matched = false;
            }
            if matched {
                continue;
            }
            visual_cache.remove(&key);
        }

        let mut sprite = match &source {
            ShrineVisualSource::Flat(image) => Sprite::from_image(image.clone()),
            ShrineVisualSource::Atlas {
                image,
                layout,
                idle_start,
                ..
            } => Sprite::from_atlas_image(
                image.clone(),
                TextureAtlas {
                    layout: layout.clone(),
                    index: *idle_start,
                },
            ),
        };
        sprite.custom_size = Some(shrine.half_extent * 2.0);

        let entity = commands
            .spawn((
                ShrineVisual,
                ShrineVisualKey(key),
                ShrineVisualAnim::default(),
                shrine_visual_atlas(&source),
                sprite,
                Transform::from_translation(translation),
                ambition_gameplay_core::platformer_runtime::lifecycle::RoomVisual,
                Name::new("Shrine visual"),
            ))
            .id();
        visual_cache.insert(key, entity);
    }

    let stale: Vec<u64> = visual_cache
        .keys()
        .copied()
        .filter(|key| !present.contains(key))
        .collect();
    for key in stale {
        if let Some(entity) = visual_cache.remove(&key) {
            // The cached entity may already be gone — shrine visuals carry
            // `RoomVisual` (=> `RoomScopedEntity`), so a room transition despawns
            // them out from under this `Local` cache. Guard the despawn instead of
            // commanding a stale handle (which raised the "Entity despawned" error).
            if let Ok(mut ec) = commands.get_entity(entity) {
                ec.despawn();
            }
        }
    }
}

pub fn animate_shrine_visuals(
    world_time: Res<ambition_time::WorldTime>,
    mut activation: ResMut<ShrineActivationPulse>,
    mut visuals: Query<
        (&mut Sprite, &mut ShrineVisualAnim, &ShrineVisualAtlas),
        With<ShrineVisual>,
    >,
) {
    let dt = world_time.scaled_dt;
    if activation.remaining > 0.0 {
        activation.remaining = (activation.remaining - dt).max(0.0);
    }
    let active = activation.remaining > 0.0;

    for (mut sprite, mut anim, atlas) in &mut visuals {
        let target = if active {
            ShrineVisualMode::Activate
        } else {
            ShrineVisualMode::Idle
        };
        if anim.mode != target {
            anim.mode = target;
            anim.frame = 0;
            anim.elapsed = 0.0;
        }

        let (start, frame_count, duration) = if active {
            (
                atlas.activate_start,
                atlas.activate_frame_count,
                atlas.activate_duration,
            )
        } else {
            (
                atlas.idle_start,
                atlas.idle_frame_count,
                atlas.idle_duration,
            )
        };
        let frame_count = frame_count.max(1);
        if duration > 0.0 {
            anim.elapsed += dt;
            while anim.elapsed >= duration {
                anim.elapsed -= duration;
                if anim.frame + 1 >= frame_count {
                    if active {
                        anim.frame = frame_count - 1;
                        break;
                    } else {
                        anim.frame = 0;
                    }
                } else {
                    anim.frame += 1;
                }
            }
        }

        if let Some(atlas_sprite) = sprite.texture_atlas.as_mut() {
            atlas_sprite.index = start + anim.frame.min(frame_count - 1);
        }
        sprite.color = if active {
            Color::srgba(1.0, 0.70, 0.70, 1.0)
        } else {
            Color::WHITE
        };
    }
}

fn shrine_visual_source(
    asset_server: &AssetServer,
    sheet_registry: Option<&SheetRegistry>,
    atlas_layouts: &mut Assets<TextureAtlasLayout>,
    cache: &mut Option<ShrineVisualSource>,
) -> ShrineVisualSource {
    if let Some(source) = cache.as_ref() {
        return source.clone();
    }

    let source = if let Some(registry) = sheet_registry {
        if let Some(record) = registry.get("shrine") {
            shrine_visual_source_from_record(asset_server, atlas_layouts, record)
        } else {
            ShrineVisualSource::Flat(asset_server.load("sprites/props/shrine.png"))
        }
    } else {
        ShrineVisualSource::Flat(asset_server.load("sprites/props/shrine.png"))
    };
    *cache = Some(source.clone());
    source
}

fn shrine_visual_source_from_record(
    asset_server: &AssetServer,
    atlas_layouts: &mut Assets<TextureAtlasLayout>,
    record: &SheetRecord,
) -> ShrineVisualSource {
    let layout = atlas_layouts.add(atlas_layout_from_record(record));
    let image = asset_server.load("sprites/shrine_spritesheet.png");
    ShrineVisualSource::Atlas {
        image,
        layout,
        idle_start: row_start_index(record, "idle").unwrap_or(0),
        idle_frame_count: row_frame_count(record, "idle").unwrap_or(1),
        idle_duration: row_duration(record, "idle").unwrap_or(0.15),
        activate_start: row_start_index(record, "activate").unwrap_or(0),
        activate_frame_count: row_frame_count(record, "activate").unwrap_or(1),
        activate_duration: row_duration(record, "activate").unwrap_or(0.09),
    }
}

fn shrine_visual_atlas(source: &ShrineVisualSource) -> ShrineVisualAtlas {
    match source {
        ShrineVisualSource::Flat(_) => ShrineVisualAtlas {
            idle_start: 0,
            idle_frame_count: 1,
            idle_duration: 1.0,
            activate_start: 0,
            activate_frame_count: 1,
            activate_duration: 1.0,
        },
        ShrineVisualSource::Atlas {
            idle_start,
            idle_frame_count,
            idle_duration,
            activate_start,
            activate_frame_count,
            activate_duration,
            ..
        } => ShrineVisualAtlas {
            idle_start: *idle_start,
            idle_frame_count: *idle_frame_count,
            idle_duration: *idle_duration,
            activate_start: *activate_start,
            activate_frame_count: *activate_frame_count,
            activate_duration: *activate_duration,
        },
    }
}

fn shrine_visual_key(shrine: &HealShrine) -> u64 {
    let mut hasher = DefaultHasher::new();
    shrine.pos.x.to_bits().hash(&mut hasher);
    shrine.pos.y.to_bits().hash(&mut hasher);
    shrine.half_extent.x.to_bits().hash(&mut hasher);
    shrine.half_extent.y.to_bits().hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shrine_sheet_exposes_idle_then_activate_rows() {
        let registry = ambition_gameplay_core::character_sprites::baked_sheet_registry();
        let record = registry.get("shrine").expect("shrine sheet record");
        assert_eq!(record.rows.len(), 2);
        assert_eq!(row_start_index(record, "idle"), Some(0));
        assert_eq!(row_start_index(record, "activate"), Some(6));
        assert_eq!(row_frame_count(record, "idle"), Some(6));
        assert_eq!(row_frame_count(record, "activate"), Some(8));
    }
}
