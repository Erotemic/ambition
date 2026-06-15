//! T-Rex seismic stomp boss-special Technique.
//!
//! Split out of the former 1.8k-line `specials.rs` (2026-06-15) — see
//! [`super`] (`specials/mod.rs`) for the shared module overview.

use super::*;

// ---- T-Rex's seismic stomp (content-only, open-seam; ground-hazard mechanic) ----

/// Content key for the T-Rex seismic stomp — matches the `Special("seismic_stomp")`
/// beats in `boss_profiles.ron`.
pub const SEISMIC_STOMP_KEY: &str = "seismic_stomp";

const SEISMIC_SEGMENTS_PER_SIDE: i32 = 5;
const SEISMIC_SPACING: f32 = 84.0;
const SEISMIC_HALF_EXTENT: ae::Vec2 = ae::Vec2::new(40.0, 26.0);
const SEISMIC_DAMAGE: i32 = 2;
const SEISMIC_KNOCKBACK: f32 = 1.8;
const SEISMIC_LIFETIME: f32 = 0.55;

/// Per-boss gate for the seismic stomp. One ground shockwave per strike.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct SeismicStompState {
    pub fired_this_strike: bool,
}

/// Pure: the x-offsets (relative to the boss) of the shock segments — a symmetric
/// line of ground boxes spreading both ways from the stomp, including the boss's
/// own tile (offset 0). Deterministic — the testable core of the Technique.
fn seismic_offsets(per_side: i32, spacing: f32) -> Vec<f32> {
    let per_side = per_side.max(0);
    (-per_side..=per_side).map(|i| i as f32 * spacing).collect()
}

/// Technique: T-Rex seismic stomp — a floor shockwave of short-lived damage boxes
/// spreading from the boss (content-only; open-seam special; ground hazard, not
/// projectiles). Jump the wave.
pub fn spawn_seismic_stomp_from_special_messages(
    mut effects: MessageWriter<EffectRequest>,
    mut messages: MessageReader<ActorActionMessage>,
    mut bosses: Query<(Entity, BossClusterRef, &mut SeismicStompState), With<FeatureSimEntity>>,
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
    for (entity, boss_feature, mut state) in &mut bosses {
        let boss = boss_feature.as_boss_ref();
        if !firing.contains(&entity) {
            state.fired_this_strike = false;
            continue;
        }
        if !boss.status.alive || state.fired_this_strike {
            continue;
        }
        // Anchor the shock at the boss's foot line so it reads as a ground wave.
        let foot_y = boss.aabb().max.y - SEISMIC_HALF_EXTENT.y;
        for dx in seismic_offsets(SEISMIC_SEGMENTS_PER_SIDE, SEISMIC_SPACING) {
            effects.write(EffectRequest {
                owner: entity,
                effect: Effect::DamageBox(ambition_sandbox::effects::DamageBoxEffect {
                    center: ae::Vec2::new(boss.kin.pos.x + dx, foot_y),
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
    use super::*;
    use super::super::*;

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
}
