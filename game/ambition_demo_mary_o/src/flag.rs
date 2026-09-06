//!  — the flagpole sequence.
//!
//! `docs/planning/demos/super-mary-o.md`: *" level-end sequencing: flagpole grab → slide →
//! walk-off → score tally."*
//!
//! Content-side, and it adds zero engine code. The whole sequence is a state
//! machine over a clock plus one authored geometry fact (where the pole is), and
//! the only thing it does to the body is write its position and suppress its
//! controls — both of which any content plugin may do.
//!
//! ## Why the score is computed from the GRAB, not from the slide
//!
//! That height is a fact about the moment of contact, and the slide is a celebration.
//!
//! ## Why the pole is not solid
//!
//! ## Why the player is not "frozen"
//!
//! A frozen body is still a body: gravity keeps pulling, the movement kernel keeps
//! resolving, and a well-placed enemy could still hit it. The sequence takes the
//! body's POSITION each tick — it drives, rather than pauses. That is also what
//! makes it testable without a physics step.

use ambition_platformer2d::engine_core as ae;
use bevy::prelude::*;

/// Where the pole is, how tall, and how thick. Mirrors the authored `goal_pole`
/// block so the sequence never has to search the world for it — the level knows.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct FlagPole {
    /// World x of the pole's center.
    pub x: f32,
    /// World y of the pole's TOP (`+y` is down, so this is the small number).
    pub top_y: f32,
    /// World y of the pole's base, where the slide ends.
    pub base_y: f32,
    /// Half the pole's authored thickness.
    pub half_width: f32,
}

impl FlagPole {
    /// Where on the pole, `0..=1`, a body at `y` caught it. `1` is the very top.
    ///
    /// Clamped, because a body can touch the pole from a platform above its top or
    /// from below its base, and neither is worth a special case.
    pub fn grab_height(&self, y: f32) -> f32 {
        let span = (self.base_y - self.top_y).max(1.0);
        ((self.base_y - y) / span).clamp(0.0, 1.0)
    }

    /// How far a body's CENTER may be from the pole's center and still count as
    /// touching it: the pole's half-thickness plus a body's half-width. That is
    /// exactly "the body's box overlaps the pole's box", expressed against the one
    /// number the sequence has (a center), so a grab fires the instant she makes
    /// contact — from either side, at any height, running or falling.
    ///
    /// A grab band smaller than the body it is meant to catch is unreachable by construction.
    pub fn grab_half_width(&self) -> f32 {
        self.half_width + GRAB_BODY_HALF_WIDTH
    }
}

/// The body half-width the grab band budgets for.
///
/// Mary-O's small form; her grown form is wider, so this errs NARROW — the tall
/// body simply overlaps the pole before the band says so, which costs a frame at
/// most. Sized off the engine default rather than a literal so a change to the
/// standard body carries here.
const GRAB_BODY_HALF_WIDTH: f32 = ae::DEFAULT_PLAYER_BODY_WIDTH * 0.5;

/// Score for a grab at `height` (`0..=1`). The classic ladder: five bands, top
/// band worth an order of magnitude more than the bottom.
///
/// A pure function of one number, so the reward curve is arguable in a code review
/// rather than discoverable in a playtest.
pub fn flag_score(height: f32) -> u32 {
    match height {
        h if h >= 0.90 => 5000,
        h if h >= 0.70 => 2000,
        h if h >= 0.50 => 800,
        h if h >= 0.25 => 400,
        _ => 100,
    }
}

/// How fast the body slides down the pole, world px per second.
pub const SLIDE_SPEED: f32 = 220.0;
/// How far right the body walks off, in world px, before the tally.
pub const WALK_OFF_PX: f32 = 96.0;
/// Walking-off speed, world px per second.
pub const WALK_OFF_SPEED: f32 = 90.0;

/// The four beats. `Idle` is the whole level; the other three are the sequence.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum FlagPhase {
    /// Nobody has touched the pole.
    #[default]
    Idle,
    /// Riding the pole down. Carries the score already earned — see the module docs.
    Sliding { score: u32 },
    /// Walking off to the right, `remaining` px to go.
    WalkingOff { score: u32, remaining: f32 },
    /// Done. The tally is on screen and the level is over.
    Tallied { score: u32 },
}

