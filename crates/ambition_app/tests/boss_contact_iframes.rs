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
use ambition_characters::actor::{BodyCombat, BodyHealth};
use ambition_engine_core::{self as ae, AabbExt};
use ambition_entity_catalog::placements::BossBrain;
use ambition_gameplay_core::actor::{BodyKinematics, PrimaryPlayerOnly};
use ambition_gameplay_core::boss_encounter::{BossEncounterPhase, EncounterDef, EncounterProgress};
use ambition_gameplay_core::combat::{HitEvent, HitSource};
use ambition_gameplay_core::features::ecs::boss_clusters::BossConfig;
use bevy::ecs::message::Messages;
use bevy::prelude::World;

/// Number of frames the player chases the boss center.
const FRAMES: usize = 300;

#[derive(Clone, Copy, Debug)]
struct PlayerSnapshot {
    pos: ae::Vec2,
    vel: ae::Vec2,
    half: ae::Vec2,
    invuln: f32,
    hitstun: f32,
    /// The brief hard control-lock at the front of a knockback.
    recoil: f32,
    attacking: bool,
    hp: i32,
}

#[derive(Clone, Copy, Debug)]
struct BossSnapshot {
    pos: ae::Vec2,
    /// FULL contact-box size (profile `combat_size` for the mockingbird).
    combat_size: ae::Vec2,
    /// Hit-flash, set to 0.18 whenever a player slash SPATIALLY connects with a
    /// boss damageable part — before the encounter HP check. So `hit_flash > 0`
    /// is a clean "the swing reached the boss" signal independent of whether
    /// the boss's HP/death accounting accepted the hit this frame (it rejects
    /// damage during the ~2s Intro invulnerability a freshly-spawned boss
    /// starts in).
    hit_flash: f32,
}

fn read_player(world: &mut World) -> PlayerSnapshot {
    let mut q =
        world.query_filtered::<(&BodyKinematics, &BodyCombat, &BodyHealth), PrimaryPlayerOnly>();
    let (kin, combat, health) = q.single(world).expect("primary player exists");
    PlayerSnapshot {
        pos: kin.pos,
        vel: kin.vel,
        half: kin.size * 0.5,
        invuln: combat.damage_invuln_timer,
        hitstun: combat.hitstun_timer,
        recoil: combat.recoil_lock_timer,
        attacking: combat.attacking,
        hp: health.current(),
    }
}

fn read_boss(world: &mut World) -> Option<BossSnapshot> {
    // Only the boss carries `BossConfig`, so this query never matches the
    // player even though both share `BodyKinematics`. hit_flash lives on the
    // boss's shared `BodyCombat` (§A1).
    let mut q = world.query::<(
        &BodyKinematics,
        &BossConfig,
        &ambition_characters::actor::BodyCombat,
    )>();
    q.iter(world).next().map(|(kin, cfg, combat)| BossSnapshot {
        pos: kin.pos,
        combat_size: cfg.behavior.combat_size.unwrap_or(kin.size),
        hit_flash: combat.hit_flash,
    })
}

