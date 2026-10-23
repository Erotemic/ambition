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

use ambition_platformer2d_core as ae;
use ambition_platformer2d_shared_tangle::orientation::ActorRoll;

use ae::frame::thread::{partition, PieceBudget, PortalLink, VolumePiece};
use ambition_portal2d::{find_portal, PlacedPortal, PortalGunPickup, PortalInputWarp, PortalShot};

use crate::clip_material::{
    clip_piece_transform, clip_plane_render, sprite_frame_basis, ClipQuadPool, PortalClipMaterial, CLIP_PLANES,
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
    // The whole-sprite copies of the fallback, remade each frame.
    copies: Query<Entity, (With<PortalBodyPiece>, With<Sprite>)>,
    // The clipped quads, kept from frame to frame ([`ClipQuadPool`]).
    quads: Query<(), (With<PortalBodyPiece>, With<Mesh2d>)>,
    portals: Query<(Entity, &PlacedPortal)>,
    images: Option<Res<Assets<Image>>>,
    layouts: Option<Res<Assets<TextureAtlasLayout>>>,
    mut meshes: Option<ResMut<Assets<Mesh>>>,
    mut clip_materials: Option<ResMut<Assets<PortalClipMaterial>>>,
    mut unit_mesh: Local<Option<Handle<Mesh>>>,
    piece_quad: Option<Res<crate::clip_material::PortalPieceQuad>>,
    // The bodies whose pieces were drawn last frame (`PortalBodyView::body`).
    mut drawn_last_frame: Local<Vec<Entity>>,
    mut pool: Local<ClipQuadPool>,
    body_visuals: Query<
        (
            Entity,
            &crate::PortalBodyView,
            Option<&crate::PortalTransitView>,
            Option<&crate::PortalHostReach>,
            Option<&ActorRoll>,
            &Sprite,
            Option<&Anchor>,
            &Transform,
            // The portal hides this body already (the far-side compositor
            // draws it by its pieces).
            Has<crate::source_visibility::PortalSourceHidden>,
        ),
        With<PortalSceneBody>,
    >,
    // The session's portal map convention, from the resource that owns it.
    tuning: Option<Res<ambition_portal2d::PortalTuning>>,
    // How far the bodies of a room are cut (the room's portal policy).
    passages: Option<Res<ambition_platformer2d_shared_tangle::portal_passages::PortalPassages>>,
) {
    let convention = tuning
        .as_deref()
        .map(|tuning| tuning.convention.map_convention())
        .unwrap_or_default();
    for entity in &copies {
        commands.entity(entity).despawn();
    }
    pool.begin(|entity| quads.contains(entity));
    let drawn_before = std::mem::take(&mut *drawn_last_frame);
    let by_room = frames.portals_by_room(portals.iter());
    for (source_body, kin, transit, reach, roll, sprite, source_anchor, source_transform, portal_hidden) in &body_visuals {
        let reach = reach.map(|reach| reach.0);
        // Outside transit the real character sprite shows whole; the pieces are a
        // transit-only replacement. Withdraw the reason instead of writing
        // `Visibility`, so the far-side compositor's hide is not overruled:
        // `resolve_portal_source_visibility` shows the body only when no reason
        // remains.
        commands
            .entity(source_body)
            .remove::<crate::source_visibility::PortalTransitHidden>();
        let Some(transit) = transit else {
            continue;
        };
        // The body's own room: its frame, and the pairs it is cut by in it.
        let Some(placement) = frames.of(source_body) else {
            continue;
        };
        let frame = placement.frame;
        let all = by_room.in_room(Some(placement.room));
        let Some(straddled) = find_portal(all, transit.straddling) else {
            continue;
        };
        // Every pair of the room, both ways; the pair the body straddles cuts
        // it first. A host's picture is cut by its limb's pair only, as deep
        // as the limb reaches.
        let mut links: Vec<PortalLink> = ambition_portal2d::linked_pairs(all)
            .into_iter()
            .enumerate()
            .flat_map(|(index, pair)| PortalLink::both_ways(index, pair))
            .collect();
        let Some(first) = links.iter().find(|link| link.enter == straddled.aperture()).map(|link| link.key) else {
            continue;
        };
        let budget = match reach {
            Some(depth) => {
                links.retain(|link| link.key == first);
                links[0].depth = Some(depth);
                PieceBudget::SINGLE_HOP
            }
            None => match passages.as_deref() {
                Some(passages) => match passages.budget_for(Some(kin.body), Some(placement.room)) {
                    Some(budget) => budget,
                    // The body is moved whole: there are no pieces.
                    None => continue,
                },
                None => PieceBudget::SINGLE_HOP,
            },
        };
        // A measurement trace (`RUST_LOG=ambition_portal::piece_trace=debug`,
        // read by `scripts/portal_seam_lab.py`): what each straddling visual
        // was drawn as this frame.
        let trace = |outcome: &str| {
            bevy::log::debug!(
                target: "ambition_portal::piece_trace",
                "piece body={:?} at={:.1},{:.1} straddling={} {}",
                source_body,
                kin.pos.x,
                kin.pos.y,
                transit.straddling.name(),
                outcome
            );
        };
        // A body that straddles is drawn by its pieces from the frame it
        // touches the portal, while nothing has crossed yet: then the near
        // piece is all of it. A piece is not drawn on the frame its quad is
        // first made (its material is not ready for the renderer), so the
        // body's own sprite is hidden only once its pieces were drawn the frame
        // before; until then it shows, whole. Cut on the frame it first
        // crossed, the first body of a session vanished for that frame
        // (measured with the seam lab, every pair, every speed). A body the
        // portal hides already is drawn by pieces now (the far-side
        // compositor's), and stays hidden.
        let pieces_were_drawn = drawn_before.contains(&kin.body) || portal_hidden;
        // Where the sprite is drawn, and not where the body's centre is: a
        // sprite can be drawn off its body (a foot anchor, a presented pose),
        // and each piece is the image of the part of the picture it holds.
        let drawn_at = ae::config::bevy_size_to_world(frame.size, source_transform.translation.truncate());
        // The roll the host states, and for a body with none stated, the roll
        // its sprite is drawn with.
        let base_roll = roll.map_or_else(|| source_transform.rotation.to_euler(EulerRot::ZYX).0, |r| r.angle);
        let source_anchor_v = source_anchor.map_or(Vec2::ZERO, |a| a.0);
        // The pose of the sprite in a piece's chart: the piece's map (one
        // pair, or several composed) applied to the sprite as drawn. A map
        // that mirrors toggles the sprite's flip (and its anchor with it, or a
        // trimmed frame is drawn from the wrong basis) and turns its roll the
        // other way.
        let pose_in = |piece: &VolumePiece, z: f32| {
            let copy = ambition_portal2d::copy_transform_of(&piece.to_piece);
            let mut anchor = source_anchor_v;
            if copy.flip_x {
                anchor.x = -anchor.x;
            }
            let base = Transform {
                translation: frame.to_render(piece.carry(drawn_at), z),
                rotation: Quat::from_rotation_z(copy.rolled(base_roll)),
                scale: source_transform.scale,
            };
            (base, sprite.flip_x != copy.flip_x, anchor)
        };

        // Texture-clipped piece path: each piece as a clip-material quad, on
        // the WORLD layer so portal captures photograph them: through a
        // pair's window you see your own copy emerging. A piece in the body's
        // own chart draws at the body's z. A carried piece sits just below the
        // window band: where the pane covers its region, the pane's captured
        // copy is the one image shown, joined to the near piece at the seam;
        // where no pane covers it, the direct draw shows. This is so for each
        // pair, a door through a thin wall included
        // (`view_cones::geometry::compute_cone`).
        let mut drew_clipped = false;
        let mut through = false;
        if let (Some(images), Some(layouts), Some(meshes), Some(materials)) =
            (images.as_deref(), layouts.as_deref(), meshes.as_deref_mut(), clip_materials.as_deref_mut())
        {
            if let Some(basis) = sprite_frame_basis(sprite, layouts, images) {
                let mesh = match piece_quad.as_deref() {
                    Some(quad) => quad.0.clone(),
                    None => unit_mesh.get_or_insert_with(|| meshes.add(Rectangle::default())).clone(),
                };
                let tint = crate::piece_tint(sprite);
                let flip_flag = |flip: bool| Vec4::new(if flip { 1.0 } else { 0.0 }, 0.0, 0.0, 0.0);
                // What is cut is the picture as drawn: its quad, in the world.
                let quad = clip_piece_transform(source_transform, source_anchor_v, basis.size);
                let corners = [Vec2::new(-0.5, -0.5), Vec2::new(0.5, -0.5), Vec2::new(0.5, 0.5), Vec2::new(-0.5, 0.5)]
                    .map(|c| ae::config::bevy_size_to_world(frame.size, quad.transform_point(c.extend(0.0)).truncate()));
                let partition = partition(
                    &ae::CombatVolume::convex(corners.to_vec()),
                    kin.pos,
                    &links,
                    Some(first),
                    budget,
                    convention,
                );
                through = partition.pieces.iter().any(VolumePiece::is_carried);
                for piece in &partition.pieces {
                    let bounds = piece.active_bounds();
                    if bounds.len() > CLIP_PLANES {
                        bevy::log::warn_once!(
                            "a portal piece is bounded by {} planes; it is drawn clipped by the first {CLIP_PLANES}",
                            bounds.len()
                        );
                    }
                    let planes: Vec<Vec4> =
                        bounds.iter().map(|plane| clip_plane_render(&frame, plane.point, plane.inward)).collect();
                    let (transform, flip, name) = if piece.is_carried() {
                        let (base, flip, anchor) = pose_in(piece, crate::PORTAL_EXIT_COPY_Z);
                        (clip_piece_transform(&base, anchor, basis.size), flip, "Portal body piece (through)")
                    } else {
                        (quad, sprite.flip_x, "Portal body piece (here)")
                    };
                    pool.draw(
                        &mut commands,
                        materials,
                        &mesh,
                        PortalClipMaterial::new(basis.uv_rect, flip_flag(flip), tint, &planes, sprite.image.clone()),
                        transform,
                        (PortalBodyPiece, placement.stamp()),
                        name,
                    );
                }

                // The pieces ARE the body this frame — the whole real sprite would
                // re-add the sunk slice (the pop this path exists to remove).
                if pieces_were_drawn {
                    commands
                        .entity(source_body)
                        .insert(crate::source_visibility::PortalTransitHidden);
                }
                drawn_last_frame.push(kin.body);
                drew_clipped = true;
            }
        }
        if !drew_clipped {
            // No picture to cut: the body's own box says what crossed.
            let body = ae::CombatVolume::aabb(ae::Aabb::new(kin.pos, kin.size * 0.5));
            let carried = partition(&body, kin.pos, &links, Some(first), budget, convention)
                .pieces
                .into_iter()
                .filter(VolumePiece::is_carried)
                .collect::<Vec<_>>();
            through = !carried.is_empty();
            // Fallback (texture not loaded / headless host): visible real sprite +
            // an unclipped whole-sprite copy in each chart something crossed
            // into, just BELOW the view window — an open window captures the
            // copy on the far side (one seamless body) and hides the redundant
            // world draw behind itself; a closed window leaves it as the
            // emerging-body visual over the rim. See [`crate::PORTAL_EXIT_COPY_Z`].
            for piece in &carried {
                let (base, flip, anchor) = pose_in(piece, crate::PORTAL_EXIT_COPY_Z);
                let mut exit_sprite = sprite.clone();
                exit_sprite.flip_x = flip;
                commands.spawn((
                    PortalBodyPiece,
                    exit_sprite,
                    base,
                    Anchor(anchor),
                    placement.stamp(),
                    Name::new("Portal body copy (exit)"),
                ));
            }
        }
        trace(match (drew_clipped, through, pieces_were_drawn) {
            (true, _, false) => "pieces made: the body shows whole",
            (true, true, true) => "cut",
            (true, false, true) => "near piece only: nothing has crossed",
            (false, true, _) => "fallback: whole, copied",
            (false, false, _) => "whole: nothing has crossed",
        });
    }
    pool.end(&mut commands);
}

