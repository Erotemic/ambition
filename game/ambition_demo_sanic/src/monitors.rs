//! Monitor boxes: Sanic's power-up crates, pure content on two engine seams.
//!
//! A monitor is an LDtk-authored named solid block (`monitor_*`); the demo
//! identifies each by its block name, like Mary-O's `GeoId` bonks but without
//! the contact seam. It breaks on Sanic's verbs: land on it while falling, or
//! touch it while rolling. (A riding body never sweeps against solid blocks,
//! code smell #13, so an unrolled runner passes through.)
//!
//! A broken monitor is removed from the World the established way: its name
//! joins the collision overlay's per-frame `removed_block_names` (the authored
//! base is never edited), so it stops colliding and, through the render
//! reconcile, stops drawing. It re-arms on room load and on replay.
//!
//! Grants, by name prefix (a block name is its identity, so each monitor in a
//! level has its own suffix):
//! - `monitor_speed…` → speed shoes: a timed multiplier on the body's own
//!   `MomentumParams` (top speed and ground accel), restored exactly on
//!   expiry. Skipped while super, because the form's params come from its
//!   identity row.
//! - `monitor_rings…` → ten rings, the classic stash behind a secret.
//!
//! There is no super monitor: the transformation lives only on the Utility
//! action (`toggle_sanic_form`).

use bevy::prelude::*;

use ambition_platformer2d::actors::session::reset::PerLiveRoom;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::platformer::lifecycle::LiveRoomInstance;
use ambition_platformer2d::platformer::markers::PrimaryPlayer;

use crate::SUPER_SANIC_CHARACTER_ID;

/// Authored block-name prefix that marks a block as a monitor.
pub const MONITOR_PREFIX: &str = "monitor_";
/// Act 1's speed monitor (block name in the LDtk file), and the prefix every
/// speed monitor's name starts with.
pub const SPEED_MONITOR: &str = "monitor_speed";
/// The prefix of a wall a rolling Sanic smashes through: the classic door to
/// a secret. A solid block like any other until he rolls into it.
pub const BREAKABLE_WALL: &str = "breakable_";
/// How far ahead of his box (px, beyond this tick's travel) a rolling Sanic
/// breaks a wall. A rider stopped by a wall keeps none of his speed, so the
/// wall has to go the tick BEFORE he would meet it.
const BREAK_REACH: f32 = 12.0;
/// The prefix of a ring monitor's block name.
pub const RING_MONITOR: &str = "monitor_rings";
/// Rings a ring monitor holds.
pub const RING_MONITOR_RINGS: i32 = 10;

/// How long the speed shoes last (sim seconds) and what they multiply.
const SPEED_SHOES_SECONDS: f32 = 8.0;
const SPEED_SHOES_TOP_SPEED_FACTOR: f32 = 1.4;
const SPEED_SHOES_ACCEL_FACTOR: f32 = 1.5;

/// Vertical tolerance (px) for "feet on the monitor's lid".
const STOMP_BAND: f32 = 16.0;

/// Which monitors (and breakable walls) are broken this run. A Vec, not a HashSet: the overlay
/// iterates it every frame, and the sim determinism contract bans std-hash
/// iteration order.
/// `Clone` because it is rollback state: the overlay subtracts these names
/// from collision every frame, so a rewind that does not restore the set
/// disagrees with the world about which monitors are solid.
///
/// Keyed by live room: two live rooms can author a monitor with one name (two
/// instances of one act), and a monitor broken in one is whole in the other.
#[derive(Resource, Default, Clone)]
pub struct SpentMonitors(PerLiveRoom<Vec<String>>);

impl SpentMonitors {
    /// A checksum over which monitors are spent.
    ///
    /// Order-independent even though this is a `Vec`: peers running the same
    /// simulation break monitors in the same order, so XORing per-name hashes
    /// loses nothing a desync check needs, and it would survive a switch to a
    /// set. Each name is hashed with its room.
    pub fn checksum(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        self.0.iter().fold(0u64, |acc, (room, names)| {
            names.iter().fold(acc, |acc, name| {
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                room.ordinal().hash(&mut hasher);
                name.hash(&mut hasher);
                acc ^ hasher.finish()
            })
        })
    }

