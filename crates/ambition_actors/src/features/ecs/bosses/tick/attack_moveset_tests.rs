//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod attack_moveset_tests` block (test-organization campaign, 2026-07-10).
//! Pure move: same test names + logic, now an adjacent child module (a direct
//! sibling, so `super` path depth is unchanged) with `use super::*;`.

use super::*;
use ambition_characters::brain::{BossAttackProfile, BossCapability};

fn warden_behavior() -> crate::features::bosses::BossBehaviorProfile {
    crate::features::bosses::BossBehaviorProfile::clockwork_warden()
}

/// Boss-fold slice (fable review §A1): EVERY boss strike runs through the SHARED
/// moveset. `boss_attack_moveset` builds one move per profile — a GEOMETRY strike
/// gets an Active-window hit volume (from `volumes_for_profile`), a SPECIAL gets a
/// sustain-`Effect` move — and `trigger_boss_attack_moves` starts whichever profile
/// is the boss's `active_profile`. This pins BOTH new links (geometry + special).
#[test]
fn a_boss_geometry_profile_triggers_its_hit_volume_move() {
    let cap = BossCapability {
        specials: vec![
            (BossAttackProfile::Strike("floor_slam".to_string()), 0.3), // geometry → hit-volume move
            (BossAttackProfile::Special("apple_rain".to_string()), 2.0),
        ],
    };
    let combat_size = ambition_engine_core::Vec2::new(80.0, 80.0);
    let moveset = crate::features::boss_attack_moveset(&cap, &warden_behavior(), combat_size, &[])
        .expect("a boss with strikes → a moveset");
    // BOTH profiles now author a move — geometry AND special.
    assert_eq!(
        moveset.0.moves.len(),
        2,
        "geometry + special both became moves"
    );
    let slam = moveset
        .0
        .move_by_id("floor_slam")
        .expect("the geometry profile became a hit-volume move");
    assert_eq!(slam.duration_s, 0.3);
    let active = &slam.windows[0];
    assert!(matches!(
        active.tag,
        ambition_entity_catalog::WindowTag::Active
    ));
    assert!(
        !active.volumes.is_empty(),
        "FloorSlam authors a body-local hit volume"
    );
    assert!(active.sustain_effect.is_none(), "geometry is not a sustain");
    assert!(
        moveset.0.move_by_id("apple_rain").is_some(),
        "the Special profile still became a sustain-move"
    );

    // Trigger a geometry strike: the driver's INTENT (§A1 split) names FloorSlam
    // as the active profile → the trigger starts the FloorSlam move.
    let mut app = App::new();
    app.add_systems(Update, trigger_boss_attack_moves);
    let intent = BossAttackIntent {
        active_profile: Some(BossAttackProfile::Strike("floor_slam".to_string())),
        ..Default::default()
    };
    let boss = app
        .world_mut()
        .spawn((
            FeatureSimEntity,
            intent,
            moveset,
            crate::actor::BodyKinematics {
                pos: ambition_engine_core::Vec2::ZERO,
                vel: ambition_engine_core::Vec2::ZERO,
                size: ambition_engine_core::Vec2::new(80.0, 80.0),
                facing: 1.0,
            },
        ))
        .id();
    app.update();
    let pb = app
        .world()
        .get::<crate::combat::moveset::MovePlayback>(boss)
        .expect("the active geometry profile started its moveset move");
    assert_eq!(pb.spec.id, "floor_slam");
    assert!(
        !pb.spec.windows[0].volumes.is_empty(),
        "the triggered move carries the strike hit volume"
    );
}

