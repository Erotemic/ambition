//! The Medic: the brawler archetype's normals under her own name, and four
//! specials of her own.
//!
//! Unarmed for every normal, on the Pugnacious Polygon's skeleton and clip
//! vocabulary. She uses the archetype's punches and timings; the difference is
//! in the sprite sheet, not this table.
//!
//! Her specials are one kit with one currency: Adrenaline buys tempo with
//! health, Field Dressing buys health back with frames, Tourniquet drags a
//! fighter into punching range, and Rescue Lift costs nothing, because a
//! recovery she has to pay for kills her at the ledge.
//!
//! The timings come from the clips in `medic_triage_v1`: each effect lands on
//! the frame the art draws it, as the constants below state.
//!
//! See [`crate::archetype_moveset`] for why the normals' ids are renamed rather
//! than shared or copied.

use ambition_entity_catalog::authoring::{
    fixed_knockback, hitless_special, impulse, sfx, strike, vfx, Strike,
};
use ambition_entity_catalog::smash_vitality::{author_vitality, VitalityParams};
use ambition_entity_catalog::{ImpulseMode, MoveSpec, MovesetContract};

/// Adrenaline, from `special.clip.json`: 10 frames at 70ms; the injector goes
/// into her thigh on frame 5.
const INJECT_AT_S: f32 = 0.35;
const INJECT_ENDS_S: f32 = 0.70;

/// Field Dressing, from `charge.clip.json`: 8 frames at 110ms, with the mend
/// plume from frame 1 to frame 7. The dressing takes hold in the middle, not
/// on the first frame.
const DRESS_AT_S: f32 = 0.44;
const DRESS_ENDS_S: f32 = 0.88;

/// Tourniquet, from `shoot.clip.json`: 8 frames at 60ms, strap live on frames
/// 3 and 4.
const STRAP_STARTUP_S: f32 = 0.18;
const STRAP_ACTIVE_S: f32 = 0.12;
const STRAP_RECOVER_S: f32 = 0.18;

/// Rescue Lift, from `fly.clip.json`: 8 frames at 60ms. She drops, then goes.
const LIFT_AT_S: f32 = 0.12;
const LIFT_ENDS_S: f32 = 0.48;

/// What one press of Adrenaline costs, and the margin it will not spend.
///
/// The floor is the move's safety: pressing it must never end a stock, so at
/// low health it is free, not fatal. See `BodyHealth::spend`.
const ADRENALINE_COST: i32 = 1;
const ADRENALINE_FLOOR: i32 = 1;

/// What Field Dressing gives back: more than one Adrenaline press costs, for
/// nearly three times as long. The exchange rate is the character.
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
/// The tempo is the cancel. There is no stat-buff system (no haste, no damage
/// multiplier), so what she buys is frame advantage: the tail cancels into a
/// smash, a special or a jab, so a point of health becomes acting first.
///
/// The cancel is `Always`, not `OnHit`: the move touches nobody.
///
/// The targets use the cancel namespace. The jab slot binds the verb `attack`
/// (there is no `jab` verb). `smash` resolves because `cancel_names_for` gives
/// a smash press `["smash", "attack", "any_attack"]`.
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
    let spec = ambition_entity_catalog::authoring::cancelable(
        spec,
        INJECT_AT_S + 0.07,
        INJECT_ENDS_S,
        &["smash", "special", "attack"],
        ambition_entity_catalog::CancelCondition::Always,
    );
    sfx(spec, 0.0, "player.attack.charge")
}

/// Side special: a strap comes off her belt and goes out flat; what it catches,
/// it drags back.
///
/// The pull is `launch_dir`, not a new mechanic. A launch direction is
/// authored per move and mirrors with facing, so aiming it back along her
/// axis launches the victim toward her.
///
/// Small damage on purpose: the payoff is position. An unarmed brawler's
/// punches only matter in a range the opponent avoids; this takes that choice
/// away. A drag that also hurt would be a command grab with no commitment.
fn tourniquet() -> MoveSpec {
    let mut spec = strike(Strike {
        id: "medic_tourniquet",
        clip: "shoot",
        startup_s: STRAP_STARTUP_S,
        active_s: STRAP_ACTIVE_S,
        recover_s: STRAP_RECOVER_S,
        // Out flat and far: the reach is the move.
        offset: (46.0, -4.0),
        half_extents: (34.0, 12.0),
        damage: 4,
        knockback: 96.0,
        // It must not grow with damage: a growing drag would pull a softened
        // fighter in less. The builder stores zero as `None` ("the stage decides"),
        // so `fixed_knockback` below is what actually fixes it.
        knockback_growth: 0.0,
        // Back along her axis and slightly down: toward her, and onto the ground
        // where her normals work.
        launch_dir: Some((-1.0, 0.22)),
        on_hit: None,
    });
    spec.display_name = Some("Tourniquet".to_string());
    // This makes the drag flat: only a volume carrying `Some(0.0)` is fixed.
    // See `knockback_growth` above.
    let spec = fixed_knockback(spec);
    let spec = sfx(spec, STRAP_STARTUP_S, "player.slash");
    ambition_entity_catalog::authoring::on_contact(spec, "player.hit")
}

