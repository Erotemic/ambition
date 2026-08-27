//! The Medic — the brawler archetype's normals, under her own name, and four
//! specials that are hers.
//!
//! Unarmed for every normal, on the Pugnacious Polygon's skeleton and its clip
//! vocabulary. She throws the archetype's punches at the archetype's timings
//! because they are literally the archetype's punches; what differs is who is
//! throwing them and what the air does about it, which is the sprite sheet's
//! business and not this table's.
//!
//! ⭐⭐ HER SPECIALS ARE A KIT, NOT FOUR MOVES. Every one of them trades in the
//! same currency: ADRENALINE buys tempo with health, FIELD DRESSING buys health
//! back with frames, TOURNIQUET drags a fighter into the range where her
//! archetype's punches are worth something, and RESCUE LIFT is the only one that
//! spends nothing — because a recovery a character has to pay for is a character
//! who dies at the ledge.
//!
//! ⛔ THE TIMINGS ARE READ OFF THE CLIPS. `medic_triage_v1` has carried all four
//! since it was forked, captioned with what each one is; the frame each effect
//! lands on is the frame the art draws it on, and the constants below say which.
//!
//! See [`crate::archetype_moveset`] for why the normals' ids are renamed rather
//! than shared or copied.

use ambition_characters::moveset_authoring::{
    hitless_special, impulse, sfx, strike, vfx, Strike,
};
use ambition_characters::smash_vitality::{author_vitality, VitalityParams};
use ambition_platformer2d::entity_catalog::{ImpulseMode, MoveSpec, MovesetContract};

/// ADRENALINE, from `special.clip.json`: 10 frames at 70ms, and the injector
/// goes into her thigh on frame 5.
const INJECT_AT_S: f32 = 0.35;
const INJECT_ENDS_S: f32 = 0.70;

/// FIELD DRESSING, from `charge.clip.json`: 8 frames at 110ms, with the mend
/// plume rising from frame 1 through frame 7. The dressing takes hold in the
/// middle of that, not at its first frame — she has to get her hands on it.
const DRESS_AT_S: f32 = 0.44;
const DRESS_ENDS_S: f32 = 0.88;

/// TOURNIQUET, from `shoot.clip.json`: 8 frames at 60ms, strap live on frames
/// 3 and 4.
const STRAP_STARTUP_S: f32 = 0.18;
const STRAP_ACTIVE_S: f32 = 0.12;
const STRAP_RECOVER_S: f32 = 0.18;

/// RESCUE LIFT, from `fly.clip.json`: 8 frames at 60ms. She drops, then goes.
const LIFT_AT_S: f32 = 0.12;
const LIFT_ENDS_S: f32 = 0.48;

/// What one press of ADRENALINE costs, and the margin it will not spend.
///
/// ⛔ THE FLOOR IS THE WHOLE SAFETY OF THE MOVE. A neutral special that can
/// finish a stock by being pressed is not a cost, and a Medic mashing it at low
/// percent must find it free rather than fatal — see
/// `BodyHealth::spend`, which states why free is the right failure.
const ADRENALINE_COST: i32 = 1;
const ADRENALINE_FLOOR: i32 = 1;

/// What FIELD DRESSING gives back. More than one press of ADRENALINE costs, and
/// it takes nearly three times as long: the exchange rate is the character.
const DRESSING_HEAL: i32 = 2;

/// Complete brawler-fundamentals repertoire, attributed to the Medic, with her
/// own four specials in place of the archetype's.
pub fn medic_moveset() -> MovesetContract {
    let mut set = crate::archetype_moveset::under_own_name(
        crate::pugnacious_polygon_moveset::pugnacious_polygon_moveset(),
        &["polygon_brawler", "pugnacious_polygon"],
        "medic",
    );
    crate::special_slots::replace_special(&mut set, "special", adrenaline());
    crate::special_slots::replace_special(&mut set, "special_forward", tourniquet());
    crate::special_slots::replace_special(&mut set, "special_down", field_dressing());
    crate::special_slots::replace_special(&mut set, "special_air_down", field_dressing_airborne());
    crate::special_slots::replace_special(&mut set, "special_up", rescue_lift());
    set
}

