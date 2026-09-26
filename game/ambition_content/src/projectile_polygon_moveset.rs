//! Projectile Polygon — ranged beast-biped fundamentals repertoire.
//!
//! A complete ranged-fundamentals table for the non-humanoid member of the
//! polygon reference trio. Neutral special is a real projectile release from a
//! head-mounted cannon; the rest of the kit stays readable so the move library
//! remains useful as a bestial pose reference rather than a one-off gimmick.

use ambition_entity_catalog::authoring::Strike;
use ambition_entity_catalog::authoring::{impulse, strike};
use ambition_entity_catalog::smash_capture::{
    author_pummel, author_standing_grab, author_throw, capture_beat, grab_shell,
    CaptureAttemptParams, CaptureCues, CapturePummelParams, CaptureThrowParams,
    SmashCaptureRepertoire,
};
use ambition_entity_catalog::smash_repertoire::{
    DownSpecial, NeutralSpecial, SmashRepertoire, UpSpecial,
};
use ambition_entity_catalog::{
    ChargeGesture, ClipBinding, ImpulseMode, MoveEvent, MoveEventKind, MoveSpec, MoveWindow,
    MovesetContract, SmashChargeSpec, WindowTag,
};

/// How long the fill row runs, in seconds: 14 frames at 62ms, from
/// `polygon_charge_shot`'s own authoring.
///
/// The VFX row is one long climb, not a loop, so a player can read "nearly
/// there" from the ring count. That works only if the fill ends exactly when
/// the charge does.
const CHARGE_FILL_S: f32 = 14.0 * 0.062;

/// Where in the windup he latches. Early, so the charge reads as "the shot
/// started and stopped", as the smash charge pose does.
const CHARGE_HOLD_AT_S: f32 = 0.10;

/// Where the charge is drawn, body-local (`+x` toward facing, `+y` gravity-down).
///
/// This is the spawn point, not the cannon art. See [`charge_shot`]: the
/// effect must match where the shot leaves, or the ball jumps on release.
const MUZZLE: (f32, f32) = (0.0, -8.0);

/// When the shot leaves, measured from the move's start. Everything before it is
/// windup that plays out on release.
const CHARGE_FIRE_AT_S: f32 = 0.26;

/// How far her line reaches, in world px.
///
/// One number for two moves: her grab is a tether
/// (`offset.0 + half_extents.0` = 86 + 64) and her up-B throws the same line
/// at a ledge. `the_tether_reaches_as_far_as_her_grab` keeps them equal,
/// because the grab is authored as a box, not a distance.
const THE_TETHERS_REACH: f32 = 150.0;

/// The held-item id her side-B takes hold of. See `polygon_ponytail` in the
/// held-item registry: while this move plays she wields her tail, and its
/// ranged verb is the throw.
const PONYTAIL: &str = "polygon_ponytail";

/// When the tail leaves her hand. Late enough to read as a wind-up and be
/// punishable on reaction.
const PONYTAIL_THROWN_AT_S: f32 = 0.16;

/// When the move ends, and with it the grip. Shorter than the tail's round
/// trip on purpose: she can act while it is still out.
const PONYTAIL_ENDS_S: f32 = 0.40;

/// The held-item id her down-B lays. It must be a registered held item, or
/// nobody can pick the bomb up.
const BOMB_ITEM: &str = "polygon_bomb";
const MINE_ITEM: &str = "polygon_mine";

/// When the bomb reaches the floor.
const BOMB_LAID_AT_S: f32 = 0.18;

/// When the move ends. She is committed for a beat afterwards, which is the
/// cost of putting a live thing on the stage.
const BOMB_ENDS_S: f32 = 0.46;