/// Build the (trigger → advance → project) chain the E53 flip runs, on one boss
/// whose FloorSlam move spans a 0.2s telegraph + 0.3s strike.
fn telegraph_boss_app() -> (App, Entity) {
    let cap = BossCapability {
        specials: vec![(BossAttackProfile::Strike("floor_slam".to_string()), 0.3)],
    };
    let combat_size = ambition_engine_core::Vec2::new(80.0, 80.0);
    let moveset = crate::features::boss_attack_moveset(
        &cap,
        &warden_behavior(),
        combat_size,
        &[(
            BossAttackProfile::Strike("floor_slam".to_string()),
            0.2,
            None,
        )],
    )
    .expect("a boss with a telegraphed strike → a moveset");

    let mut app = App::new();
    app.insert_resource(ambition_characters::actor::character_catalog::CharacterCatalog::empty());
    app.init_resource::<crate::combat::authored_volumes::AuthoredAttackVolumeResolver>();
    app.init_resource::<ambition_time::WorldTime>();
    {
        let mut wt = app.world_mut().resource_mut::<ambition_time::WorldTime>();
        wt.scaled_dt = 0.05;
        wt.raw_dt = 0.05;
    }
    app.add_message::<crate::combat::moveset::MoveEventMessage>();
    app.add_message::<ambition_vfx::vfx::VfxMessage>();
    app.add_systems(
        Update,
        (
            trigger_boss_attack_moves,
            crate::combat::moveset::advance_move_playback,
            project_boss_attack_state_from_move,
        )
            .chain(),
    );
    // §A1 split: the trigger reads the INTENT (telegraph edge → play the windup);
    // the projection WRITES the read-model `BossAttackState` from the live move.
    let intent = BossAttackIntent {
        telegraph_profile: Some(BossAttackProfile::Strike("floor_slam".to_string())),
        ..Default::default()
    };
    let boss = app
        .world_mut()
        .spawn((
            FeatureSimEntity,
            intent,
            BossAttackState::default(),
            moveset,
            crate::combat::components::ActorFaction::Boss,
            crate::actor::BodyKinematics {
                pos: ambition_engine_core::Vec2::ZERO,
                vel: ambition_engine_core::Vec2::ZERO,
                size: combat_size,
                facing: 1.0,
            },
        ))
        .id();
    (app, boss)
}

/// E53 Slice D: a Telegraph-step intent starts the move at its WINDUP (`t0 = 0`),
/// the projection reports `telegraph_profile` while the move is in windup, then
/// flips to `active_profile` once the move's clock reaches the strike window —
/// `BossAttackState` is DERIVED from the live move, both halves.
#[test]
fn telegraph_edge_trigger_projects_windup_then_strike() {
    let (mut app, boss) = telegraph_boss_app();

    // Frame 1: the telegraph intent starts the move at t0=0; one advance puts it
    // ~0.05s into the 0.2s windup — the projection reports a TELEGRAPH, no strike.
    app.update();
    let st = app.world().get::<BossAttackState>(boss).unwrap();
    assert_eq!(
        st.telegraph_profile,
        Some(BossAttackProfile::Strike("floor_slam".to_string()))
    );
    assert_eq!(st.active_profile, None, "windup has no live strike yet");

    // Advance past the 0.2s telegraph into the strike window: the projection now
    // reports the STRIKE, telegraph cleared, and active_elapsed folds in the
    // telegraph offset (t ≈ 0.25 > 0.2).
    for _ in 0..4 {
        app.update();
    }
    let st = app.world().get::<BossAttackState>(boss).unwrap();
    assert_eq!(
        st.active_profile,
        Some(BossAttackProfile::Strike("floor_slam".to_string()))
    );
    assert_eq!(st.telegraph_profile, None, "strike clears the telegraph");
    assert!(
        st.active_elapsed > 0.2,
        "active_elapsed folds in the telegraph offset; got {}",
        st.active_elapsed
    );
}

/// E53 Slice D: a windup the pattern ABANDONS (intent cleared — phase change /
/// suppress / rest) must NOT strike. The still-in-windup move is despawned before
/// its Active window opens, so no spurious hitbox — parity with the old
/// strike-edge trigger (which simply never started a move for an interrupted
/// telegraph).
#[test]
fn interrupted_windup_is_aborted_before_the_strike() {
    let (mut app, boss) = telegraph_boss_app();
    app.update();
    assert!(
        app.world()
            .get::<crate::combat::moveset::MovePlayback>(boss)
            .is_some(),
        "the telegraph started a move"
    );
    // The pattern abandons the windup (e.g. a phase transition cleared intent):
    // clearing the INTENT (§A1 split) is what the trigger observes to abort.
    app.world_mut()
        .get_mut::<BossAttackIntent>(boss)
        .unwrap()
        .clear();
    app.update();
    assert!(
        app.world()
            .get::<crate::combat::moveset::MovePlayback>(boss)
            .is_none(),
        "an abandoned windup is aborted before it can strike"
    );
}

