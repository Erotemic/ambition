//! The Officer — the brawler archetype's table, under his own name, plus the
//! one move that is his.
//!
//! Unarmed for every normal, on the Pugnacious Polygon's skeleton and its clip
//! vocabulary. He throws the archetype's punches at the archetype's timings
//! because they are literally the archetype's punches; what differs is who is
//! throwing them and what the air does about it, which is the sprite sheet's
//! business and not this table's.
//!
//! See [`crate::archetype_moveset`] for why the ids are renamed rather than
//! shared or copied.

use ambition_platformer2d::entity_catalog::{MoveEvent, MoveEventKind, MoveSpec, MovesetContract};

/// ⭐⭐ THESE FOUR NUMBERS ARE READ OFF THE ART, NOT CHOSEN. `shoot.clip.json`
/// runs 12 frames at 58ms; it raises `sidearm_vis` on frame 2, marks its
/// `hitbox.active` and draws its muzzle flare on frame 6, and drops the gun back
/// to `hand_vis` on frame 11. A table that fired on any other frame would put
/// the round somewhere the flash is not, which is the one disagreement between
/// an animation and a moveset that a player sees immediately and cannot name.
const DRAW_AT_S: f32 = 0.116;
const FIRE_AT_S: f32 = 0.348;
const HOLSTER_AT_S: f32 = 0.638;
const DRAW_ENDS_S: f32 = 0.696;

/// Complete brawler-fundamentals repertoire, attributed to the Officer, with his
/// own side special in place of the archetype's shoulder rush.
pub fn officer_moveset() -> MovesetContract {
    let mut set = crate::archetype_moveset::under_own_name(
        crate::pugnacious_polygon_moveset::pugnacious_polygon_moveset(),
        &["polygon_brawler", "pugnacious_polygon"],
        "officer",
    );
    crate::special_slots::replace_special(&mut set, "special_forward", the_draw());
    crate::special_slots::replace_special(&mut set, "special_down", the_riot_shield());
    set
}

/// Down special: he plants a riot shield and EATS what is thrown at him.
///
/// ⭐⭐ A COUNTER STANCE WHOSE ANSWER IS TO SWALLOW. The engine already gives a
/// counter a reflector for free — the projectile road gates on the same
/// `parrying()` window a stance opens — so this move exists to prove the other
/// response: `absorbs_projectiles` makes the caught shot go AWAY instead of
/// back, through the one `intercept_projectile` operation the parry already
/// uses.
///
/// ⛔ AND IT IS THE RIGHT FIGHTER FOR IT. Reflecting is loud and rewards a read;
/// absorbing is quiet and rewards STANDING THERE, which is what a man with a
/// riot shield does. The Officer's other authored move is a gun — a fighter who
/// answers ranged pressure at both ends is a coherent one.
///
/// ⚠ HIS SPECIALS WERE THE BRAWLER ARCHETYPE'S until now, which is the case Jon
/// named: *"we have a lot of characters with boring specials."* This replaces a
/// borrowed generic with something only he does.
///
/// ⚠ The response is the standing grab: absorbing a shot leaves him next to
/// whoever threw it, and a stance that ate the projectile and did nothing else
/// would be a wall rather than a decision.
fn the_riot_shield() -> ambition_platformer2d::entity_catalog::MoveSpec {
    ambition_characters::smash_counter::counter_move(
        "officer_riot_shield",
        "special",
        // Slower to plant than a sword counter: this is a commitment to a
        // POSITION, not a read on one attack.
        0.12,
        0.20,
        0.38,
        ambition_characters::smash_counter::CounterParams {
            // A heartbeat, not a duration — `parry_window_timer` decays, and the
            // stance re-arms it every frame it is live.
            window_s: 0.05,
            response: ambition_platformer2d::characters::smash_capture::CAPTURE_ATTEMPT.to_string(),
            response_params: ambition_platformer2d::entity_catalog::ParamValue::from_typed(
                &ambition_platformer2d::characters::smash_capture::CaptureAttemptParams {
                    offset: (26.0, 0.0),
                    half_extents: (22.0, 22.0),
                    hold_offset: (18.0, -2.0),
                },
            )
            .expect("the riot shield's capture params serialize"),
            // ⭐ THE WHOLE POINT OF THIS MOVE.
            absorbs_projectiles: true,
        },
    )
}

