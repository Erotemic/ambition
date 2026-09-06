//! Pirate Admiral's authored Smash repertoire.
//!
//! The kit emphasizes cutlass reach and a SUMMONED VEHICLE for a recovery — the
//! up-B calls a burning flying shark and rides it (D207). The retired grapple spends
//! most of its displacement across the stage, so vertical recovery still depends on the
//! body's other movement resources. The pistol remains an `ActionSet` capability rather
//! than a move-table entry, keeping ranged execution under one authority.

use ambition_characters::moveset_authoring::Strike;
use ambition_characters::smash_capture::{
    author_pummel, author_standing_grab, author_throw, capture_beat, grab_shell,
    CaptureAttemptParams, CaptureCues, CapturePummelParams, CaptureThrowParams,
    SmashCaptureRepertoire,
};
use ambition_characters::smash_repertoire::{
    DownSpecial, NeutralSpecial, SmashRepertoire, UpSpecial,
};
use ambition_platformer2d::entity_catalog::{
    ImpulseMode, MoveEvent, MoveEventKind, MovesetContract,
};

use ambition_characters::moveset_authoring::{impulse, on_contact, sfx, strike, vfx, vfx_at};

/// How far across the grapple hauls him, engine units per second along
/// facing.
///
/// the number that IS the move. A recovery is usually authored as a rise; this
/// one spends its whole budget on distance, so the admiral's problem offstage is
/// never *"can I climb back to the lip"* but *"can I reach the lip at all"*.
/// The character the admiral's up-B summons to ride.
///
/// ⭐ AUTHORED CONTENT THAT ALREADY EXISTED. `npc_burning_flying_shark` is the
/// mount half of the pirate sky-rider pair ADR 0020 built — a `Mountable` of
/// class `shark` with an authored saddle offset — so this move seats the admiral
/// on the same shark his raiders have always flown, rather than minting a
/// fighter-only copy of one.
pub(crate) const SHARK_CHARACTER: &str = "npc_burning_flying_shark";

/// When in the up-B the shark arrives. Long enough to read as a summon and be
/// punishable; short enough to still be a recovery.
pub(crate) const SHARK_AT_S: f32 = 0.18;

/// When the move itself ends. The RIDE outlives it — the admiral is flying by
/// then and the move's tail is only the animation of having called it.
pub(crate) const SHARK_ENDS_S: f32 = 0.34;

/// How long the admiral may stay aboard. Jon's number, and explicitly a first
/// pass: *"maybe 5 seconds is too long, but that's where I want it right now."*
pub(crate) const SHARK_RIDE_SECONDS: f32 = 5.0;

/// The weapon the side-B draws. His own row rather than the shared `gun_sword`,
/// which is a pickup in the adventure game and a raider's sidearm — a
/// side-special's payoff is not the number either of those should be balanced
/// around.
pub(crate) const ADMIRAL_GUN_SWORD: &str = "admiral_gun_sword";

/// When the side-B fires. Long enough that the draw reads as a draw and the
/// move is punishable on reaction; short enough to still answer a press.
const GUNS_FIRE_AT_S: f32 = 0.20;

