//! Pirate Admiral's authored Smash repertoire.
//!
//! The kit emphasizes cutlass reach and a summoned vehicle for a recovery: the
//! up-B calls a burning flying shark and rides it (D207). The pistol remains
//! an `ActionSet` capability, not a move-table entry, so ranged execution has
//! one authority.

use ambition_entity_catalog::authoring::Strike;
use ambition_entity_catalog::smash_capture::{
    author_pummel, author_standing_grab, author_throw, capture_beat, grab_shell,
    CaptureAttemptParams, CaptureCues, CapturePummelParams, CaptureThrowParams,
    SmashCaptureRepertoire,
};
use ambition_entity_catalog::smash_repertoire::{
    DownSpecial, NeutralSpecial, SmashRepertoire, UpSpecial,
};
use ambition_entity_catalog::{
    ImpulseMode, MoveEvent, MoveEventKind, MovesetContract,
};

use ambition_entity_catalog::authoring::{impulse, on_contact, sfx, strike, vfx, vfx_at};

/// The character the admiral's up-B summons to ride.
///
/// `npc_burning_flying_shark` is the mount half of the pirate sky-rider pair
/// from ADR 0020 (a `Mountable` of class `shark` with an authored saddle
/// offset), so he rides the same shark his raiders fly.
pub(crate) const SHARK_CHARACTER: &str = "npc_burning_flying_shark";

/// When in the up-B the shark arrives. Long enough to read as a summon and be
/// punishable; short enough to still be a recovery.
pub(crate) const SHARK_AT_S: f32 = 0.18;

/// When the move ends. The ride outlives it; the tail is only the animation
/// of having called the shark.
pub(crate) const SHARK_ENDS_S: f32 = 0.34;

/// How long the admiral may stay aboard. A first-pass design value (Jon);
/// expect it to be tuned down.
pub(crate) const SHARK_RIDE_SECONDS: f32 = 5.0;

/// The weapon the side-B draws. His own row, not the shared `gun_sword`
/// (an adventure pickup and a raider's sidearm), so the side-special can be
/// balanced separately.
pub(crate) const ADMIRAL_GUN_SWORD: &str = "admiral_gun_sword";

/// When the side-B fires. Long enough that the draw reads as a draw and the
/// move is punishable on reaction; short enough to still answer a press.
const GUNS_FIRE_AT_S: f32 = 0.20;

/// When the side-B ends, and with it the brandish. The tail is him putting
/// the gun-sword away, so the draw reads as temporary.
const GUNS_ENDS_S: f32 = 0.52;