/// Neutral special: she spends her own margin to buy tempo.
///
/// ⭐⭐ THE TEMPO IS THE CANCEL, and it is made of things the engine already
/// has. There is no stat-buff system in this repository — no haste, no damage
/// multiplier, nothing a "+20% speed for four seconds" could be written against
/// — and inventing one to give a neutral-B a payoff would have been a whole
/// mechanic bolted to one move. What she actually buys is FRAME ADVANTAGE: the
/// tail of this move cancels into a smash, a special or a jab, so a point of
/// health converts directly into acting first.
///
/// ⛔ THE CANCEL IS `Always`, NOT `OnHit`. The move touches nobody — an
/// on-connect condition would never be satisfied and the window would be
/// decoration.
fn adrenaline() -> MoveSpec {
    let mut spec = hitless_special("medic_adrenaline", "special", INJECT_AT_S, INJECT_ENDS_S);
    spec.display_name = Some("Adrenaline".to_string());
    let spec = author_vitality(
        spec,
        INJECT_AT_S,
        VitalityParams {
            change: -ADRENALINE_COST,
            floor: ADRENALINE_FLOOR,
            vfx: "classic_burst".to_string(),
            sfx: "player.attack.charge".to_string(),
        },
    );
    let spec = ambition_characters::moveset_authoring::cancelable(
        spec,
        INJECT_AT_S + 0.07,
        INJECT_ENDS_S,
        &["smash", "special", "jab"],
        ambition_platformer2d::entity_catalog::CancelCondition::Always,
    );
    sfx(spec, 0.0, "player.attack.charge")
}

/// Side special: a strap comes off her belt and goes out flat; what it catches,
/// it drags back.
///
/// ⭐⭐ THE PULL IS `launch_dir`, NOT A NEW MECHANIC. A hit's launch direction
/// is already authored per move and already mirrors with facing, so aiming it
/// back along her own axis is a drag — the victim is launched TOWARD her instead
/// of away. That is the whole move, and it needs nothing the engine did not
/// already do for every other strike.
///
/// ⛔ AND ITS DAMAGE IS SMALL ON PURPOSE. The payoff is the position, not the
/// number: an unarmed brawler's punches are only worth something in a range her
/// opponent chooses to stay out of, and this is how she takes that choice away.
/// A drag that also hurt would be a command grab with no commitment.
fn tourniquet() -> MoveSpec {
    let mut spec = strike(Strike {
        id: "medic_tourniquet",
        clip: "shoot",
        startup_s: STRAP_STARTUP_S,
        active_s: STRAP_ACTIVE_S,
        recover_s: STRAP_RECOVER_S,
        // Out flat and a long way — the reach IS the move.
        offset: (46.0, -4.0),
        half_extents: (34.0, 12.0),
        damage: 4,
        knockback: 96.0,
        // ⛔ IT DOES NOT GROW WITH DAMAGE. Every other knockback in the game
        // launches a hurt fighter FARTHER, which for a drag would mean the more
        // she has softened someone up the less she can pull them in — the move
        // getting worse exactly as it starts to matter.
        knockback_growth: 0.0,
        // Back along her own axis and slightly down: toward her, and onto the
        // ground where her normals live.
        launch_dir: Some((-1.0, 0.22)),
        on_hit: None,
    });
    spec.display_name = Some("Tourniquet".to_string());
    let spec = sfx(spec, STRAP_STARTUP_S, "player.slash");
    ambition_characters::moveset_authoring::on_contact(spec, "player.hit")
}

