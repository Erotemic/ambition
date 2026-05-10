use super::*;

pub(super) fn player_is_standing_on(player: ae::Aabb, platform: ae::Aabb) -> bool {
    let horizontally_overlaps =
        player.right() > platform.left() + 2.0 && player.left() < platform.right() - 2.0;
    let near_top = (player.bottom() - platform.top()).abs() <= 8.0;
    horizontally_overlaps && near_top
}

pub(super) fn boss_space_is_free(world: &ae::World, pos: ae::Vec2, size: ae::Vec2) -> bool {
    let aabb = ae::Aabb::new(pos, size * 0.5);
    if aabb.left() < 0.0
        || aabb.right() > world.size.x
        || aabb.top() < 0.0
        || aabb.bottom() > world.size.y
    {
        return false;
    }
    !world.body_overlaps_any(aabb, |block| {
        matches!(
            block.kind,
            ae::BlockKind::Solid | ae::BlockKind::BlinkWall { .. } | ae::BlockKind::OneWay
        )
    })
}

pub(super) fn room_paths(world: &ae::World) -> Vec<(String, ae::KinematicPath)> {
    world
        .objects
        .iter()
        .filter_map(|object| match &object.kind {
            ae::RoomObjectKind::KinematicPath(path) => Some((object.id.clone(), path.clone())),
            _ => None,
        })
        .collect()
}

// Note: the older `blocked` / `blocked_y` predicates lived here.
// They were ad-hoc collision tests used by enemy / NPC sweep code,
// and their OneWay handling did not differentiate above-vs-below
// approaches — a hostile NPC chasing the player could not drop
// through a one-way platform, breaking the chase. Both paths now
// route through `ambition_engine::step_kinematic`, which mirrors
// the player's sweep semantics exactly. Don't reintroduce the
// old helpers; if a new caller needs collision-aware motion, add
// it through `KinematicBody`.

pub(super) fn approach(value: f32, target: f32, delta: f32) -> f32 {
    if value < target {
        (value + delta).min(target)
    } else {
        (value - delta).max(target)
    }
}

pub(super) fn approximately_same_aabb(a: ae::Aabb, b: ae::Aabb) -> bool {
    // Pogo-bounce routing matches an engine-reported orb AABB against
    // sandbox-side breakable AABBs. The two are derived from the same
    // entity placement so the values agree to floating-point tolerance,
    // but a tiny epsilon avoids spurious mismatches if a future codepath
    // recomputes one of the AABBs from rounded coordinates.
    let eps = 0.5;
    (a.center() - b.center()).length() <= eps && (a.half_size() - b.half_size()).length() <= eps
}

pub(super) fn midpoint(a: ae::Vec2, b: ae::Vec2) -> ae::Vec2 {
    ae::Vec2::new((a.x + b.x) * 0.5, (a.y + b.y) * 0.5)
}

/// Pick the SFX bank entry for a hazard contact based on the hazard's
/// authored name. Substring match keeps this resilient to naming
/// drift (e.g. `Lava Pit` and `lava_pool` both resolve to lava splash)
/// without coupling the engine to an SFX-asset enum. Falls back to a
/// generic player-damage clip when no keyword matches.
///
/// Long-term, a typed `HazardKind` field on the engine-side
/// `DamageVolume` (or `RoomObjectKind::Hazard`) would let this
/// dispatch happen on a real enum; until then the substring set is
/// short enough to grep.
pub(super) fn hazard_sfx_id(name: &str) -> ambition_sfx::SfxId {
    let n = name.to_ascii_lowercase();
    if n.contains("lava") {
        ambition_sfx::ids::HAZARD_LAVA_SPLASH
    } else if n.contains("acid") {
        ambition_sfx::ids::HAZARD_ACID_SPLASH
    } else if n.contains("electric") || n.contains("shock") {
        ambition_sfx::ids::HAZARD_ELECTRIC_ARC
    } else if n.contains("saw") {
        ambition_sfx::ids::HAZARD_SAW_HIT
    } else if n.contains("spike") || n.contains("thorn") {
        ambition_sfx::ids::HAZARD_SPIKE_HIT
    } else {
        ambition_sfx::ids::PLAYER_DAMAGE
    }
}

pub(super) trait SignumOr {
    fn signum_or(self, fallback: f32) -> f32;
}

impl SignumOr for f32 {
    fn signum_or(self, fallback: f32) -> f32 {
        if self.abs() <= 0.001 {
            fallback
        } else {
            self.signum()
        }
    }
}
