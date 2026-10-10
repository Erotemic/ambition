use super::*;
use ambition_portal2d::PortalChannelColor;
use crate::PortalWorldFrame;
use bevy::camera::visibility::RenderLayers;
use bevy::sprite_render::MeshMaterial2d;

const WORLD: Vec2 = Vec2::new(1000.0, 600.0);

/// A thin-wall door pair (the c136/c137 shape): two wall portals 32px
/// apart on opposite faces, apertures aligned.
fn thin_wall_pair() -> (PlacedPortal, PlacedPortal) {
    let left = PlacedPortal::fixed(
        ambition_portal2d::PortalChannel::Authored(PortalChannelColor::Purple),
        Vec2::new(500.0, 300.0),
        Vec2::new(-1.0, 0.0),
        Vec2::new(9.0, 46.0),
    );
    let right = PlacedPortal::fixed(
        ambition_portal2d::PortalChannel::Authored(PortalChannelColor::Yellow),
        Vec2::new(532.0, 300.0),
        Vec2::new(1.0, 0.0),
        Vec2::new(9.0, 46.0),
    );
    (left, right)
}

fn test_app() -> App {
    let mut app = App::new();
    crate::one_live_room(&mut app, WORLD);
    app.insert_resource(Assets::<Image>::default());
    app.insert_resource(Assets::<TextureAtlasLayout>::default());
    app.insert_resource(Assets::<Mesh>::default());
    app.insert_resource(Assets::<PortalClipMaterial>::default());
    // See the note in `far_side::tests`: the piece builder states a reason and
    // the resolver owns the `Visibility` write, so both belong in the harness.
    app.add_systems(
        Update,
        (
            sync_portal_body_pieces,
            crate::source_visibility::resolve_portal_source_visibility,
        )
            .chain(),
    );
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
    let kin = crate::PortalBodyView {
        // Center 2px in FRONT of the left portal plane, feet-to-head
        // inside the aperture: the trailing 10px of the box has crossed.
        pos: Vec2::new(498.0, 300.0),
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
        body.insert(crate::PortalTransitView { straddling: left.channel });
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

/// Each body that straddles a portal is cut, not one body only: two bodies in
/// transit give two pairs of pieces, and both real sprites are hidden. The
/// control is a third body that does not transit, which stays whole.
#[test]
fn each_body_in_transit_is_cut_into_its_own_two_pieces() {
    let mut app = test_app();
    let sprite = loaded_sprite(&mut app);
    let first = spawn_body(&mut app, sprite.clone(), true);
    let second = spawn_body(&mut app, sprite.clone(), true);
    let whole = spawn_body(&mut app, sprite, false);
    app.update();
    app.update();
    assert_eq!(piece_materials(&mut app).len(), 4, "two pieces for each of two bodies in transit");
    for (body, hidden) in [(first, true), (second, true), (whole, false)] {
        assert_eq!(
            app.world().get::<crate::source_visibility::PortalTransitHidden>(body).is_some(),
            hidden,
            "the body in transit is drawn as its pieces, and the other as itself"
        );
    }
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

/// An app that draws each portal's frame: its line of light and its label.
fn frame_app() -> App {
    let mut app = test_app();
    app.insert_resource(Assets::<crate::PortalGlowMaterial>::default());
    app.init_resource::<Time>();
    app.add_systems(Update, (sync_portal_visuals, crate::sync_portal_glows));
    app
}

/// The name and place of each part of each portal's frame.
fn frame_parts(app: &mut App) -> Vec<(String, Vec3)> {
    app.world_mut()
        .query_filtered::<(&Name, &Transform), Or<(With<PortalVisual>, With<crate::PortalGlow>)>>()
        .iter(app.world())
        .filter(|(n, _)| n.contains("glow") || n.contains("label"))
        .map(|(n, t)| (n.to_string(), t.translation))
        .collect()
}

/// Each portal's frame (its line of light and its label) draws UNDER the
/// exit-side body slice and under the whole window band, and over the world:
/// a body is over the light on each side of a seam, and a window covers the
/// half of its own line that is behind its face. With a viewer or with none:
/// the frame's z does not follow the viewer.
#[test]
fn a_portals_frame_draws_under_the_exit_slice_and_the_window_and_over_the_world() {
    for viewer in [false, true] {
        let mut app = frame_app();
        let (left, right) = thin_wall_pair();
        app.world_mut().spawn(left);
        app.world_mut().spawn(right);
        if viewer {
            app.insert_resource(crate::PortalViewers::one(crate::PortalViewer {
                observer: None,
                present: true,
                room: Some(ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance::ACTIVATION),
                eye: Vec2::new(460.0, 300.0), // left of the left face
                half_size: Vec2::new(12.0, 20.0),
                occluders: Vec::new(),
            }));
        }
        app.update();
        app.update();

        let parts = frame_parts(&mut app);
        assert_eq!(parts.len(), 4, "a line and a label for each of two portals, got {parts:?}");
        for (name, at) in &parts {
            assert!(
                at.z < crate::PORTAL_EXIT_COPY_Z && at.z < crate::PORTAL_WINDOW_Z,
                "{name} must draw under the exit slice and the window (viewer: {viewer}), z={}",
                at.z
            );
            assert!(
                at.z > ambition_platformer2d_core::config::WORLD_Z_BLOCK,
                "{name} must draw over the world, z={}",
                at.z
            );
        }

        // And the frame stays on the WORLD layer: portal captures must
        // photograph it, so portals seen through a window still look like
        // portals, and the far portal's line is the other half of a seam.
        let layered = app
            .world_mut()
            .query_filtered::<(), (Or<(With<PortalVisual>, With<crate::PortalGlow>)>, With<RenderLayers>)>()
            .iter(app.world())
            .count();
        assert_eq!(layered, 0, "frame parts live on the default WORLD layer so captures photograph them");
    }
}

/// Two live rooms of different sizes, and the same portal pair in each (view
/// half, V2m). Each portal's visuals are placed by its own room's frame and
/// stamped into its own room, so each view draws only its room's portals.
/// Before, one size was copied from the sole live room, so with two rooms
/// live no frame was synced and every portal was drawn unbanded, by a stale
/// size, in every view.
#[test]
fn each_portal_is_drawn_in_its_own_live_room() {
    use ambition_platformer2d_shared_tangle::lifecycle::{InRoomInstance, LiveRoomInstance};
    let mut app = test_app();
    app.add_systems(Update, sync_portal_visuals);
    let small = Vec2::new(400.0, 300.0);
    let first = LiveRoomInstance::ACTIVATION;
    let second = LiveRoomInstance::from_ordinal(1);
    ambition_platformer2d_shared_tangle::lifecycle::spawn_live_room(
        app.world_mut(),
        second,
        ae::RoomGeometry(ae::World::new("small portal room", small, small * 0.5, Vec::new())),
    );
    let (left, right) = thin_wall_pair();
    for room in [first, second] {
        app.world_mut().spawn((left.clone(), InRoomInstance(room)));
        app.world_mut().spawn((right.clone(), InRoomInstance(room)));
    }
    app.update();

    let labels: Vec<(Option<LiveRoomInstance>, Vec3)> = app
        .world_mut()
        .query_filtered::<(&Name, &Transform, Option<&InRoomInstance>), With<PortalVisual>>()
        .iter(app.world())
        .filter(|(name, ..)| name.as_str() == "Portal label")
        .map(|(_, transform, stamp)| (stamp.map(|stamp| stamp.0), transform.translation))
        .collect();
    assert_eq!(labels.len(), 4, "two portals in each of two rooms, one label each: {labels:?}");
    for (room, size) in [(first, WORLD), (second, small)] {
        let frame = PortalWorldFrame { size };
        let mut expected: Vec<Vec2> = [&left, &right]
            .iter()
            .map(|portal| frame.to_render(portal.pos + portal.normal.normalize_or_zero() * 24.0, 0.0).truncate())
            .collect();
        let mut drawn: Vec<Vec2> = labels
            .iter()
            .filter(|(stamp, _)| *stamp == Some(room))
            .map(|(_, at)| at.truncate())
            .collect();
        let key = |v: &Vec2| (v.x.round() as i32, v.y.round() as i32);
        expected.sort_by_key(key);
        drawn.sort_by_key(key);
        assert_eq!(drawn, expected, "room {room:?} ({size}): its portals' labels by its own frame");
    }
}