    /// Whether this monitor (or breakable wall) of `room` is broken.
    pub fn is_broken(&self, room: LiveRoomInstance, name: &str) -> bool {
        self.spent_in(room).iter().any(|broken| broken == name)
    }

    /// The broken names of `room`, in the order they broke.
    pub fn spent_in(&self, room: LiveRoomInstance) -> &[String] {
        self.0.in_room(room).map_or(&[], Vec::as_slice)
    }

    /// Break this monitor of `room`. Idempotent.
    pub fn spend(&mut self, room: LiveRoomInstance, name: &str) {
        if !self.is_broken(room, name) {
            self.0.in_room_mut(room).push(name.to_string());
        }
    }
}

/// The break. A falling player whose feet land on a monitor's lid, or a
/// rolling player overlapping it, breaks it once: burst + cue + the grant.
///
/// Every monitor pops on a roll-through, the classic Sonic feel.
pub fn break_monitor_boxes(
    time: Res<ambition_platformer2d::time::WorldTime>,
    mut spent: ResMut<SpentMonitors>,
    // The boxes of the live room the body is in. A break is drawn in that
    // room.
    geometry: ambition_platformer2d::platformer::lifecycle::LiveRoomOf<ae::RoomGeometry>,
    mut vfx: ambition_platformer2d::vfx::VfxWriter,
    mut sfx: ambition_platformer2d::sfx::BodySfxWriter,
    mut players: Query<
        (
            Entity,
            &ae::BodyKinematics,
            &ambition_platformer2d::characters::actor::WornCharacter,
            &mut ae::MotionModel,
            Option<&crate::ball_dash::Rolling>,
            Option<&mut ambition_platformer2d::characters::actor::BodyWallet>,
        ),
        With<PrimaryPlayer>,
    >,
) {
    let Ok((player, kin, worn, mut model, rolling, mut wallet)) = players.single_mut()
    else {
        return;
    };
    let (Some(room), Some(room_geometry)) = (geometry.room_of(player), geometry.of(player)) else {
        return;
    };
    let mut vfx = vfx.for_room(Some(room));
    let rolling = rolling.is_some();
    let falling = kin.vel.y > 0.0;
    if !rolling && !falling {
        return;
    }
    let p = kin.aabb();
    // Where a rolling body will be by next tick, plus a little: see `BREAK_REACH`.
    let reach = kin.vel.abs() * time.sim_dt() * 2.0 + ae::Vec2::splat(BREAK_REACH);
    for block in &room_geometry.0.blocks {
        if block.name.starts_with(BREAKABLE_WALL) && !spent.is_broken(room, &block.name) {
            let b = block.aabb;
            let near = p.min.x - reach.x < b.max.x
                && p.max.x + reach.x > b.min.x
                && p.min.y - reach.y < b.max.y
                && p.max.y + reach.y > b.min.y;
            if rolling && near {
                spent.spend(room, &block.name);
                let center = (b.min + b.max) * 0.5;
                vfx.write(ambition_platformer2d::vfx::VfxMessage::Burst {
                    pos: center,
                    count: 24,
                    speed: 220.0,
                    color: [0.45, 0.62, 0.70, 1.0],
                    kind: ambition_platformer2d::vfx::ParticleKind::Shard,
                });
                sfx.write_from(
                    crate::provider::SANIC_EXPERIENCE,
                    ambition_platformer2d::sfx::SfxMessage::Play {
                        id: ambition_platformer2d::sfx::SfxId::from_static(crate::SFX_MONITOR),
                        pos: center,
                    },
                );
            }
            continue;
        }
        if !block.name.starts_with(MONITOR_PREFIX) || spent.is_broken(room, &block.name) {
            continue;
        }
        let b = block.aabb;
        let overlap_x = p.min.x < b.max.x && p.max.x > b.min.x;
        let overlap_y = p.min.y < b.max.y && p.max.y > b.min.y;
        let feet = p.max.y;
        let stomp =
            falling && overlap_x && feet >= b.min.y - STOMP_BAND && feet <= b.min.y + STOMP_BAND;
        let roll = rolling && overlap_x && overlap_y;
        if !(stomp || roll) {
            continue;
        }
        spent.spend(room, &block.name);
        let center = (b.min + b.max) * 0.5;
        vfx.write(ambition_platformer2d::vfx::VfxMessage::Burst {
            pos: center,
            count: 16,
            speed: 170.0,
            color: [0.55, 0.75, 0.95, 1.0],
            kind: ambition_platformer2d::vfx::ParticleKind::Shard,
        });
        // The monitor's own pop. H2/I3: a monitor is a prop in this course,
        // so the sound is the course's, not the host's. The breaker's own cue
        // (roll, stomp bounce) is emitted by the breaker.
        sfx.write_from(
            crate::provider::SANIC_EXPERIENCE,
            ambition_platformer2d::sfx::SfxMessage::Play {
                id: ambition_platformer2d::sfx::SfxId::from_static(crate::SFX_MONITOR),
                pos: center,
            },
        );
        match block.name.as_str() {
            name if name.starts_with(RING_MONITOR) => {
                if let Some(wallet) = wallet.as_deref_mut() {
                    wallet.add(RING_MONITOR_RINGS);
                }
            }
            name if name.starts_with(SPEED_MONITOR) => {
                // The shoes are a boost on the momentum the body rides: the
                // kernel folds it into the authored params and spends it, so
                // a second pair only restarts the clock. The super form
                // authors its own speed, so it takes no shoes.
                if worn.id() != SUPER_SANIC_CHARACTER_ID {
                    if let ae::MotionModel::SurfaceMomentum(momentum) = &mut *model {
                        momentum.boost = Some(ae::MomentumBoost {
                            top_speed_scale: SPEED_SHOES_TOP_SPEED_FACTOR,
                            ground_accel_scale: SPEED_SHOES_ACCEL_FACTOR,
                            remaining_s: SPEED_SHOES_SECONDS,
                        });
                    }
                }
            }
            other => {
                // An authored monitor with no grant is a level-authoring bug.
                debug_assert!(false, "monitor block '{other}' has no authored grant");
                bevy::log::error!(
                    target: "ambition_platformer2d::sanic",
                    "monitor block '{other}' has no authored grant; breaking it \
                     does nothing"
                );
            }
        }
    }
}

