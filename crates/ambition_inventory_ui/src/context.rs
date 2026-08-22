//! Inventory input-context ownership.
//!
//! While [`InventoryUiState::visible`] is true, this module declares the shared
//! inventory context for participants. Ownership comes from the backend-neutral
//! inventory model rather than game mode or renderer-specific state, so all
//! inventory frontends route input consistently.

use bevy::prelude::*;

use ambition_input::participant::{
    context_priority, ContextClaim, InputParticipant, ParticipantContexts, INVENTORY_CONTEXT,
};

use crate::InventoryUiState;

/// Declare [`INVENTORY_CONTEXT`] while the inventory surface is open, and
/// retract it when it closes.
///
/// The current inventory surface is global, so the claim is declared for every
/// participant. A future per-seat inventory can narrow the claim here.
pub fn declare_inventory_input_context(
    overlay: Option<Res<InventoryUiState>>,
    mut participants: Query<&mut ParticipantContexts, With<InputParticipant>>,
) {
    let open = overlay.is_some_and(|state| state.visible);
    for mut contexts in &mut participants {
        // Touch the component only when the claim actually moves, so a quiet
        // frame is not a change-detection event for every reader downstream.
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
        app.add_systems(
            Update,
            declare_inventory_input_context.in_set(ambition_input::InputSet::ResolveContext),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_input::participant::{
        resolve_active_input_context, ParticipantId, SeatInputContexts, GAMEPLAY_CONTEXT,
    };

    /// The property `GameMode` could not express: with no game mode ANYWHERE,
    /// an open inventory still takes gameplay away from the seat.
    #[test]
    fn an_open_inventory_takes_gameplay_routing_without_a_game_mode() {
        let mut app = App::new();
        app.init_resource::<InventoryUiState>();
        app.init_resource::<SeatInputContexts>();
        app.add_systems(
            Update,
            (
                declare_inventory_input_context,
                resolve_active_input_context,
            )
                .chain(),
        );
        let mut playing = ParticipantContexts::default();
        playing.declare(ContextClaim::capturing(
            GAMEPLAY_CONTEXT,
            context_priority::GAMEPLAY,
        ));
        app.world_mut()
            .spawn((InputParticipant::with_id(ParticipantId::PRIMARY), playing));

        app.update();
        assert!(
            app.world()
                .resource::<SeatInputContexts>()
                .gameplay_owned(0),
            "a closed inventory claims nothing"
        );

        app.world_mut()
            .resource_mut::<InventoryUiState>()
            .reset_for_open(false);
        app.update();
        let seats = app.world().resource::<SeatInputContexts>();
        assert_eq!(
            seats.for_seat(0).owner(),
            Some(INVENTORY_CONTEXT),
            "the open surface owns the seat's input"
        );
        assert!(
            !seats.gameplay_owned(0),
            "and the actor underneath stops being driven by the keys navigating the menu"
        );

        app.world_mut().resource_mut::<InventoryUiState>().close();
        app.update();
        assert!(
            app.world()
                .resource::<SeatInputContexts>()
                .gameplay_owned(0),
            "closing hands the seat back rather than leaving it captured"
        );
    }
}
