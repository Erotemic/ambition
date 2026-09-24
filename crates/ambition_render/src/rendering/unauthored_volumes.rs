//! The default attack VFX: what a strike looks like when its character authored
//! no art of its own.
//!
//! Only one shipped character authors `attack_vfx` (the protagonist's blade is
//! cut from its own hit polygon and must not be reused). So this is what almost
//! every strike draws, and it must meet that standard.
//!
//! It draws the real volume geometry, and it exists exactly while the volume
//! can hurt. The volume, position, and lifetime are the strike's own. The form
//! is a sweep over the same hull: brightest along the leading edge and fading
//! behind it. The gradient comes from the strike's reach (where the volume sits
//! relative to its body), so it is correct for any move.
//!
//! The developer debug overlay is separate.

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

/// The swing's colour and its peak opacity, along the leading edge.
///
/// The peak equals the old flat fill's opacity, so an unauthored attack stays
/// as legible as before.
const SWING_TINT: Color = Color::srgba(1.0, 0.16, 0.22, 0.34);

/// How much of the peak the trailing edge keeps.
///
/// Not zero: the tail is still part of the hit. Low enough that the gradient
/// reads as a sweep, not a lit box.
const SWING_TRAILING_FRACTION: f32 = 0.2;

/// How wide the sweep is at its trailing end, as a fraction of the volume's
/// width there.
///
/// The taper makes it read as a swing; shading alone leaves a rectangle. The
/// leading edge is unchanged and the sweep stays inside the hull, so it never
/// shows a hit that is not there. It does under-draw the trailing corners. Set
/// this to 1.0 to get the box back.
const SWING_TRAILING_WIDTH: f32 = 0.18;

/// One live volume's stand-in, tied to the `Hitbox` entity it draws.
#[derive(Component)]
pub(crate) struct UnauthoredVolumeVisual {
    hitbox: Entity,
}

/// Draw every live hit volume whose owner authored no attack VFX, and stop
/// when the volume stops existing.
///
/// The lifetime is the volume's, not a timer's. A strike volume exists exactly
/// while the owner's clock is in its Active window.
#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_unauthored_attack_volumes(
    mut commands: Commands,
    // Render assets are optional so this runs in headless/test apps.
    meshes: Option<ResMut<Assets<Mesh>>>,
    materials: Option<ResMut<Assets<ColorMaterial>>>,
    world: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
        ambition_platformer2d_core::RoomGeometry,
    >,
    active_session: Option<Res<ActiveSessionScope>>,
    // Read the combat observation, not live strike components.
    combat_geometry: Res<ambition_sim_view::CombatGeometryView>,
    // Owner facts come from the read model. `PresentedPose` is optional
    // because not every owner has one.
    owners: Query<(
        Option<&ambition_sim_view::presented_pose::PresentedPose>,
        // The read-model fact, not the catalog. See below.
        Option<&ambition_sim_view::AttackVfxView>,
    )>,
    existing: Query<(Entity, &UnauthoredVolumeVisual)>,
    mut transforms: Query<&mut Transform, With<UnauthoredVolumeVisual>>,
) {
    // Despawn stand-ins whose volume is gone. Do this first, so a reused
    // hitbox index within a frame is not mistaken for the same live volume.
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
        // world-anchored volume is a hazard or arena special, authored with the
        // room.
        if !strike.anchored_to_body {
            continue;
        }
        let Ok((presented, attack_vfx)) = owners.get(strike.owner) else {
            // Skip, but log it: a silent skip gives no signal when debugging a
            // stray VFX, and drawing at the world origin is worse. `warn_once`
            // because this runs per strike per frame.
            bevy::log::warn_once!(
                target: "ambition_platformer2d::render",
                "a live strike names owner {:?}, which is not a live entity; \
                 drawing no stand-in. If Jon's stray VFX appears while this is in \
                 the log, THIS is the system, not the slash path.",
                strike.owner
            );
            continue;
        };
        // See `engine.character-authority-is-app-local`. No component means the
        // resolver has not decided yet, so draw nothing: a stand-in claims that
        // the character authored no art.
        let Some(attack_vfx) = attack_vfx else {
            continue;
        };
        if attack_vfx.authored() {
            continue;
        }

        // Use the drawn position, not the simulated one, like the slash visual.
        // A stand-in on the sim pose shudders against a body drawn from the
        // presented pose. The debug overlay applies the same delta, so the two
        // agree.
        let to_drawn = presented.map_or(ambition_platformer2d_core::Vec2::ZERO, |p| p.delta());
        let already = existing
            .iter()
            .find(|(_, mark)| mark.hitbox == hitbox_entity)
            .map(|(entity, _)| entity);
        // Rebuild the mesh once; move it every frame.
        if let Some(visual) = already {
            if let Ok(mut transform) = transforms.get_mut(visual) {
                let centre = strike.volume.bounds().center() + to_drawn;
                let at = world_to_bevy(&world.0, centre, WORLD_Z_FX + 1.0);
                transform.translation.x = at.x;
                transform.translation.y = at.y;
            }
            continue;
        }
        // The mesh is local to the volume centre, so only the transform moves.
        let volume = &strike.volume;
        let centre = volume.bounds().center() + to_drawn;
        // Direction, from the same read-model row: the body's box and its
        // facing. With no body row, use the flat fill.
        let reach = combat_geometry
            .bodies
            .iter()
            .find(|body| body.body == strike.owner)
            .and_then(|body| {
                strike_reach(
                    body.collision.center(),
                    volume.bounds().center(),
                    body.facing,
                )
            });
        let Some(mesh) = fan_mesh(volume, volume.bounds().center(), reach) else {
            continue;
        };
        commands.spawn_session_scoped(
            session_scope,
            (
                Name::new("VFX default attack swing"),
                Mesh2d(meshes.add(mesh)),
                MeshMaterial2d(materials.add(ColorMaterial::from_color(SWING_TINT))),
                Transform::from_translation(world_to_bevy(&world.0, centre, WORLD_Z_FX + 1.0)),
                Visibility::Visible,
                UnauthoredVolumeVisual {
                    hitbox: hitbox_entity,
                },
            ),
        );
    }
}

