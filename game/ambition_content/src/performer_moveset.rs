//! The Performer — the sword archetype's table, with four specials of her own.
//!
//! ⭐ AND SHE CARRIES NO SWORD. The Pointed Polygon's frame data retargets onto
//! her for the reason the Author's does: his pen occupies the arming sword's
//! exact axis, and her conjured blade of stage light occupies it too — authored
//! as the swing's own axis extended past her hand, so the reach the table
//! assumes is the reach the sheet draws.
//!
//! ⭐⭐ HER SPECIALS ARE STAGE MACHINERY, and the two that move her move her in
//! two different ways. The FLYLINE is a wire: one beat, aimed, straight up out
//! of the scene — `smash.teleport`, the technique the Author's revision already
//! used. The TRAP is not. She goes THROUGH the floor and travels under it, and
//! that is `smash.trapdoor` plus a body mode, because Jon was explicit that it
//! is *"not a blink. It's a different kind of mobility move"* and that *"I do
//! want the player to be able to control where they move."*
//!
//! ⭐ THE OTHER TWO ARE STAGECRAFT WITHOUT MACHINERY. The MONOLOGUE and THE
//! LINE are plain strikes; what makes them hers is the SHAPE the art gave them,
//! and neither needed a technique to say it.
//!
//! See [`crate::archetype_moveset`] for why the borrowed ids are renamed rather
//! than shared or copied.

use ambition_characters::moveset_authoring::{fixed_knockback, on_contact, sfx, strike, Strike};
use ambition_characters::smash_flyline::{author_flyline, FlylineParams};
use ambition_characters::smash_trapdoor::{author_trapdoor, TrapdoorParams};
use ambition_platformer2d::entity_catalog::{MoveSpec, MovesetContract};

/// THE TRAP, AS JON SPECIFIED IT — the whole lifecycle, in his words,
/// 2026-08-28:
///
/// > *"When down-b is resolved to the trapdoor move, a trapdoor opens and the
/// > character descends underground. In this subterranean state they can move
/// > for up to the timelimit of the move (3 seconds) where they move is shown
/// > by a unopened trap door sprite on the ground. When the move ends or the
/// > character ends the move by pressing a non-move action, the final stage of
/// > the move happens, where the trapdoor opens and then they leap out with an
/// > explosion and hurtbox in the area."*
///
/// FIVE STAGES, and each one is a constant below:
///
/// 1. `DOOR_OPENS_S` — the boards give. She is still in the world.
/// 2. `SINK_AT_S` — she descends. `BodyMode::Submerged`: not drawn, not
///    hittable, no gravity, no geometry, and the door closes over her.
/// 3. `HOLD_UNDER_AT_S`..`MAX_UNDER_S` — the SUBTERRANEAN state. She steers,
///    surface-locked to the boards she went through, and what the stage shows is
///    an UNOPENED trapdoor sliding along the floor. This is the beat the move
///    exists for, and it is the one the clock freezes on.
/// 4. `EXIT_DOOR_OPENS_S` — the ask ends the freeze (or the ceiling does) and
///    the exit trapdoor bangs open. She is still under it.
/// 5. `SURFACE_AT_S` — she LEAPS OUT: the surfacing beat's own launch, the
///    firework, and a hitbox over the door. The launch is the SAME write as the
///    placement (`TrapdoorParams::leap_speed`) rather than a second event, which
///    is what it was while it silently did nothing.
///
/// ⭐⭐ THE SUBTERRANEAN BEAT IS A DURATION, NOT A HOLD. Jon caught the other
/// reading in a day: *"The latest main the actor doesn't spend any time under
/// the stage… It looks like the pop up happens immediately."* Nobody holds B
/// while steering. She gets the whole three seconds for free and an ACTION
/// press takes them back — see [`ChargeSustain::UntilPressedAgain`].
///
/// `blink_out` runs at 52ms a frame. The boards give on frame 2 and she is
/// through them by frame 3, which is when she stops being in the world.
const DOOR_OPENS_S: f32 = 0.10;
const SINK_AT_S: f32 = 0.16;
/// Where the timeline FREEZES — a hair after she is under.
///
/// ⛔ AFTER `SINK_AT_S`, AND THAT ORDER IS THE MECHANIC. The submerge beat is a
/// timed event on this timeline; freezing the clock ON it or before it would
/// hold her at the mouth of the hole forever, above ground and hittable.
const HOLD_UNDER_AT_S: f32 = 0.17;
/// THE TIME LIMIT OF THE MOVE. Jon, 2026-08-28: *"they can move for up to the
/// timelimit of the move (3 seconds)."*
///
/// ⭐ IT WAS ONE SECOND THE DAY BEFORE (*"Give them 1 second under the stage at
/// 1.2x run speed. I'm biasing towards making moves too powerful to start"*) and
/// is three now. At 1.2× her 204 run speed three seconds is roughly 735 world px
/// — several stage widths — which is exactly the bias he named, and this is the
/// first knob to turn when it turns out to be too good.
const MAX_UNDER_S: f32 = 3.0;
/// The exit trapdoor bangs open. She is still under it for `SURFACE_AT_S -
/// EXIT_DOOR_OPENS_S` — the door opens, THEN she comes out, which is the order
/// Jon's sentence puts them in.
const EXIT_DOOR_OPENS_S: f32 = 0.18;
const SURFACE_AT_S: f32 = 0.30;
/// How long the emergence hits for.
const FIREWORK_S: f32 = 0.12;
const TRAP_ENDS_S: f32 = 0.54;
/// How hard she LEAPS out of the boards, against gravity.
///
/// ⛔ A LEAP, NOT A STEP UP. She is coming out of a hole in the stage under her
/// own power and the move's payoff is the space it buys; a body that surfaced
/// standing still would be handing the position straight back.
///
/// ⛔⛔ AND IT REACHES THE BODY THROUGH `TrapdoorParams::leap_speed`, not through
/// an authored impulse. As an impulse it was overwritten by the surfacing beat
/// on the very frame it fired, every time, for as long as it existed.
const LEAP_OUT_SPEED: f32 = 430.0;

/// How far above her the engine looks for a floor to come up through.
///
/// ⛔ GENEROUS, BECAUSE SHE CANNOT SEE THE BOARDS. She has been steering blind
/// under the stage; a tight radius would drop her into open air for being a few
/// pixels below a platform she was plainly under. Past this there genuinely is
/// no floor above her — she wandered off the end of the stage — and coming up
/// into open air and falling is the honest outcome.
const SURFACE_REACH: f32 = 140.0;

/// The door itself. ⛔ NOT `four_point_glint`, which is the Author's blink
/// and the whole complaint: a trapdoor is wood and hinges, not a star flash.
const TRAPDOOR_VFX: &str = "trapdoor_boards";

/// THE PUFF OF SMOKE THE WHOLE TRICK HIDES BEHIND.
///
/// ⭐⭐ JON, 2026-08-29: *"I would like the start of her down-b to cause a puff
/// of smoke (like a real play would use to disguise going through a trap
/// door)."* That is the stagecraft the move has been missing: a real
/// disappearance is not the audience watching you leave, it is the audience
/// losing sight of you for a moment and finding you gone.
///
/// ⛔ IT FIRES ON BOTH FORMS AND ON THE FIRST FRAME, before anything is known
/// about whether the boards will give. The smoke is the MISDIRECTION, so it
/// cannot wait to find out whether the trick worked — an effect that only
/// appears when the move succeeds tells an opponent which one they are watching.
/// ⛔⛔ `smoke_puff`, AND IT WAS `smoke_burst`, WHICH IS AN EXPLOSION. Jon,
/// 2026-08-29: *"you used an explosion effect, not a poof of smoke effect, which
/// I think we have."* He is exactly right and the sheet says so: `smoke_burst`
/// is a row of **`generic_explosions`**, while `smoke_puff` is a row of
/// `generic_exotic_fx` whose frames GROW (58→64→…px), which is what a cloud of
/// smoke does and what a detonation does not.
///
/// ⭐ THE NAME WAS THE TRAP. "smoke_burst" reads as smoke and is filed under
/// explosions; nothing in the authoring layer would have said so, because an
/// effect id is a string and every string is spelled correctly.
const SMOKE_VFX: &str = "smoke_puff";

