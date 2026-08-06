//! **An open inventory OWNS the input, and says so.**
//!
//! Until now the fact "the inventory has the controls" was spelled
//! `GameMode::Paused && inventory.visible` at every site that needed it — the
//! exact derivation `ambition_input::participant` forbids in its own header:
//! *"nothing derives input ownership from `GameMode` or from the presence of a
//! controlled body"*. Two authorities for one question, and the mode is the one
//! that is wrong: `Paused` is a fact about the WORLD (stopped for everybody),
//! while owning input is a fact about a SEAT.
//!
//! The gap that spelling left is not hypothetical. A composition that never
//! registers `GameMode` — a demo, a capture harness — had the inventory open
//! with gameplay still routing underneath it, so the cube's own navigation keys
//! also drove the actor behind it. There was nothing to fix at each router,
//! because the router had no way to be told.
//!
//! ## Backend-agnostic on purpose
//!
//! The claim reads [`InventoryUiState::visible`], which BOTH inventory
//! frontends already drive (the 3D kaleidoscope and the bevy_ui grid), rather
//! than either backend's private state. So the two cannot disagree about who
//! owns the input, and a third frontend gets the claim by raising the same flag.

use bevy::prelude::*;

use ambition_input::participant::{
    context_priority, ContextClaim, InputParticipant, ParticipantContexts, INVENTORY_CONTEXT,
};

use crate::InventoryUiState;

/// Declare [`INVENTORY_CONTEXT`] while the inventory surface is open, and
/// retract it when it closes.
///
/// The claim goes to every participant because today's inventory is a global
/// surface — one screen, opened by whoever pressed the key, over a world that
/// stops. A per-seat inventory (one player in their bag while another keeps
/// playing) is a change HERE and nowhere else, which is the whole reason the
/// resolved context is keyed by seat.
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
///
/// Separate from whoever creates [`InventoryUiState`] so that a composition
/// showing an inventory without a participant stack (a pure render fixture)
/// is not forced to carry the input pipeline; and installed BY that composition
/// rather than inferred, because a claim nobody declared is the failure mode
/// this whole seam exists to make impossible.
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
