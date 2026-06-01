//! Shared Brain + ActionSet construction for ECS feature actors.
//!
//! Spawning, mount/rider transitions, and hostile NPC flips should all come
//! through this module when they need to attach or replace actor brain
//! components. Keeping the construction policy here prevents each caller from
//! hand-rolling a slightly different mix of archetype tuning, aggressiveness,
//! and per-actor jitter.

use super::super::enemies::{EnemyArchetype, EnemyBrainTemplate, EnemyRuntime};
use super::variation::{five_f32s_from_seed, seed_from_id};
use crate::brain::{
    ActionSet, Brain, MeleeBruteCfg, MeleeBruteState, SharkCfg, SharkState, SkirmisherCfg,
    SkirmisherState, SmashCfg, SmashState, SniperCfg, SniperState, StateMachineCfg, WandererCfg,
    WandererState,
};

/// Build the enemy's default `ActionSet` from its archetype spec.
///
/// Reads `melee_spec()` / `ranged_spec()` / `move_style()` straight off the
/// data-driven `EnemyArchetypeSpec` — every spec value (timings, damage,
/// reach) lives in `enemy_archetypes.ron`. Adding a new archetype is a single
/// RON row + a new `EnemyArchetype` enum variant.
pub(super) fn enemy_default_action_set(enemy: &EnemyRuntime) -> ActionSet {
    let archetype = enemy.archetype;
    let move_style = archetype.move_style();
    // Capability is separate from default hostility. A peaceful-by-default
    // PirateHeavy keeps an inert brain (`attacks_player() == false` gives her
    // MeleeBrute cfg aggressiveness 0), but she still needs a concrete melee
    // action once another system explicitly provokes / dismounts her into a
    // hostile state. PuppySlug remains harmless here because its data row has no
    // melee or ranged action.
    let mut actions = ActionSet {
        melee: archetype.melee_spec(),
        ranged: archetype.ranged_spec(),
        move_style,
        ..Default::default()
    };
    apply_archetype_held_item(archetype, &mut actions);
    actions
}

fn apply_archetype_held_item(archetype: EnemyArchetype, actions: &mut ActionSet) {
    if let Some(item) = archetype.held_item_spec() {
        item.apply_to_action_set(actions);
    }
}

pub(super) fn held_item_for_archetype(
    archetype: EnemyArchetype,
) -> Option<crate::brain::HeldItemSpec> {
    archetype.held_item_spec()
}

fn held_item_grants_ranged(archetype: EnemyArchetype) -> bool {
    archetype
        .held_item_spec()
        .as_ref()
        .is_some_and(|item| item.grants_ranged())
}

/// Build the enemy's default `Brain` from its archetype spec.
///
/// Reads `brain_template()` off the consolidated `EnemyArchetypeSpec` so adding
/// a new archetype is a single row, not a parallel match.
pub(in crate::content::features) fn enemy_default_brain(enemy: &EnemyRuntime) -> Brain {
    let archetype = enemy.archetype;
    match archetype.brain_template() {
        EnemyBrainTemplate::StandStill => Brain::StateMachine(StateMachineCfg::StandStill),
        EnemyBrainTemplate::Wanderer => Brain::StateMachine(StateMachineCfg::Wanderer {
            cfg: WandererCfg::PUPPY_SLUG_DEFAULT,
            state: WandererState::default(),
        }),
        EnemyBrainTemplate::MeleeBrute => melee_brute_brain_for_enemy(enemy),
        EnemyBrainTemplate::Shark => shark_brain_for_enemy(enemy),
        EnemyBrainTemplate::Skirmisher => skirmisher_brain_for_enemy(enemy),
        EnemyBrainTemplate::Sniper => sniper_brain_for_enemy(enemy),
        EnemyBrainTemplate::Smash => Brain::StateMachine(StateMachineCfg::Smash {
            cfg: smash_cfg_for_archetype(archetype),
            state: SmashState {
                rng_seed: seed_from_id(&enemy.id) as u64,
                ..Default::default()
            },
        }),
    }
}

/// Build the explicitly-hostile behavior for an actor that is peaceful by
/// default but has been provoked in play. Default spawn still uses
/// [`enemy_default_brain`] so cove PirateHeavy variants remain peaceful until
/// struck; this override gives them the same concrete heavy swing/capability
/// once the hostility flag is set.
pub(super) fn enemy_forced_hostile_brain_and_action_set(
    enemy: &EnemyRuntime,
) -> (Brain, ActionSet) {
    if enemy.archetype == EnemyArchetype::PirateHeavy {
        let brain = forced_hostile_melee_brute_brain(enemy, 500.0);
        let action_set = enemy_default_action_set(enemy);
        return (brain, action_set);
    }
    (enemy_default_brain(enemy), enemy_default_action_set(enemy))
}

