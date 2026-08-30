//! ⭐⭐ HOW YOU MAKE A FIGHTER PERFORM A MOVE, in one place.
//!
//! This knowledge was proved inside `moveset_takes` over many wrong answers, and
//! every one of them is a rule here rather than a comment:
//!
//! - a full stick is a SMASH, so a tilt drives `0.65` (a directional take
//!   recorded the smash for every verb until this was measured);
//! - an aerial pressed from the ground reaches the GROUNDED chain, so an aerial
//!   verb must get airborne first and be confirmed airborne, not merely jumped;
//! - a back-air driven on the tick the stick reverses resolves as FORWARD,
//!   because the gesture resolver reads `-facing` during a turnaround — so a
//!   horizontal aim settles before the press;
//! - `special_pressed` is a rising EDGE: holding it true is a press every tick;
//! - and a charge move only pays out when the button comes UP.
//!
//! ⭐⭐ A LIBRARY MODULE SINCE 2026-08-29, AND ITS OWN DOC ASKED FOR THIS. It
//! lived in `game/ambition_app_tools/src/bin/support/` and both `moveset_takes`
//! and `moveset_render` `#[path]`-INCLUDED it — two copies compiled from one
//! file, each using a subset, so everything the other consumer needed was dead
//! code in both. The note here said *"if it ever becomes real reusable domain
//! API it belongs in a domain crate, and moving it then is a rename"*. It did,
//! and it was.
//!
//! ⛔ IT LANDED IN THE HARNESS AND NOT IN A NEW CRATE because its dependencies
//! are exactly the harness's: `ambition_platformer2d` and `bevy`, and nothing
//! from the product shell. A driver that composes a smash match supplies the
//! composition; this only knows how to press.

use ambition_platformer2d::sim::ControlFrame;
use bevy::prelude::App;

/// A full stick is a SMASH. A tilt has to ask for less.
pub const TILT_AXIS: f32 = 0.65;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Button {
    Attack,
    Smash,
    Special,
    Grab,
    Taunt,
}

/// One press, as the genre spells it.
#[derive(Clone, Copy, Debug)]
pub struct Verb {
    /// The repertoire verb this drives, which is the key a viewer files it under.
    pub verb: &'static str,
    pub label: &'static str,
    pub axis_x: f32,
    pub axis_y: f32,
    pub button: Button,
    /// Jump first, and CONFIRM the apex. An aerial pressed from the ground
    /// reaches the grounded chain instead and performs the wrong move.
    pub airborne: bool,
}

impl Verb {
    /// The control frame for this verb, mirrored by the body's facing.
    ///
    /// ⛔ `edge` IS THE PRESS TICK. Every button here is a rising edge; holding
    /// the flag true is a press every tick, which re-triggers instead of holding.
    pub fn frame(&self, edge: bool, facing: f32) -> ControlFrame {
        // ⛔ A SMASH REACHES FULL STICK; a tilt must not. This is the measurement
        // that cost a whole recording: every directional take reported the SMASH
        // until the tilt stopped driving `1.0`.
        let reach = if self.button == Button::Smash {
            1.0
        } else {
            TILT_AXIS
        };
        let mut frame = ControlFrame {
            axis_x: self.axis_x * reach * facing.signum(),
            axis_y: self.axis_y * reach,
            ..Default::default()
        };
        match self.button {
            Button::Attack => {
                frame.attack_pressed = edge;
                frame.attack_held = true;
            }
            Button::Smash => {
                frame.attack_pressed = edge;
                frame.attack_held = true;
                // ⛔⛔ THE GESTURE THAT TELLS A TILT FROM A SMASH. Without it
                // every "smash" records the TILT, which looks like working data.
                frame.attack_strong_hint = true;
            }
            Button::Special => {
                frame.special_pressed = edge;
                frame.special_held = true;
            }
            Button::Grab => frame.grab_pressed = edge,
            Button::Taunt => frame.taunt_pressed = edge,
        }
        frame
    }
}

/// Look a verb up by its repertoire name.
pub fn verb_named(name: &str) -> Option<&'static Verb> {
    VERBS.iter().find(|v| v.verb == name)
}

