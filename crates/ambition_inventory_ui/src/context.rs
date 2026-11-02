//! Inventory input claims for each surface's participant.

use bevy::prelude::*;
use ambition_input::participant::{
    context_priority, ContextClaim, InputParticipant, ParticipantContexts, INVENTORY_CONTEXT,
};
use crate::{InventoryMenu, InventoryUiState};

/// Claim inventory input only for the owner of an open surface.
pub fn declare_inventory_input_context(
    menus: Query<(&InventoryMenu, &InventoryUiState)>,
    mut participants: Query<(&InputParticipant, &mut ParticipantContexts)>,
) {
    for (participant, mut contexts) in &mut participants {
        let open = menus.iter().any(|(menu, state)| menu.owner == participant.id && state.visible);
        if contexts.is_declared(INVENTORY_CONTEXT) != open {
            contexts.sync(
                ContextClaim::capturing(INVENTORY_CONTEXT, context_priority::INVENTORY),
                open,
            );
        }
    }
}

/// Install the inventory's input-context claim.
pub struct InventoryInputContextPlugin;

impl Plugin for InventoryInputContextPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update,
            declare_inventory_input_context.in_set(ambition_input::InputSet::ResolveContext));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_input::participant::{
        resolve_active_input_context, ParticipantId, SeatInputContexts, GAMEPLAY_CONTEXT,
    };

    #[test]
    fn each_inventory_claims_only_its_owner_without_a_game_mode() {
        let mut app = App::new();
        app.init_resource::<SeatInputContexts>();
        app.add_systems(Update, (declare_inventory_input_context, resolve_active_input_context).chain());
        let menus = [ParticipantId::PRIMARY, ParticipantId::SECONDARY].map(|owner| {
            let mut playing = ParticipantContexts::default();
            playing.declare(ContextClaim::capturing(GAMEPLAY_CONTEXT, context_priority::GAMEPLAY));
            app.world_mut().spawn((InputParticipant::with_id(owner), playing));
            app.world_mut().spawn(InventoryMenu { owner }).id()
        });
        app.update();
        for slot in 0..2 { assert!(app.world().resource::<SeatInputContexts>().gameplay_owned(slot)); }
        for slot in 0..2 {
            app.world_mut().get_mut::<InventoryUiState>(menus[slot]).unwrap().reset_for_open();
            app.update();
            let seats = app.world().resource::<SeatInputContexts>();
            assert_eq!(seats.for_seat(slot as u8).owner(), Some(INVENTORY_CONTEXT));
            assert!(seats.gameplay_owned(1 - slot as u8), "another participant retains input");
            app.world_mut().get_mut::<InventoryUiState>(menus[slot]).unwrap().close();
            app.update();
            assert!(app.world().resource::<SeatInputContexts>().gameplay_owned(slot as u8));
        }
        for menu in menus { app.world_mut().get_mut::<InventoryUiState>(menu).unwrap().reset_for_open(); }
        app.update();
        app.world_mut().entity_mut(menus[0]).despawn();
        app.update();
        let seats = app.world().resource::<SeatInputContexts>();
        assert!(seats.gameplay_owned(0), "a removed menu releases its claim");
        assert_eq!(seats.for_seat(1).owner(), Some(INVENTORY_CONTEXT));
    }
}
