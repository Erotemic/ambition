//! `SnapshotState` for this crate's own types — the rollback wire format.
//!
//! These impls live HERE, beside the types they encode, because
//! `ambition_platformer2d_core::snapshot` owns the trait and the orphan rule binds an impl to the
//! crate owning the trait OR the type. The orphan rule is what proves this file is in the right
//! crate: if a type moves, this stops compiling rather than drifting.
//!
//! A field added to an encoded type is a WIRE FORMAT change. Encode and
//! decode must stay in the same order, and `snapshot_unit_enum!` codes are
//! authored per variant so inserting one never renumbers the rest.

use ambition_platformer2d_core::snapshot::{
    put_bool, put_f32, put_i32, put_opt_str, put_str, put_u32, put_u64, put_u8, put_vec2, Reader,
    SnapshotCursor, SnapshotState,
};
use ambition_platformer2d_core::{self as ae, snapshot_pod, snapshot_unit_enum};

snapshot_unit_enum!(crate::actor::ai::CharacterAiMode {
    Idle = 0,
    Patrol = 1,
    Chase = 2,
    Telegraph = 3,
    Attack = 4,
    Recover = 5,
    Stunned = 6,
    Dead = 7,
});

// The platform-fighter half of a capture. Split off
// `ambition_combat:capture:CapturedBy`: the relation is generic
// and these four are the ruleset's, so they rewind under this crate's ownership
// rather than widening a row somebody else owns.
snapshot_pod!(crate::smash_capture::SmashHoldState {
    pummels_landed: u8,
    held_for: f32,
    mash_credit: f32,
    escape_seconds: f32,
    // The throw edge is an INPUT LATCH and it must rewind: a rollback past the
    // frame the captor centred the stick has to un-arm it, or a replay throws
    // where the live session pummelled.
    throw_armed: bool,
    // The cargo carry. Decided once and constant after, which is exactly why it
    // has to rewind: a restore without it gives the resimulated captor a hold
    // they can no longer walk with.
    carrying: bool,
});

snapshot_pod!(crate::actor::body::BodyCombat {
    hit_flash: f32,
    hitstop_timer: f32,
    asdi_owed: bool,
    landing_lag_timer: f32,
    damage_invuln_timer: f32,
    hitstun_timer: f32,
    recoil_lock_timer: f32,
    // ASLEEP. A control status a move applied, and rollback state for the same
    // reason every lock beside it is: two peers disagreeing about how long a
    // fighter stays helpless resimulate different matches from that moment.
    sleep_timer: f32,
    // DERIVED and still encoded: it is republished from the live move every
    // tick, so a restore converges — but the tick a rewind lands ON reads it
    // before the projection runs again, and a peer that decided a launch from a
    // stale armor bit has already diverged.
    armored: bool,
    training_dummy: bool,
});

// Actor-side mutable state. An attack cooldown that survives a rollback is an
// attack the enemy did not pay for.
snapshot_pod!(crate::actor::pose::ActorPose {
    center: vec2,
    feet: vec2,
    facing: f32,
});

snapshot_unit_enum!(crate::actor::DeathPolicy {
    HpDepleted = 0,
    Unbounded = 1,
});

/// This encoding still carried only the pool, and `decode` rebuilt the component with
/// `BodyHealth::new` — which resets the meter to 0 and the policy to `HpDepleted`.
///
/// `CanonicalCodecStrategy` uses this encoding for the STORED GGRS value, so it
/// was not a checksum omission: a fighter at 188% under `Unbounded` came back
/// from any rewind at 0% under `HpDepleted`. Knockback scales off the meter, so
/// its launch distance silently reset; and under the restored policy later damage
/// began draining the pool, so it could die by HP in a ruleset whose whole design
/// is that only the world kills. The checksum could not see any of it, because it
/// was computed over the same incomplete representation.
///
/// The policy rides as a coded unit enum rather than a bool, so a third variant
/// is a new code rather than a re-interpretation of an old byte.
impl SnapshotState for crate::actor::BodyHealth {
    fn encode(&self, out: &mut Vec<u8>) {
        put_i32(out, self.health.current);
        put_i32(out, self.health.max);
        put_u32(out, self.health.invulnerable.bits());
        put_i32(out, self.damage_taken());
        self.policy().encode(out);
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        let health = crate::actor::Health {
            current: r.i32()?,
            max: r.i32()?,
            invulnerable: crate::actor::Invulnerability::from_bits(r.u32()?),
        };
        let damage_taken = r.i32()?;
        let policy = crate::actor::DeathPolicy::decode(r)?;
        Some(crate::actor::BodyHealth::restored(
            health,
            damage_taken,
            policy,
        ))
    }
}

/// The canonical playable-persona identity: WHICH catalog character a body
/// wears. A length-delimited string id — the choice, not the content: the
/// catalog it names is authored data that survives the rewind. Registered as a
/// full component (not a resolve) because the id IS the value; the entity's
/// gameplay/presentation are re-derived from the restored identity (and, for
/// HostCode, the restored `BodyAbilities`) the following tick.
impl SnapshotState for crate::actor::WornCharacter {
    fn encode(&self, out: &mut Vec<u8>) {
        put_str(out, self.id());
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(crate::actor::WornCharacter::new(r.str()?))
    }
}

snapshot_unit_enum!(crate::actor::ActorFaction {
    Player = 0,
    Enemy = 1,
    Npc = 2,
    Boss = 3,
    Neutral = 4,
});

