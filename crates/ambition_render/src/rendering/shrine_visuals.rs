//! Shrine visuals: the obelisk sprite sync and activation-pulse animation.
//! Reads the sim shrine state (`HealShrine` and the `ShrineActivationPulse`
//! resource) from the read-model seam.

use super::sheet_atlas::{atlas_layout_from_record, row_playback, RowPlayback};
use ambition_platformer2d_shared_tangle::binding::BindingLedger;
use ambition_platformer2d_shared_tangle::lifecycle::{
    ActiveSessionScope, SessionSpawnScope, SpawnSessionScopedExt,
};
use ambition_platformer2d_shared_tangle::shrine::ShrineActivationPulse;
use ambition_sim_view::{ShrineFact, ShrinesView};
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
/// "rest here" landmark. Uses `sprites/props/shrine_spritesheet.png` (with a flat
/// `sprites/props/shrine.png` fallback), scaled to the shrine's collision
/// footprint so its base sits at the floor.
pub fn sync_shrine_visual(
    mut commands: Commands,
    // Each shrine is drawn in its own live room, by that room's geometry.
    rooms: ambition_platformer2d_shared_tangle::lifecycle::LiveRoomOf<
        ambition_platformer2d_core::RoomGeometry,
    >,
    active_session: Option<Res<ActiveSessionScope>>,
    asset_server: Res<AssetServer>,
    sheet_registry: Option<Res<SheetRegistry>>,
    mut atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    mut visual_source: Local<Option<ShrineVisualSource>>,
    mut visual_cache: Local<HashMap<u64, Entity>>,
    mut transforms: Query<&mut Transform>,
    mut sprites: Query<&mut Sprite>,
    visuals: Query<(Entity, &ShrineVisualKey)>,
    shrines: Res<ShrinesView>,
) {
    let Some(session_scope) =
        SessionSpawnScope::for_optional_active_session(active_session.as_deref())
    else {
        return;
    };
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

    for shrine in &shrines.0 {
        let key = shrine_visual_key(shrine);
        present.insert(key);
        // A shrine whose live room cannot be told is not drawn.
        let Some((room, world)) = shrine.room.and_then(|room| Some((room, rooms.in_room(room)?))) else {
            continue;
        };
        let translation = ambition_platformer2d_core::config::world_to_bevy(&world.0, shrine.pos, 8.0);

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
            .spawn_session_scoped(
                session_scope.in_room(Some(room)),
                (
                    ShrineVisual,
                    ShrineVisualKey(key),
                    ShrineVisualAnim::default(),
                    shrine_visual_atlas(&source),
                    sprite,
                    Transform::from_translation(translation),
                    ambition_platformer2d_shared_tangle::lifecycle::RoomVisual,
                    Name::new("Shrine visual"),
                ),
            )
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
            // The cached entity may already be gone: shrine visuals carry
            // `RoomVisual` (so `RoomScopedEntity`), and a room transition despawns
            // them without updating this `Local` cache. Guard the despawn instead
            // of commanding a stale handle.
            if let Ok(mut ec) = commands.get_entity(entity) {
                ec.despawn();
            }
        }
    }
}

