//! What a tick's brains need to know about the OTHER bodies.
//!
//! This is the OBSERVATION half of the actor tick, separated from the decision
//! half it feeds. It was ninety lines of interleaved map-building at the top of
//! `tick_actor_brains`, which made the boundary between "look at the world" and
//! "decide what this body does" a matter of reading far enough down.
//!
//! the phases are what make it legible, not the line count. Observation
//! reads every body once and derives; decision reads the derived facts per body.
//! Nothing here touches ECS state, so the derivations are ordinary values a test
//! can build without an App.
//!
//! liveness arrives from TWO populations and that is the seam to watch.
//! A body's foe is often outside the actor query — a controlled home body carries
//! no actor cluster — so the caller notes that population separately. That second
//! source is the visible tip of the split the kernel still has to close: one body
//! population would make [`Self::note_controlled_liveness`] disappear.

use ambition_platformer2d_core as ae;
use ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance;
use bevy::prelude::Entity;
use std::collections::{BTreeMap, HashMap};

use ambition_characters::actor::ActorFaction;
use ambition_combat::crowd::CrowdKind;

/// One body's contribution to the crowd picture.
pub(crate) struct ObservedBody<'a> {
    pub id: &'a str,
    pub pos: ae::Vec2,
    pub kind: CrowdKind,
    pub faction: Option<ActorFaction>,
    /// The body this one is fighting, when it holds a target.
    pub foe: Option<Entity>,
    /// The live room the body is in (its `InRoomInstance`). A crowd is one
    /// live room's bodies.
    pub room: Option<LiveRoomInstance>,
}

/// Accumulator for one tick's observation pass.
#[derive(Default)]
pub(crate) struct CrowdObservation {
    alive_by_entity: HashMap<Entity, bool>,
    /// One crowd per live room (OW1 cut 5d). Two instances of one room hold
    /// the same actor ids, so an id is a key only inside its own room, and a
    /// body in another room does not crowd this one.
    rooms: BTreeMap<Option<LiveRoomInstance>, RoomCrowd>,
}

/// One live room's crowd, keyed by actor id.
#[derive(Default)]
struct RoomCrowd {
    requests: Vec<(String, ae::Vec2, CrowdKind)>,
    entity_to_id: HashMap<Entity, String>,
    /// The fighter each request came from, so the derived facts are keyed
    /// by the body that reads them.
    entity_by_id: HashMap<String, Entity>,
    faction_by_id: HashMap<String, ActorFaction>,
    target_entity_by_id: HashMap<String, Entity>,
}

impl CrowdObservation {
    /// Liveness of a body the actor query cannot see.
    ///
    /// See the module note: this exists only because controlled home bodies are
    /// a second population, and a fighter must be able to perceive that its foe
    /// has died whichever population the foe belongs to.
    pub(crate) fn note_controlled_liveness(&mut self, body: Entity, alive: bool) {
        self.alive_by_entity.insert(body, alive);
    }

    /// An actor body, whether or not it is in a fight.
    ///
    /// `in_a_fight` decides whether it competes for space: a bystander is alive
    /// and identifiable but does not crowd anyone.
    pub(crate) fn note_actor(
        &mut self,
        entity: Entity,
        alive: bool,
        body: Option<ObservedBody<'_>>,
        in_a_fight: bool,
    ) {
        let Some(body) = body else {
            return;
        };
        self.alive_by_entity.insert(entity, alive);
        let crowd = self.rooms.entry(body.room).or_default();
        crowd.entity_to_id.insert(entity, body.id.to_string());
        if !in_a_fight || !alive {
            return;
        }
        crowd
            .requests
            .push((body.id.to_string(), body.pos, body.kind));
        crowd.entity_by_id.insert(body.id.to_string(), entity);
        if let Some(faction) = body.faction {
            crowd.faction_by_id.insert(body.id.to_string(), faction);
        }
        if let Some(foe) = body.foe {
            crowd.target_entity_by_id.insert(body.id.to_string(), foe);
        }
    }

    /// Derive the facts the decision half reads.
    pub(crate) fn finish(self) -> CrowdFacts {
        let mut facts = CrowdFacts {
            alive_by_entity: self.alive_by_entity,
            ..CrowdFacts::default()
        };
        for crowd in self.rooms.into_values() {
            crowd.derive_into(&mut facts);
        }
        facts
    }
}

