//! **M3 — the flagpole sequence.**
//!
//! `docs/planning/demos/super-mary-o.md`: *"M3 level-end sequencing: flagpole grab
//! → slide → walk-off → score tally."*
//!
//! Content-side, and it adds **zero engine code**. The whole sequence is a state
//! machine over a clock plus one authored geometry fact (where the pole is), and
//! the only thing it does to the body is write its position and suppress its
//! controls — both of which any content plugin may do.
//!
//! ## Why the score is computed from the GRAB, not from the slide
//!
//! In the game this pays homage to, the points depend on how high up the pole you
//! caught it. That height is a fact about the moment of contact, and the slide is a
//! celebration. Computing the score from the *live* body position would let a
//! player who grabbed high and slid fast score differently from one who grabbed
//! high and slid slow, which is a bug that reads as physics.
//!
//! ## Why the player is not "frozen"
//!
//! A frozen body is still a body: gravity keeps pulling, the movement kernel keeps
//! resolving, and a well-placed enemy could still hit it. The sequence takes the
//! body's POSITION each tick — it drives, rather than pauses. That is also what
//! makes it testable without a physics step.

use ambition::engine_core as ae;
use bevy::prelude::*;

/// Where the pole is, and how tall. Mirrors the authored `goal_pole` block so the
/// sequence never has to search the world for it — the level knows.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct FlagPole {
    /// World x of the pole's center.
    pub x: f32,
    /// World y of the pole's TOP (`+y` is down, so this is the small number).
    pub top_y: f32,
    /// World y of the pole's base, where the slide ends.
    pub base_y: f32,
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

    /// How wide a band around the pole counts as touching it. A pole is half a tile
    /// wide; a body is one tile. Half a tile of slop makes the grab feel like the
    /// game it is imitating rather than like a hitbox test.
    pub const GRAB_HALF_WIDTH: f32 = 12.0;
}

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
    /// **This is what makes the sequence immune to system ordering.** If each tick
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

/// **The whole sequence, as a pure function of `(state, pole, body, dt)`.**
///
/// Returns where the body should be this tick. `None` in `Idle` — the player is
/// still playing, and the sequence has no opinion about where they are.
pub fn step_flag_sequence(
    seq: &mut FlagSequence,
    pole: &FlagPole,
    body: ae::Vec2,
    dt: f32,
) -> Option<ae::Vec2> {
    // Once the sequence is driving, the body it is driving is the one IT last put
    // down, not whatever the physics step left behind.
    let body = seq.driven.unwrap_or(body);
    let out = step_phase(seq, pole, body, dt);
    seq.driven = out;
    out
}

