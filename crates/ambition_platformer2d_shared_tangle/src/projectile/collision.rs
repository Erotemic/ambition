//! Projectile contacts use the common material cast and one response.

use ae::frame::material_cast::{MaterialCast, MaterialCastHit};
use ae::frame::thread::{MaterialPrimitive, MotionAdmission, Refusal, RefusalReason};
use ae::frame::{NumericFault, Sim2};
use ae::world::{MaterialCollider, MaterialWorldHit};
use ae::{BodyKinematics, Vec2};
use ambition_platformer2d_core as ae;

use super::body::ProjectileGameplay;

/// World contact behavior is a property of the shot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorldHitPolicy {
    /// Bounce on support contacts while the budget permits it. Other solid
    /// contacts expire. One-way surfaces admit only support approaches.
    Bouncing,
    /// Expire on solid and one-way contact.
    ExpireOnContact,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WorldHitOutcome {
    Bounced { pos: Vec2 },
    Expired { pos: Vec2 },
    Continue,
}

/// All state is private to the candidate until the caller selects this contact.
#[derive(Debug)]
pub struct ProjectileWorldCollision<'w> {
    pub contact: Option<MaterialWorldHit<'w>>,
    pub body: BodyKinematics,
    pub bounces_remaining: u8,
    pub outcome: WorldHitOutcome,
}

fn numeric(reason: NumericFault) -> Refusal {
    Refusal {
        path: Vec::new(),
        link: None,
        reason: RefusalReason::Numeric(reason),
    }
}

fn at_contact(contact: &MaterialCastHit, reason: NumericFault) -> Refusal {
    let mut path = contact.path.clone();
    let link = path.pop();
    Refusal {
        path,
        link,
        reason: RefusalReason::Numeric(reason),
    }
}

fn surface_velocity(
    collider: MaterialCollider<'_>,
    contact: &MaterialCastHit,
    dt: f32,
    admission: Option<MotionAdmission<'_>>,
) -> Result<Vec2, Refusal> {
    let velocity = collider
        .velocity_at(contact.world_point, dt)
        .map_err(|reason| at_contact(contact, reason))?;
    match admission {
        Some(admission) => admission.uncarry_velocity(&contact.path, contact.world_point, velocity),
        None => Ok(velocity),
    }
}

/// Cast a translated body during `time`, an interval of the world tick.
/// Blocks use their published displacement. Return the selected collider
/// and a complete candidate response. A refusal changes no body state.
#[allow(clippy::too_many_arguments)]
pub fn resolve_world_collision<'w>(
    before: BodyKinematics,
    after: BodyKinematics,
    game: ProjectileGameplay,
    world: &'w ae::World,
    gravity_dir: Vec2,
    dt: f32,
    time:std::ops::Range<f64>,
    admission: Option<MotionAdmission<'_>>,
) -> Result<ProjectileWorldCollision<'w>, Refusal> {
    if !before.vel.is_finite()
        || !after.vel.is_finite()
        || !gravity_dir.is_finite()
        || !before.canonical_size.is_finite()
    {
        return Err(numeric(NumericFault::NonFinite));
    }
    if before.canonical_size.min_element() <= 0.0 {
        return Err(numeric(NumericFault::InvalidScale));
    }
    if before.material != after.material || before.canonical_size != after.canonical_size {
        return Err(numeric(NumericFault::InvalidFrame));
    }
    let half = before.canonical_size * 0.5;
    if half.min_element() == 0.0 {
        return Err(numeric(NumericFault::Underflow));
    }
    let shape = ae::CombatVolume::aabb(ae::Aabb::new(Vec2::ZERO, half));
    let delta = Sim2::IDENTITY
        .vector_from_wide(after.pos.as_dvec2() - before.pos.as_dvec2())
        .map_err(numeric)?;
    let cast = MaterialCast {
        time,
        material: MaterialPrimitive {
            domain: &shape,
            to_world: before.material_map(Vec2::ZERO).map_err(numeric)?,
        },
        root: before.pos,
        delta,
    };
    let down = super::body::projectile_down(gravity_dir);
    let contact = world.first_material_sweep(cast, admission, |collider, _, hit| {
        let one_way = match collider {
            MaterialCollider::Block(block) => {
                if !ae::collision_semantics::is_full_collision_surface(block.kind)
                    && block.kind != ae::BlockKind::OneWay
                {
                    return Ok(false);
                }
                block.kind == ae::BlockKind::OneWay
            }
            MaterialCollider::Chain { chain, segment, .. } => {
                if chain.kind == ae::world::SurfaceKind::Ground {
                    let (a, b) = chain.segment(segment);
                    let edge = b - a;
                    let normal = Vec2::new(edge.y, -edge.x).normalize_or_zero();
                    let owner_normal = hit
                        .map
                        .inverse()
                        .map_err(|reason| at_contact(hit, reason))?
                        .direction(normal);
                    if hit.normal.dot(owner_normal) < 0.5 {
                        return Ok(false);
                    }
                    true
                } else {
                    false
                }
            }
        };
        if one_way && game.world_hit == WorldHitPolicy::Bouncing {
            let surface = surface_velocity(collider, hit, dt, admission)?;
            let approach = (after.vel.as_dvec2() - surface.as_dvec2()).dot(hit.normal.as_dvec2());
            return Ok(game.bounces_remaining > 0 && hit.normal.dot(down) < -0.5 && approach < 0.0);
        }
        Ok(true)
    })?;
    let mut result = ProjectileWorldCollision {
        contact,
        body: after,
        bounces_remaining: game.bounces_remaining,
        outcome: WorldHitOutcome::Continue,
    };
    let Some(hit) = result.contact.as_ref() else {
        return Ok(result);
    };
    let surface = surface_velocity(hit.collider, &hit.contact, dt, admission)?;
    let normal = hit.contact.normal.as_dvec2();
    let relative = after.vel.as_dvec2() - surface.as_dvec2();
    let normal_speed = relative.dot(normal);
    result.body.pos = Sim2::IDENTITY
        .point_from_wide(before.pos.as_dvec2() + delta.as_dvec2() * hit.contact.toi)
        .map_err(|reason| at_contact(&hit.contact, reason))?;
    let point = hit.contact.world_point;
    if game.world_hit == WorldHitPolicy::Bouncing
        && game.bounces_remaining > 0
        && hit.contact.normal.dot(down) < -0.5
        && normal_speed < 0.0
    {
        let rebound = game.rebound;
        if !rebound.restitution.is_finite() || !rebound.min_speed.is_finite() {
            return Err(at_contact(&hit.contact, NumericFault::NonFinite));
        }
        let outward = (-normal_speed * f64::from(rebound.restitution))
            .max(f64::from(rebound.min_speed) * f64::from(before.material.scale())).max(0.0);
        result.body.vel = Sim2::IDENTITY
            .vector_from_wide(
                relative - normal * normal_speed + normal * outward + surface.as_dvec2(),
            )
            .map_err(|reason| at_contact(&hit.contact, reason))?;
        result.bounces_remaining -= 1;
        result.outcome = WorldHitOutcome::Bounced { pos: point };
    } else {
        result.outcome = WorldHitOutcome::Expired { pos: point };
    }
    Ok(result)
}

#[cfg(test)]
mod tests;