/// Every verb this exercise knows how to perform.
///
/// ⚠ CAPTURE-STATE MOVES ARE ABSENT ON PURPOSE. A pummel or a throw needs a
/// GRABBED OPPONENT, which is a state this exercise cannot establish — and
/// listing them here would promise a capture the driver would then record as a
/// mismatch. They belong here when a scenario can set that state up.
pub const VERBS: &[Verb] = &[
    Verb {
        verb: "attack",
        label: "Jab",
        axis_x: 0.0,
        axis_y: 0.0,
        button: Button::Attack,
        airborne: false,
    },
    Verb {
        verb: "attack_forward",
        label: "F-tilt",
        axis_x: 1.0,
        axis_y: 0.0,
        button: Button::Attack,
        airborne: false,
    },
    Verb {
        verb: "attack_up",
        label: "U-tilt",
        axis_x: 0.0,
        axis_y: -1.0,
        button: Button::Attack,
        airborne: false,
    },
    Verb {
        verb: "attack_down",
        label: "D-tilt",
        axis_x: 0.0,
        axis_y: 1.0,
        button: Button::Attack,
        airborne: false,
    },
    Verb {
        verb: "smash_forward",
        label: "F-smash",
        axis_x: 1.0,
        axis_y: 0.0,
        button: Button::Smash,
        airborne: false,
    },
    Verb {
        verb: "smash_up",
        label: "U-smash",
        axis_x: 0.0,
        axis_y: -1.0,
        button: Button::Smash,
        airborne: false,
    },
    Verb {
        verb: "smash_down",
        label: "D-smash",
        axis_x: 0.0,
        axis_y: 1.0,
        button: Button::Smash,
        airborne: false,
    },
    Verb {
        verb: "attack_air",
        label: "N-air",
        axis_x: 0.0,
        axis_y: 0.0,
        button: Button::Attack,
        airborne: true,
    },
    Verb {
        verb: "attack_air_forward",
        label: "F-air",
        axis_x: 1.0,
        axis_y: 0.0,
        button: Button::Attack,
        airborne: true,
    },
    Verb {
        verb: "attack_air_back",
        label: "B-air",
        axis_x: -1.0,
        axis_y: 0.0,
        button: Button::Attack,
        airborne: true,
    },
    Verb {
        verb: "attack_air_up",
        label: "U-air",
        axis_x: 0.0,
        axis_y: -1.0,
        button: Button::Attack,
        airborne: true,
    },
    Verb {
        verb: "attack_air_down",
        label: "D-air",
        axis_x: 0.0,
        axis_y: 1.0,
        button: Button::Attack,
        airborne: true,
    },
    Verb {
        verb: "special",
        label: "Neutral B",
        axis_x: 0.0,
        axis_y: 0.0,
        button: Button::Special,
        airborne: false,
    },
    Verb {
        verb: "special_forward",
        label: "Side B",
        axis_x: 1.0,
        axis_y: 0.0,
        button: Button::Special,
        airborne: false,
    },
    // ⭐ THE UP-B IS RECORDED FROM THE AIR, which is the only place it is the
    // move Jon is asking about. A grounded up-B answers the same press and shows
    // none of the recovery.
    Verb {
        verb: "special_up",
        label: "Up B (airborne)",
        axis_x: 0.0,
        axis_y: -1.0,
        button: Button::Special,
        airborne: true,
    },
    Verb {
        verb: "special_down",
        label: "Down B",
        axis_x: 0.0,
        axis_y: 1.0,
        button: Button::Special,
        airborne: false,
    },
    Verb {
        verb: "special_air_down",
        label: "Down B (air)",
        axis_x: 0.0,
        axis_y: 1.0,
        button: Button::Special,
        airborne: true,
    },
    Verb {
        verb: "grab",
        label: "Grab",
        axis_x: 0.0,
        axis_y: 0.0,
        button: Button::Grab,
        airborne: false,
    },
    Verb {
        verb: "taunt",
        label: "Taunt",
        axis_x: 0.0,
        axis_y: 0.0,
        button: Button::Taunt,
        airborne: false,
    },
];

