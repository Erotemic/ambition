//! Authored silhouettes, through the REAL damage path, for every body family.
//!
//! `tests.rs` beside this file proves the resolver picks the right timeline. That
//! is a pure function, and proving it in isolation is exactly how §7.10 shipped a
//! feature that resolved correctly and reached nobody: the volumes were published
//! into `DamageableVolumes`, and `apply_hitbox_damage` never read that component —
//! it tested the victim's coarse `CenteredAabb`. Pogo targeting and the debug
//! overlay changed; combat did not.
//!
//! So these tests assert the integration, and each one is built so the COARSE box
//! would give the opposite answer. A strike is placed inside the body's bounding
//! rectangle but outside its authored silhouette: if the authored volumes are
//! being consulted it misses, and if anything falls back to the box it hits. That
//! asymmetry is the whole point — a test that only checks "the hit lands" passes
//! just as well when the feature is disconnected.

use bevy::prelude::*;

use ambition_combat::components::{ActorFaction, CenteredAabb, DamageableVolumes};
use ambition_combat::events::HitEvent;
use ambition_combat::hitbox::{apply_hitbox_damage, HitSide, Hitbox, HitboxHits, HitboxLifetime};
use ambition_entity_catalog::{HurtboxKeyframe, HurtboxTimeline, VolumeShape};
use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::AabbExt;
use ambition_vfx::vfx::VfxMessage;

use super::{AuthoredHurtboxes, HurtboxDoc, HurtboxVolume, ResolvedHurtboxes};
use crate::features::refresh_body_damageable_volumes;

/// The body's coarse box: 28 wide, 40 tall, centred on the origin.
const COARSE_HALF: ae::Vec2 = ae::Vec2::new(14.0, 20.0);
/// The authored silhouette: a narrow torso, 8 wide.
const AUTHORED_HALF: (f32, f32) = (4.0, 18.0);

#[derive(Resource, Default)]
struct CapturedHits(Vec<HitEvent>);

fn capture_hits(mut events: MessageReader<HitEvent>, mut out: ResMut<CapturedHits>) {
    out.0.extend(
        events
            .read()
            .filter(|e| matches!(e.target, ambition_combat::events::HitTarget::Body(_)))
            .cloned(),
    );
}

/// A doc whose only volume is much narrower than the body's bounding box.
fn narrow_torso() -> HurtboxDoc {
    HurtboxDoc {
        default: Some(HurtboxTimeline {
            keyframes: vec![HurtboxKeyframe {
                at_s: 0.0,
                volumes: vec![HurtboxVolume {
                    shape: VolumeShape::Rect {
                        offset: (0.0, 0.0),
                        half_extents: AUTHORED_HALF,
                    },
                }],
            }],
        }),
        poses: Default::default(),
        moves: Default::default(),
    }
}

/// The real systems, in the real order: resolve the silhouette, publish it, then
/// let the production damage system resolve overlap against what was published.
fn fight_app() -> App {
    let mut app = App::new();
    app.add_message::<HitEvent>();
    app.add_message::<ambition_combat::hitbox::LandedBodyHit>();
    app.add_message::<VfxMessage>();
    app.init_resource::<CapturedHits>();
    app.add_systems(
        Update,
        (
            super::resolve_body_hurtboxes,
            refresh_body_damageable_volumes,
            apply_hitbox_damage,
            capture_hits,
        )
            .chain(),
    );
    app
}

/// Spawn an attacker plus a `World`-anchored strike box at `strike_x`.
/// The world box a strike spawned by [`spawn_strike`] occupies.
///
/// The prose said the strike overlaps the coarse box; move the strike to x = 40 and the miss would
/// still "pass", having proven only that a strike reaching nothing hits nothing — the A10 defect
/// class.
fn strike_box(strike_x: f32, half_width: f32) -> ae::Aabb {
    ae::Aabb::new(
        ae::Vec2::new(strike_x, 0.0),
        ae::Vec2::new(half_width, 18.0),
    )
}

