//! The construction half of neighboring-room prefetch.
//!
//! A prepared [`RoomConstructionPlan`] is an ENGINE artifact keyed by engine
//! identity — content epoch, session scope, and the live room it neighbours —
//! so the cache that promotes one into a live transition belongs beside the
//! transition, not beside the host's sprite manifests.
//!
//! The host still decides WHEN to prefetch and owns the asset half (a manifest
//! and its handle readiness are presentation facts). It publishes finished plans
//! here; the transition promotes one only if every identity term still matches,
//! so a hot reload, a provider swap, or a session change is a safe MISS rather
//! than a stale promotion.
//!
//! ⛔⛔ **THE IDENTITY USED TO BE THE CONSUMER'S ALONE, AND THE CACHE WAS
//! THEREFORE DEAD.** `publish` took a room id and a plan; only `promote` ever
//! stated an identity, and it stated it by RESETTING the cache to it. The host's
//! producer never set one — it kept its own copy of the same triple on its
//! asset-side state and reset THAT — so this cache sat at its default `(0, None,
//! None)` for the life of a session and the first promotion of every session
//! cleared every plan in it before looking one up. Measured 2026-09-08 in
//! `a_checkpoint_outlook_refuses_a_plan_prepared_without_one`: four warm
//! neighbour plans, every promotion term satisfied, `prefetch_hit=false`.
//!
//! ⭐ SO THE IDENTITY TRAVELS WITH THE PLAN. [`PrefetchIdentity`] is one value
//! and [`RoomConstructionPlanPrefetch::publish`] requires it, which makes "a plan
//! is in the cache without the cache knowing what world it was prepared for"
//! unrepresentable rather than merely unlikely. There is no public reset: the
//! identity is adopted by publishing and checked by promoting, and those are the
//! only two ways it can change.
//!
//! ⭐ THE SOURCE ROOM IS A KEY, NOT A RESET. More than one room can be live, and
//! each live room has its own neighbours. The identity has two parts. The WORLD
//! (content epoch and session scope) is one value for the cache: a change clears
//! all entries. The SOURCE ROOM is a key of each entry: a crossing from one room
//! does not remove the entries of a different live room. While the source was a
//! part of the one adopted identity, the first crossing with two live rooms
//! cleared the plans of the two rooms (measured 2026-10-04,
//! `each_live_room_keeps_the_plans_of_its_own_neighbours`). The rule is in
//! [`PrefetchedByRoom`], and the host's asset cache holds one too, so the two
//! caches cannot disagree about it.

use std::collections::BTreeMap;
use std::sync::Arc;

use bevy::prelude::Resource;

use ambition_platformer2d_actor_monolith::rooms::RoomConstructionPlan;
use ambition_platformer2d_shared_tangle::lifecycle::{RoomOccurrenceOutlook, SessionScopeId};
use ambition_platformer2d_world::rooms::RoomSpec;

/// The world a prefetched plan was prepared for.
///
/// ⚠ ONE VALUE, NOT THREE PARAMETERS. Spelled as a triple it was spelled at each
/// call site, and one of the two sites simply never spelled it — see the module
/// note. A caller that has to construct this cannot forget a term, and the two
/// sites that matter now build it the same way.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrefetchIdentity {
    content_epoch: u64,
    session_scope: Option<SessionScopeId>,
    source_room_id: String,
}

impl PrefetchIdentity {
    /// The identity of the world a plan is being prepared in, or promoted into.
    pub fn new(
        content_epoch: u64,
        session_scope: Option<SessionScopeId>,
        source_room_id: &str,
    ) -> Self {
        Self {
            content_epoch,
            session_scope,
            source_room_id: source_room_id.to_string(),
        }
    }

    /// The session this identity belongs to, for the plan-level scope check.
    pub fn session_scope(&self) -> Option<SessionScopeId> {
        self.session_scope
    }

    /// The live room that the prepared rooms are neighbours of.
    pub fn source_room_id(&self) -> &str {
        &self.source_room_id
    }

    /// Whether `other` is in the same world: the same content epoch and the
    /// same session. The source room is not a part of this question.
    fn same_world(&self, other: &Self) -> bool {
        self.content_epoch == other.content_epoch && self.session_scope == other.session_scope
    }
}