/// `Strike(key)` / `Special(key)` — a keyed reference by construction, because "a new
/// geometry strike is a new key + authored rects, with NO edit to this enum".
impl SnapshotState for crate::brain::boss_pattern::BossAttackProfile {
    fn encode(&self, out: &mut Vec<u8>) {
        use crate::brain::boss_pattern::BossAttackProfile as P;
        match self {
            P::Strike(key) => {
                put_u8(out, 0);
                put_str(out, key);
            }
            P::Special(key) => {
                put_u8(out, 1);
                put_str(out, key);
            }
        }
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        use crate::brain::boss_pattern::BossAttackProfile as P;
        match r.u8()? {
            0 => Some(P::Strike(r.str()?.to_string())),
            1 => Some(P::Special(r.str()?.to_string())),
            _ => None,
        }
    }
}

impl SnapshotState for crate::brain::boss_pattern::BossAttackState {
    fn encode(&self, out: &mut Vec<u8>) {
        put_opt_profile(out, &self.telegraph_profile);
        put_f32(out, self.telegraph_remaining);
        put_f32(out, self.telegraph_elapsed);
        put_opt_profile(out, &self.active_profile);
        put_f32(out, self.active_remaining);
        put_f32(out, self.active_elapsed);
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        use crate::brain::boss_pattern::BossAttackState;
        let telegraph_profile = read_opt_profile(r)?;
        let telegraph_remaining = r.f32()?;
        let telegraph_elapsed = r.f32()?;
        Some(BossAttackState {
            telegraph_profile,
            telegraph_remaining,
            telegraph_elapsed,
            active_profile: read_opt_profile(r)?,
            active_remaining: r.f32()?,
            active_elapsed: r.f32()?,
        })
    }
}

impl SnapshotState for crate::brain::boss_pattern::BossAttackIntent {
    fn encode(&self, out: &mut Vec<u8>) {
        put_opt_profile(out, &self.telegraph_profile);
        put_opt_profile(out, &self.active_profile);
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(crate::brain::boss_pattern::BossAttackIntent {
            telegraph_profile: read_opt_profile(r)?,
            active_profile: read_opt_profile(r)?,
        })
    }
}

snapshot_unit_enum!(crate::brain::boss_pattern::BossEncounterPhase {
    Dormant = 0,
    Intro = 1,
    Phase1 = 2,
    Transition = 3,
    Phase2 = 4,
    Stagger = 5,
    Enrage = 6,
    Death = 7,
});

/// Not a unit enum — `Approach` and `Retreat` carry their own clocks, and a boss
/// that rewinds into `Retreat` must rewind to the same retreat POSITION. Explicit
/// discriminants for the same reason as `snapshot_unit_enum!`.
impl SnapshotState for crate::brain::boss_pattern::BossMacroState {
    fn encode(&self, out: &mut Vec<u8>) {
        use crate::brain::boss_pattern::BossMacroState as M;
        match self {
            M::Engage => put_u8(out, 0),
            M::Approach { remaining_s } => {
                put_u8(out, 1);
                put_f32(out, *remaining_s);
            }
            M::Retreat {
                remaining_s,
                retreat_pos,
            } => {
                put_u8(out, 2);
                put_f32(out, *remaining_s);
                put_vec2(out, *retreat_pos);
            }
        }
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        use crate::brain::boss_pattern::BossMacroState as M;
        match r.u8()? {
            0 => Some(M::Engage),
            1 => Some(M::Approach {
                remaining_s: r.f32()?,
            }),
            2 => Some(M::Retreat {
                remaining_s: r.f32()?,
                retreat_pos: r.vec2()?,
            }),
            _ => None,
        }
    }
}

/// One beat of a resolved boss timeline.
///
/// `resolve_timeline` rolls every `Select` away before the first tick of the fight runs
/// — *"Select rolled away, Stance markers left in place as jumps"* — so a resolved
/// timeline holds only these four. A `Select` that survives into one is an invariant
/// violation, and this encodes it as a tag no decoder accepts: rejected, never silently
/// reinterpreted as a `Rest`.
///
/// The steps are *resolved instance state*, not authored content. The authored thing is
/// the `BossPattern`; the timeline is what one weighted roll made of it. Rewinding a
/// boss without rewinding the roll gives it a different fight.
impl SnapshotState for crate::brain::boss_pattern::BossPatternStep {
    fn encode(&self, out: &mut Vec<u8>) {
        use crate::brain::boss_pattern::BossPatternStep as S;
        match self {
            S::Telegraph {
                profile,
                duration,
                telegraph,
            } => {
                put_u8(out, 0);
                profile.encode(out);
                put_f32(out, *duration);
                match telegraph {
                    None => put_bool(out, false),
                    Some(spec) => {
                        put_bool(out, true);
                        put_opt_str(out, spec.pose.as_deref());
                        put_opt_str(out, spec.cue.as_deref());
                        put_opt_str(out, spec.vfx.as_deref());
                    }
                }
            }
            S::Strike { profile, duration } => {
                put_u8(out, 1);
                profile.encode(out);
                put_f32(out, *duration);
            }
            S::Rest { duration } => {
                put_u8(out, 2);
                put_f32(out, *duration);
            }
            S::Stance { id } => {
                put_u8(out, 3);
                put_str(out, id);
            }
            // Unreachable in a resolved timeline. Tag 4 decodes to `None`.
            //
            // and if it IS reached, the release build loses state silently. Tag 4 round-trips
            // to `None`, so a rewind restores a timeline missing this step and the divergence
            // surfaces later as a checksum mismatch with no name attached — the hardest kind to
            // trace.
            //
            // The wire format is deliberately unchanged: this claims to be
            // unreachable, and a new tag would be a schema change made on a
            // suspicion. The log fires exactly when the claim is false.
            S::Select { .. } => {
                debug_assert!(false, "a resolved timeline still holds a `Select`");
                bevy::log::error!(
                    target: "ambition_platformer2d::snapshot",
                    "a resolved timeline still holds a `Select`; it encodes as \
                     absent and will come back absent from a rewind"
                );
                put_u8(out, 4);
            }
        }
    }

    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        use crate::brain::boss_pattern::{BossAttackProfile, BossPatternStep as S, TelegraphSpec};
        match r.u8()? {
            0 => {
                let profile = BossAttackProfile::decode(r)?;
                let duration = r.f32()?;
                let telegraph = if r.bool()? {
                    Some(TelegraphSpec {
                        pose: r.opt_str()?.map(str::to_string),
                        cue: r.opt_str()?.map(str::to_string),
                        vfx: r.opt_str()?.map(str::to_string),
                    })
                } else {
                    None
                };
                Some(S::Telegraph {
                    profile,
                    duration,
                    telegraph,
                })
            }
            1 => Some(S::Strike {
                profile: BossAttackProfile::decode(r)?,
                duration: r.f32()?,
            }),
            2 => Some(S::Rest { duration: r.f32()? }),
            3 => Some(S::Stance {
                id: r.str()?.to_string(),
            }),
            _ => None,
        }
    }
}

