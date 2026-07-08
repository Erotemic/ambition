//! Overflow Crash — a player-wielded **lunge strike**: dash forward along the
//! aim and skewer everything in the dash corridor. The wielded kit's only
//! *offensive mobility* attack — [`crate::abilities::ranged::shockwave`] / [`crate::abilities::ranged::beam`] /
//! [`crate::abilities::ranged::volley`] are all stationary, and while [`crate::abilities::traversal::blink`] also
//! teleports, blink is a *defensive* reposition (a tiny poof at the arrival
//! point); the dive is an *offensive* gap-closer whose damage is the whole
//! **path** from start to landing. Close the distance and cut a line through
//! the mob in one commit.
//!
//! It is the **overflow** boss signature gauntlet — an aerial dive-bomber that
//! bursts past its bounds and crashes into you. Defeat it, wield its crash
//! yourself ("every boss a failed objective function, learn its attack").
//!
//! Mechanically it reuses two proven primitives: [`crate::platformer_runtime::collision::raycast_solids`]
//! (the same wall-stop the blink uses, so the lunge never lands inside geometry)
//! and a one-shot `Player`-faction [`crate::features::HitEvent`] over the dash
//! corridor (a `PlayerSlash` source, so it damages enemies and spares the
//! player). A *one-shot* event — not a lingering `Hitbox` — because a dash hits
//! at the instant it crosses, it doesn't leave a damaging box behind.
//!
//! The lunge axis-snaps (dominant aim axis, defaulting to facing) so the
//! corridor stays a clean thin rectangle; a rotated dash is a feel follow-up.

use bevy::prelude::*;

use ambition_platformer_primitives::markers::ControlledSubject;
use crate::actor::BodyKinematics;
use crate::actor::BodyMana;
use crate::features::HeldItem;
use ambition_characters::brain::ActorControl;
use ambition_engine_core::{self as ae, AabbExt};

/// Held-item id of the dive gauntlet.
pub const DIVE_ID: &str = "dive";

/// Mana the dive spends per lunge (out of 100). A committed gap-closer — gated
/// like the rest of the wielded kit so it can't be spammed across a room.
const DIVE_MANA_COST: f32 = 26.0;

/// How far (px) the player lunges along the aim, absent a wall.
const DIVE_LUNGE: f32 = 140.0;
/// Half-thickness (px) of the damaging corridor swept by the lunge.
const DIVE_WIDTH: f32 = 48.0;
/// Damage dealt to everything in the corridor.
const DIVE_DAMAGE: i32 = 4;
/// Horizontal shove imparted to struck enemies (signed by the lunge direction).
const DIVE_KNOCKBACK: f32 = 1.4;

/// Axis-snap an aim + facing to the lunge direction (a unit vector along the
/// dominant axis). A null aim falls back to `facing` (a forward dash), so a
/// plain Attack with no directional hold still lunges — it's an attack, not a
/// precise teleport like the blink (which needs an explicit aim).
fn dive_dir(aim: ae::Vec2, facing: f32) -> ae::Vec2 {
    let horizontal = if aim == ae::Vec2::ZERO {
        true
    } else {
        aim.x.abs() >= aim.y.abs()
    };
    if horizontal {
        let s = if aim.x.abs() > 0.001 {
            aim.x.signum()
        } else {
            facing.signum()
        };
        ae::Vec2::new(s, 0.0)
    } else {
        ae::Vec2::new(0.0, aim.y.signum())
    }
}

/// The damaging corridor swept from `from` to `to` — an axis-aligned box that
/// bounds both endpoints, padded by a body-width so the dash has thickness. For
/// an axis-snapped lunge this is a clean thin rectangle along the dash.
fn dive_corridor(from: ae::Vec2, to: ae::Vec2) -> ae::Aabb {
    let center = (from + to) * 0.5;
    let half = ae::Vec2::new(
        (to.x - from.x).abs() * 0.5 + DIVE_WIDTH * 0.5,
        (to.y - from.y).abs() * 0.5 + DIVE_WIDTH * 0.5,
    );
    ae::Aabb::new(center, half)
}

