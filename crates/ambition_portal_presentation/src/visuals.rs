//! Default portal-seam visuals: portal quads + labels, mid-transit body-piece
//! decomposition, and the disorientation indicator.
//!
//! Gun-specific sprites and shot/pickup markers live in `gun_visuals.rs` so the
//! reusable portal presentation surface can move toward static portals, scripted
//! emitters, moving portals, and other non-gun use cases without inheriting
//! Ambition's current portal-gun workflow.
//!
//! Every system here is read-only over the portal sim and rebuilds its transient
//! entities each frame, so visuals cannot desync from the sim.

use bevy::image::TextureAtlasLayout;
use bevy::prelude::*;
use bevy::sprite::Anchor;
use bevy::sprite_render::MeshMaterial2d;

use ambition_engine_core as ae;
#[cfg(feature = "effect_transit_masks")]
use ambition_engine_core::AabbExt;
use ambition_platformer_primitives::body::BodyKinematics;
use ambition_platformer_primitives::markers::{PlayerEntity, PrimaryPlayer};
use ambition_platformer_primitives::orientation::ActorRoll;

use ambition_portal::pieces as pp;
use ambition_portal::{
    copy_transform, find_portal, PlacedPortal, PortalGunPickup, PortalInputWarp, PortalShot,
    PortalTransit, PORTAL_VISUAL_THICKNESS,
};

use crate::clip_material::{
    clip_piece_transform, clip_plane_render, sprite_frame_basis, PortalClipMaterial,
    CLIP_PLANE_OFF,
};
use crate::{gun_visuals, PortalGunArt, PortalSceneBody, PortalWorldFrame};

/// Marks a sprite entity that visualizes a [`PlacedPortal`]. Rebuilt each frame from
/// the sim portals, so it never drifts.
#[derive(Component)]
pub struct PortalVisual;

/// Marks a transient sprite drawing one portal-aware spatial piece of a body
/// mid-transit (the entry-side slice or the exit-side slice). Rebuilt each frame.
#[derive(Component)]
pub struct PortalBodyPiece;

/// Marks the transient "portal disorientation" indicator above the controlled
/// body — visible exactly while held movement input is portal-warped.
#[derive(Component)]
pub struct PortalDisorientIndicator;

/// Show a small indicator over the controlled body whenever movement input is
/// portal-warped ([`PortalInputWarp`]) — so the "held left but moving right"
/// state is legible, and it disappears the instant the warp drops (on release /
/// redirect). Placeholder dot+glyph for now; a nicer effect (incl. on the
/// joystick visual) can replace it later.
///
/// FIXME(host-seam): this still queries Ambition's primary-player marker pair.
/// Isolate that dependency behind a host-supplied focus marker before publishing
/// this as a less-opinionated portal presentation crate.
pub fn sync_portal_disorientation_indicator(
    mut commands: Commands,
    frame: Res<PortalWorldFrame>,
    existing: Query<Entity, With<PortalDisorientIndicator>>,
    player: Query<
        (&BodyKinematics, Has<PortalInputWarp>),
        (With<PlayerEntity>, With<PrimaryPlayer>),
    >,
) {
    for entity in &existing {
        commands.entity(entity).despawn();
    }
    let Ok((kin, warped)) = player.single() else {
        return;
    };
    if !warped {
        return;
    }
    // A little spinning-arrow glyph just above the head.
    let pos = kin.pos + Vec2::new(0.0, -(kin.size.y * 0.5 + 16.0));
    let translation = frame.to_render(pos, ae::config::WORLD_Z_PLAYER + 9.0);
    commands.spawn((
        PortalDisorientIndicator,
        Text2d::new("\u{21BB}"), // ↻ clockwise open circle arrow
        TextFont {
            font_size: 18.0,
            ..default()
        },
        TextColor(Color::srgb(0.74, 0.92, 1.0)),
        Transform::from_translation(translation),
        Name::new("Portal disorientation indicator"),
    ));
}

