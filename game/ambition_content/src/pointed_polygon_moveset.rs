//! Pointed Polygon — sword archetype repertoire.
//!
//! A fundamentals table on purpose. The character is also an animation
//! reference for future humanoids, so every common Smash verb has a clear,
//! conventional answer. The distinctive choice is reach: the sword extends
//! ordinary humanoid spacing without making her a heavyweight or a projectile
//! character.

use ambition_entity_catalog::authoring::Strike;
use ambition_entity_catalog::authoring::{committed_tail, impulse, multihit, strike, Pulse};
use ambition_entity_catalog::smash_capture::{
    author_pummel, author_standing_grab, author_throw, capture_beat, grab_shell,
    CaptureAttemptParams, CaptureCues, CapturePummelParams, CaptureThrowParams,
    SmashCaptureRepertoire,
};
use ambition_entity_catalog::smash_repertoire::{
    DownSpecial, NeutralSpecial, SmashRepertoire, UpSpecial,
};
use ambition_entity_catalog::AutolinkVolume;
use ambition_entity_catalog::{ImpulseMode, MovesetContract};

/// Complete sword-fundamentals repertoire: every typed Smash slot plus all four throws.
pub fn pointed_polygon_moveset() -> MovesetContract {
    // Grounded normals.
    let jab = strike(Strike {
        id: "polygon_jab",
        clip: "jab",
        startup_s: 0.05,
        active_s: 0.05,
        recover_s: 0.12,
        offset: (30.0, -1.0),
        half_extents: (23.0, 13.0),
        damage: 3,
        knockback: 48.0,
        knockback_growth: 1.05,
        launch_dir: Some((1.0, -0.18)),
        on_hit: None,
    });
    let forward_tilt = strike(Strike {
        id: "polygon_tilt_forward",
        clip: "attack_side",
        startup_s: 0.08,
        active_s: 0.07,
        recover_s: 0.17,
        offset: (36.0, -3.0),
        half_extents: (27.0, 14.0),
        damage: 5,
        knockback: 72.0,
        knockback_growth: 1.38,
        launch_dir: Some((1.0, -0.28)),
        on_hit: None,
    });
    let up_tilt = strike(Strike {
        id: "polygon_tilt_up",
        clip: "attack_up",
        startup_s: 0.08,
        active_s: 0.07,
        recover_s: 0.17,
        offset: (10.0, -30.0),
        half_extents: (22.0, 23.0),
        damage: 5,
        knockback: 78.0,
        knockback_growth: 1.45,
        launch_dir: Some((0.15, -1.0)),
        on_hit: None,
    });
    let down_tilt = strike(Strike {
        id: "polygon_tilt_down",
        clip: "attack_down",
        startup_s: 0.07,
        active_s: 0.07,
        recover_s: 0.16,
        offset: (30.0, 11.0),
        half_extents: (25.0, 10.0),
        damage: 4,
        knockback: 58.0,
        knockback_growth: 1.22,
        launch_dir: Some((1.0, -0.18)),
        on_hit: None,
    });

    // Smashes: legible, committed kill swings.
    let mut forward_smash = strike(Strike {
        id: "polygon_smash_forward",
        clip: "smash_forward",
        startup_s: 0.24,
        active_s: 0.08,
        recover_s: 0.30,
        offset: (44.0, -4.0),
        half_extents: (31.0, 19.0),
        damage: 14,
        knockback: 148.0,
        knockback_growth: 3.05,
        launch_dir: Some((1.0, -0.36)),
        on_hit: None,
    });
    forward_smash.smash_charge_mult = 1.7;
    let mut up_smash = strike(Strike {
        id: "polygon_smash_up",
        clip: "smash_up",
        startup_s: 0.22,
        active_s: 0.08,
        recover_s: 0.29,
        offset: (4.0, -34.0),
        half_extents: (24.0, 29.0),
        damage: 13,
        knockback: 146.0,
        knockback_growth: 5.82,
        launch_dir: Some((0.0, -1.0)),
        on_hit: None,
    });
    up_smash.smash_charge_mult = 1.7;
    let mut down_smash = strike(Strike {
        id: "polygon_smash_down",
        clip: "smash_down",
        startup_s: 0.20,
        active_s: 0.09,
        recover_s: 0.31,
        offset: (0.0, 13.0),
        half_extents: (38.0, 12.0),
        damage: 12,
        knockback: 132.0,
        knockback_growth: 2.72,
        launch_dir: Some((0.85, -0.52)),
        on_hit: None,
    });
    down_smash.smash_charge_mult = 1.7;

    // Aerials.
    let neutral_air = strike(Strike {
        id: "polygon_air_neutral",
        clip: "air_neutral",
        startup_s: 0.06,
        active_s: 0.10,
        recover_s: 0.15,
        offset: (9.0, 0.0),
        half_extents: (27.0, 22.0),
        damage: 6,
        knockback: 70.0,
        knockback_growth: 1.42,
        launch_dir: None,
        on_hit: None,
    });
    let forward_air = strike(Strike {
        id: "polygon_air_forward",
        clip: "air_forward",
        startup_s: 0.09,
        active_s: 0.08,
        recover_s: 0.18,
        offset: (36.0, -4.0),
        half_extents: (27.0, 17.0),
        damage: 8,
        knockback: 98.0,
        knockback_growth: 1.94,
        launch_dir: Some((1.0, -0.34)),
        on_hit: None,
    });
    let back_air = strike(Strike {
        id: "polygon_air_back",
        clip: "air_back",
        startup_s: 0.10,
        active_s: 0.07,
        recover_s: 0.20,
        offset: (-31.0, -1.0),
        half_extents: (24.0, 16.0),
        damage: 9,
        knockback: 116.0,
        knockback_growth: 2.20,
        launch_dir: Some((-1.0, -0.32)),
        on_hit: None,
    });
    let up_air = strike(Strike {
        id: "polygon_air_up",
        clip: "air_up",
        startup_s: 0.07,
        active_s: 0.08,
        recover_s: 0.15,
        offset: (2.0, -30.0),
        half_extents: (22.0, 23.0),
        damage: 7,
        knockback: 91.0,
        knockback_growth: 1.80,
        launch_dir: Some((0.0, -1.0)),
        on_hit: None,
    });
    let mut down_air = strike(Strike {
        id: "polygon_air_down",
        clip: "air_down",
        startup_s: 0.12,
        active_s: 0.08,
        recover_s: 0.23,
        offset: (4.0, 27.0),
        half_extents: (20.0, 21.0),
        damage: 9,
        knockback: 118.0,
        knockback_growth: 2.25,
        launch_dir: Some((0.0, 1.0)),
        on_hit: None,
    });
    down_air.landing_lag_s = Some(0.24);

    // Specials deliberately teach common sword archetype motion.
    // Neutral: `polygon_point`, a thrust with a tipper.
    //
    // A thrust whose far end hits harder rewards spacing: the same button pokes
    // up close and kills at range. The strike seam takes the first authored
    // volume that reaches (`StrikeRank` order), so the tip is authored first.
    //
    // `director_moveset` borrows this table, so the Director's
    // `director_point` is this move too.
    let neutral_special = committed_tail(
        strike(Strike {
            id: "polygon_point",
            clip: "slash",
            startup_s: 0.14,
            active_s: 0.08,
            recover_s: 0.24,
            offset: (48.0, -3.0),
            half_extents: (28.0, 12.0),
            damage: 10,
            knockback: 112.0,
            knockback_growth: 2.15,
            launch_dir: Some((1.0, -0.18)),
            on_hit: None,
        }),
        0.52,
        0.20,
    );
    let neutral_special = ambition_entity_catalog::authoring::tipper(
        neutral_special,
        ambition_entity_catalog::authoring::Tip {
            // The far 28px of the thrust, reaching 80px where the base reaches 76. The
            // sweetspot is the tip's whole 28px, not the 4px the base misses: the tip is
            // authored first, so anywhere it covers is a tip hit.
            offset: (66.0, -3.0),
            half_extents: (14.0, 10.0),
            // Engine-unit launch, like the `Strike` above, not a `DamageBoxEffect` feel
            // multiplier. The base launches at 112; the tip launches harder and grows
            // faster.
            damage: 14,
            knockback: 132.0,
            knockback_growth: Some(2.4),
            launch_dir: Some((1.0, -0.22)),
        },
    );

    let side_special = impulse(
        committed_tail(
            strike(Strike {
                id: "polygon_vector_lunge",
                clip: "attack_side",
                startup_s: 0.13,
                active_s: 0.10,
                recover_s: 0.24,
                offset: (41.0, -2.0),
                half_extents: (30.0, 16.0),
                damage: 9,
                knockback: 104.0,
                knockback_growth: 2.02,
                launch_dir: Some((1.0, -0.28)),
                on_hit: None,
            }),
            0.58,
            0.12,
        ),
        0.13,
        (520.0, 0.0),
        ImpulseMode::Set,
    );

    // The rising spin: swords out horizontally, four holding pulses, then one
    // launch.
    //
    // Four autolink pulses make the rise the mechanic: each one re-aims the
    // victim at the spin's centre, so the victim rises with the move and the
    // finisher has something to launch.
    //
    // Not a capture: each pulse is a weak hit whose reaction aims inward. The
    // victim keeps every verb (DI, tech) and falls out when the pulses stop
    // reaching.
    //
    // The shape is a disk. Jon: *"The attack volume should form a broad disk /
    // horizontal spinning envelope around her, rather than reading like a narrow
    // ordinary strike."* So it is wider than tall and centred on the body: a
    // spin has no front.
    let mut rising_edge = strike(Strike {
        id: "polygon_rising_edge",
        // The side swing, not the overhead one: the sword-horizontal pose is
        // already drawn. Her sheet's hitbox polys show `attack_side` spans x 53→100
        // at torso height and `attack_up` spans x 76→196 up and away. A disk wants
        // the first.
        clip: "attack_side",
        startup_s: 0.09,
        // The finisher uses the disk too: a victim gathered on her back side must
        // be inside the launching box.
        active_s: 0.10,
        recover_s: 0.20,
        offset: (0.0, -14.0),
        half_extents: (44.0, 22.0),
        damage: 7,
        knockback: 88.0,
        knockback_growth: 1.65,
        launch_dir: Some((0.10, -1.0)),
        on_hit: None,
    });
    rising_edge.landing_lag_s = Some(0.25);
    let rising_edge = multihit(
        rising_edge,
        4,
        Pulse {
            // The disk: slightly wider than the finisher and level with the torso. Her
            // forward smash reaches x≈75 with one sword; two swords held out sideways
            // reach about that far each way.
            offset: (0.0, -12.0),
            half_extents: (48.0, 24.0),
            damage: 2,
            // Separate windows: the runtime's re-hit rule treats touching windows as
            // one track, so four touching windows would land once.
            active_s: 0.035,
            gap_s: 0.030,
            autolink: AutolinkVolume {
                // The spin's own centre, with x zero on purpose. `autolink_anchor_world`
                // mirrors the anchor with facing, so a non-zero x would make the gather
                // point depend on which way she looks. Zero pulls victims toward her from
                // either side.
                anchor: (0.0, -10.0),
                // The whole climb. The correction only closes a gap, and she rises at
                // 760 px/s; anything less leaves the victim below the move.
                carry: 1.0,
                pull: 22.0,
                max_speed: 900.0,
            },
        },
    );
    // The recovery cost comes from the up-B slot (`UpSpecial::Standard`) for
    // every fighter, as Jon asked for the whole roster. The budget returns when
    // the body is re-seated: landing, catching the ledge, being grabbed, a
    // respawn.
    let mut up_special = rising_edge;
    // A crude spin read by mirroring the sprite, on purpose. Jon: *"it is
    // acceptable to fake the spin by repeatedly flipping the sprite
    // horizontally."* Twelve mirrors a second over a ~0.5s move is about six
    // flips: fast enough to read as turning, slow enough to see her facing.
    up_special.sprite_spin_hz = Some(12.0);
    let up_special = impulse(up_special, 0.09, (0.0, -760.0), ImpulseMode::Set);

    // Down (grounded): `polygon_riposte`. Jon: *"Swordies will get a counter."*
    // It replaces `polygon_low_arc`, a plain low swipe. A counter is the
    // conventional answer for this archetype, so the table's "fundamentals"
    // rule holds.
    //
    // It answers with the blade. `counter_move` builds a stance with no volumes,
    // and the answer is its response technique; `smash.riposte_strike` supplies
    // a cut.
    //
    // The reach makes it hers: the cut is centred 52px out and spans 18..86px,
    // further than the 52px swipe it replaced (18px offset + 34px half-width).
    //
    // It starts 18px out, so it does not overlap her body. The real rule is
    // `HitSide::Player` + `FollowOwner`, which excludes the owner by identity.
    //
    // The recovery is the price: 0.42s against a 0.15s stance, close to the
    // riposte's 0.44/0.16. A counter must not be free to throw out on
    // reaction to nothing.
    let grounded_down_special = ambition_entity_catalog::smash_counter::counter_move(
        "polygon_riposte",
        "attack_down",
        0.07,
        0.15,
        0.42,
        ambition_entity_catalog::smash_counter::CounterParams {
            // A heartbeat, not a duration: `parry_window_timer` decays, and the stance
            // re-arms it every live frame. Three ticks of slack at 60Hz.
            window_s: 0.05,
            // Her own answer: the cut comes from her blade, aimed by her facing, not
            // placed on the attacker.
            answers_the_attacker: false,
            response: ambition_entity_catalog::smash_riposte::RIPOSTE_STRIKE.to_string(),
            response_params: ambition_entity_catalog::ParamValue::from_typed(
                &ambition_entity_catalog::smash_riposte::RiposteStrikeParams {
                    // Harder than the swipe it replaces (8): it must be earned by reading the
                    // swing.
                    damage: 12,
                    // A feel multiplier, not a launch speed. The old swipe's `knockback:
                    // 82.0` was a `Strike` speed; do not copy it here.
                    knockback: 1.4,
                    reach: 52.0,
                    half_extents: (34.0, 14.0),
                    lifetime_s: 0.08,
                    // A blade sound. The spawner used to set `strike_sfx: None`, so this cut
                    // and the brawler's ground shock both played the victim's material sound.
                    hit_sfx: Some("player.slash".to_string()),
                },
            )
            .expect("the riposte cut's params serialize"),
            // She returns shots: a swordfighter batting a projectile back. Absorbing is
            // the Officer's stance.
            absorbs_projectiles: false,
        },
    );
    let mut airborne_down_special = strike(Strike {
        id: "polygon_falling_edge",
        clip: "air_down",
        startup_s: 0.10,
        active_s: 0.10,
        recover_s: 0.23,
        offset: (0.0, 25.0),
        half_extents: (21.0, 22.0),
        damage: 9,
        knockback: 105.0,
        knockback_growth: 1.95,
        launch_dir: Some((0.0, 1.0)),
        on_hit: None,
    });
    airborne_down_special.landing_lag_s = Some(0.27);
    let airborne_down_special =
        impulse(airborne_down_special, 0.10, (0.0, 1050.0), ImpulseMode::Set);

    // Capture kit. Unlike several older fighters, the reference archetype answers
    // every throw direction so animation authors have a safe pose for all four.
    let grab = author_standing_grab(
        grab_shell("polygon_grab", "grab", 0.07, 0.05, 0.22),
        CaptureAttemptParams {
            offset: (16.0, 1.0),
            half_extents: (19.0, 15.0),
            hold_offset: (15.0, 3.0),
        },
    );
    let pummel = author_pummel(
        capture_beat("polygon_pummel", "pummel", 0.16),
        0.07,
        CapturePummelParams { damage: 3 },
    );
    let forward_throw = author_throw(
        capture_beat("polygon_throw_forward", "throw_forward", 0.25),
        0.12,
        CaptureThrowParams {
            damage: 7,
            knockback: 108.0,
            knockback_growth: 2.15,
            launch_dir: (1.0, -0.34),
        },
    );
    let back_throw = author_throw(
        capture_beat("polygon_throw_back", "throw_back", 0.27),
        0.13,
        CaptureThrowParams {
            damage: 8,
            knockback: 116.0,
            knockback_growth: 2.25,
            launch_dir: (-1.0, -0.30),
        },
    );
    let up_throw = author_throw(
        capture_beat("polygon_throw_up", "throw_up", 0.26),
        0.12,
        CaptureThrowParams {
            damage: 7,
            knockback: 112.0,
            knockback_growth: 2.18,
            launch_dir: (0.0, -1.0),
        },
    );
    let down_throw = author_throw(
        capture_beat("polygon_throw_down", "throw_down", 0.28),
        0.13,
        CaptureThrowParams {
            damage: 6,
            knockback: 82.0,
            knockback_growth: 1.75,
            launch_dir: (0.35, -0.92),
        },
    );

    SmashRepertoire {
        // See `select.rs` for the same shape: a stale copy is a revert with no diff to review.
        taunt: ambition_entity_catalog::authoring::taunt("pointed_polygon_taunt", 0.9),
        // The genre shape on purpose: this is the reference rig, so its dash
        // attack is the one a new humanoid should copy.
        dash_attack: ambition_entity_catalog::authoring::dash_attack(
            "pointed_polygon_dash_attack",
            ambition_entity_catalog::authoring::DashAttackShape::GENRE,
            8,
            90.0,
        ),
        jab,
        forward_tilt,
        up_tilt,
        down_tilt,
        forward_smash,
        up_smash,
        down_smash,
        neutral_air,
        forward_air,
        back_air,
        up_air,
        down_air,
        neutral_special: NeutralSpecial::Authored(neutral_special),
        side_special,
        up_special: UpSpecial::Standard(up_special),
        down_special: DownSpecial::ByPosture {
            grounded: grounded_down_special,
            airborne: airborne_down_special,
        },
        capture: SmashCaptureRepertoire {
            cues: CaptureCues::GENERIC,
            grab,
            pummel,
            forward_throw,
            back_throw: Some(back_throw),
            up_throw: Some(up_throw),
            down_throw: Some(down_throw),
        },
    }
    .into_contract()
}

