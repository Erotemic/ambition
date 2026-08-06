//! **FB6 — L3, forward rollouts on a shadow model**
//! (`docs/planning/engine/fighter-brain.md` §12).
//!
//! The rollout does NOT run the sim. The sim is authoritative, stateful, and
//! omniscient; an imagination must be cheap, pure, and exactly as ignorant as
//! its owner. [`ShadowState`] is that imagination: its only world input is a
//! [`Perceived`] view, so FB4a's type-level no-cheat enforcement carries over —
//! the rollout physically cannot contain a fact the brain could not see.
//!
//! **Determinism is load-bearing, not aspirational.** Brains run inside the
//! simulation, and under GGRS a resimulated decision tick must reproduce the
//! original decision bit-for-bit. So: the work is EXACTLY
//! `rollout_k × (1 + rollout_depth)` shadow steps (the profile's numbers ARE
//! the budget — no wall clock, no early exit, no "best so far"); the predicted
//! opponent is a pure function of the view and the [`HabitModel`] (§12.2 D3 —
//! modal habit when the model holds a genuine read, inertia otherwise); and
//! there is NO RNG anywhere in this module. Execution noise is FB4's business,
//! in the execution layer, with its own snapshot-registered stream.
//!
//! **The hit response is the real one.** `ae::hit_response` is the SAME kernel
//! `damage_apply` resolves authoritative hits with (§12.3 route 1 — carved to
//! the floor precisely so both callers exist). What the model still
//! approximates is everything else, and §12.3's stated-omissions list is
//! closed on purpose: future projectile fire, one-way platforms as anything but
//! floor, DI, shield break, cancels, charge, and any second hostile are all OUT
//! of v1. The fidelity instrument (FB6e) is what says when an omission starts
//! costing decisions — and on 2026-07-31 it said so for the first time: "terrain
//! beyond the stage box" was on this list, and `ladder_probe` traced a fighter
//! that killed itself at every difficulty back to it. The floor the body stands
//! on now has an EXTENT ([`ShadowFighter::ground_span`]); everything above it
//! remains omitted.

use ambition_entity_catalog::MoveFrameData;
use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::hit_response::{
    self, HitKnockback, HitKnockbackMagnitude, HitResponseTuning,
};

use super::habit::HabitModel;
use super::options::OptionSet;
use super::profile::FighterBrainProfile;
use super::situation::Situation;
use crate::perception::{BodyPhase, Perceived, PerceivedActor, SelfView, StageView};

/// Public knowledge about the game a rollout runs in: physics constants and
/// the standard hit response. A player who has played ten minutes knows all of
/// these numbers by feel; passing them in keeps the module pure while letting
/// the decision tick supply the game's true values. [`ShadowTuning::default`]
/// is a reasonable platformer, good enough for fixtures and tests.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShadowTuning {
    /// The victim-side hit response (launch + hitstun). The SAME kernel row
    /// shape `damage_apply` uses; the caller picks which feel row applies.
    pub response: HitResponseTuning,
    /// Gravity along `gravity_down`, engine units/s².
    pub gravity: f32,
    /// Grounded locomotion speed a driving fighter holds, units/s.
    pub ground_speed: f32,
    /// **Dash speed, which is not a faster walk.** The engine's dash SETS
    /// velocity outright (`abilities.rs`: `kinematics.vel = aim * dash_speed`)
    /// and does it airborne as readily as grounded.
    pub dash_speed: f32,
    /// How long that velocity is held (`DASH_TIME`). The dash is an impulse with
    /// a duration, not a sustained input, and the difference is what decides
    /// whether a dash near an edge is a step or a launch.
    pub dash_time: f32,
    /// Instant rise speed a predicted jump imparts, units/s.
    pub jump_speed: f32,
    /// **How fast a GROUNDED body that stopped driving loses its speed**, px/s².
    ///
    /// ⛔ **the shadow had no friction term at all until 2026-08-06** — the
    /// grounded `Hold` arm zeroed lateral velocity INSTANTLY, so a body leaving a
    /// dash at `dash_speed` stopped dead in the model and coasted ~38px in the
    /// game. That is an UNDER-prediction of travel, which is the direction that
    /// lets the movement veto approve a dash it should refuse — and
    /// `ladder_probe`'s open question had it as an air-side over-prediction,
    /// which is the opposite sign.
    ///
    /// `ae::GROUND_FRICTION`, restated here for the same reason `dash_speed` is:
    /// the constant lives above this crate and is public knowledge rather than
    /// hidden state.
    pub ground_coast_decel: f32,
    /// **How fast an AIRBORNE body bleeds lateral speed**, px/s².
    ///
    /// ⚠ the shadow's comment said *"an airborne body is ballistic and keeps
    /// everything"*, which is a real approximation and the SAFE direction — it
    /// over-predicts travel and makes the veto more cautious. It is still wrong,
    /// and modelling both sides costs one multiply.
    ///
    /// `ae::AIR_FRICTION`.
    pub air_coast_decel: f32,
    /// **What a SWING costs the body, backwards.** (`ae::SLASH_RECOIL`)
    ///
    /// Every melee press shoves the attacker along `-facing` by this much. It
    /// reads as a feel detail and it is the single biggest force acting on a
    /// fighter brain's body: the brain presses an attack on most decisions, so
    /// the recoils RATCHET — `ladder_probe` traced 200, 310, 420, 530 px/s in
    /// exact 110 steps while the emitted movement input pointed the other way.
    /// The fighter swung itself off the stage, backwards, one whiff at a time,
    /// and the rollout scored those lines as walks because nothing here knew a
    /// swing moved anything.
    pub slash_recoil: f32,
    /// Reach assumed for the FOE's attacks. The view names their phase and
    /// clock but not their move, so their range is a model assumption —
    /// stated, authored, and calibrated by FB6e's fidelity instrument.
    pub assumed_foe_reach: f32,
    /// Damage assumed for the foe's generic predicted attack.
    pub assumed_foe_damage: i32,
    /// Startup/active timings for that generic predicted attack, seconds.
    pub assumed_foe_startup_s: f32,
    pub assumed_foe_active_s: f32,
}

