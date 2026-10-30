//! Bomb — a thrown held item that explodes, damaging enemies in a radius via
//! a player-side AABB [`HitEvent`].
//!
//! Uses the held-item substrate: `bomb` is a pure-throwable `HeldItemSpec` (no
//! melee or ranged verb), so a plain `Attack` throws it and it flies as a
//! [`GroundItem`]. It is heavy and it bounces ([`BOMB_BODY`], given by
//! [`weigh_bombs`] before its first step). A thrown bomb gets a [`BombFuse`].
//! It goes off on the first of:
//!
//! * the end of its fuse ([`BOMB_FUSE_SECS`]; the first bounce cuts what is
//!   left to [`BOMB_FUSE_AFTER_BOUNCE_SECS`]);
//! * a strike on a surface at [`BOMB_SURFACE_DETONATION_SPEED`] or faster
//!   (a hard throw into a wall or the floor);
//! * a strike on a body at [`BOMB_BODY_DETONATION_SPEED`] or faster.
//!
//! The blast is a `Melee`-source hit that damages enemies and bosses in it
//! (the player is spared), with a large burst, smoke, a shockwave and a camera
//! shake. A bomb that was never thrown does not arm, and a bomb in a hand
//! does not go off on an impact.

use bevy::prelude::*;

use ambition_combat::events::{HitEvent, HitMode, HitSource, HitTarget};
use ambition_held_items::{GroundItem, ItemBody, ItemCustody, ItemStruckBody, ItemStruckSurface, Release, ReleasedAs};
use ambition_platformer2d_core as ae;

/// Held-item id the bomb grants.
pub const BOMB_ID: &str = "bomb";

/// Seconds from being thrown to detonation, when nothing sets it off first.
pub const BOMB_FUSE_SECS: f32 = 1.2;
/// The most fuse that is left after the bomb first strikes a surface: a lob
/// that lands and bounces goes off soon after.
pub const BOMB_FUSE_AFTER_BOUNCE_SECS: f32 = 0.5;
/// Speed into a surface (px/s) at or above which a bomb goes off where it
/// strikes. A tilt throw lands under it and bounces; a smash throw into a
/// wall or a throw down at the floor is over it.
pub const BOMB_SURFACE_DETONATION_SPEED: f32 = 650.0;
/// Speed (px/s) at or above which a bomb goes off on a body it strikes: any
/// throw that reaches an enemy.
pub const BOMB_BODY_DETONATION_SPEED: f32 = 300.0;
/// A strike on a surface at or above this speed (px/s) makes a sound.
const BOMB_CLUNK_SPEED: f32 = 120.0;
/// How a loose bomb moves: heavy (half again the gravity of a loose item), a
/// low bounce, and a skid that stops quickly.
pub const BOMB_BODY: ItemBody = ItemBody {
    gravity_scale: 1.5,
    bounce: 0.4,
    slide: 0.7,
    rest_speed: 60.0,
};
/// Blast half-extent (AABB), px.
const BOMB_BLAST_HALF: f32 = 80.0;
/// Explosion damage.
const BOMB_DAMAGE: i32 = 4;
/// Camera shake of a blast, px (before the game's cap).
const BOMB_SHAKE_PX: f32 = 9.0;

/// Lit fuse on a thrown bomb. Counts down to detonation.
#[derive(Component, Clone, Copy, Debug)]
pub struct BombFuse {
    pub timer: f32,
}

/// Give each bomb its body ([`BOMB_BODY`]) before it moves.
///
/// Runs in the item step that is before the physics step
/// (`HeldItemStep::Settle`), so a bomb thrown on this tick falls as a bomb
/// on its first step. It gives the body again to a bomb that lost it (a
/// rollback restore does not keep it), so the body is a property of the bomb
/// and not rollback state.
pub fn weigh_bombs(mut commands: Commands, bombs: Query<(Entity, &GroundItem), Without<ItemBody>>) {
    for (entity, ground) in &bombs {
        if ground.spec.id == BOMB_ID {
            commands.entity(entity).insert(BOMB_BODY);
        }
    }
}

