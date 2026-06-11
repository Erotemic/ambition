//! Through-portal **view windows**: each placed portal shows a trapezoid of
//! the world in front of its partner, receding INTO its host surface — you
//! look "through the portal a little bit," like glass set in the wall —
//! rendered live by an offscreen capture camera.
//!
//! ## How it works
//! Per placed portal with a placed partner, a **rig**: an offscreen image, a
//! capture `Camera2d` parked over the partner-side source rect, and a window
//! `Mesh2d` set into the entry's surface, textured with that image. All
//! geometry comes from `ambition_portal::view::view_cone` (window semantics:
//! the display map IS the body map, so the window image and a transiting body
//! agree at the face by construction):
//!
//! - the capture camera stays **axis-aligned** framing `ViewCone::source`
//!   (exact for axis-aligned portals);
//! - the body map's rotation+mirror lives entirely in the **UV mapping**:
//!   vertex `i` of the window shows `source_quad[i]`, normalized inside the
//!   source rect. Sim math is the single source of truth for what appears
//!   where; a UV-space mirror costs nothing on a textured mesh.
//!
//! Texture v runs top-down and the capture's top edge is the world rect's
//! min-y edge (render y-up flips twice), so `uv = (s - source.min) / size`
//! with no flip — pinned by `cone_uvs`' unit test below.
//!
//! ## 1-frame-lag infinite recursion
//! Window meshes are ordinary world entities on the default render layer, so
//! capture cameras see OTHER portals' windows. When a portal's host surface
//! lies inside its partner's capture rect (portals facing each other within
//! window depth), the captured window displays the image its own camera wrote
//! last frame — portal-through-portal recursion with one frame of lag per
//! depth level, Portal-style, zero extra code. No camera ever samples the
//! image it is writing (P's window shows the capture made near P's partner;
//! cross-sampling only, by construction). And because windows recede into
//! walls while captures frame the open room in FRONT of faces, a partner's
//! window never sits inside its own capture — the "portal showing its own
//! side back at you" artifact of a protruding-projection design can't happen.
//!
//! Rigs are keyed on the portal pair + config + world size and rebuilt only
//! when a key changes (a portal moved / appeared / vanished) — cameras and
//! render targets are NOT per-frame churn, unlike the cheap quad visuals.

use bevy::asset::RenderAssetUsages;
use bevy::camera::{ImageRenderTarget, RenderTarget, ScalingMode};
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use bevy::sprite_render::AlphaMode2d;

use ambition_engine_core::AabbExt;
use ambition_portal::view::view_cone;
use ambition_portal::{find_portal, PlacedPortal};

use crate::PortalWorldFrame;

/// Tuning for the view cones. A host overwrites the resource to retune; set
/// [`PortalPresentationPlugin::view_cones`](crate::PortalPresentationPlugin)
/// to `false` to drop the feature (and its capture passes) entirely.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct PortalViewConeConfig {
    /// How far the window recedes into the portal's host surface (world px).
    /// Keep near wall-thickness scale (`pieces::CARVE_DEPTH` is 60) so the
    /// window doesn't visually punch through into rooms beyond the wall.
    pub depth: f32,
    /// How much each side widens per px of depth (0 = straight corridor view).
    pub spread: f32,
    /// Offscreen capture height in texels; width follows the source rect's
    /// aspect. Capture area ≈ window area, so ~1:1 texel:px needs no more
    /// than the window is tall.
    pub resolution: u32,
    /// Render z of the window mesh. Default sits just BEHIND the portal rim
    /// (9.0) so the doorway stays crisp over its own view, above world blocks
    /// (0) and below actors (10+) — the wall visually opens up where the sim
    /// carves it.
    pub z: f32,
    /// Tint multiplied over the capture (alpha slightly < 1 lets the host
    /// surface ghost through, selling "looking INTO the surface").
    pub tint: Color,
    /// Debug: draw gizmo outlines of each portal's EXIT sample zone (the
    /// `ViewCone::source` rect, in the portal's channel color, sitting in
    /// front of its partner) and the entry window trapezoid. The host toggles
    /// this (in Ambition: `F8`). Off by default.
    pub debug_outline: bool,
}

