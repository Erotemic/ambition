//! Compatibility visuals for Ambition's portal-gun workflow.
//!
//! These visuals are deliberately kept out of `visuals.rs`, which should remain
//! about portal apertures, body pieces, and other portal-seam presentation. A
//! standalone portal presentation crate should not require a gun: hosts may open
//! portals from authored level data, scripts, moving emitters, or any other
//! control authority.

use bevy::prelude::*;
use bevy::sprite_render::MeshMaterial2d;

use ambition_engine_core as ae;
use ambition_platformer_primitives::body::BodyKinematics;
use ambition_platformer_primitives::markers::{PlayerEntity, PrimaryPlayer};
use ambition_portal::pieces as pp;
use ambition_portal::{find_portal, PlacedPortal, PortalGun, PortalGunPickup, PortalShot, PortalTransit};

use crate::clip_material::{
    clip_piece_transform, clip_plane_render, sprite_frame_basis, PortalClipMaterial,
    CLIP_PLANE_OFF,
};
use crate::{PortalAimHint, PortalGunArt, PortalVisual, PortalWorldFrame};

/// Marks the held portal-gun sprite carried by the current controlled actor.
///
/// FIXME(portal-gun-seam): this still queries Ambition's primary-player marker
/// pair. Keep the dependency isolated here until the host supplies a generic
/// `PortalGunCarrier` / controlled-viewpoint marker seam.
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
/// Mid-transit the gun decomposes exactly like the body it is attached to
/// (see `sync_portal_body_pieces`): one copy per chart, each texture-clipped
/// at its portal plane, on the main-camera-only layer. A single gun drawn at
/// the authoritative pose SNAPPED by the pair separation at the centroid
/// crossing — the one visibly teleporting attachment on an otherwise
/// continuous body (Jon's "body movement has a snap"). Without the clip
/// assets (headless host / texture not yet loaded) the single-gun fallback
/// keeps the old behavior.
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
        (&BodyKinematics, &PortalGun, Option<&PortalTransit>),
        (With<PlayerEntity>, With<PrimaryPlayer>),
    >,
) {
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
    // The gun cycles through several pairs; we only have two held-gun arts, so
    // the B end of every pair shows the "orange" art and the A end the "blue"
    // art. The placed portals carry the per-pair display colour themselves.
    let image = if gun.next_color.slot & 1 == 1 {
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
        let all: Vec<PlacedPortal> = portals.iter().copied().collect();
        if let (Some(enter_portal), Some(exit_portal)) = (
            find_portal(&all, transit.straddling),
            find_portal(&all, transit.straddling.partner()),
        ) {
            let body = ae::Aabb::new(kin.pos, kin.size * 0.5);
            let pieces =
                pp::compute_body_pieces(body, Some((enter_portal.frame(), exit_portal.frame())));
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
                        let (enter, exit) = (enter_portal.frame(), exit_portal.frame());
                        let mesh = unit_mesh
                            .get_or_insert_with(|| meshes.add(Rectangle::default()))
                            .clone();
                        let along = Vec2::new(-exit.normal.y, exit.normal.x);
                        let aperture_half = exit.aperture_half();
                        // The through chart: map the gun's world point and the
                        // aim vector through the pair — exact under the
                        // isometry, no facing/offset re-derivation.
                        let charts = [
                            (
                                "here",
                                12.0,
                                pos,
                                aim,
                                clip_plane_render(&frame, enter.pos, enter.normal),
                                CLIP_PLANE_OFF,
                                CLIP_PLANE_OFF,
                            ),
                            (
                                "through",
                                crate::PORTAL_EXIT_COPY_Z + 0.05,
                                pp::map_point(pos, &enter, &exit),
                                pp::portal_map_vec(aim, enter.normal, exit.normal),
                                clip_plane_render(&frame, exit.pos, exit.normal),
                                clip_plane_render(&frame, exit.pos - along * aperture_half, along),
                                clip_plane_render(&frame, exit.pos + along * aperture_half, -along),
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
    commands.spawn((
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
    ));
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
        commands.spawn((
            PortalVisual,
            sprite,
            Transform::from_translation(translation),
            Name::new("Portal gun pickup visual"),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_portal::PortalChannel;
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

        let left = PlacedPortal {
            channel: PortalChannel::Authored(ambition_portal::PortalChannelColor::Purple),
            pos: Vec2::new(500.0, 300.0),
            normal: Vec2::new(-1.0, 0.0),
            half_extent: Vec2::new(9.0, 46.0),
        };
        let right = PlacedPortal {
            channel: PortalChannel::Authored(ambition_portal::PortalChannelColor::Yellow),
            pos: Vec2::new(532.0, 300.0),
            normal: Vec2::new(1.0, 0.0),
            half_extent: Vec2::new(9.0, 46.0),
        };
        app.world_mut().spawn(left);
        app.world_mut().spawn(right);
        app.world_mut().spawn((
            PlayerEntity,
            PrimaryPlayer,
            BodyKinematics {
                pos: Vec2::new(498.0, 300.0),
                vel: Vec2::ZERO,
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
        let mut transits = app.world_mut().query::<&mut BodyKinematics>();
        drop(transits);
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
