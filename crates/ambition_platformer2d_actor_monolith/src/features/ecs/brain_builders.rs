//! Shared Brain + ActionSet construction for ECS feature actors.
//!
//! Spawning, mount/rider transitions, and hostile NPC flips should all come
//! through this module when they need to attach or replace actor brain
//! components. Keeping the construction policy here prevents each caller from
//! hand-rolling a slightly different mix of archetype tuning, aggressiveness,
//! and per-actor jitter.

use ambition_combat::variation::{five_f32s_from_seed, seed_from_id};
use super::HeldItem;
use ambition_characters::brain::{
    ActionSet, Brain, ChargeCrashCfg, ChargeCrashState, MeleeBruteCfg, MeleeBruteState,
    SkirmisherCfg, SkirmisherState, SmashCfg, SmashState, SniperCfg, SniperState, StateMachineCfg,
    WandererCfg,
};
use ambition_combat::actor_tuning::ActorConfig;
use ambition_combat::actor_tuning::{ActorTuning, BrainProfile, CharacterBrainTemplate};
use ambition_combat::components::CombatKit;

/// Fallback fighting kit for bodies whose character authors no repertoire.
///
/// Exploration provocation and platform-fighter fallback kits intentionally have
/// different tuning; these defaults belong to session/ruleset policy, not body
/// identity.
///
/// TODO(compat-remove): delete this fallback once every adopter supplies an
/// explicit ruleset or character fighting kit.
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
/// the twin of [`default_fighting_kit`] one authority over: that one answers
/// *what does it swing*, this one answers *how does it fight*. They were the two
/// halves the `combatant` archetype row was doing at once, and separating them
/// is what lets the row die — a body is not a policy, and neither is a kit.
///
/// `an_engine_default_provoked_policy_matches_the_combatant_row` pins the numbers against the
/// row while the row survives; when it goes, the constant stands alone and nothing has to
/// change.
///
/// A stage that wants provoked bodies to fight differently says so there; nothing says so yet.
///
/// deliberately NOT a ranged policy. `medium_striker` carried a thrown rock,
/// and using it here turned every provoked NPC — the kernel guide, a merchant —
/// into a rock-thrower instead of a melee attacker like the pirates.
pub(crate) fn default_provoked_policy() -> ambition_combat::actor_tuning::BrainProfile {
    ambition_combat::actor_tuning::BrainProfile {
        template: ambition_characters::brain::CharacterBrainTemplate::Smash,
        aggro_radius: 460.0,
        attack_range: 150.0,
        patrol_effort: 0.6774,
        chase_effort: 1.0,
        ..Default::default()
    }
}

// The constant is `ambition_characters:actor:DEFAULT_UNAUTHORED_BODY_HEALTH` now — the pool a
// body gets when no authority describes it, asked at construction — and provocation no longer
// writes health at all.

// All three read a body's weapons off an `ArchetypeSpec` — the melee timings, the ranged spec,
// the gait, the held item, the signature move — and their one production caller was
// `EnemyActorSpawnPlan::hostile`, which asked them only when the seed carried an archetype.
//
// ⇒ what a body fights with comes from its CHARACTER, through the one persona
// writer (`grant_prepared_character_body`).

pub(super) fn action_set_from_combat_kit(
    kit: &CombatKit,
    held_item: Option<&HeldItem>,
) -> ActionSet {
    kit.to_action_set(held_item.map(|item| &item.spec))
}

/// Deterministic RNG seed for a fighter brain.
///
/// Ordinary fighters mix difficulty with the stable participant id, so mirror
/// seats get independent streams without using clocks, process RNG, or Bevy
/// entity ids. Characters that explicitly preserve mirror symmetry instead mix
/// difficulty with the character id, giving twins the same initial stream; they
/// still diverge naturally once their observations differ.
fn fighter_cognition_seed(enemy: &ActorConfig, level: u8) -> u64 {
    // A participant id is `"<character>#seat<n>"`; the character alone is what is
    // left when the seat is dropped. Falling back to the whole id keeps a body
    // that carries no seat suffix (a room spawn) on a stream of its own rather
    // than silently joining a shared one.
    let identity = if enemy.preserves_mirror_symmetry {
        enemy
            .id
            .split_once('#')
            .map_or(enemy.id.as_str(), |(character, _seat)| character)
    } else {
        enemy.id.as_str()
    };
    // MIX, not add: `seed_from_id` is a 32-bit FNV-1a, so shifting it into the
    // high half and folding the level in below keeps two nearby levels of one
    // participant far apart in the stream rather than adjacent.
    ((seed_from_id(identity) as u64) << 32) ^ 0x5F37_7A11_u64.wrapping_mul(level as u64 + 1)
}