/// The colour-name label of each portal, and the gun's shot and pickup
/// markers. Rebuilt from the sim entities each frame, and reconciled with what
/// is drawn: a visual whose source wants the same look in the same room is
/// kept and moved, any other is despawned, and what is missing is spawned. A
/// label is a `Text2d`, and a new one is laid out again, so each placed portal
/// was laid out each frame while they were despawned and spawned every frame.
///
/// The portal's own look is its line of light (`crate::glow`), which is kept
/// from frame to frame because it has effects that take time.
pub fn sync_portal_visuals(
    mut commands: Commands,
    frames: PortalFrames,
    art: Option<Res<PortalGunArt>>,
    mut visuals: Query<(Entity, &gun_visuals::PortalVisualDrawn, &mut Transform), With<PortalVisual>>,
    portals: Query<(Entity, &PlacedPortal)>,
    pickups: Query<(Entity, &PortalGunPickup)>,
    projectiles: Query<(Entity, &PortalShot)>,
) {
    use gun_visuals::{PortalVisualDrawn, PortalVisualLook};
    let mut wanted: Vec<(PortalVisualDrawn, Transform)> = Vec::new();
    gun_visuals::want_portal_shot_visuals(&frames, &projectiles, &mut wanted);
    gun_visuals::want_portal_gun_pickup_visuals(&frames, art.is_some(), &pickups, &mut wanted);
    // Each portal in its own live room's frame.
    for (source, portal) in portals.iter() {
        let Some(placement) = frames.of(source) else {
            continue;
        };
        // A small color-name label just out in front of the face, so portals can
        // be referred to precisely (each linked pair is a distinct complementary
        // color: purple↔yellow, teal↔red, …). The color name IS the identifier.
        let n = portal.normal.normalize_or_zero();
        let label_pos = portal.pos + n * 24.0;
        let label_translation = placement.frame.to_render(label_pos, crate::PORTAL_FRAME_Z + 0.05);
        wanted.push((
            PortalVisualDrawn { source, room: placement.room, look: PortalVisualLook::Label(portal.channel) },
            Transform::from_translation(label_translation),
        ));
    }
    // A replaced art is new content: each pickup is drawn again from it.
    let art_replaced = art.as_ref().is_some_and(|art| art.is_changed());
    let mut taken = vec![false; wanted.len()];
    for (entity, drawn, mut transform) in &mut visuals {
        let redraw = art_replaced && matches!(drawn.look, PortalVisualLook::Pickup { .. });
        let kept = (!redraw)
            .then(|| (0..wanted.len()).find(|&at| !taken[at] && wanted[at].0 == *drawn))
            .flatten();
        match kept {
            Some(at) => {
                taken[at] = true;
                transform.set_if_neq(wanted[at].1);
            }
            None => commands.entity(entity).despawn(),
        }
    }
    for ((drawn, transform), taken) in wanted.into_iter().zip(taken) {
        if taken {
            continue;
        }
        match drawn.look {
            PortalVisualLook::Label(channel) => {
                let (_, core) = channel.display();
                commands.spawn((
                    PortalVisual,
                    drawn,
                    Text2d::new(channel.name()),
                    TextFont {
                        font_size: FontSize::Px(12.0),
                        ..default()
                    },
                    TextColor(core),
                    transform,
                    ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance(drawn.room),
                    Name::new("Portal label"),
                ));
            }
            _ => gun_visuals::spawn_gun_visual(&mut commands, drawn, transform, art.as_deref()),
        }
    }
}

#[cfg(test)]
mod tests;
