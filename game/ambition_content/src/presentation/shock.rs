//! The shock effect on a body that takes electric damage.

use std::collections::BTreeSet;

use ambition_characters::actor::{
    character_catalog::{CharacterBodyKind, CharacterCatalog},
    CharacterForm,
};
use ambition_combat::body_landmarks::{BodyLandmarks, LandmarkPose};
use ambition_platformer2d::damage::BodyHitResolved;
use ambition_platformer2d_core::{config::WORLD_Z_FX, BodyKinematics};
use ambition_platformer2d_shared_tangle::{
    body::geometry::PartSource,
    lifecycle::{LiveRooms, SessionScopedEntity, SessionSpawnScope, SpawnSessionScopedExt},
};
use ambition_render::{
    folded_mesh::{Composite, FoldedMesh, FoldedMeshSync},
    rendering::{body_visibility::body_presentation_hidden, ActorOverlaySet},
};
use ambition_sim_view::{BodyPoseView, PresentedPose};
use bevy::{prelude::*, time::Real};

mod art;

use art::{shock_art, Skeleton, DURATION};

#[derive(Component)]
struct Shock {
    body: Entity,
    age: f32,
}

pub(super) fn install(app: &mut App) {
    app.add_systems(
        Update,
        flash_shocked_bodies
            .in_set(ActorOverlaySet)
            .before(FoldedMeshSync),
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
    bodies: Query<(
        &BodyKinematics,
        Option<&PresentedPose>,
        Option<&SessionScopedEntity>,
        Option<&CharacterForm>,
        Option<&BodyPoseView>,
    )>,
    landmarks: BodyLandmarks,
    catalog: Option<Res<CharacterCatalog>>,
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
        let map = kin.material_map().ok()?;
        let down = map.direction(Vec2::Y);
        let side = map.direction(Vec2::X);
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
                    .unwrap_or_else(|| Skeleton::standing(center, size, side, down))
            });

        let scope = SessionSpawnScope::new(session.map(|scope| scope.0)).in_room(Some(room));
        let tris = shock_art(body.to_bits(), age, center, size, side, down, bones);
        Some((
            scope,
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
            commands.entity(entity).despawn();
            continue;
        }
        if renewed.remove(&shock.body) {
            shock.age = 0.0;
        } else {
            shock.age += time.delta_secs();
        }
        if let Some((_, replacement)) = (shock.age < DURATION)
            .then(|| mesh_for(shock.body, shock.age))
            .flatten()
        {
            *mesh = replacement;
        } else {
            commands.entity(entity).despawn();
        }
    }
    for body in renewed {
        if let Some((scope, mesh)) = mesh_for(body, 0.0) {
            commands.spawn_session_scoped(
                scope,
                (
                    Name::new("Shock"),
                    Shock { body, age: 0.0 },
                    mesh,
                    Transform::default(),
                    Visibility::default(),
                ),
            );
        }
    }
}
