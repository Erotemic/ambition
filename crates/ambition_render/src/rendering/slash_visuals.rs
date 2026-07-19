//! Player melee slash effect — the `robot_slash` spritesheet hooked up as a
//! one-shot VFX.
//!
//! A sheet-driven effect, so it lives next to [`super::shrine_visuals`] and
//! shares [`super::sheet_atlas`] for the record→atlas plumbing (rather than
//! the character catalog, which requires an Idle row the effect sheet doesn't
//! have). [`fx::vfx_spawn_messages`](crate::fx) dispatches `VfxMessage::Slash`
//! to [`spawn_slash`]; [`animate_slash`] steps the row once and despawns.
//!
//! The combat layer now tags each slash cue with the authored attack pose, so
//! presentation can pick the matching `side` / `up` / `down` row instead of
//! rotating one generic arc for every attack. One sheet, three rows.

use ambition_sprite_sheet::SheetRegistry;
use bevy::image::{TextureAtlas, TextureAtlasLayout};
use bevy::math::Vec2 as BVec2;
use bevy::prelude::*;

use ambition_engine_core as ae;
use ambition_engine_core::config::{world_to_bevy, WORLD_Z_FX};
use ambition_platformer_primitives::lifecycle::{
    ActiveSessionScope, SessionSpawnScope, SpawnSessionScopedExt,
};
use ambition_vfx::vfx::{SlashKind, SlashPose, VfxMessage};

use super::sheet_atlas::{
    atlas_layout_from_record, row_duration, row_frame_count, row_start_index,
};

/// The `robot_slash` sheet name in the baked [`SheetRegistry`].
const SLASH_SHEET: &str = "robot_slash";

/// One row of the slash sheet, flattened into atlas indices.
#[derive(Clone, Copy, Debug)]
struct SlashRow {
    start: usize,
    frames: usize,
    frame_duration: f32,
}

/// Loaded-once handles + per-pose indexing for the slash sheet. `side` is the
/// forward crescent, `up` the overhead anti-air row, and `down` the downward
/// cleave / poke. The runtime still rotates the chosen row to track the real
/// resolved strike under arbitrary gravity.
#[derive(Clone)]
pub(crate) struct SlashSource {
    image: Handle<Image>,
    layout: Handle<TextureAtlasLayout>,
    side_arc: SlashRow,
    up_arc: SlashRow,
    down_slash: SlashRow,
}

impl SlashSource {
    fn row(&self, kind: SlashKind, pose: SlashPose) -> SlashRow {
        match pose {
            SlashPose::Up if kind == SlashKind::Arc => self.up_arc,
            SlashPose::Down => self.down_slash,
            _ if kind == SlashKind::Poke => self.down_slash,
            _ => self.side_arc,
        }
    }
}

