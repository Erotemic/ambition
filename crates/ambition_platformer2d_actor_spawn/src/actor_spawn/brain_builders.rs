//! Shared Brain + ActionSet construction for ECS feature actors.
//!
//! Spawning, mount/rider transitions, and hostile NPC flips should all come
//! through this module when they need to attach or replace actor brain
//! components. Keeping the construction policy here prevents each caller from
//! hand-rolling a slightly different mix of archetype tuning, aggressiveness,
//! and per-actor jitter.

use ambition_characters::brain::{
    ActionSet, Brain, ChargeCrashCfg, ChargeCrashState, MeleeBruteCfg, MeleeBruteState,
    SkirmisherCfg, SkirmisherState, SmashCfg, SmashState, SniperCfg, SniperState, StateMachineCfg,
    WandererCfg,
};
use ambition_combat::actor_tuning::ActorConfig;
use ambition_combat::components::ActorIdentity;
use ambition_combat::actor_tuning::{ActorTuning, BrainProfile, CharacterBrainTemplate};
use ambition_combat::variation::{five_f32s_from_seed, seed_from_id};

// The constant is `ambition_characters:actor:DEFAULT_UNAUTHORED_BODY_HEALTH` now — the pool a
// body gets when no authority describes it, asked at construction — and provocation no longer
// writes health at all.

// All three read a body's weapons off an `ArchetypeSpec` — the melee timings, the ranged spec,
// the gait, the held item, the signature move — and their one production caller was
// `EnemyActorSpawnPlan::hostile`, which asked them only when the seed carried an archetype.
//
// ⇒ what a body fights with comes from its CHARACTER, through the one persona
// writer (`grant_prepared_character_body`).

/// Deterministic RNG seed for a fighter brain.
///
/// Ordinary fighters mix difficulty with the stable participant id, so mirror
/// seats get independent streams without using clocks, process RNG, or Bevy
/// entity ids. Characters that explicitly preserve mirror symmetry instead mix
/// difficulty with the character id, giving twins the same initial stream; they
/// still diverge naturally once their observations differ.
fn fighter_cognition_seed(enemy: &ActorConfig, id: &str, level: u8) -> u64 {
    // A participant id is `"<character>#seat<n>"`; the character alone is what is
    // left when the seat is dropped. Falling back to the whole id keeps a body
    // that carries no seat suffix (a room spawn) on a stream of its own rather
    // than silently joining a shared one.
    let identity = if enemy.preserves_mirror_symmetry {
        id.split_once('#')
            .map_or(id, |(character, _seat)| character)
    } else {
        id
    };
    // MIX, not add: `seed_from_id` is a 32-bit FNV-1a, so shifting it into the
    // high half and folding the level in below keeps two nearby levels of one
    // participant far apart in the stream rather than adjacent.
    ((seed_from_id(identity) as u64) << 32) ^ 0x5F37_7A11_u64.wrapping_mul(level as u64 + 1)
}

/// Build the enemy's default `Brain` from its resolved controller profile.
pub fn enemy_default_brain(
    enemy: &ActorConfig,
    // The policy the driver plays by (`ActorPolicy`): the body's current one,
    // or the one a provocation is installing.
    policy: &BrainProfile,
    // Who the body is: per-actor streams and jitter are keyed on its id.
    identity: &ActorIdentity,
    // **THE BODY'S OWN VERBS**, not a policy's opinion of them. See
    // [`smash_cfg_from_spec`]: a driver may only consider what this body can
    // actually do, so the same profile on a different body produces a driver
    // that reaches for different things.
    body: ambition_platformer2d_core::AbilitySet,
) -> Brain {
    lower_policy(enemy, policy, identity, body, enemy.tuning.is_hostile)
}

