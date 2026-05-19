//! Boss combat-banter lines + idle-bark ticker.
//!
//! Scholar-on-shoulder quotes for GNU-ton: pedantic, recursive,
//! self-referential. Lines should sound like someone who has read every
//! GNU manual and missed the point. Idle barks fire between strikes
//! during attacking phases (Phase1/2/Enrage) so the scholar reads as a
//! present, opinionated character even when not under attack.

use std::collections::HashMap;

use bevy::prelude::*;

use crate::content::banter::CombatBanterRegistry;
use crate::features::BossFeature;
use crate::presentation::fx::VfxMessage;
use ambition_engine as ae;

/// Register hit-bark + idle-bark lines for all boss encounters.
pub fn install_boss_banter(registry: &mut CombatBanterRegistry) {
    // GNU-ton — the scholar atop the giant wildebeest.
    // Hit barks: pompous, recursive, free-software militant.
    // Name key must match the LDtk BossSpawn entity name exactly.
    registry.set_hit_barks(
        "GNU-ton",
        vec![
            "That's GPL v3, not v2! Read the license!",
            "I wrote a 47-page rebuttal to that attack!",
            "I can see further than everyone else — standing on the shoulders of giants!",
            "It's not Linux, it's GNU slash Linux!",
            "My wildebeest is free as in freedom — free as in freedom!",
            "I was going to finish the Hurd kernel, but then you hit me!",
            "Violence is closed source!",
            "I have a recursive name. You have a recursive kick.",
            "Have you considered contributing a patch instead?",
        ],
    );
    // Idle barks: muttered between strikes so the scholar feels alive
    // even when not under attack. Fired by `tick_boss_idle_barks` while
    // the boss is in an attacking phase.
    registry.set_idle_barks(
        "GNU-ton",
        vec![
            "Did you know GNU is a recursive acronym? GNU's Not Unix.",
            "Excuse me, but it's GNU slash Linux, actually.",
            "Standing on the shoulders of giants. Literally.",
            "I once filed a 200-page bug report against the universe.",
            "'Free as in freedom', not 'free as in beer'. Common mistake.",
            "Have you read the manifesto? It's only 41 pages.",
            "The kernel is fine; the problem is the userspace.",
            "I'm porting myself to Hurd. It's almost ready. Almost.",
            "Software wants to be free. My wildebeest wants to be left alone.",
            "I am the maintainer of this wildebeest.",
            "Every commit a theorem. Every theorem a footnote.",
            "Have you considered switching to Emacs?",
        ],
    );
}

/// Per-boss idle-bark cadence state. Lives in a `Local` on the ticker
/// system so it doesn't need its own resource.
#[derive(Default)]
pub struct BossIdleBarkState {
    /// Seconds remaining until the next idle bark fires, keyed by
    /// `BossFeature::boss.id`. Initialized on first observation to a
    /// small jitter so multiple bosses don't bark in unison.
    timers: HashMap<String, f32>,
    /// Rotation counter per boss so the same line doesn't loop.
    rotations: HashMap<String, u32>,
}

const IDLE_BARK_INTERVAL_BASE: f32 = 7.0;
const IDLE_BARK_INTERVAL_JITTER: f32 = 3.0;

/// Fire idle barks for active bosses on a 5-10s cadence while they are
/// in an attacking phase. Skips bosses with no idle lines registered.
pub fn tick_boss_idle_barks(
    world_time: Res<crate::WorldTime>,
    registry: Option<Res<CombatBanterRegistry>>,
    ecs_bosses: Query<&BossFeature>,
    mut vfx: MessageWriter<VfxMessage>,
    mut state: Local<BossIdleBarkState>,
) {
    let Some(registry) = registry else {
        return;
    };
    let dt = world_time.scaled_dt;
    if dt <= 0.0 {
        return;
    }
    for feature in &ecs_bosses {
        let boss = &feature.boss;
        if !boss.alive {
            continue;
        }
        if !matches!(
            boss.encounter_phase,
            ae::BossEncounterPhase::Phase1
                | ae::BossEncounterPhase::Phase2
                | ae::BossEncounterPhase::Enrage
        ) {
            continue;
        }
        // Suppress idle barks while the hit-flash bubble is still on
        // screen so we don't talk over a hit bark.
        if boss.hit_flash > 0.0 {
            continue;
        }
        let key = boss.id.clone();
        let timer = state
            .timers
            .entry(key.clone())
            .or_insert_with(|| jitter_interval(&boss.id, 0));
        *timer -= dt;
        if *timer > 0.0 {
            continue;
        }
        let rotation_now = *state.rotations.entry(key.clone()).or_insert(0);
        if let Some(line) = registry.pick_idle_bark(&boss.name, rotation_now) {
            vfx.write(VfxMessage::SpeechBubble {
                pos: boss.bark_anchor(),
                text: line.to_string(),
            });
        }
        let next_rotation = rotation_now.wrapping_add(1);
        state.rotations.insert(key.clone(), next_rotation);
        // Reset the timer with fresh jitter so the next bark doesn't
        // land at exactly the same offset.
        state
            .timers
            .insert(key, jitter_interval(&boss.id, next_rotation));
    }
}

/// Cheap deterministic jitter keyed by boss id + counter — avoids
/// pulling in `rand` for one cadence offset. Returns a value in
/// `[IDLE_BARK_INTERVAL_BASE - IDLE_BARK_INTERVAL_JITTER,
///   IDLE_BARK_INTERVAL_BASE + IDLE_BARK_INTERVAL_JITTER]`.
fn jitter_interval(seed: &str, counter: u32) -> f32 {
    let mut hash: u32 = counter.wrapping_mul(2_654_435_761);
    for byte in seed.bytes() {
        hash = hash.wrapping_add(byte as u32).wrapping_mul(16_777_619);
    }
    let unit = (hash >> 8) as f32 / (u32::MAX >> 8) as f32; // 0..=1
    IDLE_BARK_INTERVAL_BASE + (unit * 2.0 - 1.0) * IDLE_BARK_INTERVAL_JITTER
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jitter_interval_stays_within_band() {
        for counter in 0..20 {
            let v = jitter_interval("GNU-ton", counter);
            assert!(
                v >= IDLE_BARK_INTERVAL_BASE - IDLE_BARK_INTERVAL_JITTER
                    && v <= IDLE_BARK_INTERVAL_BASE + IDLE_BARK_INTERVAL_JITTER,
                "jitter out of band: counter={counter} v={v}"
            );
        }
    }

    #[test]
    fn jitter_interval_is_deterministic_per_seed() {
        // Same (seed, counter) always returns the same value — needed
        // so the timer reset doesn't flicker each frame.
        let a = jitter_interval("GNU-ton", 3);
        let b = jitter_interval("GNU-ton", 3);
        assert_eq!(a, b);
    }
}
