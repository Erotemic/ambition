//! Service-free capability construction lanes composed beside the actor lane.
//!
//! Lanes are named struct fields, not a runtime registry. Shared generic helpers
//! keep preparation, dumps, verification, reconstruction, and commit consistent
//! across every lane. `ConstructionDomain<Services = ()>` excludes the actor lane,
//! which requires frozen construction catalogs.

use bevy::prelude::Commands;

use ambition_platformer2d_shared_tangle::construction::{
    verify_committed_roster, AuthoritativeScope, ConstructionDomain, ConstructionExecCtx,
    ConstructionLane, ConstructionPlan, ConstructionReceipt, ConstructionScope, ContentBinding,
    RosterViolation, TransactionBaseline,
};
use ambition_platformer2d_shared_tangle::gravity::construction as gravity_domain;
use ambition_platformer2d_shared_tangle::sim_id::SimId;
use std::collections::BTreeSet;

use super::SessionSpawnScope;

/// Prepare one service-free capability lane.
fn prepare_lane<D>(
    scope: ConstructionScope,
    domain_name: &'static str,
    requests: Vec<ambition_platformer2d_shared_tangle::construction::ConstructionRequest<D>>,
    suppressed: &BTreeSet<SimId>,
    registry: &ambition_platformer2d_shared_tangle::construction::ConstructionRegistry<D>,
) -> Result<ConstructionPlan<D>, ambition_platformer2d_shared_tangle::construction::ConstructionError>
where
    D: ConstructionDomain<Services = ()>,
{
    ConstructionPlan::prepare_in_lane(
        scope,
        ConstructionLane::named(domain_name),
        requests,
        suppressed,
        registry,
    )
}

/// Append one lane's canonical dump under its domain header.
fn dump_lane<D: ConstructionDomain>(
    out: &mut String,
    domain_name: &'static str,
    plan: &ConstructionPlan<D>,
) {
    use std::fmt::Write as _;
    let body = plan.deterministic_dump();
    let _ = writeln!(out, "domain\t{domain_name}\t{}", body.len());
    out.push_str(&body);
}

/// Commit one service-free capability lane, and assert it committed what it
/// planned.
fn commit_lane<D>(
    plan: &ConstructionPlan<D>,
    commands: &mut Commands,
    session: SessionSpawnScope,
    domain_name: &'static str,
) -> ConstructionReceipt
where
    D: ConstructionDomain<Services = ()>,
{
    let mut ctx = ConstructionExecCtx {
        commands,
        scope: plan.scope(),
        session,
        services: &(),
    };
    let receipt = plan.commit(&mut ctx);
    debug_assert_eq!(
        receipt.committed_ids(),
        plan.planned_ids(),
        "the `{domain_name}` lane diverged from its prepared roster",
    );
    receipt
}

/// Verify one lane against the transaction baseline every lane shares.
fn verify_lane<D: ConstructionDomain>(
    plan: &ConstructionPlan<D>,
    receipt: &ConstructionReceipt,
    baseline: &TransactionBaseline,
    world: &mut bevy::prelude::World,
    session: SessionSpawnScope,
    violations: &mut Vec<RosterViolation>,
) {
    let transaction = plan.transaction(session);
    let scope = AuthoritativeScope::gather(world, &transaction);
    violations.extend(
        verify_committed_roster(plan, receipt, baseline, &scope, world)
            .err()
            .unwrap_or_default(),
    );
}

/// Rebuild one authoritative root from this lane, if the lane planned it.
///
/// `None` means "not mine" — the caller keeps asking. `Some(false)` means the
/// lane owns the identity and could not rebuild it, which is a different answer
/// and must not be confused with the first.
fn respawn_from_lane<D>(
    plan: &ConstructionPlan<D>,
    sim_id: &SimId,
    commands: &mut Commands,
    session: SessionSpawnScope,
    domain_name: &'static str,
) -> Option<bool>
where
    D: ConstructionDomain<Services = ()>,
{
    plan.get(sim_id)?;
    let closure = plan.relation_closure(&BTreeSet::from([sim_id.clone()]));
    let mut ctx = ConstructionExecCtx {
        commands,
        scope: plan.scope(),
        session,
        services: &(),
    };
    Some(match plan.commit_subset(&closure, &mut ctx) {
        Ok(_) => true,
        Err(error) => {
            bevy::log::error!(
                target: "ambition_platformer2d::construction",
                "`{sim_id}` is planned in the `{domain_name}` lane but its reconstruction \
                 closure could not be rebuilt: {error}"
            );
            false
        }
    })
}