/// ⛔ **NOTHING DERIVES A `ShadowTuning` FROM THE BODY IT MODELS** — measured
/// 2026-08-06, and it is the structural version of the missing-friction bug one
/// level up.
///
/// ⭐ **the three divergent NUMBERS are corrected as of 2026-08-06** (gravity
/// 1400→2250, ground speed 160→270, jump 420→630) — see each field. The table
/// below is kept because it records what they were and why nobody noticed.
///
/// `ShadowTuning::default()` is the ONLY construction in the tree
/// (`FighterCfg::new` calls it and nobody overrides), so every fighter's rollout
/// predicts a body with these numbers. The duelist archetypes a smash CPU
/// actually wears author no movement override, so they inherit the engine
/// baseline — and three of these disagree with it:
///
/// | term | this model | the body | direction of the error |
/// |---|---|---|---|
/// | `gravity` | 1400 | `ae::GRAVITY` 2250 | longer airtime → OVER-predicts travel (cautious) |
/// | `ground_speed` | 160 | `MAX_RUN_SPEED` 270 | UNDER-predicts travel (dangerous) |
/// | `jump_speed` | 420 | `JUMP_SPEED` 630 | shorter arc → UNDER-predicts (dangerous) |
///
/// ⚠ **`ladder_probe` could not see this gap and said so for half a day.** Every
/// column there is byte-identical with 2250/270/630 substituted, because the only
/// rungs still dying under it carry `rollout_depth: 0` and never run a rollout.
///
/// ⭐ **`ladder_rig --scenarios` could.** Over §8's suite, 3 seeds, the rollout
/// rungs INVERTED in three recovery quadrants — a fighter with a rollout
/// recovering worse than one without. Correcting these three numbers removed two
/// of the three: `recovery_left 9v6` and `recovery_right 9v6` fall back inside
/// the seeds' spread, and only `recovery_above` still inverts. **Recovery is
/// where a wrong gravity has to show, and nothing had ever put a fighter
/// offstage.**
///
/// ⭐ **so the fix is not different constants, it is DERIVATION.** These should
/// come from the body's real `MovementTuning` the way `BrainSnapshot.attack_kit`
/// comes from its real `ActorMoveset` — body-derived truth filled in the
/// world-in port. Re-hardcoding a second set of numbers would leave the same
/// structural hole, and this one is currently unmeasurable, so a change here
/// would be a guess with no instrument. §8's scenario suite is what would make
/// it measurable.
impl Default for ShadowTuning {
    fn default() -> Self {
        Self {
            response: HitResponseTuning {
                knockback_x: 220.0,
                knockback_y: 260.0,
                hitstun_time: 0.35,
                di_max_angle: 0.0,
            },
            // ⛔ **1400 against the engine's `ae::GRAVITY` 2250 until 2026-08-06.**
            // A shadow that thinks gravity is 62% of real thinks a body HANGS —
            // so it plans recoveries the body cannot make, and every rollout
            // decision offstage was priced against a longer airtime than exists.
            gravity: 2250.0,
            // `MAX_RUN_SPEED`. 160 was a third short, so the model under-priced
            // how far a walk covers — the same under-prediction direction the
            // missing friction had.
            ground_speed: 270.0,
            // `ae::DASH_SPEED` / `ae::DASH_TIME`, restated here for the same
            // reason the foe's swing timings are: those constants live above
            // this crate, and they are public knowledge rather than hidden state.
            //
            // ⚠ the shadow used to model `Dash` as `Drive` — a 160 px/s GROUNDED
            // walk against a 760 px/s impulse that works in mid-air. 4.75x, in
            // the direction that matters: `ladder_probe` traced a level-9 fighter
            // that survived six seconds of veto-guarded walking, then dashed off
            // the right edge at 530 px/s while the rollout scored it as a stroll.
            dash_speed: 760.0,
            dash_time: 0.115,
            // `ae::JUMP_SPEED`. 420 was two thirds of it, which is the
            // dangerous direction for a ledge: a shorter modelled arc makes a
            // jump look safer than it is.
            jump_speed: 630.0,
            ground_coast_decel: 7600.0,
            air_coast_decel: 650.0,
            // `ae::SLASH_RECOIL`, restated for the same reason as the dash
            // numbers above.
            slash_recoil: 110.0,
            assumed_foe_reach: 60.0,
            assumed_foe_damage: 5,
            // The engine's standard enemy swing timings (combat events
            // vocabulary); restated here because those constants live above
            // this crate, and they are public knowledge, not hidden state.
            assumed_foe_startup_s: 0.36,
            assumed_foe_active_s: 0.20,
        }
    }
}

/// What a shadow fighter is doing. The asymmetry is honest: MY candidate move
/// has full frame data (I am deciding whether to throw it); the FOE's
/// commitment is known only as a phase and a clock, because that is all the
/// view shows.
#[derive(Clone, Debug, PartialEq)]
pub enum ShadowPhase {
    Idle,
    /// A move with known frame data — the candidate being rolled out.
    Move {
        frames: MoveFrameData,
        t: f32,
        landed: bool,
    },
    /// A commitment known only from perception: which phase, how long left.
    Committed {
        phase: BodyPhase,
        remaining: f32,
        landed: bool,
    },
    Hitstun {
        remaining: f32,
    },
}

/// One fighter, as much of it as a view shows.
#[derive(Clone, Debug, PartialEq)]
pub struct ShadowFighter {
    pub pos: ae::Vec2,
    pub vel: ae::Vec2,
    /// ±1, toward local `side`.
    pub facing: f32,
    pub half_extent: ae::Vec2,
    pub on_ground: bool,
    /// **How far the floor this body stands on actually reaches**, as `(min,
    /// max)` along the gravity frame's `side` axis. `None` is the old model: an
    /// INFINITE plane at `ground_level`.
    ///
    /// **Mid-air jumps left.** Without it the shadow has no air game at all:
    /// every airborne body falls to its death in the imagination, so every verb
    /// comes back fatal, the veto empties the list, and the halt fires on a body
    /// that cannot be helped by standing still. Recovery is the one thing an
    /// airborne fighter can DO, and it was the one thing the model could not
    /// represent.
    pub air_jumps: u8,
    /// Seconds left of an in-progress dash. Set when a dash line begins; ticked
    /// down by the integrator.
    pub dash_remaining: f32,
    /// ⚠ added 2026-07-31, and the absence was a whole class of blindness. v1's
    /// terrain model was one sentence — "a body that was STANDING re-lands at
    /// the height it stood at" — which is exactly right in an ENCLOSED room and
    /// silently wrong on a platform: a shadow body driven off the edge kept
    /// walking at the same height forever, so no rollout could ever see a
    /// walk-off. `ladder_probe` measured the consequence as identical self-KO
    /// counts at every ladder rung.
    pub ground_span: Option<(f32, f32)>,
    /// The ground plane this fighter stood on at capture (its `pos·down`),
    /// `None` if it started airborne. v1's whole terrain model: a body that
    /// was standing re-lands at the height it stood at.
    pub ground_level: Option<f32>,
    pub phase: ShadowPhase,
    pub damage: i32,
    pub health_max: i32,
    pub shield_raised: bool,
    pub invulnerable: bool,
    /// Set when a KO event fired for this fighter; a KOed body stops updating.
    pub koed: bool,
}

/// An in-flight hostile projectile, ballistic. `PerceivedProjectile` carries
/// `pos`/`vel`/`damage`, which is everything this models; whether its firer
/// ever fires AGAIN is an opponent-policy question, deliberately not a
/// physics one (§12.3).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShadowProjectile {
    pub pos: ae::Vec2,
    pub vel: ae::Vec2,
    pub damage: i32,
}

/// Everything the rollout knows. Built ONLY from a [`Perceived`] view.
#[derive(Clone, Debug, PartialEq)]
pub struct ShadowState {
    pub me: ShadowFighter,
    pub foe: ShadowFighter,
    /// Hostile projectiles in flight. They threaten ME — the view resolves
    /// `hostile_to_self` for the viewer, and whether they'd also hurt the foe
    /// is faction knowledge the view does not claim.
    pub projectiles: Vec<ShadowProjectile>,
    pub stage: StageView,
    pub gravity_down: ae::Vec2,
}

/// What one shadow step reported.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShadowEvent {
    /// `on_me` = the hit landed on me (bad); otherwise it landed on the foe.
    Hit {
        on_me: bool,
        damage: i32,
    },
    Ko {
        of_me: bool,
    },
}