/// Z-rotation (Bevy radians) to point a slash art along the world direction
/// `dir` (the attacker→hitbox vector, already gravity-relative). World y is
/// down and Bevy y is up (`world_to_bevy` inverts y), so the target Bevy angle
/// is `atan2(-dir.y, dir.x)`. The `arc` art opens toward +x at rest; the
/// `up` art points toward world up at rest; `down` / poke art points toward
/// world down at rest. Pure + frame-agnostic: feeding the four C4 gravity
/// directions yields the four correctly-rotated effects.
pub(crate) fn slash_rotation(dir: ae::Vec2, pose: SlashPose) -> f32 {
    let base = if dir.length_squared() > 1e-6 {
        (-dir.y).atan2(dir.x)
    } else {
        0.0
    };
    match pose {
        SlashPose::Side => base,
        SlashPose::Up => base - std::f32::consts::FRAC_PI_2,
        SlashPose::Down => base + std::f32::consts::FRAC_PI_2,
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
        side_arc: row("side"),
        up_arc: row("up"),
        down_slash: row("down"),
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
    world: ambition_platformer_primitives::lifecycle::SessionWorldRef<
        ambition_engine_core::RoomGeometry,
    >,
    asset_server: Res<AssetServer>,
    mut atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    sheet_registry: Option<Res<SheetRegistry>>,
    active_session: Option<Res<ActiveSessionScope>>,
    mut cache: Local<Option<SlashSource>>,
) {
    let Some(session_scope) =
        SessionSpawnScope::for_optional_active_session(active_session.as_deref())
    else {
        messages.clear();
        return;
    };
    let mut source: Option<SlashSource> = None;
    for message in messages.read() {
        let VfxMessage::Slash {
            center,
            size,
            kind,
            pose,
            dir,
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
        spawn_one(
            &mut commands,
            session_scope,
            &world.0,
            source,
            *center,
            *size,
            *kind,
            *pose,
            *dir,
        );
    }
}

/// Spawn a one-shot slash effect at `center`, `size` px square, playing `kind`
/// rotated to point along the world `dir` (attacker→hitbox, gravity-relative).
fn spawn_one(
    commands: &mut Commands,
    session_scope: SessionSpawnScope,
    world: &ae::World,
    source: &SlashSource,
    center: ae::Vec2,
    size: f32,
    kind: SlashKind,
    pose: SlashPose,
    dir: ae::Vec2,
) {
    let row = source.row(kind, pose);
    let mut sprite = Sprite::from_atlas_image(
        source.image.clone(),
        TextureAtlas {
            layout: source.layout.clone(),
            index: row.start,
        },
    );
    sprite.custom_size = Some(BVec2::splat(size.max(1.0)));
    let mut transform = Transform::from_translation(world_to_bevy(world, center, WORLD_Z_FX + 2.0));
    transform.rotation = Quat::from_rotation_z(slash_rotation(dir, pose));
    commands.spawn_session_scoped(
        session_scope,
        (
            Name::new("VFX slash"),
            sprite,
            transform,
            SlashVisual {
                age: 0.0,
                row_start: row.start,
                frames: row.frames,
                frame_duration: row.frame_duration,
            },
        ),
    );
}

/// Advance every live slash effect one frame at a time and despawn it once the
/// row finishes. Uses the render-frame presentation clock (scaled, so the swing
/// reads in bullet-time/pause) — NOT `WorldTime`, whose `scaled_dt` is the fixed
/// sim tick under the GGRS host and would tie animation speed to display refresh
/// rate (see 0693e5e88). Matches `animate_shrine_visuals`.
pub(crate) fn animate_slash(
    mut commands: Commands,
    presentation_time: ambition_time::PresentationTime,
    mut query: Query<(Entity, &mut SlashVisual, &mut Sprite)>,
) {
    let dt = presentation_time.scaled_dt();
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
        // registry and exposes the arc (side) + poke (down) rows the attack
        // maps onto.
        let registry = ambition_sprite_sheet::baked_sheet_registry();
        let record = registry
            .get(SLASH_SHEET)
            .expect("robot_slash sheet must be baked into the registry");
        // 5 frames/row: side=0..4, up=5..9, down=10..14.
        assert_eq!(row_start_index(record, "side"), Some(0));
        assert_eq!(row_start_index(record, "up"), Some(5));
        assert_eq!(row_start_index(record, "down"), Some(10));
        for row in ["side", "up", "down"] {
            assert_eq!(row_frame_count(record, row), Some(5), "{row} frames");
        }
    }

    /// The slash effect must orient in the attacker's reference frame: under
    /// each of the C4 symmetry-room gravities, the same attack's world
    /// `dir` (player→hitbox) rotates the art to point at the strike. Feeding
    /// the four cardinal directions (what the four gravities produce for a
    /// given local attack) must yield four distinct, correct rotations.
    #[test]
    fn slash_rotation_follows_the_strike_direction_under_c4() {
        use ae::Vec2;
        use std::f32::consts::{FRAC_PI_2, PI};
        let approx = |a: f32, b: f32| {
            let d = (a - b).rem_euclid(2.0 * PI);
            d < 1e-3 || (2.0 * PI - d) < 1e-3
        };
        // Side art opens +x at rest.
        assert!(approx(
            slash_rotation(Vec2::new(1.0, 0.0), SlashPose::Side),
            0.0
        ));
        assert!(approx(
            slash_rotation(Vec2::new(0.0, 1.0), SlashPose::Side),
            -FRAC_PI_2
        ));
        assert!(approx(
            slash_rotation(Vec2::new(0.0, -1.0), SlashPose::Side),
            FRAC_PI_2
        ));
        assert!(approx(
            slash_rotation(Vec2::new(-1.0, 0.0), SlashPose::Side),
            PI
        ));
        // Up art points world-up at rest; down art points world-down at rest.
        assert!(approx(
            slash_rotation(Vec2::new(0.0, -1.0), SlashPose::Up),
            0.0
        ));
        assert!(approx(
            slash_rotation(Vec2::new(0.0, 1.0), SlashPose::Down),
            0.0
        ));
        assert!(approx(
            slash_rotation(Vec2::new(1.0, 0.0), SlashPose::Down),
            FRAC_PI_2
        ));
    }
}