/// See the module doc. Fifteen moves: the genre's standard verb map plus four
/// specials.
pub fn pirate_admiral_moveset() -> MovesetContract {
    // ── grounded ─────────────────────────────────────────────────────────────
    //
    // Even the jab is a blade: it starts slower than the goblin's whole punish
    // window and reaches half a body further.
    let jab = strike(Strike {
        id: "jab",
        clip: "jab",
        startup_s: 0.06,
        active_s: 0.07,
        recover_s: 0.16,
        offset: (30.0, 0.0),
        half_extents: (22.0, 14.0),
        damage: 4,
        knockback: 55.0,
        knockback_growth: 1.10,
        launch_dir: None,
        on_hit: None,
    });

    // A rising cutlass arc. Wide, because a sword's up-tilt covers the space in
    // front of the shoulder as well as above it.
    let up_tilt = strike(Strike {
        id: "tilt_up",
        clip: "attack_up",
        startup_s: 0.09,
        active_s: 0.08,
        recover_s: 0.19,
        offset: (14.0, -28.0),
        half_extents: (22.0, 24.0),
        damage: 6,
        knockback: 80.0,
        knockback_growth: 1.35,
        launch_dir: Some((0.25, -1.0)),
        on_hit: None,
    });

    // A low sweep along the deck. Long, shallow, and it sends them along the
    // ground rather than up — the setup, not the finish.
    let down_tilt = strike(Strike {
        id: "tilt_down",
        clip: "attack_down",
        startup_s: 0.08,
        active_s: 0.07,
        recover_s: 0.18,
        offset: (30.0, 14.0),
        half_extents: (24.0, 9.0),
        damage: 5,
        knockback: 60.0,
        knockback_growth: 1.20,
        launch_dir: Some((1.0, -0.18)),
        on_hit: None,
    });

    // ── smashes ──────────────────────────────────────────────────────────────
    //
    // The slowest and hardest kill move of the three tables: a full cutlass
    // swing costs 0.38s of standing still afterwards.
    let mut f_smash = strike(Strike {
        id: "smash_forward",
        clip: "smash_forward",
        startup_s: 0.34,
        active_s: 0.08,
        recover_s: 0.38,
        offset: (44.0, -4.0),
        half_extents: (30.0, 20.0),
        damage: 17,
        knockback: 160.0,
        knockback_growth: 4.09,
        launch_dir: Some((1.0, -0.42)),
        on_hit: None,
    });
    f_smash.smash_charge_mult = 1.7;

    let mut up_smash = strike(Strike {
        id: "smash_up",
        clip: "smash_up",
        startup_s: 0.30,
        active_s: 0.09,
        recover_s: 0.34,
        offset: (8.0, -34.0),
        half_extents: (24.0, 30.0),
        damage: 15,
        knockback: 155.0,
        knockback_growth: 6.40,
        launch_dir: Some((0.0, -1.0)),
        on_hit: None,
    });
    up_smash.smash_charge_mult = 1.7;

    // Both sides at deck height — the boarding-action answer to being flanked.
    let mut down_smash = strike(Strike {
        id: "smash_down",
        clip: "smash_down",
        startup_s: 0.28,
        active_s: 0.10,
        recover_s: 0.36,
        offset: (0.0, 16.0),
        half_extents: (40.0, 12.0),
        damage: 13,
        knockback: 140.0,
        knockback_growth: 3.55,
        launch_dir: Some((0.9, -0.50)),
        on_hit: None,
    });
    down_smash.smash_charge_mult = 1.7;

    // ── aerials ──────────────────────────────────────────────────────────────
    let n_air = strike(Strike {
        id: "air_neutral",
        clip: "air_neutral",
        startup_s: 0.07,
        active_s: 0.12,
        recover_s: 0.16,
        offset: (0.0, 0.0),
        half_extents: (26.0, 22.0),
        damage: 6,
        knockback: 70.0,
        knockback_growth: 1.30,
        launch_dir: None,
        on_hit: None,
    });

    let f_air = strike(Strike {
        id: "air_forward",
        clip: "air_forward",
        startup_s: 0.11,
        active_s: 0.08,
        recover_s: 0.20,
        offset: (32.0, -2.0),
        half_extents: (24.0, 18.0),
        damage: 9,
        knockback: 100.0,
        knockback_growth: 1.90,
        launch_dir: Some((1.0, -0.30)),
        on_hit: None,
    });

    let b_air = strike(Strike {
        id: "air_back",
        clip: "air_back",
        startup_s: 0.13,
        active_s: 0.07,
        recover_s: 0.24,
        offset: (-34.0, 0.0),
        half_extents: (24.0, 18.0),
        damage: 11,
        knockback: 135.0,
        knockback_growth: 2.55,
        launch_dir: Some((-1.0, -0.35)),
        on_hit: None,
    });

    let u_air = strike(Strike {
        id: "air_up",
        clip: "air_up",
        startup_s: 0.08,
        active_s: 0.09,
        recover_s: 0.17,
        offset: (2.0, -30.0),
        half_extents: (20.0, 24.0),
        damage: 7,
        knockback: 90.0,
        knockback_growth: 1.85,
        launch_dir: Some((0.0, -1.0)),
        on_hit: None,
    });
    // The stall: a rising overhead that carries the admiral up, so he can chain
    // one hit into the next without falling out of his own combo.
    let u_air = impulse(u_air, 0.08, (0.0, -360.0), ImpulseMode::Set);
    let u_air = sfx(u_air, 0.0, "player.robot.slash.air");
    let u_air = on_contact(u_air, "player.robot.slash.impact.flesh.light");
    let u_air = vfx(u_air, 0.08, "burst_round");

    // A real spike: point-down cutlass into the blast zone. No `on_hit`
    // rebound; only the robot can bounce off what it hits.
    let d_air = strike(Strike {
        id: "air_down",
        clip: "air_down",
        startup_s: 0.14,
        active_s: 0.08,
        recover_s: 0.26,
        offset: (6.0, 28.0),
        half_extents: (20.0, 20.0),
        damage: 11,
        knockback: 130.0,
        knockback_growth: 2.30,
        launch_dir: Some((0.0, 1.0)),
        on_hit: None,
    });

    // ── the four specials ────────────────────────────────────────────────────
    //
    // Four mechanisms: one fires forward and shoves him backward; one adds to
    // his current motion; one draws and fires a weapon; one commands a full
    // stop. The shared `strike` shape is the same, so they differ in what they
    // do to the admiral.

    // Neutral: `grapeshot`. A pistol at the hip, and its recoil. The volume is
    // short and wide; the admiral is thrown backward, a spacing tool and a way
    // to leave the stage.
    let neutral_b = strike(Strike {
        id: "grapeshot",
        clip: "special",
        startup_s: 0.14,
        active_s: 0.06,
        recover_s: 0.24,
        offset: (38.0, -4.0),
        half_extents: (30.0, 16.0),
        damage: 9,
        knockback: 120.0,
        knockback_growth: 1.90,
        launch_dir: Some((0.85, -0.55)),
        on_hit: None,
    });
    // The negative side is the move. The catalog reads it as a route with
    // `lift_side < 0`, which carries its owner away from its facing. A recovery
    // search may propose it and the kernel declines it when thrown toward the
    // stage. No rule is needed.
    let neutral_b = impulse(neutral_b, 0.14, (-560.0, -120.0), ImpulseMode::Set);
    let neutral_b = sfx(neutral_b, 0.0, "player.attack.charge");
    let neutral_b = sfx(neutral_b, 0.14, "player.slash");
    let neutral_b = vfx(neutral_b, 0.14, "smoke_burst");
    let neutral_b = on_contact(neutral_b, "world.rock.hit");

    // Side: `run_out_the_guns`. He draws the gun-sword and fires it.
    //
    // Jon: *"The pirate side b should briefly equip the lasergun sword and fire
    // a lasersword projectile"* left or right as directed, aimed at the nearest
    // opponent if they are in that half plane.
    //
    // The draw is `MoveSpec::equips`. The move's clock is the timer: an
    // interrupted move puts the sword away at once, and `MoveBrandishedItem`
    // restores what it displaced.
    //
    // The shot is `MoveEventKind::Ranged`, which fires the weapon in hand (the
    // drawn `admiral_gun_sword`, not his pistol). Its look and sound come from
    // that weapon's authored `Discharge`: the spinning `lasersword` projectile,
    // the muzzle at his hand, `weapon.lasersword.fire`, the heavier recoil.
    // Checked by `the_admirals_side_b_fires_the_gun_swords_discharge`.
    //
    // The angle is the weapon's `AimAssist::half_plane`. The player picks the
    // side; the weapon picks the angle.
    //
    // No melee volume: the projectile is the damage.
    //
    // It keeps a forward step: an `Add` impulse, so the shot goes furthest out of
    // a run. It advertises no route (an additive impulse cannot be read
    // statically), so recovery search never proposes it.
    let side_b = ambition_entity_catalog::authoring::hitless_special(
        "run_out_the_guns",
        "special",
        GUNS_FIRE_AT_S,
        GUNS_ENDS_S,
    );
    let mut side_b = side_b;
    side_b.display_name = Some("Run Out the Guns".to_string());
    side_b.equips = Some(ADMIRAL_GUN_SWORD.to_string());
    side_b.events.push(MoveEvent {
        at_s: GUNS_FIRE_AT_S,
        kind: MoveEventKind::Ranged,
    });
    let side_b = impulse(side_b, 0.0, (210.0, 0.0), ImpulseMode::Add);
    let side_b = sfx(side_b, 0.0, "player.attack.charge");
    let side_b = vfx(side_b, GUNS_FIRE_AT_S, "muzzle_flash");

    // Up: `call_the_shark`. The recovery is a vehicle. Jon: *"their up-b should
    // summon a burning flying shark that they can mount and ride"*, flown with
    // the stick for a limited time.
    //
    // No attack hitbox. Jon: *"it's purely a mobility special"*, so it is a
    // `hitless_special`.
    //
    // The shark still has a hurtbox: it dies to a damage threshold, so an
    // opponent can gimp the recovery by killing it. It is `Neutral` and deals no
    // contact damage.
    //
    // The cost is the budget, not freefall. `author_summon_ride` sets one shark
    // per airtime and no helpless state (a rider must be able to act). It is a
    // strong recovery on purpose; balancing comes later.
    //
    // No impulse: the shark appears where he is and the player steers the climb.
    // Adding a rise is the first knob if the ride does not save him.
    let up_b = ambition_entity_catalog::authoring::hitless_special(
        "call_the_shark",
        "special_up",
        SHARK_AT_S,
        SHARK_ENDS_S,
    );
    let up_b = ambition_entity_catalog::smash_ride::author_summon_ride(
        up_b,
        SHARK_AT_S,
        ambition_entity_catalog::smash_ride::SummonRideParams {
            character_id: SHARK_CHARACTER.to_string(),
            // The authored shark body, matching its `Mountable` saddle offset.
            half_extents: (48.0, 22.0),
            seconds: SHARK_RIDE_SECONDS,
            // Half the ride's straight-line distance (the shark's `run_speed` is
            // 260px/s for five seconds). Under the arithmetic on purpose: a recovery
            // turns back toward the stage, and an over-claiming search kills the
            // fighter it tries to save.
            reach: SHARK_RIDE_SECONDS * 260.0 * 0.5,
        },
    );
    let up_b = sfx(up_b, 0.0, "player.attack.charge");
    let up_b = sfx(up_b, SHARK_AT_S, "player.robot.slash.air");
    // A recovery gets its own burst, so the other player sees it.
    let up_b = vfx(up_b, SHARK_AT_S, "classic_burst");

    // Down: `heave_to`. The anchor: a `Set` of `(0, 0)`, a full stop.
    //
    // It cancels the drift from a launch. It advertises no route (`local.1` is
    // not negative, so the lift derivation skips it), which is correct.
    let down_b = strike(Strike {
        id: "heave_to",
        clip: "special_down",
        startup_s: 0.12,
        active_s: 0.10,
        recover_s: 0.30,
        offset: (0.0, 22.0),
        half_extents: (30.0, 20.0),
        damage: 10,
        knockback: 105.0,
        knockback_growth: 1.75,
        launch_dir: Some((0.0, 1.0)),
        on_hit: None,
    });
    let down_b = impulse(down_b, 0.12, (0.0, 0.0), ImpulseMode::Set);
    let down_b = sfx(down_b, 0.12, "player.slash");
    let down_b = vfx(down_b, 0.12, "starburst");
    let down_b = on_contact(down_b, "player.robot.slash.impact.metal.chink");

    // The forward tilt, so "forward" is not answered by a jab. A level cut at
    // chest height: the longest tilt on the grid, and slower than the goblin's
    // jab.
    let f_tilt = strike(Strike {
        id: "tilt_forward",
        clip: "attack_side",
        startup_s: 0.10,
        active_s: 0.08,
        recover_s: 0.20,
        offset: (38.0, -4.0),
        half_extents: (26.0, 14.0),
        damage: 7,
        knockback: 78.0,
        knockback_growth: 1.30,
        launch_dir: Some((1.0, -0.30)),
        on_hit: None,
    });
    let f_tilt = vfx_at(f_tilt, 0.10, "air_slice", (38.0, -4.0), 1.0);
    let f_tilt = sfx(f_tilt, 0.10, "enemy.pirate.cutlass_swing");
    let f_tilt = on_contact(f_tilt, "player.hit");

    // ── The boarding grapple ────────────────────────────────────────────────
    //
    // The opposite of George's grab, through a different provider, sharing no
    // numbers.
    //
    // Fast (`0.07` startup vs George's `0.14`) and short (`19` reach vs `26`):
    // he must be close to board. Recovery `0.20` vs `0.30`: his grab is a
    // scramble tool, George's a commitment.
    //
    // The hold sits close and low (`13` forward, `+3` down): against the chest.
    let grab = author_standing_grab(
        grab_shell("pirate_grab", "grab", 0.07, 0.05, 0.20),
        CaptureAttemptParams {
            offset: (12.0, 1.0),
            half_extents: (19.0, 16.0),
            hold_offset: (13.0, 3.0),
        },
    );
    // A fast, light pummel: `0.13` and `2` vs George's `0.24` and `4`. About the
    // same damage per second, but each beat is a chance for the hold to break.
    let pummel = author_pummel(
        capture_beat("pirate_pummel", "attack", 0.13),
        0.06,
        CapturePummelParams { damage: 2 },
    );
    // An upward throw in the forward slot, on purpose. He heaves a body up and
    // slightly forward (`0.55` lateral, `-1.0` vertical) so it lands in front of
    // him. Less knockback than George's (`104` vs `138`) and more growth (`2.4`
    // vs `1.9`): weak early, strong late.
    let forward_throw = author_throw(
        capture_beat("pirate_fthrow", "attack", 0.26),
        0.14,
        CaptureThrowParams {
            damage: 8,
            knockback: 104.0,
            knockback_growth: 2.4,
            launch_dir: (0.55, -1.0),
        },
    );

    let back_throw = author_throw(
        capture_beat("pirate_bthrow", "attack", 0.28),
        0.15,
        CaptureThrowParams {
            damage: 9,
            knockback: 112.32,
            knockback_growth: 2.52,
            launch_dir: (-1.0, -0.62),
        },
    );

    let up_throw = author_throw(
        capture_beat("pirate_uthrow", "attack", 0.27),
        0.14,
        CaptureThrowParams {
            damage: 8,
            knockback: 108.16,
            knockback_growth: 2.45,
            launch_dir: (0.0, -1.0),
        },
    );

    let down_throw = author_throw(
        capture_beat("pirate_dthrow", "attack", 0.29),
        0.15,
        CaptureThrowParams {
            damage: 6,
            knockback: 76.96,
            knockback_growth: 1.92,
            launch_dir: (0.22, -0.92),
        },
    );

    SmashRepertoire {
        taunt: ambition_entity_catalog::authoring::taunt("pirate_admiral_taunt", 0.9),
        dash_attack: ambition_entity_catalog::authoring::dash_attack(
            "pirate_admiral_dash_attack",
            ambition_entity_catalog::authoring::DashAttackShape::GENRE,
            9,
            97.5,
        ),
        jab,
        forward_tilt: f_tilt,
        up_tilt,
        down_tilt,
        forward_smash: f_smash,
        up_smash,
        down_smash,
        neutral_air: n_air,
        forward_air: f_air,
        back_air: b_air,
        up_air: u_air,
        down_air: d_air,
        neutral_special: NeutralSpecial::Authored(neutral_b),
        side_special: side_b,
        up_special: UpSpecial::NoFreefall(up_b),
        // Authored through this crate, not the smash demo, which shows the capture
        // vocabulary is not tied to one game-owned file.
        capture: SmashCaptureRepertoire {
            cues: CaptureCues::GENERIC,
            grab,
            pummel,
            forward_throw,
            back_throw: Some(back_throw),
            up_throw: Some(up_throw),
            down_throw: Some(down_throw),
        },
        down_special: DownSpecial::OneForm(down_b),
    }
    .into_contract()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_entity_catalog::{MoveSpec, VolumeShape, WindowTag};

    fn find(set: &MovesetContract, id: &str) -> MoveSpec {
        set.moves
            .iter()
            .find(|m| m.id == id)
            .unwrap_or_else(|| panic!("{id} exists"))
            .clone()
    }

    fn startup(m: &MoveSpec) -> f32 {
        m.windows
            .iter()
            .find(|w| matches!(w.tag, WindowTag::Active))
            .expect("a strike has an active window")
            .start_s
    }

    fn reach(m: &MoveSpec) -> f32 {
        m.windows
            .iter()
            .flat_map(|w| w.volumes.iter())
            .map(|v| match v.shape {
                VolumeShape::Rect {
                    offset,
                    half_extents,
                } => offset.0.abs() + half_extents.0,
                _ => 0.0,
            })
            .fold(0.0f32, f32::max)
    }

    fn damage(m: &MoveSpec) -> i32 {
        m.windows
            .iter()
            .flat_map(|w| w.volumes.iter())
            .map(|v| v.damage)
            .max()
            .unwrap_or(0)
    }

    // Verb binding is checked by construction: `SmashRepertoire` owns the verb
    // strings and is a struct with no `Default`, so a missing slot is a compile
    // error. Coverage in every posture is checked by
    // `ambition_entity_catalog::smash_repertoire` and by
    // `smash_roster_movesets::report_the_smash_kit_every_selectable_fighter_has`.

    /// The commanded (`Set`) velocity a move states, if it states one.
    fn commanded(set: &MovesetContract, id: &str) -> Option<(f32, f32)> {
        use ambition_entity_catalog::MoveEventKind;
        find(set, id).events.iter().find_map(|e| match &e.kind {
            MoveEventKind::Impulse {
                local,
                mode: ImpulseMode::Set,
            } => Some(*local),
            _ => None,
        })
    }

    // -----------------------------------------------------------------------
    // No `RecoveryLens` fixture for this fighter: the shark up-B commands no
    // velocity, so the lens's commanded-velocity routes cannot see it (D207).
    // The planner reads it through its carry instead (D250, tested below). The
    // engine-level guard stays in `RecoveryLens`
    // (`a_tiny_lifting_move_does_not_suppress_a_viable_recovery`).

    /// Four specials, four mechanisms: a commanded retreat, a drawn weapon with
    /// an additive step, a summoned ride, a full stop. No two share a mechanism.
    ///
    /// They also exercise different corners of the catalog's route derivation (a
    /// negative side, an `Add` that states nothing, a `Set` of zero).
    #[test]
    fn the_four_specials_are_four_different_mechanisms() {
        use ambition_entity_catalog::MoveEventKind;
        let set = crate::authored_movesets::shipped("npc_pirate_admiral");

        // Neutral: a recoil. Commanded, and it points backward.
        let shot = commanded(&set, "grapeshot").expect("the pistol shoves its owner");
        assert!(shot.0 < 0.0 && shot.1 < 0.0);

        // Side: he draws a weapon and fires it. The step is additive, so it
        // commands nothing and advertises no route.
        assert!(
            commanded(&set, "run_out_the_guns").is_none(),
            "the step must ADD to the admiral's momentum, not replace it"
        );
        assert!(
            find(&set, "run_out_the_guns")
                .events
                .iter()
                .any(|e| matches!(
                    &e.kind,
                    MoveEventKind::Impulse {
                        mode: ImpulseMode::Add,
                        ..
                    }
                )),
            "…and it must still displace him"
        );
        assert_eq!(find(&set, "run_out_the_guns").frame_data().lift_speed, 0.0);
        // Both halves together. A draw with no shot is a taunt; a shot with no draw
        // fires his pistol.
        assert_eq!(
            find(&set, "run_out_the_guns").equips.as_deref(),
            Some(ADMIRAL_GUN_SWORD),
            "the side-B must draw the gun-sword; without it the shot is the pistol's"
        );
        assert!(
            find(&set, "run_out_the_guns")
                .events
                .iter()
                .any(|e| matches!(&e.kind, MoveEventKind::Ranged)),
            "the side-B must fire; a draw with no shot is a taunt"
        );
        // The draw outlives the shot. The brandish ends with the move, so a fire
        // event at or after the end would leave from a bare hand.
        let guns = find(&set, "run_out_the_guns");
        let fires_at = guns
            .events
            .iter()
            .find(|e| matches!(&e.kind, MoveEventKind::Ranged))
            .map(|e| e.at_s)
            .expect("the side-B fires");
        assert!(
            fires_at < guns.duration_s,
            "the shot fires at {fires_at}s of a {}s move, so the gun-sword is \
             already back in its sheath when the trigger is pulled",
            guns.duration_s
        );

        // Up: a vehicle. It displaces nobody; the technique on its timeline makes
        // it a recovery. The CPU recovery search reads commanded velocity, so it
        // cannot see this move (D207). Asserted so that adding an impulse later is
        // noticed.
        assert!(
            commanded(&set, "call_the_shark").is_none(),
            "the shark up-B commands a velocity, which means it is no longer the \
             vehicle recovery this fighter is built around"
        );
        assert!(
            find(&set, "call_the_shark")
                .events
                .iter()
                .any(|e| matches!(&e.kind, MoveEventKind::Effect(effect)
                    if effect.key == ambition_entity_catalog::smash_ride::SUMMON_RIDE)),
            "the up-B summons nothing, so the admiral has no recovery at all"
        );
        assert_eq!(
            find(&set, "call_the_shark").gates.recovery,
            ambition_entity_catalog::RecoveryUse::SpendWithoutFreefall,
            "the whole price is one value: one use per airtime, and no freefall. \
             It was two booleans that had to agree, and a move stating one \
             without the other was a different mechanic wearing this one's name"
        );

        // Down: a full stop, which is a commanded velocity of nothing.
        assert_eq!(commanded(&set, "heave_to"), Some((0.0, 0.0)));
        assert_eq!(
            find(&set, "heave_to").frame_data().lift_speed,
            0.0,
            "stopping dead in mid-air is not a way home from anywhere"
        );
    }

    /// Every important move is heard and seen. VFX ids are checked against the
    /// shipped FX spritesheet rows, which the renderer resolves against.
    #[test]
    fn the_specials_and_the_juggle_carry_their_own_feedback() {
        use ambition_entity_catalog::MoveEventKind;
        let set = crate::authored_movesets::shipped("npc_pirate_admiral");
        for id in [
            "grapeshot",
            "run_out_the_guns",
            "call_the_shark",
            "heave_to",
            "air_up",
        ] {
            let m = find(&set, id);
            assert!(
                m.events
                    .iter()
                    .any(|e| matches!(&e.kind, MoveEventKind::Sfx { .. })),
                "`{id}` makes no sound"
            );
            assert!(
                m.events.iter().any(|e| match &e.kind {
                    MoveEventKind::Vfx { effect, .. } => {
                        assert!(
                            ambition_platformer2d::sprite_sheet::fx::is_authored_effect(effect),
                            "`{id}` names vfx `{effect}`, which the engine's \
                             vocabulary does not contain — this is a refused load"
                        );
                        true
                    }
                    _ => false,
                }),
                "`{id}` shows nothing"
            );
            // A move with no volumes cannot land. `call_the_shark` has no hitbox by
            // design, so it needs no contact cue.
            let lands = m.windows.iter().any(|w| !w.volumes.is_empty());
            assert!(
                !lands
                    || m.windows
                        .iter()
                        .flat_map(|w| w.volumes.iter())
                        .any(|v| v.hit_sfx.is_some()),
                "`{id}` lands silently"
            );
        }

        // Control: ordinary swings are not dressed up.
        let jab = find(&set, "jab");
        assert!(
            !jab.events
                .iter()
                .any(|e| matches!(&e.kind, MoveEventKind::Vfx { .. })),
            "a jab that bursts makes the specials look like nothing"
        );
    }

    /// Three tables, three fighters, one ORDERING — and it is checked against
    /// the other two rather than against literals.
    ///
    /// The module docs make comparative claims (shorter, slower, harder), so this
    /// test compares the three tables, not literals. Retuning any of them must
    /// keep the ordering or say why.
    #[test]
    fn the_admiral_is_longer_slower_and_heavier_than_the_other_two() {
        let admiral = crate::authored_movesets::shipped("npc_pirate_admiral");
        let goblin = crate::authored_movesets::shipped("goblin");
        let robot = crate::player_robot_moveset::player_robot_moveset();

        let jabs = |set: &MovesetContract| {
            let jab = find(set, "jab");
            (reach(&jab), startup(&jab))
        };
        let (a_reach, a_startup) = jabs(&admiral);
        let (r_reach, r_startup) = jabs(&robot);
        let (g_reach, g_startup) = jabs(&goblin);

        assert!(
            a_reach > r_reach && r_reach > g_reach,
            "reach orders admiral > robot > goblin (got {a_reach}, {r_reach}, {g_reach})"
        );
        assert!(
            a_startup > r_startup && r_startup > g_startup,
            "and startup orders the same way — the longer blade is the slower one \
             (got {a_startup}, {r_startup}, {g_startup})"
        );

        let smash = |set: &MovesetContract| damage(&find(set, "smash_forward"));
        assert!(
            smash(&admiral) > smash(&robot) && smash(&robot) > smash(&goblin),
            "and the kill move pays for the commitment: {} > {} > {}",
            smash(&admiral),
            smash(&robot),
            smash(&goblin)
        );
    }

    /// The up-B is a way home the planner can see (D250).
    ///
    /// `call_the_shark` commands no impulse, so `lift_speed` stays `0.0`. Do not
    /// add a fake lift: the search would then certify a rise that does not
    /// exist.
    #[test]
    fn the_sharks_summon_advertises_seconds_of_authority_and_no_lift() {
        use ambition_entity_catalog::RecoveryRoute;
        let set = crate::authored_movesets::shipped("npc_pirate_admiral");
        let frames = find(&set, "call_the_shark").frame_data();
        assert_eq!(
            frames.lift_speed, 0.0,
            "the summon must still command no rise; a fabricated one would have \
             the recovery search certify height the move never throws"
        );
        let RecoveryRoute::SustainedAuthority { seconds, reach } = frames.recovery_route else {
            panic!(
                "the summon offers {:?}, so a recovery planner reads it as no way \
                 home — which is exactly D250",
                frames.recovery_route
            );
        };
        assert_eq!(seconds, SHARK_RIDE_SECONDS, "the ride's own length");
        assert!(
            reach > 0.0 && reach < SHARK_RIDE_SECONDS * 260.0,
            "the claimed reach is {reach}, which is either nothing or the whole \
             straight-line ride — a recovery turns back toward the stage, and a \
             search that over-claims kills the fighter it meant to save"
        );

        // The 650px is travel, not threat. The move has no hitbox, the shark is
        // `Neutral` with no contact damage, and `reach` is half the ride's distance:
        // where he can go. So `frame_data` must not fold it into hazard. The brain
        // reads it on the motion road (`brain::fighter::options::travel_of` →
        // `RecoveryRoute::carry`); the attack menu gets nothing.
        assert_eq!(
            frames.hazard, None,
            "the mobility special is advertised as {:?} of OFFENSIVE reach, so \
             the CPU admiral will summon a shark at an opponent it cannot \
             touch and stand in the resulting move while they walk up",
            frames.hazard
        );
        assert_eq!(
            frames.threat_live_at_s, None,
            "a move that threatens nobody named a time at which it does"
        );
        assert!(
            frames.coverage.is_none() && frames.push_coverage.is_none(),
            "the up-b grew a volume: coverage={:?} push={:?}",
            frames.coverage,
            frames.push_coverage
        );
        // The carry is still readable, so it is not off every menu.
        assert_eq!(
            frames.recovery_route.carry(),
            reach,
            "the travel a motion planner reads is not the ride's own reach"
        );
    }
}
