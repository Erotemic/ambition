//! Shared Brain + ActionSet construction for ECS feature actors.
//!
//! Spawning, mount/rider transitions, and hostile NPC flips should all come
//! through this module when they need to attach or replace actor brain
//! components. Keeping the construction policy here prevents each caller from
//! hand-rolling a slightly different mix of archetype tuning, aggressiveness,
//! and per-actor jitter.

use super::actor_clusters::ActorConfig;
use super::variation::{five_f32s_from_seed, seed_from_id};
use super::{CombatKit, HeldItem};
use crate::features::ecs::actor_tuning::{ActorTuning, BrainProfile, CharacterBrainTemplate};
use ambition_characters::brain::{
    ActionSet, Brain, ChargeCrashCfg, ChargeCrashState, MeleeBruteCfg, MeleeBruteState,
    SkirmisherCfg, SkirmisherState, SmashCfg, SmashState, SniperCfg, SniperState, StateMachineCfg,
    WandererCfg,
};

/// **WHAT A BODY THAT AUTHORED NO FIGHTING KIT FIGHTS WITH.**
///
/// ⭐⭐ **two scaffolds in this campaign are the SAME missing authority, and this
/// is it** (found 2026-08-12 by measuring both). `smash_fighter_kit()` grants one
/// generic swipe to any seated fighter whose character says nothing (P3.24), and
/// the PROVOCATION path hands a peaceful body a whole archetype for the same
/// reason (P2.20) — a Hall NPC authors `peaceful`, so without a granted kit a
/// provoked one would have nothing to swing. Both are "a default fighting kit",
/// spelled twice, and neither could be deleted while the concept had no name.
///
/// ⚠ **the numbers are `combatant`'s, verbatim and verified.**
/// `hostile_brain_id_for_actor()` returns the literal `"combatant"` — its last
/// matcher arm was deleted with the characters that answered it — so every body
/// reaching the provocation fallback already gets exactly this. The test below
/// asserts the two are equal rather than trusting the transcription, and it is
/// the equivalence baseline the roster's own deletion needs.
///
/// ⛔ this is a FALLBACK, not a design. Every character that authors its own
/// repertoire stops consuming it, which is the same falling-adopter-count P3.24
/// measures — and when the count is zero this function is deleted, not retuned.
///
/// ⛔⛔ **AND IT IS NOT THE ONLY ONE, WHICH IS THE POINT.** Smash's
/// `smash_fighter_kit()` answers the same question with different numbers —
/// `0.22/0.08/0.26`, 4 damage, 34 reach, against this one's `0.28/0.08/0.32`,
/// 1 damage, 28 reach. Faster, harder, longer: a platform fighter's floor rather
/// than an exploration provoke. **Merging them would retune a mode while wearing
/// a refactor's commit.**
///
/// ⇒ what the pair proves is that this default belongs to the SESSION RULESET —
/// the campaign's third authority — and not to the engine or to any character. A
/// stage states what an unarmed fighter swings for; a room states something else;
/// neither is a fact about a body. Naming it here is the step that made the
/// question askable, not the final home.
pub(crate) fn default_fighting_kit() -> CombatKit {
    CombatKit {
        innate_melee: Some(ambition_characters::brain::MeleeActionSpec::Swipe(
            ambition_characters::brain::SwipeSpec {
                windup_s: 0.28,
                active_s: 0.08,
                recover_s: 0.32,
                damage: 1,
                reach_px: 28.0,
            },
        )),
        innate_ranged: None,
        innate_special: None,
        move_style: ambition_characters::brain::MoveStyleSpec::Walk,
    }
}