impl RoomCrowd {
    /// Derive one room's neighbours and crowding, keyed by the body.
    fn derive_into(mut self, facts: &mut CrowdFacts) {
        // Resolve each fighter's target ENTITY to the target's id, dropping foes
        // that are not crowd actors of this room. The anti-clump rule reads this
        // so a body you are fighting counts as an opponent to close on, never a
        // neighbour to flee.
        let opponent_id_by_id: HashMap<String, String> = self
            .target_entity_by_id
            .iter()
            .filter_map(|(id, foe)| {
                self.entity_to_id
                    .get(foe)
                    .map(|foe_id| (id.clone(), foe_id.clone()))
            })
            .collect();
        // CANONICAL ORDER, and it is not cosmetic. This slice is built by
        // iterating a Bevy Query, whose order is not stable and is outright
        // reshuffled by GGRS entity recreation on rollback. Both derivations
        // below break ties over it — `compute_nearest_neighbors` keeps the
        // first-found nearest among equidistant peers, and the crowding sum is
        // float addition, which is not associative — so an unstable slice is a
        // desync, not a wobble. The actor id is the stable semantic key.
        self.requests.sort_by(|a, b| a.0.cmp(&b.0));
        let neighbor_by_id = super::compute_nearest_neighbors(&self.requests);
        let crowding_by_id =
            super::compute_crowding_by_id(&self.requests, &self.faction_by_id, &opponent_id_by_id);
        let body = |id: &String| self.entity_by_id.get(id).copied();
        facts.neighbor_by_entity.extend(
            neighbor_by_id
                .into_iter()
                .filter_map(|(id, pos)| Some((body(&id)?, pos))),
        );
        facts.crowding_by_entity.extend(
            crowding_by_id
                .into_iter()
                .filter_map(|(id, signal)| Some((body(&id)?, signal))),
        );
    }
}

/// The derived crowd picture one tick's decisions read.
///
/// Keyed by the body, not by its actor id: two instances of one room hold
/// the same ids, and each body reads its own room's answer.
#[derive(Default)]
pub(crate) struct CrowdFacts {
    alive_by_entity: HashMap<Entity, bool>,
    neighbor_by_entity: HashMap<Entity, ae::Vec2>,
    crowding_by_entity: HashMap<Entity, ambition_characters::brain::smash::CrowdingSignal>,
}

impl CrowdFacts {
    /// Is this body still alive? Unknown bodies read as alive, which is what a
    /// brain should assume about something it cannot see die.
    pub(crate) fn is_alive(&self, body: Entity) -> bool {
        self.alive_by_entity.get(&body).copied().unwrap_or(true)
    }

    /// Personal-space pressure on this body, if it is in the fight at all.
    pub(crate) fn crowding(
        &self,
        body: Entity,
    ) -> Option<ambition_characters::brain::smash::CrowdingSignal> {
        self.crowding_by_entity.get(&body).copied()
    }

    /// Nearest same-kind neighbour per body — handed to the movement phase for
    /// surface-walker steering.
    pub(crate) fn neighbor_index(&self) -> &HashMap<Entity, ae::Vec2> {
        &self.neighbor_by_entity
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body(id: &str, x: f32) -> ObservedBody<'_> {
        ObservedBody {
            id,
            pos: ae::Vec2::new(x, 0.0),
            kind: CrowdKind::Ground,
            faction: Some(ActorFaction::Enemy),
            foe: None,
            room: None,
        }
    }

    fn entity(index: u32) -> Entity {
        Entity::from_raw_u32(index).expect("a valid test entity id")
    }

    /// OW1 cut 5d: a crowd is one live room's bodies.
    ///
    /// Live rooms #0 and #1 are two instances of one room, so each holds a
    /// fighter `a` and a fighter `b`. In #0 they stand together; in #1 they
    /// stand far apart. The control is #0: each crowds the other, and `a`'s
    /// nearest neighbour is #0's `b`. The subject is #1: nobody is crowded,
    /// and `a`'s nearest neighbour is #1's `b`. Keyed by actor id, the two
    /// rooms were one crowd of two `a`s and two `b`s.
    #[test]
    fn a_body_is_crowded_only_by_its_own_live_room() {
        let first = LiveRoomInstance::ACTIVATION;
        let second = first.next();
        let in_room = |id, x, room| ObservedBody {
            room: Some(room),
            ..body(id, x)
        };
        let mut o = CrowdObservation::default();
        o.note_actor(entity(1), true, Some(in_room("a", 0.0, first)), true);
        o.note_actor(entity(2), true, Some(in_room("b", 8.0, first)), true);
        o.note_actor(entity(3), true, Some(in_room("a", 1000.0, second)), true);
        o.note_actor(entity(4), true, Some(in_room("b", 1300.0, second)), true);
        let facts = o.finish();

        assert!(
            facts.crowding(entity(1)).is_some() && facts.crowding(entity(2)).is_some(),
            "control: two fighters sharing a spot in one room do not feel each other"
        );
        assert_eq!(
            facts.neighbor_index().get(&entity(1)),
            Some(&ae::Vec2::new(8.0, 0.0)),
            "control: #0's `a` does not see #0's `b` as its nearest neighbour"
        );
        assert!(
            facts.crowding(entity(3)).is_none() && facts.crowding(entity(4)).is_none(),
            "a fighter was crowded by a body in another live room"
        );
        assert_eq!(
            facts.neighbor_index().get(&entity(3)),
            Some(&ae::Vec2::new(1300.0, 0.0)),
            "#1's `a` took its nearest neighbour from another live room"
        );
    }