/// Live sequence state. Mode-scoped, like the level clock — the engine despawns it
/// when the active room's mode changes, and there is no teardown code here.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct FlagSequence {
    pub phase: FlagPhase,
    /// Where the sequence has driven the body to, once it has taken over.
    ///
    /// This is what makes the sequence immune to system ordering. If each tick
    /// re-read the body's live position, a gravity step landing between this system
    /// and the next would accumulate into the slide. Once the flag is grabbed, the
    /// body's own position stops being an input.
    pub driven: Option<ae::Vec2>,
}

impl FlagSequence {
    /// Has the sequence taken over? While true, the level clock stops and the
    /// player's controls are ignored.
    pub fn active(&self) -> bool {
        !matches!(self.phase, FlagPhase::Idle)
    }

    pub fn score(&self) -> Option<u32> {
        match self.phase {
            FlagPhase::Idle => None,
            FlagPhase::Sliding { score }
            | FlagPhase::WalkingOff { score, .. }
            | FlagPhase::Tallied { score } => Some(score),
        }
    }
}

/// The whole sequence, as a pure function of `(state, pole, body, dt)`.
///
/// Returns where the body should be this tick. `None` in `Idle` — the player is still playing, and
/// the sequence has no opinion about where they are. `body_half_height` is what turns the pole's
/// base — a GROUND LINE — into a body CENTRE. A scripted pose overrules physics by design (that is
/// the point of `constrain_body_pose`), so nothing downstream was ever going to lift her back out —
/// and nothing should: this project does not shove bodies out of geometry, it puts them in the
/// right place to begin with. What the sequence is doing to the body this tick — where it puts
/// her, how fast she is going, and whether she is on the pole.
///
/// `constrain_body_pose` already takes an imposed velocity — its own doc names
/// *"a scripted end-of-level slide"* — and this sequence passed `Vec2::ZERO`. So
/// her motion facts said "standing still" while her position jumped, and the
/// animation picker, which reads exactly those facts, correctly chose Idle. The
/// clip was never the thing to fix.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FlagDrive {
    /// Where her centre is this tick.
    pub pos: ae::Vec2,
    /// How fast she is moving, imposed on the body so every reader of motion —
    /// the animation picker above all — is told the truth.
    pub vel: ae::Vec2,
    /// She is on the pole. Drives `BodyMode::Climbing`, which the picker reads.
    pub on_pole: bool,
}

pub fn step_flag_sequence(
    seq: &mut FlagSequence,
    pole: &FlagPole,
    body: ae::Vec2,
    body_half_height: f32,
    dt: f32,
) -> Option<FlagDrive> {
    // Once the sequence is driving, the body it is driving is the one IT last put
    // down, not whatever the physics step left behind.
    let body = seq.driven.unwrap_or(body);
    let out = step_phase(seq, pole, body, body_half_height, dt);
    seq.driven = out.map(|drive| drive.pos);
    out
}

