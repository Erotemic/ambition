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

use bevy::prelude::*;
use bevy::sprite::Anchor;

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

/// Draw a second copy of the transiting body emerging from the **exit** portal,
/// posed by the BODY map (`copy_transform`: rotation-only for det +1 maps,
/// rotation plus flip for det -1 maps), so a body straddling a portal shows in
/// BOTH charts (its real sprite at the entry, this copy at the exit). The copy
/// must stay in sync with the real sprite — it is rebuilt each frame from the
/// same `Sprite`, after `sync_visuals` has updated it. The copy is shared by
/// EVERY visual-effect mode (windows / masks / off).
///
/// When the legacy **Transit Masks** effect is the active
/// [`crate::PortalEffectSelection`] (compiled via `effect_transit_masks`),
/// the opaque "feet in, feet out" boxes are drawn over the invisible slice of
/// each chart, like before the view windows existed — kept selectable for
/// A/B profiling against the windows. NOTE: the masking is unfinished
/// (placeholder boxes, not texture clipping); it is a baseline, not a
/// destination.
///
/// Operates on the host-tagged [`PortalSceneBody`] visual entity.
pub fn sync_portal_body_pieces(
    mut commands: Commands,
    #[cfg(feature = "effect_transit_masks")] selection: Res<crate::PortalEffectSelection>,
    frame: Res<PortalWorldFrame>,
    pieces: Query<Entity, With<PortalBodyPiece>>,
    portals: Query<&PlacedPortal>,
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
    // The real character sprite always shows; the exit copy is additive.
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
    // Decompose via the tested Core-invariant function so the copy can never
    // drift from the collision / gameplay decomposition.
    let pieces = pp::compute_body_pieces(body, Some((enter_portal.frame(), exit_portal.frame())));
    let Some(through) = pieces.through else {
        // Touching a portal but nothing has crossed the plane yet.
        return;
    };
    let (enter, exit) = (through.enter, through.exit);
    let base_roll = roll.map_or(0.0, |r| r.angle);

    // Exit copy: the whole sprite emerging from the exit, drawn with the BODY
    // map exactly. The active convention decides whether that map factors as a
    // pure rotation or as rotation plus one x-reflection.
    let exit_center = pp::map_point(kin.pos, &enter, &exit);
    let copy = copy_transform(&enter, &exit);
    let exit_roll = base_roll + copy.roll;
    let mut exit_sprite = sprite.clone();
    let mut exit_anchor = source_anchor.cloned();
    if copy.flip_x {
        // `apply_character_frame` has already mirrored the anchor to match the
        // source sprite's current `flip_x` value. If the portal copy toggles the
        // sprite flip, mirror the anchor too; otherwise trimmed/off-centre frames
        // render from the wrong basis and can look stretched or scaled as the
        // copy emerges.
        exit_sprite.flip_x = !exit_sprite.flip_x;
        if let Some(anchor) = exit_anchor.as_mut() {
            anchor.0.x = -anchor.0.x;
        }
    }
    let exit_translation = frame.to_render(exit_center, ae::config::WORLD_Z_PLAYER);
    let exit_transform = Transform::from_translation(exit_translation)
        .with_rotation(Quat::from_rotation_z(exit_roll))
        .with_scale(source_transform.scale);
    let mut spawned = commands.spawn((
        PortalBodyPiece,
        exit_sprite,
        exit_transform,
        Name::new("Portal body copy (exit)"),
    ));
    if let Some(anchor) = exit_anchor {
        spawned.insert(anchor);
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
        let (negative_channel, positive_channel) = partner.map_or(
            (portal.channel, portal.channel),
            |partner| {
                if portal.channel.name() <= partner.channel.name() {
                    (portal.channel, partner.channel)
                } else {
                    (partner.channel, portal.channel)
                }
            },
        );
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
        // with the mapped exit-side portal texture. The color order remains
        // pair-canonical rather than "this end first."
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