/// Lower a driver's policy onto a body, engaged or not.
///
/// `hostile` is the engagement being INSTALLED: the placement's at
/// construction, and `true` for a provocation. It is never read back off the
/// construction record, which a provocation does not rewrite, so a provoked
/// body could otherwise be handed a peaceful mind under a hostile standing.
fn lower_policy(
    enemy: &ActorConfig,
    policy: &BrainProfile,
    identity: &ActorIdentity,
    body: ambition_platformer2d_core::AbilitySet,
    hostile: bool,
) -> Brain {
    match policy.template {
        CharacterBrainTemplate::StandStill => Brain::StateMachine(StateMachineCfg::StandStill),
        CharacterBrainTemplate::Fighter => {
            let level = policy.fighter_level;
            let cfg = ambition_characters::brain::fighter::FighterCfg::new(
                ambition_characters::brain::fighter::FighterBrainProfile::for_level(level),
            );
            let state = ambition_characters::brain::fighter::FighterState::new(
                &cfg,
                fighter_cognition_seed(enemy, &identity.id, level),
            );
            Brain::StateMachine(StateMachineCfg::Fighter {
                cfg: Box::new(cfg),
                state: Box::new(state),
            })
        }
        CharacterBrainTemplate::Wanderer => Brain::StateMachine(StateMachineCfg::Wanderer {
            cfg: WandererCfg::PUPPY_SLUG_DEFAULT,
        }),
        CharacterBrainTemplate::MeleeBrute => {
            melee_brute_brain_for_enemy(enemy, policy, &identity.id, hostile)
        }
        CharacterBrainTemplate::ChargeCrash => {
            charge_crash_brain_for_enemy(enemy, policy, &identity.id, hostile)
        }
        CharacterBrainTemplate::Skirmisher => {
            skirmisher_brain_from_tuning(&identity.id, &enemy.tuning, policy, hostile)
        }
        CharacterBrainTemplate::Sniper => sniper_brain_for_enemy(policy, &identity.id, hostile),
        CharacterBrainTemplate::Smash => Brain::StateMachine(StateMachineCfg::Smash {
            cfg: smash_cfg_from_spec(policy, &enemy.tuning, body),
            state: SmashState {
                rng_seed: seed_from_id(&identity.id) as u64,
                ..Default::default()
            },
        }),
        CharacterBrainTemplate::Aerial => aerial_brain_for_enemy(enemy, policy, &identity.id, hostile),
    }
}