/// The charge ball: the genre's held neutral-B, and this fighter's identity.
///
/// Hold Special and the timeline freezes at [`CHARGE_HOLD_AT_S`] while the
/// ball builds at the muzzle. Release (or reach the maximum) and the rest of
/// the windup plays into the shot, scaled by hold time: see
/// `crate::authored::projectile_polygon`'s `charged_cannon`.
///
/// It stores (see `stores` below): a full charge waits instead of firing, and
/// an interrupted one is banked for the next press (Samus/Mewtwo parity).
///
/// No `smash_charge_mult`: that scales a melee volume this move does not
/// have. The explicit `smash_charge` is what makes it hold.
///
/// The shot leaves the cannon through `Muzzle::Offset { x: 0.22, y: -0.34 }`
/// on her ranged action in `authored/projectile_polygon.rs`.
///
/// The fill VFX fires once, at the latch. A move event is a point in time,
/// so nothing re-fires it if the hold outlasts the row; the row is drawn to
/// the same length as the hold. A longer hold would need a sustained
/// presentation channel, which does not exist yet.
fn charge_shot() -> MoveSpec {
    MoveSpec {
        id: "polygon_projectile_charge_shot".to_string(),
        display_name: Some("Charge Shot".to_string()),
        clip: ClipBinding {
            clip: "shoot".to_string(),
            fallbacks: vec!["attack_side".to_string(), "idle".to_string()],
        },
        duration_s: 0.58,
        windows: vec![
            MoveWindow {
                start_s: 0.0,
                end_s: CHARGE_FIRE_AT_S,
                tag: WindowTag::Startup,
                volumes: vec![],
                sustain_effect: None,
                motion_scale: 1.0,
            },
            // No Active volume: the projectile is the damage. The recovery is the
            // settle he owes for committing.
            MoveWindow {
                start_s: CHARGE_FIRE_AT_S,
                end_s: 0.58,
                tag: WindowTag::Recovery,
                volumes: vec![],
                sustain_effect: None,
                motion_scale: 1.0,
            },
        ],
        events: vec![
            // The intake first, so the player sees the button took before the ball
            // appears.
            MoveEvent {
                at_s: 0.01,
                kind: MoveEventKind::Vfx {
                    effect: "charge_intake".to_string(),
                    at: MUZZLE,
                    scale: 1.0,
                    sfx: None,
                },
            },
            MoveEvent {
                at_s: CHARGE_HOLD_AT_S * 0.5,
                kind: MoveEventKind::Vfx {
                    effect: "charge_build".to_string(),
                    at: MUZZLE,
                    scale: 1.0,
                    sfx: None,
                },
            },
            MoveEvent {
                at_s: CHARGE_FIRE_AT_S,
                kind: MoveEventKind::Vfx {
                    effect: "charge_release".to_string(),
                    at: MUZZLE,
                    scale: 1.0,
                    sfx: None,
                },
            },
            MoveEvent {
                at_s: CHARGE_FIRE_AT_S,
                kind: MoveEventKind::Ranged,
            },
        ],
        gates: Default::default(),
        start_impulse: None,
        smash_charge_mult: 1.0,
        smash_charge: Some(SmashChargeSpec {
            hold_at_s: CHARGE_HOLD_AT_S,
            max_hold_s: CHARGE_FILL_S,
            // It stores (Samus/Mewtwo parity). The `RangedCharge` ladder and the
            // sheet's five tiers give different sizes; storing lets a player reach a
            // full ball without standing still through the whole fill. At maximum the
            // shot waits, and getting hit banks it.
            stores: true,
            roots: true,
            sustain: ambition_entity_catalog::ChargeSustain::WhileHeld,
        }),
        charge_gesture: ChargeGesture::Special,
        repeat: None,
        landing_lag_s: None,
        autocancel_after_s: None,
        sprite_spin_hz: None,
        equips: None,
        flow: None,
    }
}

