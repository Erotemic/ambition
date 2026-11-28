//! The dash's afterimages: while a body dashes it leaves copies of the
//! picture it draws, tinted and fading where they were left, so the burst
//! reads as speed and not as a teleport.
//!
//! Each copy keeps the pose and room where the body left it. Read rigged
//! parts from the local pose sample. Its compositor can reuse the atlas cell
//! while the copy is still visible.
//!
//! Presentation only, and keyed to the sim tick like the locomotion dust: a
//! copy is left on a tick the body dashes, so the trail has the same spacing
//! at any refresh rate, and a rollback that replays the tick leaves nothing
//! twice (the tick is remembered).

use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy::sprite::Anchor;
use bevy::transform::helper::TransformHelper;

use ambition_platformer2d_core::BodyMotionFacts;
use ambition_platformer2d_shared_tangle::lifecycle::{
    ActiveSessionScope, AnimatedWorldSpriteTint, LiveRooms, SessionSpawnScope, SpawnSessionScopedExt,
};
use ambition_sprite_sheet::character::NO_BAKED_IMAGE;
use ambition_time::SimTick;

use super::actors::rigged::{
    part_color, part_local_transform, root_shows, RiggedLocalSample, RiggedPresentation,
    RiggedPresentedBy,
};

/// The afterimage's tint: the dash's purple (the concept sheet's), multiplied
/// into the copy, so the white shell goes lavender and the outlines stay dark.
const AFTERIMAGE_TINT: (f32, f32, f32) = (0.74, 0.5, 1.0);
/// How opaque a copy is when it is left.
const AFTERIMAGE_ALPHA: f32 = 0.55;
/// How long a copy takes to fade out (seconds).
const AFTERIMAGE_LIFE: f32 = 0.22;
/// Sim ticks between two copies of one body.
const AFTERIMAGE_EVERY: u64 = 2;
/// The width and separation of the part stack behind the body.
const AFTERIMAGE_DEPTH: f32 = 0.01;

/// One copy of a dashing body's picture, fading.
#[derive(Component, Debug)]
#[require(AnimatedWorldSpriteTint)]
pub struct DashAfterimage {
    age: f32,
    alpha: f32,
}

/// The ticks each body last left a copy on.
#[derive(Default)]
pub struct AfterimageMemory {
    tick: Option<u64>,
    left: std::collections::HashMap<Entity, u64>,
}

/// Leave a copy of each dashing body's picture.
pub fn leave_dash_afterimages(
    tick: Res<SimTick>,
    mut memory: Local<AfterimageMemory>,
    mut commands: Commands,
    bodies: Query<(
        Entity, &BodyMotionFacts, &Sprite, Option<&Anchor>, Option<&RiggedPresentedBy>,
        Option<&Visibility>, Option<&InheritedVisibility>, Has<ChildOf>, Option<&RenderLayers>,
    )>,
    owners: Query<(&RiggedPresentation, &RiggedLocalSample)>,
    transforms: TransformHelper,
    rooms: LiveRooms,
    session: Option<Res<ActiveSessionScope>>,
) {
    if memory.tick == Some(tick.0) {
        return;
    }
    memory.tick = Some(tick.0);
    let now = tick.0;
    memory.left.retain(|_, at| now.saturating_sub(*at) < AFTERIMAGE_EVERY * 8);
    let Some(scope) = SessionSpawnScope::for_optional_active_session(session.as_deref()) else {
        return;
    };
    for (entity, facts, sprite, anchor, presented, visibility, inherited, parented, layers) in &bodies {
        if !facts.dashing || !root_shows((visibility, inherited, parented)) {
            continue;
        }
        if memory.left.get(&entity).is_some_and(|at| now.saturating_sub(*at) < AFTERIMAGE_EVERY) {
            continue;
        }
        let Ok(root) = transforms.compute_global_transform(entity) else { continue };
        let scope = scope.in_room(rooms.of(entity));
        let rig = presented.and_then(|by| owners.get(by.owner()).ok());
        let copied = if let Some((rig, sample)) = rig {
            if let (Some(parts), Some(feet)) = (sample.parts(), sample.feet) {
                let place = root.mul_transform(feet);
                let start = root.translation().z - 2.0 * AFTERIMAGE_DEPTH;
                let step = AFTERIMAGE_DEPTH / parts.len().max(1) as f32;
                let body_color = sample.direct_body_color();
                let mut copied = false;
                for (index, draw) in parts.iter().enumerate() {
                    let Some(part) = rig.part_image(draw) else { continue };
                    let sprite = Sprite {
                        image: part.page.clone(),
                        rect: Some(part.rect),
                        custom_size: Some(part.size),
                        color: part_color(draw, body_color),
                        ..default()
                    };
                    let mut pose = place.mul_transform(part_local_transform(draw, 0.0)).compute_transform();
                    pose.translation.z = start + step * index as f32;
                    copied |= leave_copy(&mut commands, scope, layers, sprite, Anchor(part.anchor), pose);
                }
                copied
            } else {
                sample.baked.is_some() && leave_whole_copy(&mut commands, scope, layers, sprite, anchor, root)
            }
        } else {
            leave_whole_copy(&mut commands, scope, layers, sprite, anchor, root)
        };
        if copied {
            memory.left.insert(entity, now);
        }
    }
}