/// The move a verb is BOUND to on this fighter, as the composed host prepared it.
///
/// ⭐⭐ THE HOST'S ANSWER, NOT A SECOND MAPPING. A press is a REQUEST; the engine
/// decides. Without this a driver can only ask "did ANY move play", which calls
/// the known back-air-resolves-as-forward-air case a success and files the
/// forward air under `attack_air_back`.
pub fn intended_move(app: &mut App, character: &str, verb: &str) -> Option<String> {
    app.world()
        .get_resource::<ambition_platformer2d::character::PreparedCharacterRegistry>()
        .and_then(|registry| registry.get(character))
        .and_then(|prepared| prepared.kit.projectable_moveset())
        .and_then(|set| set.verbs.get(verb).cloned())
}

/// How long the exercise HOLDS the button, in simulation ticks.
///
/// ⛔⛔ A CONSTANT, BECAUSE CAPTURE PARAMETERS MUST NOT CHANGE THE MOVE. The
/// renderer held while `shot < frames / 4`, so the hold depended on `--frames`
/// AND `--stride`: 24 frames at stride 2 held ~12 ticks, a 12-frame run held ~6,
/// and the recorder's own exercise holds ~37. Asking for more pictures charged
/// the smash differently, which means the two tools were photographing
/// different moves and neither said so.
pub const HOLD_TICKS: usize = 37;

/// The whole exercise, tick by tick: what to drive on simulation tick `n`.
///
/// ⭐ ONE SCHEDULE, TWO CONSUMERS. `moveset_takes` records every tick of it;
/// `moveset_render` photographs a subset. Which ticks are OBSERVED is the
/// caller's business; what the player does is not.
pub fn action_frame(verb: &Verb, action_tick: usize, facing: f32) -> ControlFrame {
    if action_tick == 0 {
        verb.frame(true, facing)
    } else if action_tick < HOLD_TICKS {
        verb.frame(false, facing)
    } else {
        // A charge move only pays out when the button comes UP.
        ControlFrame::default()
    }
}

/// What seat zero is, in the four facts every driver asks about.
///
/// ⭐⭐ ONE QUERY, AND THE SUBJECT'S ABSENCE IS A CASE. `moveset_takes` read
/// these through its full JSON sampler and `moveset_render` through three
/// separate queries, and both flattened "there is no seat-zero body" into the
/// same answer as "the body is airborne": `settle` accepted a stage with NO
/// FIGHTER ON IT as settled, because `(false, false, false)` is what an empty
/// query and an idle flyer both return. A missing subject is not a state of the
/// subject.
pub struct Subject {
    /// Which way the body points. A directional press resolves against this.
    pub facing: f32,
    /// `Some(on_ground)` from the body's ground state, `None` when it publishes
    /// none — a body with no ground state is not a body that is in the air.
    pub grounded: Option<bool>,
    pub playing: Option<String>,
    pub riding: bool,
    /// Whether this body's LOCOMOTION allows it to come to rest in the air.
    ///
    /// ⛔⛔ THE QUESTION `ever_stood` WAS STANDING IN FOR, and it is not the same
    /// question. That flag answered *"has this body reported ground since this
    /// call to `settle` began"* — an observation about the last few ticks —
    /// while the thing that decides whether an airborne fighter is settled or
    /// still falling is a CAPABILITY the body carries. A flyer at rest in the
    /// air is settled; an ordinary fighter in the air is mid-jump.
    pub flies: bool,
}

/// Seat zero, or `None` when nothing is seated there.
pub fn subject(app: &mut App) -> Option<Subject> {
    let world = app.world_mut();
    let mut q = world.query::<(
        &ambition_platformer2d::actor::MatchSeat,
        &ambition_platformer2d::actor::BodyKinematics,
        Option<&ambition_platformer2d::actor::BodyGroundState>,
        Option<&ambition_platformer2d::actor::MovePlayback>,
        Option<&ambition_platformer2d::actor::RidingOn>,
        Option<&ambition_platformer2d::actor::BodyFlightState>,
    )>();
    q.iter(world)
        .find(|(seat, ..)| seat.0 == 0)
        .map(|(_, kin, ground, playing, riding, flight)| Subject {
            facing: kin.facing,
            grounded: ground.map(|g| g.on_ground),
            playing: playing.map(|p| p.spec.id.clone()),
            riding: riding.is_some(),
            flies: flight.is_some_and(|f| f.fly_enabled),
        })
}

