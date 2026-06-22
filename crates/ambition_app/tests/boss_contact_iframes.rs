//! Flying-into-the-mockingbird contact / i-frame measurement test.
//!
//! Documents — with an explicit per-frame trace — how a player that
//! continuously steers toward a boss's center interacts with the boss's
//! body-contact damage and the post-hit invulnerability window. The
//! observed property (and the reason this test exists): because each body
//! hit grants ~0.75s of damage-invulnerability, a player can sit *inside*
//! the mockingbird's (huge: 500×185) contact box for a long time while
//! only taking damage once per i-frame window. That is the "you can stay
//! inside the collision box for a long time due to i-frames" behavior we
//! want a clear, reproducible record of (and may later want to change).
//!
//! Harness:
//!   * `SandboxSim` (fixed 60 Hz, deterministic) boots the full sim.
//!   * `SandboxSim::spawn_boss_at` drops the mockingbird at the player via
//!     the new `SpawnActorRequest` seam — no LDtk authoring required.
//!   * `SandboxSim::grant_flight` lets the player free-fly toward the boss.
//!   * Each frame steers the `ControlFrame` toward the live boss center and
//!     records pos / distance / in-box / i-frame timers / hp / contact hits.
//!
//! Run with `cargo test -p ambition_app --test boss_contact_iframes -- --nocapture`
//! to see the full movement + i-frame trace printed to stdout.

#![cfg(feature = "rl_sim")]

use ambition_app::{AgentAction, SandboxSim, TimestepMode};
use ambition_gameplay_core::actor::BossBrain;
use ambition_gameplay_core::combat::boss_clusters::BossConfig;
use ambition_gameplay_core::combat::{HitEvent, HitSource};
use ambition_gameplay_core::engine_core::{self as ae, AabbExt};
use ambition_gameplay_core::player::{
    BodyKinematics, PlayerCombatState, PlayerHealth, PrimaryPlayerOnly,
};
use bevy::ecs::message::Messages;
use bevy::prelude::World;

/// Number of frames the player chases the boss center.
const FRAMES: usize = 300;

#[derive(Clone, Copy, Debug)]
struct PlayerSnapshot {
    pos: ae::Vec2,
    half: ae::Vec2,
    invuln: f32,
    hitstun: f32,
    attacking: bool,
    hp: i32,
}

#[derive(Clone, Copy, Debug)]
struct BossSnapshot {
    pos: ae::Vec2,
    /// FULL contact-box size (profile `combat_size` for the mockingbird).
    combat_size: ae::Vec2,
}

fn read_player(world: &mut World) -> PlayerSnapshot {
    let mut q = world.query_filtered::<(
        &BodyKinematics,
        &PlayerCombatState,
        &PlayerHealth,
    ), PrimaryPlayerOnly>();
    let (kin, combat, health) = q.single(world).expect("primary player exists");
    PlayerSnapshot {
        pos: kin.pos,
        half: kin.size * 0.5,
        invuln: combat.damage_invuln_timer,
        hitstun: combat.hitstun_timer,
        attacking: combat.attacking,
        hp: health.current(),
    }
}

fn read_boss(world: &mut World) -> Option<BossSnapshot> {
    // Only the boss carries `BossConfig`, so this query never matches the
    // player even though both share `BodyKinematics`.
    let mut q = world.query::<(&BodyKinematics, &BossConfig)>();
    q.iter(world).next().map(|(kin, cfg)| BossSnapshot {
        pos: kin.pos,
        combat_size: cfg.behavior.combat_size.unwrap_or(kin.size),
    })
}

/// Boost the primary player's health so a multi-second measurement run
/// can't trip a death/respawn mid-trace (which would muddy the hp-delta
/// signal we use to detect i-frame-gated damage).
fn boost_player_health(world: &mut World, hp: i32) {
    let mut q = world.query_filtered::<&mut PlayerHealth, PrimaryPlayerOnly>();
    if let Ok(mut health) = q.single_mut(world) {
        health.health.max = hp;
        health.health.current = hp;
    }
}