#[cfg(test)]
mod tests {

    /// The thrust's tip is authored first. The strike seam takes the first
    /// authored volume that reaches (`StrikeRank { window, volume }` order), so a
    /// tip appended after the base would lose every exchange where both reach.
    #[test]
    fn his_thrusts_tip_outranks_its_base_and_is_worth_spacing_for() {
        let set = pointed_polygon_moveset();
        let thrust = set
            .moves
            .iter()
            .find(|m| m.id == "polygon_point")
            .expect("his neutral-B");
        let window = thrust
            .windows
            .iter()
            .find(|w| {
                w.tag == ambition_entity_catalog::WindowTag::Active
                    && !w.volumes.is_empty()
            })
            .expect("the thrust has an active window");
        assert_eq!(
            window.volumes.len(),
            2,
            "the thrust authors {} volume(s); a tipper is two — a tip and the \
             base it outranks",
            window.volumes.len(),
        );
        let tip = &window.volumes[0];
        let base = &window.volumes[1];
        assert!(
            tip.shape.leading_edge_x() > base.shape.leading_edge_x(),
            "the FIRST-authored volume reaches {}px and the second {}px — the \
             tip is the far one, so authoring them the other way round makes the \
             sourspot win every exchange where both reach",
            tip.shape.leading_edge_x(),
            base.shape.leading_edge_x(),
        );
        assert!(
            tip.damage > base.damage && tip.knockback > base.knockback,
            "the tip ({} dmg / {} kb) is not stronger than the base ({} / {}), \
             so the spacing it asks the player to learn buys them nothing",
            tip.damage,
            tip.knockback,
            base.damage,
            base.knockback,
        );
    }

