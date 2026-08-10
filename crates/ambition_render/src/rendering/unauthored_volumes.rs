//! **An unauthored attack is VISIBLE, not silent.**
//!
//! A character that names no `attack_vfx` draws its live hit volume directly, as
//! a translucent red shape, for exactly as long as that volume can hurt someone.
//! Jon, 2026-08-02:
//!
//! > have the concept of a character with an unauthored attack vfx. If that is
//! > the case and they don't have one associated with them the default vfx
//! > should be something that generates a transparent red polygon exactly over
//! > the hitpoly or hitbox so its clear where the attack is landing in the game
//! > even if the vfx hasn't been properly authored yet.
//!
//! ## Why this is not a placeholder sprite
//!
//! A stand-in swoosh would be a third thing that can disagree with the volume.
//! This draws the volume — the same `Hitbox` entity the damage resolver tests
//! against, at the same position, with the same shape — so it cannot drift, and
//! reading it tells you exactly what the game thinks it is doing.
//!
//! It also earns its keep in the other direction: incomplete content becomes
//! LOUD. A body whose attack was never authored used to swing an invisible box,
//! or worse, wear the protagonist's blade and look finished.
//!
//! ## Product-facing, on purpose
//!
//! This is not the debug overlay. The overlay draws every volume in the game
//! through gizmos, is off by default, and is developer tooling; this draws only
//! the volumes nobody authored art for, always, through the ordinary 2D mesh
//! path a player's machine already runs. The cost of an unfinished attack being
//! obviously unfinished is much lower than the cost of it being invisible.

use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::config::{world_to_bevy, WORLD_Z_FX};
use ambition_platformer2d_core::AabbExt;
use ambition_platformer2d_shared_tangle::lifecycle::{
    ActiveSessionScope, SessionSpawnScope, SpawnSessionScopedExt,
};
use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::sprite_render::{ColorMaterial, MeshMaterial2d};

/// The colour of "nobody drew this yet". Red, and see-through enough that the
/// body swinging it stays readable underneath.
const UNAUTHORED_TINT: Color = Color::srgba(1.0, 0.16, 0.22, 0.34);

/// One live volume's stand-in, tied to the `Hitbox` entity it draws.
#[derive(Component)]
pub(crate) struct UnauthoredVolumeVisual {
    hitbox: Entity,
}