/// Build the enemy's default `Brain` from its resolved controller profile.
pub(crate) fn enemy_default_brain(
    enemy: &ActorConfig,
    // **THE BODY'S OWN VERBS**, not a policy's opinion of them. See
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
                fighter_cognition_seed(enemy, level),
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
fn dismounted_rider_brain_and_action_set(
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
        // This reached straight for `pirate_raider`'s melee — the THIRD reader of the provocation
        // matcher's archetypes, and the one a placement census could not see because it counts
        // levels rather than code. A rider whose character authors a swing was being handed a
        // stranger's on the way down.
        action_set.melee = prepared
            .zip(rider.sprite_character_id.as_deref())
            .and_then(|(registry, character)| {
                registry.get(character)?.kit.action_set()?.melee.clone()
            })
            // `spec_for_brain` cannot fail, so every dismounted rider whose character authored
            // no swing has been given `combatant`'s ever since, silently, while this code read
            // as though it were handing out a pirate's.
            //
            // the engine's default fighting kit is what it was ACTUALLY
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
        // Derive available verbs from the body rather than duplicating them in the
        // policy. The brain only attempts actions; body capabilities and cooldowns
        // remain authoritative for enforcement.
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

    const CONSTRUCTED_STREAM: u64 = 0xC0FF_EE00_D15E_A5E5;

    fn fighter_brain(level: u8) -> Brain {
        let cfg = ambition_characters::brain::fighter::FighterCfg::new(
            ambition_characters::brain::fighter::FighterBrainProfile::for_level(level),
        );
        let state =
            ambition_characters::brain::fighter::FighterState::new(&cfg, CONSTRUCTED_STREAM);
        Brain::StateMachine(StateMachineCfg::Fighter {
            cfg: Box::new(cfg),
            state: Box::new(state),
        })
    }

    fn stream_of(brain: &Brain) -> u64 {
        match brain {
            Brain::StateMachine(StateMachineCfg::Fighter { state, .. }) => state.noise,
            other => panic!("not a fighter brain: {other:?}"),
        }
    }

    fn profile_of(brain: &Brain) -> ambition_characters::brain::fighter::FighterBrainProfile {
        match brain {
            Brain::StateMachine(StateMachineCfg::Fighter { cfg, .. }) => cfg.profile,
            other => panic!("not a fighter brain: {other:?}"),
        }
    }

    /// **A spawned fighter reads the game's rung.**
    ///
    /// So a level-1 CPU priced a kill move exactly as the hardest one did.
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

    /// **idempotent**, which is what makes it safe to run on a change-detection
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

    /// **THE PROJECTION MUST NOT RE-CHOOSE THE COGNITIVE STREAM.**
    ///
    /// This is the second half of the same-character CPU symmetry defect, and it is the half that
    /// would have silently undone the first.
    #[test]
    fn the_projection_carries_the_stream_it_was_handed() {
        let mut app = App::new();
        app.insert_resource(AuthoredFighterLadder(
            FighterBrainLadder::from_ron(LADDER).expect("the fixture ladder parses"),
        ));
        app.add_systems(Update, project_authored_fighter_ladder);
        let entity = app.world_mut().spawn(fighter_brain(1)).id();
        app.update();

        let brain = app.world().get::<Brain>(entity).expect("brain");
        // Non-vacuity: the pass must actually have DONE its job, or "the stream
        // survived" is only saying that nothing ran.
        assert_ne!(
            profile_of(brain),
            ambition_characters::brain::fighter::FighterBrainProfile::for_level(1),
            "the ladder did not project, so this test is not observing the rebuild \
             it exists to constrain"
        );
        assert_eq!(
            stream_of(brain),
            CONSTRUCTED_STREAM,
            "the ladder projection reseeded the fighter's noise stream, which is \
             what made every CPU on one rung think identical thoughts"
        );
    }

    /// **a level the ladder does not author keeps the floor** rather than
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

/// **WHO THINKS WHAT — the CPU cognition-stream policy and its one authored
/// exception.** See [`fighter_cognition_seed`].
///
/// these read `state.noise` at construction, which IS the stream: `FighterCfg`
/// stores the seed there verbatim and every later sample advances from it. It is
/// the smallest deterministic property that answers *"are these two fighters the
/// same mind?"*, so nothing here is probabilistic and nothing has to run a match.
#[cfg(test)]
mod cognition_stream_tests {
    use super::*;

    /// A CPU fighter seat, as `PreparedSeat` builds one: the participant id is
    /// `"<character>#seat<n>"` — the body's identity, not its costume's.
    fn seat(character: &str, seat_index: usize, level: u8, mirrors: bool) -> ActorConfig {
        ActorConfig {
            id: format!("{character}#seat{seat_index}"),
            name: character.to_string(),
            tuning: ActorTuning::default(),
            brain_profile: BrainProfile {
                template: CharacterBrainTemplate::Fighter,
                fighter_level: level,
                ..Default::default()
            },
            brain: ambition_entity_catalog::placements::CharacterBrain::Passive,
            sprite_override_npc_name: None,
            sprite_character_id: Some(character.to_string()),
            preserves_mirror_symmetry: mirrors,
        }
    }

    /// The stream a seat's fighter brain is actually built on — asked through the
    /// real builder, not through the seed helper, so the test constrains the
    /// composition and not an internal function.
    fn stream_for(config: &ActorConfig) -> u64 {
        match enemy_default_brain(config, ambition_platformer2d_core::AbilitySet::NONE) {
            Brain::StateMachine(StateMachineCfg::Fighter { state, .. }) => state.noise,
            other => panic!("expected a fighter brain, got {other:?}"),
        }
    }

    /// The seed was `0x5F37_7A11 * (level + 1)` and nothing else, so a
    /// same-character CPU-vs-CPU match was a perfect reflection — two brains
    /// drawing byte-identical noise while reading a symmetric stage. A viewer
    /// watching two Georges was watching one George twice.
    #[test]
    fn two_participants_of_one_character_do_not_share_a_stream() {
        let one = stream_for(&seat("george_booul", 0, 6, false));
        let two = stream_for(&seat("george_booul", 1, 6, false));
        assert_ne!(
            one, two,
            "seat 0 and seat 1 of one character at one level got the same \
             cognitive stream, so a mirror match is a reflection again"
        );
    }

    /// **Replay determinism, which is the constraint the fix had to respect.**
    ///
    /// The stream may not come from a clock, a process-global RNG or an `Entity`.
    /// Rebuilding the same participant under the same setup — which is exactly
    /// what a rollback resimulation does — must land on the same stream.
    #[test]
    fn the_same_participant_rebuilds_on_the_same_stream() {
        let config = seat("george_booul", 1, 6, false);
        assert_eq!(
            stream_for(&config),
            stream_for(&config),
            "a rebuilt participant got a different stream, so the brain is the one \
             part of the sim that does not rewind"
        );
        // And a separately CONSTRUCTED but equal seat agrees, which is the
        // property a replay actually needs — it does not keep the old value
        // around to hand back.
        assert_eq!(
            stream_for(&seat("george_booul", 1, 6, false)),
            stream_for(&config),
            "two equal seats disagreed, so something outside the seat's own \
             identity is leaking into the stream"
        );
    }

    /// **difficulty still contributes**, which the old seed got right and the
    /// fix keeps: raising a CPU's rung changes how it thinks as well as what it
    /// weighs.
    #[test]
    fn one_participant_at_two_levels_thinks_differently() {
        assert_ne!(
            stream_for(&seat("george_booul", 0, 3, false)),
            stream_for(&seat("george_booul", 0, 7, false)),
            "the same seat at two difficulties got one stream, so the level term \
             was dropped"
        );
    }

    /// **and the ordinary symmetry-breaker is the PARTICIPANT, not the
    /// character.** Two different characters differ anyway — that is not the
    /// property under test — so this pins the thing that would be wrong if
    /// somebody "fixed" the defect with per-character seed constants: seats of one
    /// character must already differ, which
    /// [`two_participants_of_one_character_do_not_share_a_stream`] proves, and
    /// the character term must not be the ONLY term.
    #[test]
    fn the_character_is_not_the_ordinary_symmetry_breaker() {
        // If the seed were keyed on the character alone, these two would be equal
        // — which is the state this whole change exists to leave.
        assert_ne!(
            stream_for(&seat("ordinary_fighter", 0, 5, false)),
            stream_for(&seat("ordinary_fighter", 1, 5, false)),
            "an ordinary character's two seats share a stream, so the participant \
             term is missing and only the character is keying the seed"
        );
    }

    /// **EMMY'S AUTHORED EXCEPTION: her twins think alike on purpose.**
    ///
    /// Two seats, two participants, one stream — because the character asked for
    /// it, not because the default leaked. This is what makes an Emmy-vs-Emmy
    /// mirror match play as a reflection when the stage is symmetric.
    #[test]
    fn a_mirror_preserving_characters_twins_share_one_stream() {
        assert_eq!(
            stream_for(&seat("npc_emmy_noether", 0, 6, true)),
            stream_for(&seat("npc_emmy_noether", 1, 6, true)),
            "two Emmys at one difficulty got different cognitive streams, so her \
             authored mirror symmetry reached nothing"
        );
    }

    /// **the exception drops the PARTICIPANT term; it does not zero the seed.**
    ///
    /// A `rng_seed = 0` style implementation would hand every mirror-preserving
    /// character in the game ONE shared stream, so two Emmys and two of somebody
    /// else would all think alike — a global, not a character trait.
    #[test]
    fn two_mirror_preserving_characters_keep_their_own_streams() {
        assert_ne!(
            stream_for(&seat("npc_emmy_noether", 0, 6, true)),
            stream_for(&seat("some_other_mirror", 0, 6, true)),
            "the exception collapsed two different characters onto one stream, so \
             it is a global rather than an authored per-character trait"
        );
    }

    /// **the exception still respects difficulty**, so two Emmys on different
    /// rungs are not forced to agree — the trait shares a stream between EQUALLY
    /// CONFIGURED twins, which is what makes the mirror a fair one.
    #[test]
    fn mirror_symmetry_does_not_flatten_difficulty() {
        assert_ne!(
            stream_for(&seat("npc_emmy_noether", 0, 2, true)),
            stream_for(&seat("npc_emmy_noether", 1, 8, true)),
            "a level-2 Emmy and a level-8 Emmy were put on one stream"
        );
    }

    /// **a body with no seat suffix keeps a stream of its own.** A room spawn's
    /// id is not `"<character>#seat<n>"`, and the exception's `split_once` must
    /// fall back to the whole id rather than silently joining every unsuffixed
    /// body of that character to one stream.
    #[test]
    fn an_unsuffixed_body_is_not_special_cased_into_sharing() {
        let mut room_body = seat("npc_emmy_noether", 0, 6, true);
        room_body.id = "npc_emmy_noether_lab_copy".to_string();
        let mut other_room_body = room_body.clone();
        other_room_body.id = "npc_emmy_noether_hall_copy".to_string();
        assert_ne!(
            stream_for(&room_body),
            stream_for(&other_room_body),
            "two differently-identified bodies with no seat suffix collapsed onto \
             one stream"
        );
    }

    /// **THE TRAIT DECIDES A STREAM AND NOTHING ELSE.** It must not reach the
    /// profile, the difficulty or the template — if it ever starts shaping how a
    /// fighter decides rather than which stream it decides from, the mirror has
    /// stopped being emergent and become a policy.
    #[test]
    fn the_trait_changes_only_the_stream() {
        let ordinary = seat("npc_emmy_noether", 0, 6, false);
        let mirroring = seat("npc_emmy_noether", 0, 6, true);
        let (
            Brain::StateMachine(StateMachineCfg::Fighter { cfg: plain_cfg, .. }),
            Brain::StateMachine(StateMachineCfg::Fighter {
                cfg: mirror_cfg, ..
            }),
        ) = (
            enemy_default_brain(&ordinary, ambition_platformer2d_core::AbilitySet::NONE),
            enemy_default_brain(&mirroring, ambition_platformer2d_core::AbilitySet::NONE),
        )
        else {
            panic!("both seats must build fighter brains");
        };
        assert_eq!(
            plain_cfg.profile, mirror_cfg.profile,
            "authoring mirror symmetry changed the fighter's PROFILE, so it is no \
             longer only choosing a stream"
        );
        assert_eq!(
            plain_cfg.decision_interval_ticks, mirror_cfg.decision_interval_ticks,
            "authoring mirror symmetry changed how often the fighter decides"
        );
    }
}

// ⇒ the constants those pins protected are unchanged, which is what the pins
// were for: `UNDESCRIBED_BODY_RESPAWN` and `default_provoked_policy()` are now
// the only authorities on their questions and they say what the row said. The
// template → brain-family mapping is pinned off a CHARACTER's profile by
// `enemy_default_brain_picks_the_family_its_policy_names` in the spawn tests.

/// Project the game's authored fighter difficulty rung into newly inserted brains.
///
/// Rebuild `FighterState` so profile-cached perception and habit fields match the
/// authored rung, while preserving the fighter's existing noise-stream position.
/// The projection is idempotent and only rewrites when the rung differs.
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
        // the stream this fighter was CONSTRUCTED on, carried across the
        // rebuild. See the note above: reseeding here is what would undo
        // `fighter_cognition_seed`.
        let stream = state.noise;
        **state = ambition_characters::brain::fighter::FighterState::new(cfg, stream);
    }
}