impl Default for PortalViewConeConfig {
    fn default() -> Self {
        Self {
            depth: 90.0,
            spread: 0.20,
            resolution: 256,
            z: 8.9,
            tint: Color::srgba(1.0, 1.0, 1.0, 0.9),
            debug_outline: false,
        }
    }
}

/// One rig: the capture camera entity carries this; `cone` is the mesh entity
/// set into the entry's host surface. Rebuilt (not mutated) when `key` drifts
/// from the live portal pair.
#[derive(Component)]
pub struct PortalViewRig {
    key: RigKey,
    cone: Entity,
}

/// Everything a rig's geometry was derived from. Float equality is exactly
/// what we want: ANY drift (portal re-fired, room resized, config retuned)
/// rebuilds the rig.
#[derive(Clone, Copy, PartialEq)]
struct RigKey {
    enter_pos: Vec2,
    enter_normal: Vec2,
    exit_pos: Vec2,
    exit_normal: Vec2,
    world_size: Vec2,
    config: PortalViewConeConfig,
    /// Stable camera order (capture cameras must run before the main camera);
    /// keyed so a change in rig count re-lays the orders deterministically.
    order: isize,
}

/// Per-vertex UVs for the cone mesh: each source-quad corner normalized inside
/// the source rect. World y-down and texture v-down agree (the render y-flip
/// cancels between camera and capture), so this is flip-free — see module docs.
fn cone_uvs(source_quad: &[Vec2; 4], source: ambition_engine_core::Aabb) -> [[f32; 2]; 4] {
    let size = source.half_size() * 2.0;
    source_quad.map(|s| {
        [
            ((s.x - source.min.x) / size.x.max(1e-6)).clamp(0.0, 1.0),
            ((s.y - source.min.y) / size.y.max(1e-6)).clamp(0.0, 1.0),
        ]
    })
}

