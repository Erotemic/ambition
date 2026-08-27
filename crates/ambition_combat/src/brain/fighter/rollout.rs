//! FB6 — L3, forward rollouts on a shadow model
//! (`docs/planning/engine/fighter-brain.md` §12).
//!
//! The rollout does NOT run the sim. The sim is authoritative, stateful, and
//! omniscient; an imagination must be cheap, pure, and exactly as ignorant as
//! its owner. [`ShadowState`] is that imagination: its only world input is a
//! [`Perceived`] view, so FB4a's type-level no-cheat enforcement carries over —
//! the rollout physically cannot contain a fact the brain could not see.
//!
//! Determinism is load-bearing, not aspirational. Brains run inside the simulation, and
//! under GGRS a resimulated decision tick must reproduce the original decision bit-for-bit.
//! Execution noise is FB4's business, in the execution layer, with its own snapshot-registered
//! stream.
//!
//! The hit response is the real one. `ae::hit_response` is the SAME kernel `damage_apply`
//! resolves authoritative hits with (§12.3 route 1 — carved to the floor precisely so both callers
//! exist). What the model still approximates is everything else, and §12.3's stated-omissions list
//! is closed on purpose: future projectile fire, one-way platforms as anything but floor, DI,
//! shield break, cancels, charge, and any second hostile are all OUT of v1. The floor the body
//! stands on now has an EXTENT ([`ShadowFighter::ground_span`]); everything above it remains
//! omitted.

use ambition_entity_catalog::MoveFrameData;
use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::hit_response::{self, HitKnockback, HitKnockbackMagnitude};

use ambition_characters::brain::fighter::data::ShadowTuning;
use ambition_characters::brain::fighter::habit::HabitModel;
use ambition_characters::brain::fighter::options::OptionSet;
use ambition_characters::brain::fighter::profile::FighterBrainProfile;
use ambition_characters::brain::fighter::situation::Situation;
use ambition_characters::perception::{BodyPhase, Perceived, PerceivedActor, SelfView, StageView};

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
    /// Mid-air jumps left. Without it the shadow has no air game at all:
    /// every airborne body falls to its death in the imagination, so every verb
    /// comes back fatal, the veto empties the list, and the halt fires on a body
    /// that cannot be helped by standing still. Recovery is the one thing an
    /// airborne fighter can DO, and it was the one thing the model could not
    /// represent.
    pub air_jumps: u8,
    /// This body began the rollout outside the stage envelope.
    ///
    /// It is RECOVERING, so the crossing that would have killed it already
    /// happened before the search began — and pricing it as a fresh KO makes
    /// every option score identically. Cleared the moment it gets back inside,
    /// after which a new crossing is a real death again.
    pub started_offstage: bool,
    /// Seconds left of an in-progress dash. Set when a dash line begins; ticked
    /// down by the integrator.
    pub dash_remaining: f32,
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
    Hit { on_me: bool, damage: i32 },
    Ko { of_me: bool },
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

fn down_unit(gravity_down: ae::Vec2) -> ae::Vec2 {
    gravity_down.normalize_or_zero()
}