    /// Jon: *"Swordies will get a counter."* This test names her so it stays
    /// true for this fighter.
    ///
    /// It checks the answer, not just the stance: a retuned harmless response
    /// would still pass a check for `smash.counter` alone.
    #[test]
    fn his_down_b_is_a_counter_that_answers_with_the_blade() {
        use ambition_entity_catalog::smash_riposte::{RiposteStrikeParams, RIPOSTE_STRIKE};

        let set = pointed_polygon_moveset();
        let stance = set
            .moves
            .iter()
            .find(|m| m.id == "polygon_riposte")
            .expect("his grounded down-B is the riposte");
        let counter: ambition_entity_catalog::smash_counter::CounterParams = stance
            .windows
            .iter()
            .filter_map(|window| window.sustain_effect.as_ref())
            .find(|effect| effect.key == ambition_entity_catalog::smash_counter::COUNTER)
            .and_then(|effect| effect.params.hydrate().ok())
            .expect("the stance carries a counter");

        assert_eq!(
            counter.response, RIPOSTE_STRIKE,
            "his counter answers with `{}` rather than the blade",
            counter.response,
        );
        let cut: RiposteStrikeParams = counter
            .response_params
            .hydrate()
            .expect("the cut's params hydrate");

        // The authoring check at test time. The ruleset refuses an unusable cut at
        // runtime and only logs it.
        assert!(
            cut.problems().is_empty(),
            "his riposte authors an unusable cut: {}",
            cut.problems().join("; "),
        );
        // The replaced swipe covered 18 + 34 = 52px. The counter must reach at
        // least that far: reach is this table's distinction.
        assert!(
            cut.reach + cut.half_extents.0 >= 52.0,
            "his counter reaches {}px, less than the low arc it replaced",
            cut.reach + cut.half_extents.0,
        );
    }
    use super::*;