/// Per-tick intent for one shadow fighter. Applied only from `Idle` (a
/// committed or reeling body has no authority — the same statement the real
/// sim makes with its control locks).
#[derive(Clone, Debug, PartialEq)]
pub enum ShadowIntent {
    Hold,
    /// Grounded drive along local `side`, −1..=1.
    Drive {
        lateral: f32,
    },
    Jump,
    /// A dash: velocity SET along local `side`, grounded or not, for
    /// [`ShadowTuning::dash_time`]. `elapsed` is how long it has been held, so a
    /// sustained intent models one dash rather than an endless rocket.
    Dash {
        lateral: f32,
    },
    /// Recover toward the stage: an air jump if the budget allows, drift if not.
    /// `toward_home` is −1..=1 along local `side`.
    Recover {
        toward_home: f32,
    },
    /// Begin the generic assumed foe attack (predicted `Choice::Attack`).
    StartAttack,
    /// Raise the guard (predicted `Choice::Shield`).
    Shield,
    /// Begin a move with KNOWN frame data (the rollout candidate).
    StartMove {
        frames: MoveFrameData,
    },
}

fn fighter_from_self(view: &SelfView, gravity_down: ae::Vec2) -> ShadowFighter {
    let down = gravity_down.normalize_or_zero();
    ShadowFighter {
        pos: view.pos,
        vel: view.vel,
        facing: if view.facing < 0.0 { -1.0 } else { 1.0 },
        half_extent: view.half_extent,
        on_ground: view.on_ground,
        ground_level: view.on_ground.then(|| view.pos.dot(down)),
        air_jumps: view.air_jumps_left,
        dash_remaining: 0.0,
        // Filled by `ShadowState::from_perceived`, which is the only place with
        // the terrain to fill it from.
        ground_span: None,
        phase: shadow_phase_from_view(view.phase, view.phase_remaining),
        damage: view.damage_taken,
        health_max: view.health_max,
        shield_raised: matches!(view.phase, BodyPhase::Shielding),
        invulnerable: view.invulnerable,
        koed: false,
    }
}

fn fighter_from_actor(actor: &PerceivedActor, gravity_down: ae::Vec2) -> ShadowFighter {
    let down = gravity_down.normalize_or_zero();
    ShadowFighter {
        pos: actor.pos,
        vel: actor.vel,
        facing: if actor.facing < 0.0 { -1.0 } else { 1.0 },
        half_extent: actor.half_extent,
        on_ground: actor.on_ground,
        ground_level: actor.on_ground.then(|| actor.pos.dot(down)),
        // A foe's remaining jumps are not observable — the body does not show
        // them. Assuming the worst (it can still recover) is the conservative
        // reading: the rollout does not get to plan around an opponent it has
        // decided is already dead.
        air_jumps: 1,
        dash_remaining: 0.0,
        // A perceived FOE's floor is not known — the view carries the actor, not
        // what it stands on. `None` keeps the old infinite plane for it, which is
        // the conservative reading: the rollout does not get to assume an
        // opponent will fall off.
        ground_span: None,
        phase: shadow_phase_from_view(actor.phase, actor.phase_remaining),
        damage: actor.damage_taken,
        health_max: actor.health_max,
        shield_raised: actor.shield_raised,
        invulnerable: actor.invulnerable,
        koed: false,
    }
}

fn shadow_phase_from_view(phase: BodyPhase, remaining: f32) -> ShadowPhase {
    match phase {
        BodyPhase::Neutral | BodyPhase::Shielding => ShadowPhase::Idle,
        BodyPhase::Hitstun => ShadowPhase::Hitstun {
            remaining: remaining.max(0.0),
        },
        BodyPhase::AttackStartup | BodyPhase::AttackActive | BodyPhase::AttackRecovery => {
            ShadowPhase::Committed {
                phase,
                remaining: remaining.max(0.0),
                landed: false,
            }
        }
    }
}

impl ShadowState {
    /// Build the imagination from a view. `None` when the view holds no
    /// hostile — there is nobody to out-read.
    ///
    /// This constructor is the module's no-cheat proof: its world input is a
    /// [`Perceived`], which only `DelayedPerception::perceive` mints, so every
    /// fact below arrived through the delay buffer like everything else the
    /// brain knows.
    pub fn from_perceived(view: Perceived<'_>) -> Option<Self> {
        let foe = view.nearest_hostile()?;
        let gravity_down = view.self_view.gravity_down;
        // The one place with the terrain to answer "how far does my floor
        // reach", projected into the gravity frame the shadow integrates in.
        let side = ae::AccelerationFrame::new(gravity_down).side;
        let ground_span = view.supporting_floor().map(|floor| {
            let a = floor.min.dot(side);
            let b = floor.max.dot(side);
            (a.min(b), a.max(b))
        });
        let mut me = fighter_from_self(&view.self_view, gravity_down);
        me.ground_span = ground_span;
        Some(Self {
            me,
            foe: fighter_from_actor(foe, gravity_down),
            projectiles: view
                .projectiles
                .iter()
                .filter(|p| p.hostile_to_self)
                .map(|p| ShadowProjectile {
                    pos: p.pos,
                    vel: p.vel,
                    damage: p.damage,
                })
                .collect(),
            stage: view.stage,
            gravity_down,
        })
    }
}

/// Advance the imagination one tick. Order (§12.3): phase clocks, intents,
/// integration, hit resolution, projectiles, KO. Pure; the same inputs step
/// to the same state on every machine and every resimulation.
pub fn shadow_step(
    s: &mut ShadowState,
    dt: f32,
    my_intent: &ShadowIntent,
    foe_intent: &ShadowIntent,
    tuning: &ShadowTuning,
) -> Vec<ShadowEvent> {
    let mut events = Vec::new();
    let down = s.gravity_down.normalize_or_zero();
    let frame = ae::AccelerationFrame::new(s.gravity_down);

    // 1 — phase clocks.
    advance_phase(&mut s.me, dt, tuning);
    advance_phase(&mut s.foe, dt, tuning);

    // 2 — intents (only an Idle body has authority), then integration.
    let toward_foe = (s.foe.pos - s.me.pos).dot(frame.side).signum();
    apply_intent(
        &mut s.me, my_intent, toward_foe, frame.side, down, tuning, dt,
    );
    let toward_me = -toward_foe;
    apply_intent(
        &mut s.foe, foe_intent, toward_me, frame.side, down, tuning, dt,
    );
    integrate(&mut s.me, dt, down, tuning);
    integrate(&mut s.foe, dt, down, tuning);

    // 3 — hit resolution, my known move first (deterministic order; a
    // same-tick trade resolves both, exactly because the order is fixed).
    if let Some(damage) = my_move_lands(&s.me, &s.foe, frame.side, down) {
        let kb = my_knockback(&s.me, &s.foe);
        strike(&mut s.foe, damage, kb.as_ref(), s.gravity_down, tuning);
        mark_landed(&mut s.me.phase);
        events.push(ShadowEvent::Hit {
            on_me: false,
            damage,
        });
    }
    if foe_attack_lands(&s.foe, &s.me, frame.side, down, tuning) {
        let kb = HitKnockback {
            dir: s.foe.facing,
            magnitude: HitKnockbackMagnitude::FeelScale(1.0),
            source_pos: s.foe.pos,
            impact_pos: s.me.pos,
            launch_dir: None,
        };
        strike(
            &mut s.me,
            tuning.assumed_foe_damage,
            Some(&kb),
            s.gravity_down,
            tuning,
        );
        mark_landed(&mut s.foe.phase);
        events.push(ShadowEvent::Hit {
            on_me: true,
            damage: tuning.assumed_foe_damage,
        });
    }

    // 4 — projectiles (they threaten me; see `ShadowState::projectiles`).
    let me = &mut s.me;
    s.projectiles.retain_mut(|p| {
        p.pos += p.vel * dt;
        let delta = me.pos - p.pos;
        let overlaps = delta.dot(frame.side).abs() <= me.half_extent.x
            && delta.dot(down).abs() <= me.half_extent.y;
        if overlaps && !me.invulnerable && !me.koed {
            if !(me.shield_raised && me.on_ground) {
                me.damage += p.damage;
                events.push(ShadowEvent::Hit {
                    on_me: true,
                    damage: p.damage,
                });
            }
            return false;
        }
        true
    });

    // 5 — KO: **past the point of return**, which is leaving the stage envelope
    // at all.
    //
    // ⚠ this used to additionally require `Hitstun`, on the reading "offstage
    // AND with no authority to do anything about it". That reading conflates two
    // different situations, and the difference is the whole of self-preservation:
    //
    //   * **offstage and reeling** — airborne past the floor's edge, still inside
    //     the envelope. Dangerous, recoverable, and NOT a KO. `distance_to_edge`
    //     is what scores this.
    //   * **past the point of return** — outside the envelope. In a room that is
    //     the wall; on a platform stage the envelope IS the blast zone, and the
    //     match rules delete you there whether you were launched or simply
    //     strolled off. Requiring hitstun made a shadow body's own walk-off free,
    //     so no rollout could ever price it.
    for (fighter, of_me) in [(&mut s.me, true), (&mut s.foe, false)] {
        if !fighter.koed && s.stage.is_known() && s.stage.offstage(fighter.pos) {
            fighter.koed = true;
            events.push(ShadowEvent::Ko { of_me });
        }
    }

    events
}