/// Rewind the mutable cursor of state-machine brains while leaving authored tuning in place.
/// Boss-pattern and Smash brains both carry replay-significant internal clocks/history.
impl SnapshotCursor for crate::brain::Brain {
    fn encode_cursor(&self, out: &mut Vec<u8>) {
        use crate::brain::{Brain, StateMachineCfg};
        match self {
            Brain::StateMachine(StateMachineCfg::BossPattern { state, .. }) => {
                put_u8(out, 1);
                match &state.last_phase {
                    None => put_bool(out, false),
                    Some(phase) => {
                        put_bool(out, true);
                        phase.encode(out);
                    }
                }
                put_u32(out, state.step_index as u32);
                put_f32(out, state.step_elapsed);
                put_f32(out, state.movement_timer);
                put_f32(out, state.pattern_timer);
                put_f32(out, state.cycle_rest_remaining);
                state.macro_state.encode(out);
                put_f32(out, state.engage_timer);
                put_u64(out, state.rng_seed);
                put_timeline(out, &state.timeline);
                put_opt_str(out, state.stance.as_deref());
                put_u32(out, state.stance_stack.len() as u32);
                for ret in &state.stance_stack {
                    put_timeline(out, &ret.timeline);
                    put_opt_str(out, ret.stance.as_deref());
                    put_u32(out, ret.step_index as u32);
                    put_f32(out, ret.step_elapsed);
                }
                put_u32(out, state.interrupt_cooldowns.len() as u32);
                for value in &state.interrupt_cooldowns {
                    put_f32(out, *value);
                }
                put_u32(out, state.interrupt_timers.len() as u32);
                for value in &state.interrupt_timers {
                    put_f32(out, *value);
                }
                match state.last_hp {
                    None => put_bool(out, false),
                    Some(hp) => {
                        put_bool(out, true);
                        put_i32(out, hp);
                    }
                }
            }
            Brain::StateMachine(StateMachineCfg::Smash { state, .. }) => {
                put_u8(out, 2);
                put_smash_state(out, state);
            }
            // FB4b. EVERY field of `FighterState` gates what the brain does next,
            // so every field is projected — the derive-memo rule applied in
            // advance. The two that would be easiest to leave out are the two
            // that matter most: `noise` decides press timing (the same fighter
            // would throw a different jab on a replay) and `apm` decides whether
            // a press happens at all.
            //
            // The perception buffer is deliberately NOT projected. It is a
            // window over views the world already owns and rewinds, and encoding
            // it would put every observed actor's full state into the checksum
            // of every fighter that can see them — quadratic, and a duplicate of
            // truth the population sweep already covers. Its DEPTH is a config,
            // not state.
            Brain::StateMachine(StateMachineCfg::Fighter { state, .. }) => {
                put_u8(out, 3);
                put_u32(out, state.ticks_until_decision);
                put_u32(out, state.apm.presses);
                put_u32(out, state.apm.elapsed_ticks);
                match state.pending_press {
                    None => put_bool(out, false),
                    Some(pending) => {
                        put_bool(out, true);
                        put_u32(out, pending.ticks);
                        put_u32(out, pending.hold_ticks);
                        put_u8(
                            out,
                            match pending.binding.verb {
                                crate::brain::attack_kit::AttackVerb::Basic => 0,
                                crate::brain::attack_kit::AttackVerb::Smash => 1,
                                crate::brain::attack_kit::AttackVerb::Special => 2,
                                crate::brain::attack_kit::AttackVerb::Grab => 3,
                            },
                        );
                        put_u8(out, attack_dir_tag(pending.binding.direction));
                    }
                }
                put_u64(out, state.noise);
                match &state.last_foe {
                    None => put_bool(out, false),
                    Some(foe) => {
                        put_bool(out, true);
                        put_bool(out, foe.attacking);
                        put_bool(out, foe.on_ground);
                        put_bool(out, foe.shielding);
                        put_f32(out, foe.closing);
                    }
                }
                // The habit model is a read of the opponent that survives across
                // decisions, so a rewind that restored a fighter with somebody
                // else's reads would play differently. Rows in a stable order —
                // `rows()` is over a BTreeMap, not a hash map.
                let rows: Vec<_> = state.habits.rows().collect();
                put_u32(out, rows.len() as u32);
                for ((situation, choice), weight) in rows {
                    put_u8(out, situation as u8);
                    put_u8(out, choice as u8);
                    put_f32(out, weight);
                }
                // The HELD intent, by the fields the emission actually uses.
                // `ActorControlFrame` is not `SnapshotState` — it is a per-tick
                // command, not stored state — and projecting the whole struct
                // here would freeze a wire format over a type that exists to
                // change.
                put_f32(out, state.held.locomotion.x);
                put_f32(out, state.held.locomotion.y);
                put_f32(out, state.held.facing);
                put_bool(out, state.held.jump_held);
                put_bool(out, state.held.shield_held);
                put_bool(out, state.held.melee_held);
                // The charge in flight. `melee_held` above is DERIVED from it,
                // so projecting only the button would let two brains agree on
                // this tick's press and disagree about how much longer the smash
                // is being paid for.
                put_u32(out, state.charge_hold_ticks);
                // ...and WHICH button it is holding, because the two sustains
                // are derived from the pair. Projecting only the counter would
                // let a rewound brain hold the wrong button for the right
                // number of ticks.
                put_u8(
                    out,
                    match state.charge_hold_gesture {
                        ambition_entity_catalog::ChargeGesture::Smash => 0,
                        ambition_entity_catalog::ChargeGesture::Special => 1,
                    },
                );
            }
            _ => put_u8(out, 0),
        }
    }
}

