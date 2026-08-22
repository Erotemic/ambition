//! Authored inventory conditions.
//!
//! `inventory.holds(item)` asks whether the player owns at least one item of a
//! kind; it is distinct from occurrence-level custody. Item spelling and aliases
//! are resolved only by [`Item::from_dialog_id`].

use ambition_platformer2d_shared_tangle::authored_logic::{
    AuthoredArg, ConditionDescriptor, ConditionId, ConditionOutcome, ParamKind, ParamSpec,
};
use bevy::prelude::World;

use ambition_items::{Item, OwnedItems};

/// The domain segment every condition in this file is published under.
pub const DOMAIN: &str = "inventory";

const ITEM: ParamSpec = ParamSpec {
    name: "item",
    kind: ParamKind::Name,
    summary: "the item kind, spelled loosely (`HealthPotion`, `health_potion`, `healthcell`)",
};

/// `inventory.holds(item)` — is the player carrying at least one of these?
pub fn holds_descriptor() -> ConditionDescriptor {
    ConditionDescriptor {
        id: ConditionId::new(DOMAIN, "holds"),
        summary: "true while the player's bag holds at least one of the named item kind",
        params: &[ITEM],
    }
}

/// `inventory.holds` — see [`holds_descriptor`]. Unknown item kinds are
/// `Unanswerable`; a known item with zero owned copies is `NotSatisfied`.
pub fn holds(world: &World, args: &[AuthoredArg]) -> ConditionOutcome {
    let Some(name) = args[0].as_name() else {
        return ConditionOutcome::unanswerable("`item` must be a name");
    };
    let Some(item) = Item::from_dialog_id(name) else {
        return ConditionOutcome::unanswerable(format!(
            "no item kind is spelled `{name}` in this composition's catalog"
        ));
    };
    let Some(owned) = world.get_resource::<OwnedItems>() else {
        return ConditionOutcome::unanswerable(
            "no inventory is installed in this composition, so nothing is carried",
        );
    };
    ConditionOutcome::from_bool(owned.count(item) > 0)
}

/// Publishes the inventory domain's conditions.
///
/// one plugin for one registration line, matching
/// [`WorldFactConditionsPlugin`](crate::world_facts::WorldFactConditionsPlugin):
/// composition adds it, and nothing else in the engine learns that the bag can
/// be asked about.
pub struct InventoryConditionsPlugin;

impl bevy::prelude::Plugin for InventoryConditionsPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        use ambition_platformer2d_shared_tangle::authored_logic::PublishCondition;
        app.publish_condition(holds_descriptor(), holds);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::*;

    fn ask(world: &World, name: &str) -> ConditionOutcome {
        holds(world, &[AuthoredArg::Name(name.to_string())])
    }

    /// THE BAG ANSWERS, AND IT ANSWERS ABOUT THE LIVE BAG.
    #[test]
    fn the_inventory_domain_reads_the_live_bag_through_loose_spelling() {
        let mut app = App::new();
        app.insert_resource(OwnedItems::starter());
        let world = app.world();

        // The starter bag carries health cells, however the author spells them.
        assert_eq!(ask(world, "HealthPotion"), ConditionOutcome::Satisfied);
        assert_eq!(ask(world, "health_potion"), ConditionOutcome::Satisfied);
        assert_eq!(ask(world, "healthcell"), ConditionOutcome::Satisfied);

        // a kind the catalog knows but the bag does not hold is NOT satisfied.
        assert_eq!(ask(world, "gunsword"), ConditionOutcome::NotSatisfied);

        // and a kind that does not exist is UNANSWERABLE, not "no" — an
        // authored typo must be reported, not silently answered forever.
        assert!(matches!(
            ask(world, "definitely_not_an_item"),
            ConditionOutcome::Unanswerable(_)
        ));

        // Taking the last one flips the answer with nothing refreshed.
        app.world_mut()
            .resource_mut::<OwnedItems>()
            .take(Item::HealthCell, u32::MAX);
        assert_eq!(
            ask(app.world(), "HealthPotion"),
            ConditionOutcome::NotSatisfied
        );
    }

    /// NO INVENTORY AT ALL IS UNANSWERABLE, NOT EMPTY.
    #[test]
    fn a_composition_with_no_inventory_cannot_answer() {
        let app = App::new();
        assert!(matches!(
            ask(app.world(), "HealthPotion"),
            ConditionOutcome::Unanswerable(_)
        ));
    }
}
