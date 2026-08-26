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
use bevy::prelude::Entity;
use std::collections::HashMap;

use ambition_combat::crowd::CrowdKind;
use crate::features::components::ActorFaction;

/// One body's contribution to the crowd picture.
pub(crate) struct ObservedBody<'a> {
    pub id: &'a str,
    pub pos: ae::Vec2,
    pub kind: CrowdKind,
    pub faction: Option<ActorFaction>,
    /// The body this one is fighting, when it holds a target.
    pub foe: Option<Entity>,
}

/// Accumulator for one tick's observation pass.
#[derive(Default)]
pub(crate) struct CrowdObservation {
    requests: Vec<(String, ae::Vec2, CrowdKind)>,
    alive_by_entity: HashMap<Entity, bool>,
    entity_to_id: HashMap<Entity, String>,
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
        self.entity_to_id.insert(entity, body.id.to_string());
        if !in_a_fight || !alive {
            return;
        }
        self.requests.push((body.id.to_string(), body.pos, body.kind));
        if let Some(faction) = body.faction {
            self.faction_by_id.insert(body.id.to_string(), faction);
        }
        if let Some(foe) = body.foe {
            self.target_entity_by_id.insert(body.id.to_string(), foe);
        }
    }

    /// Derive the facts the decision half reads.
    pub(crate) fn finish(mut self) -> CrowdFacts {
        // Resolve each fighter's target ENTITY to the target's id, dropping foes
        // that are not crowd actors. The anti-clump rule reads this so a body you
        // are fighting counts as an opponent to close on, never a neighbour to
        // flee.
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
        let crowding_by_id = super::compute_crowding_by_id(
            &self.requests,
            &self.faction_by_id,
            &opponent_id_by_id,
        );
        CrowdFacts {
            alive_by_entity: self.alive_by_entity,
            neighbor_by_id,
            crowding_by_id,
        }
    }
}

/// The derived crowd picture one tick's decisions read.
#[derive(Default)]
pub(crate) struct CrowdFacts {
    alive_by_entity: HashMap<Entity, bool>,
    neighbor_by_id: HashMap<String, ae::Vec2>,
    crowding_by_id: HashMap<String, ambition_characters::brain::smash::observation::CrowdingSignal>,
}

impl CrowdFacts {
    /// Is this body still alive? Unknown bodies read as alive, which is what a
    /// brain should assume about something it cannot see die.
    pub(crate) fn is_alive(&self, body: Entity) -> bool {
        self.alive_by_entity.get(&body).copied().unwrap_or(true)
    }

    /// Personal-space pressure on this body, if it is in the fight at all.
    pub(crate) fn crowding(&self, id: &str) -> Option<ambition_characters::brain::smash::observation::CrowdingSignal> {
        self.crowding_by_id.get(id).copied()
    }

    /// Nearest same-kind neighbour per body — handed to the movement phase for
    /// surface-walker steering.
    pub(crate) fn neighbor_index(&self) -> &HashMap<String, ae::Vec2> {
        &self.neighbor_by_id
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
        }
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
        let fighters = [Entity::from_raw_u32(1).expect("a valid test entity id"), Entity::from_raw_u32(2).expect("a valid test entity id")];
        o.note_actor(fighters[0], true, Some(body("a", 0.0)), true);
        o.note_actor(fighters[1], true, Some(body("b", 8.0)), true);
        let bystander = Entity::from_raw_u32(3).expect("a valid test entity id");
        o.note_actor(bystander, true, Some(body("c", 8.0)), false);
        let facts = o.finish();

        assert!(
            facts.crowding("a").is_some() && facts.crowding("b").is_some(),
            "two fighters sharing a spot must feel each other"
        );
        assert!(
            facts.crowding("c").is_none(),
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
        let build = |flip: bool| {
            let mut o = CrowdObservation::default();
            let ids: [(&str, f32); 3] = [("a", 0.0), ("b", 6.0), ("c", 12.0)];
            let order: Vec<usize> = if flip { vec![2, 0, 1] } else { vec![0, 1, 2] };
            for (slot, i) in order.into_iter().enumerate() {
                let (id, x) = ids[i];
                o.note_actor(Entity::from_raw_u32(slot as u32 + 1).expect("a valid test entity id"), true, Some(body(id, x)), true);
            }
            o.finish()
        };
        let (a, b) = (build(false), build(true));
        for id in ["a", "b", "c"] {
            assert_eq!(
                a.crowding(id).map(|c| c.pressure),
                b.crowding(id).map(|c| c.pressure),
                "'{id}' crowds differently depending on query order — that is a desync, \
                 not a wobble: this feeds rollback-registered decisions"
            );
            assert_eq!(
                a.neighbor_index().get(id),
                b.neighbor_index().get(id),
                "'{id}' has a different nearest neighbour depending on query order"
            );
        }
    }
}