/// How long the airborne form lasts — smoke, and a beat to be caught in.
///
/// ⭐⭐ SHORT ON PURPOSE. Jon: *"it doesn't do much in the air, just a poof of
/// smoke, but that's better than no effect in the air."* It is a FEINT: it
/// costs a little endlag, it looks exactly like the real thing for the first
/// few frames, and it buys nothing. A move that did nothing at all would be a
/// dead button; a move that did something useful would not be the trick failing.
const AIR_PUFF_ENDS_S: f32 = 0.30;

/// The emergence. ⛔ NOT the boards: those are the door opening, and this is
/// what comes out of it.
const FIREWORK_VFX: &str = "starstuff_burst";

/// The flyline catches her later than the trap drops her: the wire goes taut
/// before it pulls, which is the beat `fly`'s first two frames draw.
const WIRE_AT_S: f32 = 0.12;
/// When the winch stops and the rope lets go.
///
/// ⛔⛔ THE MOVE MUST OUTLAST THE LIFT, and this constant is the join. The kernel
/// owns the wire's own clock (`WireState::lift_remaining_s`), so a timeline that
/// ended first would leave her being flown by a maneuver nothing is animating.
/// `the_lift_fits_inside_the_move_that_authors_it` is the guard.
const WIRE_RELEASES_S: f32 = WIRE_AT_S + LIFT_S;
const WIRE_ENDS_S: f32 = WIRE_RELEASES_S + 0.10;

/// How far the wire lifts her, in world px.
///
/// ⭐⭐ JON ASKED FOR *"a fairly large vertical distance"*, and the only honest
/// form of that is a number measured against the stage it is played on. The
/// smash platform's surface sits **420px above the fall blast line** — so this
/// is exactly the depth a fighter can be knocked to and still be brought back
/// to the boards, and it is very nearly double the 215px the teleport it
/// replaces covered.
///
/// ⛔ THE FIRST KNOB TO TURN when it proves too good, the way `MAX_UNDER_S` is
/// the trapdoor's. Jon on the trap, and it applies here: *"I'm biasing towards
/// making moves too powerful to start."*
const RISE_PX: f32 = 420.0;

/// How long the lift takes.
///
/// ⛔ LONG ENOUGH TO SEE, WHICH IS CLAUSE ONE. At 60Hz this is 33 ticks and the
/// largest single one moves her about 13px; the teleport it replaces covered
/// 215px in a single frame. *"She doesn't teleport up, she gets lifted up by the
/// wire."*
const LIFT_S: f32 = 0.55;

/// How far above her the wire's anchor is when it catches.
///
/// ⭐⭐ THIS IS THE SWING RADIUS, and it is deliberately much longer than the
/// rise. A long rope swings slowly through a wide arc; a short one snaps back
/// hard. It must exceed [`RISE_PX`] or the winch would reel past its own pulley
/// — with 300px left at the release, the pendulum is still meaningful at the
/// moment it matters most.
const ROPE_PX: f32 = 720.0;

/// How far the swing may reach from straight down.
///
/// ⛔ *"A BIT"* OF HORIZONTAL RECOVERY, AND THIS IS WHAT BOUNDS IT. Measured: a
/// held stick carries her 99px sideways off a 480px platform. An uncapped
/// pendulum on a SHORTENING rope gains angle every tick — the skater pulling her
/// arms in — and would end the lift halfway across the stage.
const MAX_SWING_DEG: f32 = 18.0;

/// What a held stick contributes to the swing, in rad/s².
const SWING_ACCEL: f32 = 3.4;

/// How fast she is still rising when the wire lets go.
///
/// ⭐⭐ IT IS ALSO THE WINCH'S FINAL RATE, which is the whole reason the number
/// is small. The winch decelerates INTO the release, so the rope's last tick and
/// her first free tick are the same speed; a flat winch with a small carry rose
/// her at 764 px/s and cut her to 90, an eightfold stop at the apex that reads
/// as the teleport's own feel arriving through a different mechanic.
const RELEASE_RISE: f32 = 90.0;

/// ⛔⛔ THE WIRE DRAWS NO BURST AT ALL, AND IT USED TO DRAW A TRAPDOOR. Jon,
/// 2026-08-29: *"her up-b uses the trap door, and I don't think it should."*
/// `trapdoor_boards` is real art and it is the DOWN special's furniture — so
/// her recovery was opening a hatch in mid-air, on screen, and saying the two
/// specials were the same trick.
///
/// ⭐⭐ AND THE HONEST REPLACEMENT IS NOTHING. The rope is a PERSISTENT visual,
/// drawn from the read model for the whole lift (`rendering::flyline`) — so this
/// move already has the one picture it needs, and Jon's own clause allows the
/// wire to simply appear: *"it can instantly appear as if it went from visible
/// to invisible."* A one-shot on top of it would be decoration chosen to fill a
/// field, which is how the trapdoor got here.
///
/// ⛔ AND NOT `four_point_glint` EITHER, which is the Author's blink and the
/// whole complaint this move was rewritten for.
const WIRE_VFX: Option<&str> = None;

/// Complete sword-fundamentals repertoire, attributed to the Performer, with her
/// own down and up specials in place of the archetype's.
pub fn performer_moveset() -> MovesetContract {
    let mut set = crate::archetype_moveset::under_own_name(
        crate::pointed_polygon_moveset::pointed_polygon_moveset(),
        &["polygon", "pointed_polygon"],
        "performer",
    );
    crate::special_slots::replace_special(&mut set, "special_down", the_trap());
    crate::special_slots::replace_special(&mut set, "special_air_down", the_trap_airborne());
    crate::special_slots::replace_special(&mut set, "special_up", the_flyline());
    crate::special_slots::replace_special(&mut set, "special", the_monologue());
    crate::special_slots::replace_special(&mut set, "special_forward", the_line());
    set
}

/// Neutral special: she plants, opens both arms and DELIVERS.
///
/// ⭐⭐ *"It holds her still for as long as it holds everyone else"* — the
/// caption `special.spec.json` has carried since her library was forked, and the
/// two halves of it are authored separately because the engine says them
/// separately. She is held by `hitless_special`'s rooting, which every special
/// gets; everyone ELSE is held by FIXED KNOCKBACK.
///
/// ⛔⛔ `knockback_growth: Some(0.0)` IS THE MOVE, and it is not the same as
/// leaving the field alone. Unauthored means *the stage decides*, and the
/// stage's rule scales a launch with the victim's percent — so the monologue
/// would land differently on the fighter it was aimed at depending on how the
/// match had gone, which is the one thing a speech that holds EVERYONE must not
/// do. `Some(0.0)` is a hit that does exactly this at 0% and at 200%.
///
/// ⛔ AND THE LAUNCH IS SHALLOW ON PURPOSE. Hitstun scales off knockback
/// magnitude, so a hold is bought with enough launch to buy the frames and a
/// direction that spends them going nowhere in particular — the genre has no
/// separate stun channel and inventing one for a neutral-B would be a mechanic
/// bolted to a single move.
fn the_monologue() -> MoveSpec {
    let mut spec = strike(Strike {
        id: "performer_monologue",
        clip: "special",
        // Frames 3, 4 and 5 of 8 at 75ms — the window the art marks live.
        startup_s: 0.225,
        active_s: 0.225,
        recover_s: 0.15,
        // Wide and centred: she is addressing the room, not pointing at one
        // person. The spec inflates its drawn hull by 7px for the same reason.
        offset: (10.0, -6.0),
        half_extents: (58.0, 34.0),
        damage: 6,
        knockback: 74.0,
        // ⛔ THE BUILDER CANNOT SAY WHAT THIS MOVE IS. Its `f32` reads zero as
        // "this stage decides", so the fixed knockback goes on the VOLUME —
        // `fixed_knockback` below, which is the builder's own instruction.
        knockback_growth: 0.0,
        launch_dir: Some((0.35, -0.5)),
        on_hit: None,
    });
    spec.display_name = Some("Monologue".to_string());
    // ⛔⛔ SHE IS HELD TOO, and `strike` does not do it: it authors
    // `motion_scale: 1.0` on all three of its windows, so without this she walks
    // through her own speech while everyone she hit is pinned by the fixed
    // knockback — the inverse of the one asymmetry this move is about.
    //
    // ⇒ rooted on the strike's OWN windows rather than by pushing a blanket one
    // over the top: `motion_scale_at` folds with `min` so either works, but a
    // blanket window would have to carry a tag, and every tag says something to
    // the scorer that is not true of a whole move.
    for window in &mut spec.windows {
        window.motion_scale = 0.0;
    }
    let spec = fixed_knockback(spec);
    let spec = sfx(spec, 0.0, "player.attack.charge");
    let spec = sfx(spec, 0.225, "player.slash");
    on_contact(spec, "player.hit")
}