/// Maintain one rig per placed portal with a placed partner: spawn missing,
/// despawn stale, leave matching rigs alone (the captures re-render every
/// frame on their own — only the GEOMETRY is cached).
#[allow(clippy::too_many_arguments)]
pub fn sync_portal_view_cones(
    mut commands: Commands,
    config: Res<PortalViewConeConfig>,
    frame: Res<PortalWorldFrame>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    portals: Query<&PlacedPortal>,
    rigs: Query<(Entity, &PortalViewRig)>,
) {
    // Pre-world-sync frames have nothing to project onto.
    if frame.size == Vec2::ZERO {
        return;
    }
    // Desired rigs: every placed portal whose channel partner is also placed.
    let all: Vec<PlacedPortal> = portals.iter().copied().collect();
    let mut desired: Vec<(RigKey, PlacedPortal, PlacedPortal)> = Vec::new();
    for portal in &all {
        let Some(partner) = find_portal(&all, portal.channel.partner()) else {
            continue;
        };
        // Capture cameras render strictly before the main pass (order 0),
        // deterministically laid out by discovery index.
        let order = -8 - desired.len() as isize;
        desired.push((
            RigKey {
                enter_pos: portal.pos,
                enter_normal: portal.normal,
                exit_pos: partner.pos,
                exit_normal: partner.normal,
                world_size: frame.size,
                config: *config,
                order,
            },
            *portal,
            partner,
        ));
    }

    // Keep rigs whose key still matches a desired rig; despawn the rest.
    let mut missing: Vec<bool> = vec![true; desired.len()];
    for (entity, rig) in &rigs {
        match desired.iter().position(|(key, ..)| *key == rig.key) {
            Some(i) if missing[i] => missing[i] = false,
            _ => {
                commands.entity(entity).despawn();
                commands.entity(rig.cone).despawn();
            }
        }
    }

    for (i, (key, portal, partner)) in desired.into_iter().enumerate() {
        if !missing[i] {
            continue;
        }
        let cone = view_cone(
            &portal.frame(),
            &partner.frame(),
            key.config.depth,
            key.config.spread,
        );
        let source_size = cone.source.half_size() * 2.0;
        if source_size.x < 1.0 || source_size.y < 1.0 {
            continue;
        }

        // The offscreen capture, ~1:1 texels per world px at default depth.
        let height = key.config.resolution.max(8);
        let width = ((height as f32 * source_size.x / source_size.y) as u32).clamp(8, 2048);
        let image = images.add(Image::new_target_texture(
            width,
            height,
            TextureFormat::Rgba8UnormSrgb,
            None,
        ));

        // The window mesh, receding into the entry's host surface: positions
        // around the trapezoid centroid (render space), UVs from the
        // body-mapped source corners.
        let render_quad = cone
            .entry_quad
            .map(|p| frame.to_render(p, 0.0).truncate());
        let centroid = (render_quad[0] + render_quad[1] + render_quad[2] + render_quad[3]) * 0.25;
        let positions: Vec<[f32; 3]> = render_quad
            .iter()
            .map(|p| [p.x - centroid.x, p.y - centroid.y, 0.0])
            .collect();
        let uvs: Vec<[f32; 2]> = cone_uvs(&cone.source_quad, cone.source).to_vec();
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        mesh.insert_indices(Indices::U32(vec![0, 1, 2, 0, 2, 3]));

        let cone_entity = commands
            .spawn((
                Mesh2d(meshes.add(mesh)),
                MeshMaterial2d(materials.add(ColorMaterial {
                    color: key.config.tint,
                    alpha_mode: AlphaMode2d::Blend,
                    texture: Some(image.clone()),
                    ..default()
                })),
                Transform::from_translation(centroid.extend(key.config.z)),
                Name::new(format!("Portal view window ({})", portal.channel.name())),
            ))
            .id();

        // The capture camera, axis-aligned over the partner-side source rect.
        // Transparent clear: where the capture sees nothing, the cone shows
        // the room behind it instead of a black slab.
        let center = frame.to_render(cone.source.center(), 0.0);
        commands.spawn((
            Camera2d,
            Camera {
                order: key.order,
                clear_color: ClearColorConfig::Custom(Color::NONE),
                ..default()
            },
            // CRITICAL: the offscreen target from `new_target_texture` is
            // single-sampled, but a camera defaults to `Msaa::Sample4`. A
            // 4×-MSAA camera rendering into a 1× target silently produces
            // NOTHING — the image keeps its initial transparent clear, the
            // window samples fully-transparent texels, and you see straight
            // through the cone to the real world (which then pans/zooms with
            // your main camera). Matching the target's sample count fixes it.
            Msaa::Off,
            RenderTarget::Image(ImageRenderTarget::from(image)),
            Projection::Orthographic(OrthographicProjection {
                scaling_mode: ScalingMode::Fixed {
                    width: source_size.x,
                    height: source_size.y,
                },
                ..OrthographicProjection::default_2d()
            }),
            Transform::from_translation(center),
            PortalViewRig {
                key,
                cone: cone_entity,
            },
            Name::new(format!("Portal view capture ({})", portal.channel.name())),
        ));
    }
}