/// The explicit brain SELECTION for a catalog-backed NPC: its character
/// default preset plus whether it is on the default or an override. Self-contained
/// (preset-id strings only — no `Entity`, no runtime brain), so it is a plain
/// `register_component` and restores its own presence.
///
/// This is the authoritative snapshot state for "which brain is selected". The
/// live [`Brain`](crate::brain::Brain) cursor is a no-op for the
/// peaceful/patrol NPC brains, so after a rewind PAST a runtime brain switch the
/// live brain kind could disagree with the restored selection —
/// [`reconcile_brain_bindings`] rebuilds the brain from this binding to make them
/// agree before the next re-simulated tick.
impl SnapshotState for crate::actor::character_catalog::BrainBinding {
    fn encode(&self, out: &mut Vec<u8>) {
        use crate::actor::character_catalog::AutonomousDefault;
        use crate::actor::character_catalog::AutonomousSource;
        // THREE cases, not two. A tag byte: 0 = nothing to restore (a boss), 1 = a catalog preset,
        // 2 = the character's own authored profile.
        match &self.default_preset {
            AutonomousDefault::Preset(preset) => {
                put_u8(out, 1);
                put_str(out, preset.as_str());
            }
            AutonomousDefault::None => put_u8(out, 0),
            AutonomousDefault::CharacterProfile => put_u8(out, 2),
        }
        match &self.source {
            AutonomousSource::CatalogDefault => put_u8(out, 0),
            AutonomousSource::CatalogPreset(preset) => {
                put_u8(out, 1);
                put_str(out, preset.as_str());
            }
            // This carried a `HostileArchetypeId` that was always the string `"combatant"` — one
            // row, forever — so the rollback road resolved a roster the live road had already
            // stopped consulting. The engine states the default provoked policy; there is nothing
            // to look up.
            AutonomousSource::ProvokedDefault => put_u8(out, 2),
            // Boss: the live brain is a `BossPattern` rebuilt from the boss
            // catalog by this id (or resumed from the suspended runtime), never a
            // catalog preset. The stable boss id is all a rebuild needs.
            AutonomousSource::Boss { archetype } => {
                put_u8(out, 3);
                put_str(out, archetype.as_str());
            }
            // A provoked CHARACTER's own combat policy, by canonical id — the
            // same shape as its three neighbours, and the reason the variant
            // names a profile instead of carrying one: a tag and a string,
            // rather than a dozen tuned floats in the rollback codec.
            AutonomousSource::ProvokedProfile { profile } => {
                put_u8(out, 4);
                put_str(out, profile.as_str());
            }
            // A TAG AND NOTHING ELSE. The character's own authored policy
            // is the default; the character is reachable from the body's
            // identity, so there is no id to carry — and carrying the
            // `BrainProfile` itself would put a dozen tuned floats in the
            // rollback codec, which is the mistake `ProvokedProfile` above was
            // rewritten to avoid.
            AutonomousSource::CharacterProfile => put_u8(out, 5),
        }
    }

    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        use crate::actor::character_catalog::{
            AutonomousDefault, AutonomousSource, BossAutonomyId, BrainBinding, BrainPresetId,
        };
        let default_preset = match r.u8()? {
            0 => AutonomousDefault::None,
            1 => AutonomousDefault::Preset(BrainPresetId::new(r.str()?.to_string())),
            2 => AutonomousDefault::CharacterProfile,
            _ => return None,
        };
        let source = match r.u8()? {
            0 => AutonomousSource::CatalogDefault,
            1 => AutonomousSource::CatalogPreset(BrainPresetId::new(r.str()?.to_string())),
            2 => AutonomousSource::ProvokedDefault,
            3 => AutonomousSource::Boss {
                archetype: BossAutonomyId::new(r.str()?.to_string()),
            },
            4 => AutonomousSource::ProvokedProfile {
                profile: ambition_entity_catalog::BrainProfileId::new(r.str()?.to_string()),
            },
            5 => AutonomousSource::CharacterProfile,
            _ => return None,
        };
        Some(BrainBinding {
            default_preset,
            source,
        })
    }
}

