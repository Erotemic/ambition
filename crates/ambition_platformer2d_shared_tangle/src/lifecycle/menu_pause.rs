//! Pause requests are participant state, not device state or hitstop.

use bevy::prelude::*;
use super::{SessionScopeId, SimulationIsland};

/// Menu requests from the local presentation for the current session.
#[derive(Resource, Default)]
pub struct LocalMenuPauseVotes {
    scope: Option<SessionScopeId>,
    slots: Vec<u8>,
}

impl LocalMenuPauseVotes {
    pub fn publish(&mut self, scope: Option<SessionScopeId>, slots: impl IntoIterator<Item = u8>) {
        self.scope = scope;
        self.slots.clear();
        self.slots.extend(slots);
    }

    pub fn requested(&self, scope: SessionScopeId, slot: u8) -> bool {
        self.scope == Some(scope) && self.slots.contains(&slot)
    }
}

/// A network backend owns pause execution while this resource is present.
#[derive(Resource)]
pub struct NetworkMenuPause;

/// The runtime refreshes the admitted pause roster in this set.
#[derive(SystemSet, Clone, Debug, PartialEq, Eq, Hash)]
pub struct MenuPauseRosterPublished;

/// The host publishes the admitted roster and the rollback authority of each
/// member. An unknown island prevents a pause decision, not a resume.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct MenuPauseRoster {
    pub scope: Option<SessionScopeId>,
    pub members: Vec<(u8, Option<SimulationIsland>)>,
}

/// One decision for each known simulation island. Votes from nonmembers do
/// not count. A missing vote means running. Empty islands never pause.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IslandMenuPause {
    pub island: SimulationIsland,
    pub members: Vec<u8>,
    pub paused: bool,
}

impl MenuPauseRoster {
    pub fn decisions(&self, mut requested: impl FnMut(u8) -> bool) -> Vec<IslandMenuPause> {
        let complete = self.scope.is_some() && self.members.iter().all(|(_, island)| island.is_some());
        let mut rows: Vec<IslandMenuPause> = Vec::new();
        for &(slot, island) in &self.members {
            let Some(island) = island else { continue; };
            let vote = requested(slot);
            if let Some(row) = rows.iter_mut().find(|row| row.island == island) {
                row.members.push(slot);
                row.paused &= vote;
            } else {
                rows.push(IslandMenuPause { island, members: vec![slot], paused: complete && vote });
            }
        }
        rows
    }

    /// The legacy global GameMode adapter must never suspend another island.
    /// Room-authority pauses need their own execution gate.
    pub fn session_pause(&self, requested: impl FnMut(u8) -> bool) -> bool {
        let rows = self.decisions(requested);
        matches!(rows.as_slice(), [row] if row.paused &&
            matches!(row.island, SimulationIsland::Session(scope) if Some(scope) == self.scope))
    }
}

/// The legacy session-wide adapter. Keep the underlying mode when a menu
/// requests pause. A different mode request takes precedence on release.
#[derive(Default)]
pub struct MenuPauseHold {
    resume: Option<crate::schedule::GameMode>,
}

impl MenuPauseHold {
    pub fn update(&mut self, open: bool, current: crate::schedule::GameMode,
        pending: Option<crate::schedule::GameMode>) -> Option<crate::schedule::GameMode> {
        use crate::schedule::GameMode;
        let underlying = pending.unwrap_or(current);
        if open {
            match underlying {
                GameMode::Playing | GameMode::Dialogue | GameMode::Cutscene => {
                    self.resume = Some(underlying);
                    Some(GameMode::Paused)
                }
                GameMode::Paused => {
                    if pending.is_some() || self.resume.is_none() {
                        self.resume = Some(GameMode::Paused);
                    }
                    None
                }
                _ => { self.resume = None; None }
            }
        } else {
            self.resume.take().filter(|resume| pending.is_none() &&
                underlying == GameMode::Paused && *resume != GameMode::Paused)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn two_islands_have_independent_votes_and_cannot_request_global_pause() {
        use super::super::LiveRoomInstance;
        let scope = SessionScopeId(1);
        let a = SimulationIsland::Room(scope, LiveRoomInstance::from_ordinal(1));
        let b = SimulationIsland::Room(scope, LiveRoomInstance::from_ordinal(2));
        let roster = MenuPauseRoster { scope: Some(scope), members: vec![
            (0, Some(a)), (1, Some(a)), (2, Some(b)),
        ] };
        let rows = roster.decisions(|slot| slot != 2);
        assert_eq!(rows.len(), 2);
        assert!(rows[0].paused);
        assert!(!rows[1].paused);
        assert!(!roster.session_pause(|_| true));
    }

    #[test]
    fn unanimity_is_nonempty_and_does_not_depend_on_transport() {
        let scope = SessionScopeId(1);
        let mut roster = MenuPauseRoster { scope: Some(scope), members: vec![] };
        assert!(!roster.session_pause(|_| true));
        roster.members = vec![(0, Some(SimulationIsland::Session(scope))), (1, Some(SimulationIsland::Session(scope)))];
        assert!(!roster.session_pause(|slot| slot == 0));
        assert!(roster.session_pause(|_| true));
        roster.members.push((2, None));
        assert!(!roster.session_pause(|_| true));
    }
}
