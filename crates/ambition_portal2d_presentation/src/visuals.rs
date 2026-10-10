//! Default portal-seam visuals: portal labels, mid-transit body-piece
//! decomposition, and the disorientation indicator. A portal's own look is in
//! `glow.rs`.
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

use ambition_platformer2d_core as ae;
use ambition_platformer2d_shared_tangle::orientation::ActorRoll;

use ambition_portal2d::pieces as pp;
use ambition_portal2d::{
    copy_transform, find_portal, PlacedPortal, PortalGunPickup, PortalInputWarp, PortalShot,
};

use crate::clip_material::{
    clip_piece_transform, clip_plane_render, sprite_frame_basis, PortalClipMaterial, CLIP_PLANE_OFF,
};
use crate::{gun_visuals, PortalFrames, PortalGunArt, PortalSceneBody};

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
/// Drawn for the host-tagged [`crate::PortalAffordanceBody`], so this crate
/// never names a "player".
pub fn sync_portal_disorientation_indicator(
    mut commands: Commands,
    frames: PortalFrames,
    existing: Query<Entity, With<PortalDisorientIndicator>>,
    carrier: Query<
        (Entity, &crate::PortalBodyView, Has<PortalInputWarp>),
        With<crate::PortalAffordanceBody>,
    >,
) {
    for entity in &existing {
        commands.entity(entity).despawn();
    }
    let Ok((body, kin, warped)) = carrier.single() else {
        return;
    };
    if !warped {
        return;
    }
    let Some(placement) = frames.of(body) else {
        return;
    };
    // A little spinning-arrow glyph just above the head.
    let pos = kin.pos + Vec2::new(0.0, -(kin.size.y * 0.5 + 16.0));
    let translation = placement.frame.to_render(pos, ae::config::WORLD_Z_PLAYER + 9.0);
    commands.spawn((
        PortalDisorientIndicator,
        placement.stamp(),
        Text2d::new("\u{21BB}"), // ↻ clockwise open circle arrow
        TextFont {
            font_size: FontSize::Px(18.0),
            ..default()
        },
        TextColor(Color::srgb(0.74, 0.92, 1.0)),
        Transform::from_translation(translation),
        Name::new("Portal disorientation indicator"),
    ));
}