/// The authored brain-build context (spawn anchor + patrol radius) a catalog NPC
/// rebuilds its default/override brain from. A self-contained POD component, so a
/// plain `register_component`. Snapshot-safe so a restored `RestoreDefault` /
/// [`reconcile_brain_bindings`] recenters a patrol brain on its AUTHORED home, not
/// wherever the actor wandered before the rewind.
impl SnapshotState for crate::actor::character_catalog::AuthoredBrainContext {
    fn encode(&self, out: &mut Vec<u8>) {
        put_f32(out, self.spawn_anchor_x);
        match self.patrol_radius {
            Some(r) => {
                put_bool(out, true);
                put_f32(out, r);
            }
            None => put_bool(out, false),
        }
    }

    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(crate::actor::character_catalog::AuthoredBrainContext {
            spawn_anchor_x: r.f32()?,
            patrol_radius: if r.bool()? { Some(r.f32()?) } else { None },
        })
    }
}

/// The brain's last-tick intent, which the sim reads on the NEXT tick — the
/// `brain/README.md` calls it exactly that. So it is state, not a per-frame scratchpad,
/// and a rewind that leaves it stale hands the body an input it never chose.
///
/// Every field, in declaration order. There is no clever half of this component.
impl SnapshotState for crate::control::ActorControl {
    fn encode(&self, out: &mut Vec<u8>) {
        let f = &self.0;
        put_vec2(out, f.locomotion.vec());
        put_vec2(out, f.velocity_target.vec());
        put_f32(out, f.facing);
        put_bool(out, f.melee_pressed);
        put_bool(out, f.melee_held);
        put_bool(out, f.melee_released);
        // ⛔ A BYTE, NOT A BOOL, and this is the schema change: the strength
        // hint became three-valued (`Auto`/`Tilt`/`Smash`) so a tilt-stick can
        // force a tilt at full deflection. Encoding it as the old bool would
        // round `Tilt` to whichever of the two it happened to alias.
        put_u8(
            out,
            match f.melee_strength_hint {
                ae::AttackStrengthHint::Auto => 0,
                ae::AttackStrengthHint::Tilt => 1,
                ae::AttackStrengthHint::Smash => 2,
            },
        );
        match &f.fire {
            None => put_bool(out, false),
            Some(fire) => {
                put_bool(out, true);
                put_vec2(out, fire.dir);
                fire.dir_policy.encode(out);
                put_f32(out, fire.speed);
            }
        }
        put_vec2(out, f.attack_axis.vec());
        for b in [
            f.jump_pressed,
            f.jump_held,
            f.jump_released,
            f.burst_pressed,
            f.interact_pressed,
            f.body_contact_damage_enabled,
            f.shield_held,
            f.special_pressed,
            // The special SUSTAIN, and it is rollback state for the same reason
            // `shield_held` above it is: a chargeable neutral special reads it
            // every tick, so a resimulated tick that lost it would release a
            // charge the original tick was still holding.
            f.special_held,
            f.pogo_pressed,
            f.fast_fall_pressed,
            f.fly_toggle_pressed,
            f.projectile_pressed,
            f.projectile_held,
            f.projectile_released,
            f.blink_pressed,
            f.blink_held,
            f.blink_released,
            // The sustained modifier is rollback state like any other control
            // level: a body's rules read it every tick, so a resimulated tick that
            // lost it would diverge from the one it is replacing.
            f.modifier_held,
            f.modifier_pressed,
            // The capture EDGE. An edge is rollback state exactly like a level:
            // a resimulated tick that lost the press would not attempt the grab
            // the original tick attempted, and the two histories diverge at the
            // relationship rather than at a pixel.
            f.grab_pressed,
            // Same argument as the grab edge: a lost taunt press replays as a
            // body that never taunted, and the two histories diverge.
            f.taunt_pressed,
        ] {
            put_bool(out, b);
        }
        put_vec2(out, f.blink_quick_dir.vec());
        put_vec2(out, f.blink_aim_step.vec());
        put_vec2(out, f.aim.vec());
    }

    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        use crate::actor::control::{ActorControlFrame, ActorFireRequest};
        use ambition_platformer2d_core::reference_frame::GameplayFramePolicy;
        let locomotion = ae::LocalAxes::from_vec(r.vec2()?);
        let velocity_target = ae::WorldVec2(r.vec2()?);
        let facing = r.f32()?;
        let melee_pressed = r.bool()?;
        let melee_held = r.bool()?;
        let melee_released = r.bool()?;
        let melee_strength_hint = match r.u8()? {
            0 => ae::AttackStrengthHint::Auto,
            1 => ae::AttackStrengthHint::Tilt,
            2 => ae::AttackStrengthHint::Smash,
            // ⛔ REFUSE rather than default. A byte this codec did not write is
            // a stream from another schema, and silently reading it as `Auto`
            // would desync one peer's tilt into another's smash.
            _ => return None,
        };
        let fire = if r.bool()? {
            Some(ActorFireRequest {
                dir: r.vec2()?,
                dir_policy: GameplayFramePolicy::decode(r)?,
                speed: r.f32()?,
            })
        } else {
            None
        };
        let attack_axis = ae::LocalAxes::from_vec(r.vec2()?);
        let mut flags = [false; 22];
        for f in flags.iter_mut() {
            *f = r.bool()?;
        }
        Some(crate::control::ActorControl(ActorControlFrame {
            locomotion,
            // ⛔ NOT ON THE WIRE, and deliberately. `damped_by_move_motion`
            // records it on a LOCAL COPY at each integrator, after this
            // component has been read — the stored frame is always the
            // undamped one, so there is nothing here to restore.
            undamped_locomotion: None,
            velocity_target,
            facing,
            melee_pressed,
            melee_held,
            melee_released,
            melee_strength_hint,
            fire,
            attack_axis,
            jump_pressed: flags[0],
            jump_held: flags[1],
            jump_released: flags[2],
            burst_pressed: flags[3],
            interact_pressed: flags[4],
            body_contact_damage_enabled: flags[5],
            shield_held: flags[6],
            special_pressed: flags[7],
            special_held: flags[8],
            pogo_pressed: flags[9],
            fast_fall_pressed: flags[10],
            fly_toggle_pressed: flags[11],
            projectile_pressed: flags[12],
            projectile_held: flags[13],
            projectile_released: flags[14],
            blink_pressed: flags[15],
            blink_held: flags[16],
            blink_released: flags[17],
            modifier_held: flags[18],
            modifier_pressed: flags[19],
            grab_pressed: flags[20],
            taunt_pressed: flags[21],
            blink_quick_dir: ae::WorldVec2(r.vec2()?),
            blink_aim_step: ae::WorldVec2(r.vec2()?),
            aim: ae::LocalAxes::from_vec(r.vec2()?),
        }))
    }
}

