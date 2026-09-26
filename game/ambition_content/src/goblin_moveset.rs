//! Goblin-authored platform-fighter repertoire.
//!
//! The goblin is a short-range, fast-startup, lower-damage fighter. Moves use
//! the shared `strike` authoring shape and standard animation fallback vocabulary,
//! so missing specialized clips affect presentation rather than gameplay.

use ambition_entity_catalog::authoring::Strike;
use ambition_entity_catalog::smash_capture::{
    author_pummel, author_standing_grab, author_throw, capture_beat, grab_shell,
    CaptureAttemptParams, CaptureCues, CapturePummelParams, CaptureThrowParams,
    SmashCaptureRepertoire,
};
use ambition_entity_catalog::smash_repertoire::{
    DownSpecial, NeutralSpecial, SmashRepertoire, UpSpecial,
};
use ambition_entity_catalog::MovesetContract;

use ambition_entity_catalog::authoring::{
    committed_tail, impulse, on_contact, sfx, strike, vfx_at, wake, Wake,
};
use ambition_entity_catalog::ImpulseMode;

/// The goblin's standard platform-fighter verb map.
pub fn goblin_moveset() -> MovesetContract {
    // ── grounded ─────────────────────────────────────────────────────────────
    //
    // Faster than the robot's and weaker, which is the whole character in one
    // move: the goblin's jab is the thing it throws while walking into you.
    let jab = strike(Strike {
        id: "jab",
        clip: "jab",
        startup_s: 0.04,
        active_s: 0.05,
        recover_s: 0.12,
        offset: (22.0, 0.0),
        half_extents: (16.0, 13.0),
        damage: 2,
        knockback: 45.0,
        knockback_growth: 1.05,
        launch_dir: None,
        on_hit: None,
    });

    // An upward poke that beats a shorthop. Small volume — it is an
    // anti-air, not a wall.
    let up_tilt = strike(Strike {
        id: "tilt_up",
        clip: "attack_up",
        startup_s: 0.07,
        active_s: 0.07,
        recover_s: 0.16,
        offset: (10.0, -24.0),
        half_extents: (16.0, 18.0),
        damage: 4,
        knockback: 70.0,
        knockback_growth: 1.30,
        launch_dir: Some((0.15, -1.0)),
        on_hit: None,
    });

    // Low and forward: the goblin's ground game is knee height, which is where a
    // small body's reach actually is.
    let down_tilt = strike(Strike {
        id: "tilt_down",
        clip: "attack_down",
        startup_s: 0.06,
        active_s: 0.06,
        recover_s: 0.15,
        offset: (22.0, 12.0),
        half_extents: (18.0, 10.0),
        damage: 3,
        knockback: 50.0,
        knockback_growth: 1.15,
        launch_dir: Some((1.0, -0.25)),
        on_hit: None,
    });

    // ── smashes ──────────────────────────────────────────────────────────────
    //
    // Committed and not safe: 0.30s of recovery on a 5 HP body. Missing it is
    // how a goblin dies, which makes landing it exciting.
    let mut f_smash = strike(Strike {
        id: "smash_forward",
        clip: "smash_forward",
        startup_s: 0.28,
        active_s: 0.06,
        recover_s: 0.30,
        offset: (34.0, -2.0),
        half_extents: (24.0, 18.0),
        damage: 12,
        knockback: 135.0,
        knockback_growth: 3.79,
        launch_dir: Some((1.0, -0.40)),
        on_hit: None,
    });
    f_smash.smash_charge_mult = 1.7;

    let mut up_smash = strike(Strike {
        id: "smash_up",
        clip: "smash_up",
        startup_s: 0.24,
        active_s: 0.07,
        recover_s: 0.28,
        offset: (6.0, -30.0),
        half_extents: (20.0, 26.0),
        damage: 11,
        knockback: 140.0,
        knockback_growth: 6.59,
        launch_dir: Some((0.0, -1.0)),
        on_hit: None,
    });
    up_smash.smash_charge_mult = 1.7;

    // Both sides, low — the goblin's answer to being surrounded, and its
    // ledge-guard.
    let mut down_smash = strike(Strike {
        id: "smash_down",
        clip: "smash_down",
        startup_s: 0.22,
        active_s: 0.08,
        recover_s: 0.32,
        offset: (0.0, 14.0),
        half_extents: (32.0, 12.0),
        damage: 10,
        knockback: 125.0,
        knockback_growth: 2.55,
        launch_dir: Some((0.9, -0.55)),
        on_hit: None,
    });
    down_smash.smash_charge_mult = 1.7;

    // ── aerials ──────────────────────────────────────────────────────────────
    let n_air = strike(Strike {
        id: "air_neutral",
        clip: "air_neutral",
        startup_s: 0.05,
        active_s: 0.10,
        recover_s: 0.13,
        offset: (0.0, 0.0),
        half_extents: (22.0, 20.0),
        damage: 4,
        knockback: 60.0,
        knockback_growth: 1.25,
        launch_dir: None,
        on_hit: None,
    });

    let f_air = strike(Strike {
        id: "air_forward",
        clip: "air_forward",
        startup_s: 0.08,
        active_s: 0.07,
        recover_s: 0.16,
        offset: (26.0, -2.0),
        half_extents: (20.0, 16.0),
        damage: 6,
        knockback: 85.0,
        knockback_growth: 1.70,
        launch_dir: Some((1.0, -0.30)),
        on_hit: None,
    });

    // The goblin's best kill option, facing the wrong way: the classic trade.
    let b_air = strike(Strike {
        id: "air_back",
        clip: "air_back",
        startup_s: 0.10,
        active_s: 0.06,
        recover_s: 0.20,
        offset: (-28.0, 0.0),
        half_extents: (20.0, 16.0),
        damage: 9,
        knockback: 120.0,
        knockback_growth: 2.40,
        launch_dir: Some((-1.0, -0.35)),
        on_hit: None,
    });

    let u_air = strike(Strike {
        id: "air_up",
        clip: "air_up",
        startup_s: 0.06,
        active_s: 0.08,
        recover_s: 0.14,
        offset: (2.0, -26.0),
        half_extents: (18.0, 20.0),
        damage: 5,
        knockback: 80.0,
        knockback_growth: 1.80,
        launch_dir: Some((0.0, -1.0)),
        on_hit: None,
    });

    // Straight down and hard. No `on_hit` rebound: a goblin that could pogo
    // off a body would out-recover a character whose problem is recovery.
    let d_air = strike(Strike {
        id: "air_down",
        clip: "air_down",
        startup_s: 0.11,
        active_s: 0.07,
        recover_s: 0.22,
        offset: (4.0, 24.0),
        half_extents: (18.0, 18.0),
        damage: 8,
        knockback: 110.0,
        knockback_growth: 2.10,
        launch_dir: Some((0.0, 1.0)),
        on_hit: None,
    });

    // A forward tilt, so the most common press does not fall down the chain to
    // the jab. A scrappy shove, shorter and faster than anyone else's.
    let f_tilt = strike(Strike {
        id: "tilt_forward",
        clip: "attack_side",
        startup_s: 0.06,
        active_s: 0.06,
        recover_s: 0.14,
        offset: (24.0, -2.0),
        half_extents: (18.0, 12.0),
        damage: 4,
        knockback: 60.0,
        knockback_growth: 1.20,
        launch_dir: Some((1.0, -0.25)),
        on_hit: None,
    });
    let f_tilt = vfx_at(f_tilt, 0.06, "air_slice", (24.0, -2.0), 0.7);
    let f_tilt = sfx(f_tilt, 0.06, "enemy.goblin.attack");
    let f_tilt = on_contact(f_tilt, "enemy.goblin.hit");

    // Neutral: `scrap_flail`. No technique: it swings its whole body and hopes.
    // Wide, slow for a goblin, and the only move that covers both sides.
    let n_b = strike(Strike {
        id: "scrap_flail",
        clip: "attack",
        startup_s: 0.10,
        active_s: 0.10,
        recover_s: 0.26,
        offset: (14.0, 0.0),
        half_extents: (26.0, 20.0),
        damage: 7,
        knockback: 88.0,
        knockback_growth: 1.70,
        launch_dir: Some((0.85, -0.50)),
        on_hit: None,
    });
    let n_b = committed_tail(n_b, 0.55, 0.15);
    let n_b = vfx_at(n_b, 0.10, "air_slice", (14.0, 0.0), 1.1);
    let n_b = sfx(n_b, 0.10, "enemy.goblin.attack");
    let n_b = on_contact(n_b, "enemy.goblin.hit");

    // Side: `headlong_charge`. It runs at you. `ImpulseMode::Set`, so a falling
    // goblin gets the same charge as a standing one. The tail damps steering to
    // 0.1, not 0.0, so a scrappy fighter can still adjust.
    let side_b = strike(Strike {
        id: "headlong_charge",
        clip: "attack_side",
        startup_s: 0.14,
        active_s: 0.10,
        recover_s: 0.24,
        offset: (24.0, 2.0),
        half_extents: (22.0, 16.0),
        damage: 8,
        knockback: 100.0,
        knockback_growth: 1.90,
        launch_dir: Some((0.95, -0.35)),
        on_hit: None,
    });
    let side_b = impulse(side_b, 0.14, (560.0, 0.0), ImpulseMode::Set);
    let side_b = committed_tail(side_b, 0.60, 0.10);
    let side_b = vfx_at(side_b, 0.14, "dash_streak", (0.0, 0.0), 1.0);
    let side_b = sfx(side_b, 0.14, "enemy.goblin.attack");
    let side_b = on_contact(side_b, "enemy.goblin.hit");
    // If it connects, it does not let go: the opposite shape to the oni's flow.
    // The oni branches on a failure (`Blocked`) to escape; this waits for a
    // success (`Connected`) to commit. It needs no new node, signal or engine
    // change: the same `Wait`/`Emit` pair used the other way. The emitted grab
    // is the goblin's own, authored below.
    //
    // Wait on the connect, not the overlap. `Overlapped` is also true of a
    // blocked charge, so waiting on it would give a grab for running into a
    // shield.
    //
    // Roster decision #18, Jon's to overrule: a landed charge grabs. The captive
    // still has `grab_mash_seconds` (14.4 frames per press), so this shortens the
    // road to a throw without removing anyone's escape.
    let side_b = ambition_entity_catalog::MoveSpec {
        flow: Some(ambition_entity_catalog::TechniqueFlow {
            nodes: vec![
                // The timeout is past the active window (0.14 + 0.10) and short of the
                // tail: a charge that connected with nothing is a whiff, and whiffs are the
                // punish window.
                ambition_entity_catalog::FlowNode::Wait {
                    on: ambition_entity_catalog::FlowSignal::Connected,
                    timeout_s: 0.30,
                    then: 1,
                    on_timeout: 2,
                },
                ambition_entity_catalog::FlowNode::Emit {
                    effect: ambition_entity_catalog::EffectRef {
                        key: ambition_entity_catalog::smash_capture::CAPTURE_ATTEMPT
                            .to_string(),
                        params: ambition_entity_catalog::ParamValue::from_typed(
                            &CaptureAttemptParams {
                                // Closer than its standing grab: it is already inside you.
                                offset: (10.0, 1.0),
                                half_extents: (17.0, 14.0),
                                hold_offset: (13.0, 3.0),
                            },
                        )
                        .expect("the goblin's tackle grab params serialize"),
                    },
                    then: 2,
                },
                ambition_entity_catalog::FlowNode::Finish,
            ],
        }),
        ..side_b
    };

    // Up: `scramble_leap`, its recovery. It claws upward: weaker than a
    // heavyweight's lift and cheaper to land.
    let mut up_b = strike(Strike {
        id: "scramble_leap",
        clip: "attack_up",
        startup_s: 0.08,
        active_s: 0.12,
        recover_s: 0.18,
        offset: (0.0, -10.0),
        half_extents: (18.0, 26.0),
        damage: 6,
        knockback: 78.0,
        knockback_growth: 1.50,
        launch_dir: Some((0.10, -1.0)),
        on_hit: None,
    });
    up_b.landing_lag_s = Some(0.24);
    let up_b = impulse(up_b, 0.08, (0.0, -720.0), ImpulseMode::Set);
    let up_b = committed_tail(up_b, 0.46, 0.25);
    let up_b = vfx_at(up_b, 0.08, "landing_puff", (0.0, 16.0), 0.9);
    let up_b = sfx(up_b, 0.08, "enemy.goblin.jump");
    let up_b = on_contact(up_b, "enemy.goblin.hit");

    // DOWN — `dirt_kick`. It kicks the ground at you. Wide, low and flat,
    // and grounded-only because the whole move is that there is ground.
    let down_b = strike(Strike {
        id: "dirt_kick",
        clip: "attack_down",
        startup_s: 0.12,
        active_s: 0.08,
        recover_s: 0.28,
        offset: (18.0, 16.0),
        half_extents: (30.0, 10.0),
        damage: 6,
        knockback: 70.0,
        knockback_growth: 1.35,
        launch_dir: Some((0.70, -0.60)),
        on_hit: None,
    });
    // The dirt: the kick connects, and a target just out of range gets a face
    // full of dirt (a windbox).
    //
    // The wake is appended, so the hit wins wherever both reach. Standing in
    // both means you were kicked; being shoved is for being out of range. This
    // gives a short-ranged fighter a spacing tool.
    //
    // One-shot, not sustained: a repeating windbox is a wall; this is a shove.
    let down_b = wake(
        down_b,
        Wake {
            offset: (58.0, 14.0),
            half_extents: (24.0, 8.0),
            // Firm enough to break an approach or push someone off a ledge, well short
            // of a killing launch.
            push: 62.0,
            // Along the floor and barely up: dirt travels, it does not lift.
            push_dir: (1.0, -0.15),
            repeating: false,
        },
    );
    let down_b = committed_tail(down_b, 0.55, 0.0);
    let down_b = vfx_at(down_b, 0.12, "sand_burst", (18.0, 16.0), 1.0);
    let down_b = sfx(down_b, 0.12, "enemy.goblin.attack");
    let down_b = on_contact(down_b, "enemy.goblin.hit");

    // Down-B has two forms, like Bowser's: a slam in the air, an arc and slam
    // on the ground. Context-dependent specials are acceptable, though most
    // should not be.
    //
    // A special gated to one posture is not answered in the other: the
    // directional chain falls through to the neutral special.
    // `special_air_down` comes before `special_down` in that chain.
    // Down, in the air: `dive_stomp`. It cannot kick the ground from up there,
    // so it becomes the ground: knees up, straight down.
    let mut air_down_b = strike(Strike {
        id: "dive_stomp",
        clip: "air_down",
        startup_s: 0.08,
        active_s: 0.10,
        recover_s: 0.22,
        offset: (0.0, 24.0),
        half_extents: (18.0, 20.0),
        damage: 7,
        knockback: 84.0,
        knockback_growth: 1.60,
        launch_dir: Some((0.0, 1.0)),
        on_hit: None,
    });
    air_down_b.landing_lag_s = Some(0.22);
    let air_down_b = impulse(air_down_b, 0.08, (0.0, 1150.0), ImpulseMode::Set);
    let air_down_b = vfx_at(air_down_b, 0.08, "sand_burst", (0.0, 20.0), 0.9);
    let air_down_b = sfx(air_down_b, 0.08, "enemy.goblin.attack");
    // This is the goblin's Limit. Jon: *"give whoever gets the limit meter some
    // move they can use when it fills."*
    //
    // The price is the whole meter, so no new gate is needed: a move costing
    // exactly the cap is available exactly when the meter is full, and
    // `afford_meter` refuses it otherwise.
    //
    // An uncharged press gets the ordinary dive through `MoveGates::when_refused`
    // (the meter check is at acceptance, not in `MoveGates::permits`, because a
    // data crate must not read body state). The fallback is this move's ordinary
    // form, so a player who never fills the meter has the same goblin.
    //
    // It is the air-down, not the neutral: the other specials stay free, and
    // `SmashRepertoire` binds no air-neutral verb.
    //
    // Roster decision #21, Jon's to overrule. 60 is his whole baseline cap, and
    // the payoff matches: a dive that ends a stock.
    // Clone before the price and the buff are applied; otherwise the fallback
    // would be the 26-damage version.
    let uncharged_dive = {
        let mut spec = air_down_b.clone();
        spec.id = format!("{}_uncharged", spec.id);
        spec
    };
    let uncharged_dive = on_contact(uncharged_dive, "enemy.goblin.hit");
    let air_down_b = ambition_entity_catalog::MoveSpec {
        gates: ambition_entity_catalog::MoveGates {
            // The Limit, by name. A goblin with no Limit (outside a Limit match) cannot
            // pay and gets the uncharged dive.
            costs: vec![ambition_resource_spec::ResourceCost::new(
                ambition_entity_catalog::smash_limit::LIMIT,
                60.0,
            )],
            // Bound to no verb. `move_by_id` searches every move in the contract, so
            // the fallback only needs an id and a place in `moves`.
            when_refused: Some(uncharged_dive.id.clone()),
            ..air_down_b.gates.clone()
        },
        ..air_down_b
    };
    let air_down_b = {
        let mut spec = air_down_b;
        // The Limit's payoff is the same strike made enormous, so everything a
        // player knows about this dive stays true.
        for window in &mut spec.windows {
            for volume in &mut window.volumes {
                volume.damage = 26;
                volume.knockback = 210.0;
            }
        }
        spec
    };
    let air_down_b = on_contact(air_down_b, "enemy.goblin.hit");

    // Goblin's capture kit: short reach, fast everything, weak throw. The
    // flattest launch on the roster.
    // The grab draws `attack`, not `grab`: these sheets publish no `grab` row,
    // and each table's clip guard refuses unpublished rows.
    let grab = author_standing_grab(
        grab_shell("goblin_grab", "attack", 0.06, 0.04, 0.22),
        CaptureAttemptParams {
            offset: (12.0, 1.0),
            half_extents: (17.0, 14.0),
            hold_offset: (13.0, 3.0),
        },
    );
    let pummel = author_pummel(
        capture_beat("goblin_pummel", "attack", 0.14),
        0.06,
        CapturePummelParams { damage: 2 },
    );
    let forward_throw = author_throw(
        capture_beat("goblin_fthrow", "attack", 0.23),
        0.12,
        CaptureThrowParams {
            damage: 6,
            knockback: 100.0,
            knockback_growth: 2.2,
            launch_dir: (1.0, -0.35),
        },
    );

    let back_throw = author_throw(
        capture_beat("goblin_bthrow", "attack", 0.25),
        0.13,
        CaptureThrowParams {
            damage: 7,
            knockback: 108.0,
            knockback_growth: 2.31,
            launch_dir: (-1.0, -0.25),
        },
    );

    let up_throw = author_throw(
        capture_beat("goblin_uthrow", "attack", 0.24),
        0.12,
        CaptureThrowParams {
            damage: 6,
            knockback: 104.0,
            knockback_growth: 2.24,
            launch_dir: (0.0, -1.0),
        },
    );

    // The cargo carry: Down + Attack inside a grab hoists the captive onto its
    // back and lets the goblin walk.
    //
    // It replaces the weakest throw (`damage: 4, knockback: 74`, generic
    // fields); every fighter authors all four throws, so there was no empty
    // slot.
    //
    // The other three throws are the exit: while carrying, forward/back/up +
    // Attack still throw, because a carry is a hold with two terms changed.
    //
    // No damage on the carry: a carry that also chipped would be strictly
    // better than the throw it replaced.
    let down_throw = ambition_entity_catalog::smash_capture::author_carry(
        capture_beat("goblin_dthrow", "attack", 0.26),
        0.13,
        ambition_entity_catalog::smash_capture::CaptureCarryParams {
            // Over the shoulder. `+y` is down, so negative lifts; slightly forward so
            // the goblin is not wearing them.
            hold_offset: (6.0, -18.0),
        },
    );
    let mut contract = SmashRepertoire {
        taunt: ambition_entity_catalog::authoring::taunt("goblin_taunt", 0.9),
        dash_attack: ambition_entity_catalog::authoring::dash_attack(
            "goblin_dash_attack",
            ambition_entity_catalog::authoring::DashAttackShape::GENRE,
            6,
            75.0,
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
        neutral_special: NeutralSpecial::Authored(n_b),
        side_special: side_b,
        up_special: UpSpecial::Standard(up_b),
        // Every smash fighter has a grab. The values are per character on purpose.
        capture: SmashCaptureRepertoire {
            cues: CaptureCues::GENERIC,
            grab,
            pummel,
            forward_throw,
            back_throw: Some(back_throw),
            up_throw: Some(up_throw),
            down_throw: Some(down_throw),
        },
        down_special: DownSpecial::ByPosture {
            grounded: down_b,
            airborne: air_down_b,
        },
    }
    .into_contract();
    // The uncharged dive is reachable only through the Limit dive's
    // `when_refused`, so it joins the moves without claiming a verb.
    contract.moves.push(uncharged_dive);
    contract
}

#[cfg(test)]
mod dirt_tests {

    /// The kick wins where both reach, and that order is the move.
    ///
    /// The strike seam takes the first authored volume that reaches, so the
    /// damaging volume is first: standing in both means you were kicked. A wake
    /// ranked first would shove the people the move was about to hit. This
    /// asserts the order and each side's damage.
    #[test]
    fn the_dirt_kick_hits_what_it_reaches_and_shoves_what_it_misses() {
        use ambition_entity_catalog::{VolumeReaction, WindowTag};
        let kick = crate::authored_movesets::shipped("goblin")
            .move_by_id("dirt_kick")
            .expect("dirt_kick exists")
            .clone();
        let window = kick
            .windows
            .iter()
            .find(|w| w.tag == WindowTag::Active && !w.volumes.is_empty())
            .expect("it still strikes");
        assert_eq!(window.volumes.len(), 2, "a kick and its dust");
        assert!(window.volumes[0].damage > 0, "the KICK is ranked first");
        assert_eq!(window.volumes[1].damage, 0, "dirt does not wound");
        assert!(
            matches!(
                window.volumes[1].reaction,
                Some(VolumeReaction::Windbox(_))
            ),
            "the dust must be a push, not a weak second hitbox"
        );
        assert!(
            window.volumes[1].shape.leading_edge_x()
                > window.volumes[0].shape.leading_edge_x(),
            "the dust travels BEYOND the boot"
        );
    }

    /// The brain is told where the boot is, not where the dust is.
    ///
    /// `MoveFrameData::reach`/`coverage` are all a fighter brain knows about where
    /// a move lands. As the union of every Active volume, they would report this
    /// move reaching to the dust, and a goblin would press `dirt_kick` where only
    /// the shove lands. Here the boot ends at 48 and the dust at 82.
    #[test]
    fn the_brains_reach_for_the_dirt_kick_is_the_boot_and_not_the_dust() {
        let frames = crate::authored_movesets::shipped("goblin")
            .move_by_id("dirt_kick")
            .expect("dirt_kick exists")
            .frame_data();
        // No `expect` above the claim, so a missing region fails at the assertion
        // that names it.
        let hit = frames.coverage.map(|c| c.max.0);
        let push = frames.push_coverage.map(|c| c.max.0);
        assert_eq!(frames.reach, 48.0, "offset 18 + half-extent 30 — the boot");
        assert_eq!(hit, Some(48.0), "the hittable region ends at the boot");
        assert_eq!(push, Some(82.0), "offset 58 + half-extent 24 — the dust");
        assert!(push > hit, "the whole point of a wake: {push:?} vs {hit:?}");
    }
}

#[cfg(test)]
mod tests {
    /// The Limit costs the whole meter, so it is usable exactly when full with no
    /// new gate.
    ///
    /// The equality is the assertion. Below the cap it could be used twice at
    /// 50% (a resource move, not a Limit); above it, never, because the meter
    /// stops at the cap. Both would fail silently.
    #[test]
    fn the_goblins_limit_dive_costs_exactly_the_matchs_full_meter() {
        use ambition_entity_catalog::smash_limit::LimitMeterFill;
        let set = crate::authored_movesets::shipped("goblin");
        let dive = set
            .moves
            .iter()
            .find(|m| m.id == "dive_stomp")
            .expect("its air down-B is in the table");
        let cap = LimitMeterFill::JONS_BASELINE.cap;
        assert_eq!(
            dive.gates.costs,
            vec![ambition_resource_spec::ResourceCost::new(
                ambition_entity_catalog::smash_limit::LIMIT,
                cap,
            )],
            "the Limit dive must cost exactly the cap of {cap}, in Limit — below \
             the cap it is a resource move you can use twice, above it is a move \
             nobody can ever afford, and the meter stops filling at the cap \
             either way",
        );

        // It must hit harder than the move it replaces.
        let hardest = dive
            .windows
            .iter()
            .flat_map(|w| w.volumes.iter())
            .map(|v| v.damage)
            .max()
            .expect("the dive has a volume");
        assert!(
            hardest > 20,
            "the Limit dive's hardest volume does {hardest} — the whole meter \
             bought a poke"
        );

        // Every other special must stay free.
        let priced: Vec<&str> = set
            .moves
            .iter()
            .filter(|m| !m.gates.costs.is_empty())
            .map(|m| m.id.as_str())
            .collect();
        assert_eq!(
            priced,
            vec!["dive_stomp"],
            "more than one goblin move costs meter ({priced:?}) — an uncharged \
             goblin would be missing part of its kit rather than one button"
        );
    }

    /// The goblin's flow has the opposite shape to the oni's: it waits for a
    /// success (`Connected`) to commit, where the oni branches on a failure
    /// (`Blocked`) to escape. The same `Wait`/`Emit` pair needed no new node,
    /// signal or engine change.
    ///
    /// It must wait on `Connected`, not `Overlapped`: an overlap is also true of a
    /// blocked charge.
    #[test]
    fn the_goblins_charge_grabs_on_a_connect_and_not_on_a_mere_overlap() {
        use ambition_entity_catalog::{FlowNode, FlowSignal};
        let set = crate::authored_movesets::shipped("goblin");
        let charge = set
            .moves
            .iter()
            .find(|m| m.id == "headlong_charge")
            .expect("its side-B is in the table");
        let flow = charge
            .flow
            .as_ref()
            .expect("the charge authors no flow, so it bounces off and stands there");
        assert!(
            flow.problems().is_empty(),
            "the goblin's flow does not validate: {:?}",
            flow.problems()
        );

        let waited_on = flow.nodes.iter().find_map(|n| match n {
            FlowNode::Wait { on, .. } => Some(*on),
            _ => None,
        });
        assert_eq!(
            waited_on,
            Some(FlowSignal::Connected),
            "the tackle waits on {waited_on:?} — an `Overlapped` charge includes \
             one a guard ate, so the goblin would be rewarded for running into a \
             shield"
        );

        // The follow-up is its own grab, not a new technique.
        let emitted = flow.nodes.iter().find_map(|n| match n {
            FlowNode::Emit { effect, .. } => Some(effect.key.clone()),
            _ => None,
        });
        assert_eq!(
            emitted.as_deref(),
            Some(ambition_entity_catalog::smash_capture::CAPTURE_ATTEMPT),
            "the charge follows up with {emitted:?} rather than the capture the \
             goblin already authors"
        );

        // The whiff stays punishable: the wait gives up inside the move.
        let timeout = flow.nodes.iter().find_map(|n| match n {
            FlowNode::Wait { timeout_s, .. } => Some(*timeout_s),
            _ => None,
        });
        let active_ends = 0.14 + 0.10;
        assert!(
            timeout.is_some_and(|t| t > active_ends && t < charge.duration_s),
            "the wait times out at {timeout:?}, outside ({active_ends}, {}) — a \
             charge that hit nothing must not still be waiting to grab",
            charge.duration_s
        );
    }

    use super::*;

    // Verb binding is checked by construction: `SmashRepertoire` owns the verb
    // strings and is a struct with no `Default`, so a missing slot is a compile
    // error. Coverage in every posture is checked by
    // `ambition_entity_catalog::smash_repertoire` and by
    // `smash_roster_movesets::report_the_smash_kit_every_selectable_fighter_has`.

    /// The goblin is not the robot with different numbers.
    ///
    /// A copied table would pass every other test here. This pins the module
    /// doc's identity: shorter reach, faster jab, weaker kill.
    #[test]
    fn the_goblin_is_shorter_faster_and_weaker_than_the_robot() {
        let goblin = crate::authored_movesets::shipped("goblin");
        let robot = crate::player_robot_moveset::player_robot_moveset();
        let find = |set: &MovesetContract, id: &str| {
            set.moves
                .iter()
                .find(|m| m.id == id)
                .unwrap_or_else(|| panic!("{id} exists"))
                .clone()
        };

        let (g_jab, r_jab) = (find(&goblin, "jab"), find(&robot, "jab"));
        let startup = |m: &ambition_entity_catalog::MoveSpec| {
            m.windows
                .iter()
                .find(|w| {
                    matches!(
                        w.tag,
                        ambition_entity_catalog::WindowTag::Active
                    )
                })
                .expect("a strike has an active window")
                .start_s
        };
        assert!(
            startup(&g_jab) < startup(&r_jab),
            "the goblin's jab comes out faster"
        );

        let reach = |m: &ambition_entity_catalog::MoveSpec| {
            m.windows
                .iter()
                .flat_map(|w| w.volumes.iter())
                .map(|v| match v.shape {
                    ambition_entity_catalog::VolumeShape::Rect {
                        offset,
                        half_extents,
                    } => offset.0.abs() + half_extents.0,
                    _ => 0.0,
                })
                .fold(0.0f32, f32::max)
        };
        assert!(
            reach(&g_jab) < reach(&r_jab),
            "and it reaches less far, which is what makes it have to get close"
        );

        let damage = |m: &ambition_entity_catalog::MoveSpec| {
            m.windows
                .iter()
                .flat_map(|w| w.volumes.iter())
                .map(|v| v.damage)
                .max()
                .unwrap_or(0)
        };
        assert!(
            damage(&find(&goblin, "smash_forward")) < damage(&find(&robot, "smash_forward")),
            "and its kill move hits softer — a small fighter trades reach and \
             power for speed, or it is just the robot in a different sheet"
        );
    }

    /// A carry, not a throw, and not both. Leaving `author_throw` beside
    /// `author_carry` would make down both hoist and launch, which plays as a
    /// carry that randomly fails.
    #[test]
    fn the_goblins_down_throw_hauls_instead_of_launching() {
        let moves = crate::authored_movesets::shipped("goblin");
        let beat = moves
            .moves
            .iter()
            .find(|m| m.id == "goblin_dthrow")
            .expect("the goblin has a down-throw slot");
        let keys: Vec<&str> = beat
            .events
            .iter()
            .filter_map(|event| match &event.kind {
                ambition_entity_catalog::MoveEventKind::Effect(effect) => {
                    Some(effect.key.as_str())
                }
                _ => None,
            })
            .collect();
        assert!(
            keys.contains(&ambition_entity_catalog::smash_capture::CAPTURE_CARRY),
            "the goblin's down press does not take the weight: {keys:?}"
        );
        assert!(
            !keys.contains(&ambition_entity_catalog::smash_capture::CAPTURE_THROW),
            "the goblin's down press both hauls AND throws: {keys:?}"
        );
    }
}