/// Track-5 fold: the boss's authored `strike_speed_scale` is the MOVE's motion
/// lock — baked onto the strike's Active window as `MoveWindow::motion_scale`
/// and read back through `MoveSpec::motion_scale_at`, so body integration damps
/// the boss's steering exactly while the strike window is live. No brain-side
/// speed damping remains.
#[test]
fn the_strike_speed_throttle_is_baked_as_the_moves_motion_lock() {
    let cap = BossCapability {
        specials: vec![(BossAttackProfile::Strike("floor_slam".to_string()), 0.3)],
    };
    let behavior = warden_behavior(); // authors strike_speed_scale = 0.20
    let moveset = crate::features::boss_attack_moveset(
        &cap,
        &behavior,
        ambition_engine_core::Vec2::new(80.0, 80.0),
        &[(
            BossAttackProfile::Strike("floor_slam".to_string()),
            0.2,
            None,
        )],
    )
    .expect("a strike → a moveset");
    let slam = moveset.0.move_by_id("floor_slam").unwrap();
    let active = &slam.windows[0];
    assert!((active.motion_scale - behavior.strike_speed_scale).abs() < f32::EPSILON);
    // The per-time accessor the body integrator reads: full steering during the
    // windup, damped steering inside the strike window, full again after.
    assert_eq!(
        slam.motion_scale_at(0.1),
        1.0,
        "windup leaves steering free"
    );
    assert!(
        (slam.motion_scale_at(0.3) - behavior.strike_speed_scale).abs() < f32::EPSILON,
        "the strike window is the motion lock"
    );
    assert_eq!(slam.motion_scale_at(0.51), 1.0, "past the window");
}

/// Track-5 fold (BD3): an authored telegraph's cue/vfx are MOVE data — one-shot
/// `MoveEvent`s on the windup's rising edge, dispatched by the SAME
/// `dispatch_move_events` channel every actor move uses. A move with no authored
/// spec (or no telegraph at all) authors no events.
#[test]
fn telegraph_cue_and_vfx_bake_as_rising_edge_move_events() {
    use ambition_characters::brain::boss_pattern::TelegraphSpec;
    use ambition_entity_catalog::MoveEventKind;
    let cap = BossCapability {
        specials: vec![
            (BossAttackProfile::Strike("floor_slam".to_string()), 0.3),
            (BossAttackProfile::Strike("side_sweep".to_string()), 0.3),
        ],
    };
    let spec = TelegraphSpec {
        pose: Some("wind_up".into()),
        cue: Some("boss_windup".into()),
        vfx: Some("sparks".into()),
    };
    let moveset = crate::features::boss_attack_moveset(
        &cap,
        &warden_behavior(),
        ambition_engine_core::Vec2::new(80.0, 80.0),
        &[(
            BossAttackProfile::Strike("floor_slam".to_string()),
            0.2,
            Some(spec),
        )],
    )
    .expect("a strike → a moveset");

    let slam = moveset.0.move_by_id("floor_slam").unwrap();
    assert_eq!(slam.events.len(), 2, "cue + vfx on the telegraph edge");
    for ev in &slam.events {
        assert!(
            (ev.at_s - crate::features::bosses::TELEGRAPH_EDGE_S).abs() < f32::EPSILON,
            "anticipation fires on the windup's rising edge"
        );
        // Both events sit strictly inside the windup: a move started at the
        // strike edge (t0 = tel) never crosses them.
        assert!(ev.at_s < 0.2);
        assert!(matches!(
            ev.kind,
            MoveEventKind::Sfx { .. } | MoveEventKind::Vfx { .. }
        ));
    }

    // No authored telegraph for side_sweep → no anticipation events.
    let sweep = moveset.0.move_by_id("side_sweep").unwrap();
    assert!(sweep.events.is_empty());
}
