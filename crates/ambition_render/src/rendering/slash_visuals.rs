//! Melee slash effect: a character's authored slash spritesheet as a one-shot
//! VFX.
//!
//! A sheet-driven effect, so it lives beside [`super::shrine_visuals`] and
//! uses [`super::sheet_atlas`] for the record-to-atlas step (the character
//! catalog needs an Idle row that effect sheets do not have).
//! [`fx::vfx_spawn_messages`](crate::fx) no-ops `VfxMessage::Slash`;
//! `spawn_slash_effects` spawns it and [`animate_slash`] plays the row once
//! and despawns.
//!
//! The combat layer tags each slash cue with the authored attack pose, so
//! presentation picks the `side`, `up`, or `down` row. One sheet, three rows.

use ambition_sprite_sheet::SheetRegistry;
use bevy::image::{TextureAtlas, TextureAtlasLayout};
use bevy::math::Vec2 as BVec2;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;

use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::config::{world_to_bevy, WORLD_Z_FX};
use ambition_platformer2d_shared_tangle::lifecycle::{
    ActiveSessionScope, SessionSpawnScope, SpawnSessionScopedExt,
};
use ambition_sim_view::presented_pose::PresentedPose;
use ambition_vfx::vfx::{SlashKind, SlashPose, VfxMessage};

use super::sheet_atlas::{atlas_layout_from_record, row_playback, RowPlayback};
use ambition_platformer2d_shared_tangle::binding::BindingLedger;

/// Sheets already resolved this session, keyed by the id a character named.
///
/// A character names its own sheet (`CharacterCatalogEntry::attack_vfx`),
/// because the art is shaped to that character's hit polygon. Several
/// characters may name the same sheet. Naming none is a valid answer with its
/// own treatment (see `unauthored_volumes`), not a fallback to a shared sheet.
#[derive(Resource, Default)]
pub(crate) struct SlashSources(HashMap<String, Option<SlashSource>>);

/// Loaded-once handles and per-pose row indexing for a slash sheet. `side` is
/// the forward crescent, `up` the overhead anti-air row, and `down` the
/// downward cleave or poke. The chosen row is rotated to follow the resolved
/// strike under any gravity.
#[derive(Clone)]
pub(crate) struct SlashSource {
    image: Handle<Image>,
    layout: Handle<TextureAtlasLayout>,
    side_arc: RowPlayback,
    up_arc: RowPlayback,
    down_slash: RowPlayback,
}

impl SlashSource {
    fn row(&self, kind: SlashKind, pose: SlashPose) -> RowPlayback {
        match pose {
            SlashPose::Up if kind == SlashKind::Arc => self.up_arc,
            SlashPose::Down => self.down_slash,
            _ if kind == SlashKind::Poke => self.down_slash,
            _ => self.side_arc,
        }
    }
}

