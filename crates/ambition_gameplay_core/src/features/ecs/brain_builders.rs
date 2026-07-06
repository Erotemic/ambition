//! Shared Brain + ActionSet construction for ECS feature actors.
//!
//! Spawning, mount/rider transitions, and hostile NPC flips should all come
//! through this module when they need to attach or replace actor brain
//! components. Keeping the construction policy here prevents each caller from
//! hand-rolling a slightly different mix of archetype tuning, aggressiveness,
//! and per-actor jitter.

use super::super::enemies::CharacterArchetypeSpec;
use super::actor_clusters::ActorConfig;
use super::variation::{five_f32s_from_seed, seed_from_id};
use super::{CombatKit, HeldItem};
use crate::features::ecs::actor_tuning::{ActorTuning, CharacterBrainSpec, CharacterBrainTemplate};
use ambition_characters::brain::{
    ActionSet, Brain, ChargeCrashCfg, ChargeCrashState, MeleeBruteCfg, MeleeBruteState,
    SkirmisherCfg, SkirmisherState, SmashCfg, SmashState, SniperCfg, SniperState, StateMachineCfg,
    WandererCfg, WandererState,
};

/// Build the enemy's durable combat capability kit from archetype data.
///
/// The kit intentionally does **not** include held item overlays; a held item is
/// a separate component and can be dropped/swapped later. `ActionSet` is derived
/// from `CombatKit + HeldItem` for whichever aggression state is currently live.
pub(super) fn enemy_combat_kit_for_spec(spec: &CharacterArchetypeSpec) -> CombatKit {
    CombatKit {
        innate_melee: spec.melee_spec(),
        innate_ranged: spec.ranged_spec(),
        move_style: spec.move_style(),
    }
}

pub(super) fn action_set_from_combat_kit(
    kit: &CombatKit,
    held_item: Option<&HeldItem>,
) -> ActionSet {
    kit.to_action_set(held_item.map(|item| &item.spec))
}

/// Build the enemy's default `ActionSet` from its authored spec.
///
/// Reads `melee_spec()` / `ranged_spec()` / `move_style()` straight off the
/// data-driven `CharacterArchetypeSpec` — every spec value (timings, damage,
/// reach) lives in `character_archetypes.ron`. Spawn-time only: the spec is
/// resolved on the spawn seed before the entity exists, so the spawn path
/// never names the roster enum.
pub(super) fn enemy_default_action_set(spec: &CharacterArchetypeSpec) -> ActionSet {
    let mut actions = enemy_combat_kit_for_spec(spec).to_action_set(spec.held_item_spec().as_ref());
    // The special is a data-driven MOVE now (the moveset subsumes `ActionSet.special`,
    // fable review §A1): if the archetype authors a signature move on the `special`
    // verb, mark the capability so the brain knows to press special. The move itself
    // executes through the moveset runtime (`trigger_moveset_moves`), never the
    // retired flat resolve arm.
    if let Some(move_id) = spec
        .signature_move
        .as_ref()
        .and_then(|m| m.verbs.get("special"))
    {
        actions.special = Some(ambition_characters::brain::SpecialActionSpec::Special(
            move_id.clone(),
        ));
    }
    actions
}

pub(super) fn held_item_for_spec(
    spec: &CharacterArchetypeSpec,
) -> Option<ambition_characters::brain::HeldItemSpec> {
    spec.held_item_spec()
}

