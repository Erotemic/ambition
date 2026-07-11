//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

use super::*;
use ambition_portal::PortalChannelColor;
use bevy::camera::visibility::RenderLayers;
use bevy::sprite_render::MeshMaterial2d;

const WORLD: Vec2 = Vec2::new(1000.0, 600.0);

/// A thin-wall door pair (the c136/c137 shape): two wall portals 32px
/// apart on opposite faces, apertures aligned.
fn thin_wall_pair() -> (PlacedPortal, PlacedPortal) {
    let left = PlacedPortal::fixed(
        ambition_portal::PortalChannel::Authored(PortalChannelColor::Purple),
        Vec2::new(500.0, 300.0),
        Vec2::new(-1.0, 0.0),
        Vec2::new(9.0, 46.0),
    );
    let right = PlacedPortal::fixed(
        ambition_portal::PortalChannel::Authored(PortalChannelColor::Yellow),
        Vec2::new(532.0, 300.0),
        Vec2::new(1.0, 0.0),
        Vec2::new(9.0, 46.0),
    );
    (left, right)
}

fn test_app() -> App {
    let mut app = App::new();
    app.insert_resource(PortalWorldFrame { size: WORLD });
    app.insert_resource(Assets::<Image>::default());
    app.insert_resource(Assets::<TextureAtlasLayout>::default());
    app.insert_resource(Assets::<Mesh>::default());
    app.insert_resource(Assets::<PortalClipMaterial>::default());
    app.add_systems(Update, sync_portal_body_pieces);
    app
}

/// A loaded 48x48 sprite for the scene body.
fn loaded_sprite(app: &mut App) -> Sprite {
    let mut image = Image::default();
    image.texture_descriptor.size.width = 48;
    image.texture_descriptor.size.height = 48;
    let handle = app.world_mut().resource_mut::<Assets<Image>>().add(image);
    let mut sprite = Sprite::from_image(handle);
    sprite.custom_size = Some(Vec2::new(48.0, 48.0));
    sprite
}

fn spawn_body(app: &mut App, sprite: Sprite, transiting: bool) -> Entity {
    let (left, _) = thin_wall_pair();
    let kin = BodyKinematics {
        // Center 2px in FRONT of the left portal plane, feet-to-head
        // inside the aperture: the trailing 10px of the box has crossed.
        pos: Vec2::new(498.0, 300.0),
        vel: Vec2::ZERO,
        size: Vec2::new(24.0, 40.0),
        facing: 1.0,
    };
    let frame = PortalWorldFrame { size: WORLD };
    let translation = frame.to_render(kin.pos, 20.0);
    let mut body = app.world_mut().spawn((
        PortalSceneBody,
        kin,
        sprite,
        Transform::from_translation(translation),
    ));
    if transiting {
        body.insert(PortalTransit {
            straddling: left.channel,
            crossed: false,
        });
    }
    let body = body.id();
    let (left, right) = thin_wall_pair();
    app.world_mut().spawn(left);
    app.world_mut().spawn(right);
    body
}

fn piece_materials(app: &mut App) -> Vec<PortalClipMaterial> {
    let handles: Vec<_> = app
        .world_mut()
        .query_filtered::<&MeshMaterial2d<PortalClipMaterial>, With<PortalBodyPiece>>()
        .iter(app.world())
        .map(|m| m.0.clone())
        .collect();
    let materials = app.world().resource::<Assets<PortalClipMaterial>>();
    handles
        .iter()
        .map(|h| materials.get(h).expect("piece material exists").clone())
        .collect()
}

