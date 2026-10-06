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
//!
//! Each kind with a grant wears its box (`dress_monitors`, `dress_monitor_boxes`):
//! the renderer's item monitor with the power-up's icon on its screen, which
//! idles, breaks, and leaves its smashed shell. The block itself draws nothing.

use bevy::prelude::*;

use ambition_platformer2d::actors::session::reset::PerLiveRoom;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::platformer::lifecycle::LiveRoomInstance;
use ambition_platformer2d::platformer::markers::PlayerEntity;

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

/// The box art of each monitor kind that grants something: a prop sheet
/// (`targets/props/sanic_powerup_*.py` in the renderer), registered under its
/// own target name. The renderer draws an invincibility box too; this game
/// has no invincibility monitor, so it wears none.
pub const SPEED_BOX_SPRITE: &str = "sanic_powerup_speed";
pub const RING_BOX_SPRITE: &str = "sanic_powerup_rings";

/// The box sheet a monitor block wears, by the same name prefixes its grant
/// is chosen by.
pub fn box_sprite_of(block_name: &str) -> Option<&'static str> {
    if block_name.starts_with(SPEED_MONITOR) {
        Some(SPEED_BOX_SPRITE)
    } else if block_name.starts_with(RING_MONITOR) {
        Some(RING_BOX_SPRITE)
    } else {
        None
    }
}

/// The box sheet's rows: it idles whole, plays its break once, then shows the
/// smashed shell.
const BREAK_ROW: &str = "break";
const BROKEN_ROW: &str = "broken";

/// Dress every monitor of `room` in its box: the block draws nothing (its
/// collision is untouched) and a prop of its box sheet stands on it.
///
/// The sheet's own body box says where the box is in its frame: its lid
/// (the box's top) and its ground line (its bottom) land on the block's lid and
/// floor, so a stomp lands on the drawn lid and the box stands on the road. The
/// block is a 26 px square; the box is narrower than it at that height, which
/// only the collision knows. A sheet the build did not bake (sprites not
/// generated) leaves the block as it was: a plain tile.
pub fn dress_monitors(room: &mut ambition_platformer2d::world::rooms::RoomSpec) {
    for block in &mut room.world.blocks {
        let Some(kind) = box_sprite_of(&block.name) else {
            continue;
        };
        let Some(record) = ambition_platformer2d::sprite_sheet::character::sheets::record_for_sheet_key(kind) else {
            continue;
        };
        let Some(body) = record.body_metrics.as_ref().and_then(|metrics| metrics.body_pixel_bbox) else {
            continue;
        };
        let frame = ae::Vec2::new(record.frame_width as f32, record.frame_height as f32);
        let (lid, ground) = (body.y as f32, (body.y + body.h) as f32);
        let centre_x = body.x as f32 + body.w as f32 * 0.5;
        let aabb = block.aabb;
        let scale = (aabb.max.y - aabb.min.y) / (ground - lid);
        // The frame's centre, carried from the frame into the world through
        // the box's ground line and centre column.
        let pos = ae::Vec2::new(
            (aabb.min.x + aabb.max.x) * 0.5 + (frame.x * 0.5 - centre_x) * scale,
            aabb.max.y + (frame.y * 0.5 - ground) * scale,
        );
        block.art_color = Some([0.0, 0.0, 0.0, 0.0]);
        room.props.push(ambition_platformer2d::world::rooms::PropSpec {
            id: format!("{}_box", block.name),
            // The monitor's own name, which is what `SpentMonitors` keys on.
            name: block.name.clone(),
            kind: kind.to_string(),
            pos,
            size: frame * scale,
            flip_y: false,
            // The frame is drawn exactly over its box: built world, behind the
            // runner.
            draw: ambition_platformer2d::world::rooms::PropDraw::Structure,
        });
    }
}

/// The row a monitor's box shows (`None`: its idle), from whether the monitor
/// is broken, the row it shows now, and whether that row has played out.
fn box_row(broken: bool, showing: Option<&str>, played_out: bool) -> Option<&'static str> {
    match (broken, showing) {
        (false, _) => None,
        (true, Some(BROKEN_ROW)) => Some(BROKEN_ROW),
        (true, Some(BREAK_ROW)) if played_out => Some(BROKEN_ROW),
        (true, _) => Some(BREAK_ROW),
    }
}

