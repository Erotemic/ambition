//! T-Rex seismic stomp boss-special Technique.
//!
//! Split out of the former 1.8k-line `specials.rs` (2026-06-15) — see
//! [`super`] (`specials/mod.rs`) for the shared module overview.

use super::*;

// ---- T-Rex's seismic stomp (content-only, open-seam; world-space arena hazard) ----

/// Content key for the T-Rex seismic stomp — matches the `Special("seismic_stomp")`
/// beats in `boss_profiles.ron`.
pub const SEISMIC_STOMP_KEY: &str = "seismic_stomp";

const SEISMIC_SEGMENTS_PER_SIDE: i32 = 5;
const SEISMIC_SPACING: f32 = 84.0;
const SEISMIC_HALF_EXTENT: ae::Vec2 = ae::Vec2::new(40.0, 26.0);
const SEISMIC_DAMAGE: i32 = 2;
const SEISMIC_KNOCKBACK: f32 = 1.8;
const SEISMIC_LIFETIME: f32 = 0.55;

/// Per-boss gate for the seismic stomp. One world-floor shockwave per strike.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct SeismicStompState {
    pub fired_this_strike: bool,
}

/// Pure: the world-x offsets (relative to the boss) of the shock segments — a
/// symmetric line of arena-floor boxes spreading both ways from the stomp,
/// including the boss's own tile (offset 0). Deterministic — the testable core
/// of the Technique.
fn seismic_offsets(per_side: i32, spacing: f32) -> Vec<f32> {
    let per_side = per_side.max(0);
    (-per_side..=per_side).map(|i| i as f32 * spacing).collect()
}

/// Pure world-space arena geometry for the seismic stomp.
///
/// This is intentionally **not** a support-relative / acceleration-frame
/// mechanic. The T-Rex stomp is authored as a boss-room hazard that runs along
/// the arena's world floor line, so the geometry names that frame explicitly
/// instead of smuggling a gameplay "floor" label into block content.
fn seismic_world_floor_centers(
    boss_anchor_x: f32,
    boss_aabb: ae::Aabb,
    per_side: i32,
    spacing: f32,
) -> Vec<ae::Vec2> {
    let foot_y = boss_aabb.max.y - SEISMIC_HALF_EXTENT.y;
    seismic_offsets(per_side, spacing)
        .into_iter()
        .map(|dx| ae::Vec2::new(boss_anchor_x + dx, foot_y))
        .collect()
}

/// Technique: T-Rex seismic stomp — a world-space arena-floor shockwave of
/// short-lived damage boxes spreading from the boss (content-only; open-seam
/// special; room hazard, not projectiles). Jump the wave.
pub fn spawn_seismic_stomp_from_special_messages(
    mut effects: MessageWriter<EffectRequest>,
    mut messages: MessageReader<ActorActionMessage>,
    mut bosses: Query<(Entity, BossClusterRef, &ambition_characters::actor::BodyHealth, &mut SeismicStompState), With<FeatureSimEntity>>,
) {
    let mut firing: std::collections::HashSet<Entity> = std::collections::HashSet::new();
    for msg in messages.read() {
        if let ActionRequest::Special {
            spec: SpecialActionSpec::Special(key),
        } = &msg.request
        {
            if key == SEISMIC_STOMP_KEY {
                firing.insert(msg.actor);
            }
        }
    }
    for (entity, boss_feature, health, mut state) in &mut bosses {
        let boss = boss_feature.as_boss_ref();
        if !firing.contains(&entity) {
            state.fired_this_strike = false;
            continue;
        }
        if !health.alive() || state.fired_this_strike {
            continue;
        }
        for center in seismic_world_floor_centers(
            boss.kin.pos.x,
            boss.aabb(),
            SEISMIC_SEGMENTS_PER_SIDE,
            SEISMIC_SPACING,
        ) {
            effects.write(EffectRequest {
                owner: entity,
                effect: Effect::DamageBox(ambition_vfx::DamageBoxEffect {
                    center,
                    faction: ActorFaction::Boss,
                    half_extent: SEISMIC_HALF_EXTENT,
                    damage: SEISMIC_DAMAGE,
                    knockback: SEISMIC_KNOCKBACK,
                    lifetime_s: SEISMIC_LIFETIME,
                    name: None,
                }),
            });
        }
        state.fired_this_strike = true;
    }
}

#[cfg(test)]
mod tests {
    use super::super::*;
    use super::*;

    #[test]
    fn seismic_offsets_are_symmetric_and_include_the_boss_tile() {
        let offsets = seismic_offsets(5, 84.0);
        assert_eq!(offsets.len(), 11, "5 each side + the boss tile");
        assert!(offsets.contains(&0.0), "the boss's own tile shocks too");
        // Symmetric about 0 and strictly increasing.
        for (a, b) in offsets.iter().zip(offsets.iter().rev()) {
            assert!((a + b).abs() < 1e-3, "symmetric about the boss");
        }
        let mut prev = f32::NEG_INFINITY;
        for &x in &offsets {
            assert!(x > prev, "offsets strictly increase outward");
            prev = x;
        }
        assert_eq!(
            seismic_offsets(0, 84.0),
            vec![0.0],
            "degenerate is just the tile"
        );
    }

    #[test]
    fn seismic_centers_are_explicit_world_floor_arena_geometry() {
        let boss_anchor_x = 300.0;
        let boss_aabb = ae::Aabb::new(ae::Vec2::new(300.0, 400.0), ae::Vec2::new(48.0, 72.0));
        let centers = seismic_world_floor_centers(boss_anchor_x, boss_aabb, 1, 84.0);
        assert_eq!(centers.len(), 3);

        let expected_y = boss_aabb.max.y - SEISMIC_HALF_EXTENT.y;
        assert_eq!(centers[0].x, boss_anchor_x - 84.0);
        assert_eq!(centers[1].x, boss_anchor_x);
        assert_eq!(centers[2].x, boss_anchor_x + 84.0);
        assert!(centers.iter().all(|p| (p.y - expected_y).abs() < 1e-3));

        // The stomp is a room-authored world-floor wave, not a derived support
        // relation from the boss's or target's acceleration frame. Keeping the
        // pure helper free of gravity/control-frame inputs is part of the
        // contract: if this changes, the hazard needs a different name and tests.
    }
}
