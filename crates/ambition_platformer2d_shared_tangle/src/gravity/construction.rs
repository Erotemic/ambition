//! Transactional construction owned by the gravity capability.
//!
//! An authored gravity zone is not an actor, and it never was: its constructor lowers a
//! resolved region and direction into [`GravityZone`] plus an optional [`OscillatingZone`],
//! both of which are defined one module up.
//!
//! this module adds no dependency edge. The construction machinery
//! ([`crate::construction`]), the session-scoped spawn extension
//! ([`crate::lifecycle::SpawnSessionScopedExt`]) and the components being
//! constructed all already live in this crate, so the domain lands beside the
//! runtime capability it builds rather than beside a crate that had to grow an
//! edge to reach it.
//!
//! the parameters are RESOLVED, not the room's spec. `GravityZoneSpec`
//! lives in `ambition_platformer2d_world`, which depends on this crate — taking
//! it here would invert that. The room adapter translates instead, which is the
//! same shape the portal-gun lane uses and the same reason.

use bevy::prelude::{Name, Vec2};

use super::{GravityZone, OscillatingZone};
use crate::construction::{
    ConstructionDomain, ConstructionExecCtx, ConstructionPlan, ConstructionRegistrationError,
    ConstructionRegistry, ConstructionRequest, ConstructionRoot, RecipeDispatch, RecipeId,
    RelationDispatch,
};
use crate::lifecycle::SpawnSessionScopedExt;

pub const GRAVITY_ZONE_CONSTRUCTION_DOMAIN: &str = "gravity-zone";
pub const RECIPE_AUTHORED_GRAVITY_ZONE: &str = "ambition.authored-gravity-zone";

const OWNER: &str = env!("CARGO_PKG_NAME");
const SCHEMA: &str = "gravity-zone-construction-v1";

/// Fully resolved facts needed to construct one authored gravity zone.
#[derive(Clone, Debug, PartialEq)]
pub struct GravityZoneConstructionParams {
    pub name: String,
    /// World-space centre of the region.
    pub center: Vec2,
    /// Half-extent of the region.
    pub half_extent: Vec2,
    /// Gravity direction inside the region.
    pub dir: Vec2,
    /// Horizontal slide amplitude in px. Zero means a static column — the
    /// zone is built without an [`OscillatingZone`] at all, which is the
    /// difference between a wall of gravity and one riding a platform.
    pub oscillate_amplitude: f32,
    /// Slide frequency, read only when the amplitude is non-zero.
    pub oscillate_freq: f32,
}

/// A gravity zone stands alone: it has no inter-root relation vocabulary.
#[derive(Clone, Copy, Debug)]
pub enum GravityZoneConstructionRelation {}

/// Closed construction domain for gravity-owned authoritative roots.
pub struct GravityZoneConstruction;

impl ConstructionDomain for GravityZoneConstruction {
    type Parameters = GravityZoneConstructionParams;
    type Relation = GravityZoneConstructionRelation;
    type Services = ();

    fn dispatch(_parameters: &Self::Parameters) -> RecipeDispatch<Self> {
        RecipeDispatch {
            recipe: recipe_authored_gravity_zone(),
            construct: construct_gravity_zone,
        }
    }

    fn dispatch_relation(relation: &Self::Relation) -> RelationDispatch<Self> {
        match *relation {}
    }

    fn canonical_summary(parameters: &Self::Parameters) -> String {
        // every field, because this string is fingerprint material: a zone
        // whose direction or slide changed is a different world, and a summary
        // that omitted them would call the two rooms identical.
        format!(
            "gravity-zone name={:?} center=({}, {}) half=({}, {}) dir=({}, {}) \
             oscillate=({}, {})",
            parameters.name,
            parameters.center.x,
            parameters.center.y,
            parameters.half_extent.x,
            parameters.half_extent.y,
            parameters.dir.x,
            parameters.dir.y,
            parameters.oscillate_amplitude,
            parameters.oscillate_freq,
        )
    }

    fn canonical_relation_summary(relation: &Self::Relation) -> String {
        match *relation {}
    }
}

pub type GravityZoneConstructionRegistry = ConstructionRegistry<GravityZoneConstruction>;
pub type GravityZoneConstructionPlan = ConstructionPlan<GravityZoneConstruction>;
pub type GravityZoneConstructionRequest = ConstructionRequest<GravityZoneConstruction>;

type Ctx<'w, 's, 'a> = ConstructionExecCtx<'w, 's, 'a, GravityZoneConstruction>;

pub fn recipe_authored_gravity_zone() -> RecipeId {
    RecipeId::new(RECIPE_AUTHORED_GRAVITY_ZONE)
}

/// Closed registry for the gravity capability's construction schema.
pub fn gravity_zone_construction_registry() -> GravityZoneConstructionRegistry {
    let mut registry = GravityZoneConstructionRegistry::default();
    install_gravity_zone_construction_recipes(&mut registry)
        .expect("the gravity-zone construction schema cannot conflict with itself");
    registry
}

pub fn install_gravity_zone_construction_recipes(
    registry: &mut GravityZoneConstructionRegistry,
) -> Result<(), ConstructionRegistrationError> {
    registry.try_register_recipe(
        recipe_authored_gravity_zone(),
        OWNER,
        "authored-room",
        SCHEMA,
    )
}

fn construct_gravity_zone(
    parameters: &GravityZoneConstructionParams,
    root: ConstructionRoot,
    ctx: &mut Ctx<'_, '_, '_>,
) {
    let mut entity = ctx.commands.insert_room_in_session(
        ctx.session,
        root.entity(),
        (
            Name::new(format!("Gravity zone: {}", parameters.name)),
            GravityZone {
                aabb: ambition_platformer2d_core::Aabb::new(
                    parameters.center,
                    parameters.half_extent,
                ),
                dir: parameters.dir,
            },
        ),
    );
    // A non-zero amplitude makes the column slide horizontally (the sliding
    // gravity demo); a static column omits the OscillatingZone entirely, so
    // `collect_gravity_zones` sees a region that never moves.
    if parameters.oscillate_amplitude > 0.0 {
        entity.insert(OscillatingZone {
            base_center: parameters.center,
            half: parameters.half_extent,
            amplitude_x: parameters.oscillate_amplitude,
            freq: parameters.oscillate_freq,
            phase: 0.0,
        });
    }
}