fn spawn_strike(app: &mut App, strike_x: f32, half_width: f32) {
    let owner = app
        .world_mut()
        .spawn(CenteredAabb::new(
            ae::Vec2::new(strike_x, 0.0),
            ae::Vec2::new(4.0, 4.0),
        ))
        .id();
    app.world_mut().spawn((
        Hitbox {
            // An ordinary hit, not a gust.
            strike_sfx: None,
            owner,
            source: HitSide::Enemy,
            anchor: ambition_combat::hitbox::HitboxAnchor::World {
                center: ae::Vec2::new(strike_x, 0.0),
            },
            half_extent: ae::Vec2::new(half_width, 18.0),
            shape: None,
            facing: 1.0,
            damage: 3,
            knockback: ambition_combat::strike::HitboxKnockback::FeelScale(0.0),
            launch_dir: None,
            frame_down: ae::Vec2::new(0.0, 1.0),
            reaction: None,
        },
        HitboxLifetime { remaining_s: 0.2 },
        HitboxHits::default(),
    ));
}

/// Spawn a body that authors a narrow silhouette. `player` decides only whether
/// the `PlayerEntity` marker is present — that is the axis under test.
fn spawn_authored_body(app: &mut App, player: bool) -> Entity {
    let mut body = app.world_mut().spawn((
        CenteredAabb::new(ae::Vec2::ZERO, COARSE_HALF),
        ae::BodyKinematics {
            pos: ae::Vec2::ZERO,
            size: COARSE_HALF * 2.0,
            ..Default::default()
        },
        ActorFaction::Player,
        // Carrying this is what makes a body a damage target at all.
        DamageableVolumes::default(),
        AuthoredHurtboxes(narrow_torso()),
        ResolvedHurtboxes::default(),
        ae::BodyOffense::default(),
        ae::BodyMotionFacts::default(),
        ae::BodyShieldState::default(),
        ambition_characters::actor::BodyCombat::default(),
    ));
    if player {
        body.insert(ambition_platformer2d_shared_tangle::markers::PlayerEntity);
    }
    body.id()
}

/// A3 + A7. The primary player is hit on its AUTHORED silhouette.
///
/// Two independent reasons the same feature could not work for the most important body in the
/// game.
#[test]
fn a_player_shaped_body_is_hit_on_its_authored_hurtbox() {
    // ── Inside the coarse box, outside the authored torso: MISS ──
    // Strike spans x ∈ [12, 28]. The body's box reaches x = 14; its authored
    // torso stops at x = 4.
    let mut app = fight_app();
    let player = spawn_authored_body(&mut app, true);
    spawn_strike(&mut app, 20.0, 8.0);
    // The premise, checked: this strike must genuinely reach the coarse box, or the
    // miss below is the uninteresting kind.
    assert!(
        strike_box(20.0, 8.0).strict_intersects(ae::Aabb::new(ae::Vec2::ZERO, COARSE_HALF)),
        "the strike must overlap the body's COARSE box, or `hit on the silhouette \
         and not the box` is not the distinction this case is making"
    );
    app.update();
    assert!(
        app.world().resource::<CapturedHits>().0.is_empty(),
        "a strike that overlaps the player's bounding rectangle but NOT its \
         authored silhouette must miss; landing it means damage is still reading \
         the coarse box"
    );
    // And the publication really happened — otherwise the miss above would be
    // vacuous (an empty volume list misses everything).
    let published = app
        .world()
        .get::<DamageableVolumes>(player)
        .expect("the player publishes damageable volumes like every other body");
    assert_eq!(
        published.volumes.len(),
        1,
        "the player's authored silhouette is published, not skipped"
    );
    assert!(
        published.volumes[0].bounds().half_size().x < COARSE_HALF.x,
        "the published volume is the AUTHORED torso ({:?}), not the coarse box",
        published.volumes[0]
    );

    // ── Overlapping the authored torso: HIT ──
    let mut app = fight_app();
    spawn_authored_body(&mut app, true);
    spawn_strike(&mut app, 8.0, 8.0);
    app.update();
    let hits = &app.world().resource::<CapturedHits>().0;
    assert_eq!(
        hits.len(),
        1,
        "a strike overlapping the authored torso lands on the player"
    );
    assert_eq!(hits[0].damage, 3);
}