fn advance_phase(f: &mut ShadowFighter, dt: f32, tuning: &ShadowTuning) {
    if f.koed {
        return;
    }
    f.phase = match std::mem::replace(&mut f.phase, ShadowPhase::Idle) {
        ShadowPhase::Idle => ShadowPhase::Idle,
        ShadowPhase::Move { frames, t, landed } => {
            let t = t + dt;
            if t >= frames.total_s {
                ShadowPhase::Idle
            } else {
                ShadowPhase::Move { frames, t, landed }
            }
        }
        ShadowPhase::Committed {
            phase,
            remaining,
            landed,
        } => {
            let remaining = remaining - dt;
            if remaining > 0.0 {
                ShadowPhase::Committed {
                    phase,
                    remaining,
                    landed,
                }
            } else {
                match phase {
                    // The commitment ladder the view names: startup opens
                    // into active; everything else releases to Idle. The
                    // foe's recovery length is unknown, so ending it early is
                    // the conservative direction — the model UNDERSTATES how
                    // long they stay punishable rather than overstating it.
                    BodyPhase::AttackStartup => ShadowPhase::Committed {
                        phase: BodyPhase::AttackActive,
                        remaining: tuning.assumed_foe_active_s,
                        landed,
                    },
                    _ => ShadowPhase::Idle,
                }
            }
        }
        ShadowPhase::Hitstun { remaining } => {
            let remaining = remaining - dt;
            if remaining > 0.0 {
                ShadowPhase::Hitstun { remaining }
            } else {
                ShadowPhase::Idle
            }
        }
    };
}

fn apply_intent(
    f: &mut ShadowFighter,
    intent: &ShadowIntent,
    toward_opponent: f32,
    side: ae::Vec2,
    down: ae::Vec2,
    tuning: &ShadowTuning,
    // The step's timestep. Coasting is a RATE now, so an intent that sheds
    // speed needs to know how much time it is shedding it over.
    dt: f32,
) {
    if f.koed || !matches!(f.phase, ShadowPhase::Idle) {
        return;
    }
    match intent {
        // A grounded body that is not driving STOPS — platformer ground
        // friction is strong, and this is also what ends a lunge: the move's
        // start impulse carries exactly for the move's duration (intents are
        // ignored while committed), then the first idle Hold eats it. An
        // airborne body is ballistic and keeps everything.
        ShadowIntent::Hold => {
            // ⭐ **a RATE, not an instant stop** (2026-08-06). Both were wrong
            // and in opposite directions: grounded stopped dead where the game
            // coasts ~38px out of a dash, airborne kept everything where the
            // game bleeds 650 px/s². The first is the dangerous one — an
            // under-predicted stopping distance is a veto approving a dash that
            // really leaves the stage.
            let decel = if f.on_ground {
                tuning.ground_coast_decel
            } else {
                tuning.air_coast_decel
            };
            let along = f.vel.dot(side);
            let shed = (decel * dt).min(along.abs());
            f.vel -= side * shed * along.signum();
        }
        ShadowIntent::Drive { lateral } => {
            let lateral = lateral.clamp(-1.0, 1.0);
            // **AIR CONTROL IS NOT ZERO, AND ASSUMING IT WAS HID THE COMMONEST
            // DEATH IN THE GAME.** This branch was `if f.on_ground { ... }`, so
            // a shadow body that jumped went straight up and landed exactly
            // where it took off. `ladder_probe` traced the real thing: hold
            // right, walk to the right ledge, jump, and DRIFT right at up to
            // 310 px/s — off the stage, while the rollout scored a jump in place
            // on solid ground and vetoed nothing.
            //
            // Airborne lateral authority is modelled at full ground speed
            // because that is the floor of what was measured, not because the
            // two are the same number. The vertical component is preserved: a
            // drive does not cancel a fall.
            f.vel = side * (lateral * tuning.ground_speed) + down * f.vel.dot(down);
            if lateral.abs() > 1e-3 {
                f.facing = lateral.signum();
            }
        }
        ShadowIntent::Jump => {
            // Grounded jumps are free; airborne ones SPEND the budget, exactly
            // as the body's do. A shadow that jumped only from the ground made
            // every air jump a no-op — the line went nowhere, and going nowhere
            // scores as safe, which is the most dangerous thing a rollout can
            // report about a body over a pit.
            if f.on_ground {
                f.vel -= down * tuning.jump_speed;
                f.on_ground = false;
            } else if f.air_jumps > 0 {
                f.air_jumps -= 1;
                f.vel = f.vel - down * (f.vel.dot(down) + tuning.jump_speed);
            }
        }
        // **RECOVERY IS AN AIRBORNE JUMP PLUS DRIFT, AND IT IS BUDGETED.** A
        // grounded body recovering is just a jump toward home; an airborne one
        // spends an air jump, and when the budget is gone it has drift and
        // nothing else. Modelling it as an unlimited hover would make the
        // rollout certify every recovery, which is worse than not modelling it.
        ShadowIntent::Dash { lateral } => {
            // The dash ends when its clock does; after that the body coasts on
            // whatever the impulse gave it, which is the part that carries it
            // past a ledge.
            if f.dash_remaining > 0.0 {
                let lateral = lateral.clamp(-1.0, 1.0);
                f.vel = side * (lateral * tuning.dash_speed) + down * f.vel.dot(down);
                if lateral.abs() > 1e-3 {
                    f.facing = lateral.signum();
                }
            }
        }
        ShadowIntent::Recover { toward_home } => {
            let lateral = toward_home.clamp(-1.0, 1.0);
            if f.on_ground {
                f.vel = side * (lateral * tuning.ground_speed) - down * tuning.jump_speed;
                f.on_ground = false;
            } else if f.air_jumps > 0 {
                f.air_jumps -= 1;
                // The air jump REPLACES downward velocity rather than adding to
                // it, which is what makes a recovery survivable at all.
                f.vel = side * (lateral * tuning.ground_speed) - down * tuning.jump_speed;
            } else {
                // Out of jumps: drift only. Lateral authority in the air is a
                // fraction of the ground's, and the fall continues.
                let drift = side * (lateral * tuning.ground_speed * AIR_DRIFT_FRACTION);
                f.vel = drift + down * f.vel.dot(down);
            }
            if lateral.abs() > 1e-3 {
                f.facing = lateral.signum();
            }
        }
        ShadowIntent::StartAttack => {
            f.facing = if toward_opponent < 0.0 { -1.0 } else { 1.0 };
            f.shield_raised = false;
            f.phase = ShadowPhase::Committed {
                phase: BodyPhase::AttackStartup,
                remaining: tuning.assumed_foe_startup_s,
                landed: false,
            };
        }
        ShadowIntent::Shield => {
            if f.on_ground {
                f.shield_raised = true;
                f.vel = ae::Vec2::ZERO;
            }
        }
        ShadowIntent::StartMove { frames } => {
            f.facing = if toward_opponent < 0.0 { -1.0 } else { 1.0 };
            f.shield_raised = false;
            // The move's authored self-motion, applied EXACTLY as the real
            // trigger seam does (`trigger_moveset_moves`): body-local,
            // x mirrored by facing, rotated through the gravity frame, ADDED
            // to velocity. A lunge's effective range is reach plus this —
            // the fidelity instrument's first finding.
            let (ix, iy) = frames.start_impulse;
            if ix != 0.0 || iy != 0.0 {
                let frame = ae::AccelerationFrame::new(down);
                let world = frame.to_world(ae::Vec2::new(ix * f.facing, iy));
                f.vel += world;
                if world.dot(down) < -1e-3 {
                    f.on_ground = false;
                }
            }
            f.phase = ShadowPhase::Move {
                frames: frames.clone(),
                t: 0.0,
                landed: false,
            };
        }
    }
}

