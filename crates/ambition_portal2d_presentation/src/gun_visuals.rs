//! Compatibility visuals for Ambition's portal-gun workflow.
//!
//! TODO(compat-remove): move gun-specific presentation to the Ambition host and delete this
//! module from the generic portal presentation crate.

use bevy::prelude::*;
use bevy::sprite_render::MeshMaterial2d;

use ambition_platformer2d_core as ae;
use ambition_portal2d::pieces as pp;
use ambition_portal2d::{
    find_portal, PlacedPortal, PortalGun, PortalGunPickup, PortalShot, PortalTransit,
};

use crate::clip_material::{
    clip_piece_transform, clip_plane_render, sprite_frame_basis, PortalClipMaterial, CLIP_PLANE_OFF,
};
use crate::{
    PortalAffordanceBody, PortalAimHint, PortalBodyView, PortalGunArt, PortalVisual,
    PortalWorldFrame,
};

/// Marks the held portal-gun sprite carried by the current controlled actor.
#[derive(Component)]
pub struct PortalModeIndicator;

/// On-screen size of the portal-gun sprite, used for BOTH the held gun and the
/// ground pickup so it doesn't change size when picked up (keeps the 140x64
/// sprite aspect approximately 2.19).
const PORTAL_GUN_DISPLAY: Vec2 = Vec2::new(52.0, 24.0);