impl SnapshotState for crate::actor::BodyWallet {
    fn encode(&self, out: &mut Vec<u8>) {
        put_i32(out, self.balance);
    }

    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(Self { balance: r.i32()? })
    }
}

impl SnapshotState for crate::actor::attack_gesture::AttackGestureState {
    fn encode(&self, out: &mut Vec<u8>) {
        put_bool(out, self.flick_armed);
        match self.recent_flick {
            None => put_bool(out, false),
            Some(flick) => {
                put_bool(out, true);
                put_u8(out, attack_dir_tag(flick.direction));
                put_u8(out, flick.age_ticks);
            }
        }
        match self.active {
            None => put_bool(out, false),
            Some(intent) => {
                put_bool(out, true);
                put_attack_gesture_intent(out, intent);
            }
        }
        match self.buffered_press {
            None => put_bool(out, false),
            Some(intent) => {
                put_bool(out, true);
                put_attack_gesture_intent(out, intent);
            }
        }
        // The SPECIAL press waiting for its window, whose meaning cannot be
        // recovered from the live stick a few ticks later.
        match self.buffered_special {
            None => put_bool(out, false),
            Some(intent) => {
                use crate::actor::attack_gesture::AttackPosture;
                put_bool(out, true);
                put_u8(out, attack_dir_tag(intent.direction));
                put_u8(
                    out,
                    match intent.posture {
                        AttackPosture::Grounded => 0,
                        AttackPosture::Airborne => 1,
                    },
                );
            }
        }
        put_u8(out, self.special_turn_ticks);
        put_f32(out, self.prev_lateral_sign);
    }

    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        use crate::actor::attack_gesture::{AttackGestureState, RecentAttackFlick};
        let flick_armed = r.bool()?;
        let recent_flick = if r.bool()? {
            Some(RecentAttackFlick {
                direction: attack_dir_from_tag(r.u8()?)?,
                age_ticks: r.u8()?,
            })
        } else {
            None
        };
        let active = if r.bool()? {
            Some(read_attack_gesture_intent(r)?)
        } else {
            None
        };
        let buffered_press = if r.bool()? {
            Some(read_attack_gesture_intent(r)?)
        } else {
            None
        };
        let buffered_special = if r.bool()? {
            use crate::actor::attack_gesture::{AttackPosture, SpecialGestureIntent};
            Some(SpecialGestureIntent {
                direction: attack_dir_from_tag(r.u8()?)?,
                posture: match r.u8()? {
                    0 => AttackPosture::Grounded,
                    1 => AttackPosture::Airborne,
                    _ => return None,
                },
            })
        } else {
            None
        };
        Some(AttackGestureState {
            flick_armed,
            recent_flick,
            active,
            buffered_press,
            buffered_special,
            special_turn_ticks: r.u8()?,
            prev_lateral_sign: r.f32()?,
        })
    }
}

impl SnapshotState for crate::actor::attack_gesture::AttackGestureTuning {
    fn encode(&self, out: &mut Vec<u8>) {
        put_f32(out, self.flick_threshold);
        put_f32(out, self.rearm_threshold);
        put_u8(out, self.flick_window_ticks);
        put_f32(out, self.special_turn_deflection);
        put_u8(out, self.special_turn_window_ticks);
        put_f32(out, self.directional_deadzone);
        put_f32(out, self.action_buffer_s);
    }

    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(Self {
            flick_threshold: r.f32()?,
            rearm_threshold: r.f32()?,
            flick_window_ticks: r.u8()?,
            special_turn_deflection: r.f32()?,
            special_turn_window_ticks: r.u8()?,
            directional_deadzone: r.f32()?,
            action_buffer_s: r.f32()?,
        })
    }
}