/// Draw every live hit volume whose owner authored no attack VFX, and stop
/// drawing it the moment the volume stops existing.
///
/// The lifetime is the volume's, not a timer's: a strike volume exists exactly
/// while the owner's clock is inside its Active window, so the stand-in appears
/// and disappears with the ability to hurt. That is the property a placeholder
/// animation could not have.
#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_unauthored_attack_volumes(
    mut commands: Commands,
    // ⚠ OPTIONAL. Mesh and material stores belong to the render plugins, and
    // this system is registered in a presentation set that also runs under
    // headless and test compositions where those plugins are absent. Taking
    // them unconditionally makes Bevy's error handler panic before the system
    // body runs — 20 app integration tests, all with the same handler frame and
    // nothing in common but this.
    meshes: Option<ResMut<Assets<Mesh>>>,
    materials: Option<ResMut<Assets<ColorMaterial>>>,
    world: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
        ambition_platformer2d_core::RoomGeometry,
    >,
    active_session: Option<Res<ActiveSessionScope>>,
    // ⛔ **the OBSERVATION, not the authoritative component.** This used to hold
    // `Query<(Entity, &Hitbox)>` — a render system naming live simulation state,
    // which `engine.render-never-names-live-sim-state` exists to forbid and
    // which kept `Hitbox` stranded in `ambition_vfx`: moving it to
    // `ambition_combat` where it belongs would have forced a render→combat
    // dependency. The strike rows carry everything this needs.
    combat_geometry: Res<ambition_sim_view::CombatGeometryView>,
    // ⚠ the READ-MODEL pose, not the sim's `BodyKinematics`. Presentation reads
    // `ambition_sim_view` (E4) and `engine.render-never-names-live-sim-state`
    // enforces it; this system named the live cluster and the policy went red on
    // 2026-08-02. `rebuild_body_pose_views` requires only `BodyKinematics`, so
    // the population is identical — every body this used to match has a view.
    owners: Query<(
        &ambition_sim_view::BodyPoseView,
        Option<&ambition_sim_view::presented_pose::PresentedPose>,
        // The READ-MODEL fact, not the catalog — see the read below.
        Option<&ambition_sim_view::AttackVfxView>,
    )>,
    existing: Query<(Entity, &UnauthoredVolumeVisual)>,
    mut transforms: Query<&mut Transform, With<UnauthoredVolumeVisual>>,
) {
    // Retire stand-ins whose volume is gone. Done first so a hitbox that
    // despawned and had its index reused within a frame cannot be mistaken for
    // the same volume still being live.
    for (visual, mark) in &existing {
        if !combat_geometry
            .strikes
            .iter()
            .any(|strike| strike.strike == mark.hitbox)
        {
            commands.entity(visual).despawn();
        }
    }
    let Some(session_scope) =
        SessionSpawnScope::for_optional_active_session(active_session.as_deref())
    else {
        return;
    };
    // Retiring above still runs without a renderer; spawning cannot.
    let (Some(mut meshes), Some(mut materials)) = (meshes, materials) else {
        return;
    };

    for strike in &combat_geometry.strikes {
        let hitbox_entity = strike.strike;
        // Only a body-tracking strike stands in for a character's attack. A
        // world-anchored volume is a hazard or an arena special, and those are
        // authored as part of a room rather than as somebody's move.
        if !strike.anchored_to_body {
            continue;
        }
        let Ok((pose, presented, attack_vfx)) = owners.get(strike.owner) else {
            // ⚠ **SILENT WAS THE PROBLEM, not the skip** (queue D54). Skipping is
            // right — the alternative is the world-origin draw that cost an
            // investigation on the slash path. But D54 names THIS site as the
            // other candidate for Jon's top-left VFX and its test is *"if the
            // slash warn never fires, check the unauthored-volume stand-in"*.
            // Two silent skips make that a decision procedure with no output:
            // whichever one is happening, the log says nothing and the repro is
            // spent for nothing.
            //
            // ⚠ `warn_once`, because this runs per strike per frame and a live
            // swing would otherwise fill the log with the same line — which is
            // its own way of being unreadable.
            bevy::log::warn_once!(
                target: "ambition_platformer2d::render",
                "a live strike names owner {:?}, which publishes no combat pose \
                 view; drawing no stand-in. If Jon's stray VFX appears while this \
                 is in the log, THIS is the system, not the slash path.",
                strike.owner
            );
            continue;
        };
        // ⛔ **UNKNOWN IS NOT UNAUTHORED, and conflating them drew a stand-in
        // over every attack in the game.** This used to ask
        // `Option<Res<CharacterCatalog>>` directly: absent in every composition
        // that installs no catalog, so `attack_vfx` was `None`, so `authored`
        // was false, so a placeholder volume covered even the characters that
        // author their own art. `engine.character-authority-is-app-local` names
        // that shape for this reason.
        //
        // The read-model separates the two. No component = the resolver has not
        // spoken, and the honest response to not knowing is to draw NOTHING —
        // a stand-in is a positive claim that a character authored no art.
        let Some(attack_vfx) = attack_vfx else {
            continue;
        };
        if attack_vfx.authored() {
            continue;
        }

        // The DRAWN position, not the simulated one — the same reason the slash
        // visual samples it. A stand-in placed on the sim pose shudders against
        // a body drawn from the presented one.
        // The DRAWN position, not the simulated one — the same reason the slash
        // visual samples it. A stand-in placed on the sim pose shudders against
        // a body drawn from the presented one. The observation publishes the
        // anchor it resolved against, so re-placing is one translation rather
        // than a second evaluation of authoritative geometry.
        let drawn = presented.map_or(pose.pos, |p| p.presented());
        let to_drawn = drawn - strike.owner_anchor;
        let already = existing
            .iter()
            .find(|(_, mark)| mark.hitbox == hitbox_entity)
            .map(|(entity, _)| entity);
        // The volume is anchored to the owner, so its SHAPE is fixed for the
        // window and only the placement moves. Rebuild the mesh once; move it
        // every frame.
        if let Some(visual) = already {
            if let Ok(mut transform) = transforms.get_mut(visual) {
                let centre = strike.volume.bounds().center() + to_drawn;
                let at = world_to_bevy(&world.0, centre, WORLD_Z_FX + 1.0);
                transform.translation.x = at.x;
                transform.translation.y = at.y;
            }
            continue;
        }
        // The mesh is built in LOCAL space about the volume's own centre, so
        // the sim-resolved volume and the drawn one differ by the translation
        // alone — the shape is identical and only the transform moves.
        let volume = &strike.volume;
        let centre = volume.bounds().center() + to_drawn;
        let Some(mesh) = fan_mesh(volume, volume.bounds().center()) else {
            continue;
        };
        commands.spawn_session_scoped(
            session_scope,
            (
                Name::new("VFX unauthored attack volume"),
                Mesh2d(meshes.add(mesh)),
                MeshMaterial2d(materials.add(ColorMaterial::from_color(UNAUTHORED_TINT))),
                Transform::from_translation(world_to_bevy(&world.0, centre, WORLD_Z_FX + 1.0)),
                Visibility::Visible,
                UnauthoredVolumeVisual {
                    hitbox: hitbox_entity,
                },
            ),
        );
    }
}

/// Triangle-fan a convex volume about its own centre, in mesh-local space.
///
/// Convex is what makes a fan correct, and every volume that reaches here is:
/// `CombatVolume` is an AABB, an OBB, a circle or a convex hull by construction.
fn fan_mesh(volume: &ae::CombatVolume, centre: ae::Vec2) -> Option<Mesh> {
    let ring: Vec<ae::Vec2> = match volume {
        ae::CombatVolume::Convex { points, .. } => points.clone(),
        other => {
            let b = other.bounds();
            let (c, h) = (b.center(), b.half_size());
            vec![
                ae::Vec2::new(c.x - h.x, c.y - h.y),
                ae::Vec2::new(c.x + h.x, c.y - h.y),
                ae::Vec2::new(c.x + h.x, c.y + h.y),
                ae::Vec2::new(c.x - h.x, c.y + h.y),
            ]
        }
    };
    if ring.len() < 3 {
        return None;
    }
    // World y grows downward and Bevy's grows up, so the ring is flipped here
    // rather than at every vertex site later.
    let mut positions: Vec<[f32; 3]> = vec![[0.0, 0.0, 0.0]];
    positions.extend(
        ring.iter()
            .map(|p| [p.x - centre.x, -(p.y - centre.y), 0.0]),
    );
    let mut indices: Vec<u32> = Vec::with_capacity(ring.len() * 3);
    for i in 0..ring.len() as u32 {
        indices.extend([0, i + 1, (i + 1) % ring.len() as u32 + 1]);
    }
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_indices(Indices::U32(indices));
    Some(mesh)
}