/// Seat 0's facing, the axis a directional press is resolved against.
pub fn facing_of(app: &mut App) -> f32 {
    subject(app).map_or(1.0, |s| s.facing)
}

/// Is seat 0 off the ground? False when it is standing, has no ground state, or
/// is not there at all — every one of which is a reason not to press an aerial.
pub fn airborne(app: &mut App) -> bool {
    subject(app).and_then(|s| s.grounded) == Some(false)
}

/// Which move seat 0 is playing right now, if any.
pub fn playing_move(app: &mut App) -> Option<String> {
    subject(app).and_then(|s| s.playing)
}

/// Drive one control frame and advance one simulation tick.
pub fn step(app: &mut App, frame: ControlFrame) {
    ambition_platformer2d::sim::drive_control_frame(app.world_mut(), frame);
    app.update();
}

/// Bring the fighter to the posture this verb needs, and say whether it worked.
///
/// ⭐⭐ SHARED, BECAUSE THE TWO DRIVERS HAD DIFFERENT ALGORITHMS. The recorder
/// jumped and POLLED the ground state, giving up after forty ticks; the renderer
/// jumped and counted ten. So "perform a back air" meant two things, only one of
/// them was ever tested, and the two tools could photograph different moves. This
/// is the recorder's, which is the one the measurements below were made against.
///
/// ⛔ AIRBORNE IS CONFIRMED, NOT ASSUMED. An aerial pressed from the ground
/// reaches the GROUNDED chain and performs a different move — a take reported
/// `attack_air_down` as `smash_down` until this asked `BodyGroundState` instead
/// of counting frames after a jump.
///
/// ⛔⛔ AND IT IS CONFIRMED AFTER THE AIM, NOT BEFORE IT. Jumping and then doing
/// anything else is a different claim: a short hop or a fast-fall put the body
/// back on the floor during the aim settle, and the take recorded the grounded
/// move under the aerial's name. The takeoff and the aim are one loop that ends
/// only when the body is both airborne AND pointing the right way.
///
/// ⛔ THE AIM IS HORIZONTAL ONLY. A back-air driven on the tick the stick
/// reverses resolves FORWARD, because the gesture resolver reads `-facing` while
/// a turnaround runs. Holding DOWN for the same settle would fast-fall.
pub fn prepare(app: &mut App, verb: &Verb) -> bool {
    if !verb.airborne {
        return true;
    }
    for _attempt in 0..3 {
        if !take_off(app) {
            continue;
        }
        if verb.axis_x != 0.0 {
            for _ in 0..AIM_TICKS {
                let aim = facing_of(app);
                step(
                    app,
                    ControlFrame {
                        axis_x: verb.axis_x * TILT_AXIS * aim.signum(),
                        ..Default::default()
                    },
                );
            }
        }
        if airborne(app) {
            return true;
        }
    }
    false
}

/// How long a horizontal aim settles before the press.
const AIM_TICKS: usize = 8;

/// Jump, and wait until the world says the body has left the ground.
///
/// ⛔ ASK, DO NOT COUNT. A fixed wait recorded `attack_air_down` as `smash_down`
/// and `attack_air_up` as nothing at all, because the press landed on a body the
/// engine still called grounded and the directional chain walked past every
/// aerial.
fn take_off(app: &mut App) -> bool {
    if airborne(app) {
        return true;
    }
    step(
        app,
        ControlFrame {
            jump_pressed: true,
            jump_held: true,
            ..Default::default()
        },
    );
    for _ in 0..40 {
        step(
            app,
            ControlFrame {
                jump_held: true,
                ..Default::default()
            },
        );
        if airborne(app) {
            return true;
        }
    }
    false
}

