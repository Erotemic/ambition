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

use crate::features::{HitEvent, HitMode, HitSource, HitTarget};
use crate::items::pickup::GroundItem;
use ambition_engine_core as ae;

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

/// Arm any thrown bomb: a `bomb` [`GroundItem`] that is moving (just thrown) and
/// not yet armed gets a lit [`BombFuse`]. A resting debug bomb (never thrown)
/// stays unarmed so the player can pick it up safely.
pub fn arm_thrown_bombs(
    mut commands: Commands,
    bombs: Query<(Entity, &GroundItem), Without<BombFuse>>,
) {
    for (entity, ground) in &bombs {
        if ground.spec.id == BOMB_ID && ground.vel != ae::Vec2::ZERO {
            commands.entity(entity).insert(BombFuse {
                timer: BOMB_FUSE_SECS,
            });
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
    mut sfx: ambition_sfx::SfxWriter,
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
            volume: ae::CombatVolume::circle(ground.pos, BOMB_BLAST_HALF),
            damage: BOMB_DAMAGE,
            source: HitSource::PlayerSlash { knock_x: 0.0 },
            attacker: None,
            target: HitTarget::Volume,
            mode: HitMode::Knockback,
            knockback: None,
            ignored_targets: Vec::new(),
        });
        sfx.write(ambition_sfx::SfxMessage::Play {
            id: ambition_sfx::ids::WORLD_ROCK_HIT,
            pos: ground.pos,
        });
        vfx.write(ambition_vfx::vfx::VfxMessage::Explosion {
            pos: ground.pos,
            kind: ambition_vfx::vfx::ExplosionKind::ClassicBurst,
            scale: 1.0,
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

    #[test]
    fn a_thrown_bomb_arms_but_a_resting_one_does_not() {
        let mut app = App::new();
        app.add_systems(Update, arm_thrown_bombs);
        let thrown = app
            .world_mut()
            .spawn(bomb_ground(ae::Vec2::new(60.0, -200.0)))
            .id();
        let resting = app.world_mut().spawn(bomb_ground(ae::Vec2::ZERO)).id();
        app.update();
        assert!(
            app.world().get::<BombFuse>(thrown).is_some(),
            "thrown bomb arms"
        );
        assert!(
            app.world().get::<BombFuse>(resting).is_none(),
            "resting bomb stays safe"
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
            matches!(hit.source, HitSource::PlayerSlash { .. }),
            "player-side blast (spares the player, hits enemies)"
        );
    }
}
