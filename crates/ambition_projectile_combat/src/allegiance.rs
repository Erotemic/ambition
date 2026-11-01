//! Projectile combat allegiance that survives the firing body's lifetime.
//!
//! At materialization, a shot captures the firing body's faction, team and participant. Damage
//! routing can therefore classify the shot after its owner despawns, while reflection may rewrite
//! the copied allegiance deliberately. Live grudges remain owner state and `ProjectileOwner`
//! remains the separate identity of who fired the shot.
//!
//! Ownerless/environmental shots remain unstamped. Stamping runs immediately after each
//! projectile materializer and before the shot can step or settle.

use bevy::prelude::{Commands, Component, Entity, Query, Without};

use ambition_characters::{actor::ActorFaction, control::DrivingParticipant};
use ambition_combat::targeting::MatchTeam;

/// Faction, team and participant captured when a projectile materializes.
///
/// The stamp survives owner despawn and may be rewritten by reflection. Grudges
/// remain live owner state rather than launch-time state, while `ProjectileOwner`
/// continues to identify who fired the shot. Ownerless or unfactioned projectiles
/// remain unstamped.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct ProjectileAllegiance {
    /// The firing body's authored faction at the moment the shot took flight.
    pub faction: ActorFaction,
    /// The participant driving the firing body at launch.
    pub driver: Option<DrivingParticipant>,
    /// The firing body's match team, when it had one. `None` outside a match —
    /// the faction rule then decides, exactly as it does for any unseated body.
    pub team: Option<MatchTeam>,
}

impl ProjectileAllegiance {
    /// Capture the firing body's allegiance before control or lifetime changes.
    pub fn from_body(
        faction: ActorFaction,
        team: Option<&MatchTeam>,
        driver: Option<&DrivingParticipant>,
    ) -> Self {
        Self { faction, team: team.cloned(), driver: driver.copied() }
    }

    /// Resolve the same faction policy as a body, using launch-time control.
    pub fn effective_faction(&self) -> ActorFaction {
        ambition_combat::targeting::effective_faction(self.faction, self.driver.as_ref())
    }

    /// The team, in the borrowed shape `damage_lands_between` takes.
    pub fn team(&self) -> Option<&MatchTeam> {
        self.team.as_ref()
    }
}

/// Stamp newly materialized projectiles before stepping or settle can despawn
/// their owners.
///
/// This runs after both immediate and delayed materializers because an owner may be
/// eliminated later in the same tick. `Without<ProjectileAllegiance>` makes the
/// operation idempotent across rollback entity recreation.
pub fn stamp_new_projectile_allegiance(
    mut commands: Commands,
    unstamped: Query<
        (Entity, &ambition_projectiles::ProjectileOwner),
        Without<ProjectileAllegiance>,
    >,
    firers: Query<(&ActorFaction, Option<&MatchTeam>, Option<&DrivingParticipant>)>,
) {
    for (projectile, owner) in &unstamped {
        if let Ok((faction, team, driver)) = firers.get(owner.0) {
            commands.entity(projectile).insert(ProjectileAllegiance::from_body(*faction, team, driver));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_characters::control::PlayerSlot;
    use ambition_platformer2d_core::snapshot::{Reader, SnapshotState};
    use bevy::ecs::system::RunSystemOnce;
    use bevy::prelude::World;

    #[test]
    fn launch_allegiance_survives_control_loss_owner_loss_and_snapshot_restore() {
        let mut world = World::new();
        let driver = DrivingParticipant(PlayerSlot(1));
        let firer = world.spawn((ActorFaction::Enemy, driver, MatchTeam::new("blue"))).id();
        let shot = world.spawn(ambition_projectiles::ProjectileOwner(firer)).id();
        world.run_system_once(stamp_new_projectile_allegiance).unwrap();
        world.entity_mut(firer).remove::<DrivingParticipant>();
        world.entity_mut(firer).insert(MatchTeam::new("red"));
        world.despawn(firer);
        world.run_system_once(stamp_new_projectile_allegiance).unwrap();
        let stamp = world.get::<ProjectileAllegiance>(shot).unwrap();
        assert_eq!(stamp.faction, ActorFaction::Enemy);
        assert_eq!(stamp.driver, Some(driver));
        assert_eq!(stamp.effective_faction(), ActorFaction::Player);
        assert_eq!(stamp.team().unwrap().as_str(), "blue");
        let mut bytes = Vec::new();
        stamp.encode(&mut bytes);
        let restored = ProjectileAllegiance::decode(&mut Reader::new(&bytes)).unwrap();
        assert_eq!(&restored, stamp);
        assert_eq!(restored.effective_faction(), ActorFaction::Player);
    }
}
