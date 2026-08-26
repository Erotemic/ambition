//! Pointed Polygon — sword archetype repertoire.
//!
//! This is intentionally a FUNDAMENTALS table. The character exists partly as a
//! safe animation reference for future humanoids, so every common Smash verb has
//! a clear, conventional answer rather than a gimmick. The distinctive choice is
//! reach: the sword extends ordinary humanoid spacing without turning the fighter
//! into a heavyweight or projectile character.

use ambition_characters::moveset_authoring::Strike;
use ambition_characters::moveset_authoring::{committed_tail, impulse, multihit, strike, Pulse};
use ambition_characters::smash_capture::{
    author_pummel, author_standing_grab, author_throw, capture_beat, grab_shell,
    CaptureAttemptParams, CaptureCues, CapturePummelParams, CaptureThrowParams,
    SmashCaptureRepertoire,
};
use ambition_characters::smash_repertoire::{
    DownSpecial, NeutralSpecial, SmashRepertoire, UpSpecial,
};
use ambition_platformer2d::entity_catalog::AutolinkVolume;
use ambition_platformer2d::entity_catalog::{ImpulseMode, MovesetContract};

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
        knockback_growth: 2.95,
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

    // ⭐ THE RISING SPIN: swords out horizontally, four holding pulses, then one
    // launch.
    //
    // It used to be a single hit on the way up, which meant the move either
    // connected once and sent the victim away or missed entirely — the climb had
    // no reason to be long. Four autolink pulses make the rise itself the
    // mechanic: each one re-aims the victim at the spin's own centre, so it comes
    // UP with the move and the finisher has something to launch.
    //
    // ⛔ NOT a capture. Nothing is held: each pulse is an ordinary weak hit whose
    // reaction happens to aim inward, and the victim keeps every verb it has —
    // it can DI, it can tech the ending, and it falls out the moment the pulses
    // stop reaching it.
    //
    // ⭐⭐ THE SHAPE IS A DISK, and it was a COLUMN. Jon, W8 playtest: *"Pointed
    // extends her swords approximately horizontally. The attack volume should
    // form a broad disk / horizontal spinning envelope around her, rather than
    // reading like a narrow ordinary strike."* The pulse measured 52 wide by 60
    // tall and sat slightly in FRONT of her — taller than it was wide, and
    // one-sided, which is a rising poke rather than a spin. It is now wider than
    // it is tall and centred on the body, so a fighter on either side is inside
    // it: a spin has no front.
    let mut rising_edge = strike(Strike {
        id: "polygon_rising_edge",
        // ⭐ THE SIDE SWING, NOT THE OVERHEAD ONE — the fourth item on Jon's own
        // priority list for this move, *"rough sword-horizontal visual pose"*,
        // and it costs no art because the pose is already drawn. Her sheet's own
        // hitbox polys say which is which: `attack_side` spans x 53→100 at torso
        // height, `attack_up` spans x 76→196 reaching up and away. The disk this
        // move now IS wants the first one, and a spin drawn from an overhead
        // swing reads as a swing however fast it mirrors.
        clip: "attack_side",
        startup_s: 0.09,
        // The FINISHER, and it inherits the disk: a victim carried in on her BACK
        // side has to still be inside the box that launches, or the whole gather
        // ends by dropping half of what it caught.
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
            // The DISK: a little wider than the finisher, and level with the
            // torso rather than reaching above her head. Her forward smash
            // reaches x≈75 with one sword; two swords held out sideways reach
            // about that far each way, which is the read the shape has to carry.
            offset: (0.0, -12.0),
            half_extents: (48.0, 24.0),
            damage: 2,
            // Separated windows, because the runtime's re-hit rule refuses a
            // contiguous track — four touching windows would land once.
            active_s: 0.035,
            gap_s: 0.030,
            autolink: AutolinkVolume {
                // ⭐ THE SPIN'S OWN CENTRE, and the x is ZERO on purpose.
                // `autolink_anchor_world` mirrors the anchor with the attacker's
                // facing, so any non-zero x makes the gather point depend on
                // which way she happens to be looking — which is a statement
                // about a poke, not a spin. Zero is facing-invariant: whichever
                // side a victim comes in on, it is pulled toward HER.
                anchor: (0.0, -10.0),
                // The whole of the climb. The correction only closes a gap, and
                // this fighter is rising at 760 px/s — anything less and the
                // victim is left underneath its own move.
                carry: 1.0,
                pull: 22.0,
                max_speed: 900.0,
            },
        },
    );
    // ⭐ IT COSTS THE RECOVERY, and it no longer says so here: the up-B SLOT
    // says it for every fighter (`UpSpecial::Standard`). This was the only
    // moveset in the tree that had written the opt-in by hand, which is exactly
    // how a rule Jon asked to apply to the whole roster came to apply to one
    // character. The budget comes back when the body is re-seated: landing,
    // catching the ledge, being grabbed, a respawn.
    let mut up_special = rising_edge;
    // ⭐ THE CRUDE SPIN READ, and it is deliberately crude. Jon, W8 playtest:
    // *"it is acceptable to fake the spin by repeatedly flipping the sprite
    // horizontally if that gives the basic rotational read... Do not spend a lot
    // of time producing beautiful spin animation yet."* Twelve mirrors a second
    // over a ~0.5s move is about six flips — fast enough to read as turning,
    // slow enough to see which way she is pointing at any instant.
    up_special.sprite_spin_hz = Some(12.0);
    let up_special = impulse(up_special, 0.09, (0.0, -760.0), ImpulseMode::Set);

    let grounded_down_special = committed_tail(
        strike(Strike {
            id: "polygon_low_arc",
            clip: "attack_down",
            startup_s: 0.12,
            active_s: 0.09,
            recover_s: 0.25,
            offset: (18.0, 13.0),
            half_extents: (34.0, 11.0),
            damage: 8,
            knockback: 82.0,
            knockback_growth: 1.55,
            launch_dir: Some((0.8, -0.55)),
            on_hit: None,
        }),
        0.52,
        0.05,
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
        taunt: ambition_characters::moveset_authoring::taunt("pointed_polygon_taunt", 0.9),
        // the genre shape, deliberately: this character is the REFERENCE rig,
        // so its dash attack is the one a new humanoid should copy before it has
        // a reason to differ. A bespoke reach here would be a number nobody
        // chose being copied into every fighter that starts from these poses.
        dash_attack: ambition_characters::moveset_authoring::dash_attack(
            "pointed_polygon_dash_attack",
            ambition_characters::moveset_authoring::DashAttackShape::GENRE,
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
    use super::*;

    /// ⭐⭐ THE AUTHORED FIGHTER REACHES THE RECOVERY BUDGET — through the real
    /// moveset function, the real repertoire, and the real lowering.
    ///
    /// ⛔⛔ THIS IS THE TEST THAT WAS MISSING WHEN THE BUG SHIPPED. Every piece
    /// had a generic unit test and they were all green: `afford_recovery`
    /// refused a spent fighter, `start_move` spent the charge, `body_is_helpless`
    /// answered correctly. What nothing asked was whether authored content
    /// reaches those pieces — and it did not, twice for different reasons.
    /// `into_contract` deleted the field on the way past, and once that was
    /// repaired the rule was still opt-in with one fighter opted in.
    ///
    /// ⚠ so the claim is deliberately made against `pointed_polygon_moveset()`
    /// and not against a fixture: a fixture would have passed on both of the
    /// days this was broken in production.
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

    /// ⭐⭐ THE UP-B IS A DISK, NOT A POKE (D206).
    ///
    /// Jon, W8 playtest: *"Pointed extends her swords approximately
    /// horizontally. The attack volume should form a broad disk / horizontal
    /// spinning envelope around her, rather than reading like a narrow ordinary
    /// strike."*
    ///
    /// ⛔ IT MEASURED 52 WIDE BY 60 TALL and sat in front of her — taller than it
    /// was wide, and one-sided. That is a rising poke with a spin animation
    /// over it, which is exactly the read he was describing.
    ///
    /// ⭐ WIDER THAN TALL is the claim, not a pair of numbers: the shape has to
    /// say "swords held out sideways" rather than "a swing above the head", and
    /// an assertion on the literal extents would only restate the authoring.
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

    /// ⭐⭐ AND IT CATCHES BOTH SIDES, which is the point of the shape.
    ///
    /// Jon's own list: *"victim near Pointed's left side → multihit
    /// catches/carries; victim near Pointed's right side → multihit
    /// catches/carries; victim somewhat above/below center → broad disk still
    /// reads sensibly."*
    ///
    /// ⭐ THE FAR POINTS ARE ASSERTED TOO, and they are what keeps this from
    /// passing on a box of any size: a victim well past the swords is OUT. A
    /// test that only checked "somebody nearby is caught" would be green on a
    /// stage-wide hitbox.
    #[test]
    fn the_rising_spin_gathers_from_either_side_and_stops_somewhere() {
        // Beside her, at torso height: the two cases Jon named.
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
        // …and it ends. Roughly two body-widths out each way is not a spin any
        // more, it is a room.
        assert!(
            !caught_at((-120.0, -12.0)),
            "the spin reaches most of the stage to her left"
        );
        assert!(
            !caught_at((120.0, -12.0)),
            "the spin reaches most of the stage to her right"
        );
    }

    /// ⭐⭐ THE GATHER POINT DOES NOT DEPEND ON WHICH WAY SHE IS LOOKING.
    ///
    /// `autolink_anchor_world` mirrors an authored anchor with the attacker's
    /// facing — correct for a poke, wrong for a spin. The old anchor sat at
    /// x=+14, so a left-facing Pointed gathered her victims to her left and a
    /// right-facing one gathered them to her right: the same move, two different
    /// mechanics, decided by a fact the player was not thinking about.
    ///
    /// ⭐ ASKED THROUGH THE ENGINE'S OWN RESOLVER, not by reading the authored
    /// number: the mirroring is the resolver's rule, and a test that asserted
    /// `anchor.0 == 0.0` would pass just as happily if that rule changed.
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

    /// ⭐ AND THE FINISHER COVERS WHAT THE PULSES GATHERED.
    ///
    /// ⛔⛔ THIS IS THE ONE THE RESHAPE COULD SILENTLY BREAK. Widening the held
    /// pulses without widening the launch means a victim carried in on her back
    /// side rides the whole climb and then falls out unlaunched — the gather
    /// works and the move still does nothing, which is worse than the poke it
    /// replaced.
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
                // The anchor pulls victims IN, so what the finisher must cover is
                // the gathered cloud rather than the pulse's outer edge. Half of
                // the pulse's reach is the honest reading of "held".
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
            "polygon_low_arc",
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