fn fighter_from_self(view: &SelfView, gravity_down: ae::Vec2) -> ShadowFighter {
    let down = gravity_down.normalize_or_zero();
    ShadowFighter {
        // Set by `ShadowState::from_perceived`, which is where the stage is known.
        started_offstage: false,
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
        started_offstage: false,
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
        // `floor_below`, not `supporting_floor`. The supporting one answers
        // only while standing, so a recovering body imagined an infinite plane at
        // exactly the moment the platform's extent decides whether it lives. The
        // floor a body would LAND on is the question a rollout is asking.
        let landing = view.floor_below();
        let ground_span = landing.map(|floor| {
            let a = floor.min.dot(side);
            let b = floor.max.dot(side);
            (a.min(b), a.max(b))
        });
        let mut me = fighter_from_self(&view.self_view, gravity_down);
        me.ground_span = ground_span;
        // An airborne body has no `ground_level` of its own (it is not standing
        // on anything), but it does have a surface it is falling TOWARD, and
        // `advance_phase` needs that height to land it. Without it the shadow of
        // every airborne body fell through the world forever, which is a worse
        // model than the one the terrain actually supports.
        if me.ground_level.is_none() {
            me.ground_level = landing.map(|floor| floor.min.dot(down_unit(gravity_down)));
        }
        // See the KO block in `shadow_step`.
        let already_out = |pos: ae::Vec2| view.stage.is_known() && view.stage.offstage(pos);
        me.started_offstage = already_out(me.pos);
        let mut foe_body = fighter_from_actor(foe, gravity_down);
        foe_body.started_offstage = already_out(foe_body.pos);
        Some(Self {
            me,
            foe: foe_body,
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
            // The brain's shadow model of being hit. A gust is not a threat it
            // plans against, so the pulse it imagines is an ordinary one.
            reaction: ambition_platformer2d_core::hit_response::HitReaction::Strike,
            dir: s.foe.facing,
            magnitude: HitKnockbackMagnitude::FeelScale(1.0),
            source_pos: s.foe.pos,
            impact_pos: s.me.pos,
            launch_dir: None,
            follow: None,
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

    // A KO is crossing the stage envelope. A body that starts outside it is recovering,
    // not already dead; its recoverability is scored separately.
    for (fighter, of_me) in [(&mut s.me, true), (&mut s.foe, false)] {
        let outside = s.stage.is_known() && s.stage.offstage(fighter.pos);
        // Keep this geometric: `refine_by_rollout` evaluates whether the body's kit can
        // recover from a committed fall.
        if !fighter.koed && outside && !fighter.started_offstage {
            fighter.koed = true;
            events.push(ShadowEvent::Ko { of_me });
        }
        // Back inside: it recovered, and from here a fresh crossing IS a death.
        if fighter.started_offstage && !outside {
            fighter.started_offstage = false;
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
        // RECOVERY IS AN AIRBORNE JUMP PLUS DRIFT, AND IT IS BUDGETED. A
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

/// Approximate shadow integrator used by the rollout search.
///
/// `RecoveryLens` can evaluate terminal movement with the real movement kernel;
/// this cheaper step still models only a single terrain plane.
/// TODO(compat-remove): replace this integrator with the real kernel once its
/// decision cost is budgeted and `ladder_rig --scenarios` remains green.
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
    // WALKING OFF THE END OF THE FLOOR. A grounded body whose FOOTPRINT has
    // left its supporting solid is no longer supported — it falls, and from the
    // next step gravity has it. Without this a shadow body strolled off a
    // platform and kept walking at the same height, which is why a rollout could
    // not see the single commonest way a fighter dies.
    //
    // the footprint, and it is the kernel's own predicate, not a copy of
    // it. This tested the body's CENTRE against the span, so the shadow let go
    // of a body a half-extent before `surface_supports_body_at_rest` does. That
    // gap is invisible in the middle of a platform and decisive at its lip:
    // `refine_by_rollout` captures `left_the_ground` from this transition and
    // hands the position to the real kernel, which — still finding the footprint
    // on the platform — stood the body straight back up and reprieved a walk-off
    // that was genuinely fatal. `Perceived::supporting_floor` carries the same
    // correction for the same reason.
    let frame = ae::AccelerationFrame::new(down);
    let lateral = f.pos.dot(frame.side);
    // The AABB's own extent along `side`: exact for any frame, cardinal or not.
    let half_width =
        (f.half_extent.x * frame.side.x).abs() + (f.half_extent.y * frame.side.y).abs();
    let supported = f
        .ground_span
        .map(|span| {
            ae::collision_semantics::spans_overlap_for_support(
                (lateral - half_width, lateral + half_width),
                span,
            )
        })
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
    if foe.shield_raised && foe.on_ground && !frames.ignores_guard {
        return Some(0);
    }
    // and a GRAB is the exception, which is the third leg of the triangle
    // showing up in the planner. Attack beats grab, grab beats shield, shield
    // beats attack. While every option was priced as blocked, a shielding
    // opponent made the whole kit worth zero and the CPU picked by tie-break —
    // so the one answer the genre has to a guard was the one it could not see.
    Some(frames.max_damage)
}

fn my_knockback(me: &ShadowFighter, foe: &ShadowFighter) -> Option<HitKnockback> {
    let ShadowPhase::Move { ref frames, .. } = me.phase else {
        return None;
    };
    (frames.max_knockback > 0.0).then(|| HitKnockback {
        // Same shadow model, attacker side.
        reaction: ambition_platformer2d_core::hit_response::HitReaction::Strike,
        dir: me.facing,
        magnitude: HitKnockbackMagnitude::LaunchSpeed(frames.max_knockback),
        source_pos: me.pos,
        impact_pos: foe.pos,
        launch_dir: None,
        follow: None,
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

// Deterministic; no RNG, ever.
// ---------------------------------------------------------------------------

/// What the foe is predicted to do THIS tick.
///
/// * Mid-move: nothing — a commitment completes on its own clock, and
///   `shadow_step` ignores intents from non-Idle bodies anyway.
/// * A genuine read (`read_weight > 0`, and the modal choice strictly beats
///   the uniform prior): the modal [`Choice`](ambition_characters::brain::fighter::habit::Choice), mapped
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
    use ambition_characters::brain::fighter::habit::Choice;
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
    /// When every offered movement verb is fatal, the option with the latest
    /// predicted death. Standing still is only a fallback where it is survivable.
    pub least_bad_movement: Option<ambition_characters::brain::fighter::options::MovementVerb>,
    /// Preferred attack, or `None` when L2 offered none. This is independent
    /// from the movement-suicide veto.
    pub move_id: Option<String>,
    /// The press that reaches [`Self::move_id`], carried beside it so the
    /// refinement's winner can be EXECUTED as the move it won with. `None`
    /// exactly when `move_id` is.
    pub binding: Option<ambition_characters::brain::fighter::options::AttackBinding>,
    /// Movement lines the rollout found SUICIDAL, by L2 verb.
    ///
    /// Empty when the profile runs no rollouts or nothing self-KO'd. A verb in
    /// here walked or jumped this body out of the world within the horizon, and
    /// L2's score for it is not the question — no attack is worth a stock.
    ///
    /// A rollout that cannot see the thing killing you cannot earn its depth.
    ///
    /// * `ShadowState` carried no terrain, so a body driven past a platform's
    ///   edge did not fall — it walked on at the same height forever. The floor
    ///   now has an extent ([`ShadowFighter::ground_span`]).
    /// * the shadow's KO fired only `matches!(phase, Hitstun) && offstage`,
    ///   which made a self-inflicted exit free. It now fires on leaving the
    ///   envelope at all; "offstage and reeling" is the recoverable case and it
    ///   is INSIDE the envelope, which is what `distance_to_edge` scores.
    pub suicidal_movement: Vec<ambition_characters::brain::fighter::options::MovementVerb>,
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
    // .
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

/// L3. Re-rank L2's top `rollout_k` attacks by simulated outcome.
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
///
/// `lens` is the one place a real kernel step enters this module. `None`
/// is the shadow's own verdict, unchanged — which is what a body whose kit the
/// world-in port did not carry, or a view that names no stage, gets. See the
/// movement block below for what `Some` buys and what it costs.
pub fn refine_by_rollout(
    view: Perceived<'_>,
    situation: Situation,
    options: &OptionSet,
    habits: &HabitModel,
    profile: &FighterBrainProfile,
    tuning: &ShadowTuning,
    tick_hz: f32,
    commit_ticks: u32,
    lens: Option<&super::recovery::RecoveryLens>,
) -> Option<RefinedChoice> {
    // A fighter with no attack in range is exactly the fighter that is walking somewhere, so the
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
    // ROLL THE MOVEMENT LINES TOO. The shadow model already steps a body and
    // already reports `ShadowEvent::Ko { of_me: true }`; nothing was ever asked
    // to walk. Each verb L2 offered is rolled as a SUSTAINED intent, and a line
    // that ends with this body out of the world is named — L2 scores where the
    // floor is NOW, and this is the only thing in the brain that knows where the
    // body will BE.
    let horizon = profile.rollout_depth * MOVEMENT_HORIZON_MULTIPLE;
    let mut longest_lived: Option<(
        ambition_characters::brain::fighter::options::MovementVerb,
        u32,
    )> = None;
    let suicidal_movement = options
        .movement
        .iter()
        .filter_map(|option| {
            let intent = movement_intent(option.verb, &start)?;
            let mut probe = start.clone();
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
            // WHERE THIS LINE LAST LEFT THE GROUND. Cleared on every landing,
            // so when the line ends it holds the start of the airborne stretch
            // the line finished in — which is the state a recovery would have to
            // begin from, and the only state worth paying a kernel probe for.
            //
            // deliberately NOT the death state and NOT the line's final state:
            // a body already past the envelope has nothing left to decide, and
            // probing from there would answer "you are dead" for every line.
            //
            // the transition this reads is the KERNEL's, not a second
            // opinion about it — `integrate` decides support with the same
            // `spans_overlap_for_support` the kernel's `perpendicular_overlap`
            // is built from, so a position captured here is one the kernel also
            // calls unsupported. It did not always: a centre-in-span test handed
            // over a body still half-standing on the platform, which the probe
            // stood straight back up. See `integrate`.
            let mut left_the_ground: Option<super::recovery::RecoveryQuery> = None;
            for tick in 0..horizon {
                // THE VERB IS SUSTAINED ONLY AS LONG AS THE BODY IS COMMITTED
                // TO IT, and then the line coasts. A brain that re-decides
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
                if probe.me.on_ground {
                    left_the_ground = None;
                } else if left_the_ground.is_none() {
                    left_the_ground = Some(super::recovery::RecoveryQuery {
                        pos: probe.me.pos,
                        vel: probe.me.vel,
                        air_jumps_left: probe.me.air_jumps,
                    });
                }
                if died {
                    survived = tick;
                    break;
                }
            }
            // ON A FALL, THE REAL KERNEL OVERRULES THE SHADOW — BOTH WAYS.
            //
            // Both errors point the same way — a body that walks off a ledge is scored dead
            // whatever it owns, so near a ledge the veto empties the list and the choice falls
            // to `least_bad_movement`, which is the line that dies LATEST rather than the one
            // that lives.
            //
            // `RecoveryLens` asks the question the shadow cannot: drive THIS
            // body's own kernel, over ITS abilities and ITS movement law, at full
            // recovery effort, from the place this line left the ground. A body
            // with an unspent air jump, a wall to cling to, a ledge to catch or a
            // glide is reprieved; one without any of them is condemned even when
            // the shadow's 3.2 s never got round to killing it.
            //
            // there is NO capability list here and no stage geometry — the
            // rollout hands over a position and a kit and takes back one bit.
            //
            // Cost, stated because it is not free: at most ONE probe per
            // modelled movement verb per decision, and only for a verb whose line
            // left the ground; a line that never does pays nothing. Each probe is
            // three steering efforts capped at
            // `recovery::RECOVERY_PROBE_SECONDS`, and each effort stops the
            // instant the body lands or leaves the world. unmeasured — the
            // bench pin above (`the_worst_shipped_budget_is_cheap_enough_to_be_a_
            // non_event`) rolls no movement lines, so it does not price this.
            let doomed = match (lens, left_the_ground) {
                (Some(lens), Some(at)) => !lens.regains_support(at),
                _ => died,
            };
            if doomed {
                // HOW LONG IT LASTS, not merely that it ends. When every
                // option is fatal the caller still has to pick one, and the
                // longest-lived line is the one that leaves the most room for
                // the world to change — a foe that stops chasing, a launch that
                // was never coming. Ties fall to L2's order, which is already
                // id-tie-broken (ADR 0023).
                if longest_lived.is_none_or(|(_, best)| survived > best) {
                    longest_lived = Some((option.verb, survived));
                }
            }
            doomed.then_some(option.verb)
        })
        .collect();

    // this was `best.map(...)`, which threw the movement veto away whenever
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

/// Movement safety needs a longer horizon than attack refinement: attacks are
/// resolved over frames, while walking off a stage is a seconds-scale event.
/// With the shipped depth of 12, `16×` gives 192 ticks (3.2 s), enough to cover
/// a default-speed crossing of the stage plus the following fall.
///
/// Only the initial [`commit_ticks`](refine_by_rollout) sustain the candidate
/// verb; extending it across the full horizon would make every lateral choice
/// appear fatal. The rollout budget test guards the added search cost.
pub const MOVEMENT_HORIZON_MULTIPLE: u32 = 16;

/// How much of a body's ground speed it can steer with while airborne, once its
/// jumps are spent.
///
/// It is a SAFETY MARGIN, not a physical fact, and the engine's numbers say
/// so. `ae::AIR_ACCEL` is 3100 px/s² against a shared `MAX_RUN_SPEED` cap of
/// 270 — so a real body reaches the SAME top speed in the air as on the ground,
/// differing only in how long it takes to get there (~0.09 s). The shadow sets
/// velocity instantly and models no acceleration at all, so a fraction below 1
/// approximates the average achieved over that ramp rather than describing a
/// weaker air game.
///
/// The pessimism is still the right direction: a rollout that overestimates
/// drift certifies recoveries that will not happen, and a fighter that dives off
/// the stage on a promise is worse than one that never leaves it.
pub const AIR_DRIFT_FRACTION: f32 = 0.6;

/// The sustained shadow intent a movement verb means.
///
/// `None` for verbs the shadow model does not simulate (blink, shield-as-motion)
/// — an unmodelled verb is not judged, because a rollout that reported every
/// unknown as safe or as fatal would be lying in one direction or the other.
fn movement_intent(
    verb: ambition_characters::brain::fighter::options::MovementVerb,
    start: &ShadowState,
) -> Option<ShadowIntent> {
    use ambition_characters::brain::fighter::options::MovementVerb;
    let frame = ae::AccelerationFrame::new(start.gravity_down);
    let toward = (start.foe.pos - start.me.pos).dot(frame.side).signum();
    Some(match verb {
        MovementVerb::Approach => ShadowIntent::Drive { lateral: toward },
        MovementVerb::Dash => ShadowIntent::Dash { lateral: toward },
        MovementVerb::Retreat => ShadowIntent::Drive { lateral: -toward },
        MovementVerb::Jump => ShadowIntent::Jump,
        // `Recover` was in the unmodelled list, and it is the ONLY verb that
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
        MovementVerb::Shield | MovementVerb::Blink | MovementVerb::Dodge => return None,
    })
}

#[cfg(test)]
mod tests;