fn put_opt_profile(out: &mut Vec<u8>, v: &Option<crate::brain::boss_pattern::BossAttackProfile>) {
    match v {
        None => put_bool(out, false),
        Some(p) => {
            put_bool(out, true);
            p.encode(out);
        }
    }
}

#[allow(clippy::option_option)]
fn read_opt_profile(
    r: &mut Reader<'_>,
) -> Option<Option<crate::brain::boss_pattern::BossAttackProfile>> {
    use crate::brain::boss_pattern::BossAttackProfile as P;
    Some(if r.bool()? { Some(P::decode(r)?) } else { None })
}

fn put_timeline(out: &mut Vec<u8>, steps: &[crate::brain::boss_pattern::BossPatternStep]) {
    put_u32(out, steps.len() as u32);
    for s in steps {
        s.encode(out);
    }
}

fn put_smash_mode(out: &mut Vec<u8>, mode: crate::brain::smash::BroadMode) {
    use crate::brain::smash::BroadMode;
    put_u8(
        out,
        match mode {
            BroadMode::Idle => 0,
            BroadMode::Approach => 1,
            BroadMode::Retreat => 2,
            BroadMode::Engage => 3,
            BroadMode::Reposition => 4,
            BroadMode::Recover => 5,
        },
    );
}

fn put_smash_state(out: &mut Vec<u8>, state: &crate::brain::smash::SmashState) {
    put_smash_mode(out, state.mode);
    put_f32(out, state.mode_dwell_s);
    put_u64(out, state.rng_seed);
    put_f32(out, state.sprint_cooldown_remaining);
    let (samples, write, count) = state.obs_history.snapshot_parts();
    for (time, pos) in samples {
        put_f32(out, *time);
        put_vec2(out, *pos);
    }
    put_u32(out, write as u32);
    put_u32(out, count as u32);
    put_f32(out, state.spacing_phase);
    put_f32(out, state.neutral_jump_cooldown);
    put_f32(out, state.blink_cooldown);
    put_f32(out, state.foray_timer);
    put_f32(out, state.shield_hold_timer);
    put_f32(out, state.neutral_reset_timer);
    put_bool(out, state.was_attacking);
    put_f32(out, state.regroup_timer);
    put_f32(out, state.last_health_fraction);
    put_f32(out, state.damage_accum);
    put_f32(out, state.time_since_offense);
}

fn attack_dir_tag(dir: crate::actor::attack_gesture::AttackDir) -> u8 {
    use crate::actor::attack_gesture::AttackDir;
    match dir {
        AttackDir::Neutral => 0,
        AttackDir::Forward => 1,
        AttackDir::Up => 2,
        AttackDir::Down => 3,
        AttackDir::Back => 4,
    }
}

fn attack_dir_from_tag(tag: u8) -> Option<crate::actor::attack_gesture::AttackDir> {
    use crate::actor::attack_gesture::AttackDir;
    Some(match tag {
        0 => AttackDir::Neutral,
        1 => AttackDir::Forward,
        2 => AttackDir::Up,
        3 => AttackDir::Down,
        4 => AttackDir::Back,
        _ => return None,
    })
}

fn put_attack_gesture_intent(
    out: &mut Vec<u8>,
    intent: crate::actor::attack_gesture::AttackGestureIntent,
) {
    use crate::actor::attack_gesture::{AttackInputPhase, AttackPosture, AttackStrength};
    put_u8(out, attack_dir_tag(intent.direction));
    put_u8(
        out,
        match intent.strength {
            AttackStrength::Tilt => 0,
            AttackStrength::Smash => 1,
        },
    );
    put_u8(
        out,
        match intent.posture {
            AttackPosture::Grounded => 0,
            AttackPosture::Airborne => 1,
        },
    );
    put_u8(
        out,
        match intent.phase {
            AttackInputPhase::Press => 0,
            AttackInputPhase::Hold => 1,
            AttackInputPhase::Release => 2,
        },
    );
}

fn read_attack_gesture_intent(
    r: &mut Reader<'_>,
) -> Option<crate::actor::attack_gesture::AttackGestureIntent> {
    use crate::actor::attack_gesture::{
        AttackGestureIntent, AttackInputPhase, AttackPosture, AttackStrength,
    };
    Some(AttackGestureIntent {
        direction: attack_dir_from_tag(r.u8()?)?,
        strength: match r.u8()? {
            0 => AttackStrength::Tilt,
            1 => AttackStrength::Smash,
            _ => return None,
        },
        posture: match r.u8()? {
            0 => AttackPosture::Grounded,
            1 => AttackPosture::Airborne,
            _ => return None,
        },
        phase: match r.u8()? {
            0 => AttackInputPhase::Press,
            1 => AttackInputPhase::Hold,
            2 => AttackInputPhase::Release,
            _ => return None,
        },
    })
}

#[cfg(test)]
mod body_health_wire_tests {
    use crate::actor::{BodyHealth, DeathPolicy, Health};

    fn round_trip(health: &BodyHealth) -> BodyHealth {
        let bytes = ambition_platformer2d_core::snapshot::encode_state(health);
        ambition_platformer2d_core::snapshot::decode_state::<BodyHealth>(&bytes)
            .expect("the encoding decodes")
    }