/// The entries of a prefetch cache, each under the identity it was prepared
/// for: source room, then target room.
///
/// The world of the entries is one value. An entry point that takes a
/// [`PrefetchIdentity`] adopts its world first, so a read never crosses a
/// content epoch or a session, and an insert never leaves an entry of an older
/// world in the cache.
#[derive(Debug)]
pub struct PrefetchedByRoom<T> {
    /// The identity that was adopted last. Only its world is compared.
    world: Option<PrefetchIdentity>,
    by_source: BTreeMap<String, BTreeMap<String, T>>,
}

impl<T> Default for PrefetchedByRoom<T> {
    fn default() -> Self {
        Self {
            world: None,
            by_source: BTreeMap::new(),
        }
    }
}

impl<T> PrefetchedByRoom<T> {
    /// Adopt the world of `identity`. Answers whether it is a new world, in
    /// which case all entries were removed.
    ///
    /// A new SOURCE room in the same world removes nothing.
    pub fn adopt(&mut self, identity: &PrefetchIdentity) -> bool {
        if self.world.as_ref().is_some_and(|world| world.same_world(identity)) {
            return false;
        }
        self.world = Some(identity.clone());
        self.by_source.clear();
        true
    }

    /// Remove all entries and the world they were prepared for.
    pub fn clear(&mut self) {
        self.world = None;
        self.by_source.clear();
    }

    /// Put the entry for `target_room_id`, prepared as a neighbour of the
    /// source room of `identity`.
    pub fn insert(&mut self, identity: &PrefetchIdentity, target_room_id: &str, entry: T) {
        self.adopt(identity);
        self.by_source
            .entry(identity.source_room_id.clone())
            .or_default()
            .insert(target_room_id.to_string(), entry);
    }

    /// The entry for a crossing from the source room of `identity` to
    /// `target_room_id`, after the world of `identity` is adopted.
    pub fn get(&mut self, identity: &PrefetchIdentity, target_room_id: &str) -> Option<&T> {
        self.adopt(identity);
        self.peek(&identity.source_room_id, target_room_id)
    }

    /// The entry for `target_room_id` as a neighbour of `source_room_id`,
    /// with no question about the world.
    pub fn peek(&self, source_room_id: &str, target_room_id: &str) -> Option<&T> {
        self.by_source.get(source_room_id)?.get(target_room_id)
    }

    pub fn remove(&mut self, source_room_id: &str, target_room_id: &str) -> Option<T> {
        let targets = self.by_source.get_mut(source_room_id)?;
        let removed = targets.remove(target_room_id);
        if targets.is_empty() {
            self.by_source.remove(source_room_id);
        }
        removed
    }

    /// Keep the entries for which `keep(source room, target room)` is true.
    pub fn retain(&mut self, mut keep: impl FnMut(&str, &str) -> bool) {
        for (source, targets) in self.by_source.iter_mut() {
            targets.retain(|target, _| keep(source, target));
        }
        self.by_source.retain(|_, targets| !targets.is_empty());
    }

    /// Each entry, with the source room and the target room it is under.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str, &T)> {
        self.by_source.iter().flat_map(|(source, targets)| {
            targets
                .iter()
                .map(move |(target, entry)| (source.as_str(), target.as_str(), entry))
        })
    }

    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.by_source.values_mut().flat_map(BTreeMap::values_mut)
    }
}


/// Prepared construction plans for the rooms adjacent to each live room.
#[derive(Resource, Default, Debug)]
pub struct RoomConstructionPlanPrefetch {
    plans: PrefetchedByRoom<Arc<RoomConstructionPlan>>,
}

impl RoomConstructionPlanPrefetch {
    /// Publish a plan prepared for a neighbor of `identity`'s source room.
    ///
    /// A room that is a neighbour of two live rooms is published one time for
    /// each, with the one plan.
    pub fn publish(
        &mut self,
        identity: &PrefetchIdentity,
        room_id: &str,
        plan: Arc<RoomConstructionPlan>,
    ) {
        self.plans.insert(identity, room_id, plan);
    }

