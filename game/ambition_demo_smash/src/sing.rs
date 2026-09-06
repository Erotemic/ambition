//! Sing: an area that takes the floor away from whoever stood too close.
//!
//! ⭐⭐ NO STATUS SYSTEM WAS WRITTEN FOR THIS. `attack_support`'s
//! `hard_lock_timer` is already a `max()` over named causes of "this body cannot
//! act", and `BodyCombat::sleep_timer` is a fifth one. This module finds the
//! bodies in range and sets that timer; everything downstream — control
//! stripping, the shared decay, the wake a real hit buys — was already there.
//!
//! ⛔ THE SINGER IS NEVER CAUGHT BY THEIR OWN SONG. Not politeness: the move
//! puts everyone else to sleep and then the singer acts, which IS the move. A
//! version that slept its own caster would be a very slow suicide.

use bevy::math::bounding::IntersectsVolume as _;
use bevy::prelude::*;

use ambition_platformer2d::characters::brain::action_set::{ActionRequest, SpecialActionSpec};
use ambition_platformer2d::characters::brain::ActorActionMessage;
use ambition_platformer2d::characters::smash_sleep::{SleepParams, SLEEP};
use ambition_platformer2d::engine_core as ae;

/// Put every eligible body inside the pulse to sleep.
pub fn apply_authored_sleep(
    mut actions: MessageReader<ActorActionMessage>,
    singers: Query<&ae::BodyKinematics>,
    mut victims: Query<(
        Entity,
        &ae::CenteredAabb,
        &mut ambition_platformer2d::characters::actor::BodyCombat,
    )>,
) {
    for message in actions.read() {
        let ActionRequest::Special { spec, params } = &message.request else {
            continue;
        };
        let SpecialActionSpec::Special(key) = spec;
        if key.as_str() != SLEEP {
            continue;
        }
        let params: SleepParams = match params.hydrate() {
            Ok(p) => p,
            Err(err) => {
                warn!("smash sleep params did not hydrate: {err}");
                continue;
            }
        };
        let Ok(kin) = singers.get(message.actor) else {
            continue;
        };
        // ⭐ CENTRED ON THE SINGER AND SYMMETRIC, so the move does not care
        // which way they are facing. A directional sing would be a strike with a
        // status attached, which is a different move and a worse one.
        let reach = ae::CenteredAabb::from_center_size(
            kin.pos,
            ae::Vec2::new(params.half_extents.0 * 2.0, params.half_extents.1 * 2.0),
        )
        .aabb();
        for (body, aabb, mut combat) in &mut victims {
            if body == message.actor {
                continue;
            }
            if !reach.intersects(&aabb.aabb()) {
                continue;
            }
            // ⛔ A FLOOR, NOT AN ADDITION. Two overlapping pulses must not stack
            // into a sleep nobody can wake from; the longer one simply wins.
            combat.sleep_timer = combat.sleep_timer.max(params.duration_s);
        }
    }
}

/// Seconds one credited press burns off a sleep.
///
/// ⭐ CHOSEN AGAINST A HUMAN'S MASH RATE, and the arithmetic is the design. A
/// player mashes roughly ten times a second, so mashing decays a sleep at about
/// `1.0 + 10 * 0.05 = 1.5x` — a 1.4s song holds a struggling fighter for about
/// 0.93s. ⇒ **The mash buys back a third of the punish and cannot beat the
/// clock**, which is the shape the move needs: the singer paid a slow,
/// self-centred, area move for the window and must keep most of it.
///
/// ⛔ A RULESET CONSTANT AND NOT AN AUTHORED FIELD, deliberately. Putting it on
/// `SleepParams` is cheap — the struct already round-trips — and cheap is not a
/// reason. Two sleeps ship (the Performer's song and the Shadow Oni's seal) and
/// neither has asked to be harder or easier to escape than the other. ⇒ The
/// condition that would reopen it, so it can be checked rather than re-argued:
/// **a second move that wants a different escape rate than the first**. Until
/// then a per-move knob is a rollback-carried field on every victim with one
/// setting.
const MASH_SECONDS: f32 = 0.05;