/// Side special: she throws one, overhand, and it carries.
///
/// ⭐⭐ *"Nothing leaves her hand that anyone can see leaving it."* The line is
/// a DISJOINTED HITBOX and not a projectile, and that is the whole reading of
/// the move: the danger is out past her arm where nothing is drawn, so an
/// opponent who reads the animation reads it wrong. `shoot.spec.json` extends
/// the strike axis to 2.9 for the same reason — the art and the table agree
/// about where the reach ends.
///
/// ⛔ NOT `MoveEventKind::Ranged`, WHICH IS WHAT THE WORD "THROWS" SUGGESTS. A
/// ranged event fires the owner's ranged action, and she has none — she would
/// need a weapon to state one, and the point of the move is that there is
/// nothing in her hand.
fn the_line() -> MoveSpec {
    let mut spec = strike(Strike {
        id: "performer_the_line",
        clip: "shoot",
        // Frames 3 and 4 of 8 at 60ms.
        startup_s: 0.18,
        active_s: 0.12,
        recover_s: 0.18,
        // Far out and thin — the reach IS the move, and a fat volume would give
        // away in silhouette what the animation deliberately does not.
        offset: (62.0, -10.0),
        half_extents: (30.0, 9.0),
        damage: 9,
        knockback: 128.0,
        // ⭐ THIS ONE GROWS. It is an ordinary spacing tool and scales with its
        // victim's damage like every other normal; the monologue is the
        // exception, not the pattern.
        knockback_growth: 1.9,
        launch_dir: Some((1.0, -0.34)),
        on_hit: None,
    });
    spec.display_name = Some("The Line".to_string());
    let spec = sfx(spec, 0.18, "player.slash");
    on_contact(spec, "player.hit")
}

/// ⛔ TWO IDS FOR ONE SLOT, because the archetype's down special is a
/// `DownSpecial::ByPosture` pair and a slot left half-replaced is a press that
/// falls through to the neutral special — the player pressed down-B and got the
/// monologue.
///
/// ⛔⛔ AND THE TWO FORMS ARE NO LONGER THE SAME AUTHORING. They were, on the
/// reasoning that *"the boards are wherever she is"* — which is false, and Jon
/// said so: *"if she isn't on the ground the trap door can't open and she can't
/// go subterranian, so the move cancels."* There is no floor to cut a hatch in
/// mid-air. The grounded form is the move; the airborne one is the trick
/// failing, which is [`the_puff`].
fn the_trap() -> MoveSpec {
    trapdoor("performer_trapdoor", "blink_out")
}

/// The airborne form: the smoke goes off and nothing comes of it.
///
/// ⭐⭐ JON'S DESIGN, 2026-08-29, and the whole of it: *"So if you are in the
/// air, the smoke effect still happens, but if she isn't on the ground the trap
/// door can't open and she can't go subterranian, so the move cancels. So it
/// doesn't do much in the air, just a poof of smoke, but that's better than no
/// effect in the air."*
///
/// ⛔ IT AUTHORS NO TRAPDOOR BEAT AT ALL, rather than authoring one and relying
/// on the engine to refuse it. A refused beat would leave the rest of this
/// timeline running — including the three-second `smash_charge` freeze, which
/// does not know or care whether she went under — and she would hang motionless
/// in mid-air for three seconds with nothing on screen explaining why. The
/// engine's refusal (`apply_authored_trapdoors`) is the SAFETY NET for a
/// grounded press that leaves the boards before the sink; it is not the design.
///
/// ⛔ AND IT SPENDS NO RECOVERY AND GRANTS NO I-FRAMES. The failing trick is
/// not a free escape option: the whole cost of pressing it in the air is the
/// endlag, and the whole benefit is that an opponent has to read which one it
/// was.
fn the_trap_airborne() -> MoveSpec {
    let mut spec = ambition_characters::moveset_authoring::hitless_special(
        "performer_trapdoor_air",
        "blink_out",
        DOOR_OPENS_S,
        AIR_PUFF_ENDS_S,
    );
    spec.display_name = Some("The Trap".to_string());
    // The SAME first frames as the real thing — the same clip, the same smoke,
    // the same wooden report of a door that is about to open. What differs is
    // that no door does.
    let spec = ambition_characters::moveset_authoring::vfx(spec, 0.0, SMOKE_VFX);
    ambition_characters::moveset_authoring::sfx(spec, DOOR_OPENS_S, "world.door.open")
}

