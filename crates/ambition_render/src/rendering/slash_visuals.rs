//! Player melee slash effect — the `robot_slash` spritesheet hooked up as a
//! one-shot VFX.
//!
//! A sheet-driven effect, so it lives next to [`super::shrine_visuals`] and
//! shares [`super::sheet_atlas`] for the record→atlas plumbing (rather than
//! the character catalog, which requires an Idle row the effect sheet doesn't
//! have). [`fx::vfx_spawn_messages`](crate::fx) dispatches `VfxMessage::Slash`
//! to [`spawn_slash`]; [`animate_slash`] steps the row once and despawns.
//!
//! Directional rows map from the attacker's `AttackIntent` via [`SlashDir`]:
//! `Side`/`Up` are energy-arc crescents (the default + anti-air swings),
//! `Down` is a tapered lance/poke (down-tilt / pogo). One sheet, three rows —
//! a starting point before each attack gets a bespoke effect.

use ambition_sprite_sheet::SheetRegistry;
use bevy::image::{TextureAtlas, TextureAtlasLayout};
use bevy::math::Vec2 as BVec2;
use bevy::prelude::*;

use ambition_gameplay_core::config::{world_to_bevy, WORLD_Z_FX};
use ambition_gameplay_core::engine_core as ae;
use ambition_vfx::vfx::{SlashDir, VfxMessage};

use super::sheet_atlas::{atlas_layout_from_record, row_duration, row_frame_count, row_start_index};

/// The `robot_slash` sheet name in the baked [`SheetRegistry`].
const SLASH_SHEET: &str = "robot_slash";

/// One directional row of the slash sheet, flattened into atlas indices.
#[derive(Clone, Copy, Debug)]
struct SlashRow {
    start: usize,
    frames: usize,
    frame_duration: f32,
}

/// Loaded-once handles + per-row indexing for the slash sheet. Cached in a
/// `Local` so the layout asset is built a single time, mirroring
/// `shrine_visuals`' source cache.
#[derive(Clone)]
pub(crate) struct SlashSource {
    image: Handle<Image>,
    layout: Handle<TextureAtlasLayout>,
    side: SlashRow,
    up: SlashRow,
    down: SlashRow,
}

impl SlashSource {
    fn row(&self, dir: SlashDir) -> SlashRow {
        match dir {
            SlashDir::Side => self.side,
            SlashDir::Up => self.up,
            SlashDir::Down => self.down,
        }
    }
}

/// A live slash effect: plays its row once over `frames * frame_duration`,
/// then despawns.
#[derive(Component)]
pub(crate) struct SlashVisual {
    age: f32,
    row_start: usize,
    frames: usize,
    frame_duration: f32,
}

fn slash_source(
    asset_server: &AssetServer,
    registry: Option<&SheetRegistry>,
    atlas_layouts: &mut Assets<TextureAtlasLayout>,
    cache: &mut Option<SlashSource>,
) -> Option<SlashSource> {
    if let Some(source) = cache.as_ref() {
        return Some(source.clone());
    }
    let record = registry?.get(SLASH_SHEET)?;
    let layout = atlas_layouts.add(atlas_layout_from_record(record));
    let row = |name: &str| SlashRow {
        start: row_start_index(record, name).unwrap_or(0),
        frames: row_frame_count(record, name).unwrap_or(1).max(1),
        frame_duration: row_duration(record, name).unwrap_or(0.05).max(0.001),
    };
    let source = SlashSource {
        image: asset_server.load(format!("sprites/{SLASH_SHEET}_spritesheet.png")),
        layout,
        side: row("side"),
        up: row("up"),
        down: row("down"),
    };
    *cache = Some(source.clone());
    Some(source)
}

/// Consume `VfxMessage::Slash` cues and spawn the matching one-shot slash
/// effect. Self-contained (its own message cursor + source cache), registered
/// in `rendering::mod`; the particle dispatcher (`fx::vfx_spawn_messages`)
/// no-ops the variant. No-op when the sheet isn't loadable (headless /
/// no-asset profiles), and the source is built lazily on the first cue.
pub(crate) fn spawn_slash_effects(
    mut commands: Commands,
    mut messages: MessageReader<VfxMessage>,
    world: Res<ambition_gameplay_core::GameWorld>,
    asset_server: Res<AssetServer>,
    mut atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    sheet_registry: Option<Res<SheetRegistry>>,
    mut cache: Local<Option<SlashSource>>,
) {
    let mut source: Option<SlashSource> = None;
    for message in messages.read() {
        let VfxMessage::Slash {
            center,
            size,
            dir,
            facing,
        } = message
        else {
            continue;
        };
        if source.is_none() {
            source = slash_source(
                &asset_server,
                sheet_registry.as_deref(),
                &mut atlas_layouts,
                &mut cache,
            );
        }
        let Some(source) = source.as_ref() else {
            continue;
        };
        spawn_one(&mut commands, &world.0, source, *center, *size, *dir, *facing);
    }
}

/// Spawn a one-shot slash effect at `center`, `size` px square, flipped by
/// `facing`, playing the `dir` row.
fn spawn_one(
    commands: &mut Commands,
    world: &ae::World,
    source: &SlashSource,
    center: ae::Vec2,
    size: f32,
    dir: SlashDir,
    facing: f32,
) {
    let row = source.row(dir);
    let mut sprite = Sprite::from_atlas_image(
        source.image.clone(),
        TextureAtlas {
            layout: source.layout.clone(),
            index: row.start,
        },
    );
    sprite.custom_size = Some(BVec2::splat(size.max(1.0)));
    // The `side` arc is drawn opening toward +x; mirror it for a left swing.
    sprite.flip_x = facing < 0.0;
    commands.spawn((
        Name::new("VFX slash"),
        sprite,
        Transform::from_translation(world_to_bevy(world, center, WORLD_Z_FX + 2.0)),
        SlashVisual {
            age: 0.0,
            row_start: row.start,
            frames: row.frames,
            frame_duration: row.frame_duration,
        },
    ));
}

/// Advance every live slash effect one frame at a time and despawn it once the
/// row finishes. Uses scaled time so the swing reads in bullet-time/pause,
/// matching `animate_shrine_visuals`.
pub(crate) fn animate_slash(
    mut commands: Commands,
    world_time: Res<ambition_gameplay_core::WorldTime>,
    mut query: Query<(Entity, &mut SlashVisual, &mut Sprite)>,
) {
    let dt = world_time.scaled_dt;
    for (entity, mut slash, mut sprite) in &mut query {
        slash.age += dt;
        let frame = (slash.age / slash.frame_duration) as usize;
        if frame >= slash.frames {
            commands.entity(entity).despawn();
            continue;
        }
        if let Some(atlas) = sprite.texture_atlas.as_mut() {
            atlas.index = slash.row_start + frame;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn robot_slash_sheet_is_baked_with_directional_rows() {
        // Proves the effect is actually hooked up: the sheet is in the baked
        // registry and exposes the side/up/down rows the attack maps onto.
        let registry = ambition_gameplay_core::character_sprites::baked_sheet_registry();
        let record = registry
            .get(SLASH_SHEET)
            .expect("robot_slash sheet must be baked into the registry");
        assert_eq!(row_start_index(record, "side"), Some(0));
        assert_eq!(row_start_index(record, "up"), Some(5));
        assert_eq!(row_start_index(record, "down"), Some(10));
        for row in ["side", "up", "down"] {
            assert_eq!(row_frame_count(record, row), Some(5), "{row} frames");
        }
    }
}
