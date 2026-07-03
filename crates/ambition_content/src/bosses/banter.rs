//! Boss combat-banter lines + idle-bark ticker.
//!
//! Scholar-on-shoulder quotes for GNU-ton: voiced primarily as Isaac
//! Newton — cold, obsessive, theological, experimental, petty,
//! cosmic. The GNU joke lives in the name and art only; the *voice*
//! is the man who chased coiners to the gallows, ground his eye
//! socket with a bodkin to study optics, and counted prophecy in
//! arithmetic. Idle barks fire between strikes during attacking
//! phases (Phase1/2/Enrage) so the scholar reads as a present,
//! opinionated character even when not under attack.

use std::collections::HashMap;

use bevy::prelude::*;

use crate::banter::CombatBanterRegistry;
use ambition_gameplay_core::features::BossClusterRef;
use ambition_render::fx::VfxMessage;

/// Register hit-bark + idle-bark lines for all boss encounters.
pub fn install_boss_banter(registry: &mut CombatBanterRegistry) {
    // GNU-ton — the scholar atop the giant wildebeest, voiced as
    // Isaac Newton. Hit barks are sharp, mathematical, and a little
    // furious — the man does not enjoy being interrupted mid-proof.
    // Name key must match the LDtk BossSpawn entity name exactly.
    registry.set_hit_barks(
        "GNU-ton",
        vec![
            "Counterfeit.",
            "You ring false.",
            "Assayed. Found wanting.",
            "Your defiance has mass.",
            "Pain is data.",
            "Impact is merely mathematics becoming honest.",
            "The hypothesis has failed. Begin dissection.",
            "Your continued existence is statistically irritating.",
            "I tire of imprecision.",
        ],
    );
    // Idle barks: muttered between strikes so the scholar feels alive
    // even when not under attack. Fired by `tick_boss_idle_barks` while
    // the boss is in an attacking phase. Pulls from the full
    // Newton-voice menu — optics, alchemy, theology, the Mint — so
    // repeat encounters sound layered rather than thematic-stuck.
    registry.set_idle_barks(
        "GNU-ton",
        vec![
            // The five Jon-favorited lines lead the pool so they
            // surface first in a fresh fight.
            "Fall. I have already solved for where.",
            "The universe is not cruel. It is exact.",
            "I have split the sun. I can divide you.",
            "I know false weight when it breathes.",
            "Your continued existence is statistically irritating.",
            // Cosmic / mechanics.
            "Your motion was inevitable.",
            "I require no prophecy to know your end.",
            "Do not resist the law. You are made of it.",
            "I have measured the heavens. You are a rounding error.",
            "Every body attracts every other. Unfortunately for you, I am very large.",
            "You are not escaping. You are tracing a curve.",
            "Be still. I am observing.",
            "Hold steady. The experiment requires repetition.",
            // Optics.
            "White light concealed its colors better than you conceal your fear.",
            "I pressed behind the eye and found truth in the bruise.",
            "Look closer. Closer than sight allows.",
            "The prism does not lie. It only reveals the lie.",
            "Darkness is light refusing examination.",
            "Your shadow has an angle. Your death has a proof.",
            // Alchemy.
            "Lead dreams of gold. Flesh dreams of dust.",
            "The furnace corrects all impurities.",
            "Mercury remembers the shape of every sin.",
            "Be transmuted.",
            "The Green Lion has tasted nobler metals than you.",
            "I sought the stone. I found better uses for pressure.",
            "Your blood has interesting properties.",
            "All things refine under sufficient heat.",
            "The world is a crucible, and you are not the gold.",
            "Impurity is just matter confessing.",
            // Theology / prophecy.
            "History is a cipher. You are a solved term.",
            "The end is not near. It is numbered.",
            "I do not predict. I calculate.",
            "Even apocalypse obeys arithmetic.",
            "The beasts of Daniel were better documented than you.",
            "God wrote in number. I learned to read.",
            "Creation is not chaos. It is a ledger.",
            "You mistake mystery for permission.",
            "I have found no scripture in which you survive.",
            "The stars keep their stations. Learn from them.",
            // Mint / judgment.
            "You are light coin.",
            "Clipped, debased, and due for hanging.",
            "Bring the scales.",
            "The stamp shall reveal your value.",
            "I have sent better men to the gallows for less.",
            "Fraud always leaves a shaving.",
            // Phase / enrage — read fine as idle mutters too.
            "Enough observation. Now comes proof.",
            "You have survived the question. Face the answer.",
            "Let us remove the remaining variables.",
            "The visible world is exhausted. Proceed to the hidden one.",
            "I lower my head only to inspect the damage.",
            "Come then. Address your objection to my face.",
            "The calculation accelerates.",
            "No more approximations.",
            "Let the heavens fall inward.",
            "Now every law acts at once.",
            "I have been patient as a clock. No longer.",
            "Your margin of error is closed.",
            "The proof concludes.",
        ],
    );
}

/// Per-boss idle-bark cadence state. Lives in a `Local` on the ticker
/// system so it doesn't need its own resource.
#[derive(Default)]
pub struct BossIdleBarkState {
    /// Seconds remaining until the next idle bark fires, keyed by
    /// `BossConfig::id`. Initialized on first observation to a
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
    world_time: Res<ambition_time::WorldTime>,
    registry: Option<Res<CombatBanterRegistry>>,
    ecs_bosses: Query<(BossClusterRef, &ambition_characters::actor::BodyHealth, &ambition_characters::actor::BodyCombat)>,
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
    for (feature, health, combat) in &ecs_bosses {
        let boss = feature.as_boss_ref();
        if !health.alive() {
            continue;
        }
        if !matches!(
            boss.status.encounter_phase,
            ambition_gameplay_core::boss_encounter::BossEncounterPhase::Phase1
                | ambition_gameplay_core::boss_encounter::BossEncounterPhase::Phase2
                | ambition_gameplay_core::boss_encounter::BossEncounterPhase::Enrage
        ) {
            continue;
        }
        // Suppress idle barks while the hit-flash bubble is still on
        // screen so we don't talk over a hit bark.
        if combat.hit_flash > 0.0 {
            continue;
        }
        let key = boss.config.id.clone();
        let timer = state
            .timers
            .entry(key.clone())
            .or_insert_with(|| jitter_interval(&boss.config.id, 0));
        *timer -= dt;
        if *timer > 0.0 {
            continue;
        }
        let rotation_now = *state.rotations.entry(key.clone()).or_insert(0);
        if let Some(line) = registry.pick_idle_bark(&boss.config.name, rotation_now) {
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
            .insert(key, jitter_interval(&boss.config.id, next_rotation));
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