/// The same body, the same authored doc, the same strike — with and without the
/// player marker. The answers must be identical.
///
/// but not a player.* Before this change the two rows of this table disagreed,
/// and nothing in the codebase would have noticed.
#[test]
fn authored_hurtboxes_do_not_care_which_family_the_body_belongs_to() {
    for player in [false, true] {
        let mut app = fight_app();
        spawn_authored_body(&mut app, player);
        spawn_strike(&mut app, 20.0, 8.0);
        app.update();
        assert!(
            app.world().resource::<CapturedHits>().0.is_empty(),
            "player={player}: a strike outside the authored silhouette must miss \
             for EVERY family, not just for feature-simulation actors"
        );

        let mut app = fight_app();
        spawn_authored_body(&mut app, player);
        spawn_strike(&mut app, 8.0, 8.0);
        app.update();
        assert_eq!(
            app.world().resource::<CapturedHits>().0.len(),
            1,
            "player={player}: a strike inside the authored silhouette must land \
             for EVERY family"
        );
    }
}

/// An authored EMPTY window is invulnerability, and must not degrade into the
/// coarse box.
///
/// This is the failure mode `strike_reaches_victim` is shaped around: "published
/// nothing" and "carries no component" look the same if you test `is_empty()`,
/// and collapsing them turns an authored invulnerable frame into a hittable
/// rectangle — the single worst way for this feature to fail, because it fails
/// only during the frames a character was deliberately protected.
#[test]
fn an_authored_empty_window_is_invulnerable_not_a_fallback_to_the_box() {
    let mut app = fight_app();
    let body = app
        .world_mut()
        .spawn((
            CenteredAabb::new(ae::Vec2::ZERO, COARSE_HALF),
            ae::BodyKinematics {
                pos: ae::Vec2::ZERO,
                size: COARSE_HALF * 2.0,
                ..Default::default()
            },
            ActorFaction::Player,
            DamageableVolumes::default(),
            AuthoredHurtboxes(HurtboxDoc {
                default: Some(HurtboxTimeline {
                    keyframes: vec![HurtboxKeyframe {
                        at_s: 0.0,
                        volumes: vec![],
                    }],
                }),
                poses: Default::default(),
                moves: Default::default(),
            }),
            ResolvedHurtboxes::default(),
            ae::BodyOffense::default(),
            ae::BodyMotionFacts::default(),
            ae::BodyShieldState::default(),
            ambition_characters::actor::BodyCombat::default(),
            ambition_platformer2d_shared_tangle::markers::PlayerEntity,
        ))
        .id();
    // Dead centre of the body: the coarse box would certainly be hit.
    spawn_strike(&mut app, 0.0, 8.0);
    app.update();
    assert!(
        app.world().resource::<CapturedHits>().0.is_empty(),
        "an authored empty window means invulnerable; a strike through the middle \
         of the body must still miss"
    );
    assert!(
        app.world()
            .get::<DamageableVolumes>(body)
            .expect("published")
            .volumes
            .is_empty(),
        "and the published list is empty, which is a DECISION, not an absence"
    );
}

