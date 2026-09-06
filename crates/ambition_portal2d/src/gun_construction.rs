//! Transactional construction owned by the optional portal-gun capability.
//!
//! Portal topology does not need a gun, and actor construction does not own a
//! portal-gun pickup. This module is the narrow bridge between those facts: it
//! owns the gun pickup's closed construction vocabulary and lowers an already
//! resolved pickup description onto the root allocated by the generic
//! construction executor.
//!
//! The room/composition layer may place this plan in a named construction lane
//! beside actor construction. Executable behavior remains a closed match here;
//! the shared construction schema catalog receives metadata only.

use bevy::prelude::{Name, Vec2};

use ambition_platformer2d_shared_tangle::construction::{
    ConstructionDomain, ConstructionExecCtx, ConstructionPlan, ConstructionRegistrationError,
    ConstructionRegistry, ConstructionRequest, ConstructionRoot, RecipeDispatch, RecipeId,
    RelationDispatch,
};
use ambition_platformer2d_shared_tangle::lifecycle::SpawnSessionScopedExt;

use crate::PortalGunPickup;

pub const PORTAL_GUN_CONSTRUCTION_DOMAIN: &str = "portal-gun";
pub const RECIPE_AUTHORED_PORTAL_GUN: &str = "ambition.authored-portal-gun";

const OWNER: &str = env!("CARGO_PKG_NAME");
const SCHEMA: &str = "portal-gun-construction-v1";

/// Fully resolved facts needed to construct one authored portal-gun pickup.
#[derive(Clone, Debug, PartialEq)]
pub struct PortalGunConstructionParams {
    pub name: String,
    pub pos: Vec2,
    pub half_extent: Vec2,
    /// Which portal pair the gun this pickup yields will own. `0` is the
    /// classic blue/orange gun; a level that wants a second, independently
    /// coloured gun places another pickup on a different pair.
    pub pair: u8,
}

/// Portal-gun pickup construction has no inter-root relation vocabulary.
#[derive(Clone, Copy, Debug)]
pub enum PortalGunConstructionRelation {}

/// Closed construction domain for portal-gun-owned authoritative roots.
pub struct PortalGunConstruction;

impl ConstructionDomain for PortalGunConstruction {
    type Parameters = PortalGunConstructionParams;
    type Relation = PortalGunConstructionRelation;
    type Services = ();

    fn dispatch(_parameters: &Self::Parameters) -> RecipeDispatch<Self> {
        RecipeDispatch {
            recipe: recipe_authored_portal_gun(),
            construct: construct_portal_gun_pickup,
        }
    }

    fn dispatch_relation(relation: &Self::Relation) -> RelationDispatch<Self> {
        match *relation {}
    }

    fn canonical_summary(parameters: &Self::Parameters) -> String {
        format!(
            "portal-gun-pickup name={:?} pos=({}, {}) half=({}, {})",
            parameters.name,
            parameters.pos.x,
            parameters.pos.y,
            parameters.half_extent.x,
            parameters.half_extent.y,
        )
    }

    fn canonical_relation_summary(relation: &Self::Relation) -> String {
        match *relation {}
    }
}

pub type PortalGunConstructionRegistry = ConstructionRegistry<PortalGunConstruction>;
pub type PortalGunConstructionPlan = ConstructionPlan<PortalGunConstruction>;
pub type PortalGunConstructionRequest = ConstructionRequest<PortalGunConstruction>;

type Ctx<'w, 's, 'a> = ConstructionExecCtx<'w, 's, 'a, PortalGunConstruction>;

pub fn recipe_authored_portal_gun() -> RecipeId {
    RecipeId::new(RECIPE_AUTHORED_PORTAL_GUN)
}

/// Closed registry for the gun capability's construction schema.
pub fn portal_gun_construction_registry() -> PortalGunConstructionRegistry {
    let mut registry = PortalGunConstructionRegistry::default();
    install_portal_gun_construction_recipes(&mut registry)
        .expect("the portal-gun construction schema cannot conflict with itself");
    registry
}

