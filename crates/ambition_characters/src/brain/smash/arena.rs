//! Headless brain-vs-brain arena simulation + non-degeneracy analytics.
//!
//! This is the **fighter-AI safety net**. It runs two brains against each other
//! in a small bounded stage (floor, ceiling, side walls, a couple of platforms),
//! with a faithful-*enough* kinematic model: locomotion throttle, gravity + jump
//! arcs for grounded bodies, free 2D `velocity_target` steering for flyers, wall/
//! floor/ceiling clamping, and melee hits that apply knockback (both fighters have
//! infinite health, so the bout never ends — we study *movement*, not who wins).
//!
//! It then analyses the recorded trace for the degeneracy signatures the design
//! forbids:
//!   - **frozen / cornered** — a fighter pinned in one spot or a wall corner for
//!     too long,
//!   - **looping** — the path collapses to a tiny repeating cycle,
//!   - **dead stage space** — columns of the arena a human would use that the
//!     fighter never visits,
//!   - **one-note play** — fewer than a handful of distinct verbs over the bout.
//!
//! The assertions are **structural / statistical**, never byte-for-byte: the game
//! logic will change over time, so the test must survive a different-but-still-good
//! fight. It is the guard the user asked for against degenerate hand-authored (or,
//! later, learned) policies — not a frame-perfect replay.
//!
//! This is a brain-level harness: the model is the brain's *contract*
//! ([`BrainSnapshot`] in, [`ActorControlFrame`] out), not the full game physics.
//! Passing here means the policy is non-degenerate; in-engine feel is verified
//! separately.

use ambition_engine_core as ae;

use super::super::action_set::ActionSet;
use super::super::snapshot::BrainSnapshot;
use super::{SmashCfg, SmashState};
use crate::actor::control::ActorControlFrame;

// ----- tuning constants (brain-level kinematics) -----

/// Fixed simulation step (s) — 60 Hz, matching the game's brain tick.
const DT: f32 = 1.0 / 60.0;
/// Downward gravity for grounded bodies (px/s²). Engine `+y` is down.
const GRAVITY: f32 = 2400.0;
/// Jump launch speed (px/s, upward). Apex ≈ JUMP_SPEED²/(2·GRAVITY).
const JUMP_SPEED: f32 = 720.0;
/// Terminal fall speed (px/s).
const MAX_FALL: f32 = 1500.0;
/// Horizontal knockback imparted by a landed melee hit (px/s).
const KNOCKBACK_X: f32 = 300.0;
/// Upward pop imparted by a landed melee hit (px/s). Small — the harness melee
/// is a *jab*, which resets spacing horizontally but does NOT launch into an
/// air juggle (that's a heavy/up-tilt, which this generic striker doesn't have).
const KNOCKBACK_UP: f32 = 70.0;
/// Hitstun (s) the victim suffers — interrupts their action and resets spacing.
const HITSTUN_S: f32 = 0.20;
/// Generic melee windup/active/recover timing (s) for the harness striker.
const WINDUP_S: f32 = 0.08;
const ACTIVE_S: f32 = 0.06;
const RECOVER_S: f32 = 0.16;
const ATTACK_COOLDOWN_S: f32 = 0.34;

/// A horizontal ledge a falling body can land on from above (one-way).
#[derive(Clone, Copy, Debug)]
pub struct Platform {
    pub x0: f32,
    pub x1: f32,
    /// Top surface y (feet rest here). Engine `+y` is down → smaller = higher.
    pub y: f32,
}

/// Bounded rectangular stage. Coordinates are feet-space: a body's `pos` is its
/// feet point; `pos.y == floor` means standing on the ground.
#[derive(Clone, Debug)]
pub struct Stage {
    pub left: f32,
    pub right: f32,
    /// Ground level (largest y).
    pub floor: f32,
    /// Ceiling level (smallest y) — bodies can't rise above this.
    pub ceiling: f32,
    pub platforms: Vec<Platform>,
}

impl Stage {
    /// The Noether-Chamber-like test arena: a wide floor, a high ceiling (room to
    /// fly), and two mid platforms so verticality is *available* stage space.
    pub fn noether_like() -> Self {
        Self {
            left: 0.0,
            right: 960.0,
            floor: 540.0,
            ceiling: 40.0,
            platforms: vec![
                Platform { x0: 180.0, x1: 380.0, y: 380.0 },
                Platform { x0: 580.0, x1: 780.0, y: 380.0 },
                Platform { x0: 360.0, x1: 600.0, y: 240.0 },
            ],
        }
    }

