//! Default portal-seam visuals: portal labels and the disorientation
//! indicator. A portal's own look is in `glow.rs`; what a pair carries is
//! drawn by `carry.rs`.
//!
//! Gun-specific sprites and shot/pickup markers live in `gun_visuals.rs` so the
//! reusable portal presentation surface can move toward static portals, scripted
//! emitters, moving portals, and other non-gun use cases without inheriting
//! Ambition's current portal-gun workflow.
//!
//! Every system here is read-only over the portal sim and rebuilds its transient
//! entities each frame, so visuals cannot desync from the sim.

use bevy::prelude::*;

use ambition_platformer2d_core as ae;

use ambition_portal2d::{PlacedPortal, PortalGunPickup, PortalInputWarp, PortalShot};

use crate::{gun_visuals, PortalFrames, PortalGunArt};

/// Marks a sprite entity that visualizes a [`PlacedPortal`]. Rebuilt each frame from
/// the sim portals, so it never drifts.
#[derive(Component)]
pub struct PortalVisual;

/// Marks a quad that draws one piece of what a portal pair carries: the part
/// in front of the entry, or the part out of the exit ([`crate::carry`]).
#[derive(Component, Clone, Copy, Debug)]
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