    /// The authored fighter reaches the recovery budget, through the real
    /// moveset function, repertoire and lowering.
    ///
    /// Generic unit tests on `afford_recovery`, `start_move` and
    /// `body_is_helpless` cannot show that authored content reaches them; a
    /// fixture would miss a lowering that drops the field or a rule that is
    /// opt-in. So this uses `pointed_polygon_moveset()`.
    #[test]
    fn the_authored_up_b_costs_the_recovery_and_ends_in_freefall() {
        let set = pointed_polygon_moveset();
        let id = set
            .verbs
            .get("special_up")
            .expect("the pointed polygon bound no up-B verb");
        let up_b = set
            .moves
            .iter()
            .find(|m| &m.id == id)
            .expect("the up-B verb names a move the contract does not carry");
        assert_eq!(
            up_b.gates.recovery,
            ambition_entity_catalog::RecoveryUse::SpendAndFreefall,
            "the pointed polygon's rising spin costs nothing, so she can press \
             it forever and can only be killed by a launch that outruns her"
        );
    }

    /// The Up-B's holding pulses, as authored: `(offset, half_extents, autolink)`.
    fn rising_spin_pulses() -> Vec<((f32, f32), (f32, f32), AutolinkVolume)> {
        let spin = pointed_polygon_moveset()
            .moves
            .into_iter()
            .find(|m| m.id == "polygon_rising_edge")
            .expect("the up-special is authored");
        spin.windows
            .iter()
            .flat_map(|w| w.volumes.iter())
            .filter_map(|v| {
                let link = v.autolink()?;
                match v.shape {
                    ambition_entity_catalog::VolumeShape::Rect {
                        offset,
                        half_extents,
                    } => Some((offset, half_extents, link)),
                    _ => None,
                }
            })
            .collect()
    }