/// **THE POLICY A BODY IS DRIVEN BY WHEN IT IS PROVOKED AND SAYS NOTHING.**
///
/// ⭐ the twin of [`default_fighting_kit`] one authority over: that one answers
/// *what does it swing*, this one answers *how does it fight*. They were the two
/// halves the `combatant` archetype row was doing at once, and separating them
/// is what lets the row die — a body is not a policy, and neither is a kit.
///
/// ⛔ **what this replaces is a ROSTER LOOKUP inside provocation.** The generic
/// branch called `spec_for_brain(Custom("combatant"))` to get a `BrainProfile`,
/// which is the last reason that path knew the archetype ontology existed at
/// all. `an_engine_default_provoked_policy_matches_the_combatant_row` pins the
/// numbers against the row while the row survives; when it goes, the constant
/// stands alone and nothing has to change.
///
/// ⚠ **its home is the SESSION RULESET, not here** — the same journey
/// `unarmed_melee` took (P3.24): named in the engine first so the question is
/// askable, moved to `DeclaredCombatRules` once a second experience wants a
/// different answer. A stage that wants provoked bodies to fight differently
/// says so there; nothing says so yet.
///
/// ⛔ deliberately NOT a ranged policy. `medium_striker` carried a thrown rock,
/// and using it here turned every provoked NPC — the kernel guide, a merchant —
/// into a rock-thrower instead of a melee attacker like the pirates.
pub(crate) fn default_provoked_policy() -> crate::features::ecs::actor_tuning::BrainProfile {
    crate::features::ecs::actor_tuning::BrainProfile {
        template: ambition_characters::brain::CharacterBrainTemplate::Smash,
        aggro_radius: 460.0,
        attack_range: 150.0,
        patrol_effort: 0.6774,
        chase_effort: 1.0,
        ..Default::default()
    }
}

// ⛔ `DEFAULT_PROVOKED_HEALTH: i32 = 4` stood here, and naming it *provoked* was
// the tell: a health pool supplied by provocation is a body mutation whatever
// number it holds (D101). The constant is
// `ambition_characters::actor::DEFAULT_UNAUTHORED_BODY_HEALTH` now — the pool a
// body gets when no authority describes it, asked at construction — and
// provocation no longer writes health at all. The VALUE is unchanged and D96
// item 7 still owns it.

// ⛔⛔ **`enemy_combat_kit_for_spec` WAS HERE AND IS DELETED (AC6)** with its two
// siblings `enemy_default_action_set` and `held_item_for_spec`. All three read a
// body's weapons off an `ArchetypeSpec` — the melee timings, the ranged spec, the
// gait, the held item, the signature move — and their one production caller was
// `EnemyActorSpawnPlan::hostile`, which asked them only when the seed carried an
// archetype. A character-first seed never did, so they had already stopped
// answering for any shipped body before the type they read went.
//
// ⇒ what a body fights with comes from its CHARACTER, through the one persona
// writer (`grant_prepared_character_body`).

pub(super) fn action_set_from_combat_kit(
    kit: &CombatKit,
    held_item: Option<&HeldItem>,
) -> ActionSet {
    kit.to_action_set(held_item.map(|item| &item.spec))
}

/// Build the enemy's default `Brain` from its resolved controller profile.
pub(crate) fn enemy_default_brain(
    enemy: &ActorConfig,
    // ⭐ **THE BODY'S OWN VERBS**, not a policy's opinion of them. See
    // [`smash_cfg_from_spec`]: a driver may only consider what this body can
    // actually do, so the same profile on a different body produces a driver
    // that reaches for different things.
    body: ambition_platformer2d_core::AbilitySet,
) -> Brain {
    match enemy.brain_profile.template {
        CharacterBrainTemplate::StandStill => Brain::StateMachine(StateMachineCfg::StandStill),
        CharacterBrainTemplate::Fighter => {
            let level = enemy.brain_profile.fighter_level;
            let cfg = ambition_characters::brain::fighter::FighterCfg::new(
                ambition_characters::brain::fighter::FighterBrainProfile::for_level(level),
            );
            let state = ambition_characters::brain::fighter::FighterState::new(
                &cfg,
                // Seeded from the LEVEL, so two fighters on one rung are the same
                // fighter and a replay reproduces both. A clock-seeded stream
                // would make the brain the one part of the sim that does not
                // rewind.
                0x5F37_7A11_u64.wrapping_mul(level as u64 + 1),
            );
            Brain::StateMachine(StateMachineCfg::Fighter {
                cfg: Box::new(cfg),
                state: Box::new(state),
            })
        }
        CharacterBrainTemplate::Wanderer => Brain::StateMachine(StateMachineCfg::Wanderer {
            cfg: WandererCfg::PUPPY_SLUG_DEFAULT,
        }),
        CharacterBrainTemplate::MeleeBrute => melee_brute_brain_for_enemy(enemy),
        CharacterBrainTemplate::ChargeCrash => charge_crash_brain_for_enemy(enemy),
        CharacterBrainTemplate::Skirmisher => skirmisher_brain_for_enemy(enemy),
        CharacterBrainTemplate::Sniper => sniper_brain_for_enemy(enemy),
        CharacterBrainTemplate::Smash => Brain::StateMachine(StateMachineCfg::Smash {
            cfg: smash_cfg_from_spec(&enemy.brain_profile, &enemy.tuning, body),
            state: SmashState {
                rng_seed: seed_from_id(&enemy.id) as u64,
                ..Default::default()
            },
        }),
        CharacterBrainTemplate::Aerial => aerial_brain_for_enemy(enemy),
    }
}