/// Down special: the boards give, she drops through, and she comes up somewhere
/// else entirely.
///
/// ⭐⭐ JON, 2026-08-27, ON WHAT THIS IS NOT: *"the trapdoor move doesn't
/// actually look like a trap door. It looks like a blink… It's not a blink.
/// It's a different kind of mobility move."* The first version was
/// `smash.teleport` with a glint on each end — one instantaneous beat, she never
/// left the stage, and the effect belonged to the Author. This is four beats and
/// a body mode.
///
/// ⛔⛔ AND THE MIDDLE BEAT IS THE MOVE. Between `SINK_AT_S` and
/// `SURFACE_AT_S` she is in `BodyMode::Submerged`: not drawn, not hittable, no
/// gravity, no geometry — and still steering. That is why this is a technique
/// and a kernel mode rather than a longer animation on a teleport; a computed
/// destination would have taken the decision away from the player, which is the
/// thing Jon asked for last.
///
/// ⛔ THE `Invuln` WINDOW IS AUTHORED ANYWAY, over the same span the mode
/// covers. It is not redundant belt-and-braces: the window is what the CANCEL
/// and scoring layers read off the timeline without looking at a live body, and
/// a move whose timeline claimed she was vulnerable under the stage would be
/// scored as a punishable commitment it is not.
fn trapdoor(id: &str, clip: &str) -> MoveSpec {
    let mut spec =
        ambition_characters::moveset_authoring::hitless_special(id, clip, SINK_AT_S, TRAP_ENDS_S);
    spec.display_name = Some("The Trap".to_string());
    // ⛔⛔ THE RECOVERY BEGINS WHEN SHE SURFACES, and the gap between the two
    // beats is load-bearing. `hitless_special` roots the body across its whole
    // duration and `MoveSpec::motion_scale_at` folds overlapping windows with
    // `min`, so a Recovery window starting at `SINK_AT_S` multiplies her
    // steering by zero for the entire submerged beat — the beat this move exists
    // for. With no window covering `SINK_AT_S..SURFACE_AT_S` the fold returns
    // its identity and she steers at full authority; the `Invuln` window
    // authored below still covers that span for the cancel and scoring layers,
    // so leaving the gap costs them nothing.
    for window in &mut spec.windows {
        if matches!(window.tag, ambition_entity_catalog::WindowTag::Recovery) {
            window.start_s = SURFACE_AT_S + FIREWORK_S;
        }
    }
    // ⭐⭐ THE EMERGENCE IS A STRIKE, and Jon asked for it in those words: *"she
    // should be able to pop up at any time from it in a big firework display
    // that damages whoever is on top or above the trap door when she emerges."*
    // Standing on a door somebody is under has to be a mistake, or the move is
    // free.
    //
    // ⛔ THE COLUMN IS CENTRED AND UNFACED. `offset.x` mirrors with facing and
    // this one is zero, because the door is UNDER her — a firework that came out
    // at an angle would let a camper stand on the hinge side. It reaches from
    // just below her feet to well over her head, which is what *"on top or
    // above"* names.
    //
    // ⛔ AND IT IS AUTHORED ON THE TIMELINE, NOT EMITTED BY THE TECHNIQUE. The
    // clock FREEZES at `HOLD_UNDER_AT_S` and resumes on release, so this window
    // arrives whenever she comes up without knowing anything about when that
    // was — the same property that lets `author_trapdoor`'s surfacing beat stay
    // a plain timed event.
    spec.windows.push(ambition_entity_catalog::MoveWindow {
        start_s: SURFACE_AT_S,
        end_s: SURFACE_AT_S + FIREWORK_S,
        tag: ambition_entity_catalog::WindowTag::Active,
        volumes: vec![ambition_entity_catalog::HitVolume {
            shape: ambition_entity_catalog::VolumeShape::Rect {
                offset: (0.0, -12.0),
                half_extents: (34.0, 46.0),
            },
            damage: 12,
            knockback: 150.0,
            // Unauthored: the stage's rule scales this launch with the victim's
            // percent, which is what a kill move wants and what the monologue
            // deliberately refuses.
            knockback_growth: None,
            // Straight up, out of the floor. +y is DOWN.
            launch_dir: Some((0.0, -1.0)),
            on_hit: None,
            // ⛔ NOT A SLASH. `HitVolume::vfx` is the ARC a blade draws, and
            // naming an effect there asks the slash machinery to swing a
            // firework out of her hand. The show is authored as an event on the
            // timeline below, where it lands at the DOOR; what this volume
            // draws is the engine's default attack sweep, which is what every
            // strike of hers draws.
            vfx: None,
            hit_sfx: None,
            reaction: None,
        }],
        // Coming out of a hole is not a moment she steers through.
        motion_scale: 0.0,
        sustain_effect: None,
    });
    // ⭐⭐ STAGE THREE: THE SUBTERRANEAN BEAT, and it is the shipped timeline
    // hold rather than a new mechanic. `MoveCharge` freezes a timeline at an
    // authored point, accrues the freeze in the owner's proper time, and resumes
    // when something asks or at the maximum — which is Jon's *"they can move for
    // up to the timelimit of the move (3 seconds)… or the character ends the
    // move by pressing a non-move action"* exactly.
    //
    // ⚠ AND THE PRIMITIVE IS MISNAMED FOR THIS USE. It is spelled
    // `smash_charge` / `SmashChargeSpec` because a smash was its first customer,
    // and this move charges nothing — what it holds is a beat. The mechanic is
    // right and the noun is not; renaming it to a timeline HOLD, with charging
    // as one policy on it, is the elegant version and is a separate change.
    //
    // ⛔ AND IT MUST NOT ROOT HER. A smash's freeze roots because a windup is a
    // commitment; this one holds TRAVEL. `SmashChargeSpec::roots` is where the
    // two uses of one mechanic say which they are.
    spec.smash_charge = Some(ambition_entity_catalog::SmashChargeSpec {
        hold_at_s: HOLD_UNDER_AT_S,
        max_hold_s: MAX_UNDER_S,
        // Nothing is banked: what she was holding was a position, and she is
        // out of the hole either way.
        stores: false,
        roots: false,
        // ⛔⛔ AND NOBODY HAS TO HOLD ANYTHING. Jon, 2026-08-28, on the first
        // version of this: *"The latest main the actor doesn't spend any time
        // under the stage… It looks like the pop up happens immediately."* He
        // was not holding B, and nobody would while steering — the three seconds
        // under the stage are a DURATION he asked for outright, and ending them
        // early is a thing he asked to be able to DO, not a thing he has to stop
        // doing.
        sustain: ambition_entity_catalog::ChargeSustain::UntilPressedAgain,
    });
    // The press that STARTED this move is the one that holds it — down-Special.
    spec.charge_gesture = ambition_entity_catalog::ChargeGesture::Special;
    // ⛔⛔ SHE GOES UNDER, AND SHE COMES BACK. Two beats of one technique, and
    // the second one is the half whose absence is a fighter gone for the match.
    let spec = author_trapdoor(
        spec,
        SINK_AT_S,
        TrapdoorParams {
            submerge: true,
            surface_reach: 0.0,
            // Going under is not a launch.
            leap_speed: 0.0,
            vfx: TRAPDOOR_VFX.to_string(),
            sfx: "world.door.heavy_open".to_string(),
        },
    );
    // ⭐⭐ STAGE FOUR: THE EXIT TRAPDOOR OPENS, AND SHE IS STILL UNDER IT. Jon's
    // order, in his words: *"the trapdoor opens and then they leap out."* The
    // boards are a `trapdoor_boards` effect on their own beat rather than part of
    // the surfacing, so the twelve hundredths between them are frames of an open
    // hole with nobody out of it yet — which is the tell that something is
    // coming.
    let spec = ambition_characters::moveset_authoring::vfx(spec, EXIT_DOOR_OPENS_S, TRAPDOOR_VFX);
    let spec = ambition_characters::moveset_authoring::sfx(
        spec,
        EXIT_DOOR_OPENS_S,
        "world.door.heavy_open",
    );
    // ⭐⭐ STAGE FIVE: SHE LEAPS OUT, and the leap is now part of the SURFACING
    // rather than a second event racing it.
    //
    // ⛔⛔ THIS USED TO BE A `MoveEventKind::Impulse` ON THIS TIMELINE, authored
    // at `SURFACE_AT_S` on the reasoning that landing it on the same instant as
    // the surfacing meant *"the placement and the launch cannot disagree about
    // where she left from."* They never disagreed: an impulse is applied inline
    // in `advance_move_playback` and the trapdoor beat is a MESSAGE handled by a
    // later system, whose `TransitVelocity::Zero` overwrote it on every single
    // surfacing. `LEAP_OUT_SPEED` was dead content for as long as it existed and
    // no reading of either file alone could show it — the probe's velocity
    // column is what did: `(0,0)` on every tick from t197 to t212 with `y` never
    // leaving the floor.
    //
    // ⇒ `TrapdoorParams::leap_speed`. One writer of exit velocity.
    let spec = author_trapdoor(
        spec,
        SURFACE_AT_S,
        TrapdoorParams {
            submerge: false,
            surface_reach: SURFACE_REACH,
            // ⛔ A LEAP, NOT A STEP UP. She is coming out of a hole under her own
            // power and the move's payoff is the space it buys; a body that
            // surfaced standing still would hand the position straight back.
            leap_speed: LEAP_OUT_SPEED,
            // ⛔ THE BOARDS ALREADY OPENED, one beat ago and on their own event.
            // Asking for them again here would draw a second set over the first
            // on the frame she comes through.
            vfx: FIREWORK_VFX.to_string(),
            sfx: "world.door.heavy_open".to_string(),
        },
    );
    // ⛔ NOT A BLINK CUE ANYWHERE ON IT. The trap is carpentry: the boards give,
    // they bang shut behind her, and they give again somewhere else.
    // ⭐⭐ THE SMOKE FIRST, BEFORE THE BOARDS. It is the misdirection the trick
    // is performed behind, so it goes off on frame ONE — ahead of the door's own
    // report at `DOOR_OPENS_S` and well ahead of her going through it. See
    // [`SMOKE_VFX`].
    let spec = ambition_characters::moveset_authoring::vfx(spec, 0.0, SMOKE_VFX);
    let spec = ambition_characters::moveset_authoring::sfx(spec, DOOR_OPENS_S, "world.door.open");
    let spec =
        ambition_characters::moveset_authoring::sfx(spec, SINK_AT_S + 0.06, "world.door.close");
    let spec =
        ambition_characters::moveset_authoring::sfx(spec, TRAP_ENDS_S - 0.06, "world.door.close");
    ambition_characters::moveset_authoring::invuln(spec, SINK_AT_S, SURFACE_AT_S)
}

