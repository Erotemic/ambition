//! Compatibility visuals for Ambition's portal-gun workflow.
//!
//! TODO(compat-remove): move gun-specific presentation to the Ambition host and delete this
//! module from the generic portal presentation crate.

use bevy::prelude::*;

use ambition_portal2d::{
    PortalChannel, PortalGun, PortalGunPickup, PortalShot,
};

use crate::carry::PortalDrawnWith;
use crate::{
    PortalAffordanceBody, PortalAimHint, PortalBodyView, PortalFrames, PortalGunArt, PortalVisual,
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
/// The gun belongs to the carrier's drawable family. The shared carry path
/// clips and maps it with every other attachment.
pub fn sync_portal_mode_indicator(
    mut commands: Commands,
    aim_hint: Option<Res<PortalAimHint>>,
    frames: PortalFrames,
    art: Option<Res<PortalGunArt>>,
    mut plain: Query<
        (
            Entity,
            &mut Sprite,
            &mut Transform,
            // Read: `PortalFrames` reads the stamps too. A new room is inserted.
            &ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance,
            Option<&ambition_sprite_fx::SpriteEffect>,
            &PortalDrawnWith,
        ),
        (With<PortalModeIndicator>, Without<Mesh2d>),
    >,
    carriers: Query<
        (Entity, &PortalBodyView, &PortalGun),
        With<PortalAffordanceBody>,
    >,
) {
    let wanted: Option<(Sprite, Transform, ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance, f32, Entity)> = 'draw: {
    let Ok((carrier, kin, gun)) = carriers.single() else {
        break 'draw None;
    };
    if !gun.active {
        break 'draw None;
    }
    // In the carrier's own live room.
    let Some(placement) = frames.of(carrier) else {
        break 'draw None;
    };
    let frame = placement.frame;
    let Some(art) = art else {
        break 'draw None;
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

    let angle = (-aim.y).atan2(aim.x);
    let sprite = Sprite {
        image,
        custom_size: Some(PORTAL_GUN_DISPLAY),
        flip_y: aim.x < 0.0,
        ..default()
    };
    let transform = Transform::from_translation(frame.to_render(pos, 12.0)).with_rotation(Quat::from_rotation_z(angle));
    Some((sprite, transform, placement.stamp(), gun.next_color.art_hue_shift(), carrier))
    };
    let mut existing = plain.iter_mut();
    if let Some((want, transform, room, shift, carrier)) = wanted {
        if let Some((entity, mut sprite, mut drawn, drawn_room, effect, with)) = existing.next() {
            if sprite.image != want.image || sprite.flip_y != want.flip_y {
                sprite.image = want.image;
                sprite.flip_y = want.flip_y;
            }
            drawn.set_if_neq(transform);
            if with.0 != carrier { commands.entity(entity).insert(PortalDrawnWith(carrier)); }
            if *drawn_room != room {
                commands.entity(entity).insert(room);
            }
            // A gun on a non-default pair is the same art hue-rotated onto its
            // own colour; see the spawn below.
            let shifted = match effect {
                Some(ambition_sprite_fx::SpriteEffect::HueShift { degrees }) => *degrees,
                _ => 0.0,
            };
            if shifted != shift {
                if shift != 0.0 {
                    commands.entity(entity).insert(ambition_sprite_fx::SpriteEffect::HueShift { degrees: shift });
                } else {
                    commands.entity(entity).remove::<ambition_sprite_fx::SpriteEffect>();
                }
            }
        } else {
            let held = commands.spawn((PortalModeIndicator, PortalDrawnWith(carrier), want, transform, room, Name::new("Held portal gun"))).id();
            // A gun on a non-default pair is the same art hue-rotated onto its
            // own colour, which keeps shading and antialiased edges. Only add
            // the effect for a non-zero rotation: any shader effect moves the
            // sprite off the batched path, and most sessions hold the default gun.
            if shift != 0.0 {
                commands
                    .entity(held)
                    .insert(ambition_sprite_fx::SpriteEffect::HueShift { degrees: shift });
            }
        }
    }
    for (entity, ..) in existing {
        commands.entity(entity).despawn();
    }
}

/// What a portal visual draws: whose visual it is, in which live room, and
/// what it looks like. A visual is kept, and moved, while its source wants the
/// same look in the same room; any other look is a new visual.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct PortalVisualDrawn {
    pub source: Entity,
    pub room: ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance,
    pub look: PortalVisualLook,
}

/// The look of one portal visual.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PortalVisualLook {
    /// A shot in flight, in its pair's colour.
    Shot(PortalChannel),
    /// A gun on the floor: its art once loaded (`art`), a marker quad of its
    /// size before, in its pair's colour.
    Pickup { pair: u8, art: bool, half_extent: Vec2 },
    /// A portal's colour-name label.
    Label(PortalChannel),
}