/// Show each monitor's box as its state says: whole while it stands, then its
/// break, then the smashed shell.
///
/// Derived from [`SpentMonitors`] every frame, not from the break: that set is
/// rollback state, so a box broken on a frame a rewind throws away stands
/// again, which a look driven by the break event would not.
pub fn dress_monitor_boxes(
    mut commands: Commands,
    spent: Res<SpentMonitors>,
    rooms: ambition_platformer2d::platformer::lifecycle::LiveRooms,
    boxes: Query<(
        Entity,
        &ambition_platformer2d::render::rendering::PropVisual,
        &ambition_platformer2d::sprite_sheet::character::CharacterAnimator,
        Option<&ambition_platformer2d::render::rendering::PropClip>,
    )>,
) {
    use ambition_platformer2d::render::rendering::PropClip;
    for (entity, prop, animator, clip) in &boxes {
        if box_sprite_of(&prop.name) != Some(prop.kind.as_str()) {
            continue;
        }
        let broken = rooms.of(entity).is_some_and(|room| spent.is_broken(room, &prop.name));
        let showing = clip.map(|clip| clip.0.clip.as_str());
        let want = box_row(broken, showing, animator.clip_finished());
        if want == showing {
            continue;
        }
        match want {
            Some(row) => {
                let request = ambition_platformer2d::sim_view::ClipRequest::from_chain(&[row]).expect("one row");
                commands.entity(entity).insert(PropClip(request));
            }
            None => {
                commands.entity(entity).remove::<PropClip>();
            }
        }
    }
}

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
            Option<&ambition_platformer2d::platformer::sim_id::SimId>,
        ),
        // Every body of the player population breaks monitors, each in its
        // own live room, so a second seat breaks them too.
        With<PlayerEntity>,
    >,
) {
    // Two bodies can reach one monitor in one tick. The first breaks it and
    // takes its grant, so the order is a gameplay decision and a rewind must
    // give the same order: stable `SimId`, not query order.
    let breakers = ambition_platformer2d::platformer::sim_selection::in_deterministic_order(
        players.iter_mut(),
        |_| 0.0,
        |(_, _, _, _, _, _, id)| *id,
    );
    for (player, kin, worn, mut model, rolling, mut wallet, _) in breakers {
        let (Some(room), Some(room_geometry)) = (geometry.room_of(player), geometry.of(player)) else {
            continue;
        };
        let mut vfx = vfx.for_room(Some(room));
        let rolling = rolling.is_some();
        let falling = kin.vel.y > 0.0;
        if !rolling && !falling {
            continue;
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

    /// Every monitor of every act wears the box of its grant, standing on it:
    /// the sheet's lid on the block's lid, the sheet's ground line on the
    /// block's floor, and the block itself drawn as nothing.
    #[test]
    fn every_monitor_wears_the_box_of_its_grant() {
        let mut dressed = 0;
        for room in [crate::sanic_speedway(), crate::sanic_highway(), crate::sanic_darkness()] {
            for block in room.world.blocks.iter().filter(|block| block.name.starts_with(MONITOR_PREFIX)) {
                let kind = box_sprite_of(&block.name).unwrap_or_else(|| panic!("{} grants nothing", block.name));
                let prop = room
                    .props
                    .iter()
                    .find(|prop| prop.name == block.name)
                    .unwrap_or_else(|| panic!("{} in {} wears no box", block.name, room.id));
                let record = ambition_platformer2d::sprite_sheet::character::sheets::record_for_sheet_key(kind)
                    .unwrap_or_else(|| panic!("the build baked no `{kind}` sheet: regen the sprites"));
                let body = record.body_metrics.as_ref().and_then(|m| m.body_pixel_bbox).expect("the box sheet states its body");
                let scale = prop.size.y / record.frame_height as f32;
                let frame_top = prop.pos.y - prop.size.y * 0.5;
                let (lid, ground) = (frame_top + body.y as f32 * scale, frame_top + (body.y + body.h) as f32 * scale);
                assert_eq!(prop.kind, kind, "{}", block.name);
                assert!(
                    (lid - block.aabb.min.y).abs() < 0.01 && (ground - block.aabb.max.y).abs() < 0.01,
                    "{}: the box's lid {lid} and ground {ground} are not the block's {:?}",
                    block.name,
                    block.aabb
                );
                assert_eq!(block.art_color, Some([0.0; 4]), "{} still draws its tile", block.name);
                dressed += 1;
            }
        }
        assert_eq!(dressed, 8, "the premise: the three acts author eight monitors");
    }

    /// A box idles while its monitor stands, plays its break once when it
    /// breaks, then holds the shell; a rewind that un-breaks it stands it up
    /// whole again, from either row.
    #[test]
    fn a_box_idles_breaks_then_holds_its_shell_and_a_rewind_stands_it_up() {
        assert_eq!(box_row(false, None, false), None, "a standing monitor idles");
        assert_eq!(box_row(true, None, false), Some(BREAK_ROW), "it breaks");
        assert_eq!(box_row(true, Some(BREAK_ROW), false), Some(BREAK_ROW), "the break plays out");
        assert_eq!(box_row(true, Some(BREAK_ROW), true), Some(BROKEN_ROW), "then the shell");
        assert_eq!(box_row(true, Some(BROKEN_ROW), true), Some(BROKEN_ROW), "the shell stays");
        assert_eq!(box_row(false, Some(BREAK_ROW), false), None, "a rewind mid-break");
        assert_eq!(box_row(false, Some(BROKEN_ROW), true), None, "a rewind after it");
    }

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

    /// Two live rooms, each with one ring monitor at the same place. Returns
    /// the app and the second room.
    fn two_room_monitor_app() -> (App, LiveRoomInstance) {
        let world = ae::World::new(
            "monitor fixture",
            ae::Vec2::new(640.0, 480.0),
            ae::Vec2::new(32.0, 400.0),
            vec![ae::Block::solid(RING_MONITOR, ae::Vec2::new(100.0, 200.0), ae::Vec2::splat(32.0))],
        );
        let mut app = App::new();
        app.init_resource::<SpentMonitors>();
        app.init_resource::<ambition_platformer2d::time::WorldTime>();
        app.add_message::<ambition_platformer2d::vfx::VfxInRoom>();
        app.add_message::<ambition_platformer2d::sfx::OwnedSfxMessage>();
        ambition_platformer2d::session::insert_live_room_component(app.world_mut(), ae::RoomGeometry(world.clone()));
        let second = LiveRoomInstance::ACTIVATION.next();
        spawn_live_room(app.world_mut(), second, ae::RoomGeometry(world));
        app.add_systems(Update, break_monitor_boxes);
        (app, second)
    }

    /// A seat's body in `room`, falling onto the monitor's lid this tick,
    /// with an empty wallet.
    fn landing_seat(
        slot: u8,
        room: LiveRoomInstance,
    ) -> impl Bundle {
        (
            ambition_platformer2d::platformer::lifecycle::InRoomInstance(room),
            PlayerEntity,
            // Feet at y 204, inside the stomp band of the lid at y 200.
            ae::BodyKinematics {
                pos: ae::Vec2::new(116.0, 184.0),
                vel: ae::Vec2::new(0.0, 120.0),
                size: ae::Vec2::new(20.0, 40.0),
                facing: 1.0,
            },
            ambition_platformer2d::characters::actor::WornCharacter::new("sanic"),
            ae::MotionModel::default(),
            ambition_platformer2d::characters::actor::BodyWallet { balance: 0 },
            ambition_platformer2d::platformer::sim_id::SimId::player_slot(slot),
        )
    }

    fn rings(app: &App, body: Entity) -> i32 {
        app.world()
            .get::<ambition_platformer2d::characters::actor::BodyWallet>(body)
            .expect("the seat has a wallet")
            .balance
    }

    /// Alice, the primary seat, and Bob, a second seat, land on the ring
    /// monitor of their own live rooms in one tick. Each room's monitor breaks
    /// and each seat takes the rings. Alice's half is the control: the system
    /// ran for the primary seat in the same tick.
    #[test]
    fn a_second_seat_breaks_the_monitor_of_its_own_room() {
        let (mut app, bobs_room) = two_room_monitor_app();
        let alice = app
            .world_mut()
            .spawn((
                landing_seat(0, LiveRoomInstance::ACTIVATION),
                ambition_platformer2d::platformer::markers::PrimaryPlayer,
            ))
            .id();
        let bob = app.world_mut().spawn(landing_seat(1, bobs_room)).id();
        app.update();
        let spent = app.world().resource::<SpentMonitors>();
        assert_eq!(
            (
                spent.is_broken(LiveRoomInstance::ACTIVATION, RING_MONITOR),
                spent.is_broken(bobs_room, RING_MONITOR),
                rings(&app, alice),
                rings(&app, bob),
            ),
            (true, true, RING_MONITOR_RINGS, RING_MONITOR_RINGS),
            "(Alice's monitor broken, Bob's monitor broken, Alice's rings, Bob's rings)"
        );
    }

    /// Two seats land on one monitor in one room in one tick. It pays once, to
    /// the seat whose `SimId` sorts first, whatever order the bodies were
    /// spawned in. Bob is spawned first, so query order would pay him.
    #[test]
    fn two_seats_on_one_monitor_pay_the_first_seat_by_sim_id() {
        let (mut app, _) = two_room_monitor_app();
        let room = LiveRoomInstance::ACTIVATION;
        let bob = app.world_mut().spawn(landing_seat(1, room)).id();
        let alice = app.world_mut().spawn(landing_seat(0, room)).id();
        app.update();
        assert_eq!(
            (rings(&app, alice), rings(&app, bob)),
            (RING_MONITOR_RINGS, 0),
            "(slot 0's rings, slot 1's rings): one payout, to the first seat by SimId"
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