/// A `bomb` [`GroundItem`] is armed exactly while it is in the world because
/// somebody threw it. This system arms and disarms.
///
/// It reads [`ReleasedAs`], not velocity: `Release::Throw` versus
/// `Release::Drop` is decided by the release transaction. Velocity cannot
/// tell a throw from falling under gravity, or a catch from a rest. A Z-drop
/// hands the item to the floor and does not arm.
///
/// Disarming is in the same system, chained before [`tick_bomb_fuses`], so a
/// bomb caught this tick loses its fuse before it can burn. Throwing it again
/// re-arms with a fresh fuse.
pub fn arm_thrown_bombs(
    mut commands: Commands,
    bombs: Query<(Entity, &GroundItem, Option<&ReleasedAs>, Has<BombFuse>)>,
) {
    for (entity, ground, released, armed) in &bombs {
        if ground.spec.id != BOMB_ID {
            continue;
        }
        let thrown = matches!(released, Some(ReleasedAs(Release::Throw)));
        match (thrown, armed) {
            (true, false) => {
                commands.entity(entity).insert(BombFuse {
                    timer: BOMB_FUSE_SECS,
                });
            }
            (false, true) => {
                commands.entity(entity).remove::<BombFuse>();
            }
            _ => {}
        }
    }
}

/// Is this a hard strike, which sets a bomb off where it is? `surface` and
/// `body` are the speeds of this tick's strikes on a surface and on a body
/// (`0` for none).
pub fn strikes_hard_enough(surface: f32, body: f32) -> bool {
    surface >= BOMB_SURFACE_DETONATION_SPEED || body >= BOMB_BODY_DETONATION_SPEED
}

