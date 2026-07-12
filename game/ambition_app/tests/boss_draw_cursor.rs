//! Regression: a boss's draw cursor reaches the render layer and ADVANCES.
//!
//! The render draws bosses from a separate `FeatureVisual` mirror entity that is
//! synced by id — it never carries the sim's `BossAnimFrame` component. So the
//! frame cursor has to cross the sim→render boundary through the by-id read-model
//! (`BossFrameIndex`), like every other presentation fact. When `animate_bosses`
//! instead required a `&BossAnimFrame` component on that mirror entity, the query
//! matched ZERO bosses and every boss froze on frame 0 while still drawing its
//! correct sheet (the "static mockingbird" report).
//!
//! This test spawns the mockingbird and pins BOTH halves the render depends on:
//!   1. the SIM cursor (`BossAnimFrame`) advances (Rest loops), and
//!   2. the READ-MODEL (`BossFrameIndex::cursor_frame`) mirrors that advancing
//!      cursor by id — the value `animate_bosses` now reads to pick the atlas cell.
//!
//! Run: `cargo test -p ambition_app --features rl_sim --test mockingbird_anim_diag -- --nocapture`

#![cfg(feature = "rl_sim")]

use ambition::actors::actor::{BodyKinematics, PrimaryPlayerOnly};
use ambition::actors::boss_encounter::sprites::BossAnimFrame;
use ambition::engine_core as ae;
use ambition::entity_catalog::placements::BossBrain;
use ambition_app::{AgentAction, SandboxSim, TimestepMode};
use bevy::prelude::World;

fn read_player_pos(world: &mut World) -> ae::Vec2 {
    let mut q = world.query_filtered::<&BodyKinematics, PrimaryPlayerOnly>();
    q.single(world).expect("primary player exists").pos
}

/// The SIM-owned cursor, straight off the boss entity's `BossAnimFrame` component.
fn read_sim_cursor(world: &mut World) -> Option<usize> {
    let mut q = world.query::<&BossAnimFrame>();
    q.iter(world).next().map(|f| f.frame)
}

/// The cursor as PUBLISHED in the by-id read-model — this is exactly what
/// `animate_bosses` mirrors into its draw-only `BossAnimator`.
fn read_published_cursor(world: &mut World, id: &str) -> Option<usize> {
    let idx = world.get_resource::<ambition::sim_view::BossFrameIndex>()?;
    idx.get(id).map(|v| v.cursor_frame)
}

#[test]
fn boss_draw_cursor_is_published_and_advances() {
    let mut sim =
        SandboxSim::new_with_timestep(TimestepMode::fixed_60hz()).expect("sandbox sim builds");

    let start = read_player_pos(sim.world_mut());
    sim.spawn_boss_at(
        "diag_mockingbird",
        "mockingbird",
        (start.x, start.y - 60.0),
        (30.0, 30.0),
        BossBrain::PhaseScript {
            script_id: "mockingbird".to_string(),
        },
    );

    let mut sim_frames_seen = std::collections::BTreeSet::new();
    let mut published_frames_seen = std::collections::BTreeSet::new();
    let mut published_ever_present = false;
    let mut disagreements = 0usize;

    for i in 0..180 {
        sim.step(AgentAction::default());
        let sim_cursor = read_sim_cursor(sim.world_mut());
        let published = read_published_cursor(sim.world_mut(), "diag_mockingbird");
        if let Some(f) = sim_cursor {
            sim_frames_seen.insert(f);
        }
        if let Some(f) = published {
            published_ever_present = true;
            published_frames_seen.insert(f);
        }
        // The rebuild reads the component in the SAME tick the driver advanced it,
        // so the published cursor must equal the component cursor exactly.
        if let (Some(a), Some(b)) = (sim_cursor, published) {
            if a != b {
                disagreements += 1;
            }
        }
        if i < 40 || i % 20 == 0 {
            eprintln!("step {i:3}: sim_cursor={sim_cursor:?} published={published:?}");
        }
    }

    eprintln!(
        "SUMMARY: sim_frames={sim_frames_seen:?} published_frames={published_frames_seen:?} disagreements={disagreements}"
    );

    assert!(
        sim_frames_seen.len() > 1,
        "sim cursor never advanced past {sim_frames_seen:?} — the boss animation driver is frozen"
    );
    assert!(
        published_ever_present,
        "BossFrameIndex never carried an entry for the boss — the render would have nothing to mirror"
    );
    assert!(
        published_frames_seen.len() > 1,
        "the PUBLISHED draw cursor never advanced past {published_frames_seen:?} — the render mirrors this, so the boss would render STATIC even though the sim cursor advances"
    );
    assert_eq!(
        disagreements, 0,
        "the published cursor diverged from the sim cursor {disagreements} times — the read-model must mirror the same-tick component value the geometry sample uses"
    );
}