/// Z rotation (Bevy radians) that points the art along the swing direction
/// `dir` (attacker to hitbox, already gravity-relative). World y is down and
/// Bevy y is up, so the angle is `atan2(-dir.y, dir.x)`.
///
/// `pose` selects which artwork, never how it is turned. The rows are
/// authored in swing space to match (`robot_slash.py`).
pub(crate) fn slash_rotation(dir: ae::Vec2, _pose: SlashPose) -> f32 {
    if dir.length_squared() > 1e-6 {
        (-dir.y).atan2(dir.x)
    } else {
        0.0
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
    /// Who is swinging, and where the swing sits in their frame.
    owner: Entity,
    local: ae::SwingShape,
}

/// Resolve (and remember) the sheet a character named.
fn slash_source(
    sheet: &str,
    asset_server: &AssetServer,
    registry: Option<&SheetRegistry>,
    atlas_layouts: &mut Assets<TextureAtlasLayout>,
    cache: &mut SlashSources,
) -> Option<SlashSource> {
    if let Some(hit) = cache.0.get(sheet) {
        return hit.clone();
    }
    let built = build_slash_source(sheet, asset_server, registry, atlas_layouts);
    if built.is_none() {
        bevy::log::warn!(
            "attack vfx sheet `{sheet}` is named by a character and not in the \
             baked registry; that body will draw its hit volume instead"
        );
    }
    cache.0.insert(sheet.to_string(), built.clone());
    built
}

fn build_slash_source(
    sheet: &str,
    asset_server: &AssetServer,
    registry: Option<&SheetRegistry>,
    atlas_layouts: &mut Assets<TextureAtlasLayout>,
) -> Option<SlashSource> {
    let record = registry?.get(sheet)?;
    let layout = atlas_layouts.add(atlas_layout_from_record(record));
    // Resolve all three rows through one ledger, so renamed rows in a
    // regenerated sheet are reported together.
    let mut ledger = BindingLedger::new();
    let mut row = |name: &str| {
        row_playback(record, name, "slash visual", &mut ledger).unwrap_or(RowPlayback {
            // The effect still draws (blind runs never go black); the report
            // makes the wrong art visible.
            start: 0,
            frames: 1,
            frame_duration: 0.05,
        })
    };
    let source = SlashSource {
        // `fx-sheet`, the label the catalog's effect loads use. A bare `load`
        // would leave the image with no demand (`demand=unknown`).
        image: ambition_sprite_sheet::game_assets::load_sheet_image(
            asset_server,
            "fx-sheet",
            format!("sprites/{sheet}_spritesheet.png"),
        ),
        layout,
        side_arc: row("side"),
        up_arc: row("up"),
        down_slash: row("down"),
    };
    ledger.finish().log("slash visual");
    Some(source)
}

/// Consume `VfxMessage::Slash` cues and spawn the matching one-shot effect.
/// Self-contained (its own message cursor and source cache), registered in
/// `rendering::mod`; `fx::vfx_spawn_messages` no-ops the variant. Does nothing
/// when the sheet cannot load (headless or no-asset profiles). Sources are
/// built lazily on the first cue.
pub(crate) fn spawn_slash_effects(
    mut commands: Commands,
    mut messages: MessageReader<VfxMessage>,
    world: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
        ambition_platformer2d_core::RoomGeometry,
    >,
    asset_server: Res<AssetServer>,
    mut atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    sheet_registry: Option<Res<SheetRegistry>>,
    active_session: Option<Res<ActiveSessionScope>>,
    // The read-model pose, not the sim's `BodyKinematics`: render never names
    // live sim state (`engine.render-never-names-live-sim-state`).
    // `PresentedPose` covers every body (bosses and actors too), and gives
    // the drawn position of the swinging body.
    owners: Query<&PresentedPose>,
    // Which sheet each swinging body's character authors: the read-model
    // fact from `rebuild_attack_vfx_views`. An unresolved body has no
    // component, which differs from one whose `sheet` resolved to `None`.
    attack_vfx: Query<&ambition_sim_view::AttackVfxView>,
    mut cache: ResMut<SlashSources>,
) {
    let Some(session_scope) =
        SessionSpawnScope::for_optional_active_session(active_session.as_deref())
    else {
        messages.clear();
        return;
    };
    for message in messages.read() {
        let VfxMessage::Slash {
            shape,
            owner,
            kind,
            pose,
        } = message
        else {
            continue;
        };
        // A character names its sheet or gets no sprite. The unauthored-volume
        // pass draws the no-sheet case.
        let Some(sheet) = attack_vfx
            .get(*owner)
            .ok()
            .and_then(|view| view.sheet.clone())
        else {
            continue;
        };
        let Some(source) = slash_source(
            &sheet,
            &asset_server,
            sheet_registry.as_deref(),
            &mut atlas_layouts,
            &mut cache,
        ) else {
            continue;
        };
        let Some(at) = owner_pos(&owners, *owner) else {
            bevy::log::warn!(
                target: "ambition_platformer2d::render",
                "a slash cue names {owner:?}, which publishes no `BodyPoseView`; \
                 skipping the effect rather than drawing it at the world origin. \
                 Some spawn path is producing a swing whose owner the pose \
                 read-model does not cover."
            );
            continue;
        };
        spawn_one(
            &mut commands,
            session_scope,
            &world.0,
            &source,
            *shape,
            *owner,
            at,
            *kind,
            *pose,
        );
    }
}

/// Spawn a one-shot slash effect fitted to `shape`: centred on the swept
/// region, sized to the swing's length and width, and turned to the swing
/// axis.
///
/// The quad is not square: the art stretches to the swing. This is fully
/// accurate only when the art is generated from the same swing descriptor as
/// the hit polygon.
fn spawn_one(
    commands: &mut Commands,
    session_scope: SessionSpawnScope,
    world: &ae::World,
    source: &SlashSource,
    shape: ae::SwingShape,
    owner: Entity,
    owner_pos: ae::Vec2,
    kind: SlashKind,
    pose: SlashPose,
) {
    let row = source.row(kind, pose);
    let mut sprite = Sprite::from_atlas_image(
        source.image.clone(),
        TextureAtlas {
            layout: source.layout.clone(),
            index: row.start,
        },
    );
    // `x` runs along the swing axis and `y` across it, the frame the
    // rotation below uses. A radial swing has no axis; its extent is already
    // world-aligned and only the pose rotates it.
    let half = shape.oriented_bounds();
    sprite.custom_size = Some(BVec2::new((half.x * 2.0).max(1.0), (half.y * 2.0).max(1.0)));
    let mut transform = Transform::from_translation(world_to_bevy(
        world,
        owner_pos + shape.center(),
        WORLD_Z_FX + 2.0,
    ));
    let axis = match shape {
        ae::SwingShape::Sweep { dir, .. } => dir,
        ae::SwingShape::Radial { .. } => ae::Vec2::ZERO,
    };
    transform.rotation = Quat::from_rotation_z(slash_rotation(axis, pose));
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
                owner,
                local: shape,
            },
        ),
    );
}