/// Contribute each broken monitor's authored name to the collision overlay's
/// per-frame `removed_block_names` — the engine's immutable-base subtraction
/// seam. Runs after the overlay rebuild clears the list (its clean-slate
/// contract), the same slot Mary-O's bricks take.
pub fn contribute_broken_monitors_to_overlay(
    spent: Res<SpentMonitors>,
    mut overlays: ambition_platformer2d::world::RoomOverlays,
) {
    // Each room's monitors go into that room's own overlay.
    for (room, names) in spent.0.iter() {
        let stamp = ambition_platformer2d::platformer::lifecycle::InRoomInstance(room);
        let Some(mut overlay) = overlays.for_room(Some(&stamp)) else {
            continue;
        };
        overlay.removed_block_names.extend(names.iter().cloned());
    }
}

/// Spent monitors are per-attempt: the next life starts with a full set of
/// boxes.
///
/// Sanic declares `DeathRules::replay_level_after(0.0)`, so a pit death
/// replays the room in place, and an in-place replay does not emit
/// `RoomLoaded`. It does seat a new live room, and the state of the room it
/// replaces goes with that room.
impl ambition_platformer2d::actors::session::reset::AttemptScoped for SpentMonitors {
    type Attempt = Vec<String>;

    fn attempts(&self) -> &PerLiveRoom<Self::Attempt> {
        &self.0
    }