// ⇒ the row is gone and the constant is unchanged, so the decision is made:
// `default_fighting_kit()` is the only authority on what a provoked body swings.

/// Rebuild a fallen rider's solo brain, on the dissolution the mount ANNOUNCES.
///
/// ⭐ THE MOUNT MODULE DOES NOT CALL THE BUILDER ANY MORE. It writes
/// [`MountDied`](ambition_platformer2d_shared_tangle::body::MountDied) — which it
/// already did, for the boss bridge — and this system answers it. That is the
/// same road `ambition_boss_encounter` takes to turn the dissolution into a
/// rider boss's `External("mount_died")` phase: mount announces, the domain that
/// owns the reaction reacts.
///
/// ⛔ THE REBUILD CANNOT TRAVEL WITH A MOUNT CARVE and that is why it moved.
/// It reads `ActorConfig`, `CombatKit`, `HeldItem` and the prepared cast —
/// character-runtime facts, every one — so a mount crate that called it would
/// have to import the character runtime to dissolve a mount.
///
/// ⛔ A BOSS RIDER IS SKIPPED, unchanged: its identity is AUTHORED, not derived
/// from a kit, so re-deriving a brain for it would be wrong (ADR 0020; Q19b).
/// The component IS the marker — no new flag.
///
/// ⚠ ORDER, not shape, is what this had to preserve: the insert must land
/// before the dismounted body is simulated. It runs chained straight after
/// `enforce_mount_rider_link` in `CombatSet::Settle`, so its commands flush at
/// the same barrier the direct call's did.
pub fn rebuild_dismounted_rider_brains(
    mut commands: bevy::prelude::Commands,
    mut dismounts: bevy::prelude::MessageReader<
        ambition_platformer2d_shared_tangle::body::MountDied,
    >,
    // The prepared cast, so a dismounted rider swings its own weapon rather
    // than borrowing an archetype's.
    prepared: Option<bevy::prelude::Res<crate::character_runtime::PreparedCharacterRegistry>>,
    riders: bevy::prelude::Query<(
        &ActorConfig,
        Option<&HeldItem>,
        Option<&CombatKit>,
        Option<&ambition_boss_encounter::BossConfig>,
    )>,
) {
    for dismount in dismounts.read() {
        let Ok((config, held_item, combat_kit, boss_config)) = riders.get(dismount.rider) else {
            continue;
        };
        if boss_config.is_some() {
            continue;
        }
        // A rider always carries a CombatKit; fall back defensively.
        let kit = combat_kit.cloned().unwrap_or_default();
        let (brain, action_set) = dismounted_rider_brain_and_action_set(
            config,
            &kit,
            held_item.map(|item| &item.spec),
            prepared.as_deref(),
        );
        commands.entity(dismount.rider).insert((brain, action_set));
    }
}
