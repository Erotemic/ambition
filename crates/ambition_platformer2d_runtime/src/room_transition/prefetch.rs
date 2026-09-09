//! The construction half of neighboring-room prefetch.
//!
//! A prepared [`RoomConstructionPlan`] is an ENGINE artifact keyed by engine
//! identity — content epoch, session scope, and the room you are standing in —
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
}

/// Prepared construction plans for the rooms adjacent to the one in play.
#[derive(Resource, Default, Debug)]
pub struct RoomConstructionPlanPrefetch {
    identity: Option<PrefetchIdentity>,
    plans: BTreeMap<String, Arc<RoomConstructionPlan>>,
}

impl RoomConstructionPlanPrefetch {
    /// Adopt `identity`, dropping everything prepared under a different one.
    ///
    /// Private, and both public entry points call it: a promotion can never read
    /// across an epoch/scope/source boundary even if the producer has not run
    /// since the change, and a publication can never leave the cache claiming an
    /// identity its plans were not prepared under.
    fn adopt(&mut self, identity: &PrefetchIdentity) {
        if self.identity.as_ref() == Some(identity) {
            return;
        }
        self.identity = Some(identity.clone());
        self.plans.clear();
    }

    /// Publish a plan prepared for a neighbor of `identity`'s source room.
    pub fn publish(
        &mut self,
        identity: &PrefetchIdentity,
        room_id: &str,
        plan: Arc<RoomConstructionPlan>,
    ) {
        self.adopt(identity);
        self.plans.insert(room_id.to_string(), plan);
    }

    /// The cached plan, without promoting it.
    ///
    /// ⛔ FOR INSPECTION ONLY. Promotion is [`Self::promote`] and it exists to
    /// refuse a plan prepared against a different world; a caller that reached
    /// past it would be taking exactly the plan those checks are about.
    pub fn peek(&self, room_id: &str) -> Option<&Arc<RoomConstructionPlan>> {
        self.plans.get(room_id)
    }

    /// True when a plan for this room is already published — the producer's "do
    /// I still need to build one" question.
    ///
    /// ⚠ IT DOES NOT ASK ABOUT IDENTITY, and its doc used to claim it did. It
    /// cannot: the identity is whatever the last publication adopted, so a plan
    /// present here is by construction one prepared under it.
    pub fn holds(&self, room_id: &str) -> bool {
        self.plans.contains_key(room_id)
    }

    pub fn forget(&mut self, room_id: &str) {
        self.plans.remove(room_id);
    }

    /// Promote only when session identity, target spec, and world outlook still
    /// match the prepared plan. A hot reload, session change, or custody/disposition
    /// change is a miss and must re-prepare against the current outlook.
    pub fn promote(
        &mut self,
        identity: &PrefetchIdentity,
        target: &RoomSpec,
        outlook: &RoomOccurrenceOutlook,
    ) -> Option<Arc<RoomConstructionPlan>> {
        self.adopt(identity);
        let plan = self.plans.get(&target.id)?;
        if !plan.matches_room_spec(target)
            || plan.session_scope().id() != identity.session_scope()
            || plan.occurrence_outlook() != outlook
        {
            return None;
        }
        Some(Arc::clone(plan))
    }
}
