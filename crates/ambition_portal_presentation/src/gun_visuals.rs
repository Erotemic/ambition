//! Compatibility visuals for Ambition's portal-gun workflow.
//!
//! These visuals are deliberately kept out of `visuals.rs`, which should remain
//! about portal apertures, body pieces, and other portal-seam presentation. A
//! standalone portal presentation crate should not require a gun: hosts may open
//! portals from authored level data, scripts, moving emitters, or any other
//! control authority.

use bevy::prelude::*;

use ambition_platformer_primitives::body::BodyKinematics;
use ambition_platformer_primitives::markers::{PlayerEntity, PrimaryPlayer};
use ambition_portal::{PortalGun, PortalGunPickup, PortalShot};

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
pub fn sync_portal_mode_indicator(
    mut commands: Commands,
    aim_hint: Option<Res<PortalAimHint>>,
    frame: Res<PortalWorldFrame>,
    art: Option<Res<PortalGunArt>>,
    visuals: Query<Entity, With<PortalModeIndicator>>,
    carriers: Query<(&BodyKinematics, &PortalGun), (With<PlayerEntity>, With<PrimaryPlayer>)>,
) {
    for entity in &visuals {
        commands.entity(entity).despawn();
    }
    let Ok((kin, gun)) = carriers.single() else {
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
    let translation = frame.to_render(pos, 12.0);
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
