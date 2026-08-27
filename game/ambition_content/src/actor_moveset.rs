//! The Actor — the sword archetype's table, with four specials of her own.
//!
//! ⭐ AND SHE CARRIES NO SWORD. The Pointed Polygon's frame data retargets onto
//! her for the reason the Author's does: his pen occupies the arming sword's
//! exact axis, and her conjured blade of stage light occupies it too — authored
//! as the swing's own axis extended past her hand, so the reach the table
//! assumes is the reach the sheet draws.
//!
//! ⭐⭐ HER SPECIALS ARE STAGE MACHINERY, and two of them are the same technique
//! pointed at different destinations. The TRAP puts her behind whoever she is
//! fighting; the FLYLINE takes her straight up out of the scene. Both are
//! `smash.teleport`, which is why neither needed an engine of its own — see
//! [`TeleportParams::behind_nearest_foe`] for why one technique answers both.
//!
//! ⭐ THE OTHER TWO ARE STAGECRAFT WITHOUT MACHINERY. The MONOLOGUE and THE
//! LINE are plain strikes; what makes them hers is the SHAPE the art gave them,
//! and neither needed a technique to say it.
//!
//! See [`crate::archetype_moveset`] for why the borrowed ids are renamed rather
//! than shared or copied.

use ambition_characters::moveset_authoring::{fixed_knockback, on_contact, sfx, strike, Strike};
use ambition_characters::smash_teleport::{author_teleport, TeleportParams};
use ambition_platformer2d::entity_catalog::{MoveSpec, MovesetContract};

/// When the trap opens, and when the move lets go of her. The boards give on
/// frame 2 of `blink_out` at 52ms a frame, so the teleport fires where the art
/// says she has already gone through the floor.
const TRAP_AT_S: f32 = 0.10;
const TRAP_ENDS_S: f32 = 0.42;

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

/// Down special: she goes through the boards and comes up behind you.
fn trapdoor(id: &str, clip: &str) -> MoveSpec {
    let mut spec =
        ambition_characters::moveset_authoring::hitless_special(id, clip, TRAP_AT_S, TRAP_ENDS_S);
    spec.display_name = Some("The Trap".to_string());
    let spec = author_teleport(
        spec,
        TRAP_AT_S,
        TeleportParams {
            // ⭐ JON, 2026-08-27: *"The down special can just teleport behind the
            // nearest enemy, that is fine. We can improve it later."* So the
            // destination is a BODY, not a direction.
            behind_nearest_foe: true,
            // A stride, not a hair's breadth: she arrives in range to act and
            // not already overlapping him.
            behind_gap: 18.0,
            // ⛔ THE RANGE IS WHAT KEEPS IT A SPECIAL. Without it the move is a
            // stage-wide snap to whoever exists; with it, an opponent who keeps
            // his distance is simply not a target and she has spent the frames.
            // About four body-lengths — far enough to punish someone spacing
            // her out, short enough that he can leave.
            distance: 320.0,
            // ⛔ NO LEDGE ASSIST. That radius exists to save a RECOVERY aimed at
            // a platform edge; an ambush that quietly hopped onto a ledge she
            // did not aim at would be the assist choosing her position for her.
            ledge_assist: 0.0,
            depart_vfx: "four_point_glint".to_string(),
            arrive_vfx: "four_point_glint".to_string(),
        },
    );
    let spec = ambition_characters::moveset_authoring::sfx(spec, 0.0, "player.attack.charge");
    let spec = ambition_characters::moveset_authoring::sfx(spec, TRAP_AT_S, "player.blink");
    spec
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
            depart_vfx: "four_point_glint".to_string(),
            arrive_vfx: "four_point_glint".to_string(),
        },
    );
    let spec = ambition_characters::moveset_authoring::sfx(spec, 0.0, "player.attack.charge");
    let spec = ambition_characters::moveset_authoring::sfx(spec, WIRE_AT_S, "player.blink");
    // ⛔⛔ THROUGH THE SLOT, so it costs what an up-B costs. Inserted after
    // `SmashRepertoire::into_contract` has lowered the table it joins, nothing
    // else will stamp `gates.recovery` on it — and an up-B that spends nothing
    // is flight.
    ambition_characters::smash_repertoire::UpSpecial::Standard(spec).into_spec()
}

#[cfg(test)]
mod tests {
    use super::*;

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