/// Build the hostile aerial dive-bomber brain for an enemy archetype (the sky
/// parrot). Per-actor jitter keeps a flock from diving in lockstep. Shares
/// `StateMachineCfg::Aerial` with the peaceful catalog bird — only
/// `aggressiveness` differs.
fn aerial_brain_for_enemy(
    enemy: &ActorConfig,
    policy: &BrainProfile,
    id: &str,
    hostile: bool,
) -> Brain {
    let chase_speed = policy.chase_speed(enemy.tuning.max_run_speed);
    let jitters = five_f32s_from_seed(seed_from_id(id));
    let cruise_speed = chase_speed * (0.55 + 0.25 * jitters.0);
    let dive_speed = (chase_speed * (1.7 + 0.5 * jitters.1)).max(360.0);
    // Dive altitude / range: a bit of spread so two parrots stack their dives.
    let roam_radius = (110.0 + 60.0 * jitters.2).max(policy.attack_range * 1.5);
    Brain::StateMachine(StateMachineCfg::Aerial {
        cfg: ambition_characters::brain::state_machine::AerialCfg {
            aggressiveness: if hostile { 1.0 } else { 0.0 },
            cruise_speed,
            dive_speed,
            aggro_radius: policy.aggro_radius,
            attack_range: policy.attack_range,
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
pub fn aggressive_brain_for_enemy(
    enemy: &ActorConfig,
    policy: &BrainProfile,
    identity: &ActorIdentity,
    repertoire: Option<&ActionSet>,
    body: ambition_platformer2d_core::AbilitySet,
) -> Brain {
    // WHAT THE BODY CAN DO IS THE SELECTOR, and it is read rather than rebuilt.
    // A body's repertoire is the projection of its identity, its worn equipment
    // and its hand (`ambition_characters::repertoire`), maintained by the
    // reconcile; provocation changes who is deciding, not what the body is
    // holding, so this asks the live set instead of folding a second answer
    // from a subset of those inputs.
    //
    // A ranged-only weapon wants a spacing brain; a melee-capable actor should
    // close and swing. A pirate authored with a bow and no melee slot becomes a
    // Skirmisher without a Rust-side item-id branch.
    // `Option` so this stays a READ rather than a filter: a body that carries
    // no repertoire component answers the same question as one whose repertoire
    // is empty, which is the brain below rather than a skirmisher.
    let ranged_only = repertoire.is_some_and(|set| set.ranged.is_some() && set.melee.is_none());
    if ranged_only {
        return skirmisher_brain_from_tuning(&identity.id, &enemy.tuning, policy, true);
    }

    if let Some(min_aggro) = policy.provoke_forced_brute_min_aggro {
        return forced_hostile_melee_brute_brain(enemy, policy, &identity.id, min_aggro);
    }
    lower_policy(enemy, policy, identity, body, true)
}

fn forced_hostile_melee_brute_brain(
    enemy: &ActorConfig,
    policy: &BrainProfile,
    id: &str,
    min_aggro_radius: f32,
) -> Brain {
    let jitters = five_f32s_from_seed(seed_from_id(id));
    let aggro_radius =
        policy.aggro_radius.max(min_aggro_radius) * (0.9 + 0.2 * jitters.0);
    let chase_speed = policy.chase_speed(enemy.tuning.max_run_speed) * (0.9 + 0.2 * jitters.1);
    let attack_range = policy.attack_range.max(56.0) * (0.95 + 0.1 * jitters.2);
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

pub(super) fn melee_brute_brain_for_enemy(
    enemy: &ActorConfig,
    policy: &BrainProfile,
    id: &str,
    hostile: bool,
) -> Brain {
    let jitters = five_f32s_from_seed(seed_from_id(id));
    let aggro_radius = policy.aggro_radius * (0.8 + 0.4 * jitters.0);
    let chase_speed = policy.chase_speed(enemy.tuning.max_run_speed) * (0.85 + 0.3 * jitters.1);
    let attack_range = policy.attack_range * (0.9 + 0.2 * jitters.2);
    Brain::StateMachine(StateMachineCfg::MeleeBrute {
        cfg: MeleeBruteCfg {
            aggressiveness: if hostile { 1.0 } else { 0.0 },
            aggro_radius,
            attack_range,
            chase_speed,
        },
        state: MeleeBruteState::default(),
    })
}

fn sniper_brain_for_enemy(policy: &BrainProfile, id: &str, hostile: bool) -> Brain {
    let jitters = five_f32s_from_seed(seed_from_id(id));
    let base_cooldown_s = 1.5;
    let fire_cooldown_s = base_cooldown_s * (0.75 + 0.5 * jitters.0);
    let initial_cooldown_s = fire_cooldown_s * (0.3 + 0.7 * jitters.1);
    Brain::StateMachine(StateMachineCfg::Sniper {
        cfg: SniperCfg {
            aggressiveness: if hostile { 1.0 } else { 0.0 },
            aggro_radius: policy.aggro_radius,
            fire_cooldown_s,
        },
        state: SniperState {
            cooldown_remaining: initial_cooldown_s,
        },
    })
}

fn charge_crash_brain_for_enemy(
    enemy: &ActorConfig,
    policy: &BrainProfile,
    id: &str,
    hostile: bool,
) -> Brain {
    let jitters = five_f32s_from_seed(seed_from_id(id));
    let aggro_radius = policy.aggro_radius * (0.85 + 0.3 * jitters.0);
    let cruise_speed = policy.chase_speed(enemy.tuning.max_run_speed) * (0.85 + 0.25 * jitters.1);
    let charge_speed = (cruise_speed * (2.0 + 0.4 * jitters.2)).max(360.0);
    let bite_range = policy.attack_range * (0.85 + 0.15 * jitters.3);
    let charge_duration_s = 0.38 + 0.18 * jitters.4;
    let charge_cooldown_s = 0.75 + 0.55 * jitters.1;
    let standoff_px =
        (policy.attack_range * 0.40).max(140.0) * (0.8 + 0.4 * jitters.2);
    let vertical_wobble_px =
        (policy.attack_range * 0.12).max(20.0) * (0.8 + 0.4 * jitters.3);
    let orbit_drift_rad_s = 0.55 + 0.7 * jitters.4;
    Brain::StateMachine(StateMachineCfg::ChargeCrash {
        cfg: ChargeCrashCfg {
            aggressiveness: if hostile { 1.0 } else { 0.0 },
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

/// The solo brain a rider receives when its mount dies.
///
/// A dismount is a BRAIN transition and nothing else: falling off a shark does
/// not change the rider's identity, its worn equipment or what is in its hand,
/// so its repertoire is already what it will be on the ground. This reads the
/// config and the held item and consults no kit.
///
/// A rider still holding a ranged item keeps a ranged-capable brain so the
/// weapon stays live after the shark dies — the item is the authority, so
/// changing it in data changes this path with no second Rust branch.
///
/// It is deliberately not `enemy_default_brain`: PirateRaider's default is
/// Smash, which has tighter grounded observation requirements, and PirateHeavy's
/// default is peaceful. Dismount means "fall off and fight".
pub fn dismounted_rider_brain(
    rider: &ActorConfig,
    policy: &BrainProfile,
    identity: &ActorIdentity,
    held_item: Option<&ambition_characters::brain::HeldItemSpec>,
) -> Brain {
    if held_item.is_some_and(|item| item.grants_ranged()) {
        skirmisher_brain_from_tuning(&identity.id, &rider.tuning, policy, true)
    } else {
        forced_hostile_melee_brute_brain(rider, policy, &identity.id, 540.0)
    }
}

fn skirmisher_brain_from_tuning(
    actor_id: &str,
    tuning: &ActorTuning,
    profile: &BrainProfile,
    hostile: bool,
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
            aggressiveness: if hostile { 1.0 } else { 0.0 },
            aggro_radius: profile.aggro_radius,
            standoff_px,
            strafe_speed: profile.chase_speed(tuning.max_run_speed),
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
        chase_speed: profile.chase_speed(tuning.max_run_speed),
        retreat_speed: profile.chase_speed(tuning.max_run_speed) * 0.75,
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
    fn seat(
        character: &str,
        seat_index: usize,
        level: u8,
        mirrors: bool,
    ) -> (ActorIdentity, ActorConfig, BrainProfile) {
        let identity = ActorIdentity::new(format!("{character}#seat{seat_index}"), character);
        let config = ActorConfig {
            tuning: ActorTuning::default(),
            brain: ambition_entity_catalog::placements::CharacterBrain::Passive,
            preserves_mirror_symmetry: mirrors,
        };
        let policy = BrainProfile {
            template: CharacterBrainTemplate::Fighter,
            fighter_level: level,
            ..Default::default()
        };
        (identity, config, policy)
    }

    /// The stream a seat's fighter brain is actually built on — asked through the
    /// real builder, not through the seed helper, so the test constrains the
    /// composition and not an internal function.
    fn stream_for((identity, config, policy): &(ActorIdentity, ActorConfig, BrainProfile)) -> u64 {
        match enemy_default_brain(config, policy, identity, ambition_platformer2d_core::AbilitySet::NONE) {
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
        room_body.0.id = "npc_emmy_noether_lab_copy".to_string();
        let mut other_room_body = room_body.clone();
        other_room_body.0.id = "npc_emmy_noether_hall_copy".to_string();
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
            enemy_default_brain(&ordinary.1, &ordinary.2, &ordinary.0, ambition_platformer2d_core::AbilitySet::NONE),
            enemy_default_brain(&mirroring.1, &mirroring.2, &mirroring.0, ambition_platformer2d_core::AbilitySet::NONE),
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

// ⇒ the constant those pins protected is unchanged, which is what the pins
// were for: `UNDESCRIBED_BODY_RESPAWN` is now the only authority on its
// question and says what the row said. (The provoked policy that sat beside it
// is content now: a provider declares it and preparation resolves it.) The
// template → brain-family mapping is pinned off a CHARACTER's profile by
// `enemy_default_brain_picks_the_family_its_policy_names` in the spawn tests.

// ⇒ the row is gone and the constant is unchanged. The KIT half of it went
// further: a body must be DRIVEN by some policy, because "no policy" is not a
// state a driven body can be in, but "no repertoire" is an ordinary state and
// needs no engine answer at all. `default_fighting_kit()` is deleted; a body
// nobody described swings nothing.

/// What provocation produces: a MIND and a KIT. Never a body.
///
/// The comment three lines above the code that did it already stated the correct invariant:
/// *"provocation is one body, a different driver, a changed relationship. The body stays exactly as
/// its character built it."* It was describing the OTHER branch.
///
///  what a provocation may change is the POLICY the body is driven by, the KIT
/// it swings if it has none of its own, and its relationship to whoever struck
/// it. Its speed, its locomotion, its capabilities and its silhouette are facts
/// about the creature, and being hit is not an argument about any of them.
///
/// do not add a third. Every field on this struct is now a MIND or a KIT;
/// a body fact reappearing here is the ontology growing back.
///
/// and the brain is lowered against the BODY's tuning now, not the
/// archetype's — §4.7, a policy states normalized effort and the body states the
/// speed. A provoked villager chases at a villager's top speed, which is the
/// same sentence as the paragraph above with the consequence attached.
///
/// Reached through [`provoked_mind`], which the live provoke flip and
/// construction from a save's provocation fact both apply, so a provoked actor
/// is identical whether it was just challenged or rebuilt by a room replay.
pub struct ProvokedArchetype {
    pub brain_profile: BrainProfile,
    pub brain: Brain,
}

/// What a provocation makes of a body: the mind it projects, and the source a
/// [`BrainBinding`] records so a rewind or a release knows which policy is live.
///
/// ⛔ ONE ANSWER FOR EVERY ROAD THAT PROVOKES. The live flip and construction
/// from a save's provocation fact each asked it twice — once for a character
/// that authors its own provoked policy, once for the engine default — and the
/// two authored arms had drifted from the default ones: they skipped the
/// `config.brain` read-model, and the live one rebound and rebuilt an
/// ALREADY-hostile body, recording a policy it never installed.
///
/// [`BrainBinding`]: ambition_characters::actor::character_catalog::BrainBinding
pub struct ProvokedMind {
    pub projection: ProvokedArchetype,
    pub source: ambition_characters::actor::character_catalog::AutonomousSource,
}

/// The provoked policy a character resolved at preparation (its own, else its
/// provider's declared default), with the id a binding records, or `None` when
/// it has none and cannot be provoked.
pub fn authored_provoked_policy(
    prepared: &ambition_characters::prepared::PreparedCharacterRegistry,
    character: &str,
) -> Option<(BrainProfile, ambition_entity_catalog::BrainProfileId)> {
    let character = prepared.get(character)?;
    Some((
        character.provoked_profile?,
        character.provoked_profile_id.clone()?,
    ))
}

/// See [`ProvokedMind`].
pub fn provoked_mind(
    (policy, id): (BrainProfile, ambition_entity_catalog::BrainProfileId),
    current_config: &ActorConfig,
    identity: &ActorIdentity,
    repertoire: Option<&ambition_characters::brain::ActionSet>,
    body: ambition_platformer2d_core::AbilitySet,
) -> ProvokedMind {
    use ambition_characters::actor::character_catalog::AutonomousSource;
    let source = AutonomousSource::ProvokedProfile { profile: id };
    ProvokedMind {
        projection: provoked_projection(policy, current_config, identity, repertoire, body),
        source,
    }
}

/// The projection itself, from a POLICY rather than from a row: the one the
/// character resolved at preparation.
pub fn provoked_projection(
    brain_profile: BrainProfile,
    current_config: &ActorConfig,
    identity: &ambition_combat::components::ActorIdentity,
    repertoire: Option<&ambition_characters::brain::ActionSet>,
    body: ambition_platformer2d_core::AbilitySet,
) -> ProvokedArchetype {
    // the POLICY is the provoked one; the BODY is the one that was struck.
    let brain = aggressive_brain_for_enemy(
        current_config,
        &brain_profile,
        identity,
        repertoire,
        body,
    );
    ProvokedArchetype {
        brain,
        brain_profile,
    }
}