/// The service-free capability lanes of one room plan.
#[derive(Clone)]
pub(crate) struct CapabilityLanes {
    /// not optional and not feature-gated: every composition has gravity,
    /// so this lane proves the federation shape works for a capability that is
    /// simply always present.
    gravity:
        ambition_platformer2d_shared_tangle::gravity::construction::GravityZoneConstructionPlan,
    #[cfg(feature = "portal")]
    portal: ambition_portal2d::PortalGunConstructionPlan,
}

/// What those lanes committed. One field per lane, same list, same rule.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CapabilityReceipts {
    gravity: ConstructionReceipt,
    #[cfg(feature = "portal")]
    portal: ConstructionReceipt,
}

impl CapabilityLanes {
    /// Plan every capability lane against the same scope, outlook and
    /// suppression set the actor lane was planned against.
    pub(crate) fn prepare(
        scope: &ConstructionScope,
        room: &ambition_platformer2d_world::rooms::RoomSpec,
        outlook: &ambition_platformer2d_shared_tangle::lifecycle::RoomOccurrenceOutlook,
        suppressed: &BTreeSet<SimId>,
    ) -> Result<Self, ambition_platformer2d_shared_tangle::construction::ConstructionError> {
        use ambition_platformer2d_shared_tangle::gravity::construction as gravity;

        let gravity = prepare_lane(
            scope.clone(),
            gravity::GRAVITY_ZONE_CONSTRUCTION_DOMAIN,
            super::gravity_construction::authored_requests(room, outlook),
            suppressed,
            &gravity::gravity_zone_construction_registry(),
        )?;
        #[cfg(feature = "portal")]
        let portal = prepare_lane(
            scope.clone(),
            ambition_portal2d::PORTAL_GUN_CONSTRUCTION_DOMAIN,
            super::portal_construction::authored_requests(room, outlook),
            suppressed,
            &ambition_portal2d::portal_gun_construction_registry(),
        )?;

        Ok(Self {
            gravity,
            #[cfg(feature = "portal")]
            portal,
        })
    }

    /// Claim every lane's planned identities into the room's predicted roster.
    ///
    /// composing the roster and detecting collisions are the SAME call, so
    /// a lane that is composed is a lane that is checked — see `claim` in the
    /// parent module for why that replaced a pairwise intersection.
    pub(crate) fn claim_planned_ids(
        &self,
        room: &str,
        roster: &mut BTreeSet<String>,
    ) -> Result<(), super::RoomFeatureConstructionError> {
        let Self {
            gravity,
            #[cfg(feature = "portal")]
            portal,
        } = self;
        super::claim_lane_ids(room, &gravity.planned_ids(), roster)?;
        #[cfg(feature = "portal")]
        super::claim_lane_ids(room, &portal.planned_ids(), roster)?;
        Ok(())
    }

    /// Append every lane to the room's canonical dump, in field order.
    pub(crate) fn write_deterministic_dump(&self, out: &mut String) {
        use ambition_platformer2d_shared_tangle::gravity::construction as gravity_domain;
        let Self {
            gravity,
            #[cfg(feature = "portal")]
            portal,
        } = self;
        dump_lane(
            out,
            gravity_domain::GRAVITY_ZONE_CONSTRUCTION_DOMAIN,
            gravity,
        );
        #[cfg(feature = "portal")]
        dump_lane(
            out,
            ambition_portal2d::PORTAL_GUN_CONSTRUCTION_DOMAIN,
            portal,
        );
    }