/// The direction this strike reaches, as a unit vector in world space.
///
/// Derived from the body-to-volume vector, so it is correct for an up-tilt,
/// a spike, or a jab without naming them. A volume centred on its body has no
/// reach, so the locomotion facing is used.
///
/// `None` when neither is usable. The caller then draws a flat fill, with no
/// claim about direction.
fn strike_reach(owner_centre: ae::Vec2, volume_centre: ae::Vec2, facing: f32) -> Option<ae::Vec2> {
    let reach = volume_centre - owner_centre;
    // A very small offset is centring noise, not a direction.
    if reach.length() > 1.0 {
        return Some(reach / reach.length());
    }
    (facing.abs() > 0.0).then(|| ae::Vec2::new(facing.signum(), 0.0))
}

/// Triangle-fan a convex volume about its own centre, in mesh-local space,
/// shaded as a sweep along `reach`.
///
/// Every vertex of the hull stays a vertex of the mesh, so the shape covers
/// the volume. Only per-vertex alpha changes, from
/// [`SWING_TRAILING_FRACTION`] at the back to full at the front.
/// `ColorMaterial` multiplies its colour by vertex colour, so
/// [`SWING_TINT`]'s alpha is the peak.
///
/// `reach` of `None` shades flat.
///
/// A fan is correct because every `CombatVolume` (AABB, OBB, circle, convex
/// hull) is convex.
fn fan_mesh(volume: &ae::CombatVolume, centre: ae::Vec2, reach: Option<ae::Vec2>) -> Option<Mesh> {
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
    let ring = taper_ring(&ring, centre, reach);
    // World y grows down and Bevy y grows up, so flip here once.
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
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_COLOR,
        swing_vertex_colors(&ring, centre, reach),
    );
    mesh.insert_indices(Indices::U32(indices));
    Some(mesh)
}

