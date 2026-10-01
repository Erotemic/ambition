//! The NATIVE gradient cascade, kept as the reference trace of its procedural
//! module (`ambition_content_modules::gradient_cascade`). Test-only: the game
//! runs the module. `module_parity_tests` holds the module to this system.

use bevy::prelude::*;

use ambition_boss_encounter::BossClusterRef;
use ambition_characters::brain::ActorActionMessage;
use ambition_platformer2d::actor::FeatureSimEntity;
use ambition_platformer2d_core as ae;

const GRADIENT_CASCADE_KEY: &str = "gradient_cascade";
const GRADIENT_CASCADE_MINION_COUNT: u8 = 2;

/// `gradient_cascade` is one-shot per strike (spawn N minions at strike
/// edge). State is just the "fired" gate plus a spawn counter for
/// unique minion ids.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct GradientCascadeState {
    pub fired_this_strike: bool,
    pub spawn_index: u32,
}

/// The cascade's minions are the AI slop, as the technique's design says
/// ("spawn N slop minions"). `npc_ai_slop` is a registered, body-complete
/// character, cast the same way as `MINIMA_TRAP_MINION_CHARACTER`.
///
/// Provisional: to recast, author the new character and change this string;
/// nothing else names it.
const GRADIENT_CASCADE_MINION_CHARACTER: &str = "npc_ai_slop";
const GRADIENT_CASCADE_MINION_HALF_SIZE: ae::Vec2 = ae::Vec2::new(15.0, 20.0);
/// Vertical y where slop minions spawn (top of the arena, just below
/// the ceiling). The arena ceiling sits at y=32; minions spawn at
/// y=80 so they're visibly inside the play space rather than clipping
/// the ceiling overlay.
const GRADIENT_CASCADE_SPAWN_Y: f32 = 80.0;
/// Horizontal spread (px from arena center) for spawning N minions.
const GRADIENT_CASCADE_X_SPREAD: f32 = 220.0;

/// Even horizontal offset (px from the boss x) for the `i`-th of
/// `count` gradient-cascade minions, spread across
/// `[-X_SPREAD, +X_SPREAD]`. A lone minion drops on the boss x; N≥2
/// place the first and last at the spread edges with even spacing
/// between. Pure so the spacing is unit-testable.
fn gradient_cascade_minion_x_offset(i: i32, count: i32) -> f32 {
    let t = if count <= 1 {
        0.5
    } else {
        i as f32 / (count - 1) as f32
    };
    (t - 0.5) * 2.0 * GRADIENT_CASCADE_X_SPREAD
}

/// EFFECTS consumer: `gradient_cascade` — spawn N "slop" minions at the
/// top of the arena.
///
/// One-shot per strike. Spawns `minion_count` slop minions in a
/// horizontal spread at `GRADIENT_CASCADE_SPAWN_Y`, centered on the
/// boss x. Gravity carries them down toward the player; the
/// character's own wanderer policy and contact damage do the rest.
pub fn spawn_gradient_cascade_minions_from_special_messages(
    mut effects: MessageWriter<ambition_vfx::EffectRequest>,
    mut messages: MessageReader<ActorActionMessage>,
    mut bosses: Query<
        (
            Entity,
            BossClusterRef,
            &ambition_characters::actor::BodyHealth,
            &mut GradientCascadeState,
        ),
        With<FeatureSimEntity>,
    >,
) {
    let minion_count = GRADIENT_CASCADE_MINION_COUNT;
    let firing = super::actors_firing(&mut messages, GRADIENT_CASCADE_KEY);

    for (entity, boss_feature, health, mut state) in &mut bosses {
        let boss = boss_feature.as_boss_ref();
        if !firing.contains_key(&entity) {
            // Strike closed — reset gate.
            state.fired_this_strike = false;
            continue;
        };
        if !health.alive() {
            continue;
        }
        if state.fired_this_strike {
            continue;
        }
        let count = minion_count.max(1) as i32;
        // Spread N minions evenly across [-X_SPREAD, +X_SPREAD] around
        // the boss x.
        // Encounter id = boss's canonical behavior id (see the
        // `minima_trap` consumer above for the name-vs-id rationale).
        let encounter_id = boss.config.behavior.id.clone();
        for i in 0..count {
            let x_off = gradient_cascade_minion_x_offset(i, count);
            let spawn_pos = ae::Vec2::new(boss.kin.pos.x + x_off, GRADIENT_CASCADE_SPAWN_Y);
            let minion_id = format!(
                "gradient_sentinel_cascade:{}:{}:{}",
                boss.config.id, state.spawn_index, i
            );
            effects.write(ambition_vfx::EffectRequest {
                owner: entity,
                effect: ambition_vfx::Effect::Summon(ambition_vfx::SummonSpec {
                    id: minion_id,
                    pos: spawn_pos,
                    half_size: GRADIENT_CASCADE_MINION_HALF_SIZE,
                    character_id: GRADIENT_CASCADE_MINION_CHARACTER.to_string(),
                    encounter_id: encounter_id.clone(),
                    faction: ambition_vfx::HitSide::Enemy,
                    // A boss drops a minion; nobody rides it.
                    ridden_by_summoner: None,
                    // The sentinel's minions keep the vitals their character authors.
                    health: None,
                    // A boss minion keeps its character's hazard.
                    keeps_contact_damage: true,
                }),
            });
        }
        state.fired_this_strike = true;
        state.spawn_index = state.spawn_index.wrapping_add(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gradient_cascade_minion_offsets_spread_symmetrically() {
        // A lone minion drops on the boss x.
        assert_eq!(gradient_cascade_minion_x_offset(0, 1), 0.0);
        // Two minions land on the spread edges.
        assert_eq!(
            gradient_cascade_minion_x_offset(0, 2),
            -GRADIENT_CASCADE_X_SPREAD
        );
        assert_eq!(
            gradient_cascade_minion_x_offset(1, 2),
            GRADIENT_CASCADE_X_SPREAD
        );
        // An odd count puts the middle minion on the boss x and the
        // ends symmetric about it.
        let n = 5;
        assert_eq!(gradient_cascade_minion_x_offset(2, n), 0.0);
        let first = gradient_cascade_minion_x_offset(0, n);
        let last = gradient_cascade_minion_x_offset(n - 1, n);
        assert!((first + last).abs() < 1e-3, "ends should be symmetric");
        assert_eq!(first, -GRADIENT_CASCADE_X_SPREAD);
        // Offsets increase monotonically and stay within the spread.
        let mut prev = f32::NEG_INFINITY;
        for i in 0..n {
            let x = gradient_cascade_minion_x_offset(i, n);
            assert!(x > prev, "offsets should be strictly increasing");
            assert!(x.abs() <= GRADIENT_CASCADE_X_SPREAD + 1e-3);
            prev = x;
        }
    }
}
