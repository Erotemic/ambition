//! Healing / save-point shrine.
//!
//! An interactable shrine that, on a single `Interact`, **heals the player to
//! full** (health + mana) and acts as a **save point** (decided: one Interact
//! does both). The save is a checkpoint write: touching `Res<SandboxSave>` marks
//! it changed, and the existing `autosave_sandbox_save` persists it (desktop;
//! no-op on wasm).
//!
//! Handoff / not-yet-built:
//! - placement is LDtk-authored (`ShrineSpawn`); routing the heal/save through
//!   the affordance/prompt system via an `Interactable` is the follow-up (see
//!   TODO "Healing / save-point shrine").

use bevy::prelude::*;
use bevy::{image::TextureAtlas, image::TextureAtlasLayout, math::UVec2};
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

use crate::engine_core::{self as ae, AabbExt};
use crate::input::ControlFrame;
use crate::player::{BodyKinematics, PlayerEntity, PlayerHealth, PlayerMana, PrimaryPlayer};
use ambition_sprite_sheet::{SheetRecord, SheetRegistry};

/// A healing / save-point shrine the player can `Interact` with.
#[derive(Component, Clone, Copy, Debug)]
pub struct HealShrine {
    pub pos: Vec2,
    pub half_extent: Vec2,
}

// The heal/save shrine is now an LDtk-authored `ShrineSpawn` entity (spawned at
// room load via `spawn_room_feature_entities`); the old debug spawner is retired.

/// `Interact` while overlapping a [`HealShrine`] heals the player to full
/// (health + mana) and writes a save checkpoint. `interact_pressed` is an edge,
/// so one press = one heal.
pub fn heal_save_shrine_system(
    control: Res<ControlFrame>,
    mut players: Query<
        (&BodyKinematics, &mut PlayerHealth, &mut PlayerMana),
        (With<PlayerEntity>, With<PrimaryPlayer>),
    >,
    shrines: Query<&HealShrine>,
    mut save: ResMut<crate::persistence::save::SandboxSave>,
    mut activation: ResMut<ShrineActivationPulse>,
    mut sfx: MessageWriter<crate::audio::SfxMessage>,
) {
    if !control.interact_pressed {
        return;
    }
    let Ok((kin, mut health, mut mana)) = players.single_mut() else {
        return;
    };
    let player_aabb = ae::Aabb::new(kin.pos, kin.size * 0.5);
    let touching = shrines
        .iter()
        .any(|s| player_aabb.strict_intersects(ae::Aabb::new(s.pos, s.half_extent)));
    if !touching {
        return;
    }
    health.reset(); // health to full
    mana.meter.refill_full(); // mana to full
                              // Save checkpoint: mark the live save changed so `autosave_sandbox_save`
                              // persists the current state to disk.
    save.set_changed();
    activation.remaining = 0.78;
    sfx.write(crate::audio::SfxMessage::Play {
        id: ambition_sfx::ids::WORLD_HEALTH_COLLECT,
        pos: kin.pos,
    });
    bevy::log::info!(target: "ambition::shrine", "shrine: healed to full + saved");
}

// ---------------------------------------------------------------------------
// Presentation (visible build only).

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

#[derive(Resource, Default)]
pub struct ShrineActivationPulse {
    pub remaining: f32,
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
    world: Res<crate::GameWorld>,
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
        let translation = crate::config::world_to_bevy(&world.0, shrine.pos, 8.0);

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
                crate::presentation::rendering::RoomVisual,
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
            commands.entity(entity).despawn();
        }
    }
}