/// Build the hostile aerial dive-bomber brain for an enemy archetype (the sky
/// parrot). Per-actor jitter keeps a flock from diving in lockstep. Shares
/// `StateMachineCfg::Aerial` with the peaceful catalog bird — only
/// `aggressiveness` differs.
fn aerial_brain_for_enemy(enemy: &ActorConfig) -> Brain {
    let t = &enemy.tuning;
    let jitters = five_f32s_from_seed(seed_from_id(&enemy.id));
    let cruise_speed = t.chase_speed * (0.55 + 0.25 * jitters.0);
    let dive_speed = (t.chase_speed * (1.7 + 0.5 * jitters.1)).max(360.0);
    // Dive altitude / range: a bit of spread so two parrots stack their dives.
    let roam_radius = (110.0 + 60.0 * jitters.2).max(enemy.brain_profile.attack_range * 1.5);
    Brain::StateMachine(StateMachineCfg::Aerial {
        cfg: ambition_characters::brain::state_machine::AerialCfg {
            aggressiveness: if t.is_hostile { 1.0 } else { 0.0 },
            cruise_speed,
            dive_speed,
            aggro_radius: enemy.brain_profile.aggro_radius,
            attack_range: enemy.brain_profile.attack_range,
            roam_radius,
        },
        state: ambition_characters::brain::state_machine::AerialState::default(),
    })
}

/// Build the explicitly-hostile behavior for an actor that is peaceful by
/// default but has been provoked in play. Default spawn still uses
/// [`enemy_default_brain`] so cove PirateHeavy variants remain peaceful until
/// struck; this override gives them the same concrete heavy swing/capability
/// once the hostility flag is set.
pub(super) fn aggressive_brain_and_action_set_for_enemy(
    enemy: &ActorConfig,
    kit: &CombatKit,
    held_item: Option<&HeldItem>,
    body: ambition_platformer2d_core::AbilitySet,
) -> (Brain, ActionSet) {
    let action_set = action_set_from_combat_kit(kit, held_item);

    // Held-item capability is the high-level behavior selector for explicitly
    // aggressive actors: a ranged-only weapon wants a spacing brain, while a
    // melee-capable actor should close and swing. If a future pirate is authored
    // with a bow / bomb / pistol and no melee slot, this path becomes a
    // Skirmisher without a Rust-side item-id branch. If it has an axe / sword /
    // body melee slot, the grounded melee brain wins so point-blank targets are
    // attacked instead of kited.
    if action_set.ranged.is_some() && action_set.melee.is_none() {
        return (
            skirmisher_brain_from_tuning(&enemy.id, &enemy.tuning, &enemy.brain_profile, true),
            action_set,
        );
    }

    if let Some(min_aggro) = enemy.brain_profile.provoke_forced_brute_min_aggro {
        let brain = forced_hostile_melee_brute_brain(enemy, min_aggro);
        return (brain, action_set);
    }
    (enemy_default_brain(enemy, body), action_set)
}

fn forced_hostile_melee_brute_brain(enemy: &ActorConfig, min_aggro_radius: f32) -> Brain {
    let t = &enemy.tuning;
    let jitters = five_f32s_from_seed(seed_from_id(&enemy.id));
    let aggro_radius =
        enemy.brain_profile.aggro_radius.max(min_aggro_radius) * (0.9 + 0.2 * jitters.0);
    let chase_speed = t.chase_speed * (0.9 + 0.2 * jitters.1);
    let attack_range = enemy.brain_profile.attack_range.max(56.0) * (0.95 + 0.1 * jitters.2);
    Brain::StateMachine(StateMachineCfg::MeleeBrute {
        cfg: MeleeBruteCfg {
            aggressiveness: 1.0,
            aggro_radius,
            attack_range,
            chase_speed,
        },
        state: MeleeBruteState::default(),
    })
}

