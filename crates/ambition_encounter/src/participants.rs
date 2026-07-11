//! Generic encounter PARTICIPANTS (§3): membership as relations, not
//! boss-specific `Vec<Entity>`.
//!
//! A participant records a stable id + its resolved live entity + a generic
//! [`EncounterRole`] + an [`Ownership`] policy + a per-tick `alive` flag. Roles
//! are generic vocabulary — a boss is a `PrimaryTarget`, a wave mob a `Minion`,
//! an escortee `Protected` — so no membership shape is boss-shaped. The stable
//! `id` is the durable identity (a boss placement id, a wave mob id): an `Entity`
//! can go stale across a rewind/room-change, but the id does not (§3 "do not
//! store raw `Entity` handles as the only durable participant identity").

use bevy::prelude::*;

/// The role a participant plays in an encounter (§3). Generic vocabulary shared
/// by every encounter shape — boss fights, wave arenas, escorts, races.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EncounterRole {
    /// The main thing the fight is about (a boss, the objective enemy).
    PrimaryTarget,
    /// A tougher-than-trash supporting combatant.
    Elite,
    /// Ordinary trash / wave fodder.
    Minion,
    /// An environmental hazard the encounter owns.
    Hazard,
    /// A non-combat objective entity (a switch, a payload).
    Objective,
    /// An ally the encounter must keep alive (escort/defense).
    Protected,
    /// An ally that moves through the encounter (escort target).
    Escort,
    /// A speaker whose lines frame the encounter.
    Narrative,
    /// A racing/duelling counterpart.
    Rival,
}

/// Whether the encounter SPAWNED a participant (and owns its cleanup) or ADOPTED
/// an already-authored actor (left alone when the encounter ends) (§3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ownership {
    /// The encounter spawned it from a recipe and cleans it up on end.
    Spawned,
    /// An already-authored actor the encounter observes but does not own.
    Adopted,
}

/// One participant relation (§3).
#[derive(Clone, Debug)]
pub struct EncounterParticipant {
    /// Durable identity (boss placement id / wave mob id). Survives an `Entity`
    /// going stale across a rewind or room change.
    pub id: String,
    /// The live entity this id currently resolves to, if any. `None` before it
    /// is resolved or after it despawns.
    pub entity: Option<Entity>,
    /// The generic role this member plays.
    pub role: EncounterRole,
    /// Spawned-and-owned vs adopted.
    pub ownership: Ownership,
    /// Refreshed each tick from the resolved entity's liveness. A gone entity
    /// (despawned / left the world) reads as not alive.
    pub alive: bool,
}

impl EncounterParticipant {
    /// An adopted participant (an already-authored actor the encounter observes).
    pub fn adopted(id: impl Into<String>, entity: Entity, role: EncounterRole) -> Self {
        Self {
            id: id.into(),
            entity: Some(entity),
            role,
            ownership: Ownership::Adopted,
            alive: true,
        }
    }

    /// A spawned-and-owned participant (the encounter cleans it up on end).
    pub fn spawned(id: impl Into<String>, entity: Option<Entity>, role: EncounterRole) -> Self {
        Self {
            id: id.into(),
            entity,
            role,
            ownership: Ownership::Spawned,
            alive: true,
        }
    }
}

/// The participant relations of one encounter entity (§3). A single-boss fight
/// has one `PrimaryTarget`; a wave arena has many `Minion`s appended as they
/// spawn; a puzzle may have none.
#[derive(Component, Clone, Debug, Default)]
pub struct EncounterParticipants {
    pub members: Vec<EncounterParticipant>,
}

impl EncounterParticipants {
    pub fn new(members: Vec<EncounterParticipant>) -> Self {
        Self { members }
    }

    /// Members playing `role`.
    pub fn with_role(
        &self,
        role: EncounterRole,
    ) -> impl Iterator<Item = &EncounterParticipant> + '_ {
        self.members.iter().filter(move |m| m.role == role)
    }

    /// True once every member playing `role` is defeated (and at least one such
    /// member exists) — the common "all bosses / all minions down" shape.
    pub fn all_with_role_defeated(&self, role: EncounterRole) -> bool {
        let mut any = false;
        for member in self.with_role(role) {
            any = true;
            if member.alive {
                return false;
            }
        }
        any
    }

    /// True once any member playing `role` is defeated.
    pub fn any_with_role_defeated(&self, role: EncounterRole) -> bool {
        self.with_role(role).any(|m| !m.alive)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn member(id: &str, role: EncounterRole, alive: bool) -> EncounterParticipant {
        EncounterParticipant {
            id: id.into(),
            entity: None,
            role,
            ownership: Ownership::Adopted,
            alive,
        }
    }

    #[test]
    fn all_with_role_defeated_needs_at_least_one_and_all_dead() {
        // No members of the role → not "all defeated" (nothing to defeat).
        let empty = EncounterParticipants::default();
        assert!(!empty.all_with_role_defeated(EncounterRole::PrimaryTarget));

        let mixed = EncounterParticipants::new(vec![
            member("boss", EncounterRole::PrimaryTarget, false),
            member("add", EncounterRole::Minion, true),
        ]);
        // The only PrimaryTarget is dead → all PrimaryTargets defeated.
        assert!(mixed.all_with_role_defeated(EncounterRole::PrimaryTarget));
        // A minion is still alive → not all minions defeated.
        assert!(!mixed.all_with_role_defeated(EncounterRole::Minion));
        // Any minion defeated? None are → false.
        assert!(!mixed.any_with_role_defeated(EncounterRole::Minion));
    }

    #[test]
    fn any_with_role_defeated_is_true_on_the_first_death() {
        let parts = EncounterParticipants::new(vec![
            member("a", EncounterRole::Minion, true),
            member("b", EncounterRole::Minion, false),
        ]);
        assert!(parts.any_with_role_defeated(EncounterRole::Minion));
        assert!(!parts.all_with_role_defeated(EncounterRole::Minion));
    }
}