fn step_phase(
    seq: &mut FlagSequence,
    pole: &FlagPole,
    body: ae::Vec2,
    body_half_height: f32,
    dt: f32,
) -> Option<FlagDrive> {
    // Where her CENTRE rests when her feet are on the pole's base.
    let stand_y = pole.base_y - body_half_height;
    match seq.phase {
        FlagPhase::Idle => {
            if (body.x - pole.x).abs() > pole.grab_half_width() || body.y > pole.base_y {
                return None;
            }
            // The score is a fact about the moment of contact. Everything after
            // this is a celebration.
            seq.phase = FlagPhase::Sliding {
                score: flag_score(pole.grab_height(body.y)),
            };
            // Snap onto the pole so the slide is straight. She is on it from
            // this tick, and already sliding.
            Some(FlagDrive {
                pos: ae::Vec2::new(pole.x, body.y.max(pole.top_y)),
                vel: ae::Vec2::new(0.0, SLIDE_SPEED),
                on_pole: true,
            })
        }
        FlagPhase::Sliding { score } => {
            let y = body.y + SLIDE_SPEED * dt;
            // The slide ends when her FEET reach the base, not her centre — the
            // pole's base is a ground line.
            if y >= stand_y {
                seq.phase = FlagPhase::WalkingOff {
                    score,
                    remaining: WALK_OFF_PX,
                };
                // Her feet are down: off the pole, and walking from here.
                return Some(FlagDrive {
                    pos: ae::Vec2::new(pole.x, stand_y),
                    vel: ae::Vec2::new(WALK_OFF_SPEED, 0.0),
                    on_pole: false,
                });
            }
            Some(FlagDrive {
                pos: ae::Vec2::new(pole.x, y),
                vel: ae::Vec2::new(0.0, SLIDE_SPEED),
                on_pole: true,
            })
        }
        FlagPhase::WalkingOff { score, remaining } => {
            let step = WALK_OFF_SPEED * dt;
            if step >= remaining {
                seq.phase = FlagPhase::Tallied { score };
                // she is still WALKING on this tick — it is the stride that
                // covers the last `remaining` px. Reporting zero here was the same
                // untruth in miniature: the frame she arrives on would animate as
                // a stand while she was still crossing ground. She stops in
                // `Tallied`, on the tick she is actually still.
                return Some(FlagDrive {
                    pos: ae::Vec2::new(body.x + remaining, body.y),
                    vel: ae::Vec2::new(WALK_OFF_SPEED, 0.0),
                    on_pole: false,
                });
            }
            seq.phase = FlagPhase::WalkingOff {
                score,
                remaining: remaining - step,
            };
            Some(FlagDrive {
                pos: ae::Vec2::new(body.x + step, body.y),
                vel: ae::Vec2::new(WALK_OFF_SPEED, 0.0),
                on_pole: false,
            })
        }
        // Done. The body stays where it is; a results screen is.
        FlagPhase::Tallied { .. } => Some(FlagDrive {
            pos: body,
            vel: ae::Vec2::ZERO,
            on_pole: false,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 1.0 / 60.0;
    /// A body half-height for the tests. The pole's base is a GROUND LINE, so the
    /// slide must end this far above it — with her feet on the base, not her
    /// middle buried in it.
    const HALF_H: f32 = 24.0;

    fn pole() -> FlagPole {
        FlagPole {
            x: 1000.0,
            top_y: 100.0,
            base_y: 400.0,
            half_width: 8.0,
        }
    }

    fn run_until_tallied(seq: &mut FlagSequence, pole: &FlagPole, start: ae::Vec2) -> u32 {
        let mut body = start;
        for _ in 0..2000 {
            if let Some(next) = step_flag_sequence(seq, pole, body, HALF_H, DT) {
                body = next.pos;
            }
            if let FlagPhase::Tallied { score } = seq.phase {
                return score;
            }
        }
        panic!(
            "the sequence never finished from {start:?}: {:?}",
            seq.phase
        );
    }

    /// THE SEQUENCE SAYS WHAT SHE IS DOING, SO THE ANIMATION FOLLOWS.
    ///
    /// climb animation to slide down the pole and then the walk animation to move
    /// after … we should not hack these in … so we get these animations for
    /// free."*
    ///
    /// the clip was never the thing to fix. The sequence imposed
    /// `Vec2::ZERO` as her velocity, so every reader of her motion was told she
    /// stood still while her position jumped — and the animation picker, which
    /// reads exactly those facts, correctly chose Idle. This asserts the facts
    /// the picker consumes: `BodyMode::Climbing` on the pole (the picker's own
    /// `a_climbing_body_picks_the_ladder_clip` turns that into `LadderClimb`) and
    /// real ground speed on the walk-off.
    ///
    /// it deliberately does NOT assert a clip. Naming one here would re-implement
    /// the picker in the fixture and then agree with itself.
    #[test]
    fn the_slide_climbs_and_the_walk_off_walks() {
        let p = pole();
        let mut seq = FlagSequence::default();
        // Grab high up.
        let mut body = ae::Vec2::new(p.x, p.top_y + 10.0);

        let grab = step_flag_sequence(&mut seq, &p, body, HALF_H, DT).expect("the grab drives her");
        assert!(grab.on_pole, "the grab must put her ON the pole");
        assert!(
            grab.vel.y > 0.0,
            "she must be moving DOWN the pole, not standing on it: {:?}",
            grab.vel
        );
        body = grab.pos;

        // Slide to the base, checking every tick of it.
        let mut slide_ticks = 0usize;
        while matches!(seq.phase, FlagPhase::Sliding { .. }) {
            let d = step_flag_sequence(&mut seq, &p, body, HALF_H, DT).expect("still sliding");
            assert!(d.on_pole || !matches!(seq.phase, FlagPhase::Sliding { .. }));
            body = d.pos;
            slide_ticks += 1;
            assert!(slide_ticks < 10_000, "the slide never ended");
        }
        assert!(
            slide_ticks > 1,
            "the slide was one tick, so it proved nothing"
        );

        // Walking off: on the ground, moving along it, no longer on the pole.
        let mut walk_ticks = 0usize;
        while matches!(seq.phase, FlagPhase::WalkingOff { .. }) {
            let d = step_flag_sequence(&mut seq, &p, body, HALF_H, DT).expect("still walking");
            assert!(!d.on_pole, "the walk-off is not on the pole");
            assert!(
                d.vel.x > 0.0 && d.vel.y == 0.0,
                "the walk-off must read as walking along the ground: {:?}",
                d.vel
            );
            body = d.pos;
            walk_ticks += 1;
            assert!(walk_ticks < 10_000, "the walk-off never ended");
        }
        assert!(
            walk_ticks > 1,
            "the walk-off was one tick, so it proved nothing"
        );

        // And she stops when it is over, rather than drifting into the tally.
        let done =
            step_flag_sequence(&mut seq, &p, body, HALF_H, DT).expect("tallied still drives");
        assert_eq!(
            done.vel,
            ae::Vec2::ZERO,
            "she keeps moving after the sequence"
        );
        assert!(!done.on_pole);
    }

    /// The pole is not touched by walking near it, and not by passing under its base.
    #[test]
    fn the_sequence_only_starts_at_the_pole() {
        let p = pole();
        let mut seq = FlagSequence::default();
        assert_eq!(
            step_flag_sequence(&mut seq, &p, ae::Vec2::new(900.0, 300.0), HALF_H, DT),
            None
        );
        assert_eq!(seq.phase, FlagPhase::Idle);

        // Below the base: the player ran past the pole on the ground behind it.
        assert_eq!(
            step_flag_sequence(&mut seq, &p, ae::Vec2::new(1000.0, 500.0), HALF_H, DT),
            None
        );
        assert_eq!(seq.phase, FlagPhase::Idle);

        // On it.
        assert!(
            step_flag_sequence(&mut seq, &p, ae::Vec2::new(1005.0, 300.0), HALF_H, DT).is_some()
        );
        assert!(seq.active());
    }

    /// The score is a fact about the GRAB. A slow slide and a fast one from the
    /// same height pay the same, because the score was decided on contact.
    #[test]
    fn the_score_is_decided_at_the_moment_of_contact() {
        let p = pole();
        let mut seq = FlagSequence::default();
        step_flag_sequence(&mut seq, &p, ae::Vec2::new(1000.0, 120.0), HALF_H, DT);
        let at_grab = seq.score().unwrap();
        assert_eq!(at_grab, 5000, "caught it near the top");

        // Slide all the way down — the score never changes.
        let final_score = run_until_tallied(&mut seq, &p, ae::Vec2::new(1000.0, 120.0));
        assert_eq!(final_score, at_grab);
    }

    /// Higher pays more, and every band is reachable. A reward curve nobody can
    /// read is a reward curve nobody can balance.
    #[test]
    fn every_score_band_is_reachable_and_monotone() {
        let p = pole();
        let mut last = 0;
        for h in [0.0, 0.3, 0.6, 0.8, 1.0] {
            let s = flag_score(h);
            assert!(s > last, "band at {h} pays {s}, not more than {last}");
            last = s;
        }
        assert_eq!(flag_score(1.0), 5000);
        assert_eq!(flag_score(0.0), 100);
        // Grabbing above the top, or below the base, clamps rather than panicking.
        assert_eq!(p.grab_height(p.top_y - 999.0), 1.0);
        assert_eq!(p.grab_height(p.base_y + 999.0), 0.0);
    }

    /// Grab → slide → walk-off → tally, in that order, exactly once each.
    #[test]
    fn the_sequence_runs_its_four_beats_in_order() {
        let p = pole();
        let mut seq = FlagSequence::default();
        let mut body = ae::Vec2::new(1000.0, 200.0);
        let mut seen: Vec<&'static str> = Vec::new();

        for _ in 0..2000 {
            let label = match seq.phase {
                FlagPhase::Idle => "idle",
                FlagPhase::Sliding { .. } => "sliding",
                FlagPhase::WalkingOff { .. } => "walking",
                FlagPhase::Tallied { .. } => "tallied",
            };
            if seen.last() != Some(&label) {
                seen.push(label);
            }
            if let Some(next) = step_flag_sequence(&mut seq, &p, body, HALF_H, DT) {
                body = next.pos;
            }
            if matches!(seq.phase, FlagPhase::Tallied { .. }) {
                break;
            }
        }
        assert_eq!(seen, ["idle", "sliding", "walking"]);
        assert!(matches!(seq.phase, FlagPhase::Tallied { .. }));
    }

    /// The slide is straight down the pole, and the walk-off goes right by exactly
    /// `WALK_OFF_PX`. Both numbers are the ones the level was built around.
    #[test]
    fn the_slide_is_straight_and_the_walk_off_is_exact() {
        let p = pole();
        let mut seq = FlagSequence::default();
        let mut body = ae::Vec2::new(1004.0, 200.0);

        // Grab snaps onto the pole's x.
        body = step_flag_sequence(&mut seq, &p, body, HALF_H, DT)
            .unwrap()
            .pos;
        assert_eq!(body.x, p.x);

        while matches!(seq.phase, FlagPhase::Sliding { .. }) {
            body = step_flag_sequence(&mut seq, &p, body, HALF_H, DT)
                .unwrap()
                .pos;
            assert_eq!(body.x, p.x, "the slide never drifts sideways");
        }
        assert_eq!(
            body.y + HALF_H,
            p.base_y,
            "the slide plants her feet on the base, not her middle in it"
        );

        let walk_start = body.x;
        while matches!(seq.phase, FlagPhase::WalkingOff { .. }) {
            body = step_flag_sequence(&mut seq, &p, body, HALF_H, DT)
                .unwrap()
                .pos;
            assert_eq!(
                body.y + HALF_H,
                p.base_y,
                "and the walk-off never sinks her back into it"
            );
        }
        assert!(
            (body.x - walk_start - WALK_OFF_PX).abs() < 0.001,
            "walked {} px, expected {WALK_OFF_PX}",
            body.x - walk_start
        );
    }

    /// Once tallied, the sequence is inert: it holds the body and changes nothing.
    /// Why [`FlagSequence::driven`] exists. Once the flag is grabbed, a physics
    /// step that moves the body between ticks must not move the slide. We simulate
    /// the worst case: gravity yanks the body a full tile every frame, and the
    /// sequence still lands the same slide, the same walk-off, the same score.
    #[test]
    fn a_grabbed_sequence_ignores_whatever_physics_does_to_the_body() {
        let p = pole();
        let start = ae::Vec2::new(p.x, 130.0);

        let mut clean = FlagSequence::default();
        let clean_score = run_until_tallied(&mut clean, &p, start);

        let mut kicked = FlagSequence::default();
        let mut body = start;
        let mut score = None;
        for _ in 0..2000 {
            if let Some(next) = step_flag_sequence(&mut kicked, &p, body, HALF_H, DT) {
                // Physics runs after us and shoves the body a tile down and right.
                body = next.pos + ae::Vec2::new(16.0, 16.0);
            }
            if let FlagPhase::Tallied { score: s } = kicked.phase {
                score = Some(s);
                break;
            }
        }
        assert_eq!(score, Some(clean_score), "the shove changed the score");
        assert_eq!(
            kicked.driven.map(|v| v.y),
            clean.driven.map(|v| v.y),
            "the shove changed where the sequence ended"
        );
    }

    /// A results screen is, and this must not fight it.
    #[test]
    fn a_tallied_sequence_holds_still_forever() {
        let p = pole();
        let body = ae::Vec2::new(1234.0, 400.0);
        let mut seq = FlagSequence {
            phase: FlagPhase::Tallied { score: 800 },
            driven: Some(body),
        };
        for _ in 0..600 {
            assert_eq!(
                step_flag_sequence(&mut seq, &p, body, HALF_H, DT).map(|d| d.pos),
                Some(body)
            );
        }
        assert_eq!(seq.score(), Some(800));
    }

    /// A grab at the very top of the pole does not slide UP. The snap clamps to the
    /// top, because a body that reached the pole from a platform above it would
    /// otherwise start its slide from off-screen.
    #[test]
    fn a_grab_from_above_the_top_starts_at_the_top() {
        let p = pole();
        let mut seq = FlagSequence::default();
        let at = step_flag_sequence(&mut seq, &p, ae::Vec2::new(1000.0, 20.0), HALF_H, DT)
            .unwrap()
            .pos;
        assert_eq!(at.y, p.top_y);
        assert_eq!(seq.score(), Some(5000));
    }
}

/// Drive the sequence, and the body with it.
///
/// Content-side and engine-free: it reads the controlled body's position, hands it
/// to [`step_flag_sequence`], and writes back whatever comes out. The body is
/// DRIVEN, not frozen — gravity and the movement kernel still run, and the
/// sequence simply overrules them. Blanking the control frame is what stops a
/// player mashing jump from fighting the slide.
pub fn run_flag_sequence(
    time: Res<ambition_platformer2d::time::WorldTime>,
    pole: Option<Res<FlagPole>>,
    subject: Option<Res<ambition_platformer2d::platformer::markers::ControlledSubject>>,
    mut commands: Commands,
    mut sequences: Query<&mut FlagSequence>,
    mut bodies: Query<&mut ae::BodyKinematics>,
    mut holds: Query<&mut ambition_platformer2d::characters::control::ControlHolds>,
    // Her body mode, so the pole can say she is CLIMBING and let the animation
    // picker choose the clip.
    mut modes: Query<&mut ae::BodyModeState>,
) {
    let (Some(pole), Some(entity)) = (pole, subject.and_then(|s| s.0)) else {
        return;
    };
    let Ok(mut sequence) = sequences.single_mut() else {
        return;
    };
    let Ok(mut kin) = bodies.get_mut(entity) else {
        return;
    };
    // The pole owns the body from the grab to the tally. The engine's `ScriptedControl` blanks at
    // the one point where it is observable, and takes her out of the pickup pass while she is on
    // the pole.
    if matches!(sequence.phase, FlagPhase::Idle) {
        // the POLE's hold and nobody else's: off the pole is not the same
        // fact as free.
        ambition_platformer2d::characters::control::release_control_hold(
            &mut commands,
            entity,
            holds.get_mut(entity).ok().as_deref_mut(),
            ambition_platformer2d::characters::control::ControlHold::Sequence,
        );
    } else {
        ambition_platformer2d::characters::control::claim_control_hold(
            &mut commands,
            entity,
            ambition_platformer2d::characters::control::ControlHold::Sequence,
        );
    }

    // Her LIVE half-height, so the slide plants the form she is actually wearing:
    // small and tall Mary-O stand on the same ground line, and the tall one is
    // not left with her knees in it.
    let half_height = kin.size.y * 0.5;
    let Some(drive) =
        step_flag_sequence(&mut sequence, &pole, kin.pos, half_height, time.scaled_dt)
    else {
        return;
    };
    // The scripted end-of-level slide is an external kinematic constraint
    // (ADR 0024 authority): the sequence owns the pose while it plays.
    //
    // AND THE VELOCITY IT IMPOSES IS THE TRUE ONE. This passed
    // `Vec2::ZERO`, so every reader of her motion was told she was standing
    // still while her position jumped — which is why she appeared to translate
    // rather than slide and walk. `constrain_body_pose` has taken an imposed
    // velocity all along; the sequence simply was not telling it anything.
    ae::movement::constrain_body_pose(&mut kin, drive.pos, drive.vel);
    // Walking off is to the RIGHT, so she should be looking that way.
    if drive.vel.x.abs() > f32::EPSILON {
        kin.facing = drive.vel.x.signum();
    }
    // being on the pole is a BODY MODE, not a clip. The animation picker
    // turns `Climbing` into the climb clip on its own — the same road a ladder
    // takes — so the slide animates because the body says what it is doing, and
    // this file names no clip at all.
    if let Ok(mut mode) = modes.get_mut(entity) {
        let wanted = if drive.on_pole {
            ae::BodyMode::Climbing
        } else {
            ae::BodyMode::Standing
        };
        if mode.body_mode != wanted {
            mode.body_mode = wanted;
        }
    }
}

/// This sequence's claim on the encounter layer's priority music tier.
const VICTORY_MUSIC_OWNER: &str = "mary_o_flag";

/// Clearing the course has its own music.
///
/// The same priority-tier claim her death uses, for the same reason: it is the
/// one tier that outranks the room's own theme, and claiming rather than
/// assigning means the boss system's every-frame release cannot silence it.
///
/// The track is authorized by Mary-O's audio fragment
/// ([`crate::provider::MARY_O_VICTORY_MUSIC_TRACK`]); under provider-relative
/// playback an undeclared id is gated to silence however loudly it is requested.
pub fn play_victory_music(
    sequences: Query<&FlagSequence>,
    music: Option<
        ambition_platformer2d::platformer::lifecycle::SessionWorldMut<
            ambition_platformer2d::encounter::EncounterMusicRequest,
        >,
    >,
) {
    let (Ok(sequence), Some(mut music)) = (sequences.single(), music) else {
        return;
    };
    if matches!(sequence.phase, FlagPhase::Idle) {
        music.release_priority(VICTORY_MUSIC_OWNER);
    } else {
        music.claim_priority(
            VICTORY_MUSIC_OWNER,
            crate::provider::MARY_O_VICTORY_MUSIC_TRACK,
        );
    }
}