/// Draw the transiting body as its two portal-aware **pieces**, texture-clipped
/// at the portal planes: the real sprite is hidden and replaced by a `here`
/// quad (the sprite clipped to the front of the entry plane, at the real pose)
/// plus a `through` quad (the sprite posed by the BODY map — `copy_transform`:
/// rotation-only for det +1 maps, rotation plus flip for det -1 maps — clipped
/// to the front of the exit plane and to the exit aperture span). Because the
/// portal map is an isometry the two slices tile continuously across the seam,
/// so nothing pops when the authoritative position snaps at the centroid
/// crossing, and the sunk slice never draws over the far side of a thin wall
/// (the Q10 crossing flicker). Clipping runs in [`PortalClipMaterial`]'s
/// fragment shader against world positions, so it is exact for any anchor /
/// trim rect / flip / roll. Shared by EVERY visual-effect mode
/// (windows / masks / off).
///
/// Pieces are rebuilt each frame from the same `Sprite`, after the host's
/// animator has updated it, so they can never drift from the real sprite; the
/// decomposition frames come from the tested Core-invariant
/// [`pp::compute_body_pieces`], so they can never drift from collision.
///
/// **Fallback:** without a loaded texture / atlas layout (or on a headless
/// host that never registered the material — the asset params are `Option`al),
/// the pre-clipping behavior is kept: the real sprite stays visible and an
/// unclipped whole-sprite copy is drawn at the exit just below the view window
/// ([`crate::PORTAL_EXIT_COPY_Z`]), which captures it on the far side and
/// hides the redundant world draw.
///
/// When the legacy **Transit Masks** effect is the active
/// [`crate::PortalEffectSelection`] (compiled via `effect_transit_masks`),
/// the opaque "feet in, feet out" boxes are drawn over the invisible slice of
/// each chart, like before the view windows existed — kept selectable for
/// A/B profiling against the windows.
///
/// Known gap: sibling overlays of the body sprite (hit-flash silhouette, held
/// gun) are not decomposed; a hit flash mid-transit draws the whole silhouette
/// unclipped for its few frames.
///
/// Operates on the host-tagged [`PortalSceneBody`] visual entity.
pub fn sync_portal_body_pieces(
    mut commands: Commands,
    #[cfg(feature = "effect_transit_masks")] selection: Res<crate::PortalEffectSelection>,
    frame: Res<PortalWorldFrame>,
    pieces: Query<Entity, With<PortalBodyPiece>>,
    portals: Query<&PlacedPortal>,
    images: Option<Res<Assets<Image>>>,
    layouts: Option<Res<Assets<TextureAtlasLayout>>>,
    meshes: Option<ResMut<Assets<Mesh>>>,
    clip_materials: Option<ResMut<Assets<PortalClipMaterial>>>,
    mut unit_mesh: Local<Option<Handle<Mesh>>>,
    mut body_visual: Query<
        (
            &BodyKinematics,
            Option<&PortalTransit>,
            Option<&ActorRoll>,
            &Sprite,
            Option<&Anchor>,
            &Transform,
            &mut Visibility,
        ),
        With<PortalSceneBody>,
    >,
) {
    for entity in &pieces {
        commands.entity(entity).despawn();
    }
    let Ok((kin, transit, roll, sprite, source_anchor, source_transform, mut visibility)) =
        body_visual.single_mut()
    else {
        return;
    };
    // Outside transit the real character sprite shows whole; the pieces are a
    // transit-only replacement.
    *visibility = Visibility::Inherited;
    // The body is transiting exactly one portal — decompose against that pair.
    let Some(transit) = transit else {
        return;
    };
    let all: Vec<PlacedPortal> = portals.iter().copied().collect();
    let (Some(enter_portal), Some(exit_portal)) = (
        find_portal(&all, transit.straddling),
        find_portal(&all, transit.straddling.partner()),
    ) else {
        return;
    };
    let body = ae::Aabb::new(kin.pos, kin.size * 0.5);
    // Decompose via the tested Core-invariant function so the pieces can never
    // drift from the collision / gameplay decomposition.
    let pieces = pp::compute_body_pieces(body, Some((enter_portal.frame(), exit_portal.frame())));
    let Some(through) = pieces.through else {
        // Touching a portal but nothing has crossed the plane yet.
        return;
    };
    let (enter, exit) = (through.enter, through.exit);
    let base_roll = roll.map_or(0.0, |r| r.angle);

    // The through pose: the sprite emerging from the exit, placed by the BODY
    // map exactly. The active convention decides whether that map factors as a
    // pure rotation or as rotation plus one x-reflection.
    let exit_center = pp::map_point(kin.pos, &enter, &exit);
    let copy = copy_transform(&enter, &exit);
    let exit_roll = base_roll + copy.roll;
    // `apply_character_frame` has already mirrored the anchor to match the
    // source sprite's current `flip_x` value. If the portal copy toggles the
    // sprite flip, mirror the anchor too; otherwise trimmed/off-centre frames
    // render from the wrong basis and can look stretched or scaled as the
    // copy emerges.
    let source_anchor_v = source_anchor.map_or(Vec2::ZERO, |a| a.0);
    let mut through_flip = sprite.flip_x;
    let mut through_anchor = source_anchor_v;
    if copy.flip_x {
        through_flip = !through_flip;
        through_anchor.x = -through_anchor.x;
    }

    // Texture-clipped piece path: both charts as clip-material quads at the
    // body's own z (a piece is the body, in front of walls and windows alike).
    let mut drew_clipped = false;
    if let (Some(images), Some(layouts), Some(mut meshes), Some(mut materials)) =
        (images, layouts, meshes, clip_materials)
    {
        if let Some(basis) = sprite_frame_basis(sprite, &layouts, &images) {
            let mesh = unit_mesh
                .get_or_insert_with(|| meshes.add(Rectangle::default()))
                .clone();
            let tint = {
                let c = sprite.color.to_linear();
                Vec4::new(c.red, c.green, c.blue, c.alpha)
            };
            let flip_flag = |flip: bool| Vec4::new(if flip { 1.0 } else { 0.0 }, 0.0, 0.0, 0.0);

            // `here`: the real pose, keeping only what is still in front of
            // the entry plane (the sunk slice belongs to the exit chart).
            commands.spawn((
                PortalBodyPiece,
                Mesh2d(mesh.clone()),
                MeshMaterial2d(materials.add(PortalClipMaterial {
                    uv_rect: basis.uv_rect,
                    control: flip_flag(sprite.flip_x),
                    tint,
                    clip0: clip_plane_render(&frame, enter.pos, enter.normal),
                    clip1: CLIP_PLANE_OFF,
                    clip2: CLIP_PLANE_OFF,
                    color_texture: sprite.image.clone(),
                })),
                clip_piece_transform(source_transform, source_anchor_v, basis.size),
                Name::new("Portal body piece (here)"),
            ));

            // `through`: the mapped pose, keeping only what has emerged in
            // front of the exit plane, laterally bounded by the doorway.
            let through_base = Transform {
                translation: frame.to_render(exit_center, source_transform.translation.z),
                rotation: Quat::from_rotation_z(exit_roll),
                scale: source_transform.scale,
            };
            let along = Vec2::new(-exit.normal.y, exit.normal.x);
            let aperture_half = exit.aperture_half();
            commands.spawn((
                PortalBodyPiece,
                Mesh2d(mesh),
                MeshMaterial2d(materials.add(PortalClipMaterial {
                    uv_rect: basis.uv_rect,
                    control: flip_flag(through_flip),
                    tint,
                    clip0: clip_plane_render(&frame, exit.pos, exit.normal),
                    clip1: clip_plane_render(&frame, exit.pos - along * aperture_half, along),
                    clip2: clip_plane_render(&frame, exit.pos + along * aperture_half, -along),
                    color_texture: sprite.image.clone(),
                })),
                clip_piece_transform(&through_base, through_anchor, basis.size),
                Name::new("Portal body piece (through)"),
            ));

            // The pieces ARE the body this frame — the whole real sprite would
            // re-add the sunk slice (the pop this path exists to remove).
            *visibility = Visibility::Hidden;
            drew_clipped = true;
        }
    }

    if !drew_clipped {
        // Fallback (texture not loaded / headless host): visible real sprite +
        // unclipped whole-sprite exit copy, just BELOW the view window — an
        // open window captures the copy on the far side (one seamless body)
        // and hides the redundant world draw behind itself; a closed window
        // leaves it as the emerging-body visual over the rim. See
        // [`crate::PORTAL_EXIT_COPY_Z`].
        let mut exit_sprite = sprite.clone();
        exit_sprite.flip_x = through_flip;
        let exit_translation = frame.to_render(exit_center, crate::PORTAL_EXIT_COPY_Z);
        let exit_transform = Transform::from_translation(exit_translation)
            .with_rotation(Quat::from_rotation_z(exit_roll))
            .with_scale(source_transform.scale);
        commands.spawn((
            PortalBodyPiece,
            exit_sprite,
            exit_transform,
            Anchor(through_anchor),
            Name::new("Portal body copy (exit)"),
        ));
    }

    // Legacy Transit Masks effect: opaque boxes over the invisible slice of
    // each chart — the part of the real sprite sunk through the entry plane,
    // and the part of the exit copy that has not yet emerged.
    #[cfg(feature = "effect_transit_masks")]
    if selection.active == crate::PortalVisualEffect::TransitMasks {
        let mask_color = Color::srgb(0.80, 0.95, 1.0);
        let mask_z = ae::config::WORLD_Z_PLAYER + 1.0;
        // Entry mask: the slice that has sunk THROUGH the entry plane.
        if let Some(hidden) = pp::clip_halfspace(body, enter.pos, -enter.normal) {
            let translation = frame.to_render(hidden.center(), mask_z);
            commands.spawn((
                PortalBodyPiece,
                Sprite::from_color(mask_color, hidden.half_size() * 2.0),
                Transform::from_translation(translation),
                Name::new("Portal mask (entry, through-wall)"),
            ));
        }
        // Exit mask: the slice of the exit copy that has NOT yet emerged.
        let exit_body = pp::map_aabb(body, &enter, &exit);
        if let Some(hidden) = pp::clip_halfspace(exit_body, exit.pos, -exit.normal) {
            let translation = frame.to_render(hidden.center(), mask_z);
            commands.spawn((
                PortalBodyPiece,
                Sprite::from_color(mask_color, hidden.half_size() * 2.0),
                Transform::from_translation(translation),
                Name::new("Portal mask (exit, not-yet-emerged)"),
            ));
        }
    }
}