/// Boost the primary player's health so a multi-second measurement run
/// can't trip a death/respawn mid-trace (which would muddy the hp-delta
/// signal we use to detect i-frame-gated damage).
fn boost_player_health(world: &mut World, hp: i32) {
    let mut q = world.query_filtered::<&mut BodyHealth, PrimaryPlayerOnly>();
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
    let mut sim =
        SandboxSim::new_with_timestep(TimestepMode::fixed_60hz()).expect("sandbox sim builds");

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

/// The Hollow-Knight fix: face-tanking should not be helpless.
///
/// Same setup as the i-frame trace, but the player *swings every frame* while
/// inside the boss. Pins the two behaviors we just added:
///   1. The player can SWING (and connect on the boss) while standing in it and
///      flashing — the melee gate is now the brief recoil lock, not the full
///      hitstun window. Before the fix `attacking` could only be true after
///      hitstun (~0.94s) cleared, which outlasts the i-frame window, so you
///      could never swing while invulnerable. That helplessness is what "felt
///      the most bad".
///   2. Each contact hit engages a short, hard recoil control-lock during which
///      the player cannot steer back toward the boss (the knockback gets to
///      eject them before they can act).
///
/// Note: `update_boss_encounters` DOES auto-register/wake a programmatically-
/// spawned boss (by its `behavior.id`), but its HP only starts dropping after
/// the ~2s Intro invulnerability, and a stray room-edge can trigger a feature
/// reset that despawns it — both orthogonal to the combat-feel fix. So we
/// measure the swing CONNECTING (`hit_flash`, set before the HP check) rather
/// than boss HP, and break cleanly if the boss is reset away.
#[test]
fn face_tanking_player_swings_back_and_is_recoil_locked() {
    let mut sim =
        SandboxSim::new_with_timestep(TimestepMode::fixed_60hz()).expect("sandbox sim builds");

    boost_player_health(sim.world_mut(), 1000);
    sim.grant_flight();

    let start = read_player(sim.world_mut()).pos;
    sim.spawn_boss_at(
        "test_mockingbird",
        "mockingbird",
        (start.x, start.y),
        (30.0, 30.0),
        // Stationary (Dormant) boss. The mechanic under measurement — post-contact
        // i-frames + the player swinging back while invulnerable — is independent of
        // the boss's flight pattern, and a fixed target makes the "swing reaches the
        // boss" check deterministic instead of hostage to a chaotic flying pursuit
        // (the crude steer-at-center agent can't reliably re-catch a fleeing
        // PhaseScript mockingbird, and which basin it lands in is sensitive to
        // sub-pixel perturbations like the unified tick's corrected platform-
        // collision timing). The mockingbird's huge contact box still drives the
        // i-frame loop. Dormant bosses still deal body-contact damage on overlap.
        BossBrain::Dormant,
    );

    println!("frame | invuln | recoil | atk | flash | note");
    println!("------+--------+--------+-----+-------+-----");

    let mut prev = read_player(sim.world_mut());

    let mut frames_ran = 0usize;
    let mut swing_while_iframed = 0usize; // attacking AND invulnerable (the gate fix)
    let mut swing_connected = 0usize; // a player slash reached the boss (hit_flash set)
    let mut connect_while_iframed = 0usize; // ...on a frame the player was invulnerable
    let mut player_took_hits = 0usize;
    let mut recoil_frames = 0usize;
    let mut max_recoil = 0.0f32;
    // A recoil frame where the player was commanded toward the boss but had no
    // velocity authority toward it — proof the lock actually removes control.
    let mut recoil_suppressed_steer = false;

    for frame in 0..FRAMES {
        // The boss can be reset away by a stray room-edge transition (it isn't a
        // registered encounter); stop cleanly rather than panicking.
        let Some(boss) = read_boss(sim.world_mut()) else {
            println!("(boss reset away at frame {frame}; stopping)");
            break;
        };
        let to_center = boss.pos - prev.pos;
        let axis = |d: f32| if d.abs() > 3.0 { d.signum() } else { 0.0 };
        // Steer toward the boss center and TAP attack on a cadence whenever the
        // player BODY overlaps the boss's (huge) contact box — the literal
        // "face-tank" condition. Keying off the actual box (not an arbitrary
        // center-distance threshold) keeps the test robust to the exact flight
        // trajectory: a tiny perturbation that nudges the chaotic chase into a
        // different basin must not change whether the player is judged "inside the
        // boss and swinging". Tapping (not holding) matters: the engine applies a
        // small backward `slash_recoil` on every attack-pressed edge, so holding
        // attack every frame would continuously shove the player out of range —
        // an artifact of the synthetic input, not the mechanic.
        let boss_half = boss.combat_size * 0.5;
        let inside_boss_box = to_center.x.abs() < boss_half.x + prev.half.x
            && to_center.y.abs() < boss_half.y + prev.half.y;
        let action = AgentAction {
            move_x: axis(to_center.x),
            move_y: axis(to_center.y),
            attack: inside_boss_box && frame % 5 == 0,
            ..AgentAction::default()
        };

        sim.step(action);
        frames_ran += 1;

        let cur = read_player(sim.world_mut());
        let boss_after = read_boss(sim.world_mut());
        let boss_flash = boss_after.map(|b| b.hit_flash).unwrap_or(0.0);

        // Hold the independent variable — proximity — fixed. Once a knockback has
        // FULLY played out (recoil cleared) but the player has drifted off the
        // boss, snap it back onto the boss so the "swing connects while i-framed"
        // measurement isn't hostage to flight navigation through the sandbox's
        // incidental geometry (a free-flying body knocked onto a ledge ABOVE the
        // grounded boss cannot descend back to it). This NEVER overrides an active
        // recoil throw (`recoil > 0`), so the knockback / steering-suppression
        // measurement below stays a faithful read of the real mechanic.
        if let Some(b) = boss_after {
            let off = b.pos - cur.pos;
            let bh = b.combat_size * 0.5;
            let overlapping = off.x.abs() < bh.x + cur.half.x && off.y.abs() < bh.y + cur.half.y;
            if cur.recoil <= 0.0 && !overlapping {
                let mut pq = sim
                    .world_mut()
                    .query_filtered::<&mut BodyKinematics, PrimaryPlayerOnly>();
                if let Ok(mut kin) = pq.single_mut(sim.world_mut()) {
                    kin.pos = b.pos;
                }
            }
        }

        // The core fix: the player is mid-swing WHILE still invulnerable.
        if cur.attacking && cur.invuln > 0.0 {
            swing_while_iframed += 1;
        }
        // A player slash reached the boss this frame (`hit_flash` is set only by
        // the player-attacker boss-hit path), and whether the player was
        // invulnerable at the time = "face-tanked and hit it".
        if boss_flash > 0.0 {
            swing_connected += 1;
            if cur.invuln > 0.0 {
                connect_while_iframed += 1;
            }
        }
        if cur.hp < prev.hp {
            player_took_hits += 1;
        }
        if cur.recoil > 0.0 {
            recoil_frames += 1;
            max_recoil = max_recoil.max(cur.recoil);
            let to_boss = boss.pos - prev.pos;
            if to_boss.length() > 5.0 && cur.vel.dot(to_boss.normalize()) <= 0.0 {
                recoil_suppressed_steer = true;
            }
        }

        if frame % 6 == 0
            || cur.recoil > 0.0
            || boss_flash > 0.0
            || (cur.attacking && cur.invuln > 0.0)
        {
            println!(
                "{:5} | {:6.3} | {:6.3} | {:^3} | {:5.2} | {}",
                frame,
                cur.invuln,
                cur.recoil,
                if cur.attacking { "y" } else { "." },
                boss_flash,
                if cur.recoil > 0.0 {
                    "RECOIL-LOCK (no control)"
                } else if boss_flash > 0.0 && cur.invuln > 0.0 {
                    "SWING HIT BOSS while i-framed"
                } else if boss_flash > 0.0 {
                    "swing hit boss"
                } else if cur.attacking && cur.invuln > 0.0 {
                    "swinging while i-framed"
                } else {
                    ""
                },
            );
        }

        prev = cur;
    }

    println!("\n--- summary ---");
    println!("frames run before any reset          : {frames_ran}");
    println!("frames swinging WHILE i-framed       : {swing_while_iframed}");
    println!("frames a swing reached the boss      : {swing_connected}");
    println!("...while the player was i-framed      : {connect_while_iframed}");
    println!("player hits taken                    : {player_took_hits}");
    println!("recoil-lock frames                   : {recoil_frames}");
    println!("max recoil-lock remaining            : {max_recoil:.3}");
    println!("steering suppressed during recoil    : {recoil_suppressed_steer}");

    // Sanity: the run actually exercised several hit cycles.
    assert!(
        frames_ran >= 60,
        "expected the scenario to run a while before any reset, only got {frames_ran} frames"
    );
    assert!(
        player_took_hits >= 1,
        "the player must take >=1 boss hit, otherwise there is no recoil to measure"
    );

    // --- Ask 2: face-tanking is no longer helpless. ---
    // Before the fix the attack gate was the full ~0.94s hitstun, which
    // outlasts the 0.75s i-frame window — so the player could NEVER swing while
    // invulnerable. Now they can, and the swing reaches the boss while flashing.
    assert!(
        swing_while_iframed >= 1,
        "the player should be mid-swing while invulnerable (face-tanking); got {swing_while_iframed}"
    );
    assert!(
        swing_connected >= 1,
        "the player's swing should reach and hit the boss (face-tank and damage it); \
         got {swing_connected}"
    );
    assert!(
        connect_while_iframed >= 1,
        "the player's swing should hit the boss WHILE invulnerable (the literal \
         face-tank-and-hit-it case); got {connect_while_iframed}"
    );

    // --- Ask 1: the recoil control-lock is real, brief, and removes control. ---
    assert!(
        recoil_frames >= 1,
        "each contact hit should engage the recoil control-lock"
    );
    assert!(
        max_recoil <= 0.20,
        "the recoil lock should be brief (<= ~0.15s + a frame), got {max_recoil:.3}"
    );
    assert!(
        recoil_suppressed_steer,
        "during the recoil lock the player should NOT be able to steer back toward \
         the boss — the knockback should get to eject them first"
    );
}

/// Boss-encounter refactor canary: live HP/phase state is ENTITY-LOCAL
/// (`BossEncounter.health` + `BossEncounter.encounter`), not a shared global map.
///
/// Two of the SAME boss archetype must get independent HP / phase / death — the
/// property a gauntlet or twin-boss room needs. Before the refactor both linked
/// to a single `encounters["mockingbird"]` state and shared one HP pool;
/// damaging one would drain the other. After R3 the global map is gone, so this
/// reads the per-entity `BossEncounter`. See
/// `docs/planning/boss-entity-local-refactor.md`.
#[test]
fn two_same_archetype_bosses_have_independent_encounter_state() {
    use ambition_gameplay_core::features::ecs::boss_clusters::{BossConfig, BossEncounter};

    let mut sim =
        SandboxSim::new_with_timestep(TimestepMode::fixed_60hz()).expect("sandbox sim builds");

    let start = read_player(sim.world_mut()).pos;
    // Two mockingbirds, far enough apart that neither is on top of the other.
    sim.spawn_boss_at(
        "mock_a",
        "mockingbird",
        (start.x - 400.0, start.y),
        (30.0, 30.0),
        BossBrain::PhaseScript {
            script_id: "mockingbird".to_string(),
        },
    );
    sim.spawn_boss_at(
        "mock_b",
        "mockingbird",
        (start.x + 400.0, start.y),
        (30.0, 30.0),
        BossBrain::PhaseScript {
            script_id: "mockingbird".to_string(),
        },
    );
    // A few frames so `update_boss_encounters` seeds each boss's entity-local
    // encounter state from the profile.
    for _ in 0..3 {
        sim.step(AgentAction::default());
    }

    // Each boss carries its OWN entity-local state — independent objects.
    {
        let world = sim.world_mut();
        let mut q = world.query::<(&BossConfig, &BossEncounter)>();
        let mut a_seeded = false;
        let mut b_seeded = false;
        for (config, status) in q.iter(world) {
            match config.id.as_str() {
                "mock_a" => a_seeded = status.encounter.is_some(),
                "mock_b" => b_seeded = status.encounter.is_some(),
                _ => {}
            }
        }
        assert!(
            a_seeded && b_seeded,
            "both same-archetype bosses must carry independent entity-local state"
        );
    }

    // Damage only boss A (drop its HP + drive it to Phase1). Boss B must be
    // untouched — the gauntlet-correctness property.
    {
        let world = sim.world_mut();
        let mut q = world.query::<(&BossConfig, &mut BossEncounter, &mut BodyHealth)>();
        for (config, mut status, mut health) in q.iter_mut(world) {
            if config.id == "mock_a" {
                health.health.current = 5;
                if let Some(phase) = status.encounter.as_mut() {
                    phase.phase = BossEncounterPhase::Phase1;
                }
            }
        }
    }
    // A frame to prove the damage to A is NOT re-seeded/mirrored away and does
    // not leak into B.
    sim.step(AgentAction::default());

    let world = sim.world_mut();
    let mut q = world.query::<(&BossConfig, &BodyHealth)>();
    let mut a_hp = None;
    let mut b_hp = None;
    let mut b_max = None;
    for (config, health) in q.iter(world) {
        match config.id.as_str() {
            "mock_a" => a_hp = Some(health.current()),
            "mock_b" => {
                b_hp = Some(health.current());
                b_max = Some(health.max());
            }
            _ => {}
        }
    }
    let (a_hp, b_hp, b_max) = (a_hp.unwrap(), b_hp.unwrap(), b_max.unwrap());
    assert!(
        a_hp < b_hp,
        "damaging boss A must not lower boss B's HP (a={a_hp}, b={b_hp})"
    );
    assert_eq!(
        b_hp, b_max,
        "boss B should be at full HP — damage to A leaked into B (b={b_hp}/{b_max})"
    );
}

/// Boss-encounter refactor R2: the encounter is a first-class OPTIONAL entity.
///
/// A boss woken in the room is wrapped by a single-boss `EncounterDef` entity,
/// and its `EncounterProgress` is derived from the boss's entity-local state
/// (HP + the `BossPhaseState` phase copy) — NOT the global registry. The HUD is
/// a view bound to this progress. See `docs/planning/boss-entity-local-refactor.md`.
#[test]
fn woken_boss_is_wrapped_by_an_encounter_entity_with_live_progress() {
    let mut sim =
        SandboxSim::new_with_timestep(TimestepMode::fixed_60hz()).expect("sandbox sim builds");

    let start = read_player(sim.world_mut()).pos;
    sim.spawn_boss_at(
        "boss_with_encounter",
        "mockingbird",
        (start.x, start.y),
        (30.0, 30.0),
        BossBrain::PhaseScript {
            script_id: "mockingbird".to_string(),
        },
    );
    // A few frames: update_boss_encounters wakes the boss (Dormant→Intro),
    // sync_boss_encounter_entities wraps it, update_encounter_progress derives.
    for _ in 0..6 {
        sim.step(AgentAction::default());
    }

    let world = sim.world_mut();
    let mut q = world.query::<(&EncounterDef, &EncounterProgress)>();
    let (def, progress) = q
        .iter(world)
        .find(|(def, _)| def.placement_id == "boss_with_encounter")
        .expect("a woken boss must be wrapped by an encounter entity");
    assert!(def.hud, "the auto-created encounter binds the HUD");
    assert_eq!(def.members.len(), 1, "single-boss encounter has one member");
    let member = progress
        .members
        .first()
        .expect("progress derived from the boss member");
    assert_eq!(member.name, "mockingbird");
    assert!(member.max_hp > 0);
    assert_ne!(
        member.phase,
        BossEncounterPhase::Dormant,
        "a woken boss's encounter progress reports a live (non-Dormant) phase"
    );
}
