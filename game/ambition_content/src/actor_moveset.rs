//! The Actor — the sword archetype's table, with four specials of her own.
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
use ambition_characters::smash_teleport::{author_teleport, TeleportParams};
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
/// 5. `SURFACE_AT_S` — she LEAPS OUT: an upward impulse, the firework, and a
///    hitbox over the door.
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
/// How hard she LEAPS out of the boards, body-local and against gravity.
///
/// ⛔ A LEAP, NOT A STEP UP. She is coming out of a hole in the stage under her
/// own power and the move's payoff is the space it buys; a body that surfaced
/// standing still would be handing the position straight back.
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

/// The emergence. ⛔ NOT the boards: those are the door opening, and this is
/// what comes out of it.
const FIREWORK_VFX: &str = "starstuff_burst";

/// The flyline catches her later than the trap drops her: the wire goes taut
/// before it pulls, which is the beat `fly`'s first two frames draw.
const WIRE_AT_S: f32 = 0.12;
const WIRE_ENDS_S: f32 = 0.46;

/// Complete sword-fundamentals repertoire, attributed to the Actor, with her
/// own down and up specials in place of the archetype's.
pub fn actor_moveset() -> MovesetContract {
    let mut set = crate::archetype_moveset::under_own_name(
        crate::pointed_polygon_moveset::pointed_polygon_moveset(),
        &["polygon", "pointed_polygon"],
        "actor",
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
        id: "actor_monologue",
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
        id: "actor_the_line",
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

/// ⛔ TWO IDS FOR ONE MOVE, because the archetype's down special is a
/// `DownSpecial::ByPosture` pair and a slot left half-replaced is a press that
/// falls through to the neutral special. The trap means the same thing in both
/// postures — the boards are wherever she is — so both forms are the same
/// authoring with different ids.
fn the_trap() -> MoveSpec {
    trapdoor("actor_trapdoor", "blink_out")
}

fn the_trap_airborne() -> MoveSpec {
    trapdoor("actor_trapdoor_air", "blink_out")
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
    // ⭐⭐ STAGE FIVE: SHE LEAPS OUT. The impulse is what makes it a leap rather
    // than a body appearing on the boards, and it lands on the same instant as
    // the surfacing so the placement and the launch cannot disagree about where
    // she left from. Body-local, against gravity.
    let spec = ambition_characters::moveset_authoring::impulse(
        spec,
        SURFACE_AT_S,
        (0.0, -LEAP_OUT_SPEED),
        ambition_entity_catalog::ImpulseMode::Set,
    );
    let spec = author_trapdoor(
        spec,
        SURFACE_AT_S,
        TrapdoorParams {
            submerge: false,
            surface_reach: SURFACE_REACH,
            // ⛔ THE BOARDS ALREADY OPENED, one beat ago and on their own event.
            // Asking for them again here would draw a second set over the first
            // on the frame she comes through.
            vfx: FIREWORK_VFX.to_string(),
            sfx: "world.door.heavy_open".to_string(),
        },
    );
    // ⛔ NOT A BLINK CUE ANYWHERE ON IT. The trap is carpentry: the boards give,
    // they bang shut behind her, and they give again somewhere else.
    let spec = ambition_characters::moveset_authoring::sfx(spec, DOOR_OPENS_S, "world.door.open");
    let spec =
        ambition_characters::moveset_authoring::sfx(spec, SINK_AT_S + 0.06, "world.door.close");
    let spec =
        ambition_characters::moveset_authoring::sfx(spec, TRAP_ENDS_S - 0.06, "world.door.close");
    ambition_characters::moveset_authoring::invuln(spec, SINK_AT_S, SURFACE_AT_S)
}

/// Up special: a wire catches her at the waist and takes her out of the scene.
fn the_flyline() -> MoveSpec {
    let mut spec = ambition_characters::moveset_authoring::hitless_special(
        "actor_curtain_call",
        "fly",
        WIRE_AT_S,
        WIRE_ENDS_S,
    );
    spec.display_name = Some("Curtain Call".to_string());
    let spec = author_teleport(
        spec,
        WIRE_AT_S,
        TeleportParams {
            // Aimed, like every other recovery in the game.
            behind_nearest_foe: false,
            behind_gap: 0.0,
            // Shorter than the Author's 250: his is a revision and hers is a
            // stagehand, and a wire runs out.
            distance: 215.0,
            // ⭐⭐ THE SAME RADIUS THE AUTHOR AND THE ROBOT GET. It is a property
            // of recovering onto a stage rather than of any one fighter.
            ledge_assist: 44.0,
            // ⭐ THE SAME WINDOW THE OTHER TWO GET. She already authors i-frames
            // on the trapdoor, where being underground is the reason; this is the
            // other one — being NOWHERE, mid-wire.
            intangible_s: 0.12,
            depart_vfx: "four_point_glint".to_string(),
            arrive_vfx: "four_point_glint".to_string(),
        },
    );
    let spec = ambition_characters::moveset_authoring::sfx(spec, 0.0, "player.attack.charge");
    // ⛔⛔ NO `player.blink` HERE. This move is authored through
    // `author_teleport` twelve lines up, and `apply_authored_teleports` emits
    // `PLAYER_BLINK` at every transit — so a cue on this timeline would ask for
    // the same sound down a second road on the same frame. The executor is the
    // one authority; a move that runs it does not also ask.
    // ⛔⛔ THROUGH THE SLOT, so it costs what an up-B costs. Inserted after
    // `SmashRepertoire::into_contract` has lowered the table it joins, nothing
    // else will stamp `gates.recovery` on it — and an up-B that spends nothing
    // is flight.
    ambition_characters::smash_repertoire::UpSpecial::Standard(spec).into_spec()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ⛔⛔ SHE STEERS UNDER THE STAGE, AND FOR A WHILE SHE DID NOT.
    ///
    /// The Trap is built on `hitless_special`, which roots the body across the
    /// WHOLE duration — Startup and Recovery both author `motion_scale: 0.0` —
    /// and `MoveSpec::motion_scale_at` folds overlapping windows with `min`. So
    /// the submerged second, the beat this move exists FOR, ran with the
    /// player's steering multiplied by zero while the module doc said *"no
    /// gravity, no geometry — and still steering"*.
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
        let set = actor_moveset();
        for id in ["actor_trapdoor", "actor_trapdoor_air"] {
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
        let set = actor_moveset();
        let trap = set
            .moves
            .iter()
            .find(|m| m.id == "actor_trapdoor")
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
        let set = actor_moveset();
        let trap = set
            .moves
            .iter()
            .find(|m| m.id == "actor_trapdoor")
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
        let set = actor_moveset();
        let speech = set
            .moves
            .iter()
            .find(|m| m.id == "actor_monologue")
            .expect("her neutral special");
        for t in [0.05, 0.3, 0.5] {
            assert_eq!(
                speech.motion_scale_at(t),
                0.0,
                "at {t}s she can still steer out of her own speech"
            );
        }
    }

    /// ⛔⛔ ONE TELEPORT, ONE BLINK — and the wire was asking twice.
    ///
    /// `apply_authored_teleports` emits `PLAYER_BLINK` at every transit, which is
    /// exactly what D255/R17 established for the Author's Revision. Her up-B runs
    /// that executor (it is authored through `author_teleport`) AND carried a
    /// `player.blink` on its own timeline at `WIRE_AT_S`, so the same frame asked
    /// for the same cue down both roads.
    ///
    /// ⛔ THE EXEMPTION IN `author_teleport_blink.rs` NAMED THIS MOVE as one that
    /// *"never runs the teleport executor"*, which was simply not true of it —
    /// so the note written to explain why a duplicate was not a duplicate was
    /// covering a real one. This arm is here so the sentence cannot drift back.
    #[test]
    fn the_wire_leaves_the_blink_cue_to_the_teleport_executor() {
        use ambition_characters::smash_teleport::TELEPORT;
        use ambition_platformer2d::entity_catalog::MoveEventKind;

        let set = actor_moveset();
        let wire = set
            .moves
            .iter()
            .find(|m| m.id == "actor_curtain_call")
            .expect("her up special");
        assert!(
            wire.events.iter().any(|e| matches!(
                &e.kind,
                MoveEventKind::Effect(effect) if effect.key == TELEPORT
            )),
            "the wire must still BE a teleport, or this arm proves nothing"
        );
        assert!(
            !wire.events.iter().any(|e| matches!(
                &e.kind,
                MoveEventKind::Sfx { cue } if cue == "player.blink"
            )),
            "the wire authors its own blink beside the executor's"
        );
    }

    /// ⛔ THE DOWN SLOT IS A PAIR, and half a swap is a press that falls through.
    /// The archetype's down special is a `DownSpecial::ByPosture`, so
    /// `special_air_down` sits AHEAD of `special_down` in the verb chain: replace
    /// only the grounded form and an airborne press reaches the archetype's
    /// falling edge instead of the trap.
    #[test]
    fn both_postures_of_her_down_special_are_the_trap() {
        let set = actor_moveset();
        for verb in ["special_down", "special_air_down"] {
            let bound = set.verbs.get(verb).map(String::as_str);
            assert!(
                matches!(bound, Some(id) if id.starts_with("actor_trapdoor")),
                "{verb} must be the trap, saw {bound:?}"
            );
            let id = bound.unwrap();
            assert!(
                set.moves.iter().any(|m| m.id == id),
                "{verb} names `{id}`, which is not in the table"
            );
        }
    }

    /// ⛔ AND THE ARCHETYPE'S DOWN SPECIAL IS GONE rather than left unreachable,
    /// where every census that walks `moves` reports it as part of her kit.
    #[test]
    fn the_archetypes_down_special_does_not_linger() {
        let set = actor_moveset();
        for stale in ["actor_low_arc", "actor_falling_edge"] {
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

        let set = actor_moveset();
        for id in ["actor_trapdoor", "actor_trapdoor_air"] {
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

        let set = actor_moveset();
        for id in ["actor_trapdoor", "actor_trapdoor_air"] {
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

        let set = actor_moveset();
        let mv = set
            .moves
            .iter()
            .find(|m| m.id == "actor_trapdoor")
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
        let set = actor_moveset();
        let up = set
            .moves
            .iter()
            .find(|m| m.id == "actor_curtain_call")
            .expect("her up-B is in the table");
        assert_ne!(
            up.gates.recovery,
            ambition_platformer2d::entity_catalog::RecoveryUse::None,
            "an up-B that costs nothing is flight"
        );
    }
}
