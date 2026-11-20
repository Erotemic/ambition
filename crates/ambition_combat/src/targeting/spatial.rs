//! Combat policy over the reusable geometry tree.

use super::*;
use ae::spatial::{Spatial, SpatialFilter, SpatialTree};
use ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance;
use std::collections::{BTreeMap, HashMap};

type Candidate<'a> = (
    Entity,
    ae::Vec2,
    ActorFaction,
    Option<&'a SimId>,
    Option<&'a MatchTeam>,
    Option<LiveRoomInstance>,
);
const FACTIONS: [ActorFaction; FACTION_COUNT] = [
    ActorFaction::Player,
    ActorFaction::Enemy,
    ActorFaction::Npc,
    ActorFaction::Boss,
    ActorFaction::Neutral,
];

#[derive(Clone, Copy)]
struct Point {
    rank: usize,
    entity: Entity,
    position: ae::Vec2,
    faction: ActorFaction,
    team: usize,
}
impl Spatial for Point {
    fn spatial_bounds(&self) -> ae::Aabb {
        ae::Aabb::new(self.position, ae::Vec2::ZERO)
    }
}
#[derive(Clone, Copy)]
struct Summary {
    factions: u8,
    team: Option<usize>,
}
#[derive(Default)]
struct RoomTree {
    tree: SpatialTree<Point, Summary>,
    finite: bool,
}
#[derive(Default, Clone, Copy)]
struct TeamMask {
    stamp: usize,
    factions: u8,
    known: u8,
}

/// Derived storage rebuilt from the live candidate snapshot on every tick.
#[derive(Default)]
pub struct TargetSelectionIndex {
    rooms: BTreeMap<Option<LiveRoomInstance>, RoomTree>,
    candidate_teams: Vec<usize>,
    masks: Vec<TeamMask>,
    stamp: usize,
    named: HashMap<Entity, usize>,
    previous: Vec<Option<usize>>,
}

impl TargetSelectionIndex {
    pub(super) fn rebuild<'data>(
        &mut self,
        candidates: &[Candidate<'data>],
    ) -> Vec<Option<&'data MatchTeam>> {
        let mut groups: BTreeMap<_, Vec<usize>> = BTreeMap::new();
        let mut team_ids = BTreeMap::new();
        let mut teams = Vec::new();
        self.candidate_teams.clear();
        self.named.clear();
        self.previous.clear();
        for (rank, candidate) in candidates.iter().enumerate() {
            groups.entry(candidate.5).or_default().push(rank);
            let team = *team_ids
                .entry(candidate.4.map(MatchTeam::as_str))
                .or_insert_with(|| {
                    teams.push(candidate.4);
                    teams.len() - 1
                });
            self.candidate_teams.push(team);
            self.previous.push(self.named.insert(candidate.0, rank));
        }
        self.rooms.retain(|room, _| groups.contains_key(room));
        for (room, ranks) in groups {
            let slot = self.rooms.entry(room).or_default();
            slot.finite = slot
                .tree
                .rebuild_with_summary(
                    ranks.into_iter().map(|rank| {
                        let candidate = &candidates[rank];
                        Point {
                            rank,
                            entity: candidate.0,
                            position: candidate.1,
                            faction: candidate.2,
                            team: self.candidate_teams[rank],
                        }
                    }),
                    |point| Summary {
                        factions: 1 << point.faction as u8,
                        team: Some(point.team),
                    },
                    |all, child| Summary {
                        factions: all.factions | child.factions,
                        team: if all.team == child.team {
                            all.team
                        } else {
                            None
                        },
                    },
                )
                .is_ok();
        }
        self.masks.resize(teams.len(), TeamMask::default());
        self.masks.fill(TeamMask::default());
        self.stamp = 0;
        teams
    }

    pub(super) fn nearest(
        &mut self,
        candidates: &[Candidate<'_>],
        teams: &[Option<&MatchTeam>],
        entity: Entity,
        position: ae::Vec2,
        room: Option<LiveRoomInstance>,
        relations: Option<&FactionRelations>,
        actor: Combatant<'_>,
        grudge: Option<crate::components::Grudge>,
    ) -> Option<(Entity, ae::Vec2, f32)> {
        let tree = self.rooms.get(&room)?;
        self.stamp += 1;
        let body_grudge = grudge.and_then(|g| g.body());
        let mut filter = Filter {
            teams,
            masks: &mut self.masks,
            stamp: self.stamp,
            relations,
            actor,
            grudge: grudge.filter(|g| g.body().is_none()),
            body_grudge,
            entity,
        };
        let mut best = None;
        let mut consider = |rank: usize| {
            let candidate = &candidates[rank];
            let distance = distance_squared(candidate.1, position);
            if best.is_none_or(|(before, d)| distance < d || (distance == d && rank < before)) {
                best = Some((rank, distance));
            }
        };
        if !tree.finite || !position.is_finite() {
            // Bounds require finite positions. Invalid inputs preserve the
            // canonical scan order and its numeric comparisons.
            for (rank, candidate) in candidates.iter().enumerate() {
                if candidate.5 == room
                    && filter.accepts(&Point {
                        rank,
                        entity: candidate.0,
                        position: candidate.1,
                        faction: candidate.2,
                        team: self.candidate_teams[rank],
                    })
                {
                    consider(rank);
                }
            }
        } else {
            if let Some(body) = body_grudge {
                let mut rank = self.named.get(&body).copied();
                while let Some(index) = rank {
                    if candidates[index].5 == room && body != entity {
                        consider(index);
                    }
                    rank = self.previous[index];
                }
            }
            if let Some((point, _)) = tree.tree.nearest_filtered(position, &mut filter) {
                consider(point.rank);
            }
        }
        best.map(|(rank, distance)| (candidates[rank].0, candidates[rank].1, distance))
    }
}

