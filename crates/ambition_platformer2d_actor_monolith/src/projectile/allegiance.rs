//! Projectile combat allegiance that survives the firing body's lifetime.
//!
//! At materialization, a shot copies the firing body's authored faction and match team. Damage
//! routing can therefore classify the shot after its owner despawns, while reflection may rewrite
//! the copied allegiance deliberately. Live grudges remain owner state and `ProjectileOwner`
//! remains the separate identity of who fired the shot.
//!
//! Ownerless/environmental shots remain unstamped. Stamping runs immediately after each
//! projectile materializer and before the shot can step or settle.

use bevy::prelude::{Commands, Component, Entity, Query, Without};

use ambition_characters::actor::ActorFaction;
use ambition_combat::targeting::MatchTeam;

/// Faction/team captured from the firing body when a projectile materializes.
///
/// The stamp survives owner despawn and may be rewritten by reflection. Grudges
/// remain live owner state rather than launch-time state, while `ProjectileOwner`
/// continues to identify who fired the shot. Ownerless or unfactioned projectiles
/// remain unstamped.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct ProjectileAllegiance {
    /// The firing body's authored faction at the moment the shot took flight.
    pub faction: ActorFaction,
    /// The firing body's match team, when it had one. `None` outside a match —
    /// the faction rule then decides, exactly as it does for any unseated body.
    pub team: Option<MatchTeam>,
}

impl ProjectileAllegiance {
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
    firers: Query<(&ActorFaction, Option<&MatchTeam>)>,
) {
    for (projectile, owner) in &unstamped {
        if let Ok((faction, team)) = firers.get(owner.0) {
            commands.entity(projectile).insert(ProjectileAllegiance {
                faction: *faction,
                team: team.cloned(),
            });
        }
    }
}