/// In-flight portal-gun shots, each with where it is drawn. Sequestered from
/// portal aperture visuals so a non-gun host can replace or omit this without
/// touching portal rendering.
pub(crate) fn want_portal_shot_visuals(
    frames: &PortalFrames,
    projectiles: &Query<(Entity, &PortalShot)>,
    wanted: &mut Vec<(PortalVisualDrawn, Transform)>,
) {
    for (shot, proj) in projectiles.iter() {
        // In the shot's own live room (it carries the room it was fired in).
        let Some(placement) = frames.of(shot) else {
            continue;
        };
        let translation = placement.frame.to_render(proj.pos, 9.5);
        wanted.push((
            PortalVisualDrawn { source: shot, room: placement.room, look: PortalVisualLook::Shot(proj.channel) },
            Transform::from_translation(translation),
        ));
    }
}

/// Uncollected portal-gun pickups, each with where it is drawn. This is
/// compatibility presentation for Ambition's current gun acquisition loop, not
/// a requirement for portal use.
pub(crate) fn want_portal_gun_pickup_visuals(
    frames: &PortalFrames,
    art: bool,
    pickups: &Query<(Entity, &PortalGunPickup)>,
    wanted: &mut Vec<(PortalVisualDrawn, Transform)>,
) {
    for (entity, pickup) in pickups.iter() {
        let Some(placement) = frames.of(entity) else {
            continue;
        };
        let translation = placement.frame.to_render(pickup.pos, 9.0);
        wanted.push((
            PortalVisualDrawn {
                source: entity,
                room: placement.room,
                look: PortalVisualLook::Pickup { pair: pickup.pair, art, half_extent: pickup.half_extent },
            },
            Transform::from_translation(translation),
        ));
    }
}

/// Spawn the visual of a shot or a pickup. A label is spawned by
/// `sync_portal_visuals`.
pub(crate) fn spawn_gun_visual(
    commands: &mut Commands,
    drawn: PortalVisualDrawn,
    transform: Transform,
    art: Option<&PortalGunArt>,
) {
    let stamp = ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance(drawn.room);
    match drawn.look {
        PortalVisualLook::Shot(channel) => {
            commands.spawn((
                PortalVisual,
                drawn,
                Sprite::from_color(channel.display().1, Vec2::new(16.0, 8.0)),
                transform,
                stamp,
                Name::new("Portal shot visual"),
            ));
        }
        PortalVisualLook::Pickup { pair, half_extent, .. } => {
            // The world pickup shows the actual gun sprite (blue mode by
            // default); falls back to a marker quad before the art has loaded.
            let sprite = match art {
                Some(art) => Sprite {
                    image: art.blue.clone(),
                    // Same display size as the held gun so it doesn't visibly
                    // resize when picked up.
                    custom_size: Some(PORTAL_GUN_DISPLAY),
                    ..default()
                },
                None => Sprite::from_color(Color::srgb(0.66, 0.36, 0.92), half_extent * 2.0),
            };
            let dropped = commands
                .spawn((PortalVisual, drawn, sprite, transform, stamp, Name::new("Portal gun pickup visual")))
                .id();
            // A gun on the floor is the gun you will be holding, so it wears its
            // pair's colour too — otherwise every pickup looks like the default
            // one and the player only learns which gun it was after taking it.
            // Rotating by nothing costs a material, so pair 0 stays a plain sprite.
            let shift = ambition_portal2d::PortalGunColor::for_pair(pair).art_hue_shift();
            if shift != 0.0 {
                commands
                    .entity(dropped)
                    .insert(ambition_sprite_fx::SpriteEffect::HueShift { degrees: shift });
            }
        }
        PortalVisualLook::Label(_) => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_gun_stays_attached_to_its_carrier_drawable() {
        let mut app = App::new();
        crate::one_live_room(&mut app, Vec2::new(1000.0, 600.0));
        app.insert_resource(PortalGunArt { blue: Handle::default(), orange: Handle::default() });
        app.add_systems(Update, sync_portal_mode_indicator);
        let carrier = app.world_mut().spawn((PortalAffordanceBody, PortalBodyView {
            pos: Vec2::new(498.0, 300.0), size: Vec2::new(24.0, 40.0),
            canonical_size: Vec2::new(24.0, 40.0), material: Default::default(),
            facing: 1.0, body: Entity::PLACEHOLDER,
        }, PortalGun::default())).id();
        app.update();
        let gun = app.world_mut().query_filtered::<(Entity, &PortalDrawnWith), With<PortalModeIndicator>>()
            .single(app.world()).map(|(entity, with)| { assert_eq!(with.0, carrier); entity }).unwrap();
        app.update();
        assert_eq!(app.world_mut().query_filtered::<Entity, With<PortalModeIndicator>>()
            .single(app.world()).unwrap(), gun);
        app.world_mut().get_mut::<PortalGun>(carrier).unwrap().active = false;
        app.update();
        assert!(app.world().get_entity(gun).is_err());
    }
}
