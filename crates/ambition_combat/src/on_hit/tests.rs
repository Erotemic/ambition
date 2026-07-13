//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

use super::*;
use crate::moveset::{advance_move_playback, MoveEventMessage, MovePlayback};
use ambition_entity_catalog::{
    ClipBinding, HitVolume, MoveSpec, MoveWindow, VolumeShape, WindowTag,
};
use ambition_sfx::SfxMessage;
use ambition_time::WorldTime;
use bevy::prelude::*;

/// A down-air whose single Active volume (below the body) carries the pogo
/// on-hit effect.
fn pogo_dair() -> MoveSpec {
    MoveSpec {
        id: "attack_air_down".into(),
        clip: ClipBinding {
            clip: "dair".into(),
            fallbacks: vec![],
        },
        duration_s: 0.12,
        windows: vec![MoveWindow {
            start_s: 0.0,
            end_s: 0.12,
            tag: WindowTag::Active,
            volumes: vec![HitVolume {
                // Body-local +y = gravity-down: the volume sits below the body.
                shape: VolumeShape::Rect {
                    offset: (0.0, 24.0),
                    half_extents: (18.0, 18.0),
                },
                damage: 4,
                knockback: 0.0,
                kb_growth: 0.0,
                launch_dir: None,
                on_hit: Some(EffectRef::new(POGO_BOUNCE_KEY)),
                vfx: None,
            }],
            sustain_effect: None,
        }],
        events: vec![],
        gates: Default::default(),
        start_impulse: None,
        smash_charge_mult: 1.0,
    }
}

/// Owner (Player) playing the pogo down-air, a victim (Enemy) directly below
/// its down-volume. `victim_is_pogoable` toggles the `PogoTarget` capability.
fn harness(victim_is_pogoable: bool) -> (App, Entity) {
    let mut app = App::new();
    app.insert_resource(ambition_characters::actor::character_catalog::CharacterCatalog::empty());
    app.init_resource::<crate::authored_volumes::AuthoredAttackVolumeResolver>();
    app.add_message::<MoveEventMessage>();
    app.add_message::<ambition_vfx::vfx::VfxMessage>();
    app.add_message::<OnHitEffectMessage>();
    app.add_message::<SfxMessage>();
    app.init_resource::<WorldTime>();
    app.world_mut().resource_mut::<WorldTime>().scaled_dt = 0.016;
    app.world_mut().resource_mut::<WorldTime>().raw_dt = 0.016;
    app.add_systems(
        Update,
        (
            advance_move_playback,
            dispatch_hitbox_on_hit,
            apply_pogo_bounce,
        )
            .chain(),
    );
    let owner = app
        .world_mut()
        .spawn((
            ae::CenteredAabb::from_center_size(
                ae::Vec2::new(100.0, 100.0),
                ae::Vec2::new(28.0, 46.0),
            ),
            ae::BodyKinematics {
                pos: ae::Vec2::new(100.0, 100.0),
                vel: ae::Vec2::ZERO,
                size: ae::Vec2::new(28.0, 46.0),
                facing: 1.0,
            },
            ambition_engine_core::BodyGroundState {
                on_ground: true,
                ..Default::default()
            },
            ActorFaction::Player,
            MovePlayback::new(pogo_dair(), 1.0),
            ambition_platformer_primitives::frame_env::ResolvedMotionFrame::default(),
        ))
        .id();
    let victim = app
        .world_mut()
        .spawn((
            ae::CenteredAabb::from_center_size(
                ae::Vec2::new(100.0, 130.0),
                ae::Vec2::new(28.0, 46.0),
            ),
            ActorFaction::Enemy,
        ))
        .id();
    if victim_is_pogoable {
        app.world_mut().entity_mut(victim).insert(PogoTarget);
    }
    (app, owner)
}

#[test]
fn down_air_pogos_off_a_pogo_target() {
    let (mut app, owner) = harness(true);
    for _ in 0..2 {
        app.update();
    }
    let kin = app.world().get::<ae::BodyKinematics>(owner).unwrap();
    assert!(
        kin.vel.y < -1.0,
        "the owner rebounded gravity-up (pogo), vel={:?}",
        kin.vel
    );
    assert!(
        !app.world()
            .get::<ambition_engine_core::BodyGroundState>(owner)
            .unwrap()
            .on_ground,
        "the pogo un-grounds the owner",
    );
}

#[test]
fn down_air_pogos_off_a_factionless_world_orb() {
    // A pogo-orb is a FACTIONLESS world breakable (CenteredAabb + PogoTarget,
    // no ActorFaction). Victim-pogo and world-orb pogo unify under the one
    // capability (fable review R2.5, Jon's call).
    let mut app = App::new();
    app.insert_resource(ambition_characters::actor::character_catalog::CharacterCatalog::empty());
    app.init_resource::<crate::authored_volumes::AuthoredAttackVolumeResolver>();
    app.add_message::<MoveEventMessage>();
    app.add_message::<ambition_vfx::vfx::VfxMessage>();
    app.add_message::<OnHitEffectMessage>();
    app.add_message::<SfxMessage>();
    app.init_resource::<WorldTime>();
    app.world_mut().resource_mut::<WorldTime>().scaled_dt = 0.016;
    app.world_mut().resource_mut::<WorldTime>().raw_dt = 0.016;
    app.add_systems(
        Update,
        (
            advance_move_playback,
            dispatch_hitbox_on_hit,
            apply_pogo_bounce,
        )
            .chain(),
    );
    let owner = app
        .world_mut()
        .spawn((
            ae::CenteredAabb::from_center_size(
                ae::Vec2::new(100.0, 100.0),
                ae::Vec2::new(28.0, 46.0),
            ),
            ae::BodyKinematics {
                pos: ae::Vec2::new(100.0, 100.0),
                vel: ae::Vec2::ZERO,
                size: ae::Vec2::new(28.0, 46.0),
                facing: 1.0,
            },
            ambition_engine_core::BodyGroundState {
                on_ground: true,
                ..Default::default()
            },
            ActorFaction::Player,
            MovePlayback::new(pogo_dair(), 1.0),
            ambition_platformer_primitives::frame_env::ResolvedMotionFrame::default(),
        ))
        .id();
    // The orb below: NO ActorFaction, just the capability.
    app.world_mut().spawn((
        ae::CenteredAabb::from_center_size(ae::Vec2::new(100.0, 130.0), ae::Vec2::new(28.0, 46.0)),
        PogoTarget,
    ));
    for _ in 0..2 {
        app.update();
    }
    let kin = app.world().get::<ae::BodyKinematics>(owner).unwrap();
    assert!(
        kin.vel.y < -1.0,
        "the owner pogos off a factionless world orb, vel={:?}",
        kin.vel
    );
}

#[test]
fn no_pogo_off_a_bare_victim_without_the_capability() {
    let (mut app, owner) = harness(false);
    for _ in 0..2 {
        app.update();
    }
    let kin = app.world().get::<ae::BodyKinematics>(owner).unwrap();
    assert_eq!(
        kin.vel,
        ae::Vec2::ZERO,
        "a victim without PogoTarget grants no bounce, vel={:?}",
        kin.vel
    );
}
