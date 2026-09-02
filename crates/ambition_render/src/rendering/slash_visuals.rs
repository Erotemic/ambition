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
/// THIS WAS A `const`. One sheet, four rows, every body in the game — so the
/// protagonist's blade was the engine's blade, and a boss swung the robot's
/// crescent. It got worse once the art was shaped to a specific character's hit
/// polygon: anyone else drawing it wears a silhouette cut for someone else's
/// volume. A character names its own sheet now (`CharacterCatalogEntry::attack_vfx`),
/// several may name the same one, and naming none is a real answer with its own
/// treatment rather than a default inheritance.
#[derive(Resource, Default)]
pub(crate) struct SlashSources(HashMap<String, Option<SlashSource>>);

/// Loaded-once handles + per-pose indexing for the slash sheet. `side` is the
/// forward crescent, `up` the overhead anti-air row, and `down` the downward
/// cleave / poke. The runtime still rotates the chosen row to track the real
/// resolved strike under arbitrary gravity.
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

/// Z-rotation (Bevy radians) to point a slash art along the world direction
/// `dir` (the attacker→hitbox vector, already gravity-relative). World y is
/// down and Bevy y is up (`world_to_bevy` inverts y), so the target Bevy angle
/// is `atan2(-dir.y, dir.x)`. The `arc` art opens toward +x at rest; the
/// `up` art points toward world up at rest; `down` / poke art points toward
/// world down at rest. Pure + frame-agnostic: feeding the four C4 gravity
/// directions yields the four correctly-rotated effects.
/// Where to point the art: along the swing, and nothing else.
///
/// It stopped being coherent when the quad became the swing's own extent.
///
/// `pose` now selects WHICH artwork, never how it is turned. The rows are
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
    /// Who is swinging, and where the swing sits in THEIR frame.
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
    // All three rows through one ledger, so a regenerated sheet that renamed any
    // of them is reported together rather than one silent `unwrap_or(0)` each.
    let mut ledger = BindingLedger::new();
    let mut row = |name: &str| {
        row_playback(record, name, "slash visual", &mut ledger).unwrap_or(RowPlayback {
            // The effect still draws (blind runs never go black); the report is
            // what stops the wrong art from being silent.
            start: 0,
            frames: 1,
            frame_duration: 0.05,
        })
    };
    let source = SlashSource {
        // `fx-sheet`, the label the catalog's own effect loads already use — a
        // slash arc is an effect sheet. A bare `load` here left it in
        // `Assets<Image>` with no demand, so the ledger read `demand=unknown`.
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

/// Consume `VfxMessage::Slash` cues and spawn the matching one-shot slash
/// effect. Self-contained (its own message cursor + source cache), registered
/// in `rendering::mod`; the particle dispatcher (`fx::vfx_spawn_messages`)
/// no-ops the variant. No-op when the sheet isn't loadable (headless /
/// no-asset profiles), and the source is built lazily on the first cue.
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
    // the READ-MODEL pose, not the sim's `BodyKinematics` — presentation reads
    // `ambition_sim_view` (E4), and naming the live cluster here is what turned
    // `engine.render-never-names-live-sim-state` red.
    //
    // It was not: that view is rebuilt `With<PlayerVisual>`, so no boss and no actor ever matched
    // and every one of their slashes took the miss arm. `PresentedPose` follows `BodyKinematics`
    // and answers for every body, which is all this needs — the drawn position of the swinging
    // body.
    owners: Query<&PresentedPose>,
    // Which sheet each swinging body's character authors — the READ-MODEL fact,
    // resolved sim-side by `rebuild_attack_vfx_views`.
    //
    // The view cannot make that mistake — an unresolved body has NO component, which is not the
    // same as one whose `sheet` resolved to `None`.
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
        // A character either names its sheet or gets no sprite at all.
        // Falling back to somebody else's art is what this whole change exists
        // to stop; the unauthored-volume pass makes the silence visible.
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
/// region, sized to the swing's own length and width, and turned to the swing
/// axis.
///
/// The quad is NOT square. The art stretches to the swing now, which is only fully honest once the
/// art itself is generated from the same swing descriptor the hit polygon is.
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
    // `x` runs along the swing axis, `y` across it — the frame the rotation
    // below puts the sprite into. A radial swing has no axis; its extent is
    // already world-aligned and its rotation is the pose's alone.
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

/// Where the owner is being DRAWN this frame.
///
/// The presented pose, not the sim pose. They differ by up to a frame of interpolation, and the
/// body sprite is drawn from the presented one — so a blade placed on the sim pose shudders against
/// a body that looks perfectly stable. Where the swinging body is drawn, or `None` if it cannot be
/// found.
///
/// an absent owner is now an absent slash — nothing drawn, one warning naming the entity.
///
/// The fix stands on its own: a fallback that invents an answer is wrong whether or not it is
/// currently firing.
fn owner_pos(owners: &Query<&PresentedPose>, owner: Entity) -> Option<ae::Vec2> {
    owners
        .get(owner)
        .ok()
        .map(|presented| presented.presented())
}

/// Keep every live slash on the body that is swinging it.
///
/// The hitbox is `HitboxAnchor::FollowOwner` and re-resolves from the owner
/// every tick; this is the presentation half of the same rule. Without it the
/// damage box tracks a running attacker and the drawn blade does not, for the
/// whole 100ms the swing is live.
///
/// Only the TRANSLATION follows. The swing's direction and extent were committed
/// in the body's frame when the strike opened — the hitbox stores its own
/// `facing` and `frame_down` the same way and does not re-mirror mid-swing — so
/// re-deriving them here would be a second opinion, not an update.
///
/// An owner that has despawned mid-swing leaves its effect where it last stood
/// rather than snapping it to the origin. A body can die inside its own swing,
/// and the alternative reads as a rendering fault.
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
        // Proves the effect is actually hooked up: the sheet is in the baked
        // registry and exposes the arc (side) + poke (down) rows the attack
        // maps onto.
        // The id is the one the protagonist NAMES in the character catalog, not
        // an engine constant any more: a body with no `attack_vfx` draws its hit
        // volume rather than borrowing this sheet.
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

    /// The slash effect must orient in the attacker's reference frame: under
    /// each of the C4 symmetry-room gravities, the same attack's world
    /// `dir` (player→hitbox) rotates the art to point at the strike. Feeding
    /// the four cardinal directions (what the four gravities produce for a
    /// given local attack) must yield four distinct, correct rotations.
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
        // Restore either offset and this fails.
        for pose in [SlashPose::Side, SlashPose::Up, SlashPose::Down] {
            assert!(
                approx(slash_rotation(Vec2::new(0.0, -1.0), pose), FRAC_PI_2),
                "an upward strike points up whatever row it draws"
            );
        }
    }
}