/// The longest anything will wait for the stage to go quiet before a press.
///
/// ⛔⛔ A FIXED SETTLE IS NOT A SETTLE. Forty-five ticks was less than the
/// admiral's forward smash owes, so `smash_up`, `smash_down` and `special_up`
/// each landed inside the previous move's recovery, were dropped, and were
/// reported as moves that produced nothing — three false findings from one
/// constant. The condition is "the body is idle and standing", which the world
/// already publishes, so the wait ASKS instead of counting.
/// ⛔ ABOVE THE LONGEST RIDE. A 240-tick limit is four seconds and the shark
/// carries its rider for five, so the take after the up-B started while the
/// admiral was still airborne on a mount and reported two moves as producing
/// nothing. A settle that gives up before the previous take finishes is a
/// settle that manufactures findings.
pub const SETTLE_LIMIT: usize = 480;

/// Is the stage at rest, given what seat zero reports?
///
/// Separated from the stepping loop because the whole content of `settle` is
/// this decision, and it is answerable without a composed host.
///
/// ⛔⛔ `None` — NO SEAT ZERO — IS NOT SETTLED. This used to be
/// `(false, false, false)`, the same triple an idle airborne fighter produces,
/// so a recording with no subject read as ready on its first iteration and
/// every take after it described a stage with nobody on it.
///
/// ⛔⛔ NOT EVERY FIGHTER CAN STAND. `player_robot_v3` can FLY, and a flying body
/// is never grounded by construction (`integration.rs`: *"a flying body is never
/// grounded — the collision sweep can still find support under a hovering
/// flyer"*). A settle that demanded `grounded` never succeeded for it: every
/// take re-seated, timed out, and started from whatever the last one left
/// behind — which is why the robot's grounded forward tilt was recorded as its
/// FORWARD AIR and both of its specials as producing nothing.
pub fn settled_now(subject: Option<&Subject>) -> bool {
    let Some(subject) = subject else {
        return false;
    };
    if subject.playing.is_some() || subject.riding {
        return false;
    }
    // Grounded is rest for anybody. Airborne is rest only for a body whose
    // locomotion says so, which is the flying Robot this rule was widened for.
    subject.grounded.unwrap_or(false) || subject.flies
}

/// Step until the stage goes quiet, and say whether it did.
///
/// ⭐⭐ SHARED, BECAUSE THE RENDERER DID NOT SETTLE AT ALL and its `prepared`
/// flag lied because of it: `session_is_active` is true while the cast is still
/// DROPPING IN, so a readiness loop could hand `prepare` a FALLING body, which
/// `take_off` calls airborne and returns from without ever jumping. The manifest
/// said the posture was established and every photograph showed a GROUNDED up-B.
///
/// ⛔⛤ IT DOES NOT BUILD A FRAME TO READ FOUR FACTS. The recorder's version
/// called its full JSON sampler — every body, every hitbox, every projectile,
/// serialised and thrown away — up to 480 times a take, for three booleans.
pub fn settle(app: &mut App) -> bool {
    for _ in 0..SETTLE_LIMIT {
        step(app, ControlFrame::default());
        // Keep stepping while there is no subject — a re-seat lands through
        // deferred commands and the body may still be arriving.
        if settled_now(subject(app).as_ref()) {
            return true;
        }
    }
    false
}

/// What a driven verb actually produced.
///
/// ⛔⛔ ONE VOCABULARY, BECAUSE THE TWO TOOLS DISAGREED ABOUT THE SAME PRESS. The
/// recorder said `intended.is_none_or(|id| moves.contains(id))`, so an UNBOUND
/// verb was a success; the renderer said an unbound verb was a mismatch. The
/// same input therefore read as "reached" in the diagnostic panel and "MISMATCH"
/// in the engine panel beside it. Four answers, and none of them collapse:
/// unbound is not success, and a wrong posture is not a valid render.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Outcome {
    /// The verb is bound and that move played.
    Reached,
    /// The verb is bound and the engine played something else, or nothing.
    Missed,
    /// This fighter binds no move to this verb. The directional chain answered
    /// the press and whatever it reached is a fact about the fighter, not a
    /// mismatch — but it is not the requested move either.
    Unbound,
    /// The posture the verb needs could not be established, so whatever came out
    /// answers a DIFFERENT button. An aerial pressed from the ground reaches the
    /// grounded chain, and for a recovery special that is the whole subject.
    NotPrepared,
}

