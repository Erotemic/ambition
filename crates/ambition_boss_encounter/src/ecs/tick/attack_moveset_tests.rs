use super::*;
// The prelude wholesale: this fixture builds an `App` and chains systems, which
// is the prelude's own vocabulary rather than a dependency of the boss module.
use crate::behavior::BossBehaviorProfileExt;
use ambition_characters::brain::{BossAttackProfile, BossCapability};
use bevy::prelude::*;

fn warden_behavior() -> crate::pattern::profile::BossBehaviorProfile {
    crate::pattern::profile::BossBehaviorProfile::clockwork_warden()
}

/// The same two-profile boss the geometry test builds, as a value.
///
/// Shared with the shipped moveset builder, so a test cannot pass while the
/// shipped shape changes.
fn boss_moveset_for_test() -> ambition_combat::moveset::ActorMoveset {
    let cap = BossCapability {
        specials: vec![
            (BossAttackProfile::Strike("floor_slam".to_string()), 0.3),
            (BossAttackProfile::Special("apple_rain".to_string()), 2.0),
        ],
    };
    crate::attack_moveset::boss_attack_moveset(
        &cap,
        &warden_behavior(),
        ambition_platformer2d_core::Vec2::new(80.0, 80.0),
        &[],
    )
    .expect("a boss with strikes -> a moveset")
}

/// Every boss strike runs through the shared moveset. `boss_attack_moveset`
/// builds one move per profile (a geometry strike gets an Active-window hit
/// volume from `volumes_for_profile`; a special gets a sustain-`Effect` move),
/// and `trigger_boss_attack_moves` starts whichever profile is the boss's
/// `active_profile`. This tests both kinds.
#[test]
fn a_boss_geometry_profile_triggers_its_hit_volume_move() {
    let cap = BossCapability {
        specials: vec![
            (BossAttackProfile::Strike("floor_slam".to_string()), 0.3), // geometry → hit-volume move
            (BossAttackProfile::Special("apple_rain".to_string()), 2.0),
        ],
    };
    let combat_size = ambition_platformer2d_core::Vec2::new(80.0, 80.0);
    let moveset =
        crate::attack_moveset::boss_attack_moveset(&cap, &warden_behavior(), combat_size, &[])
            .expect("a boss with strikes → a moveset");
    // both profiles now author a move — geometry and special.
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

    // Trigger a geometry strike: the driver's intent (§A1 split) names FloorSlam
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
            ambition_platformer2d_core::BodyKinematics {
                pos: ambition_platformer2d_core::Vec2::ZERO,
                vel: ambition_platformer2d_core::Vec2::ZERO,
                size: ambition_platformer2d_core::Vec2::new(80.0, 80.0),
                facing: 1.0,
            },
        ))
        .id();
    app.update();
    let pb = app
        .world()
        .get::<ambition_combat::moveset::MovePlayback>(boss)
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
    let combat_size = ambition_platformer2d_core::Vec2::new(80.0, 80.0);
    let moveset = crate::attack_moveset::boss_attack_moveset(
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
    app.init_resource::<ambition_sprite_sheet::character::sheets::AuthoredSheets>();
    app.init_resource::<ambition_combat::authored_volumes::AuthoredAttackVolumeResolver>();
    app.init_resource::<ambition_time::WorldTime>();
    {
        let mut wt = app.world_mut().resource_mut::<ambition_time::WorldTime>();
        wt.scaled_dt = 0.05;
        wt.raw_dt = 0.05;
    }
    app.add_message::<ambition_combat::moveset::MoveEventMessage>();
    app.add_message::<ambition_vfx::vfx::VfxMessage>();
    app.add_systems(
        Update,
        (
            trigger_boss_attack_moves,
            ambition_combat::moveset::advance_move_playback,
            project_boss_attack_state_from_move,
        )
            .chain(),
    );
    // §A1 split: the trigger reads the intent (telegraph edge → play the windup);
    // the projection writes the read-model `BossAttackState` from the live move.
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
            ambition_combat::components::ActorFaction::Boss,
            ambition_platformer2d_core::BodyKinematics {
                pos: ambition_platformer2d_core::Vec2::ZERO,
                vel: ambition_platformer2d_core::Vec2::ZERO,
                size: combat_size,
                facing: 1.0,
            },
        ))
        .id();
    (app, boss)
}