/// Colored quad per portal so linked apertures are legible. Clear-and-rebuild
/// each frame — portal counts are expected to stay small in ordinary rooms, and
/// rebuilding from sim entities avoids presentation drift.
///
/// FIXME(portal-api): this visual is intentionally simple and currently assumes
/// a 2D side-profile doorway. The data model should be ready for authored,
/// runtime-opened, moving, and eventually non-axis-aligned portals, with richer
/// renderers allowed to replace this system.
pub fn sync_portal_visuals(
    mut commands: Commands,
    frame: Res<PortalWorldFrame>,
    art: Option<Res<PortalGunArt>>,
    visuals: Query<Entity, With<PortalVisual>>,
    portals: Query<&PlacedPortal>,
    pickups: Query<&PortalGunPickup>,
    projectiles: Query<&PortalShot>,
) {
    for entity in &visuals {
        commands.entity(entity).despawn();
    }
    gun_visuals::spawn_portal_shot_visuals(&mut commands, &frame, &projectiles);
    gun_visuals::spawn_portal_gun_pickup_visuals(&mut commands, &frame, art.as_deref(), &pickups);
    let all_portals: Vec<PlacedPortal> = portals.iter().copied().collect();
    for portal in &all_portals {
        let partner = find_portal(&all_portals, portal.channel.partner());
        // Draw this portal's OWN channel on the side its normal points toward,
        // and the paired channel on the back side. That makes every individual
        // aperture read the same way: the front/entering side is named by the
        // portal's own color, regardless of pair name ordering.
        let negative_channel = partner.map_or(portal.channel, |partner| partner.channel);
        let positive_channel = portal.channel;
        // A portal is a thin doorway seen in side profile (2D): a bar lying
        // ALONG the wall (perpendicular to the surface normal), thin in the
        // normal direction. `along` rotates with the normal, so a slanted
        // surface yields a slanted portal for free.
        let n = portal.normal.normalize_or_zero();
        let along = Vec2::new(-n.y, n.x);
        // Opening half-length = the portal extent projected onto the wall
        // direction: a wall portal (horizontal normal) shows its full height,
        // a floor / ceiling portal shows its width.
        let opening_half =
            along.x.abs() * portal.half_extent.x + along.y.abs() * portal.half_extent.y;
        let length = (opening_half * 2.0).max(PORTAL_VISUAL_THICKNESS);
        // World is y-down, render space is y-up — flip y to get the on-screen
        // direction of the bar's long axis, then rotate the sprite to match.
        let angle = (-along.y).atan2(along.x);
        let rotation = Quat::from_rotation_z(angle);
        // Rim (outer) + brighter thin core, both split into pair-colored halves.
        // Split ACROSS the portal face (along the normal), not along the portal's
        // long axis. For a wall portal this gives left/right halves instead of
        // top/bottom halves, so the color sheet that the actor enters lines up
        // with the mapped exit-side portal texture. The positive-normal side
        // is this portal's own channel; the negative-normal side is its partner.
        for (channel, sign, side) in [
            (negative_channel, -1.0, "negative-normal"),
            (positive_channel, 1.0, "positive-normal"),
        ] {
            let (rim, core) = channel.display();
            let rim_thickness = PORTAL_VISUAL_THICKNESS;
            let rim_center = portal.pos + n * (sign * rim_thickness * 0.25);
            let rim_translation = frame.to_render(rim_center, 9.0);
            commands.spawn((
                PortalVisual,
                Sprite::from_color(rim, Vec2::new(length, rim_thickness * 0.5)),
                Transform::from_translation(rim_translation).with_rotation(rotation),
                Name::new(format!("Portal visual (rim {side})")),
            ));

            let core_length = length * 0.86;
            let core_thickness = PORTAL_VISUAL_THICKNESS * 0.42;
            let core_center = portal.pos + n * (sign * core_thickness * 0.25);
            let core_translation = frame.to_render(core_center, 9.1);
            commands.spawn((
                PortalVisual,
                Sprite::from_color(core, Vec2::new(core_length, core_thickness * 0.5)),
                Transform::from_translation(core_translation).with_rotation(rotation),
                Name::new(format!("Portal visual (core {side})")),
            ));
        }
        // A small color-name label just out in front of the face, so portals can
        // be referred to precisely (each linked pair is a distinct complementary
        // color: purple↔yellow, teal↔red, …). The color name IS the identifier.
        let label_pos = portal.pos + n * 24.0;
        let label_translation = frame.to_render(label_pos, 9.2);
        let (_, core) = portal.channel.display();
        commands.spawn((
            PortalVisual,
            Text2d::new(portal.channel.name()),
            TextFont {
                font_size: 12.0,
                ..default()
            },
            TextColor(core),
            Transform::from_translation(label_translation),
            Name::new("Portal label"),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_portal::PortalChannelColor;
    use bevy::sprite_render::MeshMaterial2d;

    const WORLD: Vec2 = Vec2::new(1000.0, 600.0);

    /// A thin-wall door pair (the c136/c137 shape): two wall portals 32px
    /// apart on opposite faces, apertures aligned.
    fn thin_wall_pair() -> (PlacedPortal, PlacedPortal) {
        let left = PlacedPortal {
            channel: ambition_portal::PortalChannel::Authored(PortalChannelColor::Purple),
            pos: Vec2::new(500.0, 300.0),
            normal: Vec2::new(-1.0, 0.0),
            half_extent: Vec2::new(9.0, 46.0),
        };
        let right = PlacedPortal {
            channel: ambition_portal::PortalChannel::Authored(PortalChannelColor::Yellow),
            pos: Vec2::new(532.0, 300.0),
            normal: Vec2::new(1.0, 0.0),
            half_extent: Vec2::new(9.0, 46.0),
        };
        (left, right)
    }

    fn test_app() -> App {
        let mut app = App::new();
        app.insert_resource(PortalWorldFrame { size: WORLD });
        app.insert_resource(crate::PortalEffectSelection::default());
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
        let handle = app
            .world_mut()
            .resource_mut::<Assets<Image>>()
            .add(image);
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
        // portal (engine x = 530 → render x = 30), at the actor z.
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
        assert!(
            transforms.iter().all(|t| (t.z - 20.0).abs() < 1e-3),
            "pieces draw in the actor band, got {transforms:?}"
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
}