fn leave_whole_copy(
    commands: &mut Commands,
    scope: SessionSpawnScope,
    layers: Option<&RenderLayers>,
    sprite: &Sprite,
    anchor: Option<&Anchor>,
    root: GlobalTransform,
) -> bool {
    let mut transform = root.compute_transform();
    transform.translation.z -= AFTERIMAGE_DEPTH;
    leave_copy(commands, scope, layers, sprite.clone(), anchor.copied().unwrap_or_default(), transform)
}

/// Spawn a fading copy of one visible sprite. False when it is not drawn.
fn leave_copy(
    commands: &mut Commands,
    scope: SessionSpawnScope,
    layers: Option<&RenderLayers>,
    mut sprite: Sprite,
    anchor: Anchor,
    transform: Transform,
) -> bool {
    if sprite.image == Handle::default() || sprite.image == NO_BAKED_IMAGE || sprite.color.alpha() <= 0.0 {
        return false;
    }
    let (r, g, b) = AFTERIMAGE_TINT;
    let tint = Color::srgba(r, g, b, AFTERIMAGE_ALPHA).to_linear();
    let ink = sprite.color.to_linear();
    let alpha = tint.alpha * ink.alpha;
    sprite.color = Color::LinearRgba(LinearRgba::new(
        tint.red * ink.red, tint.green * ink.green, tint.blue * ink.blue, alpha,
    ));
    let mut spawned = commands.spawn_session_scoped(scope, (
        Name::new("dash afterimage"), sprite, anchor, transform, DashAfterimage { age: 0.0, alpha },
    ));
    if let Some(layers) = layers {
        spawned.insert(layers.clone());
    }
    true
}

/// Fade the copies out, and take them away once they have.
pub fn fade_dash_afterimages(
    time: Res<Time>,
    mut commands: Commands,
    mut ghosts: Query<(Entity, &mut DashAfterimage, &mut Sprite)>,
) {
    let dt = time.delta_secs();
    for (entity, mut ghost, mut sprite) in &mut ghosts {
        ghost.age += dt;
        let left = 1.0 - ghost.age / AFTERIMAGE_LIFE;
        if left <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }
        sprite.color.set_alpha(ghost.alpha * left * left);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d_shared_tangle::lifecycle::{InRoomInstance, LiveRoomInstance};
    use std::time::Duration;

    #[test]
    fn an_afterimage_keeps_its_pose_and_room_when_the_body_moves() {
        let mut app = App::new();
        app.insert_resource(SimTick(1))
            .init_resource::<Time>()
            .add_systems(Update, (fade_dash_afterimages, leave_dash_afterimages).chain());
        let room = LiveRoomInstance::ACTIVATION;
        let image: Handle<Image> = bevy::asset::uuid_handle!("d1fbdea0-48a8-460e-bb77-2799d16a538b");
        let at = Vec3::new(70.0, 51.0, 20.25);
        let anchor = Anchor(Vec2::new(-0.2, 0.35));
        let body = app.world_mut().spawn((
            BodyMotionFacts { dashing: true, ..default() },
            InRoomInstance(room),
            Sprite {
                image: image.clone(),
                flip_x: true,
                color: Color::srgba(1.0, 1.0, 1.0, 0.5),
                ..default()
            },
            anchor,
            Transform::from_translation(at),
        )).id();

        app.update();
        app.world_mut().entity_mut(body).insert((
            BodyMotionFacts::default(),
            InRoomInstance(room.next()),
            Sprite::default(),
            Transform::from_xyz(300.0, 250.0, at.z),
        ));
        app.world_mut().insert_resource(SimTick(2));
        app.update();

        let world = app.world_mut();
        let mut copies = world.query_filtered::<(&Sprite, &Anchor, &Transform, &InRoomInstance), With<DashAfterimage>>();
        let (sprite, kept_anchor, place, kept_room) = copies.single(world).unwrap();
        assert_eq!(kept_room.0, room);
        assert_eq!(place.translation, at - Vec3::Z * AFTERIMAGE_DEPTH);
        assert_eq!(kept_anchor.0, anchor.0);
        assert_eq!(sprite.image, image);
        assert!(sprite.flip_x);
        assert_eq!(sprite.color.alpha(), 0.5 * AFTERIMAGE_ALPHA);

        world.resource_mut::<Time>().advance_by(Duration::from_secs_f32(AFTERIMAGE_LIFE + 0.01));
        app.update();
        assert_eq!(copies.iter(app.world()).count(), 0);
    }
}
