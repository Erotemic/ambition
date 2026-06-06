//! The player-held [`PortalGun`]: active/next-color state, equip/unequip, the
//! color toggle, and the dev on/off switch.

use bevy::prelude::*;

use crate::brain::ActionSet;
use crate::input::ControlFrame;
use crate::item_pickup::StashedActionSet;
use crate::player::{PlayerEntity, PrimaryPlayer};

use super::color::PortalColor;

/// Player-held portal gun state.
#[derive(Component, Clone, Copy, Debug)]
pub struct PortalGun {
    /// When false the gun ignores input (stand-in for "not equipped" until
    /// held-item equip exists).
    pub active: bool,
    /// Color the next `Attack` will place.
    pub next_color: PortalColor,
    /// Seconds before another teleport is allowed (prevents ping-pong).
    pub teleport_cooldown: f32,
}

impl Default for PortalGun {
    fn default() -> Self {
        Self {
            active: true,
            next_color: PortalColor::Blue,
            teleport_cooldown: 0.0,
        }
    }
}

/// Equip the portal gun onto the player from a non-pickup source (the inventory
/// menu): stash the action set, attach an active [`PortalGun`], and clear the
/// melee swing so `Attack` fires portals (the same replacement the world pickup
/// does). Mirrors [`super::pickup::pickup_portal_gun_system`] minus the ground entity.
pub fn equip_portal_gun(commands: &mut Commands, player: Entity, action_set: &mut ActionSet) {
    commands
        .entity(player)
        .insert(StashedActionSet(action_set.clone()));
    commands.entity(player).insert(PortalGun {
        active: true,
        ..PortalGun::default()
    });
    action_set.melee = None;
}

/// Detach the portal gun and restore the stashed action set (inventory unequip).
pub fn unequip_portal_gun(
    commands: &mut Commands,
    player: Entity,
    action_set: &mut ActionSet,
    stashed: Option<&StashedActionSet>,
) {
    if let Some(stash) = stashed {
        *action_set = stash.0.clone();
    }
    commands.entity(player).remove::<PortalGun>();
    commands.entity(player).remove::<StashedActionSet>();
}

/// `Interact` toggles which color the next `Attack` will place.
pub fn portal_toggle_system(
    control: Res<ControlFrame>,
    mut players: Query<&mut PortalGun, (With<PlayerEntity>, With<PrimaryPlayer>)>,
    nearest: Option<Res<crate::player::affordances::NearestInteractable>>,
) {
    if !control.interact_pressed {
        return;
    }
    // A genuine interactable (door / NPC / switch) claims the Interact press,
    // matching the HUD label — only toggle portal mode when there's none.
    if let Some(nearest) = nearest.as_deref() {
        if !matches!(nearest.0, crate::player::affordances::InteractVariant::None) {
            return;
        }
    }
    let Ok(mut gun) = players.single_mut() else {
        return;
    };
    if gun.active {
        gun.next_color = gun.next_color.other();
    }
}

/// Dev off-switch: `F7` toggles the portal gun active/inactive so the
/// always-on slice gun doesn't fire portals on every Attack while testing
/// other sandbox mechanics. (Visible build only.) Final gating is via
/// held-item equip; this is a developer convenience until then.
pub fn portal_dev_toggle_system(keys: Res<ButtonInput<KeyCode>>, mut guns: Query<&mut PortalGun>) {
    if !keys.just_pressed(KeyCode::F7) {
        return;
    }
    for mut gun in &mut guns {
        gun.active = !gun.active;
        bevy::log::info!(target: "ambition::portal", "portal gun active = {}", gun.active);
    }
}