    pub fn width(&self) -> f32 {
        self.right - self.left
    }
    pub fn height(&self) -> f32 {
        self.floor - self.ceiling
    }
}

/// One verb a brain emitted on a tick — the alphabet the variety metric counts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Verb {
    WalkLeft,
    WalkRight,
    DashLeft,
    DashRight,
    Jump,
    Fly,
    Melee,
    Ranged,
    Special,
    Shield,
    Blink,
    Idle,
}

/// One controllable body + its brain bundle.
pub struct Fighter {
    pub name: &'static str,
    pub cfg: SmashCfg,
    pub state: SmashState,
    pub actions: ActionSet,
    /// Free-mover: ignores gravity, steers `velocity_target` in 2D.
    pub can_fly: bool,
    pub max_air_jumps: u8,
    pub max_run_speed: f32,
    /// Body half-extents (px) for hit overlap + wall clamps.
    pub half_w: f32,
    pub half_h: f32,

    // --- live kinematic state ---
    pub pos: ae::Vec2,
    pub vel: ae::Vec2,
    pub facing: f32,
    pub on_ground: bool,
    pub air_jumps: u8,
    pub attack_cooldown: f32,
    pub attack_windup: f32,
    pub attack_active: f32,
    pub attack_recover: f32,
    pub stun: f32,
}

impl Fighter {
    fn attacking(&self) -> bool {
        self.attack_windup > 0.0 || self.attack_active > 0.0 || self.attack_recover > 0.0
    }
}

/// One recorded tick for one fighter.
#[derive(Clone, Copy, Debug)]
pub struct Sample {
    pub pos: ae::Vec2,
    pub on_ground: bool,
    pub verb: Verb,
}

