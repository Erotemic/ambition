//! The bag that a session begins with.
//!
//! The bag ([`OwnedItems`]) is one process resource, and the starting bag is
//! an authored first condition of an EXPERIENCE. Its experience declares it
//! (`PlatformerExperienceAuthoring::with_initial_inventory`), the candidate
//! session carries it, and the adoption of that session installs it
//! ([`StartingBag::begin_the_session`]).
//! [`restore_inventory_from_save`](super::persist::restore_inventory_from_save)
//! then replaces the bag when the save holds an inventory. An experience that
//! declares no bag begins with an empty one.
//!
//! Two measurements gave this shape.
//!
//! 2026-10-04, on the shell host
//! (`shell_host_lifecycle::a_bag_that_a_session_changed_reaches_a_later_session_through_its_save_only`):
//! an Ambition session was granted a bomb and was then replaced by a Sanic
//! session. The Sanic session had the bomb on each of its first 31 ticks, and
//! its mirror wrote the bomb into Sanic's save. So a session does not begin
//! with the bag that the session before it left.
//!
//! 2026-10-05 (`shell_host_lifecycle::each_experience_begins_with_its_own_bag`):
//! the first repair gave each session the bag that the PROCESS began with,
//! which on the shell host is Ambition's starter set. A fresh Sanic session
//! had those 10 items and wrote them into Sanic's save. So the bag is the
//! experience's, not the composition's.
//!
//! ⛔ THE DIRECT ROAD. A composition with one session and no shell activation
//! (`install_direct_session_root`) has no adoption. Its bag is the
//! `OwnedItems` it was built with, recorded at `Startup`
//! ([`install_starting_bag`]).

use std::collections::BTreeMap;

use bevy::prelude::*;

use ambition_items::{ItemCatalog, ItemCatalogRead, OwnedItems};

/// The bag that the live session began with. A New Game gives it back
/// (`session/checkpoint.rs`).
///
/// ⭐ TWO WRITERS, ONE FOR EACH ROAD, and a composition takes one road. The
/// adoption of a candidate session writes the bag of its experience. A
/// composition with a direct session root records the bag it was built with,
/// at `Startup`.
///
/// ⛔ SESSION CONFIGURATION. Nothing writes it while a session plays.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct StartingBag(OwnedItems);

impl StartingBag {
    /// The starting bag `bag`, for a host or a test that stands in for an
    /// experience that begins with `bag`.
    pub fn of(bag: OwnedItems) -> Self {
        Self(bag)
    }

    pub fn bag(&self) -> &OwnedItems {
        &self.0
    }

    /// Give the session that is adopted its bag, and remember it as the bag
    /// that the session began with.
    ///
    /// ⛔ ONLY FROM THE ADOPTION OF A SESSION. The adoption is before the
    /// session world is live, so the timeline that a rewind can cross does
    /// not exist yet, and no saved frame of this session holds the bag that
    /// this write replaces.
    ///
    /// A composition with no bag gets none: only the record is kept.
    pub fn begin_the_session(self, world: &mut World) {
        if let Some(mut owned) = world.get_resource_mut::<OwnedItems>() {
            *owned = self.0.clone();
        }
        world.insert_resource(self);
    }
}

/// The bag that a session of an experience begins with, built from the item
/// catalog of the App. A plain function: the bag is the same for each session
/// of the experience.
pub type InitialInventoryFn = fn(&ItemCatalog) -> OwnedItems;

/// The declared starting bags, by experience id.
///
/// ⛔ HOST CONFIGURATION. Written when an experience is registered, and not
/// after.
#[derive(Resource, Default)]
pub struct InitialInventories {
    by_experience: BTreeMap<String, InitialInventoryFn>,
}

impl InitialInventories {
    /// Declare the starting bag of `experience_id`.
    ///
    /// # Panics
    /// When the experience already declared one. Two declarations are two
    /// answers to one question.
    pub fn declare(&mut self, experience_id: &str, bag: InitialInventoryFn) {
        let previous = self.by_experience.insert(experience_id.to_owned(), bag);
        assert!(
            previous.is_none(),
            "experience `{experience_id}` declared its starting bag twice"
        );
    }

    pub fn declares(&self, experience_id: &str) -> bool {
        self.by_experience.contains_key(experience_id)
    }
}

/// What the builder of a candidate session reads to give the session its bag.
#[derive(bevy::ecs::system::SystemParam)]
pub struct StartingBagOf<'w> {
    /// Absent when no experience was registered through the provider.
    declared: Option<Res<'w, InitialInventories>>,
    items: ItemCatalogRead<'w>,
    /// Before the first adoption: the bag the composition was built with
    /// (the record of `Startup`). Read only to report a bag that no
    /// experience declares.
    composition: Option<Res<'w, StartingBag>>,
}

