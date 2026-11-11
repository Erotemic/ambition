//! Ambition's portal gun is an ordinary held item (`portal_gun`): it lies in
//! the world as a `GroundItem`, a body takes it up and puts it down like any
//! other item, the menu equips and stows it like any other, and the held-item
//! renderer draws it. This adapter is the one place that knows the gun opens
//! portals: whatever holds it is a [`PortalGenerator`], and so is a gun lying
//! in the world, so its pair's portals stay open while it is on the floor.

use bevy::prelude::*;

use ambition_combat::held_items::HeldItem;
use ambition_held_items::GroundItem;
use ambition_portal2d::{OwnedPortalPair, PortalGenerator};

/// The held items that open portals.
pub const PORTAL_GENERATOR_ITEMS: &[&str] = &["portal_gun"];

/// Whether the held item `id` opens portals.
pub fn opens_portals(id: &str) -> bool {
    PORTAL_GENERATOR_ITEMS.contains(&id)
}

/// A body holding a portal-opening item is a [`PortalGenerator`], and one that
/// does not hold one is not; a portal-opening item lying in the world is one
/// too. (sim)
///
/// Every tick, not on change: a hand emptied by a removal satisfies no
/// `Changed` filter. A body takes up the pair it last held
/// ([`OwnedPortalPair`]), 0 if it never held one, and keeps that pair after
/// the item leaves its hand.
#[allow(clippy::type_complexity)]
pub fn reconcile_portal_generators(
    mut commands: Commands,
    bodies: Query<
        (
            Entity,
            Option<&HeldItem>,
            Has<PortalGenerator>,
            Option<&OwnedPortalPair>,
            Option<&ambition_platformer2d_core::BodyKinematics>,
        ),
        (Or<(With<HeldItem>, With<PortalGenerator>)>, Without<GroundItem>),
    >,
    items: Query<(Entity, &GroundItem), Without<PortalGenerator>>,
    mut sfx: ambition_sfx::SfxWriter,
) {
    for (body, held, generating, owned, kin) in &bodies {
        let wants = held.is_some_and(|held| opens_portals(held.id()));
        if wants && !generating {
            let pair = owned.map_or(0, |owned| owned.0);
            commands
                .entity(body)
                .insert((PortalGenerator::for_pair(pair), OwnedPortalPair(pair)));
            // A rising charge-up as the device wakes in the hand.
            if let Some(kin) = kin {
                sfx.write(ambition_sfx::SfxMessage::Play {
                    id: ambition_sfx::ids::PORTAL_POWERUP,
                    emitter: kin.pos.into(),
                });
            }
        } else if !wants && generating {
            commands.entity(body).remove::<PortalGenerator>();
        }
    }
    for (item, ground) in &items {
        if opens_portals(ground.spec.id.as_str()) {
            commands.entity(item).insert(PortalGenerator::default());
        }
    }
}

#[cfg(test)]
mod tests;