pub(super) fn melee_brute_brain_for_enemy(enemy: &ActorConfig) -> Brain {
    let t = &enemy.tuning;
    let jitters = five_f32s_from_seed(seed_from_id(&enemy.id));
    let aggro_radius = enemy.brain_profile.aggro_radius * (0.8 + 0.4 * jitters.0);
    let chase_speed = t.chase_speed * (0.85 + 0.3 * jitters.1);
    let attack_range = enemy.brain_profile.attack_range * (0.9 + 0.2 * jitters.2);
    Brain::StateMachine(StateMachineCfg::MeleeBrute {
        cfg: MeleeBruteCfg {
            aggressiveness: if t.is_hostile { 1.0 } else { 0.0 },
            aggro_radius,
            attack_range,
            chase_speed,
        },
        state: MeleeBruteState::default(),
    })
}

pub(super) fn skirmisher_brain_for_enemy(enemy: &ActorConfig) -> Brain {
    skirmisher_brain_from_tuning(
        &enemy.id,
        &enemy.tuning,
        &enemy.brain_profile,
        enemy.tuning.is_hostile,
    )
}

fn sniper_brain_for_enemy(enemy: &ActorConfig) -> Brain {
    let t = &enemy.tuning;
    let jitters = five_f32s_from_seed(seed_from_id(&enemy.id));
    let base_cooldown_s = 1.5;
    let fire_cooldown_s = base_cooldown_s * (0.75 + 0.5 * jitters.0);
    let initial_cooldown_s = fire_cooldown_s * (0.3 + 0.7 * jitters.1);
    Brain::StateMachine(StateMachineCfg::Sniper {
        cfg: SniperCfg {
            aggressiveness: if t.is_hostile { 1.0 } else { 0.0 },
            aggro_radius: enemy.brain_profile.aggro_radius,
            fire_cooldown_s,
        },
        state: SniperState {
            cooldown_remaining: initial_cooldown_s,
        },
    })
}

fn charge_crash_brain_for_enemy(enemy: &ActorConfig) -> Brain {
    let t = &enemy.tuning;
    let jitters = five_f32s_from_seed(seed_from_id(&enemy.id));
    let aggro_radius = enemy.brain_profile.aggro_radius * (0.85 + 0.3 * jitters.0);
    let cruise_speed = t.chase_speed * (0.85 + 0.25 * jitters.1);
    let charge_speed = (cruise_speed * (2.0 + 0.4 * jitters.2)).max(360.0);
    let bite_range = enemy.brain_profile.attack_range * (0.85 + 0.15 * jitters.3);
    let charge_duration_s = 0.38 + 0.18 * jitters.4;
    let charge_cooldown_s = 0.75 + 0.55 * jitters.1;
    let standoff_px =
        (enemy.brain_profile.attack_range * 0.40).max(140.0) * (0.8 + 0.4 * jitters.2);
    let vertical_wobble_px =
        (enemy.brain_profile.attack_range * 0.12).max(20.0) * (0.8 + 0.4 * jitters.3);
    let orbit_drift_rad_s = 0.55 + 0.7 * jitters.4;
    Brain::StateMachine(StateMachineCfg::ChargeCrash {
        cfg: ChargeCrashCfg {
            aggressiveness: if t.is_hostile { 1.0 } else { 0.0 },
            aggro_radius,
            cruise_speed,
            charge_speed,
            bite_range,
            charge_duration_s,
            charge_cooldown_s,
            standoff_px,
            vertical_wobble_px,
            orbit_drift_rad_s,
        },
        state: ChargeCrashState {
            charge_cooldown_remaining: charge_cooldown_s * (0.25 + 0.75 * jitters.0),
            ..Default::default()
        },
    })
}