/// Build the enemy's default `Brain` from its archetype spec.
///
/// Reads `brain_template()` off the consolidated `CharacterArchetypeSpec` so adding
/// a new archetype is a single row, not a parallel match.
pub(in crate::features) fn enemy_default_brain(enemy: &ActorConfig) -> Brain {
    match enemy.brain_spec.template {
        CharacterBrainTemplate::StandStill => Brain::StateMachine(StateMachineCfg::StandStill),
        CharacterBrainTemplate::Wanderer => Brain::StateMachine(StateMachineCfg::Wanderer {
            cfg: WandererCfg::PUPPY_SLUG_DEFAULT,
            state: WandererState::default(),
        }),
        CharacterBrainTemplate::MeleeBrute => melee_brute_brain_for_enemy(enemy),
        CharacterBrainTemplate::ChargeCrash => charge_crash_brain_for_enemy(enemy),
        CharacterBrainTemplate::Skirmisher => skirmisher_brain_for_enemy(enemy),
        CharacterBrainTemplate::Sniper => sniper_brain_for_enemy(enemy),
        CharacterBrainTemplate::Smash => Brain::StateMachine(StateMachineCfg::Smash {
            cfg: smash_cfg_from_spec(&enemy.brain_spec, &enemy.tuning),
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
    let roam_radius = (110.0 + 60.0 * jitters.2).max(t.attack_range * 1.5);
    Brain::StateMachine(StateMachineCfg::Aerial {
        cfg: ambition_characters::brain::state_machine::AerialCfg {
            aggressiveness: if t.attacks_player { 1.0 } else { 0.0 },
            cruise_speed,
            dive_speed,
            aggro_radius: t.aggro_radius,
            attack_range: t.attack_range,
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
            skirmisher_brain_from_tuning(&enemy.id, &enemy.tuning, true),
            action_set,
        );
    }

    if let Some(min_aggro) = enemy.brain_spec.provoke_forced_brute_min_aggro {
        let brain = forced_hostile_melee_brute_brain(enemy, min_aggro);
        return (brain, action_set);
    }
    (enemy_default_brain(enemy), action_set)
}

fn forced_hostile_melee_brute_brain(enemy: &ActorConfig, min_aggro_radius: f32) -> Brain {
    let t = &enemy.tuning;
    let jitters = five_f32s_from_seed(seed_from_id(&enemy.id));
    let aggro_radius = t.aggro_radius.max(min_aggro_radius) * (0.9 + 0.2 * jitters.0);
    let chase_speed = t.chase_speed * (0.9 + 0.2 * jitters.1);
    let attack_range = t.attack_range.max(56.0) * (0.95 + 0.1 * jitters.2);
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
    let aggro_radius = t.aggro_radius * (0.8 + 0.4 * jitters.0);
    let chase_speed = t.chase_speed * (0.85 + 0.3 * jitters.1);
    let attack_range = t.attack_range * (0.9 + 0.2 * jitters.2);
    Brain::StateMachine(StateMachineCfg::MeleeBrute {
        cfg: MeleeBruteCfg {
            aggressiveness: if t.attacks_player { 1.0 } else { 0.0 },
            aggro_radius,
            attack_range,
            chase_speed,
        },
        state: MeleeBruteState::default(),
    })
}

pub(super) fn skirmisher_brain_for_enemy(enemy: &ActorConfig) -> Brain {
    skirmisher_brain_from_tuning(&enemy.id, &enemy.tuning, enemy.tuning.attacks_player)
}

fn sniper_brain_for_enemy(enemy: &ActorConfig) -> Brain {
    let t = &enemy.tuning;
    let jitters = five_f32s_from_seed(seed_from_id(&enemy.id));
    let base_cooldown_s = 1.5;
    let fire_cooldown_s = base_cooldown_s * (0.75 + 0.5 * jitters.0);
    let initial_cooldown_s = fire_cooldown_s * (0.3 + 0.7 * jitters.1);
    Brain::StateMachine(StateMachineCfg::Sniper {
        cfg: SniperCfg {
            aggressiveness: if t.attacks_player { 1.0 } else { 0.0 },
            aggro_radius: t.aggro_radius,
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
    let aggro_radius = t.aggro_radius * (0.85 + 0.3 * jitters.0);
    let cruise_speed = t.chase_speed * (0.85 + 0.25 * jitters.1);
    let charge_speed = (cruise_speed * (2.0 + 0.4 * jitters.2)).max(360.0);
    let bite_range = t.attack_range * (0.85 + 0.15 * jitters.3);
    let charge_duration_s = 0.38 + 0.18 * jitters.4;
    let charge_cooldown_s = 0.75 + 0.55 * jitters.1;
    let standoff_px = (t.attack_range * 0.40).max(140.0) * (0.8 + 0.4 * jitters.2);
    let vertical_wobble_px = (t.attack_range * 0.12).max(20.0) * (0.8 + 0.4 * jitters.3);
    let orbit_drift_rad_s = 0.55 + 0.7 * jitters.4;
    Brain::StateMachine(StateMachineCfg::ChargeCrash {
        cfg: ChargeCrashCfg {
            aggressiveness: if t.attacks_player { 1.0 } else { 0.0 },
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
) -> (Brain, ActionSet) {
    // Rebuild the rider's solo action set from its DURABLE stored combat
    // kit (`innate_melee` / `innate_ranged` / `move_style`) plus its live
    // held item — the same inputs the spawn projection used, queried off
    // the entity so the runtime dismount never re-reads the roster enum.
    let mut action_set = kit.to_action_set(held_item);
    if action_set.melee.is_none() {
        action_set.melee = super::super::enemies::spec_for_brain(
            &ambition_entity_catalog::placements::CharacterBrain::Custom("pirate_raider".into()),
        )
        .melee_spec();
    }

    // If the dismounted rider still has a ranged held item, keep using a
    // ranged-capable brain so the weapon remains live after the shark dies.
    // This preserves the item as the authority: remove / change the held item
    // in data and this path changes without another Rust branch.
    let brain = if held_item.is_some_and(|item| item.grants_ranged()) {
        skirmisher_brain_from_tuning(&rider.id, &rider.tuning, true)
    } else {
        forced_hostile_melee_brute_brain(rider, 540.0)
    };
    (brain, action_set)
}

fn skirmisher_brain_from_tuning(
    actor_id: &str,
    tuning: &ActorTuning,
    force_hostile: bool,
) -> Brain {
    let jitters = five_f32s_from_seed(seed_from_id(actor_id));
    let base_cooldown_s = 1.5;
    let fire_cooldown_s = base_cooldown_s * (0.75 + 0.5 * jitters.0);
    let initial_cooldown_s = fire_cooldown_s * (0.3 + 0.7 * jitters.1);
    let standoff_base = (tuning.attack_range * 0.35).max(120.0);
    let standoff_px = standoff_base * (0.8 + 0.4 * jitters.2);
    let orbit_phase = jitters.3 * std::f32::consts::TAU;
    let orbit_drift_rad_s = 0.4 + 0.8 * jitters.4;
    Brain::StateMachine(StateMachineCfg::Skirmisher {
        cfg: SkirmisherCfg {
            aggressiveness: if force_hostile || tuning.attacks_player {
                1.0
            } else {
                0.0
            },
            aggro_radius: tuning.aggro_radius,
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
fn smash_cfg_from_spec(spec: &CharacterBrainSpec, tuning: &ActorTuning) -> SmashCfg {
    // Heavy vs striker base + per-archetype hit band + dash-to-close are
    // projected onto `CharacterBrainSpec` at spawn (`smash_hit_band`,
    // `smash_heavy`, `smash_dash_to_close`), so this builder reads generic
    // data rather than matching the roster enum. The 36 px hit-band
    // fallback lives in the projection.
    // Duelist > heavy > striker. The duelist base brings the neutral game
    // (footsies / neutral hops / spacing + retreat) that makes a platform
    // fighter MOVE instead of camping point-blank; `attack_range` /
    // `engage_distance` are still overridden from the body's hit band below, so
    // the spacing weaves around the body's real reach.
    let base = if spec.smash_duelist {
        SmashCfg::DUELIST_DEFAULT
    } else if spec.smash_heavy {
        SmashCfg::BRUTE_DEFAULT
    } else {
        SmashCfg::STRIKER_DEFAULT
    };
    let hit_band = spec.smash_hit_band;
    SmashCfg {
        aggro_radius: tuning.aggro_radius,
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
        dash_to_close: spec.smash_dash_to_close,
        // Blink-evade kit (authored per archetype). The brain *attempts* a blink
        // on a perceived lunge; the body's `CombatCapabilities::can_blink` +
        // blink cooldown *enforce* it. `blink_cooldown_s` here is the brain's
        // reactive restraint (policy, I4); the body owns the physical floor (I3).
        can_blink: spec.smash_can_blink,
        blink_cooldown_s: if spec.smash_can_blink { 1.2 } else { 0.0 },
        // Grounded-base hybrid flyer: the brain *prefers* grounded and flies only
        // to cover a long traversal gap (the decision lives in `decide_flight`).
        // The body's `CombatCapabilities::can_fly` is the matching enforce gate.
        can_fly: spec.smash_can_fly,
        // Reactive block: the brain *attempts* a guard (raises `shield_held`,
        // stands ground) on a perceived lunge it won't blink; the body's
        // `CombatCapabilities::can_shield` is the matching enforce gate.
        can_shield: spec.smash_can_shield,
        ..base
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_engine_core as ae;

    fn enemy(brain_key: &str) -> super::super::actor_clusters::ActorClusterSeed {
        super::super::actor_clusters::ActorClusterSeed::new(
            "e",
            "E",
            ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(24.0, 40.0)),
            ambition_entity_catalog::placements::CharacterBrain::Custom(brain_key.into()),
            &[],
        )
    }

    #[test]
    fn medium_striker_archetype_gets_a_smash_brain() {
        // The common striker (goblins) runs the Smash state machine; this
        // guards the archetype-template -> concrete-Brain dispatch.
        let e = enemy("medium_striker");
        assert!(
            matches!(
                enemy_default_brain(&e.config),
                Brain::StateMachine(StateMachineCfg::Smash { .. })
            ),
            "medium_striker should default to a Smash brain"
        );
    }

    #[test]
    fn default_action_set_is_derived_without_panicking_for_a_striker() {
        // The combat-kit -> action-set projection should yield a usable set
        // (the striker has a melee verb).
        let e = enemy("medium_striker");
        let set = enemy_default_action_set(&e.spec);
        assert!(set.melee.is_some(), "a striker should expose a melee verb");
    }

    #[test]
    fn medium_striker_carries_a_ranged_rock() {
        // Goblins now poke with a thrown rock at mid-range and close for the
        // swing — the Smash brain's verb-selection-by-range. Lock the
        // RON(`ranged: Some(Rock)`) → CombatKit → ActionSet wiring so a future
        // edit can't silently drop the ranged verb (which would revert goblins
        // to melee-only without any test noticing).
        let e = enemy("medium_striker");
        let set = enemy_default_action_set(&e.spec);
        assert!(
            matches!(
                set.ranged,
                Some(ambition_characters::brain::RangedActionSpec::Rock { .. })
            ),
            "medium_striker should carry a ranged Rock verb; got {:?}",
            set.ranged
        );
        assert!(set.melee.is_some(), "and still keeps its melee swing");
    }
}