/// Narrow the hull behind its leading edge, so the drawn shape is a sweep and
/// not a box.
///
/// Each vertex keeps its position along the reach, so the sweep starts and
/// ends where the volume does. It moves toward the reach axis more the further
/// back it is. Leading vertices do not move.
///
/// `reach` of `None` returns the hull unchanged.
fn taper_ring(ring: &[ae::Vec2], centre: ae::Vec2, reach: Option<ae::Vec2>) -> Vec<ae::Vec2> {
    let Some(reach) = reach else {
        return ring.to_vec();
    };
    let perp = ae::Vec2::new(-reach.y, reach.x);
    let along: Vec<f32> = ring.iter().map(|p| (*p - centre).dot(reach)).collect();
    let (min, max) = along
        .iter()
        .fold((f32::MAX, f32::MIN), |(lo, hi), v| (lo.min(*v), hi.max(*v)));
    if !(max - min).is_finite() || max - min <= f32::EPSILON {
        return ring.to_vec();
    }
    ring.iter()
        .zip(&along)
        .map(|(p, a)| {
            let t = ((a - min) / (max - min)).clamp(0.0, 1.0);
            let width = SWING_TRAILING_WIDTH + (1.0 - SWING_TRAILING_WIDTH) * t;
            let local = *p - centre;
            centre + reach * local.dot(reach) + perp * (local.dot(perp) * width)
        })
        .collect()
}

/// One `[r, g, b, a]` per mesh vertex: the fan centre first, then the ring, in
/// the order [`fan_mesh`] builds them. Pure, so tests check it without a
/// renderer.
fn swing_vertex_colors(
    ring: &[ae::Vec2],
    centre: ae::Vec2,
    reach: Option<ae::Vec2>,
) -> Vec<[f32; 4]> {
    let flat = vec![[1.0, 1.0, 1.0, 1.0]; ring.len() + 1];
    let Some(reach) = reach else { return flat };
    let along: Vec<f32> = ring.iter().map(|p| (*p - centre).dot(reach)).collect();
    let (min, max) = along
        .iter()
        .fold((f32::MAX, f32::MIN), |(lo, hi), v| (lo.min(*v), hi.max(*v)));
    // A volume with no extent along its own reach has no sweep to draw.
    if !(max - min).is_finite() || max - min <= f32::EPSILON {
        return flat;
    }
    let shade = |v: f32| {
        let t = ((v - min) / (max - min)).clamp(0.0, 1.0);
        [
            1.0,
            1.0,
            1.0,
            SWING_TRAILING_FRACTION + (1.0 - SWING_TRAILING_FRACTION) * t,
        ]
    };
    // The fan centre is the volume's middle, so it takes the middle value.
    // Otherwise the gradient creases at the hub.
    let mut colors = vec![shade((min + max) * 0.5)];
    colors.extend(along.into_iter().map(shade));
    colors
}

#[cfg(test)]
mod tests {
    use super::*;

    fn box_ring(half: f32) -> Vec<ae::Vec2> {
        vec![
            ae::Vec2::new(-half, -half),
            ae::Vec2::new(half, -half),
            ae::Vec2::new(half, half),
            ae::Vec2::new(-half, half),
        ]
    }