    /// Is a victim standing at `local` (in Pointed's own frame) inside the
    /// holding pulse?
    fn caught_at(local: (f32, f32)) -> bool {
        rising_spin_pulses().iter().any(|(offset, half, _)| {
            (local.0 - offset.0).abs() <= half.0 && (local.1 - offset.1).abs() <= half.1
        })
    }

    /// The up-B is a disk, not a poke (D206).
    ///
    /// Jon: *"Pointed extends her swords approximately horizontally. The attack
    /// volume should form a broad disk / horizontal spinning envelope around her,
    /// rather than reading like a narrow ordinary strike."*
    ///
    /// The claim is "wider than tall", not a pair of numbers: an assertion on the
    /// literal extents would only restate the authoring.
    #[test]
    fn the_rising_spin_is_wider_than_it_is_tall() {
        let pulses = rising_spin_pulses();
        assert!(!pulses.is_empty(), "the up-special authored no held pulses");
        for (offset, half, _) in &pulses {
            assert!(
                half.0 > half.1,
                "a holding pulse is {}x{} — taller than it is wide, which reads \
                 as a strike rather than a spin",
                half.0 * 2.0,
                half.1 * 2.0,
            );
            assert_eq!(
                offset.0, 0.0,
                "the disk is offset sideways by {}, so it is in FRONT of her \
                 rather than around her — a spin has no front",
                offset.0,
            );
        }
    }