struct Filter<'a> {
    teams: &'a [Option<&'a MatchTeam>],
    masks: &'a mut [TeamMask],
    stamp: usize,
    relations: Option<&'a FactionRelations>,
    actor: Combatant<'a>,
    grudge: Option<crate::components::Grudge>,
    body_grudge: Option<Entity>,
    entity: Entity,
}
impl Filter<'_> {
    fn mask(&mut self, team: usize, requested: u8) -> u8 {
        let slot = &mut self.masks[team];
        if slot.stamp != self.stamp {
            slot.stamp = self.stamp;
            slot.factions = 0;
            slot.known = 0;
        }
        let missing = requested & !slot.known;
        if missing != 0 {
            for faction in FACTIONS {
                if missing & (1 << faction as u8) == 0 {
                    continue;
                }
                if combat_relation(
                    self.relations,
                    self.actor,
                    self.grudge,
                    Entity::PLACEHOLDER,
                    Combatant {
                        faction,
                        driver: None,
                        team: self.teams[team],
                    },
                )
                .is_target()
                {
                    slot.factions |= 1 << faction as u8;
                }
            }
            slot.known |= missing;
        }
        slot.factions
    }
}
impl SpatialFilter<Point, Summary> for Filter<'_> {
    fn accepts(&mut self, point: &Point) -> bool {
        point.entity != self.entity
            && (self.body_grudge == Some(point.entity)
                || self.mask(point.team, 1 << point.faction as u8) & (1 << point.faction as u8)
                    != 0)
    }
    fn may_match(&mut self, summary: &Summary) -> bool {
        match summary.team {
            Some(team) => self.mask(team, summary.factions) & summary.factions != 0,
            // Without an observer team, candidate teams cannot change policy.
            None if self.actor.team.is_none() => {
                self.mask(0, summary.factions) & summary.factions != 0
            }
            None => true,
        }
    }
}

#[cfg(test)]
mod tests;
