//! A component a character's body carries from the batch that builds it.

use bevy_ecs::component::Component;
use bevy_ecs::system::EntityCommands;

/// One component type, at its default value, that every body wearing a
/// character carries from the batch that builds the body.
///
/// A game uses it for its own state of one creature, such as a shell phase.
/// The engine cannot name that type, and a pass that adds it after the body
/// is built leaves the body incomplete for a tick.
///
/// The author names the type only. The insert is generated here, so a kit
/// writes one default value on the body being built and can do nothing else.
/// The write is `insert_if_new`: a body that is granted its character again
/// keeps the state it has.
#[derive(Clone, Copy)]
pub struct CarriedComponent {
    insert: fn(&mut EntityCommands),
    type_name: &'static str,
}

fn insert_default<C: Component + Default>(entity: &mut EntityCommands) {
    entity.insert_if_new(C::default());
}

impl CarriedComponent {
    /// `C`, at `C::default()`.
    pub fn of<C: Component + Default>() -> Self {
        Self {
            insert: insert_default::<C>,
            type_name: core::any::type_name::<C>(),
        }
    }

    /// The component's type name, which is also its identity here.
    pub fn type_name(&self) -> &'static str {
        self.type_name
    }

    /// Write the default value on `entity`, unless it already has one.
    pub fn insert_into(&self, entity: &mut EntityCommands) {
        (self.insert)(entity);
    }
}

impl core::fmt::Debug for CarriedComponent {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.type_name)
    }
}

impl PartialEq for CarriedComponent {
    fn eq(&self, other: &Self) -> bool {
        self.type_name == other.type_name
    }
}

impl Eq for CarriedComponent {}

/// Serialized as its type name, so a content identity that serializes a
/// character includes what the character carries.
impl serde::Serialize for CarriedComponent {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.type_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_ecs::world::World;

    #[derive(Component, Debug, Default, PartialEq)]
    enum Phase {
        #[default]
        Resting,
        Moving,
    }

    /// A body built without the component gets the default; a body granted
    /// its character again keeps the state it has.
    #[test]
    fn a_carried_component_is_written_once_and_then_kept() {
        let mut world = World::new();
        let carried = CarriedComponent::of::<Phase>();
        let fresh = world.spawn_empty().id();
        let moving = world.spawn(Phase::Moving).id();
        {
            let mut commands = world.commands();
            carried.insert_into(&mut commands.entity(fresh));
            carried.insert_into(&mut commands.entity(moving));
        }
        world.flush();
        assert_eq!(world.get::<Phase>(fresh), Some(&Phase::Resting));
        assert_eq!(world.get::<Phase>(moving), Some(&Phase::Moving));
    }
}
