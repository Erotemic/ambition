//! Captured allegiance for projectiles and detached strike volumes.
//! Capture runs before contacts and before an owner can leave the world.
//! Reflection can replace the capture. Grudges remain live owner state.

use bevy::prelude::{Commands, Component, Entity, Query, Without, With, Or};

use ambition_characters::{actor::ActorFaction, control::DrivingParticipant};
use crate::targeting::{MatchTeam, Combatant};

/// The source body's side when an attack materializes.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct AttackAllegiance {
    /// The source body's authored faction when the attack materialized.
    pub faction: ActorFaction,
    /// The participant driving the source body at materialization.
    pub driver: Option<DrivingParticipant>,
    /// The source body's match team, when it had one. `None` outside a match —
    /// the faction rule then decides, exactly as it does for any unseated body.
    pub team: Option<MatchTeam>,
}

impl AttackAllegiance {
    /// Capture the source body's allegiance before control or lifetime changes.
    pub fn from_body(
        faction: ActorFaction,
        team: Option<&MatchTeam>,
        driver: Option<&DrivingParticipant>,
    ) -> Self {
        Self { faction, team: team.cloned(), driver: driver.copied() }
    }

    /// Resolve the same faction policy as a body, using launch-time control.
    pub fn effective_faction(&self) -> ActorFaction {
        crate::targeting::effective_faction(self.faction, self.driver.as_ref())
    }

    pub fn as_combatant(&self) -> Combatant<'_> {
        Combatant { faction: self.faction, driver: self.driver.as_ref(), team: self.team.as_ref() }
    }

    /// The team, in the borrowed shape `damage_lands_between` takes.
    pub fn team(&self) -> Option<&MatchTeam> {
        self.team.as_ref()
    }
}

/// Capture each new detached attack once, before damage or owner retirement.
pub fn stamp_attack_allegiance(
    mut commands: Commands,
    unstamped: Query<
        (Entity, Option<&ambition_projectiles::ProjectileOwner>, Option<&crate::strike::Hitbox>),
        (Without<AttackAllegiance>, Or<(With<ambition_projectiles::ProjectileOwner>, With<crate::strike::Hitbox>)>),
    >,
    firers: Query<(Option<&ActorFaction>, Option<&MatchTeam>, Option<&DrivingParticipant>)>,
) {
    for (attack, projectile_owner, hitbox) in &unstamped {
        let (owner, fallback) = match (projectile_owner, hitbox) {
            (Some(owner), _) => (owner.0, None),
            (_, Some(hitbox)) if matches!(hitbox.anchor, crate::strike::HitboxAnchor::World { .. })
                && hitbox.source != ambition_vfx::HitSide::Environment => {
                (hitbox.owner, Some(crate::actor_faction_from_hit_side(hitbox.source)))
            }
            _ => continue,
        };
        let Ok((faction, team, driver)) = firers.get(owner) else { continue; };
        if let Some(faction) = faction.copied().or(fallback) {
            commands.entity(attack).insert(AttackAllegiance::from_body(faction, team, driver));
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
        world.run_system_once(stamp_attack_allegiance).unwrap();
        world.entity_mut(firer).remove::<DrivingParticipant>();
        world.entity_mut(firer).insert(MatchTeam::new("red"));
        world.despawn(firer);
        world.run_system_once(stamp_attack_allegiance).unwrap();
        let stamp = world.get::<AttackAllegiance>(shot).unwrap();
        assert_eq!(stamp.faction, ActorFaction::Enemy);
        assert_eq!(stamp.driver, Some(driver));
        assert_eq!(stamp.effective_faction(), ActorFaction::Player);
        assert_eq!(stamp.team().unwrap().as_str(), "blue");
        let mut bytes = Vec::new();
        stamp.encode(&mut bytes);
        let restored = AttackAllegiance::decode(&mut Reader::new(&bytes)).unwrap();
        assert_eq!(&restored, stamp);
        assert_eq!(restored.effective_faction(), ActorFaction::Player);
    }
}