/// Build the explicitly-hostile solo behavior a rider receives when its mount dies.
///
/// This is intentionally not `enemy_default_brain`: PirateRaider's default is
/// Smash, which has tighter grounded observation requirements, and PirateHeavy's
/// default is peaceful. Dismount means "fall off and fight," so the builder
/// installs an aggressive MeleeBrute brain plus a melee-only action set.
pub(super) fn dismounted_rider_brain_and_action_set(
    rider: &ActorConfig,
    kit: &CombatKit,
    held_item: Option<&ambition_characters::brain::HeldItemSpec>,
    // **The prepared cast**, so a rider that fell off can be asked what IT
    // swings rather than borrowing `pirate_raider`'s. See below.
    prepared: Option<&crate::character_runtime::PreparedCharacterRegistry>,
) -> (Brain, ActionSet) {
    // Rebuild the rider's solo action set from its DURABLE stored combat
    // kit (`innate_melee` / `innate_ranged` / `move_style`) plus its live
    // held item — the same inputs the spawn projection used, queried off
    // the entity so the runtime dismount never re-reads the roster enum.
    let mut action_set = kit.to_action_set(held_item);
    if action_set.melee.is_none() {
        // ⭐ **THE RIDER'S OWN SWING FIRST** (ledger D84). This reached straight
        // for `pirate_raider`'s melee — the THIRD reader of the provocation
        // matcher's archetypes, and the one a placement census could not see
        // because it counts levels rather than code. A rider whose character
        // authors a swing was being handed a stranger's on the way down.
        action_set.melee = prepared
            .zip(rider.sprite_character_id.as_deref())
            .and_then(|(registry, character)| {
                registry.get(character)?.kit.action_set()?.melee.clone()
            })
            // ⛔⛔ **THIS NAMED A ROW THAT NO LONGER EXISTS.** It asked the
            // roster for `pirate_raider`'s melee — and that row was deleted on
            // 2026-08-11 when the raider became `npc_pirate_raider` and authored
            // its own body. `spec_for_brain` cannot fail, so every dismounted
            // rider whose character authored no swing has been given
            // `combatant`'s ever since, silently, while this code read as though
            // it were handing out a pirate's.
            //
            // ⭐ the engine's default fighting kit is what it was ACTUALLY
            // getting — `default_fighting_kit` is pinned equal to `combatant`'s
            // melee (P3.24) — so this is the same swing with the lie removed,
            // and it stops depending on a row at all.
            .or_else(|| default_fighting_kit().innate_melee);
    }

    // If the dismounted rider still has a ranged held item, keep using a
    // ranged-capable brain so the weapon remains live after the shark dies.
    // This preserves the item as the authority: remove / change the held item
    // in data and this path changes without another Rust branch.
    let brain = if held_item.is_some_and(|item| item.grants_ranged()) {
        skirmisher_brain_from_tuning(&rider.id, &rider.tuning, &rider.brain_profile, true)
    } else {
        forced_hostile_melee_brute_brain(rider, 540.0)
    };
    (brain, action_set)
}

fn skirmisher_brain_from_tuning(
    actor_id: &str,
    tuning: &ActorTuning,
    profile: &BrainProfile,
    force_hostile: bool,
) -> Brain {
    let jitters = five_f32s_from_seed(seed_from_id(actor_id));
    let base_cooldown_s = 1.5;
    let fire_cooldown_s = base_cooldown_s * (0.75 + 0.5 * jitters.0);
    let initial_cooldown_s = fire_cooldown_s * (0.3 + 0.7 * jitters.1);
    let standoff_base = (profile.attack_range * 0.35).max(120.0);
    let standoff_px = standoff_base * (0.8 + 0.4 * jitters.2);
    let orbit_phase = jitters.3 * std::f32::consts::TAU;
    let orbit_drift_rad_s = 0.4 + 0.8 * jitters.4;
    Brain::StateMachine(StateMachineCfg::Skirmisher {
        cfg: SkirmisherCfg {
            aggressiveness: if force_hostile || tuning.is_hostile {
                1.0
            } else {
                0.0
            },
            aggro_radius: profile.aggro_radius,
            standoff_px,
            strafe_speed: tuning.chase_speed,
            fire_cooldown_s,
            orbit_drift_rad_s,
        },
        state: SkirmisherState {
            cooldown_remaining: initial_cooldown_s,
            orbit_phase,
            ..Default::default()
        },
    })
}