/// Because the portal map is an isometry the two slices tile continuously across the seam, so
/// nothing pops when the authoritative position snaps at the centroid crossing, and the sunk slice
/// never draws over the far side of a thin wall (the Q10 crossing flicker). Clipping runs in
/// [`PortalClipMaterial`]'s fragment shader against world positions, so it is exact for any anchor
/// / trim rect / flip / roll. Shared by EVERY visual-effect mode (windows / off).
///
/// Pieces are rebuilt each frame from the same `Sprite`, after the host's
/// animator has updated it, so they can never drift from the real sprite; the
/// decomposition frames come from the tested Core-invariant
/// [`pp::compute_body_pieces`], so they can never drift from collision.
///
/// Fallback: without a loaded texture / atlas layout (or on a headless
/// host that never registered the material — the asset params are `Option`al),
/// the pre-clipping behavior is kept: the real sprite stays visible and an
/// unclipped whole-sprite copy is drawn at the exit just below the view window
/// ([`crate::PORTAL_EXIT_COPY_Z`]), which captures it on the far side and
/// hides the redundant world draw.
///
/// Known gap: sibling overlays of the body sprite (hit-flash silhouette, held
/// gun) are not decomposed; a hit flash mid-transit draws the whole silhouette
/// unclipped for its few frames.
///
/// Operates on each host-tagged [`PortalSceneBody`] visual entity that the
/// host says is straddling a portal ([`crate::PortalTransitView`]): the
/// player, an NPC, a dog. Each body that crosses is cut the same way.
pub fn sync_portal_body_pieces(
    mut commands: Commands,
    frames: PortalFrames,
    pieces: Query<Entity, With<PortalBodyPiece>>,
    portals: Query<(Entity, &PlacedPortal)>,
    images: Option<Res<Assets<Image>>>,
    layouts: Option<Res<Assets<TextureAtlasLayout>>>,
    mut meshes: Option<ResMut<Assets<Mesh>>>,
    mut clip_materials: Option<ResMut<Assets<PortalClipMaterial>>>,
    mut unit_mesh: Local<Option<Handle<Mesh>>>,
    body_visuals: Query<
        (
            Entity,
            &crate::PortalBodyView,
            Option<&crate::PortalTransitView>,
            Option<&ActorRoll>,
            &Sprite,
            Option<&Anchor>,
            &Transform,
        ),
        With<PortalSceneBody>,
    >,
    // The session's portal map convention, from the resource that owns it.
    tuning: Option<Res<ambition_portal2d::PortalTuning>>,
) {
    let convention = tuning
        .as_deref()
        .map(|tuning| tuning.convention.map_convention())
        .unwrap_or_default();
    for entity in &pieces {
        commands.entity(entity).despawn();
    }
    let by_room = frames.portals_by_room(portals.iter());
    for (source_body, kin, transit, roll, sprite, source_anchor, source_transform) in &body_visuals {
        // Outside transit the real character sprite shows whole; the pieces are a
        // transit-only replacement. Withdraw the reason instead of writing
        // `Visibility`, so the far-side compositor's hide is not overruled:
        // `resolve_portal_source_visibility` shows the body only when no reason
        // remains.
        commands
            .entity(source_body)
            .remove::<crate::source_visibility::PortalTransitHidden>();
        // The body is transiting exactly one portal — decompose against that pair.
        let Some(transit) = transit else {
            continue;
        };
        // The body's own room: its frame, and the pair it is crossing in it.
        let Some(placement) = frames.of(source_body) else {
            continue;
        };
        let frame = placement.frame;
        let all = by_room.in_room(Some(placement.room));
        let (Some(enter_portal), Some(exit_portal)) = (
            find_portal(all, transit.straddling),
            find_portal(all, transit.straddling.partner()),
        ) else {
            continue;
        };
        let body = ae::Aabb::new(kin.pos, kin.size * 0.5);
        // Decompose via the tested Core-invariant function so the pieces can never
        // drift from the collision / gameplay decomposition.
        let pieces = pp::compute_body_pieces(
            body,
            Some((enter_portal.aperture(), exit_portal.aperture())),
            convention,
        );
        let Some(through) = pieces.through else {
            // Touching a portal but nothing has crossed the plane yet.
            continue;
        };
        let (enter, exit) = (through.enter, through.exit);
        // The roll the host states, and for a body with none stated, the roll
        // its sprite is drawn with.
        let base_roll = roll.map_or_else(|| source_transform.rotation.to_euler(EulerRot::ZYX).0, |r| r.angle);

        // The through pose: the sprite emerging from the exit, placed by the BODY
        // map exactly. The active convention decides whether that map factors as a
        // pure rotation or as rotation plus one x-reflection.
        // Where the sprite is drawn, and not where the body's centre is: a
        // sprite can be drawn off its body (a foot anchor, a presented pose),
        // and the far slice must be the image of the near one. Placed from the
        // body's centre, the two slices did not meet at the seam by that
        // offset.
        let drawn_at = ae::config::bevy_size_to_world(frame.size, source_transform.translation.truncate());
        let exit_center = pp::map_point(drawn_at, &enter.frame, &exit.frame, convention);
        let copy = copy_transform(&enter.frame, &exit.frame, convention);
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

        // Texture-clipped piece path: both charts as clip-material quads, on the
        // WORLD layer so portal captures photograph them: through a pair's
        // window you see your own copy emerging. The `here` slice draws at the
        // body's z. The `through` slice sits just below the window band: where
        // the pane covers its region, the pane's captured copy is the one
        // image shown, joined to the `here` slice at the seam; where no pane
        // covers it, the direct draw shows. This is so for each pair, a door
        // through a thin wall included (`view_cones::geometry::compute_cone`).
        let mut drew_clipped = false;
        if let (Some(images), Some(layouts), Some(meshes), Some(materials)) =
            (images.as_deref(), layouts.as_deref(), meshes.as_deref_mut(), clip_materials.as_deref_mut())
        {
            if let Some(basis) = sprite_frame_basis(sprite, layouts, images) {
                let mesh = unit_mesh
                    .get_or_insert_with(|| meshes.add(Rectangle::default()))
                    .clone();
                let tint = crate::piece_tint(sprite);
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
                        clip0: clip_plane_render(&frame, enter.frame.origin, enter.frame.normal),
                        clip1: CLIP_PLANE_OFF,
                        clip2: CLIP_PLANE_OFF,
                        color_texture: sprite.image.clone(),
                    })),
                    clip_piece_transform(source_transform, source_anchor_v, basis.size),
                    placement.stamp(),
                    Name::new("Portal body piece (here)"),
                ));

                // `through`: the mapped pose, keeping only what has emerged in
                // front of the exit plane, laterally bounded by the doorway.
                let through_base = Transform {
                    translation: frame.to_render(exit_center, crate::PORTAL_EXIT_COPY_Z),
                    rotation: Quat::from_rotation_z(exit_roll),
                    scale: source_transform.scale,
                };
                let along = Vec2::new(-exit.frame.normal.y, exit.frame.normal.x);
                let aperture_half = exit.half_length;
                commands.spawn((
                    PortalBodyPiece,
                    Mesh2d(mesh),
                    MeshMaterial2d(materials.add(PortalClipMaterial {
                        uv_rect: basis.uv_rect,
                        control: flip_flag(through_flip),
                        tint,
                        clip0: clip_plane_render(&frame, exit.frame.origin, exit.frame.normal),
                        clip1: clip_plane_render(
                            &frame,
                            exit.frame.origin - along * aperture_half,
                            along,
                        ),
                        clip2: clip_plane_render(
                            &frame,
                            exit.frame.origin + along * aperture_half,
                            -along,
                        ),
                        color_texture: sprite.image.clone(),
                    })),
                    clip_piece_transform(&through_base, through_anchor, basis.size),
                    placement.stamp(),
                    Name::new("Portal body piece (through)"),
                ));

                // The pieces ARE the body this frame — the whole real sprite would
                // re-add the sunk slice (the pop this path exists to remove).
                commands
                    .entity(source_body)
                    .insert(crate::source_visibility::PortalTransitHidden);
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
                placement.stamp(),
                Name::new("Portal body copy (exit)"),
            ));
        }
    }
}