/// Burn down lit fuses, and set off each bomb whose fuse ended or that struck
/// something hard this tick. A blast is a player-side [`HitEvent`] (it
/// damages enemies and bosses in the AABB, not the player), with its sound,
/// burst and camera shake, and the bomb despawns.
///
/// It reads the strikes that `ground_item_physics` published this tick
/// ([`ItemStruckSurface`], [`ItemStruckBody`]). Only a loose bomb strikes:
/// a hand is not a surface.
pub fn tick_bomb_fuses(
    time: Res<ambition_time::WorldTime>,
    mut commands: Commands,
    mut bombs: Query<(
        Entity,
        &GroundItem,
        &ItemCustody,
        &mut BombFuse,
        Option<&ItemStruckSurface>,
        Option<&ItemStruckBody>,
    )>,
    mut hits: MessageWriter<HitEvent>,
    mut sfx: ambition_sfx::BodySfxWriter,
    mut vfx: ambition_vfx::vfx::VfxWriter,
    mut shake: MessageWriter<ambition_platformer2d_shared_tangle::camera_ease::CameraShakeRequest>,
    // The blast is in the bomb's live room. It has no attacker to ask.
    rooms: ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
) {
    let dt = time.sim_dt();
    if dt <= 0.0 {
        return;
    }
    for (entity, ground, custody, mut fuse, surface, body) in &mut bombs {
        let loose = custody.in_world();
        let surface = surface.filter(|_| loose).map_or(0.0, |hit| hit.impact_speed);
        let body = body.filter(|_| loose).map_or(0.0, |hit| hit.impact_speed);
        if surface > 0.0 {
            // It landed or hit a wall: what is left of the fuse is short.
            fuse.timer = fuse.timer.min(BOMB_FUSE_AFTER_BOUNCE_SECS);
            if surface >= BOMB_CLUNK_SPEED {
                sfx.write_for(
                    entity,
                    ambition_sfx::SfxMessage::Play {
                        id: ambition_sfx::ids::WORLD_ROCK_HIT,
                        pos: ground.pos,
                    },
                );
            }
        }
        fuse.timer -= dt;
        if fuse.timer > 0.0 && !strikes_hard_enough(surface, body) {
            continue;
        }
        let room = rooms.of(entity);
        // Detonate: a broadcast player-side hit over a disc, so the blast is
        // radial.
        hits.write(HitEvent {
            pieces: Vec::new(),
            strike_sfx: None,
            volume: ae::CombatVolume::circle(ground.pos, BOMB_BLAST_HALF),
            damage: BOMB_DAMAGE,
            source: HitSource::Melee,
            attacker: None,
            room,
            target: HitTarget::Volume,
            mode: HitMode::Knockback,
            knockback: None,
            ignored_targets: Vec::new(),
            attacker_move_instance: None,
        });
        // The bomb inherits its thrower's source at spawn, so the blast plays
        // the thrower's cue, or the session's when nothing was stamped.
        sfx.write_for(
            entity,
            ambition_sfx::SfxMessage::Play {
                id: ambition_sfx::ids::WORLD_EXPLOSION,
                pos: ground.pos,
            },
        );
        // A large burst, the smoke it leaves, and a ring: the blast reads as
        // big as the area it hurts.
        let mut room_vfx = vfx.for_room(room);
        for (fx, scale) in [
            (ambition_vfx::fx::ids::CLASSIC_BURST, 2.2),
            (ambition_vfx::fx::ids::SMOKE_BURST, 1.8),
            (ambition_vfx::fx::ids::SHOCKWAVE, 1.6),
        ] {
            room_vfx.write(ambition_vfx::vfx::VfxMessage::Effect {
                pos: ground.pos,
                fx,
                scale,
                pose: ambition_vfx::FxPose::UPRIGHT,
            });
        }
        shake.write(ambition_platformer2d_shared_tangle::camera_ease::CameraShakeRequest {
            amplitude_px: BOMB_SHAKE_PX,
        });
        commands.entity(entity).despawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bomb_ground(vel: ae::Vec2) -> GroundItem {
        GroundItem::released(
            ambition_characters::brain::held_item_by_id(BOMB_ID).unwrap(),
            ae::Vec2::new(100.0, 100.0),
            vel,
            ae::Vec2::splat(14.0),
        )
    }

    /// A thrown bomb arms. Checked through the release, not a velocity.
    #[test]
    fn a_thrown_bomb_arms_but_a_bomb_nobody_threw_does_not() {
        let mut app = App::new();
        app.add_systems(Update, arm_thrown_bombs);
        let thrown = app
            .world_mut()
            .spawn((
                bomb_ground(ae::Vec2::new(60.0, -200.0)),
                ReleasedAs(Release::Throw),
            ))
            .id();
        let resting = app.world_mut().spawn(bomb_ground(ae::Vec2::ZERO)).id();
        app.update();
        assert!(
            app.world().get::<BombFuse>(thrown).is_some(),
            "thrown bomb arms"
        );
        assert!(
            app.world().get::<BombFuse>(resting).is_none(),
            "a bomb nobody threw stays safe"
        );
    }

    /// A bomb the room authored at rest and then falls under gravity is not
    /// thrown and does not arm.
    #[test]
    fn a_falling_bomb_nobody_threw_does_not_arm_itself() {
        let mut app = App::new();
        app.add_systems(Update, arm_thrown_bombs);
        // The velocity of a second of ordinary free fall.
        let authored = app
            .world_mut()
            .spawn(bomb_ground(ae::Vec2::new(0.0, 980.0)))
            .id();
        app.update();
        assert!(
            app.world().get::<BombFuse>(authored).is_none(),
            "a bomb armed itself by falling — ordinary gravity is being read as \
             a player's throw"
        );
    }

    /// Catching a live bomb disarms it. `arm_thrown_bombs` is chained before
    /// `tick_bomb_fuses` in the shipped schedule, so the disarm lands before
    /// the fuse can burn; this test runs them in that order.
    #[test]
    fn catching_a_live_bomb_puts_the_fuse_out() {
        let mut app = App::new();
        app.add_message::<HitEvent>();
        app.add_message::<ambition_sfx::OwnedSfxMessage>();
        app.add_message::<ambition_vfx::vfx::VfxInRoom>();
        app.add_message::<ambition_platformer2d_shared_tangle::camera_ease::CameraShakeRequest>();
        let mut wt = ambition_time::WorldTime::default();
        wt.set_sim_dt(0.05);
        app.insert_resource(wt);
        app.add_systems(Update, (arm_thrown_bombs, tick_bomb_fuses).chain());

        // In the air with a fuse almost out, as a caught bomb is.
        let caught = app
            .world_mut()
            .spawn((
                bomb_ground(ae::Vec2::ZERO),
                ReleasedAs(Release::Throw),
                BombFuse { timer: 0.001 },
            ))
            .id();
        // A body takes custody: the release is over, so its record is removed.
        app.world_mut().entity_mut(caught).remove::<ReleasedAs>();

        app.update();

        assert!(
            app.world().get::<GroundItem>(caught).is_some(),
            "a bomb detonated in the hand that caught it"
        );
        assert!(
            app.world().get::<BombFuse>(caught).is_none(),
            "the fuse survived the catch, so it is still counting down in custody"
        );
    }

    /// A Z-drop hands the item to the floor; it is not an attack and does not
    /// arm.
    #[test]
    fn a_dropped_bomb_does_not_arm() {
        let mut app = App::new();
        app.add_systems(Update, arm_thrown_bombs);
        let dropped = app
            .world_mut()
            .spawn((bomb_ground(ae::Vec2::ZERO), ReleasedAs(Release::Drop)))
            .id();
        app.update();
        assert!(app.world().get::<BombFuse>(dropped).is_none());
    }

    /// Throwing it again re-arms with a fresh fuse: a catch disarms, it does
    /// not defuse for good.
    #[test]
    fn re_throwing_a_caught_bomb_arms_it_again() {
        let mut app = App::new();
        app.add_systems(Update, arm_thrown_bombs);
        let bomb = app.world_mut().spawn(bomb_ground(ae::Vec2::ZERO)).id();
        app.update();
        assert!(app.world().get::<BombFuse>(bomb).is_none());

        app.world_mut()
            .entity_mut(bomb)
            .insert(ReleasedAs(Release::Throw));
        app.update();
        assert_eq!(
            app.world().get::<BombFuse>(bomb).map(|fuse| fuse.timer),
            Some(BOMB_FUSE_SECS),
            "a re-thrown bomb gets a FULL fuse, not the remainder of its last one"
        );
    }

    #[derive(Resource, Default)]
    struct CapturedHits(Vec<HitEvent>);

    fn capture_hits(mut reader: MessageReader<HitEvent>, mut out: ResMut<CapturedHits>) {
        out.0.extend(reader.read().cloned());
    }

    #[test]
    fn fuse_expiry_detonates_a_player_side_blast_and_despawns() {
        let mut app = App::new();
        app.add_message::<HitEvent>();
        app.add_message::<ambition_sfx::OwnedSfxMessage>();
        app.add_message::<ambition_vfx::vfx::VfxInRoom>();
        app.add_message::<ambition_platformer2d_shared_tangle::camera_ease::CameraShakeRequest>();
        app.init_resource::<CapturedHits>();
        let mut wt = ambition_time::WorldTime::default();
        wt.set_sim_dt(0.05); // sim_dt() > the 0.001 fuse
        app.insert_resource(wt);
        app.add_systems(Update, (tick_bomb_fuses, capture_hits).chain());
        let bomb = app
            .world_mut()
            .spawn((bomb_ground(ae::Vec2::ZERO), BombFuse { timer: 0.001 }))
            .id();
        app.update();
        assert!(
            app.world().get::<GroundItem>(bomb).is_none(),
            "bomb despawns on detonation"
        );
        let hits = &app.world().resource::<CapturedHits>().0;
        let hit = hits.first().expect("a blast HitEvent was emitted");
        assert_eq!(hit.damage, BOMB_DAMAGE);
        assert!(
            matches!(hit.source, HitSource::Melee),
            "player-side blast (spares the player, hits enemies)"
        );
    }

    /// An app that runs the fuse ticker with every message it writes, and
    /// captures the blasts.
    fn fuse_app() -> App {
        let mut app = App::new();
        app.add_message::<HitEvent>();
        app.add_message::<ambition_sfx::OwnedSfxMessage>();
        app.add_message::<ambition_vfx::vfx::VfxInRoom>();
        app.add_message::<ambition_platformer2d_shared_tangle::camera_ease::CameraShakeRequest>();
        app.init_resource::<CapturedHits>();
        let mut wt = ambition_time::WorldTime::default();
        wt.set_sim_dt(1.0 / 60.0);
        app.insert_resource(wt);
        app.add_systems(Update, (tick_bomb_fuses, capture_hits).chain());
        app
    }

    /// A lit bomb with a full fuse, and this tick's strikes.
    fn lit(app: &mut App, surface: Option<f32>, body: Option<f32>) -> Entity {
        let mut bomb = app.world_mut().spawn((bomb_ground(ae::Vec2::ZERO), BombFuse { timer: BOMB_FUSE_SECS }));
        if let Some(impact_speed) = surface {
            bomb.insert(ItemStruckSurface { impact_speed });
        }
        if let Some(impact_speed) = body {
            bomb.insert(ItemStruckBody { impact_speed, bodies: vec![Entity::PLACEHOLDER] });
        }
        bomb.id()
    }

    /// A hard throw into a wall sets the bomb off where it strikes, with a
    /// full fuse left; a soft landing bounces and cuts the fuse short.
    #[test]
    fn a_hard_strike_on_a_surface_goes_off_and_a_soft_one_cuts_the_fuse() {
        let mut app = fuse_app();
        let hard = lit(&mut app, Some(BOMB_SURFACE_DETONATION_SPEED + 1.0), None);
        let soft = lit(&mut app, Some(BOMB_SURFACE_DETONATION_SPEED * 0.5), None);
        app.update();
        assert!(app.world().get::<GroundItem>(hard).is_none(), "a hard strike did not set the bomb off");
        assert_eq!(app.world().resource::<CapturedHits>().0.len(), 1, "one blast, the hard one's");
        let left = app.world().get::<BombFuse>(soft).expect("a soft landing does not set it off").timer;
        assert!(
            left <= BOMB_FUSE_AFTER_BOUNCE_SECS,
            "the first bounce cuts the fuse to {BOMB_FUSE_AFTER_BOUNCE_SECS} s: {left}"
        );
        // And the short fuse runs out.
        app.world_mut().entity_mut(soft).remove::<ItemStruckSurface>();
        for _ in 0..(BOMB_FUSE_AFTER_BOUNCE_SECS * 60.0) as usize + 2 {
            app.update();
        }
        assert!(app.world().get::<GroundItem>(soft).is_none(), "the cut fuse ran out");
    }

    /// A thrown bomb that reaches a body goes off on it.
    #[test]
    fn a_bomb_that_strikes_a_body_goes_off() {
        let mut app = fuse_app();
        let struck = lit(&mut app, None, Some(BOMB_BODY_DETONATION_SPEED));
        let grazed = lit(&mut app, None, Some(BOMB_BODY_DETONATION_SPEED * 0.5));
        app.update();
        assert!(app.world().get::<GroundItem>(struck).is_none(), "the strike on a body did not set it off");
        assert!(app.world().get::<GroundItem>(grazed).is_some(), "a slow touch set it off");
    }

    /// A bomb in a hand is not set off by a strike: a hand is not a surface.
    #[test]
    fn a_carried_bomb_is_not_set_off_by_a_strike() {
        let mut app = fuse_app();
        let holder = app.world_mut().spawn_empty().id();
        let carried = lit(&mut app, Some(BOMB_SURFACE_DETONATION_SPEED * 2.0), Some(BOMB_BODY_DETONATION_SPEED * 2.0));
        app.world_mut().entity_mut(carried).insert(ItemCustody::Held { holder });
        app.update();
        assert!(app.world().get::<GroundItem>(carried).is_some());
        assert!(app.world().resource::<CapturedHits>().0.is_empty());
    }

    /// Each bomb gets its body before it moves, and no other item does.
    #[test]
    fn a_bomb_is_weighed_and_an_axe_is_not() {
        let mut app = App::new();
        app.add_systems(Update, weigh_bombs);
        let bomb = app.world_mut().spawn(bomb_ground(ae::Vec2::ZERO)).id();
        let axe = app
            .world_mut()
            .spawn(GroundItem::at_rest(
                ambition_characters::brain::held_item_by_id("axe").unwrap(),
                ae::Vec2::ZERO,
                ae::Vec2::splat(14.0),
            ))
            .id();
        app.update();
        assert_eq!(app.world().get::<ItemBody>(bomb), Some(&BOMB_BODY));
        assert_eq!(app.world().get::<ItemBody>(axe), None);
    }
}