/// Draw the portal-gun sprite in the current controlled actor's hand, rotated
/// to point where the host says the portal opener is aiming. This is a gun UI
/// affordance, not part of portal topology or transit math.
///
/// During transit the gun follows the body's chart decomposition: one clipped
/// copy per chart on the main-camera layer. Without clip assets, use the
/// single-gun fallback.
pub fn sync_portal_mode_indicator(
    mut commands: Commands,
    aim_hint: Option<Res<PortalAimHint>>,
    frame: Res<PortalWorldFrame>,
    art: Option<Res<PortalGunArt>>,
    visuals: Query<Entity, With<PortalModeIndicator>>,
    portals: Query<&PlacedPortal>,
    images: Option<Res<Assets<Image>>>,
    layouts: Option<Res<Assets<bevy::image::TextureAtlasLayout>>>,
    meshes: Option<ResMut<Assets<Mesh>>>,
    clip_materials: Option<ResMut<Assets<PortalClipMaterial>>>,
    mut unit_mesh: Local<Option<Handle<Mesh>>>,
    carriers: Query<
        (&PortalBodyView, &PortalGun, Option<&PortalTransit>),
        With<PortalAffordanceBody>,
    >,
    tuning: Option<Res<ambition_portal2d::PortalTuning>>,
) {
    // The session's portal map convention, from the resource that owns it — not
    // from a process global. `Option` because a composition without the portal
    // plugin has no tuning, and the default convention is the honest answer.
    let convention = tuning
        .as_deref()
        .map(|tuning| tuning.convention.map_convention())
        .unwrap_or_default();
    for entity in &visuals {
        commands.entity(entity).despawn();
    }
    let Ok((kin, gun, transit)) = carriers.single() else {
        return;
    };
    if !gun.active {
        return;
    }
    let Some(art) = art else {
        return;
    };
    // A gun owns one pair and toggles between its two ends, and we have exactly
    // two held-gun arts — so the art tracks the END: B shows "orange", A shows
    // "blue". A gun on another pair reuses the same two arts; the placed portals
    // carry the per-pair display colour themselves, which is where the pair is
    // actually legible.
    let image = if gun.next_color.is_end_b() {
        art.orange.clone()
    } else {
        art.blue.clone()
    };
    let facing = if kin.facing >= 0.0 { 1.0 } else { -1.0 };
    // In the carrier's hand: just in front of the body at roughly hand height
    // (y-down world, so a small +y is slightly below centre). z=12 keeps it
    // in front of the actor sprite.
    let pos = kin.pos + Vec2::new(facing * (kin.size.x * 0.45 + 6.0), kin.size.y * 0.06);
    // Aim the barrel where the shot will go. The host supplies a resolved
    // world-space aim through `PortalAimHint` so presentation does not import a
    // concrete input type; zero / unset aim falls back to facing.
    let hinted = aim_hint.as_deref().map_or(Vec2::ZERO, |h| h.aim);
    let aim = if hinted.length() > 0.0 {
        hinted
    } else {
        Vec2::new(if kin.facing >= 0.0 { 1.0 } else { -1.0 }, 0.0)
    }
    .normalize_or_zero();

    // Mid-transit with a through slice: the gun exists in both charts, like
    // the body. Decompose against the same pair, from the same Core function.
    if let Some(transit) = transit {
        let all: Vec<PlacedPortal> = portals.iter().cloned().collect();
        if let (Some(enter_portal), Some(exit_portal)) = (
            find_portal(&all, transit.straddling),
            find_portal(&all, transit.straddling.partner()),
        ) {
            let body = ae::Aabb::new(kin.pos, kin.size * 0.5);
            let pieces = pp::compute_body_pieces(
                body,
                Some((enter_portal.aperture(), exit_portal.aperture())),
                convention,
            );
            if pieces.through.is_some() {
                if let (Some(images), Some(layouts), Some(mut meshes), Some(mut materials)) =
                    (images, layouts, meshes, clip_materials)
                {
                    let probe = Sprite {
                        image: image.clone(),
                        custom_size: Some(PORTAL_GUN_DISPLAY),
                        ..default()
                    };
                    if let Some(basis) = sprite_frame_basis(&probe, &layouts, &images) {
                        let (enter, exit) = (enter_portal.aperture(), exit_portal.aperture());
                        let mesh = unit_mesh
                            .get_or_insert_with(|| meshes.add(Rectangle::default()))
                            .clone();
                        let along = exit.frame.tangent();
                        let aperture_half = exit.half_length;
                        // The through chart: map the gun's world point and the
                        // aim vector through the pair — exact under the
                        // isometry, no facing/offset re-derivation.
                        let charts = [
                            (
                                "here",
                                12.0,
                                pos,
                                aim,
                                clip_plane_render(&frame, enter.frame.origin, enter.frame.normal),
                                CLIP_PLANE_OFF,
                                CLIP_PLANE_OFF,
                            ),
                            (
                                "through",
                                crate::PORTAL_EXIT_COPY_Z + 0.05,
                                pp::map_point(pos, &enter.frame, &exit.frame, convention),
                                pp::portal_map_vec(
                                    aim,
                                    enter.frame.normal,
                                    exit.frame.normal,
                                    convention,
                                ),
                                clip_plane_render(&frame, exit.frame.origin, exit.frame.normal),
                                clip_plane_render(
                                    &frame,
                                    exit.frame.origin - along * aperture_half,
                                    along,
                                ),
                                clip_plane_render(
                                    &frame,
                                    exit.frame.origin + along * aperture_half,
                                    -along,
                                ),
                            ),
                        ];
                        for (chart, chart_z, chart_pos, chart_aim, clip0, clip1, clip2) in charts {
                            let angle = (-chart_aim.y).atan2(chart_aim.x);
                            let base = Transform {
                                translation: frame.to_render(chart_pos, chart_z),
                                rotation: Quat::from_rotation_z(angle),
                                scale: Vec3::ONE,
                            };
                            let flip_y = chart_aim.x < 0.0;
                            commands.spawn((
                                PortalModeIndicator,
                                Mesh2d(mesh.clone()),
                                MeshMaterial2d(materials.add(PortalClipMaterial {
                                    uv_rect: basis.uv_rect,
                                    control: Vec4::new(
                                        0.0,
                                        if flip_y { 1.0 } else { 0.0 },
                                        0.0,
                                        0.0,
                                    ),
                                    // ⚠ UNTINTED, and a non-default gun is
                                    // its authored colour for these frames.
                                    // This material MULTIPLIES, and a hue
                                    // rotation is not a multiply — writing the
                                    // pair colour here would darken the gun
                                    // rather than recolour it. Mid-transit is a
                                    // few frames; a wrong colour would be worse
                                    // than the authored one.
                                    tint: Vec4::ONE,
                                    clip0,
                                    clip1,
                                    clip2,
                                    color_texture: image.clone(),
                                })),
                                clip_piece_transform(&base, Vec2::ZERO, basis.size),
                                Name::new(format!("Held portal gun ({chart})")),
                            ));
                        }
                        return;
                    }
                }
            }
        }
    }

    let angle = (-aim.y).atan2(aim.x);
    let held = commands
        .spawn((
        PortalModeIndicator,
        Sprite {
            image,
            custom_size: Some(PORTAL_GUN_DISPLAY),
            flip_y: aim.x < 0.0,
            ..default()
        },
        Transform::from_translation(frame.to_render(pos, 12.0))
            .with_rotation(Quat::from_rotation_z(angle)),
        Name::new("Held portal gun"),
        ))
        .id();
    // ⭐ THE GUN IN THE HAND SHOWS WHICH GUN IT IS. Two authored arts, any
    // number of pairs — so a gun on a non-default pair is the SAME drawing
    // rotated onto its own colour, keeping shading, highlights and antialiased
    // edges that a multiply would have flattened.
    //
    // ⛔ ONLY WHEN THERE IS A ROTATION. A `HueShift` of 0.0 is not free: every
    // shader effect draws through a material, so attaching one unconditionally
    // would move the DEFAULT blue/orange gun — the one almost every session
    // holds — off the batched sprite path to compute a rotation by nothing.
    let shift = gun.next_color.art_hue_shift();
    if shift != 0.0 {
        commands
            .entity(held)
            .insert(ambition_sprite_fx::SpriteEffect::HueShift { degrees: shift });
    }
}

/// Draw in-flight portal-gun shots. Sequestered from portal aperture visuals so
/// a non-gun host can replace or omit this without touching portal rendering.
pub(crate) fn spawn_portal_shot_visuals(
    commands: &mut Commands,
    frame: &PortalWorldFrame,
    projectiles: &Query<&PortalShot>,
) {
    for proj in projectiles.iter() {
        let color = proj.channel.display().1;
        let translation = frame.to_render(proj.pos, 9.5);
        commands.spawn((
            PortalVisual,
            Sprite::from_color(color, Vec2::new(16.0, 8.0)),
            Transform::from_translation(translation),
            Name::new("Portal shot visual"),
        ));
    }
}