/// A sleeping fighter's press buys back time — the counterplay the status was
/// missing.
///
/// ⛔⛔ `BodyCombat::sleep_timer`'s OWN DOC SAID THIS WAS ABSENT: *"THIS IS A
/// DISABLE, NOT YET A SLEEP. It buys 'cannot act for a duration' and
/// wake-on-damage. What it does NOT buy is the specific POSE or the MASH
/// escape."* That sentence was true for as long as the status shipped, and a
/// sleep is the LONGEST disable in the game — the one window in a 1v1 where a
/// human had nothing at all to do.
///
/// ⭐⭐ IT READS `ActorControl` RATHER THAN THE GATED INPUT, AND THAT IS THE
/// WHOLE TRICK. `attack_support::apply_post_hit_input_gates` blanks every verb
/// while `hard_lock_timer() > 0.0`, and a sleep is one of the five causes it
/// folds — so a reader placed after the gate samples zeros and would conclude
/// that sleeping fighters never struggle. ⇒ It works here because the gate runs
/// on a TRANSIENT `InputState` built by `engine_input_from_actor_control`; the
/// component keeps the raw press. `sample_capture_escape` had to be scheduled
/// twice to dodge the same trap, because a captive's frame is blanked in place.
///
/// ⛔ ONE CREDIT PER TICK, NOT ONE PER BUTTON — copied from that function
/// deliberately, and for its reason: a chord of six buttons would otherwise be
/// six presses and the escape would reward a controller layout.
///
/// ⚠ ROLLBACK: nothing new. `sleep_timer` is a field on `BodyCombat`, which
/// already snapshots (`ambition_characters::snapshot_impls`), and every press
/// this reads is `ActorControl`, which is re-derived from the rolled-back input
/// every tick. A rewind restores the timer and re-plays the presses that
/// shortened it.
pub fn mash_out_of_sleep(
    mut sleepers: Query<(
        &mut ambition_platformer2d::characters::actor::BodyCombat,
        &ambition_platformer2d::characters::control::ActorControl,
    )>,
) {
    for (mut combat, control) in &mut sleepers {
        // Read before write: an awake body must not be touched at all, or every
        // fighter takes a change-detection write to rollback state every tick
        // for a status none of them has.
        if combat.sleep_timer <= 0.0 {
            continue;
        }
        let frame = &control.0;
        // Any action press. Asking for one specific button would be a
        // control-scheme decision this has no reason to make, and a sleeper
        // mashing the "wrong" one would look like a broken mechanic.
        let pressed = frame.melee_pressed
            || frame.jump_pressed
            || frame.burst_pressed
            || frame.special_pressed
            || frame.grab_pressed
            || frame.projectile_pressed;
        if pressed {
            combat.sleep_timer = (combat.sleep_timer - MASH_SECONDS).max(0.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d::characters::actor::BodyCombat;

    fn app() -> App {
        let mut app = App::new();
        app.add_message::<ActorActionMessage>();
        // The SHIPPED order (see `lib.rs`): the mash runs first, so a press is
        // never spent on a sleep that did not exist when it was made.
        app.add_systems(
            Update,
            (mash_out_of_sleep, apply_authored_sleep, note_writes).chain(),
        );
        app
    }

    fn body(app: &mut App, at: ae::Vec2) -> Entity {
        app.world_mut()
            .spawn((
                ae::BodyKinematics {
                    pos: at,
                    size: ae::Vec2::new(28.0, 46.0),
                    ..Default::default()
                },
                ae::CenteredAabb::from_center_size(at, ae::Vec2::new(28.0, 46.0)),
                BodyCombat::default(),
                control(),
                Wrote::default(),
            ))
            .id()
    }

    fn sing(app: &mut App, singer: Entity, duration_s: f32) {
        app.world_mut().write_message(ActorActionMessage {
            actor: singer,
            request: ActionRequest::Special {
                spec: SpecialActionSpec::Special(SLEEP.to_string()),
                params: ambition_platformer2d::entity_catalog::ParamValue::from_typed(
                    &SleepParams {
                        duration_s,
                        half_extents: (70.0, 40.0),
                    },
                )
                .expect("sleep params serialize"),
            },
        });
    }

    fn slept(app: &App, body: Entity) -> f32 {
        app.world().get::<BodyCombat>(body).unwrap().sleep_timer
    }

    /// The song catches whoever is near and never the singer.
    ///
    /// ⛔⛔ THE SELF-EXCLUSION IS THE MOVE, NOT POLITENESS. Sing puts everyone
    /// else to sleep and then the singer acts — that IS the payoff. A version
    /// that slept its own caster would be an elaborate way to lose, and nothing
    /// about the area or the duration would reveal it.
    #[test]
    fn the_song_catches_the_room_and_never_the_singer() {
        let mut app = app();
        let singer = body(&mut app, ae::Vec2::new(0.0, 0.0));
        let near = body(&mut app, ae::Vec2::new(40.0, 0.0));
        let far = body(&mut app, ae::Vec2::new(400.0, 0.0));
        sing(&mut app, singer, 1.4);
        app.update();

        assert_eq!(slept(&app, singer), 0.0, "the singer slept through their own song");
        assert!(
            slept(&app, near) > 0.0,
            "a body inside the pulse was not caught"
        );
        assert_eq!(
            slept(&app, far),
            0.0,
            "a body well outside the pulse was caught, so the area means nothing"
        );
    }

    /// Per-body scratch: was this fighter's `BodyCombat` written this tick?
    ///
    /// ⛔ NOT A TEST-ONLY `Resource` COLLECTING `Changed<..>`, which is what the
    /// first version was: `per_attempt_resource_census` reddened on it, and
    /// correctly — it scans `game/` for collection-holding `Resource` types and
    /// asks of each whether it is per-attempt state that a death or replay must
    /// re-arm. A fixture's scratch vector cannot answer that and should never
    /// have been asked; a component on the body it describes is the right shape
    /// anyway.
    ///
    /// ⛔⛔ AND READING `is_changed()` FROM OUTSIDE A SYSTEM DOES NOT WORK — the
    /// second draft did that and the POSITIVE CONTROL caught it. A `Ref` taken
    /// from `&World` compares against the world's change tick, which `update()`
    /// has already advanced past, so EVERY body reads unchanged. That version
    /// would have passed `mashing_while_awake_takes_no_write_at_all` for the
    /// worst possible reason. Inside a system, `Ref::is_changed()` compares
    /// against THAT system's previous run, which is the question being asked.
    #[derive(Component, Default)]
    struct Wrote(bool);

    fn note_writes(
        mut bodies: Query<(
            &mut Wrote,
            bevy::prelude::Ref<ambition_platformer2d::characters::actor::BodyCombat>,
        )>,
    ) {
        for (mut wrote, combat) in &mut bodies {
            wrote.0 = combat.is_changed();
        }
    }

    fn was_written(app: &App, who: Entity) -> bool {
        app.world().get::<Wrote>(who).expect("scratch flag").0
    }

    fn control() -> ambition_platformer2d::characters::control::ActorControl {
        ambition_platformer2d::characters::control::ActorControl(Default::default())
    }

    /// Press one action THIS tick on `who`. Rising edges, so a fixture that
    /// wanted a hold would have to re-set them every tick — which is the point.
    fn mash(app: &mut App, who: Entity) {
        let mut frame = app
            .world_mut()
            .get_mut::<ambition_platformer2d::characters::control::ActorControl>(who)
            .expect("the fixture gives every fighter a control frame");
        frame.0.melee_pressed = true;
    }

    /// ⭐⭐ THE SLEEPING PLAYER HAS SOMETHING TO DO, AND UNTIL 2026-09-06 THEY
    /// DID NOT. `BodyCombat::sleep_timer`'s own doc said so: *"THIS IS A
    /// DISABLE, NOT YET A SLEEP. It buys 'cannot act for a duration' and
    /// wake-on-damage. What it does NOT buy is the specific POSE or the MASH
    /// escape."* This is the second of those two, turned from a comment into a
    /// rule — the sleep is the longest disable in the game and it was the only
    /// one a human could not answer.
    #[test]
    fn a_mashing_fighter_wakes_sooner_than_one_who_waits() {
        let mut app = app();
        let singer = body(&mut app, ae::Vec2::new(0.0, 0.0));
        let masher = body(&mut app, ae::Vec2::new(30.0, 0.0));
        let still = body(&mut app, ae::Vec2::new(50.0, 0.0));
        sing(&mut app, singer, 1.4);
        app.update();
        assert_eq!(slept(&app, masher), slept(&app, still), "the two started apart");

        for _ in 0..10 {
            mash(&mut app, masher);
            app.update();
        }
        assert!(
            slept(&app, masher) < slept(&app, still),
            "mashing bought nothing: masher {} vs still {}",
            slept(&app, masher),
            slept(&app, still),
        );
        // ⛔ AND IT DOES NOT DELETE THE PUNISH. Ten presses off a 1.4s song
        // leaves most of it: a sleep a mash ends outright is not a status, and
        // the singer paid a slow, self-centred move for the window.
        assert!(
            slept(&app, masher) > 0.0,
            "ten presses ended the whole sleep, so the singer's payoff is gone",
        );
    }

    /// ⛔ ONE CREDIT PER TICK, NEVER ONE PER BUTTON — the same rule
    /// `sample_capture_escape` states for a grab, and for its reason: a chord of
    /// six buttons would otherwise be six presses, and escape would reward a
    /// control-scheme trick rather than a mash.
    #[test]
    fn a_chord_of_every_button_buys_exactly_what_one_press_buys() {
        let mut app = app();
        let singer = body(&mut app, ae::Vec2::new(0.0, 0.0));
        let one = body(&mut app, ae::Vec2::new(30.0, 0.0));
        let all = body(&mut app, ae::Vec2::new(50.0, 0.0));
        sing(&mut app, singer, 1.4);
        app.update();

        mash(&mut app, one);
        {
            let mut frame = app
                .world_mut()
                .get_mut::<ambition_platformer2d::characters::control::ActorControl>(all)
                .expect("a control frame");
            frame.0.melee_pressed = true;
            frame.0.jump_pressed = true;
            frame.0.burst_pressed = true;
            frame.0.special_pressed = true;
            frame.0.grab_pressed = true;
            frame.0.projectile_pressed = true;
        }
        app.update();
        assert_eq!(
            slept(&app, one),
            slept(&app, all),
            "six buttons in one tick beat one button, so the escape rewards a \
             controller layout instead of a mash",
        );
    }

    /// ⛔⛔ A FIGHTER WHO IS NOT ASLEEP TAKES NO WRITE — and the FIRST draft of
    /// this test could not see its own subject.
    ///
    /// It asserted `sleep_timer == 0.0` after an awake fighter mashed, which is
    /// true of the guarded version AND of one that subtracts unconditionally:
    /// `(0.0 - 0.05).max(0.0)` is `0.0`. The value is identical; what differs is
    /// that the unguarded version takes a change-detection write to ROLLBACK
    /// STATE on every body on every tick, forever, for a status none of them
    /// has. That is the defect `sample_capture_escape`'s `With<CapturedBy>`
    /// filter exists to prevent, and the number cannot distinguish them.
    ///
    /// ⇒ So it observes the WRITE. `Changed<BodyCombat>` after the system, with
    /// a baseline tick first because inserting a component marks it changed.
    #[test]
    fn mashing_while_awake_takes_no_write_at_all() {
        let mut app = app();
        let awake = body(&mut app, ae::Vec2::new(0.0, 0.0));
        // The insertion itself is a change; spend it before measuring.
        app.update();

        mash(&mut app, awake);
        app.update();
        assert_eq!(
            slept(&app, awake),
            0.0,
            "an awake fighter's timer moved",
        );
        assert!(
            !was_written(&app, awake),
            "an awake fighter's BodyCombat was written to; that is a rollback \
             checksum churning every tick for a status nobody has",
        );
    }

    /// ⭐ AND THE POSITIVE CONTROL FOR THE ASSERTION ABOVE, without which it
    /// passes against a `note_touched` that never sees anything: a fighter who
    /// IS asleep and mashes must show up in the very same set.
    #[test]
    fn a_sleeping_masher_does_take_the_write() {
        let mut app = app();
        let singer = body(&mut app, ae::Vec2::new(0.0, 0.0));
        let victim = body(&mut app, ae::Vec2::new(30.0, 0.0));
        sing(&mut app, singer, 1.4);
        app.update();

        mash(&mut app, victim);
        app.update();
        assert!(
            was_written(&app, victim),
            "a sleeping fighter mashed and nothing was written, so the test \
             above proves nothing",
        );
        assert!(
            !was_written(&app, singer),
            "the singer, who is awake, was written on the same tick — so \
             `was_written` answers about the TICK rather than about the body",
        );
    }

    /// Two songs do not STACK; the longer one wins.
    ///
    /// ⛔ ADDITION WOULD BE UNBOUNDED. Two singers, or one singer twice, would
    /// compound into a sleep nobody wakes from — and the wake a real hit buys
    /// would stop being the counterplay it is meant to be.
    #[test]
    fn overlapping_songs_take_the_longer_one_rather_than_the_sum() {
        let mut app = app();
        let singer = body(&mut app, ae::Vec2::new(0.0, 0.0));
        let victim = body(&mut app, ae::Vec2::new(40.0, 0.0));
        sing(&mut app, singer, 1.4);
        app.update();
        sing(&mut app, singer, 0.6);
        app.update();
        assert_eq!(
            slept(&app, victim),
            1.4,
            "overlapping songs did not take the longer one — a shorter song \
             either extended the sleep or cut it short"
        );
    }
}
