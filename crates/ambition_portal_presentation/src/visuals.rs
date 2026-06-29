//! The default portal visuals, moved verbatim from the Ambition sandbox's
//! render-gated `portal/presentation.rs` (host types swapped for the crate
//! seams): portal quads + labels, the held / pickup gun sprite, the body-piece
//! decomposition mid-transit, and the disorientation indicator.
//!
//! Every system here is read-only over the portal sim and rebuilds its
//! transient entities each frame, so the visuals can never desync from the sim.

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
    copy_transform, find_portal, PlacedPortal, PortalGun, PortalGunPickup,
    PortalInputWarp, PortalShot, PortalTransit, PORTAL_VISUAL_THICKNESS,
};

use crate::{PortalAimHint, PortalGunArt, PortalSceneBody, PortalWorldFrame};

/// Marks a sprite entity that visualizes a [`PlacedPortal`]. Rebuilt each frame from
/// the sim portals, so it never drifts.
#[derive(Component)]
pub struct PortalVisual;

/// Marks a transient sprite drawing one portal-aware spatial piece of a body
/// mid-transit (the entry-side slice or the exit-side slice). Rebuilt each frame.
#[derive(Component)]
pub struct PortalBodyPiece;

/// Marks the transient "portal disorientation" indicator above the player —
/// visible exactly while the held movement input is portal-warped.
#[derive(Component)]
pub struct PortalDisorientIndicator;

/// Show a small indicator over the player whenever their movement input is
/// portal-warped ([`PortalInputWarp`]) — so the "I'm holding left but moving
/// right" state is legible, and it disappears the instant the warp drops (on
/// release / redirect). Placeholder dot+glyph for now; a nicer effect (incl. on
/// the joystick visual) can replace it later.
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

/// Marks the held portal-gun sprite carried in the player's hand.
#[derive(Component)]
pub struct PortalModeIndicator;

/// On-screen size of the portal-gun sprite, used for BOTH the held gun and the
/// ground pickup so it doesn't change size when you pick it up (keeps the
/// 140×64 sprite aspect ≈ 2.19).
const PORTAL_GUN_DISPLAY: Vec2 = Vec2::new(52.0, 24.0);

/// Draw the portal-gun sprite **in the player's hand**, rotated to point where
/// you're AIMING (the same direction the host fires the portal — like a wielded
/// weapon), tinted to the active mode color so toggling visibly swaps
/// blue↔orange.
pub fn sync_portal_mode_indicator(
    mut commands: Commands,
    aim_hint: Option<Res<PortalAimHint>>,
    frame: Res<PortalWorldFrame>,
    art: Option<Res<PortalGunArt>>,
    visuals: Query<Entity, With<PortalModeIndicator>>,
    players: Query<(&BodyKinematics, &PortalGun), (With<PlayerEntity>, With<PrimaryPlayer>)>,
) {
    for entity in &visuals {
        commands.entity(entity).despawn();
    }
    let Ok((kin, gun)) = players.single() else {
        return;
    };
    if !gun.active {
        return;
    }
    let Some(art) = art else {
        return;
    };
    // The gun cycles through several pairs; we only have two held-gun arts, so
    // the B end of every pair shows the "orange" art and the A end the "blue"
    // art. The placed portals carry the per-pair display colour themselves.
    let image = if gun.next_color.slot & 1 == 1 {
        art.orange.clone()
    } else {
        art.blue.clone()
    };
    let facing = if kin.facing >= 0.0 { 1.0 } else { -1.0 };
    // In the player's hand: just in front of the body at roughly hand height
    // (y-down world, so a small +y is slightly below centre). z=12 keeps it
    // in front of the player sprite.
    let pos = kin.pos + Vec2::new(facing * (kin.size.x * 0.45 + 6.0), kin.size.y * 0.06);
    let translation = frame.to_render(pos, 12.0);
    // Aim the barrel where the shot will go (same aim the host's input adapter
    // resolves for `FirePortalGun`: right-stick aim, else move axis, else
    // facing). World y-down → render y-up; aiming left flips vertically so the
    // gun stays upright rather than upside-down. The aim is supplied by the
    // host via `PortalAimHint` (so portal presentation stays content-agnostic);
    // a zero hint (or no hint) falls back to facing.
    let hinted = aim_hint.as_deref().map_or(Vec2::ZERO, |h| h.aim);
    let aim = if hinted.length() > 0.0 {
        hinted
    } else {
        Vec2::new(if kin.facing >= 0.0 { 1.0 } else { -1.0 }, 0.0)
    }
    .normalize_or_zero();
    let angle = (-aim.y).atan2(aim.x);
    commands.spawn((
        PortalModeIndicator,
        Sprite {
            image,
            custom_size: Some(PORTAL_GUN_DISPLAY),
            flip_y: aim.x < 0.0,
            ..default()
        },
        Transform::from_translation(translation).with_rotation(Quat::from_rotation_z(angle)),
        Name::new("Held portal gun"),
    ));
}

/// Colored quad per portal so the player can actually see them. Clear-and-
/// rebuild each frame — there are at most two portals, so the churn is
/// negligible and the visuals can never desync from the sim entities.
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
    // In-flight portal shots: a small bright streak in the shot's color.
    for proj in &projectiles {
        let color = proj.channel.display().1;
        let translation = frame.to_render(proj.pos, 9.5);
        commands.spawn((
            PortalVisual,
            Sprite::from_color(color, Vec2::new(16.0, 8.0)),
            Transform::from_translation(translation),
            Name::new("Portal shot visual"),
        ));
    }
    // Uncollected portal-gun pickup: a purple marker quad.
    for pickup in &pickups {
        let translation = frame.to_render(pickup.pos, 9.0);
        // The world pickup shows the actual gun sprite (blue mode by default);
        // falls back to a marker quad before the art has loaded.
        let sprite = match art.as_ref() {
            Some(art) => Sprite {
                image: art.blue.clone(),
                // Same display size as the held gun so it doesn't visibly
                // resize when you pick it up.
                custom_size: Some(PORTAL_GUN_DISPLAY),
                ..default()
            },
            None => Sprite::from_color(Color::srgb(0.66, 0.36, 0.92), pickup.half_extent * 2.0),
        };
        commands.spawn((
            PortalVisual,
            sprite,
            Transform::from_translation(translation),
            Name::new("Portal gun pickup visual"),
        ));
    }
    for portal in &portals {
        let (rim, core) = portal.channel.display();
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
        // Rim (outer) + brighter thin core, both oriented along the wall.
        let rim_translation = frame.to_render(portal.pos, 9.0);
        let core_translation = frame.to_render(portal.pos, 9.1);
        commands.spawn((
            PortalVisual,
            Sprite::from_color(rim, Vec2::new(length, PORTAL_VISUAL_THICKNESS)),
            Transform::from_translation(rim_translation).with_rotation(rotation),
            Name::new("Portal visual (rim)"),
        ));
        commands.spawn((
            PortalVisual,
            Sprite::from_color(
                core,
                Vec2::new(length * 0.86, PORTAL_VISUAL_THICKNESS * 0.42),
            ),
            Transform::from_translation(core_translation).with_rotation(rotation),
            Name::new("Portal visual (core)"),
        ));
        // A small color-name label just out in front of the face, so portals can
        // be referred to precisely (each linked pair is a distinct complementary
        // color: purple↔yellow, teal↔red, …). The color name IS the identifier.
        let label_pos = portal.pos + n * 24.0;
        let label_translation = frame.to_render(label_pos, 9.2);
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
