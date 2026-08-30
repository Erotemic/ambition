//! Bomb — a thrown held item that explodes on a fuse, damaging enemies in a
//! radius via a player-side AABB [`HitEvent`].
//!
//! Reuses the held-item substrate: `bomb` is a "pure throwable" `HeldItemSpec`
//! (no melee/ranged verb), so a plain `Attack` throws it and it arcs under
//! gravity as a [`GroundItem`] (see `item_pickup`). The frame it starts moving
//! (i.e. has been thrown) it gets a [`BombFuse`]; when the fuse burns out it
//! emits a `PlayerSlash`-source explosion that damages enemies/bosses in the
//! blast AABB (the existing damage loop spares the player since the source is
//! player-side) and despawns. A resting debug bomb never arms until thrown.

use bevy::prelude::*;

use ambition_combat::events::{HitEvent, HitMode, HitSource, HitTarget};
use crate::items::pickup::{GroundItem, Release, ReleasedAs};
use ambition_platformer2d_core as ae;

/// Held-item id the bomb grants.
pub const BOMB_ID: &str = "bomb";

/// Seconds from being thrown to detonation.
pub const BOMB_FUSE_SECS: f32 = 0.9;
/// Blast half-extent (AABB), px.
const BOMB_BLAST_HALF: f32 = 80.0;
/// Explosion damage.
const BOMB_DAMAGE: i32 = 4;

/// Lit fuse on an airborne/thrown bomb. Counts down to detonation.
#[derive(Component, Clone, Copy, Debug)]
pub struct BombFuse {
    pub timer: f32,
}

/// A `bomb` [`GroundItem`] is armed exactly while it is in the world because
/// somebody THREW it, and this is where that becomes true in both directions.
///
/// ⛔⛔ IT USED TO ASK THE VELOCITY, and a velocity does not know who moved it:
/// `ground.vel != ZERO` armed any bomb the room authored at rest the instant
/// `ground_item_physics` gave it gravity. Ordinary falling read as "a player
/// threw this". The reverse failed too — catching an armed bomb zeroed the
/// velocity and left the lit fuse, and the ticker did not care whose hand it was
/// in, so it counted down and went off in custody.
///
/// ⭐ [`ReleasedAs`] IS THE FACT, and `Release::Throw` versus `Release::Drop` was
/// already decided by the one release transaction — it just had nowhere to say
/// so. A Z-drop is handing the item to the floor, not an attack, so it does not
/// arm; that was the old answer too, but only because a drop happens to launch
/// at zero velocity.
///
/// ⚠ AND DISARMING IS THE SAME SYSTEM, deliberately. It is chained ahead of
/// [`tick_bomb_fuses`], so a bomb caught this tick has its fuse removed before
/// anything can burn it down — catching a live bomb is a defined outcome, not a
/// race between two systems. Re-throwing re-arms with a fresh fuse, which is
/// what `Without`-style arming already implied.
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