impl StartingBagOf<'_> {
    /// The bag that a session of `experience_id` begins with: the declared
    /// one, or an empty bag.
    pub fn of(&self, experience_id: &str) -> StartingBag {
        let declared = self
            .declared
            .as_deref()
            .and_then(|declared| declared.by_experience.get(experience_id));
        match declared {
            Some(bag) => StartingBag(bag(self.items.get())),
            None => StartingBag::default(),
        }
    }

    /// Does the composition hold a bag with items, that no experience
    /// declares? Such a bag reaches no session of a shell host. The remedy is
    /// a declaration on the experience it is for.
    pub fn an_undeclared_composition_bag(&self) -> bool {
        let no_declaration = self
            .declared
            .as_deref()
            .is_none_or(|declared| declared.by_experience.is_empty());
        no_declaration
            && self
                .composition
                .as_deref()
                .is_some_and(|bag| bag.0 != OwnedItems::default())
    }
}

fn record_the_starting_bag(owned: Res<OwnedItems>, mut commands: Commands) {
    commands.insert_resource(StartingBag(owned.clone()));
}

/// Record the bag that the composition was built with: the starting bag of a
/// composition with a direct session root.
///
/// Call it after the composition has `OwnedItems`. On a shell host the
/// adoption of each session replaces the record with the bag of its
/// experience.
pub fn install_starting_bag(app: &mut App) {
    app.add_systems(Startup, record_the_starting_bag);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_items::Item;

    fn bombs(world: &World) -> u32 {
        world.resource::<OwnedItems>().count(Item::Bomb)
    }

    fn a_bomb(catalog: &ItemCatalog) -> OwnedItems {
        let mut bag = OwnedItems::default();
        bag.grant(catalog, Item::Bomb, 1);
        bag
    }

    /// A session begins with the bag its experience declares, and with an
    /// empty bag when it declares none. The bag that the session before it
    /// left does not reach it.
    #[test]
    fn a_session_begins_with_the_bag_its_experience_declares() {
        let catalog = ambition_items::builtin_item_catalog();
        let mut app = App::new();
        app.insert_resource(OwnedItems::starter(catalog));
        let mut declared = InitialInventories::default();
        declared.declare("with_a_bomb", a_bomb);
        app.insert_resource(declared);
        let starter = app.world().resource::<OwnedItems>().clone();
        assert_eq!(bombs(app.world()), 0, "the starter set has a bomb, so the arm shows nothing");

        let bag_of = |app: &mut App, experience: &'static str| {
            app.world_mut()
                .run_system_once(move |bags: StartingBagOf| bags.of(experience))
                .expect("the system runs")
        };
        use bevy::ecs::system::RunSystemOnce;

        let with_a_bomb = bag_of(&mut app, "with_a_bomb");
        assert_eq!(with_a_bomb.bag(), &a_bomb(catalog));
        with_a_bomb.begin_the_session(app.world_mut());
        assert_eq!(
            (app.world().resource::<OwnedItems>(), app.world().resource::<StartingBag>().bag()),
            (&a_bomb(catalog), &a_bomb(catalog)),
            "the session does not have the bag its experience declares, or a New Game \
             would give another one"
        );

        // The next session is of an experience that declares nothing.
        let nothing = bag_of(&mut app, "declares_nothing");
        nothing.begin_the_session(app.world_mut());
        assert_eq!(
            app.world().resource::<OwnedItems>(),
            &OwnedItems::default(),
            "a session of an experience that declares no bag has the bag of the session \
             before it"
        );
        assert_ne!(starter, OwnedItems::default(), "control: the composition's bag has items");
    }

    /// A composition with a direct session root begins with the bag it was
    /// built with. The control is a grant after `Startup`: the record does
    /// not follow the live bag.
    #[test]
    fn a_composition_with_no_activation_begins_with_the_bag_it_was_built_with() {
        let catalog = ambition_items::builtin_item_catalog();
        let mut app = App::new();
        app.insert_resource(OwnedItems::starter(catalog));
        install_starting_bag(&mut app);
        app.update();
        let starter = app.world().resource::<OwnedItems>().clone();
        assert_eq!(app.world().resource::<StartingBag>().bag(), &starter);

        app.world_mut()
            .resource_mut::<OwnedItems>()
            .grant(catalog, Item::Bomb, 1);
        app.update();
        assert_eq!(bombs(app.world()), 1, "control: the grant did not reach the bag");
        assert_eq!(
            app.world().resource::<StartingBag>().bag(),
            &starter,
            "the starting bag followed the live bag"
        );
    }
}