/// Boss body / attack contact hits visible in the message channel this
/// frame. Soft signal (Bevy double-buffers messages across one rotation),
/// used only for the trace narrative; the authoritative damage signal is
/// the hp delta.
fn boss_contact_hits(world: &World) -> usize {
    world
        .resource::<Messages<HitEvent>>()
        .iter_current_update_messages()
        .filter(|e| matches!(e.source, HitSource::BossBody | HitSource::BossAttack))
        .count()
}

#[test]
fn flying_into_mockingbird_traces_iframe_gated_contact_damage() {
    let mut sim = SandboxSim::new_with_timestep(TimestepMode::fixed_60hz())
        .expect("sandbox sim builds");

    // Survive the whole run so hp only ever moves downward (one delta per
    // landed hit) — no respawn to confuse the trace.
    boost_player_health(sim.world_mut(), 1000);

    // Free-fly so the player can actually steer toward the boss center
    // rather than being pinned by gravity to the floor.
    sim.grant_flight();

    // Drop the mockingbird centered on the player. Its profile `combat_size`
    // (500×185) means the contact box dwarfs the spawn body, so the player
    // starts already overlapping; the boss then AirSwoops around this anchor
    // and the player chases its moving center — that chase IS the movement
    // trace. `half_size` only seeds the (small, cleanly-floating) kinematic
    // body; the contact box comes from the profile.
    let start = read_player(sim.world_mut()).pos;
    sim.spawn_boss_at(
        "test_mockingbird",
        "mockingbird",
        (start.x, start.y),
        (30.0, 30.0),
        BossBrain::PhaseScript {
            script_id: "mockingbird".to_string(),
        },
    );

    println!(
        "frame |   player (x,y)    |    boss (x,y)     |  dist | box | invuln | hitstn | atk | hp  | hit"
    );
    println!(
        "------+-------------------+-------------------+-------+-----+--------+--------+-----+-----+----"
    );

    // Trace + measurement accumulators.
    let mut overlap_frames = 0usize;
    let mut damage_events = 0usize; // distinct frames where hp dropped
    let mut total_hp_lost = 0i32;
    let mut max_gap_between_damage = 0usize; // frames between consecutive hits
    let mut max_iframe_overlap_streak = 0usize; // in-box, invuln>0, no damage
    let mut cur_gap = 0usize;
    let mut cur_iframe_streak = 0usize;
    // The core invariant: damage only lands while (effectively) vulnerable.
    let mut max_invuln_at_damage = 0.0f32;
    let mut had_first_hit = false;

    let mut prev = read_player(sim.world_mut());

    for frame in 0..FRAMES {
        let boss = read_boss(sim.world_mut()).expect("boss spawned");

        // Steer toward the live boss center. Sim world space is +Y-down, and
        // the AgentAction move axes map straight onto the flight stick, so the
        // sign of the delta points the player at the center. Idle each axis
        // once within a few px so the player settles at the center instead of
        // jittering across it.
        let to_center = boss.pos - prev.pos;
        let axis = |d: f32| if d.abs() > 3.0 { d.signum() } else { 0.0 };
        let action = AgentAction {
            move_x: axis(to_center.x),
            move_y: axis(to_center.y),
            ..AgentAction::default()
        };

        // The invuln value entering this frame (after last frame's decay) —
        // the gate the boss-damage check reads before this frame decays it.
        let invuln_before = prev.invuln;

        sim.step(action);

        let hits = boss_contact_hits(sim.world());
        let cur = read_player(sim.world_mut());

        let player_box = ae::Aabb::new(cur.pos, cur.half);
        let boss_box = ae::Aabb::new(boss.pos, boss.combat_size * 0.5);
        let in_box = boss_box.strict_intersects(player_box);
        let dist = (boss.pos - cur.pos).length();
        let hp_dropped = cur.hp < prev.hp;

        if in_box {
            overlap_frames += 1;
        }
        if hp_dropped {
            damage_events += 1;
            total_hp_lost += prev.hp - cur.hp;
            // Damage must only land when the player was (within one frame's
            // decay of) vulnerable — never deep inside the i-frame window.
            max_invuln_at_damage = max_invuln_at_damage.max(invuln_before);
            if had_first_hit {
                max_gap_between_damage = max_gap_between_damage.max(cur_gap);
            }
            had_first_hit = true;
            cur_gap = 0;
            cur_iframe_streak = 0;
        } else {
            cur_gap += 1;
            // A frame where the player sits in the box, is invulnerable, and
            // takes no damage — the literal "stuck inside, i-frames eat it".
            if in_box && cur.invuln > 0.0 {
                cur_iframe_streak += 1;
                max_iframe_overlap_streak = max_iframe_overlap_streak.max(cur_iframe_streak);
            } else {
                cur_iframe_streak = 0;
            }
        }

        // Print every 4th frame plus every frame a hit lands, to keep the
        // trace readable while never hiding a damage event.
        if frame % 4 == 0 || hp_dropped {
            println!(
                "{:5} | ({:7.1},{:7.1}) | ({:7.1},{:7.1}) | {:5.0} | {:^3} | {:6.3} | {:6.3} | {:^3} | {:3} | {}",
                frame,
                cur.pos.x,
                cur.pos.y,
                boss.pos.x,
                boss.pos.y,
                dist,
                if in_box { "in" } else { "out" },
                cur.invuln,
                cur.hitstun,
                if cur.attacking { "y" } else { "." },
                cur.hp,
                if hp_dropped {
                    format!("DMG -{} (hits={})", prev.hp - cur.hp, hits)
                } else if hits > 0 {
                    format!("(hits={}, blocked by i-frames)", hits)
                } else {
                    String::new()
                },
            );
        }

        prev = cur;
    }

    println!("\n--- summary ---");
    println!("overlap frames inside contact box : {overlap_frames} / {FRAMES}");
    println!("distinct damage events            : {damage_events}");
    println!("total hp lost                     : {total_hp_lost}");
    println!("max frames between hits (i-frames): {max_gap_between_damage}");
    println!("longest in-box invuln no-dmg streak: {max_iframe_overlap_streak}");
    println!("max invuln remaining when hit      : {max_invuln_at_damage:.3}");

    // --- Assertions: pin the i-frame behavior the trace documents. ---

    // The player really does spend most of the run inside the contact box.
    assert!(
        overlap_frames >= 120,
        "expected the player to spend most of the {FRAMES}-frame run inside the \
         mockingbird's contact box, got {overlap_frames}"
    );

    // Damage is taken more than once, proving i-frames expire and re-arm
    // (not a single hit, not a continuous drain).
    assert!(
        damage_events >= 2,
        "expected >= 2 distinct contact-damage events over the run, got {damage_events}"
    );

    // The defining property: despite ~{overlap} frames inside the box, the
    // ~0.75s (~45-frame) i-frame window means there is a long stretch where
    // the player sits in the box, invulnerable, taking no damage.
    assert!(
        max_iframe_overlap_streak >= 25,
        "expected a sustained (>=25 frame) window of sitting in the contact box \
         while invulnerable and taking no damage, got {max_iframe_overlap_streak}"
    );

    // Damage is heavily rate-limited by i-frames relative to overlap: far
    // less than one-point-per-overlap-frame. This is the quantitative form of
    // \"you can stay inside the box for a long time\".
    assert!(
        total_hp_lost <= (overlap_frames as i32) / 5,
        "i-frames should rate-limit contact damage well below 1/overlap-frame: \
         lost {total_hp_lost} hp over {overlap_frames} overlap frames"
    );

    // The invariant behind it all: damage never lands while the player is
    // meaningfully inside the i-frame window. A hit can only land on a frame
    // whose entering invuln had already decayed to ~0 (<= one frame of dt).
    assert!(
        max_invuln_at_damage <= 0.05,
        "contact damage landed while the player still had {max_invuln_at_damage:.3}s of \
         i-frames remaining — i-frames are supposed to gate it"
    );
}