    /// A fighter above 100% under `Unbounded` comes back as itself.
    ///
    /// The encoding carried the pool only, and `decode` rebuilt the component
    /// with `BodyHealth::new` — a zero meter and the default policy. This is the
    /// smallest statement of what that cost.
    #[test]
    fn a_body_over_100_percent_keeps_its_meter_and_its_policy() {
        let mut health = BodyHealth::new(Health::new(100)).with_policy(DeathPolicy::Unbounded);
        health.damage(188);
        assert_eq!(health.damage_taken(), 188);
        assert!(health.alive(), "an unbounded pool does not drain");

        let restored = round_trip(&health);
        assert_eq!(
            restored.damage_taken(),
            188,
            "the meter reset across the wire, so the body's knockback scaling \
             silently went back to a fresh fighter's"
        );
        assert_eq!(
            restored.policy(),
            DeathPolicy::Unbounded,
            "the policy reset to HpDepleted, so later damage drains the pool and \
             the fighter can die by HP in a ruleset where only the world kills"
        );
        assert_eq!(restored.current(), health.current());
        assert_eq!(restored.max(), health.max());
    }

    /// The ordinary body is unchanged — the default policy and an empty meter
    /// survive as themselves rather than by accident.
    #[test]
    fn an_ordinary_damaged_body_round_trips_too() {
        let mut health = BodyHealth::new(Health::new(50));
        health.damage(20);
        let restored = round_trip(&health);
        assert_eq!(restored.current(), 30);
        assert_eq!(restored.damage_taken(), 20);
        assert_eq!(restored.policy(), DeathPolicy::HpDepleted);
    }
}
impl SnapshotState for crate::control::SlotInteractionState {
    fn encode(&self, out: &mut Vec<u8>) {
        for index in 0..crate::control::SlotControls::MAX_SLOTS {
            let gestures = self.get(crate::control::PlayerSlot(index as u8));
            put_f32(out, gestures.down_tap_timer);
            put_f32(out, gestures.up_tap_timer);
            put_f32(out, gestures.interact_buffer_timer);
            put_bool(out, gestures.double_tap_down_pending);
            put_bool(out, gestures.double_tap_up_pending);
            put_f32(out, gestures.up_hold_timer);
        }
    }

    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        let mut state = Self::default();
        for index in 0..crate::control::SlotControls::MAX_SLOTS {
            // `?` rather than an unwrap: the loop is bounded by `MAX_SLOTS`, so
            // this cannot be `None` today, and a decode that could not address
            // one of its own slots is a failed decode rather than a panic.
            *state.get_mut(crate::control::PlayerSlot(index as u8))? =
                crate::control::SlotGestures {
                    down_tap_timer: r.f32()?,
                    up_tap_timer: r.f32()?,
                    interact_buffer_timer: r.f32()?,
                    double_tap_down_pending: r.bool()?,
                    double_tap_up_pending: r.bool()?,
                    up_hold_timer: r.f32()?,
                };
        }
        Some(state)
    }
}

#[cfg(test)]
mod attack_gesture_wire_tests {
    use crate::actor::attack_gesture::{
        AttackDir, AttackGestureIntent, AttackGestureState, AttackInputPhase, AttackPosture,
        AttackStrength,
    };

    /// A press held open by the input buffer is rollback state: a resimulation
    /// that restores mid-window has to replay the SAME press, or the corrected
    /// timeline throws a tilt where the original threw a smash.
    ///
    /// ⭐ BOTH SLOTS, and the SPECIAL one carries a direction and a posture for
    /// the same reason the attack one carries a strength: neither can be
    /// recovered from the live stick a few ticks later. A buffered up-special
    /// replayed off a centred stick is a neutral special, which is a different
    /// move.
    #[test]
    fn a_buffered_press_survives_the_wire_intact() {
        use crate::actor::attack_gesture::SpecialGestureIntent;
        let state = AttackGestureState {
            flick_armed: false,
            recent_flick: None,
            active: None,
            buffered_press: Some(AttackGestureIntent {
                direction: AttackDir::Forward,
                strength: AttackStrength::Smash,
                posture: AttackPosture::Grounded,
                phase: AttackInputPhase::Press,
            }),
            buffered_special: Some(SpecialGestureIntent {
                direction: AttackDir::Up,
                posture: AttackPosture::Airborne,
            }),
            // Mid-window and mid-flick: a restore that lost either would
            // classify the replayed B-reverse differently.
            special_turn_ticks: 3,
            prev_lateral_sign: -1.0,
        };
        let bytes = ambition_platformer2d_core::snapshot::encode_state(&state);
        let back = ambition_platformer2d_core::snapshot::decode_state::<AttackGestureState>(&bytes)
            .expect("the encoding decodes");
        assert_eq!(
            back, state,
            "the buffered press did not round-trip, so a rollback across the \
             buffer window replays a different press than the one that was made"
        );
    }
}

impl SnapshotState for crate::actor::ai::ActorStatus {
    fn encode(&self, out: &mut Vec<u8>) {
        put_f32(out, self.respawn_timer);
        self.ai_mode.encode(out);
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(crate::actor::ai::ActorStatus {
            respawn_timer: r.f32()?,
            ai_mode: crate::actor::ai::CharacterAiMode::decode(r)?,
        })
    }
}