/// Build a `SmashCfg` from the archetype's tuning row. Heavier archetypes
/// (Brute) get a longer attack reach + slower chase; lighter archetypes
/// (Skitter / Lurker) get a tighter engage band.
///
/// IMPORTANT: the archetype's `attack_range` in `character_archetypes.ron` is the
/// AI-decision aggro distance (~150 px for goblins). That's the radius at which
/// the brain commits to "I'm attacking this target", NOT the distance at which
/// the swing actually hits. The melee swing's reach is in the `SwipeSpec::reach_px`
/// (~28 px); the brain needs to close to roughly `body_half_width +
/// swing_reach` before emitting MeleeAttack, otherwise the windup fires from too
/// far away and the player walks out of the active window.
fn smash_cfg_from_spec(
    profile: &BrainProfile,
    tuning: &ActorTuning,
    body: ambition_platformer2d_core::AbilitySet,
) -> SmashCfg {
    // Heavy vs striker base + per-archetype hit band + dash-to-close are
    // projected onto `BrainProfile` at spawn (`smash_hit_band`,
    // `smash_heavy`, `smash_sprint_to_close`), so this builder reads generic
    // data rather than matching the roster enum. The 36 px hit-band
    // fallback lives in the projection.
    // Duelist > heavy > striker. The duelist base brings the neutral game
    // (footsies / neutral hops / spacing + retreat) that makes a platform
    // fighter MOVE instead of camping point-blank; `attack_range` /
    // `engage_distance` are still overridden from the body's hit band below, so
    // the spacing weaves around the body's real reach.
    let base = if profile.smash_duelist {
        SmashCfg::DUELIST_DEFAULT
    } else if profile.smash_heavy {
        SmashCfg::BRUTE_DEFAULT
    } else {
        SmashCfg::STRIKER_DEFAULT
    };
    let hit_band = profile.smash_hit_band;
    SmashCfg {
        aggro_radius: profile.aggro_radius,
        attack_range: hit_band,
        // Engage band: the brain holds position once inside this radius even if
        // the swing is on cooldown. Slightly larger than `attack_range` so the
        // actor does not bob in/out of engage as it inches forward through approach.
        engage_distance: hit_band * 1.6,
        // Retreat threshold — well inside the hit band so a player dashing into
        // the goblin's space pushes it back rather than getting eaten.
        too_close_distance: (hit_band * 0.5).max(18.0),
        chase_speed: tuning.chase_speed,
        retreat_speed: tuning.chase_speed * 0.75,
        // Goblins dash to close a large gap (richer action set: melee +
        // ranged + dash + jump). Kept off for the other strikers so it
        // doesn't blanket-change every melee enemy's feel.
        sprint_to_close: profile.smash_sprint_to_close,
        // ⭐⭐ **THE BODY'S VERBS, ASKED — not a policy's copy of them.**
        //
        // ⛔ these three read `profile.smash_can_blink/_fly/_shield` until
        // 2026-08-11 (Jon's redirect §7): capability mirrors on a controller
        // policy, which the profile's own doc already called wrong. They made
        // reuse a lie — the SAME shared profile on a body with no blink limb
        // would still have told its driver to try blinking, and on a body that
        // CAN blink but was authored by somebody who forgot the mirror, the
        // driver would never reach for it.
        //
        // ⭐ this is the compositional behaviour the whole campaign is for:
        // `medium_striker` + a PCA body considers the PCA's abilities;
        // `medium_striker` + a puppy slug cannot invent them.
        //
        // The brain still only ATTEMPTS: the body's `CombatCapabilities` +
        // cooldowns are the enforce gate, and `blink_cooldown_s` is the driver's
        // own reactive restraint (policy, I4) over the physical floor (I3).
        can_blink: body.blink,
        blink_cooldown_s: if body.blink { 1.2 } else { 0.0 },
        can_fly: body.fly || body.fly_toggle,
        can_shield: body.shield,
        ..base
    }
}

#[cfg(test)]
mod ladder_projection_tests {
    use super::*;
    use ambition_characters::brain::fighter::{AuthoredFighterLadder, FighterBrainLadder};
    use bevy::prelude::*;