    /// It catches both sides.
    ///
    /// Jon: *"victim near Pointed's left side → multihit catches/carries; victim
    /// near Pointed's right side → multihit catches/carries; victim somewhat
    /// above/below center → broad disk still reads sensibly."*
    ///
    /// Far points are asserted too: a victim well past the swords is out, so a
    /// stage-wide hitbox would fail.
    #[test]
    fn the_rising_spin_gathers_from_either_side_and_stops_somewhere() {
        // Beside her, at torso height.
        assert!(
            caught_at((-34.0, -12.0)),
            "a victim on her BACK side is outside the spin"
        );
        assert!(
            caught_at((34.0, -12.0)),
            "a victim in FRONT of her is outside the spin"
        );
        // Somewhat above and below centre.
        assert!(
            caught_at((0.0, -30.0)),
            "a victim above her centre is outside the spin"
        );
        assert!(
            caught_at((0.0, 6.0)),
            "a victim at her feet is outside the spin"
        );
        // And it ends: about two body-widths out each way is outside.
        assert!(
            !caught_at((-120.0, -12.0)),
            "the spin reaches most of the stage to her left"
        );
        assert!(
            !caught_at((120.0, -12.0)),
            "the spin reaches most of the stage to her right"
        );
    }

    /// The gather point does not depend on her facing.
    ///
    /// `autolink_anchor_world` mirrors an anchor with facing, which is right for
    /// a poke and wrong for a spin. A non-zero x would gather victims to whichever
    /// side she faces.
    ///
    /// Asked through the engine's resolver, not by reading `anchor.0`, so a change
    /// to the mirroring rule is caught.
    #[test]
    fn the_gather_point_is_the_same_whichever_way_she_faces() {
        use ambition_platformer2d_core::hit_response::autolink_anchor_world;
        use ambition_platformer2d_core::Vec2;

        const HER: Vec2 = Vec2::new(300.0, 200.0);
        const DOWN: Vec2 = Vec2::new(0.0, 1.0);

        for (_, _, link) in rising_spin_pulses() {
            let authored = Vec2::new(link.anchor.0, link.anchor.1);
            let facing_right = autolink_anchor_world(authored, HER, 1.0, DOWN);
            let facing_left = autolink_anchor_world(authored, HER, -1.0, DOWN);
            assert_eq!(
                facing_right, facing_left,
                "the spin gathers to a different point depending on her facing"
            );
            assert!(
                facing_right.distance(HER) < 24.0,
                "the gather point is {:?}, which is not ON her — a spin pulls \
                 victims into itself",
                facing_right,
            );
        }
    }

