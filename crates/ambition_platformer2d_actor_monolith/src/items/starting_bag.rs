//! The bag that a session begins with.
//!
//! The bag ([`OwnedItems`]) is one process resource. A game states its starting
//! bag when the App is built (it inserts `OwnedItems`; a composition that
//! inserts none has an empty one), and
//! [`restore_inventory_from_save`](super::persist::restore_inventory_from_save)
//! replaces the bag only when the save holds an inventory. With no inventory in
//! the save it "keeps the live starter set". That was true of the first
//! session of a process only: later, the live bag is what the last session
//! left.
//!
//! Measured 2026-10-04 on the shell host
//! (`shell_host_lifecycle::a_bag_that_a_session_changed_reaches_a_later_session_through_its_save_only`):
//! an Ambition session was granted a bomb and was then replaced by a Sanic
//! session. The Sanic session had the bomb on each of its first 31 ticks, its
//! mirror wrote the bomb into Sanic's save, and the peer rows
//! `AmbitionGameSave` and `OwnedItemsBaseline` differed from a fresh host's on
//! ticks 1 to 30. A later session of the same experience had the old bag for
//! tick 0 only, because its save then replaced the bag.
//!
//! So each session begins with the bag that the process began with, and the
//! save of the session is then the only thing that changes it.

use bevy::prelude::*;

use ambition_items::OwnedItems;
use ambition_platformer2d_shared_tangle::lifecycle::{SessionScopeActivated, SessionScopeSet};

/// The bag that this process began with.
///
/// ⭐ ONE AUTHORITY: the `OwnedItems` that the composition built. It is
/// recorded at `Startup`, before a session can be live, so no second
/// statement of the starting bag has to agree with the first one.
///
/// ⛔ HOST CONFIGURATION. Nothing writes it after `Startup`.
#[derive(Resource, Clone, Debug)]
pub struct StartingBag(OwnedItems);

impl StartingBag {
    pub fn bag(&self) -> &OwnedItems {
        &self.0
    }
}

fn record_the_starting_bag(owned: Res<OwnedItems>, mut commands: Commands) {
    commands.insert_resource(StartingBag(owned.clone()));
}

/// Give the session that is activated the bag that the process began with.
///
/// The activation is before the session world is live, so the timeline that a
/// rewind can cross does not exist yet (see
/// `reset_session_scoped_resources_on_activation`).
pub fn start_the_bag_again_on_activation(
    mut activated: MessageReader<SessionScopeActivated>,
    starting: Res<StartingBag>,
    mut owned: ResMut<OwnedItems>,
) {
    if activated.read().count() == 0 {
        return;
    }
    *owned = starting.0.clone();
}

/// Record the starting bag and give it to each session.
///
/// Call it after the composition has `OwnedItems`. A composition with no
/// session lifecycle has no activation; the reader then reads an empty
/// channel.
pub fn install_starting_bag(app: &mut App) {
    app.add_message::<SessionScopeActivated>()
        .add_systems(Startup, record_the_starting_bag)
        .add_systems(
            Update,
            start_the_bag_again_on_activation
                .in_set(SessionScopeSet::Activate)
                .run_if(resource_exists::<StartingBag>),
        );
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_items::Item;
    use ambition_platformer2d_shared_tangle::lifecycle::{SessionScopeId, SessionScopePlugin};

    fn bombs(app: &App) -> u32 {
        app.world().resource::<OwnedItems>().count(Item::Bomb)
    }

    /// The bag of the composition is the starting bag, and an activation gives
    /// it back. The control is a frame with no activation: the bag is kept.
    #[test]
    fn a_session_activation_gives_the_bag_that_the_process_began_with() {
        let catalog = ambition_items::builtin_item_catalog();
        let mut app = App::new();
        app.add_plugins(SessionScopePlugin);
        app.insert_resource(OwnedItems::starter(catalog));
        install_starting_bag(&mut app);
        app.update();
        let starter = app.world().resource::<OwnedItems>().clone();
        assert_eq!(app.world().resource::<StartingBag>().bag(), &starter);
        assert_eq!(bombs(&app), 0, "the starter set has a bomb, so the arm shows nothing");

        app.world_mut()
            .resource_mut::<OwnedItems>()
            .grant(catalog, Item::Bomb, 1);
        app.update();
        assert_eq!(bombs(&app), 1, "control: a frame with no activation took the bomb");
        assert_eq!(
            app.world().resource::<StartingBag>().bag(),
            &starter,
            "the starting bag followed the live bag"
        );

        app.world_mut()
            .write_message(SessionScopeActivated(SessionScopeId(2)));
        app.update();
        assert_eq!(
            app.world().resource::<OwnedItems>(),
            &starter,
            "the next session has the bag of the session that ended"
        );
    }
}