    /// The trap aims at a BODY. A teleport authored `Aimed` would send her
    /// wherever the stick happened to be, which is the recovery's rule and not
    /// this move's.
    #[test]
    fn the_trap_aims_behind_the_nearest_foe_and_the_flyline_does_not() {
        use ambition_characters::smash_teleport::{TeleportParams, TELEPORT};
        use ambition_platformer2d::entity_catalog::MoveEventKind;

        let set = actor_moveset();
        let params_of = |id: &str| -> TeleportParams {
            let mv = set
                .moves
                .iter()
                .find(|m| m.id == id)
                .unwrap_or_else(|| panic!("`{id}` is in her table"));
            let event = mv
                .events
                .iter()
                .find_map(|e| match &e.kind {
                    MoveEventKind::Effect(effect) if effect.key == TELEPORT => Some(effect),
                    _ => None,
                })
                .unwrap_or_else(|| panic!("`{id}` authors a teleport"));
            event.params.hydrate().expect("teleport params hydrate")
        };
        assert!(
            params_of("actor_trapdoor").behind_nearest_foe,
            "the trap puts her behind somebody"
        );
        assert!(
            !params_of("actor_curtain_call").behind_nearest_foe,
            "the flyline is a recovery and is aimed"
        );
    }

    /// ⛔⛔ ALL FIVE SPECIAL VERBS ARE HERS, and the two added last are the ones
    /// most easily left half-done: `special` and `special_forward` are the
    /// repertoire's own names for the neutral and side slots, and a
    /// `replace_special` aimed at `special_neutral` or `special_side` — the two
    /// names `SPECIAL_VERBS` used to carry — binds a press nobody makes and
    /// leaves the archetype's move still coming out.
    #[test]
    fn every_special_slot_answers_to_a_move_of_her_own() {
        let set = actor_moveset();
        for (verb, expected) in [
            ("special", "actor_monologue"),
            ("special_forward", "actor_the_line"),
            ("special_down", "actor_trapdoor"),
            ("special_air_down", "actor_trapdoor_air"),
            ("special_up", "actor_curtain_call"),
        ] {
            assert_eq!(
                set.verbs.get(verb).map(String::as_str),
                Some(expected),
                "`{verb}` must answer to her own move"
            );
        }
    }

    /// ⛔⛔ THE MONOLOGUE HOLDS EVERYONE THE SAME, and that is `Some(0.0)` and
    /// not `None`. Unauthored growth means *the stage decides*, and the stage
    /// scales a launch with the victim's percent — so the speech would land
    /// differently depending on how the match had gone, which is the one thing
    /// it must not do. The two values are one character apart in the source and
    /// produce opposite moves.
    #[test]
    fn the_monologue_lands_the_same_at_every_percent() {
        let set = actor_moveset();
        let speech = set
            .moves
            .iter()
            .find(|m| m.id == "actor_monologue")
            .expect("the monologue");
        let growths: Vec<Option<f32>> = speech
            .windows
            .iter()
            .flat_map(|w| w.volumes.iter())
            .map(|v| v.knockback_growth)
            .collect();
        assert!(!growths.is_empty(), "the monologue has a live volume");
        for growth in growths {
            assert_eq!(
                growth,
                Some(0.0),
                "fixed knockback is the move; `None` would hand it to the stage's \
                 percent-scaled rule"
            );
        }
    }

    /// ⛔ THE PAIRED ARM: her side special is an ORDINARY spacing tool and wants
    /// the stage's own growth. Fixed knockback is the monologue's exception, and
    /// an arm that only checked the exception would pass against a table that
    /// had frozen every hit she owns.
    #[test]
    fn the_line_still_grows_with_its_victims_damage() {
        let set = actor_moveset();
        let line = set
            .moves
            .iter()
            .find(|m| m.id == "actor_the_line")
            .expect("the line");
        for volume in line.windows.iter().flat_map(|w| w.volumes.iter()) {
            assert!(
                volume.knockback_growth.is_some_and(|g| g > 0.0),
                "the line grows with its victim's damage like every other \
                 normal, and it authored {:?}",
                volume.knockback_growth
            );
        }
        // ⭐ AND ITS REACH IS THE MOVE. `shoot.spec.json` extends the drawn axis
        // to 2.9 because the danger is out past her hand where nothing is drawn;
        // a table that pulled the volume back onto her arm would make the
        // animation honest and the move pointless.
        let reach = line
            .windows
            .iter()
            .flat_map(|w| w.volumes.iter())
            .filter_map(|v| match v.shape {
                ambition_platformer2d::entity_catalog::VolumeShape::Rect {
                    offset,
                    half_extents,
                } => Some(offset.0 + half_extents.0),
                _ => None,
            })
            .fold(f32::MIN, f32::max);
        assert!(
            reach > 80.0,
            "the line must out-range her own arm, and it reaches {reach}"
        );
    }

    /// ⛔ AND THE RECOVERY STILL COSTS AN AIRTIME. `UpSpecial::Standard` stamps
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