fn forced_hostile_melee_brute_brain(enemy: &EnemyRuntime, min_aggro_radius: f32) -> Brain {
    let archetype = enemy.archetype;
    let jitters = five_f32s_from_seed(seed_from_id(&enemy.id));
    let aggro_radius = archetype.aggro_radius().max(min_aggro_radius) * (0.9 + 0.2 * jitters.0);
    let chase_speed = archetype.chase_speed() * (0.9 + 0.2 * jitters.1);
    let attack_range = archetype.attack_range().max(56.0) * (0.95 + 0.1 * jitters.2);
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

pub(super) fn melee_brute_brain_for_enemy(enemy: &EnemyRuntime) -> Brain {
    let archetype = enemy.archetype;
    let jitters = five_f32s_from_seed(seed_from_id(&enemy.id));
    let aggro_radius = archetype.aggro_radius() * (0.8 + 0.4 * jitters.0);
    let chase_speed = archetype.chase_speed() * (0.85 + 0.3 * jitters.1);
    let attack_range = archetype.attack_range() * (0.9 + 0.2 * jitters.2);
    Brain::StateMachine(StateMachineCfg::MeleeBrute {
        cfg: MeleeBruteCfg {
            aggressiveness: if archetype.attacks_player() { 1.0 } else { 0.0 },
            aggro_radius,
            attack_range,
            chase_speed,
        },
        state: MeleeBruteState::default(),
    })
}

pub(super) fn skirmisher_brain_for_enemy(enemy: &EnemyRuntime) -> Brain {
    let archetype = enemy.archetype;
    skirmisher_brain_from_archetype(&enemy.id, archetype, archetype.attacks_player())
}

fn sniper_brain_for_enemy(enemy: &EnemyRuntime) -> Brain {
    let archetype = enemy.archetype;
    let jitters = five_f32s_from_seed(seed_from_id(&enemy.id));
    let base_cooldown_s = 1.5;
    let fire_cooldown_s = base_cooldown_s * (0.75 + 0.5 * jitters.0);
    let initial_cooldown_s = fire_cooldown_s * (0.3 + 0.7 * jitters.1);
    Brain::StateMachine(StateMachineCfg::Sniper {
        cfg: SniperCfg {
            aggressiveness: if archetype.attacks_player() { 1.0 } else { 0.0 },
            aggro_radius: archetype.aggro_radius(),
            fire_cooldown_s,
        },
        state: SniperState {
            cooldown_remaining: initial_cooldown_s,
        },
    })
}

fn shark_brain_for_enemy(enemy: &EnemyRuntime) -> Brain {
    let archetype = enemy.archetype;
    let jitters = five_f32s_from_seed(seed_from_id(&enemy.id));
    let aggro_radius = archetype.aggro_radius() * (0.85 + 0.3 * jitters.0);
    let cruise_speed = archetype.chase_speed() * (0.85 + 0.25 * jitters.1);
    let charge_speed = (cruise_speed * (2.0 + 0.4 * jitters.2)).max(360.0);
    let bite_range = archetype.attack_range() * (0.85 + 0.15 * jitters.3);
    let charge_duration_s = 0.38 + 0.18 * jitters.4;
    let charge_cooldown_s = 0.75 + 0.55 * jitters.1;
    let standoff_px = (archetype.attack_range() * 0.40).max(140.0) * (0.8 + 0.4 * jitters.2);
    let vertical_wobble_px = (archetype.attack_range() * 0.12).max(20.0) * (0.8 + 0.4 * jitters.3);
    let orbit_drift_rad_s = 0.55 + 0.7 * jitters.4;
    Brain::StateMachine(StateMachineCfg::Shark {
        cfg: SharkCfg {
            aggressiveness: if archetype.attacks_player() { 1.0 } else { 0.0 },
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
        state: SharkState {
            charge_cooldown_remaining: charge_cooldown_s * (0.25 + 0.75 * jitters.0),
            ..Default::default()
        },
    })
}

/// Build the rider's MOUNTED behavior for a composite mount/rider spawn.
///
/// The mounted rider is intentionally not the rider archetype's standalone
/// brain. While mounted, the rider gets a hostile Skirmisher brain keyed off
/// the composite archetype's ranged spec and variation keyed by the rider id.
pub(super) fn mounted_rider_brain_and_action_set(
    rider_id: &str,
    rider_archetype: EnemyArchetype,
    composite_archetype: EnemyArchetype,
) -> (Brain, ActionSet) {
    let brain = skirmisher_brain_from_archetype(rider_id, composite_archetype, true);
    let mut action_set = ActionSet {
        melee: None,
        ranged: composite_archetype.ranged_spec(),
        move_style: rider_archetype.move_style(),
        ..Default::default()
    };
    apply_archetype_held_item(composite_archetype, &mut action_set);
    (brain, action_set)
}

/// Build the explicitly-hostile solo behavior a rider receives when its mount dies.
///
/// This is intentionally not `enemy_default_brain`: PirateRaider's default is
/// Smash, which has tighter grounded observation requirements, and PirateHeavy's
/// default is peaceful. Dismount means "fall off and fight," so the builder
/// installs an aggressive MeleeBrute brain plus a melee-only action set.
pub(super) fn dismounted_rider_brain_and_action_set(
    rider: &EnemyRuntime,
    held_item: Option<&crate::brain::HeldItemSpec>,
) -> (Brain, ActionSet) {
    let mut action_set = enemy_default_action_set(rider);
    if let Some(item) = held_item {
        item.apply_to_action_set(&mut action_set);
    }
    if action_set.melee.is_none() {
        action_set.melee = EnemyArchetype::PirateRaider.melee_spec();
    }

    // If the dismounted rider still has a ranged held item, keep using a
    // ranged-capable brain so the weapon remains live after the shark dies.
    // This preserves the item as the authority: remove / change the held item
    // in data and this path changes without another Rust branch.
    let brain = if held_item.is_some_and(|item| item.grants_ranged())
        || held_item_grants_ranged(rider.archetype)
    {
        skirmisher_brain_from_archetype(&rider.id, rider.archetype, true)
    } else {
        forced_hostile_melee_brute_brain(rider, 540.0)
    };
    (brain, action_set)
}

fn skirmisher_brain_from_archetype(
    actor_id: &str,
    archetype: EnemyArchetype,
    force_hostile: bool,
) -> Brain {
    let jitters = five_f32s_from_seed(seed_from_id(actor_id));
    let base_cooldown_s = 1.5;
    let fire_cooldown_s = base_cooldown_s * (0.75 + 0.5 * jitters.0);
    let initial_cooldown_s = fire_cooldown_s * (0.3 + 0.7 * jitters.1);
    let standoff_base = (archetype.attack_range() * 0.35).max(120.0);
    let standoff_px = standoff_base * (0.8 + 0.4 * jitters.2);
    let orbit_phase = jitters.3 * std::f32::consts::TAU;
    let orbit_drift_rad_s = 0.4 + 0.8 * jitters.4;
    Brain::StateMachine(StateMachineCfg::Skirmisher {
        cfg: SkirmisherCfg {
            aggressiveness: if force_hostile || archetype.attacks_player() {
                1.0
            } else {
                0.0
            },
            aggro_radius: archetype.aggro_radius(),
            standoff_px,
            strafe_speed: archetype.chase_speed(),
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
/// IMPORTANT: the archetype's `attack_range` in `enemy_archetypes.ron` is the
/// AI-decision aggro distance (~150 px for goblins). That's the radius at which
/// the brain commits to "I'm attacking this target", NOT the distance at which
/// the swing actually hits. The melee swing's reach is in the `SwipeSpec::reach_px`
/// (~28 px); the brain needs to close to roughly `body_half_width +
/// swing_reach` before emitting MeleeAttack, otherwise the windup fires from too
/// far away and the player walks out of the active window.
fn smash_cfg_for_archetype(arch: EnemyArchetype) -> SmashCfg {
    use super::super::enemies::EnemyArchetype::*;
    let base = match arch {
        LargeBrute | LargeColossus => SmashCfg::BRUTE_DEFAULT,
        _ => SmashCfg::STRIKER_DEFAULT,
    };
    // Per-archetype hit-band sizing. Mirrors the legacy `MeleeBruteCfg` defaults
    // (Striker ~32px, Brute ~48px), but uses concrete archetype buckets so
    // Smash can keep enemy-specific speed/radius from data.
    let hit_band = match arch {
        LargeBrute | LargeColossus => 48.0,
        MediumStriker | SmallSkitter | SmallLurker => 32.0,
        _ => 36.0,
    };
    SmashCfg {
        aggro_radius: arch.aggro_radius(),
        attack_range: hit_band,
        // Engage band: the brain holds position once inside this radius even if
        // the swing is on cooldown. Slightly larger than `attack_range` so the
        // actor does not bob in/out of engage as it inches forward through approach.
        engage_distance: hit_band * 1.6,
        // Retreat threshold — well inside the hit band so a player dashing into
        // the goblin's space pushes it back rather than getting eaten.
        too_close_distance: (hit_band * 0.5).max(18.0),
        chase_speed: arch.chase_speed(),
        retreat_speed: arch.chase_speed() * 0.75,
        ..base
    }
}