/// `Attack` while holding the dive gauntlet lunges the player along the aim and
/// emits a one-shot `Player`-faction hit over the dash corridor. Plain Attack
/// only — `Shield + Attack` drops the item (the id is `UseSystem`, excluded from
/// throw-on-plain-Attack in `throw_held_item_system`).
pub fn fire_dive_system(
    gravity: crate::physics::GravityCtx,
    world: crate::features::CollisionWorld,
    // Ability ORIGIN = the controlled subject, not a `PrimaryPlayer` filter.
    controlled: Res<ControlledSubject>,
    mut players: Query<(
        Entity,
        &ActorControl,
        &mut BodyKinematics,
        &HeldItem,
        &mut BodyMana,
    )>,
    mut sfx: MessageWriter<ambition_sfx::SfxMessage>,
    mut hits: MessageWriter<crate::features::HitEvent>,
) {
    let Some(subject) = controlled.0 else {
        return;
    };
    let Ok((player, control, mut kin, held, mut mana)) = players.get_mut(subject) else {
        return;
    };
    let c = control.0;
    if !c.melee_pressed || c.shield_held {
        return;
    }
    if held.spec.id != DIVE_ID {
        return;
    }
    if !mana.meter.try_spend(DIVE_MANA_COST) {
        return;
    }
    let gravity_dir = gravity.dir_at(kin.pos);
    let frame = ae::AccelerationFrame::new(gravity_dir);
    let local_aim = crate::items::pickup::ability_aim_local(&c, kin.facing);
    let local_dir = dive_dir(local_aim, kin.facing).normalize_or_zero();
    let dir = frame.to_world(local_dir).normalize_or_zero();
    let from = kin.pos;
    // Stop a body-half short of the wall so the lunge never embeds. The pull-back
    // must use the body's extent IN THE LUNGE DIRECTION -- half-height for a
    // vertical dive, not half-width -- the same direction-aware clamp the blink
    // uses (or a down/diagonal dive embeds in the floor and trips the OOB detector).
    let half = kin.size * 0.5;
    let margin = (half.x * dir.x.abs() + half.y * dir.y.abs()) + 2.0;
    // One composited collision view, shared by the clamp raycast and the embed
    // safety net, so the lunge is stopped by moving platforms / ECS solids too.
    let collision = world.solids();
    let mut target = match collision.as_ref().and_then(|w| {
        crate::platformer_runtime::collision::raycast_solids(
            &**w,
            from,
            dir,
            DIVE_LUNGE + margin,
            false,
        )
    }) {
        Some((hit, _normal)) => hit - dir * margin,
        None => from + dir * DIVE_LUNGE,
    };
    // Safety net: if the landing AABB still overlaps a solid (a corner / grazing
    // the center-ray missed), fall back to the start instead of embedding.
    if let Some(w) = collision.as_ref() {
        let landing = ae::Aabb::new(target, half);
        let embeds = w.blocks.iter().any(|b| {
            matches!(
                b.kind,
                ae::BlockKind::Solid | ae::BlockKind::BlinkWall { .. }
            ) && landing.strict_intersects(b.aabb)
        });
        if embeds {
            target = from;
        }
    }
    kin.pos = target;
    if local_dir.x.abs() > 0.001 {
        kin.facing = local_dir.x.signum();
    }
    // The dash corridor cuts everything between start and landing — a one-shot
    // PlayerSlash volume (spares the player, shoves enemies along the dash).
    hits.write(crate::features::HitEvent {
        volume: dive_corridor(from, target).into(),
        damage: DIVE_DAMAGE,
        source: crate::features::HitSource::PlayerSlash {
            knock_x: local_dir.x * DIVE_KNOCKBACK,
        },
        attacker: Some(player),
        target: crate::features::HitTarget::Volume,
        mode: crate::features::HitMode::Knockback,
        knockback: None,
        ignored_targets: Vec::new(),
    });
    sfx.write(ambition_sfx::SfxMessage::Play {
        id: ambition_sfx::ids::PLAYER_BLINK,
        pos: target,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abilities::test_support::spawn_primary_player_holding;

    fn test_app() -> App {
        let mut app = App::new();
        app.add_message::<ambition_sfx::SfxMessage>();
        app.add_message::<crate::features::HitEvent>();
        app.add_systems(Update, fire_dive_system);
        app
    }

    #[derive(bevy::prelude::Resource, Default)]
    struct CapturedHits(Vec<crate::features::HitEvent>);

    fn capture_hits(
        mut reader: bevy::prelude::MessageReader<crate::features::HitEvent>,
        mut out: bevy::prelude::ResMut<CapturedHits>,
    ) {
        out.0.extend(reader.read().cloned());
    }

    #[test]
    fn dive_lunges_the_player_forward_and_cuts_a_corridor() {
        let mut app = test_app();
        app.init_resource::<CapturedHits>();
        app.add_systems(Update, capture_hits.after(fire_dive_system));
        let player = spawn_primary_player_holding(&mut app, DIVE_ID);
        app.world_mut()
            .get_mut::<ActorControl>(player)
            .unwrap()
            .0
            .melee_pressed = true;
        app.update();
        // No world → no walls → full lunge along facing (+x).
        let pos = app.world().get::<BodyKinematics>(player).unwrap().pos;
        assert!(
            (pos.x - (100.0 + DIVE_LUNGE)).abs() < 0.01,
            "player lunged a full DIVE_LUNGE forward: {pos:?}"
        );
        let hits = &app.world().resource::<CapturedHits>().0;
        assert_eq!(hits.len(), 1, "one corridor hit emitted");
        assert_eq!(hits[0].damage, DIVE_DAMAGE);
        assert!(
            matches!(
                hits[0].source,
                crate::features::HitSource::PlayerSlash { .. }
            ),
            "player-side source so it spares the player",
        );
        // The corridor spans the dash: from start (100) to landing (240) along x.
        assert!(
            hits[0].volume.bounds().min.x <= 100.0
                && hits[0].volume.bounds().max.x >= 100.0 + DIVE_LUNGE,
            "corridor covers start..landing: {:?}",
            hits[0].volume
        );
    }

    #[test]
    fn downward_dive_does_not_embed_in_the_floor() {
        // Regression (same class as the blink fix): a vertical lunge must clamp by
        // the body's half-HEIGHT, not half-width, or a down dive embeds in the floor.
        let mut app = test_app();
        let player = spawn_primary_player_holding(&mut app, DIVE_ID); // (100,100), 24x40
        app.insert_resource(ambition_engine_core::RoomGeometry(ae::World::new(
            "test",
            ae::Vec2::new(600.0, 600.0),
            ae::Vec2::new(100.0, 100.0),
            vec![ae::Block::solid(
                "floor",
                ae::Vec2::new(0.0, 200.0),
                ae::Vec2::new(600.0, 400.0),
            )],
        )));
        {
            let mut control = app.world_mut().get_mut::<ActorControl>(player).unwrap();
            control.0.melee_pressed = true;
            control.0.aim = ae::Vec2::new(0.0, 1.0); // brain-resolved local aim: down
        }
        app.update();
        let pos = app.world().get::<BodyKinematics>(player).unwrap().pos;
        assert!(
            pos.y + 20.0 <= 200.0 + 1e-3,
            "downward dive embedded the body in the floor: bottom={}, floor top=200",
            pos.y + 20.0,
        );
        assert!(
            pos.y > 100.0,
            "the dive should still carry the player downward"
        );
    }

    #[test]
    fn no_dive_without_attack_or_item() {
        let mut app = test_app();
        app.init_resource::<CapturedHits>();
        app.add_systems(Update, capture_hits.after(fire_dive_system));
        let player = spawn_primary_player_holding(&mut app, DIVE_ID);
        app.update(); // no attack pressed
        assert_eq!(app.world().resource::<CapturedHits>().0.len(), 0);
        assert_eq!(
            app.world().get::<BodyKinematics>(player).unwrap().pos.x,
            100.0,
            "no lunge without an attack press"
        );
    }

    #[test]
    fn dive_costs_mana_and_is_blocked_when_empty() {
        let mut app = test_app();
        app.init_resource::<CapturedHits>();
        app.add_systems(Update, capture_hits.after(fire_dive_system));
        let player = spawn_primary_player_holding(&mut app, DIVE_ID);
        app.world_mut()
            .get_mut::<BodyMana>(player)
            .unwrap()
            .meter
            .current = 5.0;
        app.world_mut()
            .get_mut::<ActorControl>(player)
            .unwrap()
            .0
            .melee_pressed = true;
        app.update();
        assert_eq!(
            app.world().resource::<CapturedHits>().0.len(),
            0,
            "no dive when mana < cost"
        );
        assert_eq!(
            app.world().get::<BodyKinematics>(player).unwrap().pos.x,
            100.0,
            "and no lunge either"
        );

        app.world_mut()
            .get_mut::<BodyMana>(player)
            .unwrap()
            .meter
            .current = 100.0;
        app.update();
        assert_eq!(
            app.world().resource::<CapturedHits>().0.len(),
            1,
            "fires once there's mana"
        );
    }

    #[test]
    fn dive_dir_snaps_to_the_dominant_axis() {
        // Engine y grows downward, so "up" is -y.
        assert_eq!(
            dive_dir(ae::Vec2::new(0.0, -1.0), 1.0),
            ae::Vec2::new(0.0, -1.0)
        );
        assert_eq!(
            dive_dir(ae::Vec2::new(1.0, 0.0), 1.0),
            ae::Vec2::new(1.0, 0.0)
        );
        // Null aim falls back to facing.
        assert_eq!(dive_dir(ae::Vec2::ZERO, -1.0), ae::Vec2::new(-1.0, 0.0));
        // Dominant axis wins on a diagonal.
        assert_eq!(
            dive_dir(ae::Vec2::new(0.3, -0.9), 1.0),
            ae::Vec2::new(0.0, -1.0)
        );
    }

    #[test]
    fn dive_corridor_is_a_thin_rectangle_spanning_the_dash() {
        // A horizontal dash: long along x, thin along y.
        let c = dive_corridor(ae::Vec2::new(100.0, 100.0), ae::Vec2::new(240.0, 100.0));
        assert!(c.min.x <= 100.0 && c.max.x >= 240.0, "spans the dash on x");
        let half_y = (c.max.y - c.min.y) * 0.5;
        let half_x = (c.max.x - c.min.x) * 0.5;
        assert!(
            half_x > half_y,
            "horizontal corridor is long along x: {c:?}"
        );
    }
}
