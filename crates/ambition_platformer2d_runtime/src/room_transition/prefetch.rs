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

use std::collections::BTreeMap;
use std::sync::Arc;

use bevy::prelude::Resource;

use ambition_platformer2d_actor_monolith::rooms::{RoomConstructionPlan, RoomSpec};
use ambition_platformer2d_shared_tangle::lifecycle::{RoomOccurrenceOutlook, SessionScopeId};

/// Prepared construction plans for the rooms adjacent to the one in play.
#[derive(Resource, Default, Debug)]
pub struct RoomConstructionPlanPrefetch {
    content_epoch: u64,
    session_scope: Option<SessionScopeId>,
    source_room_id: Option<String>,
    plans: BTreeMap<String, Arc<RoomConstructionPlan>>,
}

impl RoomConstructionPlanPrefetch {
    /// Drop everything prepared under a different identity.
    ///
    /// Called by both the producer and the consumer, so a promotion can never
    /// read across an epoch/scope/source boundary even if the producer has not
    /// run since the change.
    pub fn reset_for(
        &mut self,
        content_epoch: u64,
        session_scope: Option<SessionScopeId>,
        source_room_id: &str,
    ) {
        if self.content_epoch == content_epoch
            && self.session_scope == session_scope
            && self.source_room_id.as_deref() == Some(source_room_id)
        {
            return;
        }
        self.content_epoch = content_epoch;
        self.session_scope = session_scope;
        self.source_room_id = Some(source_room_id.to_string());
        self.plans.clear();
    }

    /// Publish a plan prepared for a neighbor of the current source room.
    pub fn publish(&mut self, room_id: &str, plan: Arc<RoomConstructionPlan>) {
        self.plans.insert(room_id.to_string(), plan);
    }

    /// True when a plan for this room is already prepared under the current
    /// identity — the host's "do I still need to build one" question.
    pub fn holds(&self, room_id: &str) -> bool {
        self.plans.contains_key(room_id)
    }

    pub fn forget(&mut self, room_id: &str) {
        self.plans.remove(room_id);
    }

    /// Promote the exact immutable artifact prepared for this target, if one
    /// was prepared under identity that still holds AND still describes the
    /// target spec. A same-id hot reload or a session change is a miss.
    ///
    /// ⭐ **and so is a plan prepared against different DISPOSITIONS.** A room
    /// build asks where the occurrences it minted before actually are, and a
    /// plan froze that answer: one prepared while an authored object was being
    /// carried deliberately OMITS it, and committing that plan after the object
    /// was put down and destroyed would leave the room permanently short of a
    /// thing it authors. `outlook` is what the world remembers NOW; a plan that
    /// was prepared against anything else is a miss, which costs one preparation
    /// and is what the miss path is for.
    ///
    /// ⚠ **the outlook and not a suppressed-identity set, because the answer now
    /// carries a POSITION.** A plan that reinstated a relocated object where it
    /// used to lie names the same identity as one that reinstates it where it
    /// lies now; comparing identities alone would promote the stale one.
    pub fn promote(
        &mut self,
        content_epoch: u64,
        session_scope: Option<SessionScopeId>,
        source_room_id: &str,
        target: &RoomSpec,
        outlook: &RoomOccurrenceOutlook,
    ) -> Option<Arc<RoomConstructionPlan>> {
        self.reset_for(content_epoch, session_scope, source_room_id);
        let plan = self.plans.get(&target.id)?;
        if !plan.matches_room_spec(target)
            || plan.session_scope().id() != session_scope
            || plan.occurrence_outlook() != outlook
        {
            return None;
        }
        Some(Arc::clone(plan))
    }
}