/// Debug overlay (gated by [`PortalViewConeConfig::debug_outline`]): for every
/// portal with a placed partner, outline the **exit sample zone** — the world
/// rect (`ViewCone::source`, sitting in front of the partner) that this
/// portal's capture camera frames — plus the entry window trapezoid where it
/// is displayed. The sample zone is drawn in the portal's own channel color,
/// so e.g. the purple portal's zone (bright purple) appears in front of the
/// yellow portal: "purple samples HERE." When the zone doesn't sit where the
/// exit actually is, the capture is mis-aimed; when it does but the window
/// still looks wrong, the bug is in the texture/UVs, not the geometry.
pub fn debug_portal_view_zones(
    config: Res<PortalViewConeConfig>,
    frame: Res<PortalWorldFrame>,
    portals: Query<&PlacedPortal>,
    mut gizmos: Gizmos,
) {
    if !config.debug_outline || frame.size == Vec2::ZERO {
        return;
    }
    let all: Vec<PlacedPortal> = portals.iter().copied().collect();
    let to_render = |p: Vec2| frame.to_render(p, 0.0).truncate();
    for portal in &all {
        let Some(partner) = find_portal(&all, portal.channel.partner()) else {
            continue;
        };
        let cone = view_cone(
            &portal.frame(),
            &partner.frame(),
            config.depth,
            config.spread,
        );
        let (_, core) = portal.channel.display();

        // Exit sample zone: the source rect, in render space (axis-aligned in
        // world stays axis-aligned through the y-flip). Bright channel color.
        let s = cone.source;
        let zone = [
            to_render(Vec2::new(s.min.x, s.min.y)),
            to_render(Vec2::new(s.max.x, s.min.y)),
            to_render(Vec2::new(s.max.x, s.max.y)),
            to_render(Vec2::new(s.min.x, s.max.y)),
            to_render(Vec2::new(s.min.x, s.min.y)),
        ];
        gizmos.linestrip_2d(zone, core);

        // Entry window trapezoid: where the zone is displayed, dimmer so the
        // two never read as the same shape.
        let entry: Vec<Vec2> = cone
            .entry_quad
            .iter()
            .chain(std::iter::once(&cone.entry_quad[0]))
            .map(|p| to_render(*p))
            .collect();
        gizmos.linestrip_2d(entry, core.with_alpha(0.4));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_engine_core as ae;
    use ambition_portal::pieces::PortalFrame;

    /// Pin the flip-free UV convention: the source-rect corner with MINIMAL
    /// world coords (left, world-top) is texture (0,0); maximal is (1,1).
    /// If the capture orientation ever changes, this is the test that catches
    /// the upside-down cone before a human does.
    #[test]
    fn cone_uvs_are_flip_free_in_world_space() {
        let source = ae::Aabb::new(Vec2::new(100.0, 50.0), Vec2::new(40.0, 20.0));
        let quad = [
            Vec2::new(60.0, 30.0),  // world min corner
            Vec2::new(140.0, 30.0), // world max-x, min-y
            Vec2::new(140.0, 70.0), // world max corner
            Vec2::new(60.0, 70.0),  // world min-x, max-y
        ];
        let uvs = cone_uvs(&quad, source);
        assert_eq!(uvs[0], [0.0, 0.0]);
        assert_eq!(uvs[1], [1.0, 0.0]);
        assert_eq!(uvs[2], [1.0, 1.0]);
        assert_eq!(uvs[3], [0.0, 1.0]);
    }

    /// The UVs always cover the full unit square's bounds (the source rect IS
    /// the quad's bbox), rotated per the view map — pinned for a 90° pair.
    #[test]
    fn cone_uvs_rotate_with_the_view_map() {
        let enter = PortalFrame {
            pos: Vec2::new(100.0, 300.0),
            normal: Vec2::new(0.0, -1.0),
            half_extent: Vec2::new(46.0, 9.0),
        };
        let exit = PortalFrame {
            pos: Vec2::new(400.0, 200.0),
            normal: Vec2::new(-1.0, 0.0),
            half_extent: Vec2::new(9.0, 46.0),
        };
        let cone = view_cone(&enter, &exit, 120.0, 0.25);
        let uvs = cone_uvs(&cone.source_quad, cone.source);
        // Every UV inside the unit square…
        for uv in &uvs {
            assert!((0.0..=1.0).contains(&uv[0]) && (0.0..=1.0).contains(&uv[1]));
        }
        // …and the quad touches all four sides (it spans its own bbox).
        let touch = |f: &dyn Fn(&[f32; 2]) -> f32, v: f32| uvs.iter().any(|uv| (f(uv) - v).abs() < 1e-4);
        assert!(touch(&|uv| uv[0], 0.0) && touch(&|uv| uv[0], 1.0));
        assert!(touch(&|uv| uv[1], 0.0) && touch(&|uv| uv[1], 1.0));
        // 90° pair: the entry's near edge (corners 0,1 — the portal face) maps
        // onto the exit face, which is the source rect's max-x edge → u = 1.
        assert!((uvs[0][0] - 1.0).abs() < 1e-4 && (uvs[1][0] - 1.0).abs() < 1e-4);
    }
}