    /// The sweep reaches as far as the volume, at full width where it leads, and
    /// never past it. It can under-draw the trailing corners
    /// (`SWING_TRAILING_WIDTH`), but it never shows a hit that is not there.
    #[test]
    fn the_swing_reaches_the_volume_exactly_and_never_past_it() {
        // `Aabb::new` is (centre, half_size).
        let volume = ae::CombatVolume::aabb(ae::Aabb::new(
            ae::Vec2::new(40.0, -12.0),
            ae::Vec2::new(18.0, 6.0),
        ));
        let centre = volume.bounds().center();
        let mesh =
            fan_mesh(&volume, centre, Some(ae::Vec2::new(1.0, 0.0))).expect("a box volume meshes");
        let positions = match mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
            Some(bevy::mesh::VertexAttributeValues::Float32x3(p)) => p.clone(),
            other => panic!("expected float3 positions, got {other:?}"),
        };
        assert_eq!(positions.len(), 5, "the fan centre plus every hull vertex");
        let half = volume.bounds().half_size();
        for v in &positions[1..] {
            // Inside the volume, always. Mesh y is flipped; the bound is not.
            assert!(
                v[0].abs() <= half.x + 1e-3 && v[1].abs() <= half.y + 1e-3,
                "vertex {v:?} escaped the authored volume (half {half:?})"
            );
        }
        // Reach is +x, so the two leading corners keep the full half-height.
        let leading: Vec<&[f32; 3]> = positions[1..]
            .iter()
            .filter(|v| (v[0] - half.x).abs() < 1e-3)
            .collect();
        assert_eq!(leading.len(), 2, "two vertices lead: {positions:?}");
        for v in leading {
            assert!(
                (v[1].abs() - half.y).abs() < 1e-3,
                "a leading vertex was narrowed: {v:?}"
            );
        }
        // The trailing pair is narrowed; otherwise this is still a box.
        let trailing: Vec<&[f32; 3]> = positions[1..]
            .iter()
            .filter(|v| (v[0] + half.x).abs() < 1e-3)
            .collect();
        assert_eq!(trailing.len(), 2);
        for v in trailing {
            assert!(
                v[1].abs() < half.y * 0.5,
                "the sweep did not taper behind its leading edge: {v:?}"
            );
        }
    }

    /// Brightest at the leading edge, dimmer behind, and never invisible. The peak
    /// is the full authored alpha.
    #[test]
    fn the_sweep_is_brightest_where_the_strike_reaches() {
        let ring = box_ring(10.0);
        let reach = ae::Vec2::new(1.0, 0.0);
        let colors = swing_vertex_colors(&ring, ae::Vec2::ZERO, Some(reach));
        assert_eq!(colors.len(), ring.len() + 1);

        // Ring order is [-x-y, +x-y, +x+y, -x+y]; index 0 is the fan centre.
        let alpha = |i: usize| colors[i][3];
        let leading = alpha(2).min(alpha(3));
        let trailing = alpha(1).max(alpha(4));
        assert!(
            (leading - 1.0).abs() < 1e-6,
            "the leading edge must keep the full authored alpha, got {leading}"
        );
        assert!(
            (trailing - SWING_TRAILING_FRACTION).abs() < 1e-6,
            "the trailing edge must sit at the floor, got {trailing}"
        );
        assert!(trailing > 0.0, "no part of the hit volume may disappear");
        assert!(
            colors
                .iter()
                .all(|c| c[0] == 1.0 && c[1] == 1.0 && c[2] == 1.0),
            "the hue is the material's; vertex colour carries only the ramp"
        );
    }

    /// The gradient follows the strike, not the screen. An up-attack sweeps up.
    #[test]
    fn the_sweep_follows_the_strike_and_not_an_axis() {
        let ring = box_ring(10.0);
        // World y grows downward, so "up" is negative y.
        let up = swing_vertex_colors(&ring, ae::Vec2::ZERO, Some(ae::Vec2::new(0.0, -1.0)));
        // Ring indices 0 and 1 are the -y (upper) corners.
        assert!(
            up[1][3] > up[3][3],
            "an upward strike must brighten toward its own up: {:?}",
            up
        );
        let down = swing_vertex_colors(&ring, ae::Vec2::ZERO, Some(ae::Vec2::new(0.0, 1.0)));
        assert!(down[3][3] > down[1][3], "and a spike the other way");
    }

    /// No usable direction gives a flat fill.
    #[test]
    fn a_strike_with_no_reach_shades_flat() {
        let ring = box_ring(10.0);
        let flat = swing_vertex_colors(&ring, ae::Vec2::ZERO, None);
        assert!(flat.iter().all(|c| c[3] == 1.0));

        // A volume centred on its own body has no reach; facing supplies one.
        let body = ae::Vec2::new(100.0, 50.0);
        assert!(
            strike_reach(body, body, 0.0).is_none(),
            "and no facing either"
        );
        let by_facing = strike_reach(body, body, -1.0).expect("facing is a direction");
        assert!(by_facing.x < 0.0 && by_facing.y == 0.0);

        // A real offset overrides facing: an up-tilt from a right-facing body
        // sweeps up.
        let up = strike_reach(body, body + ae::Vec2::new(0.0, -40.0), 1.0)
            .expect("an offset volume has reach");
        assert!(
            up.y < -0.9,
            "reach must follow the volume, not the facing: {up:?}"
        );
    }
}