pub fn projectile_polygon_moveset() -> MovesetContract {
    let jab = strike(Strike {
        id: "polygon_projectile_jab",
        clip: "jab",
        startup_s: 0.04,
        active_s: 0.05,
        recover_s: 0.10,
        offset: (22.0, -2.0),
        half_extents: (18.0, 15.0),
        damage: 4,
        knockback: 52.0,
        knockback_growth: 1.10,
        launch_dir: Some((1.0, -0.18)),
        on_hit: None,
    });
    let forward_tilt = strike(Strike {
        id: "polygon_projectile_tilt_forward",
        clip: "attack_side",
        startup_s: 0.07,
        active_s: 0.07,
        recover_s: 0.15,
        offset: (27.0, -1.0),
        half_extents: (21.0, 16.0),
        damage: 7,
        knockback: 82.0,
        knockback_growth: 1.52,
        launch_dir: Some((1.0, -0.27)),
        on_hit: None,
    });
    let up_tilt = strike(Strike {
        id: "polygon_projectile_tilt_up",
        clip: "attack_up",
        startup_s: 0.07,
        active_s: 0.07,
        recover_s: 0.15,
        offset: (8.0, -25.0),
        half_extents: (20.0, 24.0),
        damage: 6,
        knockback: 86.0,
        knockback_growth: 1.58,
        launch_dir: Some((0.10, -1.0)),
        on_hit: None,
    });
    let down_tilt = strike(Strike {
        id: "polygon_projectile_tilt_down",
        clip: "attack_down",
        startup_s: 0.06,
        active_s: 0.06,
        recover_s: 0.14,
        offset: (24.0, 12.0),
        half_extents: (22.0, 11.0),
        damage: 5,
        knockback: 68.0,
        knockback_growth: 1.31,
        launch_dir: Some((1.0, -0.16)),
        on_hit: None,
    });

    let mut forward_smash = strike(Strike {
        id: "polygon_projectile_smash_forward",
        clip: "smash_forward",
        startup_s: 0.22,
        active_s: 0.08,
        recover_s: 0.29,
        offset: (30.0, -3.0),
        half_extents: (24.0, 20.0),
        damage: 16,
        knockback: 162.0,
        knockback_growth: 3.25,
        launch_dir: Some((1.0, -0.31)),
        on_hit: None,
    });
    forward_smash.smash_charge_mult = 1.75;
    let mut up_smash = strike(Strike {
        id: "polygon_projectile_smash_up",
        clip: "smash_up",
        startup_s: 0.20,
        active_s: 0.08,
        recover_s: 0.28,
        offset: (5.0, -29.0),
        half_extents: (23.0, 29.0),
        damage: 15,
        knockback: 158.0,
        knockback_growth: 5.83,
        launch_dir: Some((0.0, -1.0)),
        on_hit: None,
    });
    up_smash.smash_charge_mult = 1.75;
    let mut down_smash = strike(Strike {
        id: "polygon_projectile_smash_down",
        clip: "smash_down",
        startup_s: 0.19,
        active_s: 0.09,
        recover_s: 0.29,
        offset: (0.0, 13.0),
        half_extents: (34.0, 13.0),
        damage: 13,
        knockback: 142.0,
        knockback_growth: 2.82,
        launch_dir: Some((0.80, -0.60)),
        on_hit: None,
    });
    down_smash.smash_charge_mult = 1.75;
    // The remote mine is added to the down smash; the swing is unchanged.
    //
    // So the smash stays a smash. A hitless mine-placer would make
    // `smash_charge_mult` meaningless and cost her a charged down smash. She
    // sweeps low and leaves something behind.
    //
    // Pressing again detonates the mine and still swings: the hitbox is not
    // conditional. Detonating from far away costs a swing at air; detonating
    // on top of it is a two-hit option that can hurt her too (the blast is
    // neutral).
    let down_smash = ambition_entity_catalog::smash_mine::author_place_mine(
        down_smash,
        // End of the active window: the sweep plants it. Placing during startup
        // would let her cancel the smash and keep the mine.
        0.28,
        ambition_entity_catalog::smash_mine::PlaceMineParams {
            item_id: MINE_ITEM.to_string(),
            // Longer than the move (0.57s), so plant-and-detonate is never one
            // continuous input.
            arm_s: 1.2,
            // Below the bomb's 12 on purpose: a mine picks its own moment, so it
            // should not also win on damage.
            damage: 10,
            blast_radius: 52.0,
            half_extents: (8.0, 8.0),
            // Behind and below the sweep, like the bomb, so it is not inside her body
            // where nobody can pick it up.
            offset: (-18.0, 14.0),
        },
    );

    let neutral_air = strike(Strike {
        id: "polygon_projectile_air_neutral",
        clip: "air_neutral",
        startup_s: 0.05,
        active_s: 0.10,
        recover_s: 0.14,
        offset: (7.0, 0.0),
        half_extents: (24.0, 23.0),
        damage: 7,
        knockback: 79.0,
        knockback_growth: 1.50,
        launch_dir: None,
        on_hit: None,
    });
    let forward_air = strike(Strike {
        id: "polygon_projectile_air_forward",
        clip: "air_forward",
        startup_s: 0.08,
        active_s: 0.08,
        recover_s: 0.17,
        offset: (28.0, -3.0),
        half_extents: (22.0, 18.0),
        damage: 9,
        knockback: 108.0,
        knockback_growth: 2.05,
        launch_dir: Some((1.0, -0.32)),
        on_hit: None,
    });
    let back_air = strike(Strike {
        id: "polygon_projectile_air_back",
        clip: "air_back",
        startup_s: 0.09,
        active_s: 0.07,
        recover_s: 0.18,
        offset: (-26.0, -1.0),
        half_extents: (22.0, 17.0),
        damage: 10,
        knockback: 124.0,
        knockback_growth: 2.30,
        launch_dir: Some((-1.0, -0.30)),
        on_hit: None,
    });
    let up_air = strike(Strike {
        id: "polygon_projectile_air_up",
        clip: "air_up",
        startup_s: 0.06,
        active_s: 0.08,
        recover_s: 0.14,
        offset: (2.0, -27.0),
        half_extents: (21.0, 23.0),
        damage: 8,
        knockback: 99.0,
        knockback_growth: 1.88,
        launch_dir: Some((0.0, -1.0)),
        on_hit: None,
    });
    let mut down_air = strike(Strike {
        id: "polygon_projectile_air_down",
        clip: "air_down",
        startup_s: 0.11,
        active_s: 0.08,
        recover_s: 0.21,
        offset: (2.0, 25.0),
        half_extents: (21.0, 21.0),
        damage: 10,
        knockback: 126.0,
        knockback_growth: 2.35,
        launch_dir: Some((0.0, 1.0)),
        on_hit: None,
    });
    down_air.landing_lag_s = Some(0.25);

    let neutral_special = charge_shot();

    // Side: `polygon_ponytail_boomerang`. Jon: *"use her ponytail as a boomarang
    // for her side-b."*
    //
    // It uses the same seam as the admiral's gun-sword: `MoveSpec::equips` puts
    // a thing in her hands while the move's clock runs and fires its ranged
    // verb. So grab-and-throw is one move, and the tail's flight is authored on
    // the tail.
    //
    // No melee volume: the tail is the damage, going out and coming back.
    let side_special = ambition_entity_catalog::authoring::hitless_special(
        "polygon_ponytail_boomerang",
        "attack_side",
        PONYTAIL_THROWN_AT_S,
        PONYTAIL_ENDS_S,
    );
    let mut side_special = side_special;
    side_special.display_name = Some("Ponytail".to_string());
    side_special.equips = Some(PONYTAIL.to_string());
    side_special.events.push(MoveEvent {
        at_s: PONYTAIL_THROWN_AT_S,
        kind: MoveEventKind::Ranged,
    });
    // A small additive step on the throw: a lean, not a lunge, and no route for
    // a recovery search.
    let side_special = impulse(
        side_special,
        PONYTAIL_THROWN_AT_S,
        (140.0, 0.0),
        ImpulseMode::Add,
    );

    let mut uppercut = strike(Strike {
        id: "polygon_projectile_recoil_lift",
        clip: "attack_up",
        startup_s: 0.08,
        active_s: 0.12,
        recover_s: 0.19,
        offset: (5.0, -20.0),
        half_extents: (22.0, 27.0),
        damage: 9,
        knockback: 104.0,
        knockback_growth: 1.88,
        launch_dir: Some((0.08, -1.0)),
        on_hit: None,
    });
    uppercut.landing_lag_s = Some(0.25);
    let up_special = impulse(uppercut, 0.08, (0.0, -745.0), ImpulseMode::Set);

    // The up-B throws her line: a tether that bites a ledge and reels her in,
    // the ranged fighter's version of her grab. The reach is
    // `THE_TETHERS_REACH`, the same as her grab.
    //
    // It catches nothing by itself. The reel brings her to the anchor the ledge
    // authority wants and lets go; `try_start_ledge_grab_clusters_in_frame`
    // decides if she catches, with its cooldown and eligibility. See
    // `ambition_demo_smash::tether`.
    //
    // It must be aimed: the line goes where she faces. Facing away from the
    // stage gives only the 745px/s pop.
    let up_special = ambition_entity_catalog::smash_tether::author_tether_pull(
        up_special,
        // Just after the pop, so the line goes out while she is still rising.
        0.10,
        ambition_entity_catalog::smash_tether::TetherPullParams {
            reach: THE_TETHERS_REACH,
            // Faster than the pop: once the line bites, the reel is her motion, and a
            // reel slower than a fall would lose ground.
            speed: 900.0,
            // 315px of travel for a 150px line: a generous failsafe for a ledge that
            // stops being reachable.
            timeout_s: 0.35,
        },
    );

    // Down (grounded): `polygon_lay_bomb`. She puts a live bomb on the floor.
    //
    // Jon: *"The bomb should detonate in 4 seconds or if it hits something with
    // enough velocity, whichever comes first."* Anyone can pick it up and throw
    // it.
    //
    // The bomb is a ground item, so pick-up and throw come from existing engine
    // machinery. See `ambition_demo_smash::bomb`.
    //
    // No melee volume: the bomb is the hit.
    let grounded_down_special = ambition_entity_catalog::authoring::hitless_special(
        "polygon_lay_bomb",
        "attack_down",
        BOMB_LAID_AT_S,
        BOMB_ENDS_S,
    );
    let mut grounded_down_special = grounded_down_special;
    grounded_down_special.display_name = Some("Lay a Bomb".to_string());
    let grounded_down_special = ambition_entity_catalog::smash_bomb::author_drop_bomb(
        grounded_down_special,
        BOMB_LAID_AT_S,
        ambition_entity_catalog::smash_bomb::DropBombParams {
            item_id: BOMB_ITEM.to_string(),
            // The design value: detonate in 4 seconds.
            fuse_s: 4.0,
            damage: 12,
            blast_radius: 56.0,
            // "Enough velocity" sits between the two ways a bomb stops:
            // `THROW_SPEED_X` is 320, so a thrown bomb detonates on impact and a dropped
            // one does not.
            impact_speed: 260.0,
            half_extents: (9.0, 9.0),
            // Just behind and below her, so it is not inside her body where nobody can
            // pick it up.
            offset: (-16.0, 14.0),
        },
    );
    let grounded_down_special =
        ambition_entity_catalog::authoring::sfx(grounded_down_special, BOMB_LAID_AT_S, "player.land.heavy");
    let grounded_down_special = ambition_entity_catalog::authoring::vfx(
        grounded_down_special,
        BOMB_LAID_AT_S,
        "poof_small",
    );

    let mut airborne_down_special = strike(Strike {
        id: "polygon_projectile_downward_vector",
        clip: "air_down",
        startup_s: 0.10,
        active_s: 0.11,
        recover_s: 0.22,
        offset: (0.0, 24.0),
        half_extents: (24.0, 24.0),
        damage: 12,
        knockback: 132.0,
        knockback_growth: 2.42,
        launch_dir: Some((0.0, 1.0)),
        on_hit: None,
    });
    airborne_down_special.landing_lag_s = Some(0.29);
    let airborne_down_special =
        impulse(airborne_down_special, 0.10, (0.0, 1080.0), ImpulseMode::Set);

    // The tether is her grab, not a special or a new mechanic: she is the
    // grid's ranged fighter. `acquire_captures` builds its box as
    // `CenteredAabb::new(captor.pos + placed.world_offset, placed.half_extent)`
    // with no distance limit in `ambition_combat::capture`, so a tether grab is
    // only authored reach.
    //
    // The height shrinks as the reach grows. 150px of reach at 16px tall would
    // be a grabbing wall; at 10px it is a line she must aim by height.
    //
    // The recovery pays for it: 0.34s (was 0.21s) and a slower extension
    // (0.10s startup, was 0.06s). A whiffed tether is a committed animation at
    // long range.
    let grab = author_standing_grab(
        grab_shell("polygon_projectile_grab", "grab", 0.10, 0.06, 0.34),
        CaptureAttemptParams {
            // 86 + 64 = `THE_TETHERS_REACH`. Her up-B reads the constant, so the two
            // stay equal.
            offset: (86.0, 1.0),
            half_extents: (64.0, 10.0),
            // The same hold as before: where a captive is held depends on her hands,
            // not on how far away she caught them, and the throws are shared.
            hold_offset: (14.0, 3.0),
        },
    );
    let pummel = author_pummel(
        capture_beat("polygon_projectile_pummel", "pummel", 0.15),
        0.06,
        CapturePummelParams { damage: 4 },
    );
    let forward_throw = author_throw(
        capture_beat("polygon_projectile_throw_forward", "throw_forward", 0.24),
        0.11,
        CaptureThrowParams {
            damage: 8,
            knockback: 116.0,
            knockback_growth: 2.22,
            launch_dir: (1.0, -0.30),
        },
    );
    let back_throw = author_throw(
        capture_beat("polygon_projectile_throw_back", "throw_back", 0.26),
        0.12,
        CaptureThrowParams {
            damage: 9,
            knockback: 126.0,
            knockback_growth: 2.35,
            launch_dir: (-1.0, -0.27),
        },
    );
    let up_throw = author_throw(
        capture_beat("polygon_projectile_throw_up", "throw_up", 0.25),
        0.11,
        CaptureThrowParams {
            damage: 8,
            knockback: 120.0,
            knockback_growth: 2.28,
            launch_dir: (0.0, -1.0),
        },
    );
    let down_throw = author_throw(
        capture_beat("polygon_projectile_throw_down", "throw_down", 0.27),
        0.12,
        CaptureThrowParams {
            damage: 7,
            knockback: 88.0,
            knockback_growth: 1.82,
            launch_dir: (0.28, -0.96),
        },
    );

    SmashRepertoire {
        // The genre shapes are deliberate: this is still a reusable reference fighter.
        // Projectile identity belongs to the head cannon and shoot pose, not
        // to making every grounded movement action species-specific.
        taunt: ambition_entity_catalog::authoring::taunt("projectile_polygon_taunt", 0.9),
        dash_attack: ambition_entity_catalog::authoring::dash_attack(
            "projectile_polygon_dash_attack",
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

    /// Her neutral game has no hit volume.
    ///
    /// The boomerang and the charge shot author no Active volume (the projectile
    /// is the damage), so `MoveFrameData::coverage` is `None` for both. An option
    /// layer that read `None` as "reaches nobody" would remove both from her
    /// attack menu. She is admitted through `hazard_reach`, from the
    /// `MoveEventKind::Ranged` event.
    ///
    /// This tests the join, not the number: 1000px is a stage-crossing
    /// placeholder, because the body owns the shot's real speed and flight.
    /// Retune this test if that changes; do not delete it.
    #[test]
    fn her_two_neutral_projectiles_tell_the_brain_they_cross_the_stage() {
        let set = crate::authored_movesets::shipped("projectile_polygon");
        for id in [
            "polygon_ponytail_boomerang",
            "polygon_projectile_charge_shot",
        ] {
            let m = set
                .moves
                .iter()
                .find(|m| m.id == id)
                .unwrap_or_else(|| panic!("`{id}` is in her moveset"));
            let frames = m.frame_data();
            // Premise: if one of these gets a hit volume, the test below means
            // nothing.
            assert!(
                frames.coverage.is_none(),
                "`{id}` now authors a hit volume, so this test no longer \
                 guards the road it was written for",
            );
            // A request, not a distance. The shot's speed, flight and lifetime are on
            // the body, so the catalog asks and the kit builder answers with her
            // `RangedActionSpec`. A reader that never joins a body gets the standing
            // `RANGED_ACTION_REACH` through `MoveHazard::reach`, asserted beside it.
            assert_eq!(
                frames.hazard,
                Some(ambition_entity_catalog::MoveHazard::OwnersRangedAction),
                "`{id}` fires the body's ranged action but tells the option \
                 layer it reaches nowhere, so it would never be offered",
            );
            assert_eq!(
                frames.hazard.expect("checked above").reach(),
                ambition_entity_catalog::RANGED_ACTION_REACH,
            );
        }
    }

    /// Control: a move that reaches through something it spawns itself answers
    /// that thing's flight, not the placeholder. Without this, a catalog that
    /// answered every hitless move 1000px would pass the test above.
    #[test]
    fn her_bomb_answers_its_own_flight_rather_than_the_ranged_placeholder() {
        let set = crate::authored_movesets::shipped("projectile_polygon");
        let bomb = set
            .moves
            .iter()
            .find(|m| m.id == "polygon_lay_bomb")
            .expect("she lays a bomb");
        let hazard = bomb
            .frame_data()
            .hazard
            .expect("her bomb puts a hazard in the world");
        let reach = hazard.reach();
        // Not live when it lands: laying the bomb is not a hit, and the fuse is
        // four seconds. Pricing the drop as an immediate blast would price a trap
        // as a strike.
        assert_eq!(
            hazard.detonates_by_s(),
            4.0,
            "her bomb's fuse is not the four seconds it authors: {}s",
            hazard.detonates_by_s()
        );
        // Its travel is zero: it is placed, so there is nothing to aim. See
        // `ThreatTravel::live_at_s`.
        assert_eq!(hazard.travel_to(reach - 1.0), Some(0.0));
        assert_eq!(
            hazard.travel_to(reach + 1.0),
            None,
            "her bomb answered a distance outside its own blast"
        );
        assert!(
            reach > 0.0 && reach < ambition_entity_catalog::RANGED_ACTION_REACH,
            "her bomb reaches {reach}px, which is either nothing or the \
             ranged placeholder — neither is its own arc",
        );
    }

    /// Her grab's comment says 86 + 64 is the tether's reach. Check it: retuning
    /// either move alone fails this.
    #[test]
    fn the_tether_reaches_as_far_as_her_grab() {
        let set = crate::authored_movesets::shipped("projectile_polygon");
        let grab = set
            .moves
            .iter()
            .find(|m| m.id == "polygon_projectile_grab")
            .expect("she has a grab");
        // A grab's reach is not on its timeline. `author_standing_grab` hangs the
        // capture on the active window as a `sustain_effect`, so a scan of `events`
        // finds nothing.
        let capture: ambition_entity_catalog::smash_capture::CaptureAttemptParams = grab
            .windows
            .iter()
            .filter_map(|window| window.sustain_effect.as_ref())
            .find(|effect| effect.key == ambition_entity_catalog::smash_capture::CAPTURE_ATTEMPT)
            .and_then(|effect| effect.params.hydrate().ok())
            .expect("her grab captures");

        let lift = set
            .moves
            .iter()
            .find(|m| m.id == "polygon_projectile_recoil_lift")
            .expect("she has an up-B");
        let tether: ambition_entity_catalog::smash_tether::TetherPullParams = lift
            .events
            .iter()
            .find_map(|event| match &event.kind {
                ambition_entity_catalog::MoveEventKind::Effect(effect)
                    if effect.key == ambition_entity_catalog::smash_tether::TETHER_PULL =>
                {
                    effect.params.hydrate().ok()
                }
                _ => None,
            })
            .expect("her up-B throws the tether");

        assert_eq!(
            tether.reach,
            capture.reach_x(),
            "her up-B throws a {}px line while her grab reaches {}px — one \
             fiction, two numbers",
            tether.reach,
            capture.reach_x(),
        );
        assert_eq!(tether.reach, THE_TETHERS_REACH);
    }
    use super::*;

    #[test]
    fn the_reference_projectile_fighter_answers_the_complete_typed_repertoire() {
        let moves = crate::authored_movesets::shipped("projectile_polygon");
        for id in [
            "polygon_projectile_jab",
            "polygon_projectile_tilt_forward",
            "polygon_projectile_tilt_up",
            "polygon_projectile_tilt_down",
            "polygon_projectile_smash_forward",
            "polygon_projectile_smash_up",
            "polygon_projectile_smash_down",
            "polygon_projectile_air_neutral",
            "polygon_projectile_air_forward",
            "polygon_projectile_air_back",
            "polygon_projectile_air_up",
            "polygon_projectile_air_down",
            // The charge shot is the neutral special.
            "polygon_projectile_charge_shot",
            "polygon_ponytail_boomerang",
            "polygon_projectile_recoil_lift",
            "polygon_lay_bomb",
            "polygon_projectile_downward_vector",
            "polygon_projectile_grab",
            "polygon_projectile_pummel",
            "polygon_projectile_throw_forward",
            "polygon_projectile_throw_back",
            "polygon_projectile_throw_up",
            "polygon_projectile_throw_down",
            "projectile_polygon_taunt",
            "projectile_polygon_dash_attack",
        ] {
            assert!(moves.moves.iter().any(|m| m.id == id), "missing {id}");
        }
    }

    /// The mine is an addition and the swing is unchanged. Both in one test: a
    /// check for the mine alone would pass a down smash that lost its hitbox.
    #[test]
    fn her_down_smash_still_swings_and_now_also_plants_a_mine() {
        let moves = crate::authored_movesets::shipped("projectile_polygon");
        let down_smash = moves
            .moves
            .iter()
            .find(|m| m.id == "polygon_projectile_smash_down")
            .expect("she has a down smash");

        // The swing, unchanged.
        assert_eq!(down_smash.smash_charge_mult, 1.75, "still a charged smash");
        assert!(
            down_smash
                .windows
                .iter()
                .flat_map(|window| window.volumes.iter())
                .any(|volume| volume.damage > 0),
            "the down smash still has a hit volume that hurts"
        );

        // The mine, added.
        let params = down_smash
            .events
            .iter()
            .find_map(|event| match &event.kind {
                ambition_entity_catalog::MoveEventKind::Effect(effect)
                    if effect.key == ambition_entity_catalog::smash_mine::PLACE_MINE =>
                {
                    Some(
                        effect
                            .params
                            .hydrate::<ambition_entity_catalog::smash_mine::PlaceMineParams>()
                            .expect("place-mine params hydrate"),
                    )
                }
                _ => None,
            })
            .expect("her down smash plants a mine");

        // Plant-and-detonate must never be one continuous input: the mine is still
        // arming when the planting move ends.
        assert!(
            params.arm_s > down_smash.duration_s,
            "the mine arms in {}s but the move lasts {}s, so she could plant and \
             detonate without ever letting go",
            params.arm_s,
            down_smash.duration_s,
        );

        // The object must be pickable, so its held item must be registered. Its art
        // is checked in `items::held_visuals` (`register` is
        // `pub(in crate::items)`).
        assert!(
            ambition_characters::brain::held_item_by_id(&params.item_id).is_some(),
            "`{}` is not a registered held item, so nobody could pick the mine up",
            params.item_id,
        );
    }
}

#[cfg(test)]
mod threat_timing_tests {
    use super::*;

    /// A projectile goes live when it is thrown, not when the move ends.
    ///
    /// `startup_s` is "time until the first Active window" and falls back to the
    /// whole move duration when there is none, as for every ranged move. Leading
    /// a target by it overshoots: `charge_shot` fires at 0.26s but reports
    /// `startup_s` 0.58s, which at 200px/s closing speed is 64px of error against
    /// an `ADMISSION_SLACK_PX` of 24.
    ///
    /// Control: [`a_strike_threatens_when_its_hitbox_opens`], where the two
    /// numbers are equal. Without it, a field that is always early would pass.
    #[test]
    fn a_projectile_threatens_when_it_is_thrown_not_when_the_move_ends() {
        let set = crate::authored_movesets::shipped("projectile_polygon");
        // Read the authored trigger times from the constants the moves are built
        // from, not from the events the derivation reads, so the test does not
        // restate the implementation.
        for (id, thrown_at) in [
            ("polygon_projectile_charge_shot", CHARGE_FIRE_AT_S),
            ("polygon_ponytail_boomerang", PONYTAIL_THROWN_AT_S),
            ("polygon_lay_bomb", BOMB_LAID_AT_S),
        ] {
            let spec = set
                .moves
                .iter()
                .find(|m| m.id == id)
                .unwrap_or_else(|| panic!("{id} is on the reference projectile fighter"));
            let f = spec.frame_data();
            let live = f.threat_live_at_s.unwrap_or_else(|| {
                panic!(
                    "{id} threatens nobody at any time, so it is off the attack \
                     menu entirely — see `hazard_reach`"
                )
            });
            assert!(
                live < f.startup_s,
                "{id} throws at {live:.2}s and reports `startup_s` {:.2}s; \
                 equal means a consumer leading by the threat time is leading \
                 by the whole move duration after all",
                f.startup_s
            );
            // And it is the authored throw time, not just something smaller.
            assert!(
                (live - thrown_at).abs() < 1e-4,
                "{id} is authored to throw at {thrown_at:.3}s and reports its \
                 threat live at {live:.3}s"
            );
        }
    }

    /// Control: for a move whose threat is its hitbox, the two numbers are equal
    /// by construction.
    #[test]
    fn a_strike_threatens_when_its_hitbox_opens() {
        let set = crate::authored_movesets::shipped("projectile_polygon");
        let jab = set
            .moves
            .iter()
            .find(|m| m.id == "polygon_projectile_jab")
            .expect("she has a jab");
        let f = jab.frame_data();
        assert_eq!(
            f.threat_live_at_s,
            Some(f.startup_s),
            "an ordinary swing's threat is its first Active window, so the two \
             must agree — if they can differ here, the subject arm's \
             `live < startup_s` is a statement about the FIELD rather than \
             about ranged moves"
        );
    }

    /// A move that offers the opponent nothing reports `None`: the same
    /// population the attack menu's third arm refuses.
    #[test]
    fn a_move_that_threatens_nobody_names_no_time() {
        let set = crate::authored_movesets::shipped("projectile_polygon");
        let lift = set
            .moves
            .iter()
            .find(|m| m.id == "polygon_projectile_recoil_lift")
            .expect("she has a recoil lift");
        let f = lift.frame_data();
        if f.coverage.is_none() && f.push_coverage.is_none() && f.hazard.is_none() {
            assert_eq!(
                f.threat_live_at_s, None,
                "a move with no coverage, no shove and no hazard named a time \
                 at which it threatens somebody"
            );
        } else {
            assert!(
                f.threat_live_at_s.is_some(),
                "a move that covers, shoves or spawns a hazard must say WHEN"
            );
        }
    }
}