    /// Every lane was planned against the same content generation as the actor
    /// lane. A lane that disagreed would commit rows prepared against content
    /// nobody else can see.
    pub(crate) fn debug_assert_binding(&self, binding: ContentBinding) {
        let Self {
            gravity,
            #[cfg(feature = "portal")]
            portal,
        } = self;
        debug_assert_eq!(gravity.scope().binding, binding);
        #[cfg(feature = "portal")]
        debug_assert_eq!(portal.scope().binding, binding);
        let _ = binding;
    }

    pub(crate) fn commit(
        &self,
        commands: &mut Commands,
        session: SessionSpawnScope,
    ) -> CapabilityReceipts {
        let Self {
            gravity,
            #[cfg(feature = "portal")]
            portal,
        } = self;
        CapabilityReceipts {
            gravity: commit_lane(
                gravity,
                commands,
                session,
                gravity_domain::GRAVITY_ZONE_CONSTRUCTION_DOMAIN,
            ),
            #[cfg(feature = "portal")]
            portal: commit_lane(
                portal,
                commands,
                session,
                ambition_portal2d::PORTAL_GUN_CONSTRUCTION_DOMAIN,
            ),
        }
    }

    /// the operation a new lane is likeliest to be left out of, because
    /// omitting it costs nothing at commit time and only shows up as a roster
    /// nobody checked. The destructure is what makes leaving it out impossible.
    pub(crate) fn verify(
        &self,
        receipts: &CapabilityReceipts,
        baseline: &TransactionBaseline,
        world: &mut bevy::prelude::World,
        session: SessionSpawnScope,
        violations: &mut Vec<RosterViolation>,
    ) {
        let Self {
            gravity,
            #[cfg(feature = "portal")]
            portal,
        } = self;
        verify_lane(
            gravity,
            &receipts.gravity,
            baseline,
            world,
            session,
            violations,
        );
        #[cfg(feature = "portal")]
        verify_lane(
            portal,
            &receipts.portal,
            baseline,
            world,
            session,
            violations,
        );
    }

    /// Ask each lane in turn whether it owns this identity, and rebuild it if so.
    pub(crate) fn respawn(
        &self,
        sim_id: &SimId,
        commands: &mut Commands,
        session: SessionSpawnScope,
    ) -> Option<bool> {
        let Self {
            gravity,
            #[cfg(feature = "portal")]
            portal,
        } = self;
        if let Some(outcome) = respawn_from_lane(
            gravity,
            sim_id,
            commands,
            session,
            gravity_domain::GRAVITY_ZONE_CONSTRUCTION_DOMAIN,
        ) {
            return Some(outcome);
        }
        #[cfg(feature = "portal")]
        if let Some(outcome) = respawn_from_lane(
            portal,
            sim_id,
            commands,
            session,
            ambition_portal2d::PORTAL_GUN_CONSTRUCTION_DOMAIN,
        ) {
            return Some(outcome);
        }
        None
    }

    /// `cfg(test)`, both of these: production never asks a room WHICH LANE
    /// built something. It asks for the roster, and each lane verifies itself
    /// against the shared baseline. A lane accessor exists so a test can prove an
    /// identity lives in one lane and not another — a claim only a test makes.
    #[cfg(test)]
    pub(crate) fn gravity(
        &self,
    ) -> &ambition_platformer2d_shared_tangle::gravity::construction::GravityZoneConstructionPlan
    {
        &self.gravity
    }

    #[cfg(all(test, feature = "portal"))]
    pub(crate) fn portal(&self) -> &ambition_portal2d::PortalGunConstructionPlan {
        &self.portal
    }
}

impl CapabilityReceipts {
    /// Extend the room's committed roster with every lane's committed ids.
    pub(crate) fn extend_committed_ids(&self, roster: &mut BTreeSet<String>) {
        let Self {
            gravity,
            #[cfg(feature = "portal")]
            portal,
        } = self;
        roster.extend(gravity.committed_ids().iter().map(ToString::to_string));
        #[cfg(feature = "portal")]
        roster.extend(portal.committed_ids().iter().map(ToString::to_string));
    }
}