/// When the side-B ends, and with it the brandish. The tail is him putting the
/// gun-sword away, which is the half of the animation that makes the draw read
/// as temporary rather than as a permanent second weapon.
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
    // the slowest kill move of the three tables, and the hardest. An admiral
    // who commits to a full cutlass swing has decided the exchange is worth 0.38s
    // of standing still afterwards.
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
        knockback_growth: 3.10,
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
        knockback_growth: 2.95,
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
        knockback_growth: 2.70,
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
    // THE STALL. A rising cutlass overhead that takes the admiral up with
    // it — the genre's juggle aerial, and the reason this table can chain one
    // hit into the next instead of falling out from under its own combo.
    let u_air = impulse(u_air, 0.08, (0.0, -360.0), ImpulseMode::Set);
    let u_air = sfx(u_air, 0.0, "player.robot.slash.air");
    let u_air = on_contact(u_air, "player.robot.slash.impact.flesh.light");
    let u_air = vfx(u_air, 0.08, "burst_round");

    // a real spike: point-down cutlass, straight into the blast zone. no
    // `on_hit` rebound, same as the goblin's — the robot is the only body that
    // says it can bounce off what it hits, and that is a property of the
    // character rather than of down-airs.
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
    // four MECHANISMS, and none of them is another one rotated. One fires
    // forward and shoves its owner backward; one adds to whatever the body was
    // already doing; one commands a diagonal haul; one commands a full stop. The
    // shared `strike` gives them all the same timeline shape, so what separates
    // them is what they do to the ADMIRAL, which is the only axis a table can
    // differentiate a special on without new engine mechanisms.

    // NEUTRAL — `grapeshot`. A pistol at the hip, and the recoil that comes
    // with firing one from a standing start. The volume is short and wide; the
    // admiral is thrown BACKWARD out of it, which is a real spacing tool and a
    // real way to remove yourself from the stage.
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
    // negative side, and that sign is the whole move. The catalog reads
    // this as a route with `lift_side < 0` — a displacement that carries its
    // owner AWAY from whatever it is facing. A recovery search will happily
    // propose it and the kernel will decline it every time it is thrown toward
    // the stage, which is the correct outcome and needs no rule anywhere.
    let neutral_b = impulse(neutral_b, 0.14, (-560.0, -120.0), ImpulseMode::Set);
    let neutral_b = sfx(neutral_b, 0.0, "player.attack.charge");
    let neutral_b = sfx(neutral_b, 0.14, "player.slash");
    let neutral_b = vfx(neutral_b, 0.14, "smoke_burst");
    let neutral_b = on_contact(neutral_b, "world.rock.hit");

    // SIDE — `run_out_the_guns`. HE DRAWS THE GUN-SWORD AND FIRES IT.
    //
    // ⭐⭐ JON'S DESIGN, 2026-08-27: *"The pirate side b should briefly equip the
    // lasergun sword and fire a lasersword projectile in the left/right
    // direction the side b was directed towards. When the side-b resolves it
    // should locate the nearest opponent and angle the equipped gun and shot so
    // it fires in their direction if they are in the half plane the side-b was
    // directed towards."*
    //
    // Three statements, each made where it belongs:
    //
    // ⭐ THE DRAW is `MoveSpec::equips`. The move's own clock is the timer, so
    // there is no equip duration to keep in step with the animation: a move
    // interrupted at frame three puts the sword away at frame three, and the
    // admiral's own hands come back after (`MoveBrandishedItem` remembers what
    // it displaced).
    //
    // ⭐ THE SHOT is `MoveEventKind::Ranged`, which fires the weapon in the hand
    // — the drawn `admiral_gun_sword`, not the pistol his catalog row gives him.
    // Everything a gun-sword discharge looks and sounds like comes off that
    // weapon's authored `Discharge`: the spinning `lasersword` projectile, the
    // muzzle at his hand, `weapon.lasersword.fire`, the heavier recoil.
    //
    // ⛔⛔ THIS PARAGRAPH WAS FALSE FOR A DAY. Those four choices were made at
    // the fire site by `held_item_id == Some("gun_sword")`, and the admiral's
    // sidearm is a different string — so the move his own comment describes
    // fired a generic shot out of his midriff. A comment stating a rule is a
    // specification; this one is now checked by
    // `the_admirals_side_b_fires_the_gun_swords_discharge`.
    //
    // ⭐ THE ANGLE is the weapon's `AimAssist::half_plane`, authored on the
    // gun-sword row. The player picks the side; the weapon picks the angle
    // within it.
    //
    // ⛔ NO MELEE VOLUME. This replaces `boarding_run`, a shoulder-first charge
    // whose damage was a body-to-body hitbox — and a move that both fired a
    // projectile and carried a strike would be two moves wearing one button. The
    // projectile IS the damage, as it is for every ranged move in the tree.
    //
    // ⛔ AND IT KEEPS THE CHARGE'S FORWARD STEP, which is the only thing the old
    // move contributed that this one still wants: an `Add` impulse that
    // CONTRIBUTES to his momentum, so the shot is longest out of a run and
    // nearly nothing from a standstill. It advertises no route (a static reader
    // cannot say what an additive impulse produces), which is why the recovery
    // search never proposes this move as a way home.
    let side_b = ambition_characters::moveset_authoring::hitless_special(
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

    // UP — `call_the_shark`. THE RECOVERY, AND IT IS A VEHICLE.
    //
    // ⭐⭐ JON'S DESIGN, 2026-08-26: *"their up-b should summon a burning flying
    // shark that they can mount and ride (and effectively fly around for a
    // limited time using the control stick)."* It replaces `grapple_line`, a
    // hauled boarding line whose recovery was almost all horizontal.
    //
    // ⛔ NO ATTACK HITBOX ANYWHERE ON IT — Jon: *"There is no hurtbox on this
    // up-b, it's purely a mobility special"*, and the thing he means is the
    // striking half: the move hits nobody. That is why it is a
    // `hitless_special` rather than a `strike` carrying an empty volume list.
    //
    // ⚠ THE SHARK STILL HAS A HURTBOX, and must: Jon asked for it to die to a
    // damage threshold so an opponent can gimp the recovery by killing it. What
    // the shark declines is CONTACT damage — it is `Neutral` and takes no side —
    // not the ability to be hit.
    //
    // ⭐ THE PRICE IS THE BUDGET, NOT FREEFALL. `author_summon_ride` sets both
    // gates together: one shark per airtime like anybody else's recovery, and no
    // helpless episode, because a rider that cannot act is not riding. Jon is
    // explicit that this is a strong recovery and that the balancing comes
    // later: *"This is a recovery, and yes it is a strong recovery. We will
    // balance later, maybe 5 seconds is too long, but that's where I want it
    // right now."*
    //
    // ⭐ NO IMPULSE. The shark appears where the admiral is and the climb is the
    // player's to steer, which is the whole mechanic — a rise bolted on top
    // would be a second recovery inside the first, and it is the thing to reach
    // for first if five seconds of flight turns out not to save him.
    let up_b = ambition_characters::moveset_authoring::hitless_special(
        "call_the_shark",
        "special_up",
        SHARK_AT_S,
        SHARK_ENDS_S,
    );
    let up_b = ambition_characters::smash_ride::author_summon_ride(
        up_b,
        SHARK_AT_S,
        ambition_characters::smash_ride::SummonRideParams {
            character_id: SHARK_CHARACTER.to_string(),
            // The authored shark body, so the summoned mount is the one the
            // saddle offset on its `Mountable` was authored against.
            half_extents: (48.0, 22.0),
            seconds: SHARK_RIDE_SECONDS,
            // ⭐ HALF THE RIDE'S STRAIGHT-LINE DISTANCE (the shark's own
            // `run_speed` is 260px/s over five seconds). Deliberately under the
            // arithmetic: a recovery is a turn back toward the stage rather than
            // a sprint in one direction, and a search that over-claims kills the
            // fighter it was trying to save.
            reach: SHARK_RIDE_SECONDS * 260.0 * 0.5,
        },
    );
    let up_b = sfx(up_b, 0.0, "player.attack.charge");
    let up_b = sfx(up_b, SHARK_AT_S, "player.robot.slash.air");
    // a recovery activating gets its own burst: seeing one is how the other
    // player knows this fighter is not dead yet.
    let up_b = vfx(up_b, SHARK_AT_S, "classic_burst");

    // DOWN — `heave_to`. The anchor. It commands a FULL STOP: `(0, 0)`, a
    // `Set` of zero.
    //
    // the only move in the repo whose commanded velocity is nothing, and it is
    // a real one — it kills the drift a launch gave you, which is what turns a
    // read into a survivable one. it advertises NO route (`local.1` is not
    // negative, so the catalog's lift derivation skips it), which is correct:
    // stopping dead in mid-air is not a way home from anywhere.
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

    // the forward tilt. With all four specials authored, this was the only
    // press on the admiral that fell down the directional chain — a fighter
    // carrying a cutlass answering "forward" with a jab. A LEVEL CUT at chest
    // height: the longest tilt on the grid, because reach is what the cutlass is
    // for, and slower than the goblin's whole jab because carrying one costs.
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
    // the deliberate opposite of George's, and the pair is the point: two
    // fighters authored through two different providers, sharing no numbers.
    //
    // The admiral carries a cutlass and boards ships. His grab is FAST (`0.07`
    // startup, half of George's `0.14`) and SHORT (`19` of reach against
    // George's `26`) — you have to be on top of somebody to board them, and once
    // you have decided to, it happens. Recovery `0.20` against George's `0.30`:
    // whiffing costs him a third less, because his grab is a scramble tool and
    // George's is a commitment.
    //
    // the hold sits CLOSE and LOW (`13` forward, `+3` down): hauled in against
    // the chest, not held out at arm's length.
    let grab = author_standing_grab(
        grab_shell("pirate_grab", "grab", 0.07, 0.05, 0.20),
        CaptureAttemptParams {
            offset: (12.0, 1.0),
            half_extents: (19.0, 16.0),
            hold_offset: (13.0, 3.0),
        },
    );
    // A FAST, LIGHT pummel — `0.13` and `2`, against George's `0.24` and `4`.
    // Nearly twice the rate for half the damage: the same damage per second by
    // arithmetic and a completely different thing to play against, because every
    // beat is a chance for the hold to break.
    let pummel = author_pummel(
        capture_beat("pirate_pummel", "attack", 0.13),
        0.06,
        CapturePummelParams { damage: 2 },
    );
    // an UPWARD throw wearing the forward slot, and that is a character
    // fact rather than a mistake. George throws flat and across for stage
    // control; the admiral heaves a body up and slightly forward (`0.55` lateral
    // against `-1.0` vertical) so it lands in front of him and he keeps swinging.
    // Less knockback than George's (`104` against `138`) and more growth (`2.4`
    // against `1.9`): weak early, frightening late.
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
        taunt: ambition_characters::moveset_authoring::taunt("pirate_admiral_taunt", 0.9),
        dash_attack: ambition_characters::moveset_authoring::dash_attack(
            "pirate_admiral_dash_attack",
            ambition_characters::moveset_authoring::DashAttackShape::GENRE,
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
        // the second fighter to author one, and through a DIFFERENT
        // provider — this crate, not the smash demo. That is the falsifier:
        // the capture vocabulary is not quietly tied to one game-owned file.
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
    use ambition_platformer2d::entity_catalog::{MoveSpec, VolumeShape, WindowTag};

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

    // Fourteen fighters each carried a copy of it: every bound verb names a move
    // this table defines, and the table binds the whole vocabulary. Both are now
    // unwritable defects rather than tested ones. `SmashRepertoire` owns the verb
    // strings, so there is no string in this file to misspell; it is a struct
    // with no `Default` and no private fields, so a missing or renamed slot is a
    // COMPILE error here. What the fourteen copies stood for — that every press
    // is answered, in every posture it is asked in — is checked once, by
    // `ambition_characters::smash_repertoire`, and by the host ratchet
    // `smash_roster_movesets::report_the_smash_kit_every_selectable_fighter_has`.

    /// The commanded (`Set`) velocity a move states, if it states one.
    fn commanded(set: &MovesetContract, id: &str) -> Option<(f32, f32)> {
        use ambition_platformer2d::entity_catalog::MoveEventKind;
        find(set, id).events.iter().find_map(|e| match &e.kind {
            MoveEventKind::Impulse {
                local,
                mode: ImpulseMode::Set,
            } => Some(*local),
            _ => None,
        })
    }

    /// THE POISON, AUTHORED: A TINY UPWARD ATTACK OUTRANKS THE RECOVERY ON
    /// THE SCALAR, AND MUST NOT BE THE RECOVERY.
    ///
    /// this test asserts the SETUP, not the fix. The fix lives in
    /// `RecoveryLens::best_route`, which searches every route instead of ranking
    /// them, and it is pinned there
    /// (`a_tiny_lifting_move_does_not_suppress_a_viable_recovery`). What is
    /// pinned HERE is that the setup is real — that this table genuinely
    /// contains a kit where the scalar ordering and the useful ordering
    /// disagree. If a retune ever makes them agree, that fixture stops standing
    /// for anything and this test says so.

    // -----------------------------------------------------------------------
    // ⛔⛔ THE ADMIRAL'S RECOVERY-SEARCH FIXTURE LIVED HERE AND IS GONE WITH THE
    // GRAPPLE. Two tests drove the real `RecoveryLens` over this fighter's own
    // airborne kit — one pinning that the scalar ordering and the useful
    // ordering disagree in it (the fixture behind
    // `a_tiny_lifting_move_does_not_suppress_a_viable_recovery`), one pinning
    // that the search picks the grapple anyway. Both read `grapple_line`'s
    // commanded velocity, and the shark up-B commands none: it is a recovery
    // because of the technique on its timeline, which that lens cannot see.
    //
    // ⇒ the CPU admiral currently has NO recovery the search can find. That is
    // the open half of D207 and it is stated here rather than papered over,
    // because the next person to wonder where these tests went is the person who
    // should read it. The engine-level fix keeps its own guard in
    // `RecoveryLens`; what is missing is this fighter's integration arm.

    /// The admiral's kit as an AIRBORNE body sees it — the posture filter the
    /// runtime applies before the brain ever looks, so a grounded-only tilt
    /// cannot be proposed off the side of a stage.
    ///
    /// the BINDING is not what this fixture measures (a route is identified by
    /// its move id), so every candidate carries the same placeholder press.

    /// A 1600x800 stage whose only surface is far off to the right: `x` in
    /// `650..1450`, top face at `y = 500`. A body high and far to the left is
    /// ABOVE that face, so its problem is entirely lateral.

    /// THE ACCEPTANCE MEASUREMENT: the admiral's own table, the brain's own
    /// route derivation, and the real movement kernel agree that `grapple_line`
    /// is the way home — and they do it without anybody naming him.
    ///
    /// Every step is the shipped one. The kit is this file's `MovesetContract`
    /// posture-filtered the way the runtime filters it; the routes come from
    /// `lifting_candidates`, which reads nothing but `MoveFrameData`; and the
    /// verdict comes from `RecoveryLens::best_route`, which clones a body and
    /// drives `step_motion`. There is no character conditional anywhere in that
    /// chain and this test would read identically for any fighter.
    ///
    /// and the ORDER is asserted first, because the order is the trap.
    /// `air_up` sorts above `grapple_line` on the only number a static reader
    /// has. A layer that took the first candidate would take the juggle aerial.

    /// FOUR SPECIALS, FOUR MECHANISMS.
    ///
    /// four rotations of one strike would be a re-skin, so the assertion is
    /// about what each does to the ADMIRAL: one commands a retreat, one
    /// contributes to his own momentum, one commands a diagonal haul, and one
    /// commands a full stop. No two share a mechanism.
    ///
    /// and three of the four exercise a different corner of the catalog's
    /// route derivation — a negative side, an `Add` that states nothing, and a
    /// `Set` of zero — which is how this table doubles as the derivation's
    /// content-level falsifier.
    #[test]
    fn the_four_specials_are_four_different_mechanisms() {
        use ambition_platformer2d::entity_catalog::MoveEventKind;
        let set = pirate_admiral_moveset();

        // Neutral: a recoil. Commanded, and it points BACKWARD.
        let shot = commanded(&set, "grapeshot").expect("the pistol shoves its owner");
        assert!(shot.0 < 0.0 && shot.1 < 0.0);

        // Side: he DRAWS a weapon and fires it. The step is additive, so it
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
        // ⭐⭐ THE TWO HALVES OF THE MOVE, ASSERTED TOGETHER, because either one
        // alone is a different move: a draw with no shot is a taunt, and a shot
        // with no draw fires the admiral's PISTOL — his catalog row's weapon,
        // one damage, no aim assist — out of a gun-sword nobody drew.
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
        // ⛔ AND THE DRAW OUTLIVES THE SHOT. The brandish ends with the move, so
        // a fire event at or after the move's end would put the gun-sword away
        // on the same tick it went off — and the shot would leave a bare hand.
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

        // Up: a VEHICLE. It displaces nobody — the shark appears where the
        // admiral is and the climb is the player's to steer — so the mechanism
        // that makes it a recovery is the technique on its timeline, not a
        // number in its motion.
        //
        // ⛔⛔ AND THAT IS WHY IT IS UNREACHABLE TO THE CPU'S RECOVERY SEARCH,
        // which reads commanded velocity to find a way home. The two tests that
        // used to pin the admiral's search fixture went with `grapple_line`; see
        // D207 for the open question of teaching the lens about a summoned
        // ride. Asserted rather than left implicit so the day somebody gives
        // this move an impulse, they find out it was load-bearing.
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
                    if effect.key == ambition_characters::smash_ride::SUMMON_RIDE)),
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

    /// EVERY IMPORTANT MOVE IS HEARD AND SEEN.
    ///
    /// the vfx ids are checked against the SHIPPED ART — the rows of the
    /// published FX spritesheets, which is what the renderer resolves against
    /// and what `MoveSpec::presentation_problems` is handed. A typo here would
    /// be a move that plays nothing. So this asserts membership rather than
    /// spelling.
    #[test]
    fn the_specials_and_the_juggle_carry_their_own_feedback() {
        use ambition_platformer2d::entity_catalog::MoveEventKind;
        let set = pirate_admiral_moveset();
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
            // ⛔ A MOVE WITH NO VOLUMES CANNOT LAND, so it cannot land
            // silently. `call_the_shark` is the admiral's mobility special and
            // has no hurtbox anywhere on it by design; asserting a contact cue
            // on it would be asserting that it hits.
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

        // poison: the ordinary swings are NOT dressed up. A table where every
        // move flashed would satisfy the loop above and differentiate nothing.
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
    /// this is what stops a repertoire being the previous one renumbered. The
    /// claim in every module doc here is comparative (*shorter*, *slower*,
    /// *harder*), so the test has to be comparative too; pinning the admiral's
    /// numbers alone would go green on a table that had quietly become the
    /// goblin's.
    ///
    /// it also means retuning ANY of the three has to keep the ordering true or
    /// say why, which is the point: these are the characters' relationships to
    /// each other, not three independent piles of numbers.
    #[test]
    fn the_admiral_is_longer_slower_and_heavier_than_the_other_two() {
        let admiral = pirate_admiral_moveset();
        let goblin = crate::goblin_moveset::goblin_moveset();
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

    /// ⭐⭐ THE UP-B IS A WAY HOME THAT THE PLANNER CAN SEE.
    ///
    /// ⛔⛔ D250: `call_the_shark` commands no impulse, so `lift_speed` is
    /// `0.0` — and the recovery planner modelled every route as one thrown
    /// velocity, so the CPU admiral had no recovery at all. ⛔ Not fixed by
    /// fabricating a lift: this asserts the impulse is still ZERO, because a
    /// route that lied about a rise would make the search certify one.
    #[test]
    fn the_sharks_summon_advertises_seconds_of_authority_and_no_lift() {
        use ambition_platformer2d::entity_catalog::RecoveryRoute;
        let set = pirate_admiral_moveset();
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
    }
}