/// Draw uncollected portal-gun pickups. This is compatibility presentation for
/// Ambition's current gun acquisition loop, not a requirement for portal use.
pub(crate) fn spawn_portal_gun_pickup_visuals(
    commands: &mut Commands,
    frame: &PortalWorldFrame,
    art: Option<&PortalGunArt>,
    pickups: &Query<&PortalGunPickup>,
) {
    for pickup in pickups.iter() {
        let translation = frame.to_render(pickup.pos, 9.0);
        // The world pickup shows the actual gun sprite (blue mode by default);
        // falls back to a marker quad before the art has loaded.
        let sprite = match art {
            Some(art) => Sprite {
                image: art.blue.clone(),
                // Same display size as the held gun so it doesn't visibly
                // resize when picked up.
                custom_size: Some(PORTAL_GUN_DISPLAY),
                ..default()
            },
            None => Sprite::from_color(Color::srgb(0.66, 0.36, 0.92), pickup.half_extent * 2.0),
        };
        let dropped = commands
            .spawn((
                PortalVisual,
                sprite,
                Transform::from_translation(translation),
                Name::new("Portal gun pickup visual"),
            ))
            .id();
        // A gun on the floor is the gun you will be holding, so it wears its
        // pair's colour too — otherwise every pickup looks like the default one
        // and the player only learns which gun it was after taking it.
        // Rotating by nothing costs a material, so pair 0 stays a plain sprite.
        let shift = ambition_portal2d::PortalGunColor::for_pair(pickup.pair).art_hue_shift();
        if shift != 0.0 {
            commands
                .entity(dropped)
                .insert(ambition_sprite_fx::SpriteEffect::HueShift { degrees: shift });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_portal2d::PortalChannel;
    use bevy::image::TextureAtlasLayout;

    /// Mid-transit the held gun draws once per chart as clip-material quads
    /// (like the body pieces) instead of a single sprite at the authoritative
    /// pose — the single gun visibly SNAPPED by the pair separation at the
    /// centroid crossing while the body slices stayed continuous.
    #[test]
    fn transiting_carrier_gun_decomposes_into_two_clipped_charts() {
        let mut app = App::new();
        app.insert_resource(PortalWorldFrame {
            size: Vec2::new(1000.0, 600.0),
        });
        app.insert_resource(Assets::<Image>::default());
        app.insert_resource(Assets::<TextureAtlasLayout>::default());
        app.insert_resource(Assets::<Mesh>::default());
        app.insert_resource(Assets::<PortalClipMaterial>::default());
        app.add_systems(Update, sync_portal_mode_indicator);

        let mut image = Image::default();
        image.texture_descriptor.size.width = 140;
        image.texture_descriptor.size.height = 64;
        let handle = app.world_mut().resource_mut::<Assets<Image>>().add(image);
        app.insert_resource(PortalGunArt {
            blue: handle.clone(),
            orange: handle,
        });

        let left = PlacedPortal::fixed(
            PortalChannel::Authored(ambition_portal2d::PortalChannelColor::Purple),
            Vec2::new(500.0, 300.0),
            Vec2::new(-1.0, 0.0),
            Vec2::new(9.0, 46.0),
        );
        let right = PlacedPortal::fixed(
            PortalChannel::Authored(ambition_portal2d::PortalChannelColor::Yellow),
            Vec2::new(532.0, 300.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(9.0, 46.0),
        );
        app.world_mut().spawn(left.clone());
        app.world_mut().spawn(right);
        app.world_mut().spawn((
            PortalAffordanceBody,
            PortalBodyView {
                pos: Vec2::new(498.0, 300.0),
                size: Vec2::new(24.0, 40.0),
                facing: 1.0,
            },
            PortalGun::default(),
            PortalTransit {
                straddling: left.channel,
                crossed: false,
            },
        ));
        app.update();

        let guns: Vec<(String, bool)> = app
            .world_mut()
            .query_filtered::<(&Name, Has<Mesh2d>), With<PortalModeIndicator>>()
            .iter(app.world())
            .map(|(n, m)| (n.to_string(), m))
            .collect();
        assert_eq!(guns.len(), 2, "one gun copy per chart, got {guns:?}");
        assert!(
            guns.iter().all(|(_, mesh)| *mesh),
            "both copies are clip-material quads, got {guns:?}"
        );

        // Without a transit, exactly one plain-sprite gun.
        let player = app
            .world_mut()
            .query_filtered::<Entity, With<PortalGun>>()
            .single(app.world())
            .unwrap();
        app.world_mut().entity_mut(player).remove::<PortalTransit>();
        app.update();
        let guns = app
            .world_mut()
            .query_filtered::<Has<Mesh2d>, With<PortalModeIndicator>>()
            .iter(app.world())
            .collect::<Vec<_>>();
        assert_eq!(guns, vec![false], "no transit: one plain sprite gun");
    }
}