/// A Telegraph-step intent starts the move at its windup (`t0 = 0`). The
/// projection reports `telegraph_profile` during the windup, then
/// `active_profile` once the move's clock reaches the strike window:
/// `BossAttackState` is derived from the live move in both halves.
#[test]
fn telegraph_edge_trigger_projects_windup_then_strike() {
    let (mut app, boss) = telegraph_boss_app();

    // Frame 1: the telegraph intent starts the move at t0=0; one advance puts it
    // ~0.05s into the 0.2s windup — the projection reports a telegraph, no strike.
    app.update();
    let st = app.world().get::<BossAttackState>(boss).unwrap();
    assert_eq!(
        st.telegraph_profile,
        Some(BossAttackProfile::Strike("floor_slam".to_string()))
    );
    assert_eq!(st.active_profile, None, "windup has no live strike yet");

    // Advance past the 0.2s telegraph into the strike window: the projection
    // now reports the strike, the telegraph is cleared, and active_elapsed
    // includes the telegraph offset (t ≈ 0.25 > 0.2).
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

/// E53 Slice D: a windup the pattern abandons (intent cleared — phase change / suppress / rest)
/// must not strike.
#[test]
fn interrupted_windup_is_aborted_before_the_strike() {
    let (mut app, boss) = telegraph_boss_app();
    app.update();
    assert!(
        app.world()
            .get::<ambition_combat::moveset::MovePlayback>(boss)
            .is_some(),
        "the telegraph started a move"
    );
    // The pattern abandons the windup (e.g. a phase transition cleared intent):
    // clearing the intent (§A1 split) is what the trigger observes to abort.
    app.world_mut()
        .get_mut::<BossAttackIntent>(boss)
        .unwrap()
        .clear();
    app.update();
    assert!(
        app.world()
            .get::<ambition_combat::moveset::MovePlayback>(boss)
            .is_none(),
        "an abandoned windup is aborted before it can strike"
    );
}

/// The boss's authored `strike_speed_scale` is the move's motion lock: baked
/// onto the strike's Active window as `MoveWindow::motion_scale` and read back
/// through `MoveSpec::motion_scale_at`, so body integration damps the boss's
/// steering only while the strike window is live.
#[test]
fn the_strike_speed_throttle_is_baked_as_the_moves_motion_lock() {
    let cap = BossCapability {
        specials: vec![(BossAttackProfile::Strike("floor_slam".to_string()), 0.3)],
    };
    let behavior = warden_behavior(); // authors strike_speed_scale = 0.20
    let moveset = crate::attack_moveset::boss_attack_moveset(
        &cap,
        &behavior,
        ambition_platformer2d_core::Vec2::new(80.0, 80.0),
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

/// An authored telegraph's cue/vfx are move data: one-shot `MoveEvent`s on the
/// windup's rising edge, dispatched by the same `dispatch_move_events` channel
/// as every actor move. A move with no authored spec (or no telegraph) has no
/// events.
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
    let moveset = crate::attack_moveset::boss_attack_moveset(
        &cap,
        &warden_behavior(),
        ambition_platformer2d_core::Vec2::new(80.0, 80.0),
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
            (ev.at_s - crate::attack_moveset::TELEGRAPH_EDGE_S).abs() < f32::EPSILON,
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

/// Two boss moves separated by idle take different occurrence numbers, and
/// each playback carries the number the body reached.
///
/// The second assertion is the identity check: the generic guard in
/// `moveset/tests.rs` only checks that the counter is present, not that
/// `playback.instance == occurrence.0`. `trigger_boss_attack_moves` is a
/// second production start road (installed in `CombatSet::Trigger`), so it
/// needs its own witness.
///
/// This fails if the boss insert drops `.at_occurrence(occurrence)` (both
/// moves stay at `instance: 0`) or drops the `MoveOccurrence` insert (the
/// body's count restarts).
#[test]
fn two_boss_moves_separated_by_idle_take_different_occurrences() {
    use ambition_combat::moveset::{MoveOccurrence, MovePlayback};

    let mut app = App::new();
    app.add_systems(Update, trigger_boss_attack_moves);
    let moveset = boss_moveset_for_test();

    let boss = app
        .world_mut()
        .spawn((
            FeatureSimEntity,
            BossAttackIntent {
                active_profile: Some(BossAttackProfile::Strike("floor_slam".to_string())),
                ..Default::default()
            },
            moveset,
            ambition_platformer2d_core::BodyKinematics {
                pos: ambition_platformer2d_core::Vec2::ZERO,
                vel: ambition_platformer2d_core::Vec2::ZERO,
                size: ambition_platformer2d_core::Vec2::new(80.0, 80.0),
                facing: 1.0,
            },
        ))
        .id();

    // first move.
    app.update();
    let first = {
        let w = app.world();
        let playback = w
            .get::<MovePlayback>(boss)
            .expect("the boss trigger started a move");
        let occurrence = w
            .get::<MoveOccurrence>(boss)
            .expect("the boss road joined the body-owned mint");
        assert_eq!(
            playback.instance, occurrence.0,
            "the first boss move's playback carries `instance: {}` while the body \
             reached `MoveOccurrence({})`. A playback whose number disagrees with \
             the body's counter credits another use's hits — and a guard that \
             only checks the counter is PRESENT cannot see it.",
            playback.instance, occurrence.0
        );
        occurrence.0
    };

    // idle: the move is removed and no intent is standing, so nothing starts.
    app.world_mut().entity_mut(boss).remove::<MovePlayback>();
    app.world_mut().entity_mut(boss).insert(BossAttackIntent::default());
    app.update();
    assert!(
        app.world().get::<MovePlayback>(boss).is_none(),
        "the idle tick started a move, so the gap this test needs does not exist"
    );

    // second move, after the gap.
    app.world_mut().entity_mut(boss).insert(BossAttackIntent {
        active_profile: Some(BossAttackProfile::Strike("floor_slam".to_string())),
        ..Default::default()
    });
    app.update();
    let w = app.world();
    let playback = w
        .get::<MovePlayback>(boss)
        .expect("the boss trigger started a second move");
    let occurrence = w
        .get::<MoveOccurrence>(boss)
        .expect("the counter survived the idle gap");

    assert_ne!(
        occurrence.0, first,
        "two boss moves separated by an idle tick both took occurrence {first}. \
         `MovePlayback::new_at` leaves `instance` at 0, so before the mint every \
         boss move reused the same number and two uses credited each other."
    );
    assert_eq!(
        playback.instance, occurrence.0,
        "the second boss move's playback carries `instance: {}` while the body \
         reached `MoveOccurrence({})`.",
        playback.instance, occurrence.0
    );
}