pub fn animate_shrine_visuals(
    presentation_time: ambition_time::PresentationTime,
    // Read-only: the pulse timer ticks on the sim side
    // (`sim_view::tick_shrine_activation_pulse`).
    activation: Res<ShrineActivationPulse>,
    mut visuals: Query<
        (&mut Sprite, &mut ShrineVisualAnim, &ShrineVisualAtlas),
        With<ShrineVisual>,
    >,
) {
    let dt = presentation_time.scaled_dt();
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
    // Load through `load_sheet_image`, not a bare `asset_server.load`. It
    // stamps the demand road, so the residency census can attribute the
    // sheet, and it takes the `RENDER_WORLD`-only path, so no CPU copy stays
    // alive.
    // In `props/`, where `scripts/regen/sprites.sh` writes the shrine prop
    // (`write_shrine_prop`). The record's `image` is a bare file name. A load
    // of `sprites/shrine_spritesheet.png` failed in every game with a shrine,
    // and the shrine drew from a handle with no image.
    let image = ambition_sprite_sheet::game_assets::load_sheet_image(
        asset_server,
        "shrine-sheet",
        format!("sprites/props/{}", record.image),
    );

    let mut ledger = BindingLedger::new();
    let idle = row_playback(record, "idle", "shrine visual", &mut ledger);
    let activate = row_playback(record, "activate", "shrine visual", &mut ledger);
    ledger.finish().log("shrine visual");

    // The fallbacks stay: a shrine with an unresolvable row still draws (blind
    // runs must never go black). The report above keeps the miss visible.
    let idle = idle.unwrap_or(RowPlayback {
        start: 0,
        frames: 1,
        frame_duration: 0.15,
    });
    let activate = activate.unwrap_or(RowPlayback {
        start: 0,
        frames: 1,
        frame_duration: 0.09,
    });

    ShrineVisualSource::Atlas {
        image,
        layout,
        idle_start: idle.start,
        idle_frame_count: idle.frames,
        idle_duration: idle.frame_duration,
        activate_start: activate.start,
        activate_frame_count: activate.frames,
        activate_duration: activate.frame_duration,
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

fn shrine_visual_key(shrine: &ShrineFact) -> u64 {
    let mut hasher = DefaultHasher::new();
    shrine.pos.x.to_bits().hash(&mut hasher);
    shrine.pos.y.to_bits().hash(&mut hasher);
    shrine.half_extent.x.to_bits().hash(&mut hasher);
    shrine.half_extent.y.to_bits().hash(&mut hasher);
    // Two live instances of one room have their shrines at the same point;
    // each is its own visual.
    shrine.room.map(|room| room.ordinal()).hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two live instances of one room each draw their shrine (view half, cut
    /// V2h). The shrines stand at the same point of the same room, so the
    /// visual key must tell them apart by room, and each visual carries its
    /// room's stamp. A shrine whose room cannot be told is not drawn.
    #[test]
    fn two_instances_of_one_room_each_draw_their_own_shrine() {
        use ambition_platformer2d_core as ae;
        use ambition_platformer2d_shared_tangle::lifecycle::{
            insert_live_room_component, spawn_live_room, InRoomInstance, LiveRoomInstance,
        };
        let world = || {
            ae::RoomGeometry(ae::World::new("shrine room", ae::Vec2::new(800.0, 600.0), ae::Vec2::new(40.0, 40.0), Vec::new()))
        };
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(bevy::asset::AssetPlugin::default());
        app.init_asset::<Image>();
        app.init_asset::<TextureAtlasLayout>();
        insert_live_room_component(app.world_mut(), world());
        let second = LiveRoomInstance::ACTIVATION.next();
        spawn_live_room(app.world_mut(), second, world());
        let shrine = |room| ShrineFact {
            pos: ae::Vec2::new(100.0, 200.0),
            half_extent: ae::Vec2::new(16.0, 32.0),
            room,
        };
        app.insert_resource(ShrinesView(vec![
            shrine(Some(LiveRoomInstance::ACTIVATION)),
            shrine(Some(second)),
            shrine(None),
        ]));
        app.add_systems(Update, sync_shrine_visual);
        app.update();
        app.update();

        let world = app.world_mut();
        let mut q = world.query_filtered::<Option<&InRoomInstance>, With<ShrineVisual>>();
        let mut rooms: Vec<Option<u32>> = q.iter(world).map(|stamp| stamp.map(|stamp| stamp.0.ordinal())).collect();
        rooms.sort();
        assert_eq!(
            rooms,
            vec![Some(LiveRoomInstance::ACTIVATION.ordinal()), Some(second.ordinal())],
            "each live instance must draw its own shrine, stamped with its room"
        );

        // The second instance's shrine goes (its room retired): its visual
        // goes with it, and the first instance keeps its own.
        app.insert_resource(ShrinesView(vec![shrine(Some(LiveRoomInstance::ACTIVATION))]));
        app.update();
        app.update();
        let world = app.world_mut();
        let mut q = world.query_filtered::<Option<&InRoomInstance>, With<ShrineVisual>>();
        let rooms: Vec<Option<u32>> = q.iter(world).map(|stamp| stamp.map(|stamp| stamp.0.ordinal())).collect();
        assert_eq!(
            rooms,
            vec![Some(LiveRoomInstance::ACTIVATION.ordinal())],
            "the second instance's shrine is gone and its visual stayed"
        );
    }
    #[test]
    fn shrine_sheet_exposes_idle_then_activate_rows() {
        let registry = ambition_sprite_sheet::baked_sheet_registry();
        let record = registry.get("shrine").expect("shrine sheet record");
        assert_eq!(record.rows.len(), 2);

        let mut ledger = BindingLedger::new();
        let idle = row_playback(record, "idle", "test", &mut ledger).expect("idle row");
        let activate = row_playback(record, "activate", "test", &mut ledger).expect("activate row");
        assert!(
            ledger.finish().is_empty(),
            "the shipped shrine sheet still spells both rows the visual asks for"
        );

        assert_eq!((idle.start, idle.frames), (0, 6));
        assert_eq!((activate.start, activate.frames), (6, 8));
    }
}
