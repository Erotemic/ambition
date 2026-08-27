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
//! [`ambition_characters::smash_teleport::TeleportAim`] for why one technique
//! answers both.
//!
//! See [`crate::archetype_moveset`] for why the borrowed ids are renamed rather
//! than shared or copied.

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
    set
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