    /// Two rungs that differ from the engine floor in the way the SHIPPED ladder
    /// does: a lower `apm_cap`, and — the one that matters — weights a beginner
    /// does not have.
    const LADDER: &str = "[
        (level: 1, reaction_ms: 500.0, apm_cap: 60.0, execution_noise: 0.40,
         rollout_depth: 0, rollout_k: 0, read_weight: 0.0,
         utility_weights: (reach_fit: 1.0, frame_advantage: 0.10, kill_potential: 0.00, stage_risk: -0.10, expected_payoff: 0.00)),
        (level: 2, reaction_ms: 450.0, apm_cap: 90.0, execution_noise: 0.35,
         rollout_depth: 0, rollout_k: 0, read_weight: 0.0,
         utility_weights: (reach_fit: 1.0, frame_advantage: 0.20, kill_potential: 0.00, stage_risk: -0.20, expected_payoff: 0.00)),
    ]";

    fn fighter_brain(level: u8) -> Brain {
        let cfg = ambition_characters::brain::fighter::FighterCfg::new(
            ambition_characters::brain::fighter::FighterBrainProfile::for_level(level),
        );
        let state = ambition_characters::brain::fighter::FighterState::new(
            &cfg,
            0x5F37_7A11_u64.wrapping_mul(level as u64 + 1),
        );
        Brain::StateMachine(StateMachineCfg::Fighter {
            cfg: Box::new(cfg),
            state: Box::new(state),
        })
    }

    fn profile_of(brain: &Brain) -> ambition_characters::brain::fighter::FighterBrainProfile {
        match brain {
            Brain::StateMachine(StateMachineCfg::Fighter { cfg, .. }) => cfg.profile,
            other => panic!("not a fighter brain: {other:?}"),
        }
    }

    /// **A spawned fighter reads the game's rung.**
    ///
    /// ⛔ the property that was broken: `for_level` hands EVERY rung
    /// `UtilityWeights::default()`, which is `v1()`, which is the authored level
    /// NINE. So a level-1 CPU priced a kill move exactly as the hardest one did.
    #[test]
    fn a_spawned_fighter_takes_the_authored_rung_over_the_floor() {
        let mut app = App::new();
        app.insert_resource(AuthoredFighterLadder(
            FighterBrainLadder::from_ron(LADDER).expect("the fixture ladder parses"),
        ));
        app.add_systems(Update, project_authored_fighter_ladder);

        let floor = ambition_characters::brain::fighter::FighterBrainProfile::for_level(1);
        let entity = app.world_mut().spawn(fighter_brain(1)).id();
        app.update();

        let projected = profile_of(app.world().get::<Brain>(entity).expect("brain"));
        assert_ne!(
            projected, floor,
            "the spawned fighter kept the engine floor, so the authored ladder \
             reached nothing"
        );
        assert!(
            projected.utility_weights.kill_potential < floor.utility_weights.kill_potential,
            "a level-1 CPU still values a kill move as highly as the hardest rung \
             does — floor {:?}, projected {:?}",
            floor.utility_weights,
            projected.utility_weights,
        );
        assert_eq!(projected.apm_cap, 60.0, "the authored action cap");
    }

    /// **No ladder means the floor, which is the engine's stated rule.**
    #[test]
    fn without_a_ladder_the_engine_floor_stands() {
        let mut app = App::new();
        app.add_systems(Update, project_authored_fighter_ladder);
        let entity = app.world_mut().spawn(fighter_brain(1)).id();
        app.update();
        assert_eq!(
            profile_of(app.world().get::<Brain>(entity).expect("brain")),
            ambition_characters::brain::fighter::FighterBrainProfile::for_level(1),
            "a game that shipped no rows had its fighter rewritten anyway"
        );
    }

    /// ⚠ **idempotent**, which is what makes it safe to run on a change-detection
    /// filter that does not rewind. A second pass must land on the same value.
    #[test]
    fn projecting_twice_lands_on_the_same_brain() {
        let mut app = App::new();
        app.insert_resource(AuthoredFighterLadder(
            FighterBrainLadder::from_ron(LADDER).expect("the fixture ladder parses"),
        ));
        app.add_systems(Update, project_authored_fighter_ladder);
        let entity = app.world_mut().spawn(fighter_brain(2)).id();
        app.update();
        let once = profile_of(app.world().get::<Brain>(entity).expect("brain"));
        // Force it to be seen as freshly added again.
        let brain = app.world().get::<Brain>(entity).expect("brain").clone();
        app.world_mut().entity_mut(entity).insert(brain);
        app.update();
        assert_eq!(
            profile_of(app.world().get::<Brain>(entity).expect("brain")),
            once,
            "a second projection moved the brain, so the pass is not idempotent"
        );
    }

    /// ⚠ **a level the ladder does not author keeps the floor** rather than
    /// failing — the same fallback `profile_for_level` states, so the two agree.
    #[test]
    fn an_unauthored_level_keeps_the_floor() {
        let mut app = App::new();
        app.insert_resource(AuthoredFighterLadder(
            FighterBrainLadder::from_ron(LADDER).expect("the fixture ladder parses"),
        ));
        app.add_systems(Update, project_authored_fighter_ladder);
        let entity = app.world_mut().spawn(fighter_brain(7)).id();
        app.update();
        assert_eq!(
            profile_of(app.world().get::<Brain>(entity).expect("brain")),
            ambition_characters::brain::fighter::FighterBrainProfile::for_level(7),
            "level 7 is not in the two-rung fixture and must keep the floor"
        );
    }
}