    /// The observation derives without an App, which is the point of it
    /// being a value rather than ninety lines inside a Bevy system.
    ///
    /// Two same-faction bodies standing on top of each other crowd each other;
    /// a bystander that is not in a fight is still known to be alive but takes
    /// no part in the crowd.
    #[test]
    fn crowding_counts_fighters_and_ignores_bystanders() {
        let mut o = CrowdObservation::default();
        let fighters = [
            Entity::from_raw_u32(1).expect("a valid test entity id"),
            Entity::from_raw_u32(2).expect("a valid test entity id"),
        ];
        o.note_actor(fighters[0], true, Some(body("a", 0.0)), true);
        o.note_actor(fighters[1], true, Some(body("b", 8.0)), true);
        let bystander = Entity::from_raw_u32(3).expect("a valid test entity id");
        o.note_actor(bystander, true, Some(body("c", 8.0)), false);
        let facts = o.finish();

        assert!(
            facts.crowding(fighters[0]).is_some() && facts.crowding(fighters[1]).is_some(),
            "two fighters sharing a spot must feel each other"
        );
        assert!(
            facts.crowding(bystander).is_none(),
            "a bystander does not crowd and is not crowded — it is not in the fight"
        );
        assert!(
            facts.is_alive(bystander),
            "not fighting is not the same as not existing"
        );
    }

    /// Liveness answers for both populations through one accessor.
    ///
    /// A fighter's foe is often a controlled body, which carries no actor
    /// cluster and so never reaches `note_actor`. an unknown body reads as
    /// ALIVE: a brain that has not seen something die must not act as though it
    /// has.
    #[test]
    fn liveness_spans_both_populations_and_defaults_to_alive() {
        let mut o = CrowdObservation::default();
        let dead_actor = Entity::from_raw_u32(1).expect("a valid test entity id");
        o.note_actor(dead_actor, false, Some(body("a", 0.0)), true);
        let dead_controlled = Entity::from_raw_u32(2).expect("a valid test entity id");
        o.note_controlled_liveness(dead_controlled, false);
        let live_controlled = Entity::from_raw_u32(3).expect("a valid test entity id");
        o.note_controlled_liveness(live_controlled, true);
        let facts = o.finish();

        assert!(!facts.is_alive(dead_actor));
        assert!(!facts.is_alive(dead_controlled));
        assert!(facts.is_alive(live_controlled));
        assert!(
            facts.is_alive(Entity::from_raw_u32(99).expect("a valid test entity id")),
            "a body nobody observed reads as alive"
        );
    }

    /// Observation order does not reach the derivations.
    ///
    /// Bevy query order is unstable and GGRS entity recreation reshuffles it, so
    /// two orderings of the same bodies must derive the same crowd. The sort by
    /// actor id inside `finish` is what makes that true.
    #[test]
    fn the_same_bodies_derive_the_same_crowd_in_any_order() {
        // Each id keeps its own body in both orders, so the two derivations
        // are compared body by body.
        let ids: [(&str, f32); 3] = [("a", 0.0), ("b", 6.0), ("c", 12.0)];
        let build = |flip: bool| {
            let mut o = CrowdObservation::default();
            let order: Vec<usize> = if flip { vec![2, 0, 1] } else { vec![0, 1, 2] };
            for i in order {
                let (id, x) = ids[i];
                o.note_actor(entity(i as u32 + 1), true, Some(body(id, x)), true);
            }
            o.finish()
        };
        let (a, b) = (build(false), build(true));
        for (i, (id, _)) in ids.iter().enumerate() {
            let body = entity(i as u32 + 1);
            assert_eq!(
                a.crowding(body).map(|c| c.pressure),
                b.crowding(body).map(|c| c.pressure),
                "'{id}' crowds differently depending on query order — that is a desync, \
                 not a wobble: this feeds rollback-registered decisions"
            );
            assert_eq!(
                a.neighbor_index().get(&body),
                b.neighbor_index().get(&body),
                "'{id}' has a different nearest neighbour depending on query order"
            );
        }
    }
}