    /// The cached plan, without promoting it.
    ///
    /// ⛔ FOR INSPECTION ONLY. Promotion is [`Self::promote`] and it exists to
    /// refuse a plan prepared against a different world; a caller that reached
    /// past it would be taking exactly the plan those checks are about.
    pub fn peek(&self, source_room_id: &str, room_id: &str) -> Option<&Arc<RoomConstructionPlan>> {
        self.plans.peek(source_room_id, room_id)
    }

    /// True when a plan for `room_id` is published as a neighbour of
    /// `source_room_id` — the producer's "do I still need to build one"
    /// question.
    ///
    /// ⚠ IT DOES NOT ASK ABOUT THE WORLD, and its doc used to claim it did. It
    /// cannot: the world is whatever the last publication adopted, so a plan
    /// present here is by construction one prepared under it.
    pub fn holds(&self, source_room_id: &str, room_id: &str) -> bool {
        self.plans.peek(source_room_id, room_id).is_some()
    }

    pub fn forget(&mut self, source_room_id: &str, room_id: &str) {
        self.plans.remove(source_room_id, room_id);
    }

    /// Keep the plans for which `keep(source room, target room)` is true: the
    /// producer's retirement of the plans of a room that is no longer live.
    pub fn retain(&mut self, keep: impl FnMut(&str, &str) -> bool) {
        self.plans.retain(keep);
    }

    /// Promote only when session identity, target spec, and world outlook still
    /// match the prepared plan. A hot reload, session change, or custody/disposition
    /// change is a miss and must re-prepare against the current outlook.
    ///
    /// The plan is the one prepared for a neighbour of the source room of
    /// `identity`. The plans of a different source room stay.
    pub fn promote(
        &mut self,
        identity: &PrefetchIdentity,
        target: &RoomSpec,
        outlook: &RoomOccurrenceOutlook,
    ) -> Option<Arc<RoomConstructionPlan>> {
        let plan = self.plans.get(identity, &target.id)?;
        if !plan.matches_room_spec(target)
            || plan.session_scope().id() != identity.session_scope()
            || plan.occurrence_outlook() != outlook
        {
            return None;
        }
        Some(Arc::clone(plan))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn from(epoch: u64, source: &str) -> PrefetchIdentity {
        PrefetchIdentity::new(epoch, None, source)
    }

    /// The source room is a key and the world is a reset.
    ///
    /// Two live rooms each have entries. A read for one room keeps the entries
    /// of the other room. A read from a new content epoch, or from a new
    /// session, removes all of them.
    #[test]
    fn a_new_source_room_keeps_the_entries_and_a_new_world_clears_them() {
        let mut cache = PrefetchedByRoom::<u32>::default();
        cache.insert(&from(1, "alley"), "shaft", 10);
        cache.insert(&from(1, "relay"), "pipes", 20);
        // One target as a neighbour of two rooms is two entries.
        cache.insert(&from(1, "relay"), "shaft", 30);

        assert_eq!(cache.get(&from(1, "relay"), "pipes"), Some(&20));
        assert_eq!(
            cache.get(&from(1, "alley"), "shaft"),
            Some(&10),
            "a read for one source room removed the entry of a different source room"
        );
        assert_eq!(cache.get(&from(1, "relay"), "shaft"), Some(&30));
        // An entry is under its own source room only.
        assert_eq!(cache.get(&from(1, "alley"), "pipes"), None);
        assert_eq!(cache.iter().count(), 3, "a miss removed an entry");

        cache.retain(|source, _| source != "relay");
        assert_eq!(
            cache.iter().collect::<Vec<_>>(),
            vec![("alley", "shaft", &10)],
            "the retirement of one source room did not leave the entries of the other"
        );

        assert_eq!(cache.get(&from(2, "alley"), "shaft"), None, "an entry crossed a content epoch");
        assert_eq!(cache.iter().count(), 0);

        cache.insert(&from(2, "alley"), "shaft", 40);
        let other_session = PrefetchIdentity::new(2, Some(SessionScopeId(7)), "alley");
        assert_eq!(cache.get(&other_session, "shaft"), None, "an entry crossed a session");
        assert_eq!(cache.iter().count(), 0);
    }
}