fn integrate(f: &mut ShadowFighter, dt: f32, down: ae::Vec2, tuning: &ShadowTuning) {
    if f.koed {
        return;
    }
    f.dash_remaining = (f.dash_remaining - dt).max(0.0);
    if !f.on_ground {
        f.vel += down * (tuning.gravity * dt);
    }
    f.pos += f.vel * dt;
    // v1's whole terrain model: a body that was STANDING re-lands at the
    // height it stood at. A body that started airborne has no known floor
    // and keeps falling — which is exactly the doubt an offstage rollout
    // should carry.
    // **WALKING OFF THE END OF THE FLOOR.** A grounded body whose lateral
    // position has left its supporting solid is no longer supported — it falls,
    // and from the next step gravity has it. Without this a shadow body strolled
    // off a platform and kept walking at the same height, which is why a rollout
    // could not see the single commonest way a fighter dies.
    let frame = ae::AccelerationFrame::new(down);
    let lateral = f.pos.dot(frame.side);
    let supported = f
        .ground_span
        .map(|(min, max)| lateral >= min && lateral <= max)
        .unwrap_or(true);
    if f.on_ground && !supported {
        f.on_ground = false;
    }
    if let Some(level) = f.ground_level {
        let depth = f.pos.dot(down);
        if !f.on_ground && supported && depth >= level && f.vel.dot(down) > 0.0 {
            f.pos -= down * (depth - level);
            f.vel -= down * f.vel.dot(down);
            f.on_ground = true;
        }
    }
}

/// Does my current move land THIS tick? `Some(damage)` when an Active span is
/// open, the foe is inside reach along my facing, our boxes overlap
/// vertically, and the move has not already landed (single-hit v1).
fn my_move_lands(
    me: &ShadowFighter,
    foe: &ShadowFighter,
    side: ae::Vec2,
    down: ae::Vec2,
) -> Option<i32> {
    if me.koed || foe.koed || foe.invulnerable {
        return None;
    }
    let ShadowPhase::Move {
        ref frames,
        t,
        landed,
    } = me.phase
    else {
        return None;
    };
    if landed || !frames.active_spans.iter().any(|&(s, e)| s <= t && t < e) {
        return None;
    }
    let delta = foe.pos - me.pos;
    let lateral = delta.dot(side) * me.facing;
    let vertical = delta.dot(down).abs();
    let in_reach = lateral >= -foe.half_extent.x && lateral <= frames.reach + foe.half_extent.x;
    let overlapping = vertical <= me.half_extent.y + foe.half_extent.y;
    if !in_reach || !overlapping {
        return None;
    }
    if foe.shield_raised && foe.on_ground {
        // Blocked: the swing is spent (mark via damage 0 — caller still marks
        // landed), no damage, no launch.
        return Some(0);
    }
    Some(frames.max_damage)
}

fn my_knockback(me: &ShadowFighter, foe: &ShadowFighter) -> Option<HitKnockback> {
    let ShadowPhase::Move { ref frames, .. } = me.phase else {
        return None;
    };
    (frames.max_knockback > 0.0).then(|| HitKnockback {
        dir: me.facing,
        magnitude: HitKnockbackMagnitude::LaunchSpeed(frames.max_knockback),
        source_pos: me.pos,
        impact_pos: foe.pos,
        launch_dir: None,
    })
}

/// Does the foe's committed attack land on me this tick? Their reach is the
/// model assumption `assumed_foe_reach` — the view names their phase and
/// clock, not their move.
fn foe_attack_lands(
    foe: &ShadowFighter,
    me: &ShadowFighter,
    side: ae::Vec2,
    down: ae::Vec2,
    tuning: &ShadowTuning,
) -> bool {
    if foe.koed || me.koed || me.invulnerable {
        return false;
    }
    let ShadowPhase::Committed {
        phase: BodyPhase::AttackActive,
        landed: false,
        ..
    } = foe.phase
    else {
        return false;
    };
    if me.shield_raised && me.on_ground {
        return false;
    }
    let delta = me.pos - foe.pos;
    let lateral = delta.dot(side) * foe.facing;
    let vertical = delta.dot(down).abs();
    lateral >= -me.half_extent.x
        && lateral <= tuning.assumed_foe_reach + me.half_extent.x
        && vertical <= foe.half_extent.y + me.half_extent.y
}

fn mark_landed(phase: &mut ShadowPhase) {
    match phase {
        ShadowPhase::Move { landed, .. } | ShadowPhase::Committed { landed, .. } => {
            *landed = true;
        }
        _ => {}
    }
}

/// Apply a landed hit to `victim` with the REAL response kernel — the same
/// launch and the same hitstun the authoritative victim path arms.
fn strike(
    victim: &mut ShadowFighter,
    damage: i32,
    kb: Option<&HitKnockback>,
    gravity_down: ae::Vec2,
    tuning: &ShadowTuning,
) {
    if damage <= 0 && kb.is_none() {
        return;
    }
    victim.damage += damage.max(0);
    victim.vel = hit_response::knockback_velocity(
        victim.pos,
        victim.facing,
        gravity_down,
        kb,
        ae::Vec2::ZERO,
        &tuning.response,
    );
    victim.phase = ShadowPhase::Hitstun {
        remaining: hit_response::hitstun_duration(kb, &tuning.response),
    };
    victim.shield_raised = false;
    if kb.is_some() {
        victim.on_ground = false;
    }
}

// ---------------------------------------------------------------------------
// FB6c — the predicted opponent (§12.2 D3). Deterministic; no RNG, ever.
// ---------------------------------------------------------------------------