// ⛔⛔ **`mod tests` WAS HERE AND IS EMPTY OF SUBJECTS, so it is deleted** (AC6).
// It held five archetype-table tests: two that pinned engine constants against
// the reserved `combatant` row while it survived
// (`the_undescribed_respawn_policy_matches_the_combatant_row`,
// `an_engine_default_provoked_policy_matches_the_combatant_row` — both of which
// said in as many words that they go WITH the row rather than being weakened),
// and three that measured `medium_striker`'s row through
// `enemy_default_action_set`.
//
// ⇒ the constants those pins protected are unchanged, which is what the pins
// were for: `UNDESCRIBED_BODY_RESPAWN` and `default_provoked_policy()` are now
// the only authorities on their questions and they say what the row said. The
// template → brain-family mapping is pinned off a CHARACTER's profile by
// `enemy_default_brain_picks_the_family_its_policy_names` in the spawn tests.

/// **A fighter brain gets the GAME's rung, not the engine's floor.**
///
/// ⛔ **this is a PROJECTION because threading was tried and cascaded.** The
/// ladder is authored content that lives in the pack, above this crate, and it is
/// needed at the LEAF of a spawn tree whose roots are many and unalike — a match
/// activation, a hostility reconciler, an encounter wave, a thrown puppy-slug
/// ability. Passing it down four levels so an ability can hand a difficulty
/// ladder to a brain builder reached 323 lines without compiling once. A value
/// with that shape is projected, not threaded.
///
/// ⚠ **at INSERTION, and that is not a detail.** `FighterState::new` caches
/// `DelayedPerception::from_reaction_ms(profile.reaction_ms)` and
/// `HabitModel::new(profile.read_weight)` — the two axes that matter most — so
/// overwriting `cfg.profile` alone after the fact would change nothing the player
/// could see. The state has to be rebuilt, and the only moment that costs nothing
/// is before any habit has accumulated.
///
/// ⚠ **the seed is the construction seed, reproduced exactly.** Both builders use
/// `0x5F37_7A11 * (level + 1)` precisely so two fighters on one rung are the same
/// fighter and a replay reproduces both; a projection that reseeded differently
/// would make the brain the one part of the sim that does not rewind.
///
/// ⭐ **idempotent, which is what makes it safe under change detection.** It runs
/// on `Added<Brain>` and rewrites only when the authored rung differs from what
/// is there, so running it twice — or after a rollback re-inserts a brain — lands
/// on the same value. `Added` not rewinding is therefore harmless: the snapshot
/// stores the PROJECTED profile, because that is what was live.
pub fn project_authored_fighter_ladder(
    ladder: Option<bevy::prelude::Res<ambition_characters::brain::fighter::AuthoredFighterLadder>>,
    mut brains: bevy::prelude::Query<&mut Brain, bevy::prelude::Added<Brain>>,
) {
    let Some(ladder) = ladder else {
        // No ladder shipped: the engine floor is the answer, which is the rule
        // `profile_for_level` states.
        return;
    };
    for mut brain in &mut brains {
        let Brain::StateMachine(StateMachineCfg::Fighter { cfg, state }) = &mut *brain else {
            continue;
        };
        let level = cfg.profile.level;
        let Some(rung) = ladder.0.level(level) else {
            continue;
        };
        if cfg.profile == *rung {
            continue;
        }
        cfg.profile = *rung;
        **state = ambition_characters::brain::fighter::FighterState::new(
            cfg,
            0x5F37_7A11_u64.wrapping_mul(level as u64 + 1),
        );
    }
}

// ⛔⛔ **`default_fighting_kit_tests` WAS HERE AND IS DELETED, exactly as its one
// test asked to be** (AC6). `the_default_kit_equals_what_the_provocation_fallback_builds`
// pinned `default_fighting_kit()` against the `combatant` row's kit so that
// naming the concept in the engine was provably a rename rather than a retune —
// and it said so: *"when `combatant` is finally deleted this test goes with it,
// and by then the default has to be a DECISION somebody made rather than a
// copy."*
//
// ⇒ the row is gone and the constant is unchanged, so the decision is made:
// `default_fighting_kit()` is the only authority on what a provoked body swings.