    /// The finisher covers what the pulses gathered. If the pulses are wider than
    /// the launch, a victim carried in on her back side rides the climb and
    /// falls out unlaunched.
    #[test]
    fn the_launch_reaches_everything_the_pulses_held() {
        let spin = pointed_polygon_moveset()
            .moves
            .into_iter()
            .find(|m| m.id == "polygon_rising_edge")
            .expect("the up-special is authored");
        // The finisher is the one volume that authors a launch.
        let (offset, half) = spin
            .windows
            .iter()
            .flat_map(|w| w.volumes.iter())
            .find(|v| v.autolink().is_none())
            .and_then(|v| match v.shape {
                ambition_entity_catalog::VolumeShape::Rect {
                    offset,
                    half_extents,
                } => Some((offset, half_extents)),
                _ => None,
            })
            .expect("the spin ends with a launching rect");

        for (pulse_offset, pulse_half, _) in rising_spin_pulses() {
            for side in [-1.0f32, 1.0] {
                // The anchor pulls victims in, so the finisher must cover the gathered
                // cloud, not the pulse's outer edge. Half the pulse reach is "held".
                let held = pulse_offset.0 + side * pulse_half.0 * 0.5;
                assert!(
                    (held - offset.0).abs() <= half.0,
                    "a victim gathered to x={held} is outside the launch box \
                     [{}, {}] — the spin catches it and then lets it go",
                    offset.0 - half.0,
                    offset.0 + half.0,
                );
            }
        }
    }

    #[test]
    fn the_reference_sword_fighter_answers_the_complete_typed_repertoire() {
        let moves = pointed_polygon_moveset();
        for id in [
            "polygon_jab",
            "pointed_polygon_dash_attack",
            "pointed_polygon_taunt",
            "polygon_tilt_forward",
            "polygon_tilt_up",
            "polygon_tilt_down",
            "polygon_smash_forward",
            "polygon_smash_up",
            "polygon_smash_down",
            "polygon_air_neutral",
            "polygon_air_forward",
            "polygon_air_back",
            "polygon_air_up",
            "polygon_air_down",
            "polygon_point",
            "polygon_vector_lunge",
            "polygon_rising_edge",
            "polygon_riposte",
            "polygon_falling_edge",
            "polygon_grab",
            "polygon_pummel",
            "polygon_throw_forward",
            "polygon_throw_back",
            "polygon_throw_up",
            "polygon_throw_down",
        ] {
            assert!(moves.moves.iter().any(|m| m.id == id), "missing {id}");
        }
    }
}
