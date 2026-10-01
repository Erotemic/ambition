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

use ambition_platformer2d::engine_core as ae;
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
#[derive(Resource, Default, Clone)]
pub struct SpentMonitors(pub Vec<String>);

impl SpentMonitors {
    /// A checksum over which monitors are spent.
    ///
    /// Order-independent even though this is a `Vec`: peers running the same
    /// simulation break monitors in the same order, so XORing per-name hashes
    /// loses nothing a desync check needs, and it would survive a switch to a
    /// set.
    pub fn checksum(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        self.0.iter().fold(0u64, |acc, name| {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            name.hash(&mut hasher);
            acc ^ hasher.finish()
        })
    }

    fn is_broken(&self, name: &str) -> bool {
        self.0.iter().any(|broken| broken == name)
    }
}

/// The break. A falling player whose feet land on a monitor's lid, or a
/// rolling player overlapping it, breaks it once: burst + cue + the grant.
///
/// Every monitor pops on a roll-through, the classic Sonic feel.
pub fn break_monitor_boxes(
    time: Res<ambition_platformer2d::time::WorldTime>,
    mut spent: ResMut<SpentMonitors>,
    geometry: ambition_platformer2d::platformer::lifecycle::SoleLiveRoom<ae::RoomGeometry>,
    mut vfx: ambition_platformer2d::vfx::VfxWriter,
    mut sfx: ambition_platformer2d::sfx::BodySfxWriter,
    mut players: Query<
        (
            &ae::BodyKinematics,
            &ambition_platformer2d::characters::actor::WornCharacter,
            &mut ae::MotionModel,
            Option<&crate::ball_dash::Rolling>,
            Option<&mut ambition_platformer2d::characters::actor::BodyWallet>,
        ),
        With<PrimaryPlayer>,
    >,
) {
    let Ok((kin, worn, mut model, rolling, mut wallet)) = players.single_mut()
    else {
        return;
    };
    let rolling = rolling.is_some();
    let falling = kin.vel.y > 0.0;
    if !rolling && !falling {
        return;
    }
    let p = kin.aabb();
    // Where a rolling body will be by next tick, plus a little: see `BREAK_REACH`.
    let reach = kin.vel.abs() * time.scaled_dt * 2.0 + ae::Vec2::splat(BREAK_REACH);
    for block in &geometry.0.blocks {
        if block.name.starts_with(BREAKABLE_WALL) && !spent.is_broken(&block.name) {
            let b = block.aabb;
            let near = p.min.x - reach.x < b.max.x
                && p.max.x + reach.x > b.min.x
                && p.min.y - reach.y < b.max.y
                && p.max.y + reach.y > b.min.y;
            if rolling && near {
                spent.0.push(block.name.clone());
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
        if !block.name.starts_with(MONITOR_PREFIX) || spent.is_broken(&block.name) {
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
        spent.0.push(block.name.clone());
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
    // The sole live room's overlay: this content is one room.
    let Some(mut overlay) = overlays.sole() else {
        return;
    };
    overlay.removed_block_names.extend(spent.0.iter().cloned());
}

/// Spent monitors are per-attempt: the next life starts with a full set of
/// boxes.
///
/// Sanic declares `DeathRules::replay_level_after(0.0)`, so a pit death
/// replays the room in place, and an in-place replay does not emit
/// `RoomLoaded`. `AttemptScoped` re-arms on both signals; an implementor
/// names what to re-arm, not which signal counts.
impl ambition_platformer2d::actors::session::reset::AttemptScoped for SpentMonitors {
    /// Any room: both acts author monitors, and you stand in one at a time.
    const ROOM: Option<&'static str> = None;

    fn rearm(&mut self) {
        self.0.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d::world::FeatureEcsWorldOverlay;
    use crate::SPEEDWAY_ROOM_ID;
    use ambition_platformer2d::world::rooms::RoomLoaded;

    #[test]
    fn a_broken_monitor_is_subtracted_from_the_collision_overlay() {
        let mut app = App::new();
        ambition_platformer2d::session::insert_live_room_component(app.world_mut(), FeatureEcsWorldOverlay::default());
        app.insert_resource(SpentMonitors(vec![SPEED_MONITOR.to_string()]));
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

    #[test]
    fn a_reload_rearms_the_monitors() {
        let mut app = App::new();
        app.insert_resource(SpentMonitors(vec![SPEED_MONITOR.to_string()]));
        app.add_message::<RoomLoaded>();
        // Required even though this arm never writes it. A system that reads
        // an unregistered message fails parameter validation and is dropped
        // silently, so the test would pass or fail for an unrelated reason.
        app.add_message::<ambition_platformer2d::combat::events::RoomReplayAdmitted>();
        app.add_systems(
            Update,
            ambition_platformer2d::actors::session::reset::rearm_attempt_scoped::<SpentMonitors>,
        );
        app.world_mut()
            .resource_mut::<bevy::ecs::message::Messages<RoomLoaded>>()
            .write(RoomLoaded {
                room_id: SPEEDWAY_ROOM_ID.to_string(),
            });
        app.update();
        assert!(
            app.world().resource::<SpentMonitors>().0.is_empty(),
            "a level (re)load restocks the monitors"
        );
    }

    /// Each act authors monitors. An arrival starts with whole boxes.
    #[test]
    fn arriving_in_the_other_act_restocks_the_monitors() {
        let mut app = App::new();
        app.insert_resource(SpentMonitors(vec![SPEED_MONITOR.to_string()]));
        app.add_message::<RoomLoaded>();
        app.add_message::<ambition_platformer2d::combat::events::RoomReplayAdmitted>();
        app.add_systems(
            Update,
            ambition_platformer2d::actors::session::reset::rearm_attempt_scoped::<SpentMonitors>,
        );
        app.world_mut()
            .resource_mut::<bevy::ecs::message::Messages<RoomLoaded>>()
            .write(RoomLoaded {
                room_id: crate::HIGHWAY_ROOM_ID.to_string(),
            });
        app.update();
        assert!(
            app.world().resource::<SpentMonitors>().0.is_empty(),
            "Act 2's load restocks the monitors Act 1 spent"
        );
    }

    /// A pit death replays the room in place and never emits `RoomLoaded`
    /// (`DeathRules::replay_level_after(0.0)`), so the replay must re-arm the
    /// monitors. A reload test does not cover this; the two messages differ.
    #[test]
    fn a_death_replay_rearms_the_monitors() {
        let mut app = App::new();
        app.insert_resource(SpentMonitors(vec![SPEED_MONITOR.to_string()]));
        app.add_message::<RoomLoaded>();
        app.add_message::<ambition_platformer2d::combat::events::RoomReplayAdmitted>();
        app.add_systems(
            Update,
            ambition_platformer2d::actors::session::reset::rearm_attempt_scoped::<SpentMonitors>,
        );
        app.world_mut()
            .resource_mut::<bevy::ecs::message::Messages<
                ambition_platformer2d::combat::events::RoomReplayAdmitted,
            >>()
            .write(ambition_platformer2d::combat::events::RoomReplayAdmitted {
                reason: ambition_platformer2d::combat::events::RoomResetReason::PlayerDeath,
                // No controlled body in a rules-only harness; the re-arm is a
                // room-wide restock and does not read the subject.
                subject: None,
            });
        app.update();
        assert!(
            app.world().resource::<SpentMonitors>().0.is_empty(),
            "a death replay must restock the monitors; only a room LOAD did"
        );
    }

    /// Nothing re-arms them when neither signal fires. Otherwise the arms
    /// above would pass on a system that clears every frame, which would give
    /// an infinite supply mid-run.
    #[test]
    fn a_quiet_frame_leaves_broken_monitors_broken() {
        let mut app = App::new();
        app.insert_resource(SpentMonitors(vec![SPEED_MONITOR.to_string()]));
        app.add_message::<RoomLoaded>();
        app.add_message::<ambition_platformer2d::combat::events::RoomReplayAdmitted>();
        app.add_systems(
            Update,
            ambition_platformer2d::actors::session::reset::rearm_attempt_scoped::<SpentMonitors>,
        );
        app.update();
        assert_eq!(
            app.world().resource::<SpentMonitors>().0.len(),
            1,
            "a broken monitor must stay broken until the room reloads or replays"
        );
    }
}