/// Full recording of a bout for both fighters.
pub struct FightTrace {
    pub stage: Stage,
    pub names: [&'static str; 2],
    pub samples: [Vec<Sample>; 2],
}

/// The arena: stage + two fighters + clock.
pub struct Arena {
    pub stage: Stage,
    pub fighters: [Fighter; 2],
    pub t: f32,
    pub trace: FightTrace,
}

impl Arena {
    pub fn new(stage: Stage, mut a: Fighter, mut b: Fighter) -> Self {
        // Seat each body on the floor and seed its RNG from a stable per-fighter
        // id (NOT a clock or Entity — replay-safe determinism, per the design).
        for (i, f) in [&mut a, &mut b].into_iter().enumerate() {
            f.pos.y = stage.floor;
            f.on_ground = !f.can_fly;
            f.air_jumps = f.max_air_jumps;
            f.state.rng_seed = 0x51ED_0000 ^ (i as u64 + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        }
        let names = [a.name, b.name];
        let trace = FightTrace {
            stage: stage.clone(),
            names,
            samples: [Vec::new(), Vec::new()],
        };
        Self {
            stage,
            fighters: [a, b],
            t: 0.0,
            trace,
        }
    }

    fn snapshot_for(&self, me: usize) -> BrainSnapshot {
        let f = &self.fighters[me];
        let opp = &self.fighters[1 - me];
        let mut s = BrainSnapshot::idle();
        s.actor_pos = f.pos;
        s.actor_vel = f.vel;
        s.actor_facing = f.facing;
        s.actor_on_ground = f.on_ground;
        s.alive = true;
        s.target_pos = opp.pos;
        s.target_alive = true;
        s.sim_time = self.t;
        s.dt = DT;
        s.max_run_speed = f.max_run_speed;
        s.attack_cooldown_remaining = f.attack_cooldown;
        s.attack_windup_remaining = f.attack_windup;
        s.attack_active_remaining = f.attack_active;
        s.attack_recover_remaining = f.attack_recover;
        s.stun_remaining = f.stun;
        s.air_jumps_remaining = f.air_jumps;
        s
    }

    /// Advance one tick: tick both brains, integrate, resolve melee, record.
    pub fn step(&mut self) {
        let frames: [ActorControlFrame; 2] = [self.tick_brain(0), self.tick_brain(1)];
        for i in 0..2 {
            self.apply_frame(i, &frames[i]);
        }
        // Resolve melee strikes AFTER both have moved this tick.
        self.resolve_melee(0);
        self.resolve_melee(1);
        for i in 0..2 {
            self.integrate(i);
            self.tick_timers(i);
        }
        for i in 0..2 {
            let verb = classify_verb(&frames[i], &self.fighters[i]);
            let f = &self.fighters[i];
            self.trace.samples[i].push(Sample {
                pos: f.pos,
                on_ground: f.on_ground,
                verb,
            });
        }
        self.t += DT;
    }

    fn tick_brain(&mut self, i: usize) -> ActorControlFrame {
        let snap = self.snapshot_for(i);
        let mut frame = ActorControlFrame::neutral();
        let f = &mut self.fighters[i];
        super::tick_smash(&f.cfg, &mut f.state, &f.actions, &snap, &mut frame);
        frame
    }

    fn apply_frame(&mut self, i: usize, frame: &ActorControlFrame) {
        let f = &mut self.fighters[i];
        if f.stun > 0.0 {
            return; // hitstunned bodies don't act on intent
        }
        if frame.facing.abs() > 0.001 {
            f.facing = frame.facing.signum();
        }
        if f.can_fly {
            // Free-mover: steer toward the commanded velocity directly. The Smash
            // brain emits grounded `locomotion`; until the aerial verbs land we
            // translate that throttle into a horizontal velocity and let the
            // explicit `velocity_target` (when the brain sets it) win.
            let vt = if frame.velocity_target.length_squared() > 1.0 {
                frame.velocity_target
            } else {
                ae::Vec2::new(frame.locomotion.x * f.max_run_speed, 0.0)
            };
            f.vel = vt;
        } else {
            // Grounded: locomotion.x is a throttle in [-1, 1] of max_run_speed.
            f.vel.x = frame.locomotion.x.clamp(-1.0, 1.0) * f.max_run_speed;
            // Jump edges.
            if frame.jump_pressed {
                if f.on_ground {
                    f.vel.y = -JUMP_SPEED;
                    f.on_ground = false;
                } else if f.air_jumps > 0 {
                    f.vel.y = -JUMP_SPEED;
                    f.air_jumps -= 1;
                }
            }
        }
        // Melee start: only when off cooldown and not already swinging.
        if frame.melee_pressed
            && f.actions.melee.is_some()
            && f.attack_cooldown <= 0.0
            && !f.attacking()
        {
            f.attack_windup = WINDUP_S;
            f.attack_active = 0.0;
            f.attack_recover = 0.0;
            f.attack_cooldown = ATTACK_COOLDOWN_S;
        }
        // Blink: an instantaneous reposition (used by later aerial verbs). The
        // harness models it as a short teleport along the quick-blink dir / facing.
        if frame.blink_pressed {
            let dir = if frame.blink_quick_dir.length_squared() > 0.01 {
                frame.blink_quick_dir.normalize_or_zero()
            } else {
                ae::Vec2::new(f.facing, 0.0)
            };
            f.pos += dir * 140.0;
        }
    }

    fn resolve_melee(&mut self, attacker: usize) {
        // A hit lands during the active window, when the victim is within reach in
        // front of the attacker. We promote windup→active here (one-tick windup
        // model) so a committed swing always gets an active frame.
        let victim = 1 - attacker;
        let (reach, swinging, windup, facing, apos) = {
            let f = &self.fighters[attacker];
            (
                f.cfg.attack_range + f.half_w,
                f.attack_active > 0.0,
                f.attack_windup,
                f.facing,
                f.pos,
            )
        };
        // Transition windup→active exactly when windup elapses (handled in timers);
        // here we only deal damage on an active frame.
        let _ = windup;
        if !swinging {
            return;
        }
        let v_half_w = self.fighters[victim].half_w;
        let v_half_h = self.fighters[victim].half_h;
        let vpos = self.fighters[victim].pos;
        let to_v = vpos - apos;
        let in_front = to_v.x.signum() == facing.signum() || to_v.x.abs() < 1.0;
        let horiz_reach = reach + v_half_w;
        let vert_reach = self.fighters[attacker].half_h + v_half_h;
        // Hit lands only when in horizontal reach AND vertically overlapping —
        // a grounded jab can't strike a target standing on a platform overhead.
        if to_v.x.abs() <= horiz_reach && to_v.y.abs() <= vert_reach && in_front {
            let push = if to_v.x.abs() < 1.0 {
                facing.signum()
            } else {
                to_v.x.signum()
            };
            let v = &mut self.fighters[victim];
            // Don't re-stun every active tick: only (re)apply if not already fresh.
            if v.stun <= 0.0 {
                v.vel.x = push * KNOCKBACK_X;
                v.vel.y = -KNOCKBACK_UP;
                v.on_ground = false;
                v.stun = HITSTUN_S;
                // Interrupt the victim's own swing.
                v.attack_windup = 0.0;
                v.attack_active = 0.0;
            }
        }
    }

    fn integrate(&mut self, i: usize) {
        let stage = &self.stage;
        let f = &mut self.fighters[i];
        if !f.can_fly {
            // Gravity unless standing.
            if !f.on_ground {
                f.vel.y = (f.vel.y + GRAVITY * DT).min(MAX_FALL);
            }
        } else {
            // Flyer: clamp ceiling/floor handled below; no gravity.
            f.on_ground = false;
        }
        let prev_y = f.pos.y;
        f.pos += f.vel * DT;

        // Side walls.
        let min_x = stage.left + f.half_w;
        let max_x = stage.right - f.half_w;
        if f.pos.x < min_x {
            f.pos.x = min_x;
            if f.vel.x < 0.0 {
                f.vel.x = 0.0;
            }
        } else if f.pos.x > max_x {
            f.pos.x = max_x;
            if f.vel.x > 0.0 {
                f.vel.x = 0.0;
            }
        }
        // Ceiling.
        if f.pos.y < stage.ceiling {
            f.pos.y = stage.ceiling;
            if f.vel.y < 0.0 {
                f.vel.y = 0.0;
            }
        }
        // Floor.
        if f.pos.y >= stage.floor {
            f.pos.y = stage.floor;
            if !f.can_fly {
                if !f.on_ground {
                    f.on_ground = true;
                    f.air_jumps = f.max_air_jumps;
                }
                f.vel.y = 0.0;
            }
        } else if !f.can_fly {
            // One-way platform landings: only when falling and crossing from above.
            if f.vel.y > 0.0 {
                for p in &stage.platforms {
                    if f.pos.x >= p.x0 - f.half_w
                        && f.pos.x <= p.x1 + f.half_w
                        && prev_y <= p.y
                        && f.pos.y >= p.y
                    {
                        f.pos.y = p.y;
                        f.vel.y = 0.0;
                        f.on_ground = true;
                        f.air_jumps = f.max_air_jumps;
                        break;
                    }
                }
            }
            // Walked off the side of whatever we were standing on.
            if f.on_ground && f.pos.y < stage.floor {
                let still_supported = stage.platforms.iter().any(|p| {
                    (f.pos.y - p.y).abs() < 1.0 && f.pos.x >= p.x0 - f.half_w && f.pos.x <= p.x1 + f.half_w
                });
                if !still_supported {
                    f.on_ground = false;
                }
            }
        }
    }

    fn tick_timers(&mut self, i: usize) {
        let f = &mut self.fighters[i];
        f.stun = (f.stun - DT).max(0.0);
        f.attack_cooldown = (f.attack_cooldown - DT).max(0.0);
        if f.attack_windup > 0.0 {
            f.attack_windup = (f.attack_windup - DT).max(0.0);
            if f.attack_windup <= 0.0 {
                f.attack_active = ACTIVE_S;
            }
        } else if f.attack_active > 0.0 {
            f.attack_active = (f.attack_active - DT).max(0.0);
            if f.attack_active <= 0.0 {
                f.attack_recover = RECOVER_S;
            }
        } else if f.attack_recover > 0.0 {
            f.attack_recover = (f.attack_recover - DT).max(0.0);
        }
    }

    /// Run the bout for `secs` seconds of sim time.
    pub fn run(&mut self, secs: f32) {
        let ticks = (secs / DT).round() as usize;
        for _ in 0..ticks {
            self.step();
        }
    }
}

fn classify_verb(frame: &ActorControlFrame, f: &Fighter) -> Verb {
    // Priority: action verbs over movement; movement over idle.
    if frame.melee_pressed {
        return Verb::Melee;
    }
    if frame.special_pressed {
        return Verb::Special;
    }
    if frame.blink_pressed {
        return Verb::Blink;
    }
    if frame.shield_held {
        return Verb::Shield;
    }
    if frame.fire.is_some() {
        return Verb::Ranged;
    }
    if frame.jump_pressed {
        return Verb::Jump;
    }
    if f.can_fly && frame.velocity_target.length_squared() > 1.0 {
        return Verb::Fly;
    }
    let x = if f.can_fly {
        frame.velocity_target.x
    } else {
        frame.locomotion.x
    };
    let mag = x.abs();
    if mag < 0.05 {
        return Verb::Idle;
    }
    // A throttle near full is a dash; a partial throttle is a walk.
    let dash = mag > 0.9;
    match (x > 0.0, dash) {
        (true, true) => Verb::DashRight,
        (true, false) => Verb::WalkRight,
        (false, true) => Verb::DashLeft,
        (false, false) => Verb::WalkLeft,
    }
}

// ===================== analytics =====================

/// Computed degeneracy metrics for one fighter's trace.
#[derive(Clone, Debug)]
pub struct FighterReport {
    pub name: &'static str,
    /// Distinct horizontal columns (of `H_BINS`) the fighter occupied.
    pub x_bins_visited: usize,
    pub x_bins_total: usize,
    /// Longest continuous time (s) spent inside one column — the "frozen/loop in
    /// place" detector.
    pub max_still_s: f32,
    /// Longest continuous time (s) spent in a wall-corner region.
    pub max_corner_s: f32,
    /// Fraction of ticks spent airborne (off the ground) — vertical-space usage.
    pub airborne_frac: f32,
    /// Distinct verbs used over the bout.
    pub distinct_verbs: usize,
    pub verbs: Vec<(Verb, usize)>,
    /// Total horizontal path length (px) — a frozen fighter scores ~0.
    pub path_len: f32,
}

/// How many horizontal columns the stage is binned into for coverage.
const H_BINS: usize = 12;
/// A "corner" is within this fraction of the stage width from a side wall.
const CORNER_FRAC: f32 = 0.12;
/// Radius (px) within which a fighter counts as "camping a spot" for the still
/// detector. Comfortably larger than a neutral footsies weave so normal spacing
/// isn't flagged, but far smaller than the stage so genuine freezing is.
const STILL_RADIUS: f32 = 90.0;

pub fn analyze_fighter(stage: &Stage, name: &'static str, samples: &[Sample]) -> FighterReport {
    let w = stage.width().max(1.0);
    let bin_w = w / H_BINS as f32;
    let mut visited = [false; H_BINS];
    let bin_of = |x: f32| -> usize {
        (((x - stage.left) / bin_w).floor() as isize).clamp(0, H_BINS as isize - 1) as usize
    };

    let corner_margin = stage.width() * CORNER_FRAC;
    let in_corner = |x: f32| x < stage.left + corner_margin || x > stage.right - corner_margin;

    // "Still" = camping a spot, measured bin-independently: the longest stretch
    // during which the fighter never strayed more than STILL_RADIUS from an
    // anchor (reset when it does). Captures genuine freezing/looping-in-place
    // without the fixed-column aliasing that penalizes a normal neutral weave.
    let mut max_still_s = 0.0_f32;
    let mut still_anchor: Option<ae::Vec2> = None;
    let mut still_start_t = 0.0_f32;
    let mut t = 0.0_f32;
    let mut max_corner_s = 0.0_f32;
    let mut cur_corner = 0.0_f32;
    let mut airborne = 0usize;
    let mut path_len = 0.0_f32;
    let mut verb_counts: std::collections::HashMap<Verb, usize> = std::collections::HashMap::new();
    let mut prev_x: Option<f32> = None;

    for s in samples {
        let b = bin_of(s.pos.x);
        visited[b] = true;
        match still_anchor {
            Some(a) if (s.pos - a).length() <= STILL_RADIUS => {
                max_still_s = max_still_s.max(t - still_start_t);
            }
            _ => {
                still_anchor = Some(s.pos);
                still_start_t = t;
            }
        }
        t += DT;

        if in_corner(s.pos.x) {
            cur_corner += DT;
        } else {
            cur_corner = 0.0;
        }
        max_corner_s = max_corner_s.max(cur_corner);

        if !s.on_ground {
            airborne += 1;
        }
        if let Some(px) = prev_x {
            path_len += (s.pos.x - px).abs();
        }
        prev_x = Some(s.pos.x);
        *verb_counts.entry(s.verb).or_insert(0) += 1;
    }

    let x_bins_visited = visited.iter().filter(|v| **v).count();
    let mut verbs: Vec<(Verb, usize)> = verb_counts.into_iter().collect();
    verbs.sort_by_key(|(_, c)| std::cmp::Reverse(*c));
    // "Distinct verbs" counts only meaningfully-used verbs (>= a few ticks), so a
    // single stray frame doesn't inflate variety.
    let distinct_verbs = verbs.iter().filter(|(v, c)| *c >= 3 && *v != Verb::Idle).count();
    let airborne_frac = if samples.is_empty() {
        0.0
    } else {
        airborne as f32 / samples.len() as f32
    };

    FighterReport {
        name,
        x_bins_visited,
        x_bins_total: H_BINS,
        max_still_s,
        max_corner_s,
        airborne_frac,
        distinct_verbs,
        verbs,
        path_len,
    }
}

/// Thresholds the non-degeneracy assertion enforces. Deliberately lenient — they
/// catch *degenerate* play (frozen, cornered, one-note), not "imperfect" play.
#[derive(Clone, Copy, Debug)]
pub struct NonDegenerateThresholds {
    pub min_x_bins: usize,
    pub max_still_s: f32,
    pub max_corner_s: f32,
    pub min_distinct_verbs: usize,
    pub min_path_len: f32,
}

impl Default for NonDegenerateThresholds {
    fn default() -> Self {
        Self {
            min_x_bins: 5,        // must roam ≥ ~40% of the arena's width
            max_still_s: 5.0,     // never camped within one ~90px spot > 5 s
            max_corner_s: 6.0,    // never pinned in a corner > 6 s
            min_distinct_verbs: 3,
            min_path_len: 1500.0, // must actually travel
        }
    }
}

impl FighterReport {
    /// Returns the list of degeneracy violations (empty = healthy).
    pub fn violations(&self, th: &NonDegenerateThresholds) -> Vec<String> {
        let mut v = Vec::new();
        if self.x_bins_visited < th.min_x_bins {
            v.push(format!(
                "{}: dead stage space — only visited {}/{} columns (want ≥ {})",
                self.name, self.x_bins_visited, self.x_bins_total, th.min_x_bins
            ));
        }
        if self.max_still_s > th.max_still_s {
            v.push(format!(
                "{}: frozen/looping — stayed in one column {:.1}s (max {:.1}s)",
                self.name, self.max_still_s, th.max_still_s
            ));
        }
        if self.max_corner_s > th.max_corner_s {
            v.push(format!(
                "{}: cornered — pinned in a wall corner {:.1}s (max {:.1}s)",
                self.name, self.max_corner_s, th.max_corner_s
            ));
        }
        if self.distinct_verbs < th.min_distinct_verbs {
            v.push(format!(
                "{}: one-note — only {} distinct verbs (want ≥ {}); used {:?}",
                self.name, self.distinct_verbs, th.min_distinct_verbs, self.verbs
            ));
        }
        if self.path_len < th.min_path_len {
            v.push(format!(
                "{}: barely moves — path length {:.0}px (want ≥ {:.0})",
                self.name, self.path_len, th.min_path_len
            ));
        }
        v
    }

