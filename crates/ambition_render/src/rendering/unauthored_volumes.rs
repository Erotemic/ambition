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
use ambition_vfx::{Hitbox, HitboxAnchor};
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
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    world: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
        ambition_platformer2d_core::RoomGeometry,
    >,
    active_session: Option<Res<ActiveSessionScope>>,
    catalog: Option<Res<ambition_characters::actor::character_catalog::CharacterCatalog>>,
    hitboxes: Query<(Entity, &Hitbox)>,
    owners: Query<(
        &ae::BodyKinematics,
        Option<&ambition_sim_view::presented_pose::PresentedPose>,
        Option<&ambition_characters::actor::WornCharacter>,
    )>,
    existing: Query<(Entity, &UnauthoredVolumeVisual)>,
    mut transforms: Query<&mut Transform, With<UnauthoredVolumeVisual>>,
) {
    // Retire stand-ins whose volume is gone. Done first so a hitbox that
    // despawned and had its index reused within a frame cannot be mistaken for
    // the same volume still being live.
    for (visual, mark) in &existing {
        if hitboxes.get(mark.hitbox).is_err() {
            commands.entity(visual).despawn();
        }
    }
    let Some(session_scope) =
        SessionSpawnScope::for_optional_active_session(active_session.as_deref())
    else {
        return;
    };

    for (hitbox_entity, hitbox) in &hitboxes {
        // Only a body-tracking strike stands in for a character's attack. A
        // World-anchored volume is a hazard or an arena special, and those are
        // authored as part of a room rather than as somebody's move.
        if !matches!(hitbox.anchor, HitboxAnchor::FollowOwner { .. }) {
            continue;
        }
        let Ok((kin, presented, worn)) = owners.get(hitbox.owner) else {
            continue;
        };
        // Authored ⇒ its own art draws it. Unauthored, unknown, or no worn
        // character at all ⇒ this.
        let authored = catalog
            .as_deref()
            .zip(worn)
            .and_then(|(catalog, worn)| catalog.attack_vfx(worn.id()))
            .is_some();
        if authored {
            continue;
        }

        // The DRAWN position, not the simulated one — the same reason the slash
        // visual samples it. A stand-in placed on the sim pose shudders against
        // a body drawn from the presented one.
        let drawn = presented.map_or(kin.pos, |p| p.presented());
        let already = existing
            .iter()
            .find(|(_, mark)| mark.hitbox == hitbox_entity)
            .map(|(entity, _)| entity);
        // The volume is anchored to the owner, so its SHAPE is fixed for the
        // window and only the placement moves. Rebuild the mesh once; move it
        // every frame.
        if let Some(visual) = already {
            if let Ok(mut transform) = transforms.get_mut(visual) {
                let centre = hitbox.world_volume(drawn).bounds().center();
                let at = world_to_bevy(&world.0, centre, WORLD_Z_FX + 1.0);
                transform.translation.x = at.x;
                transform.translation.y = at.y;
            }
            continue;
        }
        let volume = hitbox.world_volume(drawn);
        let centre = volume.bounds().center();
        let Some(mesh) = fan_mesh(&volume, centre) else {
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