impl Outcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Outcome::Reached => "reached",
            Outcome::Missed => "missed",
            Outcome::Unbound => "unbound",
            Outcome::NotPrepared => "not_prepared",
        }
    }

    /// Did this press produce the move it asked for, in the posture it asked for?
    pub fn reached(self) -> bool {
        self == Outcome::Reached
    }
}

/// Judge one exercise. `observed` is every move the subject played during it.
///
/// ⛔ PREPARATION IS JUDGED FIRST. A grounded up-B and an airborne one can be
/// the same move id, so "the intended move appeared" cannot tell them apart —
/// and the airborne one is the only one anybody opens this view to look at.
pub fn outcome<S: std::borrow::Borrow<str> + Ord>(
    prepared: bool,
    intended: Option<&str>,
    observed: &std::collections::BTreeSet<S>,
) -> Outcome {
    if !prepared {
        return Outcome::NotPrepared;
    }
    match intended {
        None => Outcome::Unbound,
        Some(want) => {
            if observed.iter().any(|id| id.borrow() == want) {
                Outcome::Reached
            } else {
                Outcome::Missed
            }
        }
    }
}

/// Has an exercise that stopped at `last_action_tick` passed the release?
///
/// ⛔⛔ A SHORT OBSERVATION IS NOT A SHORT MOVE, AND IT MUST NOT CLAIM TO BE ONE.
/// `--frames 4 --stride 2` executes action ticks 0..8 and then exits, so its
/// manifest reporting `hold_ticks: 37` says what the schedule IS and proves
/// nothing about what this run DID. A charge that is never released is a charge
/// whose payout was never photographed.
pub fn released_by(last_action_tick: usize) -> bool {
    last_action_tick >= HOLD_TICKS
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn resting(grounded: bool, playing: bool, riding: bool, flies: bool) -> Subject {
        Subject {
            facing: 1.0,
            grounded: Some(grounded),
            playing: playing.then(|| "some_move".to_string()),
            riding,
            flies,
        }
    }

    /// ⛔⛔ AN EMPTY STAGE IS NOT A QUIET ONE.
    #[test]
    fn a_stage_with_nobody_on_it_never_settles() {
        assert!(!settled_now(None));
    }

    /// ⛔ AND AN ORDINARY FIGHTER IN THE AIR IS MID-JUMP, not at rest. The old
    /// `ever_stood` flag asked *"has this body reported ground since this call
    /// began"* — an observation about the last few ticks — where the question is
    /// a CAPABILITY the body carries.
    #[test]
    fn an_ordinary_fighter_settles_only_once_it_is_standing() {
        assert!(!settled_now(Some(&resting(false, false, false, false))));
        assert!(settled_now(Some(&resting(true, false, false, false))));
    }

    /// ⭐ AND A FLYER AT REST IN THE AIR IS SETTLED, which is the case the loop
    /// was widened for and the only one the old rule got right.
    #[test]
    fn a_flying_fighter_settles_without_ever_touching_the_floor() {
        assert!(settled_now(Some(&resting(false, false, false, true))));
    }

    /// A move still playing or a body still riding is busy either way.
    #[test]
    fn a_busy_body_is_not_settled_however_it_moves() {
        for flies in [false, true] {
            assert!(!settled_now(Some(&resting(true, true, false, flies))));
            assert!(!settled_now(Some(&resting(true, false, true, flies))));
        }
    }

    fn set(ids: &[&str]) -> BTreeSet<String> {
        ids.iter().map(|s| s.to_string()).collect()
    }

    /// ⛔⛔ CAPTURE PARAMETERS CHOOSE WHAT IS OBSERVED, NEVER WHAT IS DRIVEN. The
    /// renderer held while `shot < frames / 4` and the recorder while
    /// `tick < TAKE_TICKS / 4`, so asking for more pictures charged a smash
    /// differently and a 12-frame run performed a different move from a 24-frame
    /// one. Nothing in this signature can see how many pictures were asked for,
    /// and this is the test that keeps it that way.
    #[test]
    fn the_schedule_depends_only_on_the_action_tick() {
        let verb = verb_named("smash_forward").expect("the table has an f-smash");
        for tick in 0..(HOLD_TICKS + 8) {
            let a = action_frame(verb, tick, 1.0);
            let b = action_frame(verb, tick, 1.0);
            assert_eq!(a.attack_pressed, b.attack_pressed);
            assert_eq!(a.attack_held, b.attack_held);
            assert_eq!(a.axis_x, b.axis_x);
        }
    }

    /// The press is a rising EDGE on tick zero and a HOLD after it; holding the
    /// flag true is a press every tick, which re-triggers instead of charging.
    #[test]
    fn the_edge_is_the_first_tick_and_the_release_is_the_hold_ticks_one() {
        let verb = verb_named("smash_forward").expect("the table has an f-smash");
        assert!(
            action_frame(verb, 0, 1.0).attack_pressed,
            "tick 0 is the press"
        );
        assert!(
            !action_frame(verb, 1, 1.0).attack_pressed,
            "tick 1 is a hold, not a press"
        );
        assert!(
            action_frame(verb, HOLD_TICKS - 1, 1.0).attack_held,
            "still charging"
        );
        assert!(
            !action_frame(verb, HOLD_TICKS, 1.0).attack_held,
            "a charge pays out when the button comes UP, and that tick is HOLD_TICKS"
        );
    }

    /// ⛔⛤ AN UNBOUND VERB IS NOT A SUCCESS. `moveset_takes` said
    /// `intended.is_none_or(|id| moves.contains(id))` while `moveset_render`
    /// called the same press a mismatch, so the diagnostic panel and the engine
    /// panel beside it could disagree about one input.
    #[test]
    fn an_unbound_verb_is_its_own_answer_and_not_a_reached_one() {
        let played = set(&["heave_to"]);
        let verdict = outcome(true, None, &played);
        assert_eq!(verdict, Outcome::Unbound);
        assert!(
            !verdict.reached(),
            "unbound must never read as the requested move"
        );
    }

    /// ⛔⛔ A GROUNDED UP-B AND AN AIRBORNE ONE CAN BE THE SAME MOVE ID, so "the
    /// intended move appeared" cannot tell them apart — and the airborne one is
    /// the only one this campaign exists to look at.
    #[test]
    fn a_failed_preparation_is_not_a_valid_render_even_when_the_move_appears() {
        let played = set(&["call_the_shark"]);
        assert_eq!(
            outcome(true, Some("call_the_shark"), &played),
            Outcome::Reached
        );
        assert_eq!(
            outcome(false, Some("call_the_shark"), &played),
            Outcome::NotPrepared,
            "the move came out, in the wrong posture, answering a different button"
        );
        assert!(!outcome(false, Some("call_the_shark"), &played).reached());
    }

    #[test]
    fn a_bound_verb_that_played_something_else_is_a_miss() {
        assert_eq!(
            outcome(true, Some("attack_air_back"), &set(&["attack_air_forward"])),
            Outcome::Missed
        );
        assert_eq!(outcome(true, Some("jab"), &set(&[])), Outcome::Missed);
    }

    /// ⛔ A SHORT OBSERVATION HORIZON IS NOT A SHORT MOVE. `--frames 4 --stride 2`
    /// executes action ticks 0..8 and exits, so it never crosses the release.
    #[test]
    fn a_run_that_stops_before_the_release_says_so() {
        assert!(
            !released_by(7),
            "four frames at stride two end at action tick 7"
        );
        assert!(!released_by(HOLD_TICKS - 1));
        assert!(released_by(HOLD_TICKS));
        assert!(released_by(46), "24 frames at stride 2 crosses it");
    }

    /// Every verb the two drivers advertise can be looked up by name, so a
    /// browser request and a command line agree about what exists.
    #[test]
    fn every_advertised_verb_resolves() {
        for verb in VERBS {
            assert!(
                verb_named(verb.verb).is_some(),
                "{} is not findable",
                verb.verb
            );
        }
        assert!(
            verb_named("pummel").is_none(),
            "capture-state verbs are absent on purpose"
        );
    }
}