    pub fn summary(&self) -> String {
        format!(
            "{:>8}: cols {}/{}  still {:.1}s  corner {:.1}s  air {:.0}%  verbs {} {:?}  path {:.0}px",
            self.name,
            self.x_bins_visited,
            self.x_bins_total,
            self.max_still_s,
            self.max_corner_s,
            self.airborne_frac * 100.0,
            self.distinct_verbs,
            self.verbs.iter().map(|(v, _)| *v).collect::<Vec<_>>(),
            self.path_len,
        )
    }
}

impl FightTrace {
    pub fn reports(&self) -> [FighterReport; 2] {
        [
            analyze_fighter(&self.stage, self.names[0], &self.samples[0]),
            analyze_fighter(&self.stage, self.names[1], &self.samples[1]),
        ]
    }
}

// ===================== tests =====================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::brain::action_set::{MeleeActionSpec, SwipeSpec};

    fn striker_actions() -> ActionSet {
        ActionSet {
            melee: Some(MeleeActionSpec::Swipe(SwipeSpec::STRIKER_DEFAULT)),
            ..ActionSet::peaceful()
        }
    }

    /// A grounded brawler — stands in for the "player robot" opponent.
    fn robot(name: &'static str, x: f32) -> Fighter {
        Fighter {
            name,
            cfg: SmashCfg::DUELIST_DEFAULT,
            state: SmashState::default(),
            actions: striker_actions(),
            can_fly: false,
            max_air_jumps: 1,
            max_run_speed: 200.0,
            half_w: 16.0,
            half_h: 26.0,
            pos: ae::Vec2::new(x, 0.0),
            vel: ae::Vec2::ZERO,
            facing: 1.0,
            on_ground: true,
            air_jumps: 1,
            attack_cooldown: 0.0,
            attack_windup: 0.0,
            attack_active: 0.0,
            attack_recover: 0.0,
            stun: 0.0,
        }
    }