/// Burn down lit fuses; on detonation emit a player-side blast [`HitEvent`]
/// (damages enemies/bosses in the AABB, not the player) and despawn the bomb.
pub fn tick_bomb_fuses(
    time: Res<ambition_time::WorldTime>,
    mut commands: Commands,
    mut bombs: Query<(Entity, &GroundItem, &mut BombFuse)>,
    mut hits: MessageWriter<HitEvent>,
    mut sfx: ambition_sfx::BodySfxWriter,
    mut vfx: MessageWriter<ambition_vfx::vfx::VfxMessage>,
) {
    let dt = time.sim_dt();
    if dt <= 0.0 {
        return;
    }
    for (entity, ground, mut fuse) in &mut bombs {
        fuse.timer -= dt;
        if fuse.timer > 0.0 {
            continue;
        }
        // Detonate: a broadcast player-side hit over the blast radius — a real
        // disc, so the blast is radial (corners of the old square no longer hit).
        hits.write(HitEvent {
            strike_sfx: None,
            volume: ae::CombatVolume::circle(ground.pos, BOMB_BLAST_HALF),
            damage: BOMB_DAMAGE,
            source: HitSource::Melee,
            attacker: None,
            target: HitTarget::Volume,
            mode: HitMode::Knockback,
            knockback: None,
            ignored_targets: Vec::new(),
        });
        // The bomb inherits its thrower's source at spawn, so the blast is the
        // thrower's cue — and falls back to the session when nothing stamped it.
        sfx.write_for(
            entity,
            ambition_sfx::SfxMessage::Play {
                id: ambition_sfx::ids::WORLD_ROCK_HIT,
                pos: ground.pos,
            },
        );
        vfx.write(ambition_vfx::vfx::VfxMessage::Effect {
            pos: ground.pos,
            fx: ambition_vfx::fx::ids::CLASSIC_BURST,
            scale: 1.0,
            pose: ambition_vfx::FxPose::UPRIGHT,
        });
        commands.entity(entity).despawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bomb_ground(vel: ae::Vec2) -> GroundItem {
        GroundItem {
            spec: ambition_characters::brain::held_item_by_id(BOMB_ID).unwrap(),
            pos: ae::Vec2::new(100.0, 100.0),
            vel,
            half_extent: ae::Vec2::splat(14.0),
        }
    }

    /// ⭐ THE ORIGINAL CONTRACT, ASKED OF THE RIGHT FACT. This arm used to spawn
    /// a bomb with a nonzero velocity and call it "thrown"; the velocity was the
    /// heuristic under test, so the test agreed with the bug by construction.
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

    /// ⛔⛔ THE FIRST HALF OF THE DEFECT. A bomb the room authored begins at rest
    /// and then FALLS: `ground_item_physics` gives it a velocity, and the old
    /// arming read that as evidence a player threw it. Nobody threw it; it is
    /// just subject to gravity like everything else.
    #[test]
    fn a_falling_bomb_nobody_threw_does_not_arm_itself() {
        let mut app = App::new();
        app.add_systems(Update, arm_thrown_bombs);
        // The velocity a second of ordinary free-fall gives it. Under the old
        // rule this is indistinguishable from a throw.
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

    /// ⛔⛔ THE SECOND HALF, and the one that kills you. Catching a live bomb
    /// zeroed its velocity and left the lit fuse; the ticker did not care whose
    /// hand it was in, so it counted down and detonated in custody.
    ///
    /// ⚠ `arm_thrown_bombs` is CHAINED AHEAD of `tick_bomb_fuses` in the shipped
    /// schedule, so the disarm lands before anything can burn the fuse down —
    /// this arm runs them in that order for the same reason.
    #[test]
    fn catching_a_live_bomb_puts_the_fuse_out() {
        let mut app = App::new();
        app.add_message::<HitEvent>();
        app.add_message::<ambition_sfx::OwnedSfxMessage>();
        app.add_message::<ambition_vfx::vfx::VfxMessage>();
        let mut wt = ambition_time::WorldTime::default();
        wt.scaled_dt = 0.05;
        app.insert_resource(wt);
        app.add_systems(Update, (arm_thrown_bombs, tick_bomb_fuses).chain());

        // In the air with a fuse almost out, exactly as a caught bomb is.
        let caught = app
            .world_mut()
            .spawn((
                bomb_ground(ae::Vec2::ZERO),
                ReleasedAs(Release::Throw),
                BombFuse { timer: 0.001 },
            ))
            .id();
        // A body takes custody: the release is over, so its record is retracted.
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

    /// A Z-drop is handing the item to the floor, not an attack. The old rule
    /// agreed, but only because a drop happens to launch at zero velocity —
    /// which is not a reason.
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

    /// And throwing it again re-arms with a fresh fuse, which is what a catch
    /// being a DISARM rather than a permanent defusal means.
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
        app.add_message::<ambition_vfx::vfx::VfxMessage>();
        app.init_resource::<CapturedHits>();
        let mut wt = ambition_time::WorldTime::default();
        wt.scaled_dt = 0.05; // sim_dt() > the 0.001 fuse
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
}