pub fn animate_shrine_visuals(
    world_time: Res<crate::WorldTime>,
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
    let layout = atlas_layouts.add(shrine_atlas_layout(record));
    let image = asset_server.load("sprites/shrine_spritesheet.png");
    ShrineVisualSource::Atlas {
        image,
        layout,
        idle_start: shrine_row_start_index(record, "idle").unwrap_or(0),
        idle_frame_count: shrine_row_frame_count(record, "idle").unwrap_or(1),
        idle_duration: shrine_row_duration(record, "idle").unwrap_or(0.15),
        activate_start: shrine_row_start_index(record, "activate").unwrap_or(0),
        activate_frame_count: shrine_row_frame_count(record, "activate").unwrap_or(1),
        activate_duration: shrine_row_duration(record, "activate").unwrap_or(0.09),
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

fn shrine_atlas_layout(record: &SheetRecord) -> TextureAtlasLayout {
    let inset = 1u32;
    let mut textures = Vec::new();
    let mut total_w = 1u32;
    let mut total_h = 1u32;

    for row in &record.rows {
        for rect in row.rects.iter().take(row.frame_count as usize) {
            let rect = frame_rect_to_urect(rect).expect("shrine frame rect must be non-negative");
            let rect = inset_rect(rect, inset);
            total_w = total_w.max(rect.max.x);
            total_h = total_h.max(rect.max.y);
            textures.push(rect);
        }
    }

    let mut layout = TextureAtlasLayout::new_empty(UVec2::new(total_w, total_h));
    for rect in textures {
        layout.add_texture(rect);
    }
    layout
}

fn frame_rect_to_urect(rect: &ambition_sprite_sheet::FrameRect) -> Option<bevy::math::URect> {
    let x = u32::try_from(rect.x).ok()?;
    let y = u32::try_from(rect.y).ok()?;
    let w = u32::try_from(rect.w).ok()?;
    let h = u32::try_from(rect.h).ok()?;
    Some(bevy::math::URect {
        min: UVec2::new(x, y),
        max: UVec2::new(x + w, y + h),
    })
}

fn inset_rect(rect: bevy::math::URect, inset: u32) -> bevy::math::URect {
    let inset = inset.min(rect.width().min(rect.height()) / 4);
    bevy::math::URect {
        min: UVec2::new(rect.min.x + inset, rect.min.y + inset),
        max: UVec2::new(rect.max.x - inset, rect.max.y - inset),
    }
}

fn shrine_row_start_index(record: &SheetRecord, animation: &str) -> Option<usize> {
    let mut flat = 0usize;
    for row in &record.rows {
        if row.animation == animation {
            return Some(flat);
        }
        flat += row.frame_count as usize;
    }
    None
}

fn shrine_row_frame_count(record: &SheetRecord, animation: &str) -> Option<usize> {
    record
        .rows
        .iter()
        .find(|row| row.animation == animation)
        .map(|row| row.frame_count as usize)
}

fn shrine_row_duration(record: &SheetRecord, animation: &str) -> Option<f32> {
    record
        .rows
        .iter()
        .find(|row| row.animation == animation)
        .map(|row| row.duration_secs)
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
    use crate::player::PlayerBaseSize;

    #[test]
    fn interacting_at_the_shrine_heals_to_full() {
        let mut app = App::new();
        app.add_message::<crate::audio::SfxMessage>();
        app.insert_resource(ControlFrame::default());
        app.init_resource::<crate::persistence::save::SandboxSave>();
        app.init_resource::<ShrineActivationPulse>();
        app.add_systems(Update, heal_save_shrine_system);

        let player = app
            .world_mut()
            .spawn((
                PlayerEntity,
                PrimaryPlayer,
                BodyKinematics {
                    pos: Vec2::new(100.0, 100.0),
                    vel: Vec2::ZERO,
                    size: Vec2::new(24.0, 40.0),
                    facing: 1.0,
                },
                PlayerBaseSize {
                    base_size: Vec2::new(24.0, 40.0),
                },
                PlayerHealth::new(crate::actor::Health {
                    current: 1,
                    max: 5,
                    invulnerable: false,
                }),
                PlayerMana::default(),
            ))
            .id();
        // Drain mana so we can see it refill.
        app.world_mut()
            .get_mut::<PlayerMana>(player)
            .unwrap()
            .meter
            .try_spend(40.0);
        app.world_mut().spawn(HealShrine {
            pos: Vec2::new(100.0, 100.0),
            half_extent: Vec2::new(22.0, 40.0),
        });

        // Interact while overlapping → heal to full.
        app.world_mut()
            .resource_mut::<ControlFrame>()
            .interact_pressed = true;
        app.update();

        let health = *app.world().get::<PlayerHealth>(player).unwrap();
        assert_eq!(health.current(), health.max(), "health should be full");
        let mana = app.world().get::<PlayerMana>(player).unwrap().meter;
        assert!(
            mana.is_full(),
            "mana should be refilled, got {}",
            mana.current
        );
    }

    #[test]
    fn shrine_sheet_exposes_idle_then_activate_rows() {
        let registry = crate::presentation::character_sprites::baked_sheet_registry();
        let record = registry.get("shrine").expect("shrine sheet record");
        assert_eq!(record.rows.len(), 2);
        assert_eq!(shrine_row_start_index(record, "idle"), Some(0));
        assert_eq!(shrine_row_start_index(record, "activate"), Some(6));
        assert_eq!(shrine_row_frame_count(record, "idle"), Some(6));
        assert_eq!(shrine_row_frame_count(record, "activate"), Some(8));
    }

    #[test]
    fn no_heal_without_interact_or_when_not_touching() {
        let mut app = App::new();
        app.add_message::<crate::audio::SfxMessage>();
        app.insert_resource(ControlFrame::default());
        app.init_resource::<crate::persistence::save::SandboxSave>();
        app.init_resource::<ShrineActivationPulse>();
        app.add_systems(Update, heal_save_shrine_system);
        let player = app
            .world_mut()
            .spawn((
                PlayerEntity,
                PrimaryPlayer,
                BodyKinematics {
                    pos: Vec2::new(100.0, 100.0),
                    vel: Vec2::ZERO,
                    size: Vec2::new(24.0, 40.0),
                    facing: 1.0,
                },
                PlayerBaseSize {
                    base_size: Vec2::new(24.0, 40.0),
                },
                PlayerHealth::new(crate::actor::Health {
                    current: 1,
                    max: 5,
                    invulnerable: false,
                }),
                PlayerMana::default(),
            ))
            .id();
        // A shrine far away.
        app.world_mut().spawn(HealShrine {
            pos: Vec2::new(900.0, 900.0),
            half_extent: Vec2::new(22.0, 40.0),
        });

        // Interact pressed but not touching → no heal.
        app.world_mut()
            .resource_mut::<ControlFrame>()
            .interact_pressed = true;
        app.update();
        assert_eq!(
            app.world().get::<PlayerHealth>(player).unwrap().current(),
            1,
            "no heal when not at the shrine"
        );
    }
}