/// Up special: a wire comes down out of the flies, takes her at the waist, and a
/// winch walks her up while she swings.
///
/// ⭐⭐ JON, 2026-08-29, ON WHAT THIS IS NOT: *"It is not a teleport and should
/// not get the teleport sound. It needs to be a rope or wire that reaches down
/// from the sky… but she doesn't teleport up, she gets lifted up by the wire, a
/// fairly large vertical distance, and while she is being lifted by the wire her
/// motion controls should let her swing like a pendulum so she has a bit of
/// horizontal recovery with it too."*
///
/// ⛔⛔ AND IT WAS LITERALLY THE AUTHOR'S TELEPORT WITH A DIFFERENT COMMENT —
/// one beat, one placement, 215px of `smash.teleport`. The blink cue Jon heard
/// came from `apply_authored_teleports`, which emits it at EVERY transit, so no
/// edit to this timeline could ever have silenced it. A move that runs the
/// teleport executor IS a teleport. The fix is a different technique.
///
/// ⛔⛔ AND THE MIDDLE BEAT IS THE MOVE, exactly as it is for the Trap. Between
/// `WIRE_AT_S` and `WIRE_RELEASES_S` her position is `(anchor, rope, angle)`:
/// gravity is off, her velocity is not integrated, and the stick buys ANGULAR
/// acceleration. She is still drawn, still solid and still hittable — which is
/// why the wire is a maneuver in the movement kernel and not a body mode.
fn the_flyline() -> MoveSpec {
    let mut spec = ambition_characters::moveset_authoring::hitless_special(
        "performer_curtain_call",
        "fly",
        WIRE_AT_S,
        WIRE_ENDS_S,
    );
    spec.display_name = Some("Curtain Call".to_string());
    // ⛔⛔ THE ROOT IS LIFTED OFF THE LIFT, AND THAT GAP IS LOAD-BEARING. This is
    // the same correction the Trap needed and for the same reason:
    // `hitless_special` roots the body across its whole duration, the kernel's
    // wire reads the DAMPED stick (`InputState::local_axis`, the one every other
    // mode reads), and `MoveSpec::motion_scale_at` folds overlapping windows with
    // `min` — so a Recovery window starting at `WIRE_AT_S` multiplies her swing
    // by zero for the entire beat the move exists for, and clause six is deleted
    // while every spec test stays green.
    //
    // ⇒ the Recovery begins where the rope lets go. With no window covering the
    // lift the fold returns its identity and she steers the pendulum at full
    // authority.
    for window in &mut spec.windows {
        if matches!(window.tag, ambition_entity_catalog::WindowTag::Recovery) {
            window.start_s = WIRE_RELEASES_S;
        }
    }
    let spec = author_flyline(
        spec,
        WIRE_AT_S,
        FlylineParams {
            rope_length: ROPE_PX,
            rise: RISE_PX,
            lift_s: LIFT_S,
            max_swing_deg: MAX_SWING_DEG,
            swing_accel: SWING_ACCEL,
            release_rise: RELEASE_RISE,
            vfx: WIRE_VFX.map(str::to_string),
            // Rope and pulley — the same bank the Trap's carpentry came out of.
            // ⛔ NOT `player.blink`, and nothing on this move may reach it.
            sfx: "world.door.heavy_open".to_string(),
        },
    );
    // ⭐ THE SAME I-FRAMES THE OTHER RECOVERIES GET, over the span she is on the
    // rope. The teleport bought them with `TeleportParams::intangible_s` for
    // being NOWHERE mid-blink; hers are for being off the stage on a wire, which
    // is a longer and more visible commitment.
    //
    // ⛔ AUTHORED ON THE TIMELINE rather than granted by the technique, for the
    // reason the Trap's are: the window is what the CANCEL and scoring layers
    // read without looking at a live body.
    let spec = ambition_characters::moveset_authoring::invuln(spec, WIRE_AT_S, WIRE_RELEASES_S);
    let spec = ambition_characters::moveset_authoring::sfx(spec, 0.0, "player.attack.charge");
    // ⛔⛔ NO `player.blink` ANYWHERE ON IT, and now nothing downstream asks for
    // one either. `apply_authored_flylines` writes no position, picks no
    // destination and records no Class-B remap — there is nothing for a teleport
    // cue to be attached to.
    // ⛔⛔ THROUGH THE SLOT, so it costs what an up-B costs. Inserted after
    // `SmashRepertoire::into_contract` has lowered the table it joins, nothing
    // else will stamp `gates.recovery` on it — and an up-B that spends nothing
    // is flight. The swing is *"a bit"* of recovery, not a free traversal.
    ambition_characters::smash_repertoire::UpSpecial::Standard(spec).into_spec()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ⛔⛔ SHE STEERS UNDER THE STAGE, AND THE ROOTING IS WHAT TAKES IT AWAY.
    /// `hitless_special` roots the body across its whole duration and
    /// `motion_scale_at` folds with `min`, so the submerged beat has to be left
    /// UNCOVERED by any window — see `trapdoor`, which moves Recovery's start.
    ///
    /// ⛔⛔ THE SUBTERRANEAN BEAT IS THE SHIPPED TIMELINE HOLD rather than a
    /// second mechanic. Jon, 2026-08-28: *"they can move for up to the timelimit
    /// of the move (3 seconds)… or the character ends the move by pressing a
    /// non-move action."*
    ///
    /// Three facts, and each of them is a way the move breaks if it is missing.
    /// The gesture must be `Special`, because `charged_by_gesture` enters charge
    /// mode only where the press that started the use and the move's own
    /// `charge_gesture` AGREE — a `Smash` policy here would freeze on a button
    /// this move is never reached by, which is to say never. The hold point must
    /// sit AFTER the submerge event, or the clock stops at the mouth of the hole
    /// with her above ground and hittable. And it must not ROOT: `motion_scale`
    /// is not the only thing that can take her steering away, and the other one
    /// is the freeze itself.
    #[test]
    fn the_subterranean_beat_is_a_duration_an_action_press_can_cut_short() {
        let set = performer_moveset();
        // ⛔ THE GROUNDED FORM ONLY. The airborne one authors no hold at all —
        // it is a puff of smoke and an endlag, see `the_puff_in_the_air_...`
        // below — and a loop that still expected a three-second freeze from it
        // was asserting the design this move used to have.
        for id in ["performer_trapdoor"] {
            let trap = set
                .moves
                .iter()
                .find(|m| m.id == id)
                .unwrap_or_else(|| panic!("{id} is in the table"));
            assert_eq!(
                trap.charge_gesture,
                ambition_entity_catalog::ChargeGesture::Special,
                "{id} is reached by a Special press, so that is the press that holds it",
            );
            let policy = trap
                .charge_policy()
                .unwrap_or_else(|| panic!("{id} authors a hold"));
            assert!(
                policy.hold_at_s > SINK_AT_S,
                "{id} freezes at {}s, which is not after the submerge beat at {SINK_AT_S}s —                  she would be held at the mouth of the hole, above ground and hittable",
                policy.hold_at_s,
            );
            assert!(
                policy.hold_at_s < SURFACE_AT_S,
                "{id} freezes at {}s, which is not before she surfaces at {SURFACE_AT_S}s",
                policy.hold_at_s,
            );
            assert_eq!(
                policy.sustain,
                ambition_entity_catalog::ChargeSustain::UntilPressedAgain,
                "{id} freezes only while a button is DOWN, so a player steering \
                 with the stick spends three ticks under the boards instead of \
                 three seconds — the exact shape Jon reported",
            );
            assert!(
                !policy.roots,
                "{id} would root her for the whole held beat, and travelling is what \
                 the beat is FOR",
            );
            assert!(
                (policy.max_hold_s - MAX_UNDER_S).abs() < 1e-6,
                "{id} may be held for {}s, wanted the authored ceiling {MAX_UNDER_S}s",
                policy.max_hold_s,
            );
        }
    }

    /// ⛔⛔ THE EMERGENCE HITS, and the column is what Jon named: *"a big
    /// firework display that damages whoever is on top or above the trap
    /// door."*
    ///
    /// ⛔ THE ARMS STRADDLE THE DOOR, because a volume test that only samples
    /// the middle cannot tell a column from a puddle. A body standing ON the
    /// boards and a body a body-height above them are both inside it; a body
    /// standing a stage-width away is not, or the move would be a screen wipe.
    #[test]
    fn coming_up_through_the_boards_is_a_strike_over_the_door() {
        let set = performer_moveset();
        let trap = set
            .moves
            .iter()
            .find(|m| m.id == "performer_trapdoor")
            .expect("the trap is in the table");
        let firework = trap
            .windows
            .iter()
            .find(|w| matches!(w.tag, ambition_entity_catalog::WindowTag::Active))
            .expect("the trap ends in a strike");
        assert!(
            firework.start_s >= SURFACE_AT_S && firework.start_s < TRAP_ENDS_S,
            "the firework starts at {}s, which is not when she surfaces ({SURFACE_AT_S}s)",
            firework.start_s,
        );
        let volume = firework.volumes.first().expect("the strike has a volume");
        assert!(volume.damage > 0, "a firework that hits for nothing is a puff");
        let ambition_entity_catalog::VolumeShape::Rect {
            offset,
            half_extents,
        } = volume.shape
        else {
            panic!("the firework is a rect");
        };
        assert_eq!(
            offset.0, 0.0,
            "the door is UNDER her, so the column may not lean the way she faces",
        );
        for (point, inside, what) in [
            ((0.0, 24.0), true, "standing on the boards"),
            ((0.0, -48.0), true, "a body-height above the door"),
            ((0.0, -120.0), false, "well above the door"),
            ((120.0, 0.0), false, "a stage-width away"),
        ] {
            let hit = (point.0 - offset.0).abs() <= half_extents.0
                && (point.1 - offset.1).abs() <= half_extents.1;
            assert_eq!(hit, inside, "a body {what} is {}inside the firework", if hit { "" } else { "not " });
        }
    }

    /// ⛔ THE ARMS STRADDLE BOTH EDGES, because a rule about an interval that is
    /// only sampled inside it cannot tell "the gap is open" from "every window
    /// is 1.0". Dropping through the boards and climbing back out are still
    /// committed, and that is half of what makes the middle a decision.
    #[test]
    fn the_trap_roots_her_at_both_ends_and_lets_her_steer_between_them() {
        let set = performer_moveset();
        let trap = set
            .moves
            .iter()
            .find(|m| m.id == "performer_trapdoor")
            .expect("the trap is in the table");
        for (t, want, what) in [
            (SINK_AT_S * 0.5, 0.0, "dropping through the boards"),
            (SINK_AT_S + 0.01, 1.0, "just under"),
            (SURFACE_AT_S - 0.01, 1.0, "about to come up"),
            (SURFACE_AT_S + 0.01, 0.0, "climbing back out"),
        ] {
            assert!(
                (trap.motion_scale_at(t) - want).abs() < 1e-4,
                "at {t}s ({what}) the trap allows {} of her steering, wanted {want}",
                trap.motion_scale_at(t),
            );
        }
    }

    /// ⛔⛔ THE SPEECH HOLDS HER TOO, and for a while it held only the audience.
    ///
    /// The doc on [`the_monologue`] says *"She is held by `hitless_special`'s
    /// rooting, which every special gets"* — and the function is built from
    /// `strike`, which authors `motion_scale: 1.0` on every window it makes. So
    /// the one asymmetry the move is about ran backwards: everyone she hit was
    /// pinned by the fixed knockback and she could walk away mid-sentence.
    #[test]
    fn the_monologue_holds_the_speaker_as_well_as_the_room() {
        let set = performer_moveset();
        let speech = set
            .moves
            .iter()
            .find(|m| m.id == "performer_monologue")
            .expect("her neutral special");
        for t in [0.05, 0.3, 0.5] {
            assert_eq!(
                speech.motion_scale_at(t),
                0.0,
                "at {t}s she can still steer out of her own speech"
            );
        }
    }

    /// ⛔⛔ IT IS NOT A TELEPORT, WHICH IS THE WHOLE COMPLAINT THIS MOVE WAS
    /// REWRITTEN FOR. Jon, 2026-08-29: *"It is not a teleport and should not get
    /// the teleport sound."*
    ///
    /// ⛔⛔ AND THE ONLY ASSERTION THAT MEANS ANYTHING IS ON THE TECHNIQUE, NOT
    /// ON THE CUE. `apply_authored_teleports` emits `PLAYER_BLINK` at every
    /// transit, so a move authored through `author_teleport` makes the teleport's
    /// sound no matter what its timeline says — this move carried no `player.blink`
    /// of its own for months and Jon heard one anyway. A timeline that is merely
    /// SILENT about the cue is the shape of the bug, not the fix. What has to be
    /// true is that the move never reaches that executor.
    ///
    /// ⛔ THE EXEMPTION IN `author_teleport_blink.rs` NAMED THIS MOVE as one that
    /// *"never runs the teleport executor"* while it plainly did. That sentence
    /// is true now, and this arm is what keeps it true.
    #[test]
    fn the_wire_is_a_flyline_and_never_reaches_the_teleport_executor() {
        use ambition_characters::smash_flyline::FLYLINE;
        use ambition_characters::smash_teleport::TELEPORT;
        use ambition_platformer2d::entity_catalog::MoveEventKind;

        let set = performer_moveset();
        let wire = set
            .moves
            .iter()
            .find(|m| m.id == "performer_curtain_call")
            .expect("her up special");
        assert!(
            wire.events.iter().any(|e| matches!(
                &e.kind,
                MoveEventKind::Effect(effect) if effect.key == FLYLINE
            )),
            "the up-B authors no flyline at all, so it lifts nobody"
        );
        for event in &wire.events {
            match &event.kind {
                MoveEventKind::Effect(effect) => assert_ne!(
                    effect.key, TELEPORT,
                    "the wire is a flyline, not a teleport with a longer clip"
                ),
                MoveEventKind::Sfx { cue } => assert!(
                    !cue.contains("blink"),
                    "the wire plays `{cue}`; it is rope and pulley"
                ),
                MoveEventKind::Vfx { effect, .. } => assert!(
                    !effect.contains("glint"),
                    "the wire draws `{effect}`, which is the Author's blink"
                ),
                _ => {}
            }
        }
    }

    /// ⛔⛔ THE MOVE MUST OUTLAST THE LIFT IT AUTHORS. The wire's clock is the
    /// KERNEL's (`WireState::lift_remaining_s`), and `author_flyline` can no more
    /// see the rest of the timeline than `author_trapdoor` can — so a beat that
    /// started a 0.55s lift on a 0.46s move would leave her being flown by a
    /// maneuver nothing was animating, with her Recovery window already over.
    ///
    /// ⭐ THE SAME SHAPE AS THE TRAP'S "she goes under AND she comes back" guard,
    /// and it exists for the same reason: the half that is missing is the half a
    /// spec test cannot feel.
    #[test]
    fn the_lift_fits_inside_the_move_that_authors_it() {
        use ambition_characters::smash_flyline::{FlylineParams, FLYLINE};
        use ambition_platformer2d::entity_catalog::MoveEventKind;

        let set = performer_moveset();
        let wire = set
            .moves
            .iter()
            .find(|m| m.id == "performer_curtain_call")
            .expect("her up special");
        let (at_s, params) = wire
            .events
            .iter()
            .find_map(|ev| match &ev.kind {
                MoveEventKind::Effect(effect) if effect.key == FLYLINE => Some((
                    ev.at_s,
                    effect
                        .params
                        .hydrate::<FlylineParams>()
                        .expect("flyline params hydrate"),
                )),
                _ => None,
            })
            .expect("the up-B authors a flyline");
        assert!(
            at_s + params.lift_s <= wire.duration_s,
            "the wire catches at {at_s}s and reels for {}s, past the move's own \
             {}s",
            params.lift_s,
            wire.duration_s
        );
        // ⛔ AND THE ROPE MUST OUTLAST THE RISE, or the winch reels past its own
        // pulley and the lift stops short at the kernel's minimum length.
        assert!(
            params.rope_length > params.rise,
            "a {}px rope cannot deliver a {}px lift",
            params.rope_length,
            params.rise
        );
        // ⛔ AND THE CARRY MUST BE SLOWER THAN THE AVERAGE CLIMB, or the winch
        // would have to ACCELERATE into the release to still travel the authored
        // rise — see `apply_authored_flylines`, which clamps rather than obeying.
        assert!(
            params.release_rise < 2.0 * params.rise / params.lift_s,
            "release_rise {} is faster than the climb it is supposed to end",
            params.release_rise
        );
    }

    /// ⛔⛔ SHE STEERS ON THE WIRE, AND THE ROOT IS WHAT WOULD DELETE IT.
    ///
    /// This is the Trap's bug in a second move. `hitless_special` authors
    /// `motion_scale: 0.0` on Startup AND Recovery, the kernel's wire reads the
    /// DAMPED stick like every other mode, and `motion_scale_at` folds with
    /// `min` — so a Recovery window starting at the catch multiplies the swing by
    /// zero for the whole lift. Clause six of the ask dies, the move still rises,
    /// and every other test in this file stays green.
    ///
    /// ⛔ THE STRADDLE IS THE POINT: the beat BEFORE the catch must still be
    /// rooted. A rule that simply removed the rooting would pass a "she can
    /// steer" arm and hand her a special she can walk out of.
    #[test]
    fn she_steers_the_swing_and_is_rooted_either_side_of_it() {
        let set = performer_moveset();
        let wire = set
            .moves
            .iter()
            .find(|m| m.id == "performer_curtain_call")
            .expect("her up special");
        for t in [
            WIRE_AT_S + 0.01,
            WIRE_AT_S + LIFT_S * 0.5,
            WIRE_RELEASES_S - 0.01,
        ] {
            assert!(
                wire.motion_scale_at(t) > 0.0,
                "at {t}s her swing is multiplied by {}",
                wire.motion_scale_at(t)
            );
        }
        assert_eq!(
            wire.motion_scale_at(WIRE_AT_S - 0.01),
            0.0,
            "she can walk out of her own startup"
        );
        assert_eq!(
            wire.motion_scale_at(WIRE_RELEASES_S + 0.01),
            0.0,
            "the landing lag is not lag"
        );
    }

    /// ⛔⛔ IN THE AIR IT IS A PUFF OF SMOKE AND NOTHING ELSE.
    ///
    /// ⭐⭐ JON, 2026-08-29, the whole design: *"if you are in the air, the smoke
    /// effect still happens, but if she isn't on the ground the trap door can't
    /// open and she can't go subterranian, so the move cancels. So it doesn't do
    /// much in the air, just a poof of smoke, but that's better than no effect in
    /// the air."*
    ///
    /// ⛔ THE ABSENCES ARE THE ASSERTIONS. A trapdoor beat here would be refused
    /// by the engine and leave the REST of this timeline running — the
    /// three-second freeze most of all — with a fighter hanging motionless in
    /// mid-air and nothing on screen to explain it. Authoring none is what makes
    /// the cancel unnecessary rather than merely handled.
    #[test]
    fn the_airborne_form_is_a_puff_of_smoke_and_no_trapdoor_at_all() {
        use ambition_characters::smash_trapdoor::TRAPDOOR;
        use ambition_platformer2d::entity_catalog::MoveEventKind;

        let set = performer_moveset();
        let air = set
            .moves
            .iter()
            .find(|m| m.id == "performer_trapdoor_air")
            .expect("her airborne down special");

        assert!(
            air.events.iter().any(|ev| matches!(
                &ev.kind,
                MoveEventKind::Vfx { effect, .. } if effect == SMOKE_VFX
            )),
            "the airborne press makes no smoke, which leaves it a dead button"
        );
        assert!(
            !air.events.iter().any(|ev| matches!(
                &ev.kind,
                MoveEventKind::Effect(effect) if effect.key == TRAPDOOR
            )),
            "the airborne form authors a trapdoor beat; there is no floor to cut \
             a hatch in, and the engine refusing it leaves the rest of this \
             timeline running over a body standing in the air"
        );
        assert!(
            air.smash_charge.is_none(),
            "the airborne form authors a hold; a freeze whose beat never happens \
             is three seconds of a fighter stuck in mid-air"
        );
        assert!(
            air.duration_s <= 0.5,
            "the failing trick lasts {}s — it buys nothing, so it may not cost \
             the opponent a read that long",
            air.duration_s
        );
    }

    /// ⛔⛔ AND THE SMOKE IS ON BOTH FORMS, ON THE FIRST FRAME.
    ///
    /// ⭐⭐ IT IS THE MISDIRECTION, WHICH IS WHY IT CANNOT WAIT TO SEE WHETHER
    /// THE TRICK WORKED. Jon asked for it *"like a real play would use to
    /// disguise going through a trap door"* — so an effect that only appeared on
    /// the grounded form would tell an opponent, in the first two frames, which
    /// of the two they were watching, and delete the feint the airborne form
    /// exists to be.
    ///
    /// ⛔ AHEAD OF THE BOARDS, TOO. The door's own report is at `DOOR_OPENS_S`;
    /// smoke that arrived with it or after it would be smoke revealing a hole
    /// rather than hiding one.
    #[test]
    fn the_smoke_goes_off_on_the_first_frame_of_both_forms() {
        use ambition_platformer2d::entity_catalog::MoveEventKind;

        let set = performer_moveset();
        for id in ["performer_trapdoor", "performer_trapdoor_air"] {
            let mv = set
                .moves
                .iter()
                .find(|m| m.id == id)
                .unwrap_or_else(|| panic!("`{id}` is in her table"));
            let smoke_at = mv
                .events
                .iter()
                .find_map(|ev| match &ev.kind {
                    MoveEventKind::Vfx { effect, .. } if effect == SMOKE_VFX => Some(ev.at_s),
                    _ => None,
                })
                .unwrap_or_else(|| panic!("`{id}` makes no smoke"));
            assert_eq!(
                smoke_at, 0.0,
                "`{id}` puffs at {smoke_at}s; the misdirection is the FIRST thing \
                 that happens or it is not misdirection"
            );
            assert!(
                smoke_at < DOOR_OPENS_S,
                "`{id}` puffs at {smoke_at}s, at or after the boards give at \
                 {DOOR_OPENS_S}s — that is smoke revealing a hole, not hiding one"
            );
        }
    }

    /// ⛔⛤ EVERY EFFECT SHE NAMES IS A ROW THAT SHIPS.
    ///
    /// ⛔⛔ SHE DID NOT HAVE THIS CHECK AND FOUR OTHER FIGHTERS DID — Emmy, the
    /// Oiler, George Booul and the Pirate Admiral all run
    /// `presentation_problems(is_authored_effect)` over their tables. A
    /// capability four of five adopt is not a capability, it is a habit, and the
    /// fifth is where a defect gets to live.
    ///
    /// ⚠ AND IT WOULD NOT HAVE CAUGHT EITHER OF THE TWO BUGS JON JUST REPORTED,
    /// which is worth saying rather than letting the green tick imply otherwise.
    /// `smoke_burst` and `trapdoor_boards` are both REAL rows that ship; one was
    /// simply an explosion and the other belonged to a different move. This arm
    /// answers *"does the art exist"*, and the question that failed was *"is it
    /// the right art"* — which no oracle over the sheets can answer, and which
    /// [`the_wire_names_nothing_belonging_to_the_trapdoor`] answers for exactly
    /// one case because that is as far as a test can honestly reach.
    ///
    /// ⭐ THE ORACLE IS THE ART. `is_authored_effect` reads the rows out of the
    /// baked manifests, so this asks exactly what the renderer will ask.
    #[test]
    fn every_effect_she_names_is_a_row_that_ships() {
        let set = performer_moveset();
        for m in &set.moves {
            for problem in
                m.presentation_problems(ambition_platformer2d::sprite_sheet::fx::is_authored_effect)
            {
                panic!("{problem}");
            }
        }
    }

    /// ⛔⛔ AND HER UP-B NAMES NO PART OF HER DOWN-B. Jon, 2026-08-29: *"her up-b
    /// uses the trap door, and I don't think it should."*
    ///
    /// ⭐ THE SHAPE OF THE MISTAKE IS WORTH THE ARM: `FlylineParams::vfx` was a
    /// REQUIRED `String`, so authoring the wire meant putting SOMETHING there,
    /// and the nearest thing to hand was the other special's furniture. A field
    /// that cannot say "nothing" gets filled with something wrong.
    #[test]
    fn the_wire_names_nothing_belonging_to_the_trapdoor() {
        use ambition_platformer2d::entity_catalog::MoveEventKind;

        let set = performer_moveset();
        let wire = set
            .moves
            .iter()
            .find(|m| m.id == "performer_curtain_call")
            .expect("her up special");
        for event in &wire.events {
            if let MoveEventKind::Vfx { effect, .. } = &event.kind {
                assert!(
                    !effect.contains("trapdoor") && !effect.contains("door"),
                    "the wire draws `{effect}`, which belongs to the hatch"
                );
            }
        }
    }

    /// ⛔ THE DOWN SLOT IS A PAIR, and half a swap is a press that falls through.
    /// The archetype's down special is a `DownSpecial::ByPosture`, so
    /// `special_air_down` sits AHEAD of `special_down` in the verb chain: replace
    /// only the grounded form and an airborne press reaches the archetype's
    /// falling edge instead of the trap.
    #[test]
    fn both_postures_of_her_down_special_are_the_trap() {
        let set = performer_moveset();
        for verb in ["special_down", "special_air_down"] {
            let bound = set.verbs.get(verb).map(String::as_str);
            assert!(
                matches!(bound, Some(id) if id.starts_with("performer_trapdoor")),
                "{verb} must be the trap, saw {bound:?}"
            );
            let id = bound.unwrap();
            assert!(
                set.moves.iter().any(|m| m.id == id),
                "{verb} names `{id}`, which is not in the table"
            );
        }
    }

    /// ⛔⛔ ONE WRITER OF EXIT VELOCITY, AND FOR MONTHS THERE WERE TWO.
    ///
    /// The trap authored `LEAP_OUT_SPEED` as a `MoveEventKind::Impulse` at
    /// `SURFACE_AT_S` AND a `smash.trapdoor` surfacing beat on the same instant.
    /// The impulse is applied inline in `advance_move_playback`; the beat is a
    /// message handled by a later system whose `TransitVelocity::Zero` won every
    /// time. Stage five of a five-stage move never happened, and the authoring
    /// test was green throughout because both halves were correctly on the spec.
    ///
    /// ⇒ so the guard is not "is there an impulse" or "is there a leap" — it is
    /// that the timeline does not carry BOTH. `TrapdoorParams::leap_speed` is
    /// the authority; an `Impulse` re-added beside it would be the same bug.
    #[test]
    fn the_leap_has_one_authority_and_it_is_the_surfacing_beat() {
        use ambition_characters::smash_trapdoor::{TrapdoorParams, TRAPDOOR};
        use ambition_platformer2d::entity_catalog::MoveEventKind;
        let set = performer_moveset();
        let trap = set
            .moves
            .iter()
            .find(|m| m.id == "performer_trapdoor")
            .expect("the trap is in her table");

        assert!(
            !trap
                .events
                .iter()
                .any(|e| matches!(e.kind, MoveEventKind::Impulse { .. })),
            "the trap authors an Impulse again. It lands on the same frame as \
             the surfacing beat, which writes velocity from a LATER system, so \
             the impulse is silently deleted — put the launch on \
             `TrapdoorParams::leap_speed` instead"
        );

        let surfacing: Vec<TrapdoorParams> = trap
            .events
            .iter()
            .filter_map(|e| match &e.kind {
                MoveEventKind::Effect(effect) if effect.key == TRAPDOOR => {
                    effect.params.clone().hydrate().ok()
                }
                _ => None,
            })
            .filter(|p: &TrapdoorParams| !p.submerge)
            .collect();
        assert_eq!(
            surfacing.len(),
            1,
            "she comes back up exactly once; {} beats claim to",
            surfacing.len()
        );
        assert!(
            surfacing[0].leap_speed > 0.0,
            "she surfaces at {} px/s, so she steps up out of the hole instead of \
             leaping — the move's whole payoff is the space the leap buys",
            surfacing[0].leap_speed
        );
    }

    /// ⛔ AND THE ARCHETYPE'S DOWN SPECIAL IS GONE rather than left unreachable,
    /// where every census that walks `moves` reports it as part of her kit.
    #[test]
    fn the_archetypes_down_special_does_not_linger() {
        let set = performer_moveset();
        for stale in ["performer_low_arc", "performer_falling_edge"] {
            assert!(
                !set.moves.iter().any(|m| m.id == stale),
                "`{stale}` is the archetype's down special and must not survive \
                 the replacement"
            );
        }
    }

    /// ⛔⛔ SHE GOES UNDER AND SHE COMES BACK. Two beats of `smash.trapdoor`,
    /// and the second is the half whose absence is a fighter in
    /// `BodyMode::Submerged` for the rest of the match — invisible, intangible,
    /// and unable to be hit out of it. `author_trapdoor` cannot see the rest of
    /// the timeline, so this is where that guard lives.
    #[test]
    fn the_trap_puts_her_under_the_stage_and_brings_her_back() {
        use ambition_characters::smash_trapdoor::{TrapdoorParams, TRAPDOOR};
        use ambition_platformer2d::entity_catalog::MoveEventKind;

        let set = performer_moveset();
        // ⛔ THE GROUNDED FORM ONLY, for the reason above: the airborne one goes
        // nowhere, so it has no return trip to owe.
        for id in ["performer_trapdoor"] {
            let mv = set
                .moves
                .iter()
                .find(|m| m.id == id)
                .unwrap_or_else(|| panic!("`{id}` is in her table"));
            let beats: Vec<(f32, TrapdoorParams)> = mv
                .events
                .iter()
                .filter_map(|ev| match &ev.kind {
                    MoveEventKind::Effect(effect) if effect.key == TRAPDOOR => Some((
                        ev.at_s,
                        effect.params.hydrate().expect("trapdoor params hydrate"),
                    )),
                    _ => None,
                })
                .collect();
            assert_eq!(beats.len(), 2, "`{id}` is a round trip, not a one-way door");
            assert!(beats[0].1.submerge, "the first beat drops her through");
            assert!(!beats[1].1.submerge, "the second beat brings her back");
            assert!(
                beats[0].0 < beats[1].0,
                "`{id}` surfaces at {}s before it submerges at {}s",
                beats[1].0,
                beats[0].0
            );
            assert!(
                beats[1].0 < mv.duration_s,
                "`{id}` surfaces after the move ends, so it never surfaces"
            );
            assert!(
                beats[1].1.surface_reach > 0.0,
                "`{id}` comes up through a FLOOR; a zero reach disables the \
                 search and drops her wherever she happened to be"
            );
        }
    }

    /// ⛔⛔ IT IS NOT A BLINK, WHICH IS THE WHOLE COMPLAINT THIS MOVE WAS
    /// REWRITTEN FOR. Jon, 2026-08-27: *"It looks like a blink… It's not a
    /// blink."* The old version was `smash.teleport` with the Author's
    /// `four_point_glint` on each end, and the thing that would quietly bring it
    /// back is a copy-paste from his table.
    #[test]
    fn the_trap_carries_no_teleport_and_no_blink_dressing() {
        use ambition_characters::smash_teleport::TELEPORT;
        use ambition_platformer2d::entity_catalog::MoveEventKind;

        let set = performer_moveset();
        for id in ["performer_trapdoor", "performer_trapdoor_air"] {
            let mv = set.moves.iter().find(|m| m.id == id).expect("her trap");
            for event in &mv.events {
                match &event.kind {
                    MoveEventKind::Effect(effect) => assert_ne!(
                        effect.key, TELEPORT,
                        "`{id}` is a trapdoor, not a teleport with a longer clip"
                    ),
                    MoveEventKind::Sfx { cue } => assert!(
                        !cue.contains("blink"),
                        "`{id}` plays `{cue}`; the trap is carpentry"
                    ),
                    MoveEventKind::Vfx { effect, .. } => assert!(
                        !effect.contains("glint"),
                        "`{id}` draws `{effect}`, which is the Author's blink"
                    ),
                    _ => {}
                }
            }
        }
    }

    /// ⛔ AND SHE CANNOT BE HIT UNDER THE STAGE, stated on the TIMELINE as well
    /// as by the mode. The mode is what actually protects her; the window is
    /// what the cancel and scoring layers read without looking at a live body,
    /// and a timeline claiming she is vulnerable down there would score the move
    /// as a punishable commitment it is not.
    #[test]
    fn the_timeline_says_she_is_untouchable_for_the_whole_trip() {
        use ambition_characters::smash_trapdoor::{TrapdoorParams, TRAPDOOR};
        use ambition_platformer2d::entity_catalog::{MoveEventKind, WindowTag};

        let set = performer_moveset();
        let mv = set
            .moves
            .iter()
            .find(|m| m.id == "performer_trapdoor")
            .expect("her trap");
        let beat = |submerge: bool| -> f32 {
            mv.events
                .iter()
                .find_map(|ev| match &ev.kind {
                    MoveEventKind::Effect(effect) if effect.key == TRAPDOOR => {
                        let params: TrapdoorParams =
                            effect.params.hydrate().expect("trapdoor params");
                        (params.submerge == submerge).then_some(ev.at_s)
                    }
                    _ => None,
                })
                .expect("both beats")
        };
        let (under, up) = (beat(true), beat(false));
        assert!(
            mv.windows.iter().any(|w| matches!(w.tag, WindowTag::Invuln)
                && w.start_s <= under + 1e-4
                && w.end_s >= up - 1e-4),
            "no Invuln window covers {under}s..{up}s, the span she is not in the world"
        );
    }

    /// ⛔ AND THE RECOVERY STILL COSTS AN AIRTIME.    /// ⛔ AND THE RECOVERY STILL COSTS AN AIRTIME. `UpSpecial::Standard` stamps
    /// `gates.recovery` on the move it lowers; a replacement inserted after that
    /// lowering carries the cost itself or she gets unlimited flight.
    #[test]
    fn the_flyline_spends_the_airtimes_recovery() {
        let set = performer_moveset();
        let up = set
            .moves
            .iter()
            .find(|m| m.id == "performer_curtain_call")
            .expect("her up-B is in the table");
        assert_ne!(
            up.gates.recovery,
            ambition_platformer2d::entity_catalog::RecoveryUse::None,
            "an up-B that costs nothing is flight"
        );
    }
}