/// Down special: she goes to one knee and holds pressure on her own ribs.
///
/// The maintainer asked for a self-healing move for the Medic; this is it,
/// and it is why `smash.vitality` exists (`BodyHealth::heal` was otherwise
/// reachable only from pickups and shrines).
///
/// It also heals the damage meter, which decides how far the next hit sends
/// her. `BodyHealth::heal` repays both.
///
/// Slow enough to punish: 0.88s rooted and hitting nobody.
fn field_dressing() -> MoveSpec {
    dressing("medic_field_dressing")
}

/// Two ids for one move: the archetype's down special is a
/// `DownSpecial::ByPosture` pair, and a half-replaced slot falls through to
/// the neutral special. Both forms use the same authoring.
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
/// Wrap through the slot so `gates.recovery` is stamped: this is inserted
/// after `SmashRepertoire::into_contract`, and an up-B that costs nothing is
/// flight.
///
/// It is the one special that charges her no health: a recovery with a
/// health price would only collect it at the ledge, when it is fatal.
fn rescue_lift() -> MoveSpec {
    let mut spec = hitless_special("medic_rescue_lift", "fly", LIFT_AT_S, LIFT_ENDS_S);
    spec.display_name = Some("Rescue Lift".to_string());
    // Mostly vertical, with a little drift to correct a bad angle.
    let spec = impulse(spec, LIFT_AT_S, (34.0, -905.0), ImpulseMode::Set);
    let spec = sfx(spec, 0.0, "player.attack.charge");
    let spec = vfx(spec, LIFT_AT_S, "classic_burst");
    ambition_entity_catalog::smash_repertoire::UpSpecial::Standard(spec).into_spec()
}

#[cfg(test)]
mod tourniquet_tests {

    /// The drag does not weaken as the victim takes damage.
    ///
    /// `strike` stores a builder growth of zero as `None`, and `None` means the
    /// ruleset's growth applies. This asserts `Some(0.0)`, not "small": a
    /// magnitude check would pass a plausible small growth.
    #[test]
    fn the_tourniquet_pulls_the_same_at_every_percent() {
        let strap = crate::authored_movesets::shipped("medic")
            .move_by_id("medic_tourniquet")
            .expect("medic_tourniquet exists")
            .clone();
        let volumes: Vec<_> = strap.windows.iter().flat_map(|w| w.volumes.iter()).collect();
        assert!(!volumes.is_empty(), "the strap still has a hitbox");
        for volume in volumes {
            assert_eq!(
                volume.knockback_growth,
                Some(0.0),
                "`None` here means the stage decides, which is the bug this \
                 move's own comment describes"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_entity_catalog::smash_vitality::{VitalityParams, VITALITY};
    use ambition_entity_catalog::MoveEventKind;

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

    /// All five special verbs are hers. `special_air_down` comes before
    /// `special_down` in the brawler's chain, so replacing only the grounded half
    /// would send an airborne press to the archetype's move.
    #[test]
    fn every_special_slot_answers_to_a_move_of_her_own() {
        let set = crate::authored_movesets::shipped("medic");
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

    /// She pays for one and is paid by the other. One technique serves both, so
    /// the risk is a sign error: an Adrenaline that healed would be a free press
    /// with a cancel window.
    #[test]
    fn adrenaline_costs_and_the_dressing_gives_back() {
        let set = crate::authored_movesets::shipped("medic");
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
        // The exchange rate: one heal costs more frames than the presses it
        // repays.
        assert!(
            heal.change > -cost.change,
            "a dressing that gave back less than one adrenaline costs makes the \
             kit a treadmill"
        );
    }

    /// The floor is never zero on a price. `VitalityParams::floor` defaults to
    /// `0` and the engine clamps it to `1`, but content should not rely on a clamp
    /// elsewhere to disagree with it.
    #[test]
    fn her_price_names_a_floor_that_cannot_kill_her() {
        let set = crate::authored_movesets::shipped("medic");
        assert!(
            vitality_of(&set, "medic_adrenaline").floor >= 1,
            "a neutral special that can finish a stock by being pressed is not \
             a cost"
        );
    }

    /// The strap pulls. With the default launch direction it would send the
    /// victim away, and nothing else would look wrong.
    #[test]
    fn the_tourniquet_launches_its_victim_toward_her() {
        let set = crate::authored_movesets::shipped("medic");
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

    /// The recovery is free: a price on a recovery is only collected at the
    /// ledge.
    #[test]
    fn the_rescue_lift_costs_her_nothing() {
        let set = crate::authored_movesets::shipped("medic");
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