/// Down special: she goes to one knee and holds pressure on her own ribs.
///
/// ⭐⭐ JON, 2026-08-26: *"the medic could have a self healing move."* This is
/// it, and it is the reason `smash.vitality` exists at all — `BodyHealth::heal`
/// was reachable by a pickup and by a shrine and by nothing a fighter could
/// press.
///
/// ⛔ IT HEALS THE METER, WHICH IS THE PART THAT MATTERS IN A MATCH. Refilling
/// the pool alone would be a heal she could not feel: the accumulated-damage
/// meter is what decides how far the next hit sends her, and `BodyHealth::heal`
/// repays both. That is stated where it lives rather than here.
///
/// ⛔⛔ AND IT IS SLOW ENOUGH TO PUNISH. 0.88s rooted, hitting nobody, is most
/// of a second of standing still in front of somebody who wants to hit you —
/// which is what stops two points of health from being free.
fn field_dressing() -> MoveSpec {
    dressing("medic_field_dressing")
}

/// ⛔ TWO IDS FOR ONE MOVE, because the archetype's down special is a
/// `DownSpecial::ByPosture` pair and a slot left half-replaced is a press that
/// falls through to the neutral special. Holding pressure on your own ribs means
/// the same thing in the air as on the ground — it is just a worse idea — so
/// both forms are the same authoring under different ids.
fn field_dressing_airborne() -> MoveSpec {
    dressing("medic_field_dressing_air")
}

fn dressing(id: &str) -> MoveSpec {
    let mut spec = hitless_special(id, "charge", DRESS_AT_S, DRESS_ENDS_S);
    spec.display_name = Some("Field Dressing".to_string());
    let spec = author_vitality(
        spec,
        DRESS_AT_S,
        VitalityParams {
            change: DRESSING_HEAL,
            floor: 0,
            vfx: "classic_burst".to_string(),
            sfx: "player.heal".to_string(),
        },
    );
    sfx(spec, 0.0, "player.attack.charge")
}