/// Mid-transit with a loaded texture: the real sprite is hidden and both
/// charts draw as clip-material pieces — `here` clipped at the entry
/// plane, `through` at the exit plane — so the body never draws its sunk
/// slice over the far side of the thin wall and nothing pops at the
/// centroid snap.
#[test]
fn transit_replaces_sprite_with_two_clipped_pieces() {
    let mut app = test_app();
    let sprite = loaded_sprite(&mut app);
    let body = spawn_body(&mut app, sprite, true);
    app.update();

    assert_eq!(
        *app.world().get::<Visibility>(body).expect("visibility"),
        Visibility::Hidden,
        "the pieces REPLACE the real sprite during transit"
    );
    let materials = piece_materials(&mut app);
    assert_eq!(materials.len(), 2, "one piece per chart");

    // Both pieces clip against a wall plane: render-space normals are
    // (-1, 0) for the here piece (front of the left portal) and (1, 0)
    // for the through piece (front of the right portal).
    let normals: Vec<Vec2> = materials
        .iter()
        .map(|m| Vec2::new(m.clip0.z, m.clip0.w))
        .collect();
    assert!(
        normals.contains(&Vec2::new(-1.0, 0.0)) && normals.contains(&Vec2::new(1.0, 0.0)),
        "clip planes face out of each portal, got {normals:?}"
    );

    // The through piece is bounded laterally by the exit aperture (its
    // material carries all three active planes).
    let through = materials
        .iter()
        .find(|m| m.clip0.z > 0.5)
        .expect("through piece");
    assert!(
        through.clip1.zw() != Vec2::ZERO && through.clip2.zw() != Vec2::ZERO,
        "through piece clips to the aperture span"
    );

    // The through piece sits at the mapped exit pose: body center 2px in
    // front of the left plane maps to 2px shy of emerged at the right
    // portal (engine x = 530 → render x = 30).
    let mut transforms = app
        .world_mut()
        .query_filtered::<&Transform, (With<PortalBodyPiece>, With<Mesh2d>)>()
        .iter(app.world())
        .map(|t| t.translation)
        .collect::<Vec<_>>();
    transforms.sort_by(|a, b| a.x.total_cmp(&b.x));
    assert_eq!(transforms.len(), 2);
    assert!(
        (transforms[0].x - -2.0).abs() < 1e-3,
        "here piece at the real pose, got {transforms:?}"
    );
    assert!(
        (transforms[1].x - 30.0).abs() < 1e-3,
        "through piece at the mapped exit pose, got {transforms:?}"
    );
    // z bands: the here piece IS the body (actor band); the through
    // piece sits just BELOW the window band, so a DISJOINT pair's
    // wormhole pane stays the single source wherever it covers the exit
    // region. At a DOORWAY pair no pane reaches either slice (the pane is
    // clipped to the wall slab; the slices are clipped to be outside it),
    // so both slices draw direct and the chart swap at the centroid snap
    // trades like for like. Pieces stay on the WORLD layer: through a
    // disjoint pair's window you must see your own copy emerging.
    assert!(
        (transforms[0].z - 20.0).abs() < 1e-3,
        "here piece in the actor band, got {transforms:?}"
    );
    assert!(
        (transforms[1].z - crate::PORTAL_EXIT_COPY_Z).abs() < 1e-3,
        "through piece below the window band, got {transforms:?}"
    );
    let layered = app
        .world_mut()
        .query_filtered::<(), (With<PortalBodyPiece>, With<RenderLayers>)>()
        .iter(app.world())
        .count();
    assert_eq!(
        layered, 0,
        "pieces live on the default WORLD layer so portal captures photograph them"
    );
}

/// No transit: no pieces, the real sprite shows whole.
#[test]
fn no_transit_keeps_real_sprite_visible() {
    let mut app = test_app();
    let sprite = loaded_sprite(&mut app);
    let body = spawn_body(&mut app, sprite, false);
    app.update();

    assert_eq!(
        *app.world().get::<Visibility>(body).expect("visibility"),
        Visibility::Inherited
    );
    let count = app
        .world_mut()
        .query_filtered::<(), With<PortalBodyPiece>>()
        .iter(app.world())
        .count();
    assert_eq!(count, 0);
}

/// Texture not loaded: fall back to the pre-clipping behavior — visible
/// real sprite plus one unclipped whole-sprite exit copy just below the
/// view window band.
#[test]
fn missing_texture_falls_back_to_sprite_copy() {
    let mut app = test_app();
    let mut sprite = Sprite::from_image(Handle::default());
    sprite.custom_size = Some(Vec2::new(48.0, 48.0));
    let body = spawn_body(&mut app, sprite, true);
    app.update();

    assert_eq!(
        *app.world().get::<Visibility>(body).expect("visibility"),
        Visibility::Inherited,
        "fallback keeps the real sprite visible"
    );
    let copies = app
        .world_mut()
        .query_filtered::<&Transform, (With<PortalBodyPiece>, With<Sprite>)>()
        .iter(app.world())
        .map(|t| t.translation)
        .collect::<Vec<_>>();
    assert_eq!(copies.len(), 1, "one unclipped exit copy");
    assert!(
        (copies[0].z - crate::PORTAL_EXIT_COPY_Z).abs() < 1e-3,
        "fallback copy hides below the window band, got {copies:?}"
    );
}