    /// The PCA — today a grounded Smash striker like the robot; the aerial /
    /// blink / special verbs land in later slices and this fighter flips to
    /// `can_fly = true`.
    fn pca(name: &'static str, x: f32) -> Fighter {
        Fighter {
            name,
            cfg: SmashCfg::DUELIST_DEFAULT,
            state: SmashState::default(),
            actions: striker_actions(),
            can_fly: false,
            max_air_jumps: 1,
            max_run_speed: 210.0,
            half_w: 16.0,
            half_h: 26.0,
            pos: ae::Vec2::new(x, 0.0),
            vel: ae::Vec2::ZERO,
            facing: -1.0,
            on_ground: true,
            air_jumps: 1,
            attack_cooldown: 0.0,
            attack_windup: 0.0,
            attack_active: 0.0,
            attack_recover: 0.0,
            stun: 0.0,
        }
    }

    /// Characterization: a bout runs to completion, stays in bounds, no NaNs, and
    /// the trace analytics produce a sane report. This is the always-green floor;
    /// the non-degeneracy thresholds are asserted in
    /// `pca_vs_robot_is_non_degenerate` (which the brain work makes pass).
    #[test]
    fn bout_runs_and_stays_in_bounds() {
        let stage = Stage::noether_like();
        let mut arena = Arena::new(stage.clone(), pca("PCA", 720.0), robot("Robot", 240.0));
        arena.run(20.0);
        for i in 0..2 {
            for s in &arena.trace.samples[i] {
                assert!(s.pos.x.is_finite() && s.pos.y.is_finite(), "NaN in trace");
                assert!(
                    s.pos.x >= stage.left - 1.0 && s.pos.x <= stage.right + 1.0,
                    "{} left the arena horizontally at x={}",
                    arena.trace.names[i],
                    s.pos.x
                );
                assert!(
                    s.pos.y <= stage.floor + 1.0 && s.pos.y >= stage.ceiling - 1.0,
                    "{} left the arena vertically at y={}",
                    arena.trace.names[i],
                    s.pos.y
                );
            }
            assert!(
                !arena.trace.samples[i].is_empty(),
                "no samples recorded for fighter {i}"
            );
        }
        // Observe the trace (the user asked to be able to see it).
        let [ra, rb] = arena.trace.reports();
        println!("--- characterization bout ---\n{}\n{}", ra.summary(), rb.summary());
    }

    /// The degeneracy guard the user asked for. Both fighters must use the stage
    /// (roam horizontally, not freeze, not camp a corner) and employ a variety of
    /// verbs. Structural, not byte-for-byte — it survives logic changes.
    ///
    /// Passes now that the duelist neutral game (footsies weave + neutral hops +
    /// platform-only vertical chase) replaced point-blank mashing: the fighters
    /// dance in and out of poke range across the stage instead of collapsing into
    /// a wall.
    #[test]
    fn pca_vs_robot_is_non_degenerate() {
        let stage = Stage::noether_like();
        let mut arena = Arena::new(stage.clone(), pca("PCA", 720.0), robot("Robot", 240.0));
        arena.run(30.0);
        let reports = arena.trace.reports();
        let th = NonDegenerateThresholds::default();
        let mut all = Vec::new();
        for r in &reports {
            println!("{}", r.summary());
            all.extend(r.violations(&th));
        }
        assert!(
            all.is_empty(),
            "degenerate fight detected:\n  {}",
            all.join("\n  ")
        );
    }
}