/// A body that authored nothing keeps its coarse box, so this change is not a
/// silent nerf to every un-migrated enemy in the game.
#[test]
fn an_unauthored_body_is_still_hit_on_its_coarse_box() {
    let mut app = fight_app();
    app.world_mut().spawn((
        CenteredAabb::new(ae::Vec2::ZERO, COARSE_HALF),
        ae::BodyKinematics {
            pos: ae::Vec2::ZERO,
            size: COARSE_HALF * 2.0,
            ..Default::default()
        },
        ActorFaction::Player,
        DamageableVolumes::default(),
        ae::BodyOffense::default(),
        ae::BodyMotionFacts::default(),
        ae::BodyShieldState::default(),
        ambition_characters::actor::BodyCombat::default(),
    ));
    // x ∈ [12, 28]: inside the coarse box, outside where an authored torso would
    // have been. With no doc, the box IS the answer, so this must land.
    spawn_strike(&mut app, 20.0, 8.0);
    app.update();
    assert_eq!(
        app.world().resource::<CapturedHits>().0.len(),
        1,
        "no authored doc ⇒ the coarse box is still the silhouette"
    );
}

/// The schedule slot is part of the contract, so it is asserted, not commented.
///
/// `resolve_body_hurtboxes` lived in `Update` at first. That is wrong twice over:
/// `Update` runs once per FRAME while the sim schedule re-runs many times per
/// frame during rollback resimulation, so every rewound tick read a stale
/// silhouette — for state declared rollback-DERIVED on the promise that the sim
/// rebuilds it — and a frame-rate-dependent hurtbox means two peers at different
/// frame rates disagree about what got hit.
///
/// It is now pinned between `advance_move_playback` and `apply_hitbox_damage`,
/// and this test asserts the CONSEQUENCE of that pinning rather than the graph
/// edges, because Bevy encodes `.after(some_fn)` as an edge from that function's
/// `SystemTypeSet` — so a naive system-to-system edge lookup finds nothing and
/// passes for the wrong reason.
///
/// The move's silhouette WIDENS at t = 0.1s. A strike is parked where only the
/// wide keyframe reaches it. If resolution runs after the move clock, the hit
/// lands on the very tick the clock crosses 0.1s; if it runs before, the wide
/// keyframe is selected a tick late and the hit lands a tick late. One tick is
/// the entire difference between these two schedules, and it is exactly the
/// error a comment cannot catch.
#[test]
fn a_widening_move_silhouette_is_hittable_on_the_tick_it_widens() {
    use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt;

    const WIDEN_AT_S: f32 = 0.1;
    let doc = HurtboxDoc {
        default: Some(HurtboxTimeline {
            keyframes: vec![HurtboxKeyframe {
                at_s: 0.0,
                volumes: vec![HurtboxVolume {
                    shape: VolumeShape::Rect {
                        offset: (0.0, 0.0),
                        half_extents: (4.0, 18.0),
                    },
                }],
            }],
        }),
        poses: Default::default(),
        moves: std::collections::BTreeMap::from([(
            "swing".to_string(),
            HurtboxTimeline {
                keyframes: vec![
                    HurtboxKeyframe {
                        at_s: 0.0,
                        volumes: vec![HurtboxVolume {
                            shape: VolumeShape::Rect {
                                offset: (0.0, 0.0),
                                half_extents: (4.0, 18.0),
                            },
                        }],
                    },
                    HurtboxKeyframe {
                        at_s: WIDEN_AT_S,
                        volumes: vec![HurtboxVolume {
                            shape: VolumeShape::Rect {
                                offset: (0.0, 0.0),
                                half_extents: (30.0, 18.0),
                            },
                        }],
                    },
                ],
            },
        )]),
    };

    let mut app = App::new();
    crate::schedule::configure_platformer2d_simulation_phases(&mut app);
    // The session gate (`simulation_authorized`) requires EXACTLY ONE `SessionRoot`, and every
    // gameplay set is nested inside it.
    app.world_mut()
        .spawn(ambition_platformer2d_shared_tangle::lifecycle::SessionRoot(
            ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId(0),
        ));
    app.add_plugins(crate::character_runtime::CharacterRuntimePlugin);
    // The PRODUCTION registration of the publication, ordering included.
    crate::features::register_damage_facing_volume_publication(&mut app);
    let sim = app.sim_schedule();
    app.add_systems(
        sim,
        (
            // The PHASES production puts these in, not the umbrella set. The
            // system under test orders itself `.after(Playback).before(Resolve)`,
            // so a fixture that drops its stand-ins into `Combat` directly leaves
            // those edges constraining nothing and silently tests a different
            // schedule than the one that ships.
            ambition_combat::moveset::advance_move_playback
                .in_set(crate::schedule::CombatSet::Playback),
            (apply_hitbox_damage, capture_hits)
                .chain()
                .in_set(crate::schedule::CombatSet::Resolve),
        ),
    );
    app.add_message::<HitEvent>();
    app.add_message::<ambition_combat::hitbox::LandedBodyHit>();
    app.add_message::<VfxMessage>();
    app.add_message::<ambition_combat::moveset::MoveEventMessage>();
    app.init_resource::<CapturedHits>();
    app.init_resource::<ambition_time::WorldTime>();
    app.insert_resource(
        ambition_combat::authored_volumes::AuthoredAttackVolumeResolver::disabled(),
    );
    app.insert_resource(ambition_characters::actor::character_catalog::CharacterCatalog::empty());
    app.init_resource::<ambition_sprite_sheet::character::sheets::AuthoredSheets>();
    {
        let mut time = app.world_mut().resource_mut::<ambition_time::WorldTime>();
        time.scaled_dt = 1.0 / 60.0;
        time.raw_dt = 1.0 / 60.0;
    }

    let body = app
        .world_mut()
        .spawn((
            CenteredAabb::new(ae::Vec2::ZERO, ae::Vec2::new(4.0, 18.0)),
            ae::BodyKinematics {
                pos: ae::Vec2::ZERO,
                size: ae::Vec2::new(8.0, 36.0),
                facing: 1.0,
                ..Default::default()
            },
            ActorFaction::Player,
            DamageableVolumes::default(),
            AuthoredHurtboxes(doc),
            ResolvedHurtboxes::default(),
            ae::BodyOffense::default(),
            ae::BodyMotionFacts::default(),
            ae::BodyShieldState::default(),
            ambition_characters::actor::BodyCombat::default(),
            ambition_combat::moveset::MovePlayback::new(widening_swing(), 1.0),
        ))
        .id();
    // Parked where ONLY the wide keyframe reaches: x in [20, 36]. The narrow
    // silhouette stops at x = 4; the wide one reaches x = 30.
    spawn_strike(&mut app, 28.0, 8.0);

    let mut landed_at: Option<f32> = None;
    for _ in 0..20 {
        app.update();
        if !app.world().resource::<CapturedHits>().0.is_empty() {
            landed_at = app
                .world()
                .get::<ambition_combat::moveset::MovePlayback>(body)
                .map(|p| p.t);
            break;
        }
    }
    let t = landed_at.expect("the widened silhouette must eventually be hit");
    assert!(
        t < WIDEN_AT_S + 1.5 / 60.0,
        "the hit landed at t = {t}s, but the silhouette widens at {WIDEN_AT_S}s — \
         more than one tick of slack means hurtbox resolution is reading a move \
         clock it has not yet advanced"
    );
}

/// A move long enough to outlive its own widening keyframe.
fn widening_swing() -> ambition_entity_catalog::MoveSpec {
    ambition_entity_catalog::MoveSpec {
        display_name: None,
        landing_lag_s: None,
        autocancel_after_s: None,
        sprite_spin_hz: None,
        equips: None,
        id: "swing".to_string(),
        clip: ambition_entity_catalog::ClipBinding {
            clip: "swing".to_string(),
            fallbacks: vec![],
        },
        duration_s: 1.0,
        events: vec![],
        windows: vec![],
        gates: ambition_entity_catalog::MoveGates::default(),
        start_impulse: None,
        smash_charge_mult: 1.0,
        smash_charge: None,
        charge_gesture: ambition_entity_catalog::ChargeGesture::default(),
        repeat: None,
    }
}