/// What the foe is predicted to do THIS tick.
///
/// * Mid-move: nothing — a commitment completes on its own clock, and
///   `shadow_step` ignores intents from non-Idle bodies anyway.
/// * A genuine read (`read_weight > 0`, and the modal choice strictly beats
///   the uniform prior): the modal [`Choice`](super::habit::Choice), mapped
///   to an intent.
/// * Otherwise: inertia. Ignorance predicts a body keeps doing what it is
///   doing, not that it starts doing something new.
pub fn predicted_foe_intent(
    state: &ShadowState,
    situation: Situation,
    habits: &HabitModel,
    read_weight: f32,
    tuning: &ShadowTuning,
) -> ShadowIntent {
    use super::habit::Choice;
    if !matches!(state.foe.phase, ShadowPhase::Idle) {
        return ShadowIntent::Hold;
    }
    let frame = ae::AccelerationFrame::new(state.gravity_down);
    // The foe's "toward my opponent" sense, resolved against live geometry so
    // "approach" keeps meaning approach after a crossup. `Drive.lateral` is
    // always world-frame `side` units.
    let toward_me = (state.me.pos - state.foe.pos).dot(frame.side).signum();
    if read_weight > 0.0 {
        if let Some((choice, frequency)) = habits.read(situation) {
            let uniform = 1.0 / Choice::ALL.len() as f32;
            if frequency > uniform {
                return match choice {
                    Choice::Approach => ShadowIntent::Drive { lateral: toward_me },
                    Choice::Retreat => ShadowIntent::Drive {
                        lateral: -toward_me,
                    },
                    Choice::Jump => ShadowIntent::Jump,
                    Choice::Attack => ShadowIntent::StartAttack,
                    Choice::Shield => ShadowIntent::Shield,
                    Choice::Wait => ShadowIntent::Hold,
                };
            }
        }
    }
    // Inertia: a grounded mover keeps its lateral drive; an airborne body is
    // ballistic and Hold already models that.
    if state.foe.on_ground {
        let lateral = state.foe.vel.dot(frame.side) / tuning.ground_speed.max(1e-3);
        if lateral.abs() > 0.05 {
            return ShadowIntent::Drive {
                lateral: lateral.clamp(-1.0, 1.0),
            };
        }
    }
    ShadowIntent::Hold
}

// ---------------------------------------------------------------------------
// FB6d — refine_by_rollout (§12.4).
// ---------------------------------------------------------------------------

/// L3's verdict: which of L2's top-k attacks the rollouts prefer, and by how
/// much over doing nothing.
#[derive(Clone, Debug, PartialEq)]
pub struct RefinedChoice {
    /// **When EVERY offered verb is fatal, the one that dies latest.**
    ///
    /// ⚠ a veto that can empty the list needs this, and finding out cost a
    /// measurement: with `Recover` finally modelled the rollout could strike it,
    /// and at the time `Recover` was the ONLY verb offered in
    /// `Situation::Recovery`. So an airborne body's list emptied, the halt fired,
    /// and a doomed recovery was replaced by standing still — which for a body in
    /// the air is not caution, it is the same death with the last option thrown
    /// away. Survival at level 9 fell 40.2 s → 9.2 s the moment the model got
    /// good enough to condemn the verb.
    ///
    /// ⛔ **"the only verb" is no longer true, and the stale sentence is worth
    /// more corrected than deleted** — it sent one investigation down a path the
    /// repo had already closed (2026-08-03). Measured, `Situation::Recovery` now
    /// offers:
    /// | jump left, can blink | `Recover@1.00` `Blink@0.90` `Jump@0.50` |
    /// | no jump, can blink   | `Recover@1.00` `Blink@0.90` |
    /// | no jump, no blink    | `Recover@1.00` |
    /// so the list only empties for a body with neither a jump nor a blink. ⚠ and
    /// note `Recover` outranks `Blink` at every budget, while `Jump` correctly
    /// disappears when the budget is gone — `Recover`'s emit presses jump without
    /// checking it. That is NOT a live defect: the shadow models the empty-budget
    /// case honestly (drift only, the fall continues), so L3 can strike a doomed
    /// `Recover` and leave `Blink` unjudged and therefore still available. See
    /// [`AIR_DRIFT_FRACTION`] — out-of-jumps drift was measured OFF the path to
    /// the ladder deaths on 2026-07-31.
    ///
    /// Standing still is only a fallback where standing still is survivable. On
    /// the ground it is; in the air it never is.
    pub least_bad_movement: Option<crate::brain::fighter::options::MovementVerb>,
    /// The preferred attack — **`None` when L2 offered none**, which is the
    /// `Recovery` situation ("a body past the blastzone has exactly one
    /// problem"). A consumer reads this for the attack and
    /// [`Self::suicidal_movement`] for the veto; the two are independent, and
    /// conflating them is what made the veto skip the body that needed it.
    ///
    /// ⛔ **this was a `String` whose EMPTY value meant "no attack", and the
    /// sentinel cost the demo its difficulty curve** (traced 2026-07-31). The
    /// consumer read `refined.move_id.clone()` into an `Option`, so the moment a
    /// rollout ran at all the answer was `Some("")` — an attack request naming
    /// no move — and the fighter armed a press EVERY decision, including in
    /// `Recovery` where L2 deliberately offers none. Each press lunges the body
    /// forward, so a level-9 fighter offstage and holding LEFT was carried right
    /// at 700 px/s by its own attacks while the trace showed it asking to come
    /// back. Levels 1–5, which run no rollout, took the `or_else` branch and
    /// were fine — which is exactly the shape of the A/B that said the rollout
    /// made things worse.
    pub move_id: Option<String>,
    /// **The press that reaches [`Self::move_id`]**, carried beside it so the
    /// refinement's winner can be EXECUTED as the move it won with. `None`
    /// exactly when `move_id` is.
    pub binding: Option<super::options::AttackBinding>,
    /// **Movement lines the rollout found SUICIDAL**, by L2 verb.
    ///
    /// Empty when the profile runs no rollouts or nothing self-KO'd. A verb in
    /// here walked or jumped this body out of the world within the horizon, and
    /// L2's score for it is not the question — no attack is worth a stock.
    ///
    /// ⚠ this is why L3 exists at all on a stage with edges. `ladder_probe`
    /// measured identical self-KO counts at every ladder rung, INCLUDING across
    /// the `rollout_depth: 0 -> 12` boundary, because the rollout only ever
    /// refined attacks — and a self-KO is a movement defect. A rollout that
    /// cannot see the thing killing you cannot earn its depth.
    ///
    /// The two reasons it was empty for the COMMON case, both found by measuring
    /// rather than reading, and both closed 2026-07-31:
    ///
    /// * `ShadowState` carried no terrain, so a body driven past a platform's
    ///   edge did not fall — it walked on at the same height forever. The floor
    ///   now has an extent ([`ShadowFighter::ground_span`]).
    /// * the shadow's KO fired only `matches!(phase, Hitstun) && offstage`,
    ///   which made a self-inflicted exit free. It now fires on leaving the
    ///   envelope at all; "offstage and reeling" is the recoverable case and it
    ///   is INSIDE the envelope, which is what `distance_to_edge` scores.
    ///
    pub suicidal_movement: Vec<crate::brain::fighter::options::MovementVerb>,
    /// Rollout score of the chosen line MINUS the do-nothing baseline, in the
    /// same units as L2's dot product. Positive means the rollouts saw the
    /// move buy something real.
    pub value_over_baseline: f32,
}