/// The colour-name label of each portal, and the gun's shot and pickup
/// markers. Clear-and-rebuild each frame: portal counts stay small in ordinary
/// rooms, and rebuilding from sim entities avoids presentation drift.
///
/// The portal's own look is its line of light (`crate::glow`), which is kept
/// from frame to frame because it has effects that take time.
pub fn sync_portal_visuals(
    mut commands: Commands,
    frames: PortalFrames,
    art: Option<Res<PortalGunArt>>,
    visuals: Query<Entity, With<PortalVisual>>,
    portals: Query<(Entity, &PlacedPortal)>,
    pickups: Query<(Entity, &PortalGunPickup)>,
    projectiles: Query<(Entity, &PortalShot)>,
) {
    for entity in &visuals {
        commands.entity(entity).despawn();
    }
    gun_visuals::spawn_portal_shot_visuals(&mut commands, &frames, &projectiles);
    gun_visuals::spawn_portal_gun_pickup_visuals(&mut commands, &frames, art.as_deref(), &pickups);
    // Each live room's portals, in that room's frame. A portal's partner is in
    // its own room, as the mechanic pairs them.
    let by_room = frames.portals_by_room(portals.iter());
    for (room, room_portals) in by_room.rooms() {
        let Some(placement) = frames.in_room(room) else {
            continue;
        };
        for portal in room_portals {
            // A small color-name label just out in front of the face, so portals can
            // be referred to precisely (each linked pair is a distinct complementary
            // color: purple↔yellow, teal↔red, …). The color name IS the identifier.
            let n = portal.normal.normalize_or_zero();
            let label_pos = portal.pos + n * 24.0;
            let label_translation = placement.frame.to_render(label_pos, crate::PORTAL_FRAME_Z + 0.05);
            let (_, core) = portal.channel.display();
            commands.spawn((
                PortalVisual,
                Text2d::new(portal.channel.name()),
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(core),
                Transform::from_translation(label_translation),
                placement.stamp(),
                Name::new("Portal label"),
            ));
        }
    }
}

#[cfg(test)]
mod tests;
