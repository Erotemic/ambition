use std::collections::BTreeMap;

use ambition_platformer2d_core::config::{WORLD_Z_FX, WORLD_Z_PLAYER};
use ambition_platformer2d_shared_tangle::body::geometry::PartSource;
use ambition_platformer2d_shared_tangle::lifecycle::{
    ActiveSessionScope, LiveRoomInstance, LiveRooms, SessionSpawnScope, SpawnSessionScopedExt,
};
use ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhase;
use ambition_vfx::vfx::{AttackMarkKey, AttackMarks};
use bevy::prelude::*;

use crate::folded_mesh::{Composite, FoldedMesh, FoldedMeshSync, Tris};

pub(crate) struct AttackMarksPlugin;

impl Plugin for AttackMarksPlugin {
    fn build(&self, app: &mut App) {
        if app.get_sub_app(bevy::render::RenderApp).is_some() {
            app.add_systems(
                Update,
                show_marks
                    .after(Platformer2dSimulationPhase::CoreSimulation)
                    .before(FoldedMeshSync),
            );
        }
    }
}

#[derive(Component)]
struct LiveMark {
    key: AttackMarkKey,
    room: LiveRoomInstance,
}

fn show_marks(
    mut commands: Commands,
    marks: Option<Res<AttackMarks>>,
    session: Option<Res<ActiveSessionScope>>,
    rooms: LiveRooms,
    mut visible: Query<(Entity, &LiveMark, &mut FoldedMesh)>,
) {
    let Some(marks) = marks.filter(|marks| marks.is_changed()) else {
        return;
    };
    let mut pending: BTreeMap<_, _> = marks
        .0
        .iter()
        .filter(|mark| rooms.of(mark.key.owner) == Some(mark.room))
        .map(|mark| (&mark.key, mark))
        .collect();
    for (entity, mark, mut mesh) in &mut visible {
        if let Some(next) = pending.get(&mark.key) {
            if mark.room == next.room {
                *mesh = area_mesh(
                    mark.key.owner,
                    next.center,
                    next.half,
                    next.progress,
                    next.active,
                );
                pending.remove(&mark.key);
                continue;
            }
        }
        commands.entity(entity).despawn();
    }
    if let Some(scope) = SessionSpawnScope::for_optional_active_session(session.as_deref()) {
        for (key, mark) in pending {
            let mesh = area_mesh(
                key.owner,
                mark.center,
                mark.half,
                mark.progress,
                mark.active,
            );
            commands.spawn_session_scoped(
                scope.in_room(Some(mark.room)),
                (
                    Name::new("Attack mark"),
                    Transform::default(),
                    Visibility::default(),
                    LiveMark {
                        key: key.clone(),
                        room: mark.room,
                    },
                    mesh,
                ),
            );
        }
    }
}

fn quad(tris: &mut Tris, low: Vec2, high: Vec2, color: [f32; 4]) {
    tris.polygon(&[
        (low, color),
        (Vec2::new(high.x, low.y), color),
        (high, color),
        (Vec2::new(low.x, high.y), color),
    ]);
}

fn area_mesh(owner: Entity, center: Vec2, half: Vec2, progress: f32, active: bool) -> FoldedMesh {
    let low = center - half;
    let high = center + half;
    let (fill, edge) = if active {
        (
            [0.09, 0.015, 0.22, 0.38 - 0.14 * progress],
            [0.61, 0.30, 0.94, 0.47],
        )
    } else {
        (
            [0.14, 0.044, 0.016, 0.22 + 0.09 * progress],
            [0.96, 0.38, 0.081, 0.82],
        )
    };
    let width = 1.6_f32.min(half.min_element());
    let mut tris = Tris::default();
    quad(&mut tris, low, high, fill);
    quad(&mut tris, low, Vec2::new(high.x, low.y + width), edge);
    quad(&mut tris, Vec2::new(low.x, high.y - width), high, edge);
    quad(
        &mut tris,
        low + Vec2::Y * width,
        Vec2::new(low.x + width, high.y - width),
        edge,
    );
    quad(
        &mut tris,
        Vec2::new(high.x - width, low.y + width),
        high - Vec2::Y * width,
        edge,
    );

    if active {
        strike_streak(&mut tris, center, half, progress);
    } else {
        let corner = half.min_element().min(14.0);
        for x in [low.x, high.x - corner] {
            quad(
                &mut tris,
                Vec2::new(x, low.y),
                Vec2::new(x + corner, low.y + width * 2.0),
                edge,
            );
            quad(
                &mut tris,
                Vec2::new(x, high.y - width * 2.0),
                Vec2::new(x + corner, high.y),
                edge,
            );
        }
        let inner_x = low.x + width;
        let bar = (half.x - width) * 2.0 * progress;
        quad(
            &mut tris,
            Vec2::new(inner_x, high.y - 3.0 * width.min(half.y / 2.0)),
            Vec2::new(inner_x + bar, high.y - width),
            edge,
        );
    }
    FoldedMesh {
        tris,
        root: center,
        source: PartSource::World { owner: Some(owner) },
        // Scenery and actors can cover a warning if it uses the terrain depth.
        z: if active { WORLD_Z_FX - 1.0 } else { WORLD_Z_PLAYER + 1.0 },
        composite: Composite::Blend,
    }
}

fn strike_streak(tris: &mut Tris, center: Vec2, half: Vec2, progress: f32) {
    let (along, across) = if half.x >= half.y {
        (Vec2::X * half.x, Vec2::Y * half.y)
    } else {
        (Vec2::Y * half.y, Vec2::X * half.x)
    };
    let outer = [0.45, 0.12, 0.91, 0.0];
    let core = [0.96, 0.82, 1.0, 0.96 - 0.31 * progress];
    let point = |x: f32| {
        let taper = 1.0 - x.abs();
        let height = (0.24 + 0.36 * taper) * (1.0 - 0.63 * progress);
        let travel = (x * 17.0 - progress * 12.0).sin() * 0.19 * taper;
        (
            center + along * x + across * travel,
            across * height * taper.sqrt(),
        )
    };
    const STEPS: u32 = 16;
    let mut vertex = point(-1.0);
    for step in 1..=STEPS {
        let next = point(2.0 * step as f32 / STEPS as f32 - 1.0);
        let (a, wa) = vertex;
        let (b, wb) = next;
        for sign in [-1.0, 1.0] {
            tris.polygon(&[
                (a, core),
                (b, core),
                (b + sign * wb, outer),
                (a + sign * wa, outer),
            ]);
        }
        vertex = next;
    }
}