/// Roll one line `depth` ticks forward and score the end state. `candidate`
/// is `None` for the do-nothing baseline (§12.4 — the baseline is what makes
/// the score a DELTA, so a rollout cannot credit a move for damage the
/// opponent was going to take anyway).
fn rollout_value(
    start: &ShadowState,
    candidate: Option<&MoveFrameData>,
    // What THIS body does on every step of the line.
    //
    // ⚠ added 2026-07-31. It was hardcoded `Hold`, which is why a rollout could
    // only ever answer "which attack" — every line moved the foe and stood
    // still. `ladder_probe` measured the consequence: identical self-KO counts
    // at every rung, across the `rollout_depth: 0 -> 12` boundary, because the
    // defect being measured was MOVEMENT and movement never entered a rollout.
    sustained: &ShadowIntent,
    depth: u32,
    dt: f32,
    situation: Situation,
    habits: &HabitModel,
    profile: &FighterBrainProfile,
    tuning: &ShadowTuning,
) -> f32 {
    let mut s = start.clone();
    if let Some(frames) = candidate {
        let frame = ae::AccelerationFrame::new(s.gravity_down);
        let toward = (s.foe.pos - s.me.pos).dot(frame.side).signum();
        apply_intent(
            &mut s.me,
            &ShadowIntent::StartMove {
                frames: frames.clone(),
            },
            toward,
            frame.side,
            s.gravity_down.normalize_or_zero(),
            &tuning,
            dt,
        );
    }
    let mut ko_me = false;
    let mut ko_foe = false;
    for _ in 0..depth {
        let foe_intent = predicted_foe_intent(&s, situation, habits, profile.read_weight, tuning);
        for event in shadow_step(&mut s, dt, sustained, &foe_intent, tuning) {
            match event {
                ShadowEvent::Ko { of_me: true } => ko_me = true,
                ShadowEvent::Ko { of_me: false } => ko_foe = true,
                ShadowEvent::Hit { .. } => {}
            }
        }
    }
    score_line(start, &s, ko_me, ko_foe, profile)
}

/// Normalized damage: the meter fraction where the max is known, a
/// smash-percent-ish `damage / 100` where it is not, so an unknown-max foe
/// still prices hits instead of reading as unhittable.
fn normalized_damage(f: &ShadowFighter) -> f32 {
    if f.health_max > 0 {
        f.damage as f32 / f.health_max as f32
    } else {
        f.damage as f32 / 100.0
    }
}

/// §12.4: the terminal state scored in L2's OWN weight vocabulary — damage
/// deltas and KOs priced by `kill_potential`, terminal stage position priced
/// by `stage_risk`. One vocabulary, two horizons.
fn score_line(
    start: &ShadowState,
    end: &ShadowState,
    ko_me: bool,
    ko_foe: bool,
    profile: &FighterBrainProfile,
) -> f32 {
    let w = &profile.utility_weights;
    let damage_swing = (normalized_damage(&end.foe) - normalized_damage(&start.foe))
        - (normalized_damage(&end.me) - normalized_damage(&start.me));
    let ko_swing = (ko_foe as i32 as f32) - (ko_me as i32 as f32);
    let stage_risk = {
        let half_stage = (end.stage.bounds.max - end.stage.bounds.min).length() * 0.5;
        if half_stage <= 0.0 {
            1.0
        } else {
            (1.0 - end.stage.distance_to_edge(end.me.pos) / half_stage).clamp(0.0, 1.0)
        }
    };
    w.kill_potential * (damage_swing + ko_swing) + w.stage_risk * stage_risk
}

/// **L3.** Re-rank L2's top `rollout_k` attacks by simulated outcome.
///
/// Returns `None` — "use L2's order unchanged" — when the profile disables
/// rollouts (`rollout_depth == 0` or `rollout_k == 0`, the graceful
/// degradation §1 promises) or when the view holds no hostile. An empty attack
/// list is NOT a reason to return `None` — see [`RefinedChoice::move_id`].
///
/// The work is EXACTLY `min(k, attacks) + 1` attack lines of `rollout_depth`
/// steps, plus one movement line of `rollout_depth ×
/// `[`MOVEMENT_HORIZON_MULTIPLE`] steps per modelled verb; nothing about the
/// machine, the load, or the clock can change what this function returns.
pub fn refine_by_rollout(
    view: Perceived<'_>,
    situation: Situation,
    options: &OptionSet,
    habits: &HabitModel,
    profile: &FighterBrainProfile,
    tuning: &ShadowTuning,
    tick_hz: f32,
    commit_ticks: u32,
) -> Option<RefinedChoice> {
    // ⚠ `attacks.is_empty()` used to short-circuit here, which silently made the
    // MOVEMENT veto conditional on having something to swing. A fighter with no
    // attack in range is exactly the fighter that is walking somewhere, so the
    // one moment the veto matters most was the one moment it never ran.
    if !profile.uses_rollouts() {
        return None;
    }
    let start = ShadowState::from_perceived(view)?;
    let dt = 1.0 / tick_hz.max(1.0);
    let baseline = rollout_value(
        &start,
        None,
        &ShadowIntent::Hold,
        profile.rollout_depth,
        dt,
        situation,
        habits,
        profile,
        tuning,
    );
    let mut best: Option<(usize, f32)> = None;
    for (index, option) in options
        .attacks
        .iter()
        .take(profile.rollout_k as usize)
        .enumerate()
    {
        let value = rollout_value(
            &start,
            Some(&option.frames),
            &ShadowIntent::Hold,
            profile.rollout_depth,
            dt,
            situation,
            habits,
            profile,
            tuning,
        ) - baseline;
        // Strictly-greater keeps the EARLIER candidate on ties — L2's order,
        // already id-tie-broken, stays the arbiter (ADR 0023: no
        // order-dependent decisions means ties fall to a stated order, not to
        // float luck).
        if best.map_or(true, |(_, b)| value > b) {
            best = Some((index, value));
        }
    }
    // **ROLL THE MOVEMENT LINES TOO.** The shadow model already steps a body and
    // already reports `ShadowEvent::Ko { of_me: true }`; nothing was ever asked
    // to walk. Each verb L2 offered is rolled as a SUSTAINED intent, and a line
    // that ends with this body out of the world is named — L2 scores where the
    // floor is NOW, and this is the only thing in the brain that knows where the
    // body will BE.
    let horizon = profile.rollout_depth * MOVEMENT_HORIZON_MULTIPLE;
    let mut longest_lived: Option<(crate::brain::fighter::options::MovementVerb, u32)> = None;
    let suicidal_movement = options
        .movement
        .iter()
        .filter_map(|option| {
            let intent = movement_intent(option.verb, &start)?;
            let mut probe = start.clone();
            // **THE SWING THAT COMES WITH THIS DECISION.** (traced 2026-07-31)
            //
            // The same decision that picks a movement verb also arms an attack
            // whenever L2 offers one, and every melee press shoves the body
            // `slash_recoil` BACKWARDS along its facing. That is 110 px/s in this
            // engine, per press, with almost nothing to bleed it off in the air —
            // so the presses ratchet, and a movement line that models the walk
            // and not the swing is modelling the smaller of the two forces.
            //
            // The probe takes the recoil once, up front, because the press is
            // armed by THIS decision. It is deliberately not repeated across the
            // horizon: the line coasts after `commit_ticks` and the brain will
            // re-decide before the next swing lands, so charging every tick would
            // veto every verb from every position — the paralysis this file has
            // already produced once by over-sustaining an intent.
            if !options.attacks.is_empty() {
                let side = ae::AccelerationFrame::new(probe.gravity_down).side;
                probe.me.vel -= side * (probe.me.facing * tuning.slash_recoil);
            }
            // A dash line ARMS the dash clock; the intent itself only steers
            // while the clock runs, so without this the dash would model as
            // doing nothing at all. Armed here rather than inside `apply_intent`
            // because starting a dash is a decision and continuing one is not.
            if matches!(intent, ShadowIntent::Dash { .. }) {
                probe.me.dash_remaining = tuning.dash_time;
            }
            let dt = 1.0 / tick_hz.max(1.0);
            let mut died = false;
            let mut survived = horizon;
            for tick in 0..horizon {
                // **THE VERB IS SUSTAINED ONLY AS LONG AS THE BODY IS COMMITTED
                // TO IT**, and then the line coasts. A brain that re-decides
                // every `commit_ticks` never actually walks for 3.2 s; asking
                // "what if I did" answers a question nobody faces, and answers
                // it fatally for every direction from every position.
                //
                // What the rest of the horizon is for is the CONSEQUENCE — the
                // walk-off already begun, the fall that follows, the body
                // leaving the world. Those need seconds. The input does not.
                let held = if tick < commit_ticks {
                    intent.clone()
                } else {
                    ShadowIntent::Hold
                };
                let foe_intent =
                    predicted_foe_intent(&probe, situation, habits, profile.read_weight, tuning);
                for event in shadow_step(&mut probe, dt, &held, &foe_intent, tuning) {
                    if matches!(event, ShadowEvent::Ko { of_me: true }) {
                        died = true;
                    }
                }
                if died {
                    survived = tick;
                    break;
                }
            }
            if died {
                // **HOW LONG IT LASTS, not merely that it ends.** When every
                // option is fatal the caller still has to pick one, and the
                // longest-lived line is the one that leaves the most room for
                // the world to change — a foe that stops chasing, a launch that
                // was never coming. Ties fall to L2's order, which is already
                // id-tie-broken (ADR 0023).
                if longest_lived.is_none_or(|(_, best)| survived > best) {
                    longest_lived = Some((option.verb, survived));
                }
            }
            died.then_some(option.verb)
        })
        .collect();

    // ⚠ this was `best.map(...)`, which threw the movement veto away whenever
    // there was no attack to name — the same `attacks.is_empty()` blindness a
    // second time, one screen further down. The refined choice is now two
    // independent answers, and a body with nothing to swing still gets the one
    // that keeps it alive.
    Some(RefinedChoice {
        least_bad_movement: longest_lived.map(|(verb, _)| verb),
        move_id: best.map(|(index, _)| options.attacks[index].move_id.clone()),
        binding: best.map(|(index, _)| options.attacks[index].binding),
        value_over_baseline: best.map_or(0.0, |(_, value)| value),
        suicidal_movement,
    })
}