/// Up special: she drops, then goes straight up under whatever is above her.
///
/// ⛔⛔ THROUGH THE SLOT, so it costs what an up-B costs. Inserted after
/// `SmashRepertoire::into_contract` has lowered the table it joins, nothing else
/// will stamp `gates.recovery` on it — and an up-B that spends nothing is
/// flight.
///
/// ⛔ AND IT IS THE ONE SPECIAL THAT CHARGES HER NOTHING. Every other move in
/// this kit trades health for something; a recovery she has to pay for is a
/// character who cannot come back from the ledge at the moment she most needs
/// to, which is a cost that only ever lands when it is fatal.
fn rescue_lift() -> MoveSpec {
    let mut spec = hitless_special("medic_rescue_lift", "fly", LIFT_AT_S, LIFT_ENDS_S);
    spec.display_name = Some("Rescue Lift".to_string());
    // Mostly vertical, with just enough drift to correct a bad angle — arms
    // locked overhead is not a pose you steer with.
    let spec = impulse(spec, LIFT_AT_S, (34.0, -905.0), ImpulseMode::Set);
    let spec = sfx(spec, 0.0, "player.attack.charge");
    let spec = vfx(spec, LIFT_AT_S, "classic_burst");
    ambition_characters::smash_repertoire::UpSpecial::Standard(spec).into_spec()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_characters::smash_vitality::{VitalityParams, VITALITY};
    use ambition_platformer2d::entity_catalog::MoveEventKind;

    fn vitality_of(set: &MovesetContract, id: &str) -> VitalityParams {
        set.moves
            .iter()
            .find(|m| m.id == id)
            .unwrap_or_else(|| panic!("`{id}` is in the table"))
            .events
            .iter()
            .find_map(|ev| match &ev.kind {
                MoveEventKind::Effect(effect) if effect.key == VITALITY => {
                    Some(effect.params.hydrate().expect("vitality params hydrate"))
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("`{id}` authors a health change"))
    }

    /// ⛔⛔ ALL FIVE SPECIAL VERBS ARE HERS. `special_air_down` sits AHEAD of
    /// `special_down` in the brawler's chain, so replacing only the grounded
    /// half would leave an airborne press falling through to the archetype's
    /// falling edge — the exact half-replacement the Actor's trap had to learn.
    #[test]
    fn every_special_slot_answers_to_a_move_of_her_own() {
        let set = medic_moveset();
        for (verb, expected) in [
            ("special", "medic_adrenaline"),
            ("special_forward", "medic_tourniquet"),
            ("special_down", "medic_field_dressing"),
            ("special_air_down", "medic_field_dressing_air"),
            ("special_up", "medic_rescue_lift"),
        ] {
            assert_eq!(
                set.verbs.get(verb).map(String::as_str),
                Some(expected),
                "`{verb}` must answer to her own move"
            );
        }
        assert!(
            !set.moves.iter().any(|m| m.id.starts_with("medic_polygon")
                || m.id.contains("shoulderrush")
                || m.id.contains("uppercut")),
            "the archetype's specials left the table, they were not shadowed"
        );
    }

    /// ⛔⛔ SHE PAYS FOR ONE AND IS PAID BY THE OTHER, AND THE SIGNS ARE
    /// OPPOSITE. A single technique serves both, so the one thing that could go
    /// wrong silently is a sign — an Adrenaline that healed would be a free
    /// press with a cancel window, which is the strongest move in the game.
    #[test]
    fn adrenaline_costs_and_the_dressing_gives_back() {
        let set = medic_moveset();
        let cost = vitality_of(&set, "medic_adrenaline");
        assert!(
            cost.change < 0,
            "adrenaline is a PRICE, and it authored {}",
            cost.change
        );
        let heal = vitality_of(&set, "medic_field_dressing");
        assert!(
            heal.change > 0,
            "the dressing is a RESTORE, and it authored {}",
            heal.change
        );
        assert_eq!(
            vitality_of(&set, "medic_field_dressing_air").change,
            heal.change,
            "both halves of the down slot mend the same amount"
        );
        // ⭐ AND THE EXCHANGE RATE IS THE CHARACTER: one press back costs more
        // frames than the presses it repays.
        assert!(
            heal.change > -cost.change,
            "a dressing that gave back less than one adrenaline costs makes the \
             kit a treadmill"
        );
    }

    /// ⛔⛔ THE FLOOR IS NEVER ZERO ON A PRICE. `VitalityParams::floor` defaults
    /// to `0` and the engine clamps it up to `1`, but a table that authored `0`
    /// would be saying "this may take her last point" — and relying on a clamp
    /// somewhere else to disagree with what the content says is how the two
    /// drift apart.
    #[test]
    fn her_price_names_a_floor_that_cannot_kill_her() {
        let set = medic_moveset();
        assert!(
            vitality_of(&set, "medic_adrenaline").floor >= 1,
            "a neutral special that can finish a stock by being pressed is not \
             a cost"
        );
    }

    /// ⛔⛔ THE STRAP PULLS. A drag authored with the default launch direction
    /// is an ordinary poke that sends its victim AWAY — the exact opposite of
    /// the move — and nothing about the timings or the reach would look wrong.
    #[test]
    fn the_tourniquet_launches_its_victim_toward_her() {
        let set = medic_moveset();
        let strap = set
            .moves
            .iter()
            .find(|m| m.id == "medic_tourniquet")
            .expect("the tourniquet");
        let dirs: Vec<(f32, f32)> = strap
            .windows
            .iter()
            .flat_map(|w| w.volumes.iter())
            .filter_map(|v| v.launch_dir)
            .collect();
        assert!(!dirs.is_empty(), "the strap has a live volume");
        for dir in dirs {
            assert!(
                dir.0 < 0.0,
                "the strap must launch back along her own axis, and it authored {dir:?}"
            );
        }
    }

    /// ⛔ AND THE RECOVERY IS FREE. Every other special she has charges her
    /// something; a recovery with a price only ever collects it at the ledge.
    #[test]
    fn the_rescue_lift_costs_her_nothing() {
        let set = medic_moveset();
        let lift = set
            .moves
            .iter()
            .find(|m| m.id == "medic_rescue_lift")
            .expect("the lift");
        assert!(
            !lift.events.iter().any(|ev| matches!(
                &ev.kind,
                MoveEventKind::Effect(effect) if effect.key == VITALITY
            )),
            "her way home is the one move that does not take health"
        );
    }
}