/// With a viewer in front of one face of the thin-wall pair, the NEAR
/// portal's frame draws above the glass (always whole) while the FAR
/// portal's frame drops under the window band — the open pane hides it
/// with the rest of the far side, instead of the frame punching through
/// the glass as a second portal (Jon: "I still see two portals when one
/// should be covered by the cone").
#[test]
fn far_portal_frame_hides_under_the_glass() {
    let mut app = test_app();
    app.add_systems(Update, sync_portal_visuals);
    let (left, right) = thin_wall_pair();
    app.world_mut().spawn(left);
    app.world_mut().spawn(right);
    app.insert_resource(crate::PortalViewer {
        present: true,
        eye: Vec2::new(460.0, 300.0), // left of the left face
        half_size: Vec2::new(12.0, 20.0),
        occluders: Vec::new(),
    });
    app.update();

    let window_band_top =
        crate::PORTAL_WINDOW_Z + crate::PortalViewConeConfig::default().z_proximity_span;
    let parts: Vec<(String, Vec3)> = app
        .world_mut()
        .query_filtered::<(&Name, &Transform), With<PortalVisual>>()
        .iter(app.world())
        .filter(|(n, _)| {
            let n = n.to_string();
            n.contains("rim") || n.contains("core") || n.contains("label")
        })
        .map(|(n, t)| (n.to_string(), t.translation))
        .collect();
    assert!(
        parts.len() >= 10,
        "both portals' frames spawn, got {parts:?}"
    );
    // Left portal (world x 500) renders near x 0; right (532) near x 32.
    for (name, t) in &parts {
        if t.x < 16.0 {
            assert!(
                t.z > window_band_top,
                "near frame part {name} at x={:.1} must draw over the glass, z={}",
                t.x,
                t.z
            );
        } else {
            assert!(
                t.z < crate::PORTAL_WINDOW_Z,
                "far frame part {name} at x={:.1} must hide under the glass, z={}",
                t.x,
                t.z
            );
        }
    }
}

/// The identifying frame (rim/core/label) is an OVERLAY: every portal
/// visual draws ABOVE the whole window z band, so a pane of takeover
/// glass can never hide half a portal (the c136/c137 "portal only half
/// appearing"), and BELOW actors, so a body in front still occludes it.
#[test]
fn portal_frame_draws_above_the_window_band_and_below_actors() {
    let mut app = test_app();
    app.add_systems(Update, sync_portal_visuals);
    let (left, right) = thin_wall_pair();
    app.world_mut().spawn(left);
    app.world_mut().spawn(right);
    app.update();

    let window_band_top =
        crate::PORTAL_WINDOW_Z + crate::PortalViewConeConfig::default().z_proximity_span;
    let zs: Vec<(String, f32)> = app
        .world_mut()
        .query_filtered::<(&Name, &Transform), With<PortalVisual>>()
        .iter(app.world())
        .map(|(n, t)| (n.to_string(), t.translation.z))
        .collect();
    let frame_parts: Vec<&(String, f32)> = zs
        .iter()
        .filter(|(n, _)| n.contains("rim") || n.contains("core") || n.contains("label"))
        .collect();
    assert!(
        frame_parts.len() >= 10,
        "two portals × (2 rims + 2 cores) + labels, got {zs:?}"
    );
    for (name, z) in &frame_parts {
        assert!(
            *z > window_band_top,
            "{name} must draw above the window band top {window_band_top}, got {z}"
        );
        assert!(*z < 20.0, "{name} must stay below the actor band, got {z}");
    }

    // And the frame stays on the WORLD layer: portal captures must
    // photograph it, so portals seen through a disjoint pair's window
    // still look like portals.
    let layered = app
        .world_mut()
        .query_filtered::<(), (With<PortalVisual>, With<RenderLayers>)>()
        .iter(app.world())
        .count();
    assert_eq!(
        layered, 0,
        "frame parts live on the default WORLD layer so captures photograph them"
    );
}