pub fn install_portal_gun_construction_recipes(
    registry: &mut PortalGunConstructionRegistry,
) -> Result<(), ConstructionRegistrationError> {
    registry.try_register_recipe(
        recipe_authored_portal_gun(),
        OWNER,
        "authored-room",
        SCHEMA,
    )
}

fn construct_portal_gun_pickup(
    parameters: &PortalGunConstructionParams,
    root: ConstructionRoot,
    ctx: &mut Ctx<'_, '_, '_>,
) {
    ctx.commands.insert_room_in_session(
        ctx.session,
        root.entity(),
        (
            Name::new(format!("Portal gun pickup: {}", parameters.name)),
            PortalGunPickup {
                pos: parameters.pos,
                half_extent: parameters.half_extent,
                // Authored pickups are immediately available. A dropped gun is
                // the host inventory adapter's runtime-dynamic object and keeps
                // its short anti-regrab arm delay there.
                arm_timer: 0.0,
                pair: parameters.pair,
            },
        ),
    );
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use bevy::prelude::{Vec2, World};

    use ambition_platformer2d_shared_tangle::construction::{
        AuthoritativeScope, ConstructionExecCtx, ConstructionLane, ConstructionScope,
        ContentBinding, SpawnOrigin, TransactionBaseline, verify_committed_roster,
    };
    use ambition_platformer2d_shared_tangle::lifecycle::SessionSpawnScope;
    use ambition_platformer2d_shared_tangle::sim_id::SimId;

    use super::{
        portal_gun_construction_registry, PortalGunConstructionParams,
        PortalGunConstructionPlan, PortalGunConstructionRequest,
        PORTAL_GUN_CONSTRUCTION_DOMAIN,
    };
    use crate::PortalGunPickup;

    #[test]
    fn authored_pickup_is_a_portal_owned_named_lane() {
        let request = PortalGunConstructionRequest {
            sim_id: SimId::placement("gun"),
            origin: SpawnOrigin::Authored {
                source: "room".to_string(),
                instance: "gun".to_string(),
            },
            parameters: PortalGunConstructionParams {
                name: "Aperture Device".to_string(),
                pos: Vec2::new(10.0, 20.0),
                half_extent: Vec2::new(8.0, 6.0),
                pair: 0,
            },
            relations: Vec::new(),
        };
        let registry = portal_gun_construction_registry();
        let plan = PortalGunConstructionPlan::prepare_in_lane(
            ConstructionScope {
                binding: ContentBinding::Content(ambition_platformer2d_core::ContentEpoch(1)),
                room: Some("room".to_string()),
            },
            ConstructionLane::named(PORTAL_GUN_CONSTRUCTION_DOMAIN),
            [request],
            &BTreeSet::new(),
            &registry,
        )
        .expect("portal-owned plan");

        let mut world = World::new();
        let baseline = TransactionBaseline::capture(&mut world).expect("clean baseline");
        let receipt = {
            let mut commands = world.commands();
            let mut ctx = ConstructionExecCtx {
                commands: &mut commands,
                scope: plan.scope(),
                session: SessionSpawnScope::UNSCOPED,
                services: &(),
            };
            plan.commit(&mut ctx)
        };
        world.flush();

        let transaction = plan.transaction(SessionSpawnScope::UNSCOPED);
        let scope = AuthoritativeScope::gather(&mut world, &transaction);
        verify_committed_roster(&plan, &receipt, &baseline, &scope, &world)
            .expect("portal lane verifies independently");

        let root = receipt.entity(&SimId::placement("gun")).expect("gun root");
        let pickup = world.get::<PortalGunPickup>(root).expect("pickup component");
        assert_eq!(pickup.pos, Vec2::new(10.0, 20.0));
        assert_eq!(pickup.arm_timer, 0.0);
        assert_eq!(plan.lane().as_str(), PORTAL_GUN_CONSTRUCTION_DOMAIN);
    }
}
