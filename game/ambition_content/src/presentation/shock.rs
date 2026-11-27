//! The shock effect on a body that takes electric damage.

use std::collections::BTreeSet;

use ambition_characters::actor::{
    character_catalog::{CharacterBodyKind, CharacterCatalog},
    CharacterForm,
};
use ambition_combat::body_landmarks::{BodyLandmarks, LandmarkPose};
use ambition_platformer2d::damage::BodyHitResolved;
use ambition_platformer2d_core::{
    config::{world_to_bevy, WORLD_Z_FX},
    BodyKinematics, RoomGeometry,
};
use ambition_platformer2d_shared_tangle::{
    body::geometry::{PartSource, ReachesFrom},
    lifecycle::{
        LiveRoomOf, LiveRooms, PresentationOf, SessionScopedEntity, SessionSpawnScope,
        SpawnSessionScopedExt,
    },
};
use ambition_render::{
    folded_mesh::{Composite, FoldedMesh},
    rendering::{
        body_visibility::body_presentation_hidden, ActorOverlaySet, AnimatedWorldSpriteTint,
        BodyOwnedDrawableSync,
    },
};
use ambition_sim_view::{BodyPoseView, PresentedPose};
use ambition_sprite_sheet::{game_assets::load_sheet_image, SpriteContent};
use bevy::{prelude::*, time::Real};

mod art;

use art::{arcs, Bone, Skeleton, DURATION};

#[derive(Component)]
struct Shock {
    body: Entity,
    age: f32,
    parts: Vec<Entity>,
}

pub(super) fn install(app: &mut App) {
    app.add_systems(
        Update,
        flash_shocked_bodies
            .in_set(ActorOverlaySet)
            .in_set(BodyOwnedDrawableSync),
    );
}

fn did_shock(hit: &BodyHitResolved) -> bool {
    hit.resolution.damage_applied() > 0 && hit.damage_types.contains("ambition.electric")
}

fn flash_shocked_bodies(
    mut commands: Commands,
    mut hits: MessageReader<BodyHitResolved>,
    time: Res<Time<Real>>,
    rooms: LiveRooms,
    geometry: LiveRoomOf<RoomGeometry>,
    bodies: Query<(
        &BodyKinematics,
        Option<&PresentedPose>,
        Option<&SessionScopedEntity>,
        Option<&CharacterForm>,
        Option<&BodyPoseView>,
    )>,
    landmarks: BodyLandmarks,
    catalog: Option<Res<CharacterCatalog>>,
    assets: Option<Res<AssetServer>>,
    sprite_content: Res<SpriteContent>,
    mut atlas: Local<Option<Handle<Image>>>,
    mut effects: Query<(Entity, &mut Shock, &mut FoldedMesh)>,
) {
    let mut renewed: BTreeSet<Entity> = hits
        .read()
        .filter(|hit| {
            did_shock(hit)
                && rooms.of(hit.body) == hit.identity.room
                && bodies.get(hit.body).is_ok_and(|(_, _, session, _, _)| {
                    session.map(|scope| scope.0) == hit.identity.session
                })
        })
        .map(|hit| hit.body)
        .collect();

    let mesh_for = |body, age| {
        let (kin, presented, session, form, pose) = bodies.get(body).ok()?;
        let room = rooms.of(body)?;
        let world = &geometry.of(body)?.0;
        let map = kin.material_map().ok()?;
        let down = map.direction(Vec2::Y);
        let side = map.direction(Vec2::X);
        let forward = if kin.facing < 0.0 { -side } else { side };
        let center = presented.map_or(kin.pos, |pose| pose.presented());
        let size = kin.size();
        let may_have_bones = form
            .and_then(|form| catalog.as_ref()?.get(form.id()))
            .is_some_and(|entry| {
                entry.body_kind == CharacterBodyKind::Standard
                    && !entry.tags.iter().any(|tag| {
                        matches!(
                            tag.as_str(),
                            "boneless"
                                | "construct"
                                | "elemental"
                                | "ghost"
                                | "slime"
                                | "robot"
                                | "automaton"
                                | "mechanical"
                                | "synthetic"
                                | "insect"
                                | "insectoid"
                                | "plant"
                                | "spirit"
                        )
                    })
            });
        let bones = (may_have_bones && pose.is_none_or(|pose| !body_presentation_hidden(pose)))
            .then(|| {
                let joint = |mark| {
                    let point = landmarks
                        .in_world(body, mark, LandmarkPose::ThisTick, kin)
                        .ok()??;
                    Some(point + (center - kin.pos))
                };
                Skeleton::from_landmarks(joint)
                    .unwrap_or_else(|| Skeleton::standing(center, size, forward, down))
                    .parts(center, size, forward, down)
            });

        let scope = SessionSpawnScope::new(session.map(|scope| scope.0)).in_room(Some(room));
        let tris = arcs(body.to_bits(), age, center, size, side, down);
        Some((
            scope,
            world,
            bones.unwrap_or_default(),
            FoldedMesh {
                tris,
                root: center,
                source: PartSource::Body(body),
                z: WORLD_Z_FX + 4.0,
                composite: Composite::Blend,
            },
        ))
    };

    for (entity, mut shock, mut mesh) in &mut effects {
        if rooms.of(entity) != rooms.of(shock.body) {
            remove_parts(&mut commands, &mut shock.parts);
            commands.entity(entity).despawn();
            continue;
        }
        if renewed.remove(&shock.body) {
            shock.age = 0.0;
        } else {
            shock.age += time.delta_secs();
        }
        if let Some((scope, world, bones, replacement)) = (shock.age < DURATION)
            .then(|| mesh_for(shock.body, shock.age))
            .flatten()
        {
            let art = bone_image(&mut atlas, &sprite_content, assets.as_deref(), &bones);
            show_parts(
                &mut commands,
                scope,
                world,
                &replacement,
                &mut shock,
                art,
                bones,
            );
            *mesh = replacement;
        } else {
            remove_parts(&mut commands, &mut shock.parts);
            commands.entity(entity).despawn();
        }
    }
    for body in renewed {
        if let Some((scope, world, bones, mesh)) = mesh_for(body, 0.0) {
            let mut shock = Shock {
                body,
                age: 0.0,
                parts: Vec::new(),
            };
            let art = bone_image(&mut atlas, &sprite_content, assets.as_deref(), &bones);
            show_parts(&mut commands, scope, world, &mesh, &mut shock, art, bones);
            commands.spawn_session_scoped(
                scope,
                (
                    Name::new("Shock"),
                    shock,
                    mesh,
                    Transform::default(),
                    Visibility::default(),
                ),
            );
        }
    }
}