/// Where the swinging body is drawn this frame, or `None` if it is missing.
///
/// The presented pose, not the sim pose. They differ by up to a frame of
/// interpolation, and the body sprite uses the presented one, so a blade on
/// the sim pose would shudder against the body. A missing owner means no
/// slash: nothing is drawn, and one warning names the entity.
fn owner_pos(owners: &Query<&PresentedPose>, owner: Entity) -> Option<ae::Vec2> {
    owners
        .get(owner)
        .ok()
        .map(|presented| presented.presented())
}

/// Keep every live slash on the body that is swinging it.
///
/// The hitbox is `HitboxAnchor::FollowOwner` and re-resolves from the owner
/// every tick; this is the presentation half of the same rule. Without it
/// the drawn blade would not track a running attacker during the ~100 ms
/// swing.
///
/// Only the translation follows. The swing's direction and extent were fixed
/// in the body's frame when the strike opened (the hitbox also stores its own
/// `facing` and `frame_down`), so they are not re-derived here.
///
/// If the owner despawns mid-swing, the effect stays where it last was
/// instead of snapping to the origin. A body can die inside its own swing.
pub(crate) fn follow_slash_owner(
    world: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
        ambition_platformer2d_core::RoomGeometry,
    >,
    owners: Query<&PresentedPose>,
    mut slashes: Query<(&SlashVisual, &mut Transform)>,
) {
    for (slash, mut transform) in &mut slashes {
        let Ok(presented) = owners.get(slash.owner) else {
            continue;
        };
        let pos = presented.presented();
        let target = world_to_bevy(&world.0, pos + slash.local.center(), WORLD_Z_FX + 2.0);
        transform.translation.x = target.x;
        transform.translation.y = target.y;
    }
}

/// Advance every live slash effect one frame at a time and despawn it once the row finishes.
/// Matches `animate_shrine_visuals`.
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
        // The effect is hooked up: the sheet is in the baked registry and has
        // the side and down rows the attack maps onto. The id is the one the
        // protagonist names in the character catalog.
        let registry = ambition_sprite_sheet::baked_sheet_registry();
        let record = registry
            .get("robot_slash")
            .expect("robot_slash sheet must be baked into the registry");
        // 5 frames/row: side=0..4, up=5..9, down=10..14.
        let mut ledger = BindingLedger::new();
        let mut row = |name: &str| {
            row_playback(record, name, "test", &mut ledger).expect("the sheet has this row")
        };
        assert_eq!(row("side").start, 0);
        assert_eq!(row("up").start, 5);
        assert_eq!(row("down").start, 10);
        for name in ["side", "up", "down"] {
            assert_eq!(row(name).frames, 5, "{name} frames");
        }
        assert!(
            ledger.finish().is_empty(),
            "the shipped sheet still spells every row the effect asks for"
        );
    }

    /// The slash effect orients in the attacker's frame. The four cardinal
    /// directions (what the four C4 gravities give for one local attack) must
    /// give four distinct, correct rotations.
    #[test]
    fn slash_rotation_follows_the_strike_direction_and_only_that() {
        use ae::Vec2;
        use std::f32::consts::{FRAC_PI_2, PI};
        let approx = |a: f32, b: f32| {
            let d = (a - b).rem_euclid(2.0 * PI);
            d < 1e-3 || (2.0 * PI - d) < 1e-3
        };
        // Art opens along +x at rest and turns with the swing.
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
        // The pose must not add an offset.
        for pose in [SlashPose::Side, SlashPose::Up, SlashPose::Down] {
            assert!(
                approx(slash_rotation(Vec2::new(0.0, -1.0), pose), FRAC_PI_2),
                "an upward strike points up whatever row it draws"
            );
        }
    }
}
