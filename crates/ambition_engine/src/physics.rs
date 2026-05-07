//! Data-level physics vocabulary for Ambition.
//!
//! The first Avian integration lives in the Bevy sandbox because it is runtime
//! presentation/experimentation code. These engine structs describe reusable
//! gameplay intent without forcing the custom player controller to become an
//! Avian rigid body. Future story crates can author breakable debris, enemy
//! ragdolls, physics props, or an experimental physics-controlled player in
//! terms of this vocabulary and let a backend plugin instantiate the concrete
//! physics components.

use crate::Vec2;

/// Coarse physical body behavior requested by authored game data.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PhysicsBodyKind {
    /// Does not move under simulation; used for room collision geometry.
    Static,
    /// Simulated by the physics backend; used for debris and ragdoll pieces.
    Dynamic,
    /// Moved by gameplay code while still participating in contact queries.
    Kinematic,
    /// Reports overlap/contact but does not physically block.
    Sensor,
}

/// Why a body exists in Ambition gameplay terms.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PhysicsBodyRole {
    WorldCollider,
    BreakableDebris,
    RagdollLimb,
    EnemyCorpse,
    BossDebris,
    PickupProp,
    PlayerPrototype,
}

/// Backend-neutral primitive shape. Keep this deliberately small until the game
/// needs richer convex decomposition.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PhysicsShape {
    Box { half_size: Vec2 },
    Circle { radius: f32 },
}

impl PhysicsShape {
    pub fn is_valid(self) -> bool {
        match self {
            Self::Box { half_size } => {
                half_size.x.is_finite()
                    && half_size.y.is_finite()
                    && half_size.x > 0.0
                    && half_size.y > 0.0
            }
            Self::Circle { radius } => radius.is_finite() && radius > 0.0,
        }
    }
}

/// Simple material hints that can be mapped to a concrete physics backend.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PhysicsMaterial {
    pub friction: f32,
    pub restitution: f32,
    pub density: f32,
}

impl Default for PhysicsMaterial {
    fn default() -> Self {
        Self {
            friction: 0.65,
            restitution: 0.25,
            density: 1.0,
        }
    }
}

impl PhysicsMaterial {
    pub fn debris() -> Self {
        Self {
            friction: 0.82,
            restitution: 0.32,
            density: 0.8,
        }
    }

    pub fn is_valid(self) -> bool {
        self.friction.is_finite()
            && self.restitution.is_finite()
            && self.density.is_finite()
            && self.friction >= 0.0
            && self.restitution >= 0.0
            && self.density > 0.0
    }
}

/// Reusable authored physics intent.
#[derive(Clone, Debug, PartialEq)]
pub struct PhysicsBodySpec {
    pub id: String,
    pub role: PhysicsBodyRole,
    pub body_kind: PhysicsBodyKind,
    pub shape: PhysicsShape,
    pub material: PhysicsMaterial,
    pub gravity_scale: f32,
    pub locked_rotation: bool,
}

impl PhysicsBodySpec {
    pub fn new(
        id: impl Into<String>,
        role: PhysicsBodyRole,
        body_kind: PhysicsBodyKind,
        shape: PhysicsShape,
    ) -> Self {
        Self {
            id: id.into(),
            role,
            body_kind,
            shape,
            material: PhysicsMaterial::default(),
            gravity_scale: 1.0,
            locked_rotation: false,
        }
    }

    pub fn debris(id: impl Into<String>, half_size: Vec2) -> Self {
        Self {
            id: id.into(),
            role: PhysicsBodyRole::BreakableDebris,
            body_kind: PhysicsBodyKind::Dynamic,
            shape: PhysicsShape::Box { half_size },
            material: PhysicsMaterial::debris(),
            gravity_scale: 1.0,
            locked_rotation: false,
        }
    }

    pub fn is_valid(&self) -> bool {
        !self.id.is_empty()
            && self.shape.is_valid()
            && self.material.is_valid()
            && self.gravity_scale.is_finite()
            && self.gravity_scale >= 0.0
    }
}

/// High-level ragdoll/debris recipe for enemies and bosses.
#[derive(Clone, Debug, PartialEq)]
pub struct RagdollSpec {
    pub id: String,
    pub role: PhysicsBodyRole,
    pub piece_count: usize,
    pub piece_half_size: Vec2,
    pub outward_impulse: f32,
    pub lifetime_seconds: f32,
}

impl RagdollSpec {
    pub fn enemy(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            role: PhysicsBodyRole::EnemyCorpse,
            piece_count: 5,
            piece_half_size: Vec2::new(6.0, 5.0),
            outward_impulse: 260.0,
            lifetime_seconds: 4.0,
        }
    }

    pub fn breakable(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            role: PhysicsBodyRole::BreakableDebris,
            piece_count: 7,
            piece_half_size: Vec2::new(5.0, 4.0),
            outward_impulse: 210.0,
            lifetime_seconds: 4.5,
        }
    }

    pub fn is_valid(&self) -> bool {
        !self.id.is_empty()
            && self.piece_count > 0
            && self.piece_half_size.x.is_finite()
            && self.piece_half_size.y.is_finite()
            && self.piece_half_size.x > 0.0
            && self.piece_half_size.y > 0.0
            && self.outward_impulse.is_finite()
            && self.outward_impulse >= 0.0
            && self.lifetime_seconds.is_finite()
            && self.lifetime_seconds > 0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debris_body_spec_is_valid() {
        let spec = PhysicsBodySpec::debris("crate_shard", Vec2::new(4.0, 3.0));
        assert!(spec.is_valid());
        assert_eq!(spec.body_kind, PhysicsBodyKind::Dynamic);
    }

    #[test]
    fn ragdoll_recipe_requires_positive_pieces() {
        let mut spec = RagdollSpec::enemy("enemy");
        assert!(spec.is_valid());
        spec.piece_count = 0;
        assert!(!spec.is_valid());
    }

    #[test]
    fn physics_shape_box_validates_half_size() {
        assert!(PhysicsShape::Box {
            half_size: Vec2::new(1.0, 1.0),
        }
        .is_valid());
        // Zero or negative half-size is invalid.
        assert!(!PhysicsShape::Box {
            half_size: Vec2::new(0.0, 1.0),
        }
        .is_valid());
        assert!(!PhysicsShape::Box {
            half_size: Vec2::new(-1.0, 1.0),
        }
        .is_valid());
        // NaN is rejected.
        assert!(!PhysicsShape::Box {
            half_size: Vec2::new(f32::NAN, 1.0),
        }
        .is_valid());
    }

    #[test]
    fn physics_shape_circle_validates_radius() {
        assert!(PhysicsShape::Circle { radius: 5.0 }.is_valid());
        assert!(!PhysicsShape::Circle { radius: 0.0 }.is_valid());
        assert!(!PhysicsShape::Circle { radius: -2.0 }.is_valid());
        assert!(!PhysicsShape::Circle {
            radius: f32::INFINITY,
        }
        .is_valid());
    }

    #[test]
    fn physics_material_default_is_finite_and_in_range() {
        let m = PhysicsMaterial::default();
        assert!(m.friction.is_finite() && m.friction >= 0.0);
        assert!(m.restitution.is_finite() && m.restitution >= 0.0);
        assert!(m.density.is_finite() && m.density > 0.0);
    }
}