fn bone_image<'a>(
    atlas: &'a mut Option<Handle<Image>>,
    content: &SpriteContent,
    assets: Option<&AssetServer>,
    bones: &[Bone],
) -> Option<&'a Handle<Image>> {
    let assets = assets.filter(|_| !bones.is_empty())?;
    Some(atlas.get_or_insert_with(|| {
        load_sheet_image(
            Some(content),
            assets,
            "shock",
            "content://ambition_shared/sprites/shock_bones.png",
        )
    }))
}

fn remove_parts(commands: &mut Commands, parts: &mut Vec<Entity>) {
    for part in parts.drain(..) {
        commands.entity(part).try_despawn();
    }
}

fn show_parts(
    commands: &mut Commands,
    scope: SessionSpawnScope,
    world: &ambition_platformer2d_core::World,
    mesh: &FoldedMesh,
    shock: &mut Shock,
    image: Option<&Handle<Image>>,
    bones: Vec<Bone>,
) {
    let Some(image) = image else {
        remove_parts(commands, &mut shock.parts);
        return;
    };
    let alpha = (1.0 - shock.age / DURATION).clamp(0.0, 1.0).sqrt();
    for part in shock.parts.drain(bones.len().min(shock.parts.len())..) {
        commands.entity(part).try_despawn();
    }
    for (index, bone) in bones.into_iter().enumerate() {
        let tint = if bone.far {
            (0.64, 0.74, 0.89)
        } else {
            (1.0, 1.0, 1.0)
        };
        let sprite = Sprite {
            image: image.clone(),
            rect: Some(bone.rect()),
            custom_size: Some(bone.size),
            flip_x: bone.flip_x,
            color: Color::srgba(tint.0, tint.1, tint.2, alpha),
            ..default()
        };
        let transform = Transform::from_translation(world_to_bevy(
            world,
            bone.at,
            mesh.z - 0.25 + index as f32 * 0.01,
        ))
        .with_rotation(Quat::from_rotation_z(bone.angle));
        let drawable = (sprite, transform, ReachesFrom(mesh.root));
        if let Some(&part) = shock.parts.get(index) {
            commands.entity(part).try_insert(drawable);
        } else {
            let part = commands
                .spawn_session_scoped(
                    scope,
                    (
                        Name::new("Shock bone"),
                        AnimatedWorldSpriteTint,
                        PresentationOf(shock.body),
                        drawable,
                    ),
                )
                .id();
            shock.parts.push(part);
        }
    }
}