fn step_phase(
    seq: &mut FlagSequence,
    pole: &FlagPole,
    body: ae::Vec2,
    dt: f32,
) -> Option<ae::Vec2> {
    match seq.phase {
        FlagPhase::Idle => {
            if (body.x - pole.x).abs() > FlagPole::GRAB_HALF_WIDTH || body.y > pole.base_y {
                return None;
            }
            // The score is a fact about the moment of contact. Everything after
            // this is a celebration.
            seq.phase = FlagPhase::Sliding {
                score: flag_score(pole.grab_height(body.y)),
            };
            // Snap onto the pole so the slide is straight.
            Some(ae::Vec2::new(pole.x, body.y.max(pole.top_y)))
        }
        FlagPhase::Sliding { score } => {
            let y = body.y + SLIDE_SPEED * dt;
            if y >= pole.base_y {
                seq.phase = FlagPhase::WalkingOff {
                    score,
                    remaining: WALK_OFF_PX,
                };
                return Some(ae::Vec2::new(pole.x, pole.base_y));
            }
            Some(ae::Vec2::new(pole.x, y))
        }
        FlagPhase::WalkingOff { score, remaining } => {
            let step = WALK_OFF_SPEED * dt;
            if step >= remaining {
                seq.phase = FlagPhase::Tallied { score };
                return Some(ae::Vec2::new(body.x + remaining, body.y));
            }
            seq.phase = FlagPhase::WalkingOff {
                score,
                remaining: remaining - step,
            };
            Some(ae::Vec2::new(body.x + step, body.y))
        }
        // Done. The body stays where it is; a results screen is M4's.
        FlagPhase::Tallied { .. } => Some(body),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 1.0 / 60.0;

    fn pole() -> FlagPole {
        FlagPole {
            x: 1000.0,
            top_y: 100.0,
            base_y: 400.0,
        }
    }

    fn run_until_tallied(seq: &mut FlagSequence, pole: &FlagPole, start: ae::Vec2) -> u32 {
        let mut body = start;
        for _ in 0..2000 {
            if let Some(next) = step_flag_sequence(seq, pole, body, DT) {
                body = next;
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

    /// The pole is not touched by walking near it, and not by passing under its base.
    #[test]
    fn the_sequence_only_starts_at_the_pole() {
        let p = pole();
        let mut seq = FlagSequence::default();
        assert_eq!(
            step_flag_sequence(&mut seq, &p, ae::Vec2::new(900.0, 300.0), DT),
            None
        );
        assert_eq!(seq.phase, FlagPhase::Idle);

        // Below the base: the player ran past the pole on the ground behind it.
        assert_eq!(
            step_flag_sequence(&mut seq, &p, ae::Vec2::new(1000.0, 500.0), DT),
            None
        );
        assert_eq!(seq.phase, FlagPhase::Idle);

        // On it.
        assert!(step_flag_sequence(&mut seq, &p, ae::Vec2::new(1005.0, 300.0), DT).is_some());
        assert!(seq.active());
    }

    /// **The score is a fact about the GRAB.** A slow slide and a fast one from the
    /// same height pay the same, because the score was decided on contact.
    #[test]
    fn the_score_is_decided_at_the_moment_of_contact() {
        let p = pole();
        let mut seq = FlagSequence::default();
        step_flag_sequence(&mut seq, &p, ae::Vec2::new(1000.0, 120.0), DT);
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
            if let Some(next) = step_flag_sequence(&mut seq, &p, body, DT) {
                body = next;
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
        body = step_flag_sequence(&mut seq, &p, body, DT).unwrap();
        assert_eq!(body.x, p.x);

        while matches!(seq.phase, FlagPhase::Sliding { .. }) {
            body = step_flag_sequence(&mut seq, &p, body, DT).unwrap();
            assert_eq!(body.x, p.x, "the slide never drifts sideways");
        }
        assert_eq!(body.y, p.base_y, "it stops at the base, not past it");

        let walk_start = body.x;
        while matches!(seq.phase, FlagPhase::WalkingOff { .. }) {
            body = step_flag_sequence(&mut seq, &p, body, DT).unwrap();
            assert_eq!(body.y, p.base_y, "the walk-off never leaves the ground");
        }
        assert!(
            (body.x - walk_start - WALK_OFF_PX).abs() < 0.001,
            "walked {} px, expected {WALK_OFF_PX}",
            body.x - walk_start
        );
    }

    /// Once tallied, the sequence is inert: it holds the body and changes nothing.
    /// **Why [`FlagSequence::driven`] exists.** Once the flag is grabbed, a physics
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
            if let Some(next) = step_flag_sequence(&mut kicked, &p, body, DT) {
                // Physics runs after us and shoves the body a tile down and right.
                body = next + ae::Vec2::new(16.0, 16.0);
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

    /// A results screen is M4's, and this must not fight it.
    #[test]
    fn a_tallied_sequence_holds_still_forever() {
        let p = pole();
        let body = ae::Vec2::new(1234.0, 400.0);
        let mut seq = FlagSequence {
            phase: FlagPhase::Tallied { score: 800 },
            driven: Some(body),
        };
        for _ in 0..600 {
            assert_eq!(step_flag_sequence(&mut seq, &p, body, DT), Some(body));
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
        let at = step_flag_sequence(&mut seq, &p, ae::Vec2::new(1000.0, 20.0), DT).unwrap();
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
    time: Res<ambition::time::WorldTime>,
    pole: Option<Res<FlagPole>>,
    subject: Option<Res<ambition::platformer::markers::ControlledSubject>>,
    mut sequences: Query<&mut FlagSequence>,
    mut bodies: Query<(
        &mut ae::BodyKinematics,
        &mut ambition::characters::brain::ActorControl,
    )>,
) {
    let (Some(pole), Some(entity)) = (pole, subject.and_then(|s| s.0)) else {
        return;
    };
    let Ok(mut sequence) = sequences.single_mut() else {
        return;
    };
    let Ok((mut kin, mut control)) = bodies.get_mut(entity) else {
        return;
    };

    let Some(next) = step_flag_sequence(&mut sequence, &pole, kin.pos, time.scaled_dt) else {
        return;
    };
    kin.pos = next;
    kin.vel = ae::Vec2::ZERO;
    control.0 = ambition::characters::actor::control::ActorControlFrame::default();
}
