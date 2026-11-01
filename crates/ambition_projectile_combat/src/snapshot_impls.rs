//! Wire format for projectile combat state.

use ambition_platformer2d_core::snapshot::{put_bool, put_str, put_u8, Reader, SnapshotState};

/// A shot's own side of the fight.
///
/// The stamp is taken from the firer the first tick the bolt flies; after the firer is gone
/// there is nothing left to re-derive it from, so a rewind that dropped it would restore a shot
/// that had forgotten whose attack it is — indiscriminate, hitting its own team, which is the
/// state closed.
impl SnapshotState for crate::ProjectileAllegiance {
    fn encode(&self, out: &mut Vec<u8>) {
        self.faction.encode(out);
        put_bool(out, self.driver.is_some());
        if let Some(driver) = self.driver { put_u8(out, driver.0.0); }
        match &self.team {
            None => put_bool(out, false),
            Some(team) => {
                put_bool(out, true);
                put_str(out, team.as_str());
            }
        }
    }

    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        let faction = ambition_characters::actor::ActorFaction::decode(r)?;
        let driver = if r.bool()? {
            Some(ambition_characters::control::DrivingParticipant(
                ambition_characters::control::PlayerSlot(r.u8()?),
            ))
        } else { None };
        let team = if r.bool()? {
            Some(ambition_combat::targeting::MatchTeam::new(r.str()?))
        } else {
            None
        };
        Some(Self { faction, driver, team })
    }
}