    fn attempts_mut(&mut self) -> &mut PerLiveRoom<Self::Attempt> {
        &mut self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d::world::FeatureEcsWorldOverlay;
    use ambition_platformer2d::platformer::lifecycle::spawn_live_room;

    fn spent(room: LiveRoomInstance, name: &str) -> SpentMonitors {
        let mut spent = SpentMonitors::default();
        spent.spend(room, name);
        spent
    }

    #[test]
    fn a_broken_monitor_is_subtracted_from_the_collision_overlay() {
        let mut app = App::new();
        ambition_platformer2d::session::insert_live_room_component(app.world_mut(), FeatureEcsWorldOverlay::default());
        app.insert_resource(spent(LiveRoomInstance::ACTIVATION, SPEED_MONITOR));
        app.add_systems(Update, contribute_broken_monitors_to_overlay);
        app.update();
        let removed = &ambition_platformer2d::session::sole_live_room_component::<FeatureEcsWorldOverlay>(app
            .world()).expect("the live room has a collision overlay")
            .removed_block_names;
        assert!(
            removed.contains(&SPEED_MONITOR.to_string()),
            "broken monitors are named in removed_block_names: {removed:?}"
        );
    }

    /// Two live rooms: the monitor broken in Bob's room is subtracted from
    /// Bob's overlay only. Before, the contribution wrote the sole live room's
    /// overlay, so while two rooms were live no broken monitor left the
    /// collision world.
    #[test]
    fn each_live_room_subtracts_only_its_own_broken_monitors() {
        let mut app = App::new();
        let alices = ambition_platformer2d::session::insert_live_room_component(
            app.world_mut(),
            FeatureEcsWorldOverlay::default(),
        );
        let bob = LiveRoomInstance::ACTIVATION.next();
        let bobs = spawn_live_room(app.world_mut(), bob, FeatureEcsWorldOverlay::default());
        app.insert_resource(spent(bob, SPEED_MONITOR));
        app.add_systems(Update, contribute_broken_monitors_to_overlay);
        app.update();
        let removed = |root| {
            app.world()
                .get::<FeatureEcsWorldOverlay>(root)
                .expect("each live room has an overlay")
                .removed_block_names
                .clone()
        };
        assert_eq!(
            (removed(alices), removed(bobs)),
            (Vec::<String>::new(), vec![SPEED_MONITOR.to_string()]),
            "(Alice's room, Bob's room): each room subtracts its own broken monitors"
        );
    }

    /// A death replay or a load seats a new live room, so the replaced room's
    /// monitors go with it, and the new room has a full set. Bob's room keeps
    /// its own. A quiet frame (no room replaced) restocks nothing: otherwise
    /// this would pass on a system that cleared every frame, which would give
    /// an infinite supply mid-run.
    #[test]
    fn a_replaced_room_takes_its_spent_monitors_with_it() {
        let mut app = App::new();
        let alices = ambition_platformer2d::session::insert_live_room_component(
            app.world_mut(),
            FeatureEcsWorldOverlay::default(),
        );
        let bob = LiveRoomInstance::ACTIVATION.next();
        spawn_live_room(app.world_mut(), bob, FeatureEcsWorldOverlay::default());
        let mut both = spent(LiveRoomInstance::ACTIVATION, SPEED_MONITOR);
        both.spend(bob, SPEED_MONITOR);
        app.insert_resource(both);
        app.add_systems(
            Update,
            ambition_platformer2d::actors::session::reset::rearm_attempt_scoped::<SpentMonitors>,
        );
        app.update();
        assert!(
            app.world().resource::<SpentMonitors>().is_broken(LiveRoomInstance::ACTIVATION, SPEED_MONITOR),
            "a quiet frame restocked a live room's monitors"
        );
        let replayed = bob.next();
        app.world_mut().entity_mut(alices).insert(replayed);
        app.update();
        let spent = app.world().resource::<SpentMonitors>();
        assert_eq!(
            (
                spent.is_broken(LiveRoomInstance::ACTIVATION, SPEED_MONITOR),
                spent.is_broken(replayed, SPEED_MONITOR),
                spent.is_broken(bob, SPEED_MONITOR),
            ),
            (false, false, true),
            "(the replaced room, its replay, Bob's room): the replay has a full set and Bob's monitor stays broken"
        );
    }
}