/// **The movement veto rolls further than attack refinement does, and the ratio
/// is the point.** The two questions live on different timescales:
///
/// * *will this attack connect* is a FRAMES question. A shipped `rollout_depth`
///   of 12 is 0.2 s at 60 Hz, which is a startup plus an active span — exactly
///   the right window, and deliberately short because the cost is multiplied by
///   `rollout_k`.
/// * *will walking this way kill me* is a SECONDS question. At 160 px/s a body
///   is two seconds from the edge of a 640 px stage, and 0.2 s of lookahead
///   cannot see a walk-off at all. `ladder_probe` measured this as a depth A/B
///   that moved NOTHING: 7.2 s to first self-KO at `rollout_depth` 0 and 12
///   alike, because both horizons were blind to the thing doing the killing.
///
/// 16× is sized to the second number: at 160 px/s a body needs 2.0 s to walk
/// from the middle of a 640 px stage to its edge, so 12 × 16 = 192 ticks = 3.2 s
/// covers that crossing with room for the fall that follows it.
///
/// ⚠ **and the line is NOT the verb sustained for all of it** — see
/// [`commit_ticks`](refine_by_rollout). Sustaining a walk for 3.2 s is 512 px,
/// which is wider than the stage; every lateral verb would be fatal from every
/// position, the veto would fire on every decision, and the "fighter" would be a
/// body that had reasoned itself into never moving. That is what the first cut
/// of this did, and the survival number went UP, which is exactly how a
/// paralysis reads on a metric that counts staying alive.
///
/// The cost is `modelled_verbs × depth × 16` steps against `rollout_k × depth`
/// for attacks; there are four modelled verbs, and FB6e's bench pin
/// (`the_worst_shipped_budget_is_cheap_enough_to_be_a_non_event`) is what says
/// whether that is still free.
pub const MOVEMENT_HORIZON_MULTIPLE: u32 = 16;

/// How much of a body's ground speed it can steer with while airborne, once its
/// jumps are spent.
///
/// **It is a SAFETY MARGIN, not a physical fact, and the engine's numbers say
/// so.** `ae::AIR_ACCEL` is 3100 px/s² against a shared `MAX_RUN_SPEED` cap of
/// 270 — so a real body reaches the SAME top speed in the air as on the ground,
/// differing only in how long it takes to get there (~0.09 s). The shadow sets
/// velocity instantly and models no acceleration at all, so a fraction below 1
/// approximates the average achieved over that ramp rather than describing a
/// weaker air game.
///
/// The pessimism is still the right direction: a rollout that overestimates
/// drift certifies recoveries that will not happen, and a fighter that dives off
/// the stage on a promise is worse than one that never leaves it.
///
/// ⚠ **measured 2026-07-31: moving it to 1.0 changes `ladder_probe` by NOTHING**
/// — same first self-KO, same survival, at every rung. Whatever is killing this
/// fighter, out-of-jumps drift is not on the path. Do not tune this number
/// hoping to move that measurement.
pub const AIR_DRIFT_FRACTION: f32 = 0.6;

/// The sustained shadow intent a movement verb means.
///
/// `None` for verbs the shadow model does not simulate (blink, shield-as-motion)
/// — an unmodelled verb is not judged, because a rollout that reported every
/// unknown as safe or as fatal would be lying in one direction or the other.
fn movement_intent(
    verb: crate::brain::fighter::options::MovementVerb,
    start: &ShadowState,
) -> Option<ShadowIntent> {
    use crate::brain::fighter::options::MovementVerb;
    let frame = ae::AccelerationFrame::new(start.gravity_down);
    let toward = (start.foe.pos - start.me.pos).dot(frame.side).signum();
    Some(match verb {
        MovementVerb::Approach => ShadowIntent::Drive { lateral: toward },
        MovementVerb::Dash => ShadowIntent::Dash { lateral: toward },
        MovementVerb::Retreat => ShadowIntent::Drive { lateral: -toward },
        MovementVerb::Jump => ShadowIntent::Jump,
        // ⚠ `Recover` was in the unmodelled list, and it is the ONLY verb that
        // can save an airborne body — so the one verb the model could not judge
        // was the one the situation it could not represent depended on. Both
        // halves closed together; neither is worth anything alone.
        MovementVerb::Recover => ShadowIntent::Recover {
            toward_home: ((start.stage.bounds.min.dot(frame.side)
                + start.stage.bounds.max.dot(frame.side))
                * 0.5
                - start.me.pos.dot(frame.side))
            .signum(),
        },
        MovementVerb::Shield | MovementVerb::Blink => return None,
    })
}

#[cfg(test)]
mod tests;