/// Side special: he draws the sidearm and fires one round.
///
/// ⭐⭐ JON'S DESIGN, 2026-08-26: *"we should also polish the officer and give
/// him a side b that pulls out and shoots a gun."*
///
/// ⛔⛔ NO `equips`, AND THAT IS THE ONE PLACE THIS DIVERGES FROM THE ADMIRAL'S
/// `run_out_the_guns`, WHICH IS OTHERWISE THE TEMPLATE. His gun-sword is a
/// PROP — a separate sprite a move puts in an empty hand — so his move draws it
/// with `MoveSpec::equips`. The Officer's sidearm is part of HIS OWN SHEET: the
/// rig carries `holster` and `sidearm` parts on an opacity channel, and the
/// `shoot` clip raises them. Equipping a held item on top would register a
/// second gun and draw it beside the one his hand is already holding.
///
/// ⭐ SO THE CAPABILITY IS THE BODY'S. `MoveEventKind::Ranged` fires the owner's
/// ranged action, and with nothing brandished that is the action set his
/// character states — see `crate::authored::officer`, which is where a character
/// says what it DOES.
///
/// ⛔ NO MELEE VOLUME. This replaces `officer_shoulderrush`, a body-to-body
/// charge whose damage was a hitbox, and a move that both fired a round and
/// carried a strike would be two moves wearing one button. The round IS the
/// damage, as it is for every ranged move in the tree.
///
/// ⛔ AND NO FORWARD IMPULSE, WHICH THE SHOULDER RUSH HAD. A draw is a move you
/// plant your feet for; carrying the rush's momentum into it would make the
/// shot longest out of a run, and this move's whole read is that he stopped.
fn the_draw() -> MoveSpec {
    let mut spec = ambition_characters::moveset_authoring::hitless_special(
        "officer_the_draw",
        "shoot",
        FIRE_AT_S,
        DRAW_ENDS_S,
    );
    spec.display_name = Some("The Draw".to_string());
    spec.events.push(MoveEvent {
        at_s: FIRE_AT_S,
        kind: MoveEventKind::Ranged,
    });
    let spec = ambition_characters::moveset_authoring::sfx(spec, DRAW_AT_S, "player.attack.charge");
    // ⭐ THE SHOT'S OWN SOUND IS NOT AUTHORED HERE. A `Ranged` event routes both
    // the report and the projectile off the weapon that fired it, so a pistol
    // sounds like a pistol without this table naming a cue it would then have to
    // keep in step with the weapon.
    let spec = ambition_characters::moveset_authoring::vfx(spec, FIRE_AT_S, "muzzle_flash");
    ambition_characters::moveset_authoring::committed_tail(spec, HOLSTER_AT_S, 0.35)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ⛔⛔ THE VERB IS `special_forward`, NOT `special_side`. `SmashRepertoire`
    /// binds the four specials as `special`, `special_forward`, `special_up` and
    /// `special_down` (+ `special_air_down`), and a `replace_special` aimed at a
    /// name nothing answers to inserts a move, binds a verb no press produces,
    /// and leaves the archetype's own side special still bound — a fighter with
    /// a new move nobody can reach and an old one that still comes out.
    #[test]
    fn the_draw_answers_the_side_special_and_the_shoulder_rush_is_gone() {
        let set = officer_moveset();
        assert_eq!(
            set.verbs.get("special_forward").map(String::as_str),
            Some("officer_the_draw"),
            "his side special is the draw"
        );
        assert!(
            !set.moves.iter().any(|m| m.id.contains("shoulderrush")),
            "the archetype's shoulder rush left the table, it was not shadowed"
        );
    }

    /// ⛔⛔ THE SHOT FIRES WHERE THE ART FLASHES. `shoot.spec.json` marks frame 6
    /// active and draws the muzzle there; frame 6 at 58ms is 0.348s. This is the
    /// arm that goes red if either side moves without the other.
    #[test]
    fn the_round_leaves_on_the_frame_the_muzzle_flares() {
        let set = officer_moveset();
        let draw = set
            .moves
            .iter()
            .find(|m| m.id == "officer_the_draw")
            .expect("the draw");
        let fired: Vec<f32> = draw
            .events
            .iter()
            .filter(|ev| matches!(ev.kind, MoveEventKind::Ranged))
            .map(|ev| ev.at_s)
            .collect();
        assert_eq!(fired.len(), 1, "one draw, one round");
        assert!(
            (fired[0] - 0.348).abs() < 1e-4,
            "the round leaves at {}s and the muzzle flares at 0.348s",
            fired[0]
        );
        assert!(
            fired[0] < draw.duration_s,
            "a shot scheduled past the move's end never fires"
        );
    }

    /// ⛔ AND IT CARRIES NO STRIKE. The round is the damage; a hitbox as well
    /// would be two moves on one button, and it is the shape the shoulder rush
    /// this replaced actually had.
    #[test]
    fn the_draw_hits_nobody_with_his_body() {
        let set = officer_moveset();
        let draw = set
            .moves
            .iter()
            .find(|m| m.id == "officer_the_draw")
            .expect("the draw");
        assert!(
            draw.windows.iter().all(|w| w.volumes.is_empty()),
            "the draw is hitless — the projectile is the damage"
        );
    }
}
