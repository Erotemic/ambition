//! A conducted boss: a module that performs a boss's moves itself — where the
//! body goes, what it swings, what it is drawn as — every tick, while the
//! boss's pattern decides WHICH move and WHEN.

use ambition_extension_sdk::wire::{self, WireError, WireReader};
use ambition_extension_sdk::{Port, PortKey, PortRole};

/// The trigger port marker for one tick of a conducted boss.
///
/// Port card (`docs/planning/engine/extension-domain-contracts.md`):
///
/// * **Operation** — a boss lives this tick. The entry's selector is the
///   boss's behaviour id (`BossConfig::behavior.id`): the kind of boss, not
///   one placement of it.
/// * **Owner** — `ambition_boss_encounter::extension`.
/// * **Scope** — one invocation for EACH boss whose id is bound, EACH tick
///   with a gameplay step greater than zero, in boss query order.
/// * **Time** — `boss_conduct`: after every non-boss body integrated; the
///   pose a module asks for is the last word before combat reads it.
/// * **Read model** — world units, +Y down. `side` is the side the conductor
///   last chose (`ConductedPose::side`), `1.0` or `-1.0`; at birth it is the
///   side the body was built facing. `telegraph` and `active` are the boss
///   pattern's live `Special` moves and the seconds left in each. `hall` is
///   the floor under the boss and the walls either side of it, in the boss's
///   OWN live room, measured this tick.
///   `holding` says the boss holds a body (`ambition.combat.body_hold`):
///   false once it mashed free, was thrown or released. `minions` counts the
///   living bodies of the boss's own encounter that are not bosses (its
///   summons), in its live room. `between_phases` says the encounter holds
///   its beat between two phases (a transition lock, or the `Transition`
///   phase): the boss is invulnerable and its pattern runs no move. `room`
///   is the size of the boss's live room (its origin is the top left): a
///   boss with no floor under it (one that flies an open sky) stages its
///   moves against the room itself. `health` is the boss's current health.
///   `invulnerable` says the encounter refuses damage to the boss now (a
///   transition lock, or the `Dormant`, `Intro`, `Transition` or `Death`
///   phase): it is wider than `between_phases`. `target_body` is where the
///   tracked body is now and its size; `target` is where the boss last saw
///   it, and the two differ when the body moved after the boss looked. Its
///   `swing` is the move the tracked body plays now (a boss that reads a
///   blow guards it, or strikes when it is over): `None` when it plays none.
///   `size` is the boss's own body size (its feet are `position.y + size.y / 2`).
///   `awake` says the encounter has left `Dormant`: a boss whose encounter
///   has a wake radius sleeps, and its conductor stages its sleep, until a
///   player comes that near.
/// * **Absence** — `target` is `None` when the boss tracks nothing, and
///   `target_body` also when the tracked entity has no body; `hall`
///   is `None` when the room cannot be told or has no floor under the boss;
///   `room` is `None` when the room cannot be told.
/// * **Replay** — derived each tick from rollback state.
pub struct BossConductPort;

/// A floor and the walls either side of it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RoomHall {
    /// The floor's top surface.
    pub floor: f32,
    /// Inner faces of the side walls.
    pub left: f32,
    pub right: f32,
}

/// A live `Special` move of the boss's pattern.
#[derive(Clone, Debug, PartialEq)]
pub struct LiveMove {
    pub key: String,
    /// Seconds the pattern says remain in this part.
    pub remaining: f32,
}

/// The boss, at the read cut.
#[derive(Clone, Debug, PartialEq)]
pub struct BossConduct {
    pub position: [f32; 2],
    pub velocity: [f32; 2],
    /// The sign of the body's facing.
    pub facing: f32,
    pub side: f32,
    pub alive: bool,
    pub telegraph: Option<LiveMove>,
    pub active: Option<LiveMove>,
    pub target: Option<[f32; 2]>,
    pub hall: Option<RoomHall>,
    /// A participant drives the boss.
    pub driven: bool,
    /// The encounter is in its enrage phase.
    pub enraged: bool,
    /// The boss holds a body.
    pub holding: bool,
    /// Living summons of its encounter, in its live room.
    pub minions: u32,
    /// The encounter holds its beat between two phases: no move runs.
    pub between_phases: bool,
    /// The size of the boss's live room.
    pub room: Option<[f32; 2]>,
    /// The boss's current health.
    pub health: i32,
    /// The encounter refuses damage to the boss now.
    pub invulnerable: bool,
    /// The tracked body, now.
    pub target_body: Option<BodyBox>,
    /// The boss's own body size.
    pub size: [f32; 2],
    /// The encounter has woken: it left `Dormant` (a boss with a wake
    /// radius sleeps until a player comes that near).
    pub awake: bool,
}

/// A body's centre and size, and the move it plays.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BodyBox {
    pub position: [f32; 2],
    pub size: [f32; 2],
    pub swing: Option<BodySwing>,
}

/// The move a body plays, as one who watches it sees it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BodySwing {
    /// Seconds since the move started.
    pub since: f32,
    /// Seconds until it can first hurt. 0 when it can hurt now, when it is
    /// past that, and when it never hurts.
    pub hurts_in: f32,
    /// It can hurt now.
    pub live: bool,
    /// Seconds until the move is over.
    pub ends_in: f32,
}

fn put_move(out: &mut Vec<u8>, m: Option<&LiveMove>) {
    wire::put_bool(out, m.is_some());
    if let Some(m) = m {
        wire::put_str(out, &m.key);
        wire::put_f32(out, m.remaining);
    }
}

fn move_of(r: &mut WireReader<'_>) -> Result<Option<LiveMove>, WireError> {
    if !r.bool()? {
        return Ok(None);
    }
    Ok(Some(LiveMove {
        key: r.str()?.to_owned(),
        remaining: r.f32()?,
    }))
}

impl Port for BossConductPort {
    /// Version 2 (2026-10-06): `holding` and `minions`. Version 3
    /// (2026-10-06): `between_phases`. Version 4 (2026-10-06): `room`.
    /// Version 5 (2026-10-10): `health`, `invulnerable`, `target_body`.
    /// Version 6 (2026-10-10): `size`. Version 7 (2026-10-12): the
    /// `swing` of `target_body`. Version 8 (2026-10-18): `awake`.
    const KEY: PortKey = PortKey::new("ambition.boss.conduct", 8);
    const ROLE: PortRole = PortRole::Trigger;
    type Value = BossConduct;

    fn encode(v: &BossConduct, out: &mut Vec<u8>) {
        wire::put_vec2(out, v.position);
        wire::put_vec2(out, v.velocity);
        wire::put_f32(out, v.facing);
        wire::put_f32(out, v.side);
        wire::put_bool(out, v.alive);
        put_move(out, v.telegraph.as_ref());
        put_move(out, v.active.as_ref());
        wire::put_opt(out, v.target, wire::put_vec2);
        wire::put_bool(out, v.hall.is_some());
        if let Some(h) = v.hall {
            wire::put_f32(out, h.floor);
            wire::put_f32(out, h.left);
            wire::put_f32(out, h.right);
        }
        wire::put_bool(out, v.driven);
        wire::put_bool(out, v.enraged);
        wire::put_bool(out, v.holding);
        wire::put_u32(out, v.minions);
        wire::put_bool(out, v.between_phases);
        wire::put_opt(out, v.room, wire::put_vec2);
        wire::put_i32(out, v.health);
        wire::put_bool(out, v.invulnerable);
        wire::put_opt(out, v.target_body, |out, b| {
            wire::put_vec2(out, b.position);
            wire::put_vec2(out, b.size);
            wire::put_opt(out, b.swing, |out, s| {
                wire::put_f32(out, s.since);
                wire::put_f32(out, s.hurts_in);
                wire::put_bool(out, s.live);
                wire::put_f32(out, s.ends_in);
            });
        });
        wire::put_vec2(out, v.size);
        wire::put_bool(out, v.awake);
    }

    fn decode(r: &mut WireReader<'_>) -> Result<BossConduct, WireError> {
        Ok(BossConduct {
            position: r.vec2()?,
            velocity: r.vec2()?,
            facing: r.f32()?,
            side: r.f32()?,
            alive: r.bool()?,
            telegraph: move_of(r)?,
            active: move_of(r)?,
            target: r.opt(WireReader::vec2)?,
            hall: if r.bool()? {
                Some(RoomHall {
                    floor: r.f32()?,
                    left: r.f32()?,
                    right: r.f32()?,
                })
            } else {
                None
            },
            driven: r.bool()?,
            enraged: r.bool()?,
            holding: r.bool()?,
            minions: r.u32()?,
            between_phases: r.bool()?,
            room: r.opt(WireReader::vec2)?,
            health: r.i32()?,
            invulnerable: r.bool()?,
            target_body: r.opt(|r| {
                Ok(BodyBox {
                    position: r.vec2()?,
                    size: r.vec2()?,
                    swing: r.opt(|r| Ok(BodySwing { since: r.f32()?, hurts_in: r.f32()?, live: r.bool()?, ends_in: r.f32()? }))?,
                })
            })?,
            size: r.vec2()?,
            awake: r.bool()?,
        })
    }
}

/// The request port marker for where a conducted boss is, and which side it
/// faces.
///
/// Port card:
///
/// * **Operation** — with a `pose`, put the boss there with that velocity
///   (an external kinematic constraint, ADR 0024) and hold its pose: the
///   body integrator leaves its locomotion alone (`PoseOwnedExternally`).
///   Without one, release the pose to whoever drives the body. Either way,
///   a `side` is the side the boss faces while its pose is held, and the side
///   the next tick's trigger reports; with no `side` the conductor leaves the
///   boss's facing to whoever drives it (a rider: GNU-ton's scholar).
/// * **Version 2** (2026-10-10): `side` is optional.
/// * **Owner** — `ambition_boss_encounter::extension`.
/// * **Scope** — the boss the invocation ran for. A scope that is not a boss
///   is refused.
/// * **Time** — `boss_conduct`, this tick.
pub struct ConductedPosePort;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pose {
    pub position: [f32; 2],
    pub velocity: [f32; 2],
}

#[derive(Clone, Debug, PartialEq)]
pub struct ConductedPose {
    pub pose: Option<Pose>,
    pub side: Option<f32>,
}

impl Port for ConductedPosePort {
    const KEY: PortKey = PortKey::new("ambition.boss.conducted_pose", 2);
    const ROLE: PortRole = PortRole::Request;
    type Value = ConductedPose;

    fn encode(v: &ConductedPose, out: &mut Vec<u8>) {
        wire::put_bool(out, v.pose.is_some());
        if let Some(p) = v.pose {
            wire::put_vec2(out, p.position);
            wire::put_vec2(out, p.velocity);
        }
        wire::put_opt(out, v.side, wire::put_f32);
    }

    fn decode(r: &mut WireReader<'_>) -> Result<ConductedPose, WireError> {
        let pose = if r.bool()? {
            Some(Pose {
                position: r.vec2()?,
                velocity: r.vec2()?,
            })
        } else {
            None
        };
        Ok(ConductedPose { pose, side: r.opt(WireReader::f32)? })
    }
}

/// The request port marker for a boss's guard: whether its hull turns blows.
///
/// Port card:
///
/// * **Operation** — guard the boss (`guarded`), or drop its guard. A
///   guarded boss takes no HP from melee or projectile hits: they clang off
///   with a spark, the way a puzzle boss's do. The guard holds until changed,
///   so a conductor states it every tick it has an opinion. The Mockingbird
///   keeps its guard up out of reach and drops it when it dives in to bite.
/// * **Owner** — `ambition_boss_encounter::extension` (`BossEncounter::guarded`).
/// * **Scope** — the boss the invocation ran for. A scope that is not a boss
///   is refused.
/// * **Time** — `boss_conduct`, this tick: before combat reads it.
pub struct BossGuardPort;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BossGuard {
    pub guarded: bool,
}

impl Port for BossGuardPort {
    const KEY: PortKey = PortKey::new("ambition.boss.guard", 1);
    const ROLE: PortRole = PortRole::Request;
    type Value = BossGuard;

    fn encode(v: &BossGuard, out: &mut Vec<u8>) {
        wire::put_bool(out, v.guarded);
    }

    fn decode(r: &mut WireReader<'_>) -> Result<BossGuard, WireError> {
        Ok(BossGuard { guarded: r.bool()? })
    }
}

/// The request port marker for a boss's size.
///
/// Port card:
///
/// * **Operation** — draw and build the boss at `scale` times the size it was
///   built at: its drawn quad, its body box and its hurt volumes, with its
///   feet where they were. The size holds until changed, so a conductor that
///   grows its boss (a serum, a transformation) states the scale each tick it
///   changes it. `1.0` is the size the boss was built at.
/// * **Owner** — `ambition_boss_encounter::conduct` (`BossEncounter::scale`).
/// * **Scope** — the boss the invocation ran for. A scope that is not a boss
///   is refused.
/// * **Time** — `boss_conduct`, this tick: before movement and combat read
///   the body.
pub struct BossScalePort;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BossScale {
    pub scale: f32,
}

impl Port for BossScalePort {
    const KEY: PortKey = PortKey::new("ambition.boss.scale", 1);
    const ROLE: PortRole = PortRole::Request;
    type Value = BossScale;

    fn encode(v: &BossScale, out: &mut Vec<u8>) {
        wire::put_f32(out, v.scale);
    }

    fn decode(r: &mut WireReader<'_>) -> Result<BossScale, WireError> {
        Ok(BossScale { scale: r.f32()? })
    }
}

/// The request port marker for what a boss stages around it: how lit its
/// room is, a flash over the room, and its name card.
///
/// Port card:
///
/// * **Operation** — light the boss's live room at `lit` (0 dark, 1 as the
///   room is lit), flash it white at `flash` (0 none, 1 full), and show the
///   boss's name card (its name and title, large on the screen) at `card` (0
///   hidden, 1 shown). Presentation only: no body, hit or move reads it.
///   Holds until changed, so a conductor states it each tick it changes it.
///   A boss that never states it stands in a lit room with no card.
/// * **Owner** — `ambition_boss_encounter::conduct` (`BossEncounter::stage`);
///   drawn by `ambition_render` (`rendering::boss_stage`).
/// * **Scope** — the boss the invocation ran for. A scope that is not a boss
///   is refused.
/// * **Time** — `boss_conduct`, this tick; drawn this frame.
pub struct BossStagePort;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BossStage {
    pub lit: f32,
    pub flash: f32,
    pub card: f32,
}

impl Default for BossStage {
    /// A lit room, no flash, no card.
    fn default() -> Self {
        Self { lit: 1.0, flash: 0.0, card: 0.0 }
    }
}

impl Port for BossStagePort {
    const KEY: PortKey = PortKey::new("ambition.boss.stage", 1);
    const ROLE: PortRole = PortRole::Request;
    type Value = BossStage;

    fn encode(v: &BossStage, out: &mut Vec<u8>) {
        wire::put_f32(out, v.lit);
        wire::put_f32(out, v.flash);
        wire::put_f32(out, v.card);
    }

    fn decode(r: &mut WireReader<'_>) -> Result<BossStage, WireError> {
        Ok(BossStage { lit: r.f32()?, flash: r.f32()?, card: r.f32()? })
    }
}

/// The request port marker for the row a body is drawn with.
///
/// Port card:
///
/// * **Operation** — draw the part with the first of `names` its sheet has,
///   `elapsed` seconds in, looping or holding its last frame; with no names,
///   draw it as the engine would. Presentation: no simulation state reads it,
///   and it is derived again each tick (the module's own clock is the elapsed
///   time).
/// * **Owner** — `ambition_boss_encounter::conduct` (`PinnedRow`).
/// * **Scope** — the body the invocation ran for, the mount it rides, or a
///   limb of that mount's rig ([`DrawnPart`]). A part that is not there is
///   refused with a warning.
/// * **Time** — `boss_conduct`.
pub struct DrawnRowPort;

/// The part a [`DrawnRow`] draws.
#[derive(Clone, Debug, PartialEq)]
pub enum DrawnPart {
    /// The body the invocation ran for.
    Body,
    /// The mount that body rides.
    Mount,
    /// The limb in this slot of that mount's rig.
    Limb(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct DrawnRow {
    pub on: DrawnPart,
    /// Rows in order of preference: a sheet without the first is drawn with
    /// the next.
    pub names: Vec<String>,
    pub elapsed: f32,
    pub looping: bool,
}

impl DrawnRow {
    /// Draw the body with one row.
    pub fn body(name: &str, elapsed: f32, looping: bool) -> Self {
        Self { on: DrawnPart::Body, names: vec![name.into()], elapsed, looping }
    }

    /// Draw the body as the engine would.
    pub fn body_as_engine() -> Self {
        Self { on: DrawnPart::Body, names: Vec::new(), elapsed: 0.0, looping: false }
    }
}

impl Port for DrawnRowPort {
    const KEY: PortKey = PortKey::new("ambition.presentation.drawn_row", 2);
    const ROLE: PortRole = PortRole::Request;
    type Value = DrawnRow;

    fn encode(v: &DrawnRow, out: &mut Vec<u8>) {
        match &v.on {
            DrawnPart::Body => wire::put_u8(out, 0),
            DrawnPart::Mount => wire::put_u8(out, 1),
            DrawnPart::Limb(slot) => {
                wire::put_u8(out, 2);
                wire::put_str(out, slot);
            }
        }
        wire::put_u32(out, v.names.len() as u32);
        for name in &v.names {
            wire::put_str(out, name);
        }
        wire::put_f32(out, v.elapsed);
        wire::put_bool(out, v.looping);
    }

    fn decode(r: &mut WireReader<'_>) -> Result<DrawnRow, WireError> {
        let on = match r.u8()? {
            0 => DrawnPart::Body,
            1 => DrawnPart::Mount,
            2 => DrawnPart::Limb(r.str()?.to_owned()),
            other => return Err(WireError::BadTag(other)),
        };
        let count = r.read_len()?;
        let mut names = Vec::new();
        for _ in 0..count {
            names.push(r.str()?.to_owned());
        }
        Ok(DrawnRow { on, names, elapsed: r.f32()?, looping: r.bool()? })
    }
}

fn put_hall(out: &mut Vec<u8>, hall: Option<RoomHall>) {
    wire::put_opt(out, hall, |out, h| {
        wire::put_f32(out, h.floor);
        wire::put_f32(out, h.left);
        wire::put_f32(out, h.right);
    });
}

fn hall_of(r: &mut WireReader<'_>) -> Result<Option<RoomHall>, WireError> {
    r.opt(|r| Ok(RoomHall { floor: r.f32()?, left: r.f32()?, right: r.f32()? }))
}

/// The observation port marker for the mount a conducted boss rides.
///
/// Port card:
///
/// * **Operation** — the body the boss rides (its `RidingOn`): where it is,
///   how fast it moves, its size, the side it faces and whether its art
///   turns with it; its back, when its character authors one
///   (`ambition_mount::BackLedge`), as a world box; the floor and walls
///   under it in its own live room; and its limbs (its `LimbRig`), each with
///   where it is, its size, where it rests about the mount
///   (`Limb::home_offset`), the health it lost since it was last full, and
///   whether the rider holds its pose (`PoseOwnedExternally`).
/// * **Owner** — `ambition_boss_encounter::conduct`.
/// * **Scope** — the boss the invocation ran for.
/// * **Time** — `boss_conduct`, at the read cut: after the riders are
///   synced to their mounts.
/// * **Read model** — world units, +Y down; a box is `[min x, min y, max x,
///   max y]`. Limbs are in the rig's slot order.
/// * **Absence** — no value when the boss rides nothing, or its mount has no
///   body. `back` is `None` for a mount whose character authors no back;
///   `hall` is `None` when the room cannot be told or has no floor under it.
/// * **Replay** — derived each tick from rollback state.
pub struct MountViewPort;

/// One limb of the mount.
#[derive(Clone, Debug, PartialEq)]
pub struct MountLimb {
    /// The rig's slot name (`hand_left`, `hand_right`).
    pub slot: String,
    pub position: [f32; 2],
    pub size: [f32; 2],
    /// Where it rests, as an offset from the mount's position.
    pub home_offset: [f32; 2],
    /// Its maximum health less its health now.
    pub lost_health: i32,
    /// The rider holds its pose.
    pub held: bool,
    /// Where it touches something solid, when it does: a held limb is
    /// stopped by what it meets, on its own side or through a portal pair it
    /// reaches into, and this is where (in the world, at the far end when it
    /// is through). `None`: it touches nothing.
    pub contact: Option<[f32; 2]>,
}

/// The mount a boss rides, at the read cut.
#[derive(Clone, Debug, PartialEq)]
pub struct MountView {
    pub position: [f32; 2],
    pub velocity: [f32; 2],
    pub size: [f32; 2],
    pub facing: f32,
    /// Its art does not turn with it (`Unmirrored`).
    pub unmirrored: bool,
    pub back: Option<[f32; 4]>,
    pub hall: Option<RoomHall>,
    pub limbs: Vec<MountLimb>,
}

impl Port for MountViewPort {
    const KEY: PortKey = PortKey::new("ambition.boss.mount", 2);
    const ROLE: PortRole = PortRole::Observation;
    type Value = MountView;

    fn encode(v: &MountView, out: &mut Vec<u8>) {
        wire::put_vec2(out, v.position);
        wire::put_vec2(out, v.velocity);
        wire::put_vec2(out, v.size);
        wire::put_f32(out, v.facing);
        wire::put_bool(out, v.unmirrored);
        wire::put_opt(out, v.back, |out, b| {
            for c in b {
                wire::put_f32(out, c);
            }
        });
        put_hall(out, v.hall);
        wire::put_u32(out, v.limbs.len() as u32);
        for limb in &v.limbs {
            wire::put_str(out, &limb.slot);
            wire::put_vec2(out, limb.position);
            wire::put_vec2(out, limb.size);
            wire::put_vec2(out, limb.home_offset);
            wire::put_i32(out, limb.lost_health);
            wire::put_bool(out, limb.held);
            wire::put_opt(out, limb.contact, wire::put_vec2);
        }
    }

    fn decode(r: &mut WireReader<'_>) -> Result<MountView, WireError> {
        let position = r.vec2()?;
        let velocity = r.vec2()?;
        let size = r.vec2()?;
        let facing = r.f32()?;
        let unmirrored = r.bool()?;
        let back = r.opt(|r| Ok([r.f32()?, r.f32()?, r.f32()?, r.f32()?]))?;
        let hall = hall_of(r)?;
        let count = r.u32()?;
        let mut limbs = Vec::new();
        for _ in 0..count {
            limbs.push(MountLimb {
                slot: r.str()?.to_owned(),
                position: r.vec2()?,
                size: r.vec2()?,
                home_offset: r.vec2()?,
                lost_health: r.i32()?,
                held: r.bool()?,
                contact: r.opt(|r| r.vec2())?,
            });
        }
        Ok(MountView { position, velocity, size, facing, unmirrored, back, hall, limbs })
    }
}

/// The request port marker for the pose of a limb of the mount a boss rides.
///
/// Port card:
///
/// * **Operation** — with a `hold`, put the limb there with that velocity
///   and facing (an external kinematic constraint, ADR 0024), and hold its
///   pose: the body integrator leaves it alone (`PoseOwnedExternally`), and
///   its ground contact is cleared, so the next step finds the ground again.
///   Without one, let go: the limb keeps where it is and its own vertical
///   speed and falls under its own gravity.
/// * **Owner** — `ambition_boss_encounter::conduct`.
/// * **Scope** — the boss the invocation ran for. The limb is `slot` of the
///   rig of the mount the boss rides (`ambition.boss.mount`). A boss that
///   rides nothing, or a slot the rig does not have, is refused with a
///   warning.
/// * **Time** — `boss_conduct`, this tick: the last word before combat
///   reads the limb.
pub struct LimbPosePort;

/// Where a held limb is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LimbHold {
    pub position: [f32; 2],
    pub velocity: [f32; 2],
    /// `1.0` draws the limb as authored, `-1.0` as its mirror.
    pub facing: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LimbPose {
    pub slot: String,
    pub hold: Option<LimbHold>,
}

impl Port for LimbPosePort {
    const KEY: PortKey = PortKey::new("ambition.boss.limb_pose", 1);
    const ROLE: PortRole = PortRole::Request;
    type Value = LimbPose;

    fn encode(v: &LimbPose, out: &mut Vec<u8>) {
        wire::put_str(out, &v.slot);
        wire::put_opt(out, v.hold, |out, h| {
            wire::put_vec2(out, h.position);
            wire::put_vec2(out, h.velocity);
            wire::put_f32(out, h.facing);
        });
    }

    fn decode(r: &mut WireReader<'_>) -> Result<LimbPose, WireError> {
        Ok(LimbPose {
            slot: r.str()?.to_owned(),
            hold: r.opt(|r| Ok(LimbHold { position: r.vec2()?, velocity: r.vec2()?, facing: r.f32()? }))?,
        })
    }
}

/// The request port marker for where the bond of a limb of the mount a boss
/// rides starts.
///
/// Port card:
///
/// * **Operation** — draw the limb's bond (the trail that joins it to its
///   host) from `start`, not from its host's body: a bond that is a module's
///   own (a spell that joins the two portals an arm is in). With no `start`
///   the limb has no bond: it is on its arm, which the mount's picture draws
///   (version 2). Presentation: no simulation state reads it. Asked for each
///   tick: a limb whose bond is not asked for in a tick has its bond from its
///   host's body again.
/// * **Owner** — `ambition_boss_encounter::conduct`, which keeps the limb's
///   `LimbBondStart`. The limb's view carries it.
/// * **Scope** — the boss the invocation ran for; the limb is `slot` of the
///   rig of the mount it rides. A boss that rides nothing, or a slot the rig
///   does not have, is refused with a warning.
/// * **Time** — `boss_conduct`.
pub struct LimbBondPort;

#[derive(Clone, Debug, PartialEq)]
pub struct LimbBond {
    pub slot: String,
    /// Where the bond starts, in world units. `None`: no bond.
    pub start: Option<[f32; 2]>,
}

impl Port for LimbBondPort {
    const KEY: PortKey = PortKey::new("ambition.boss.limb_bond", 2);
    const ROLE: PortRole = PortRole::Request;
    type Value = LimbBond;

    fn encode(v: &LimbBond, out: &mut Vec<u8>) {
        wire::put_str(out, &v.slot);
        wire::put_opt(out, v.start, |out, start| wire::put_vec2(out, start));
    }

    fn decode(r: &mut WireReader<'_>) -> Result<LimbBond, WireError> {
        Ok(LimbBond { slot: r.str()?.to_owned(), start: r.opt(|r| r.vec2())? })
    }
}

/// The request port marker for whether a limb of the mount a boss rides can
/// be struck.
///
/// Port card:
///
/// * **Operation** — `open` lets the limb be struck; closed, it refuses
///   damage (its scripted invulnerability). Set each tick the module wants
///   it; the last request of a tick stands, and it stays until the next.
/// * **Owner** — `ambition_boss_encounter::conduct`.
/// * **Scope** — the boss the invocation ran for; the limb is `slot` of the
///   rig of the mount it rides. A boss that rides nothing, or a slot the rig
///   does not have, is refused with a warning.
/// * **Time** — `boss_conduct`, this tick: combat reads it after.
pub struct LimbOpeningPort;

#[derive(Clone, Debug, PartialEq)]
pub struct LimbOpening {
    pub slot: String,
    pub open: bool,
}

impl Port for LimbOpeningPort {
    const KEY: PortKey = PortKey::new("ambition.boss.limb_opening", 1);
    const ROLE: PortRole = PortRole::Request;
    type Value = LimbOpening;

    fn encode(v: &LimbOpening, out: &mut Vec<u8>) {
        wire::put_str(out, &v.slot);
        wire::put_bool(out, v.open);
    }

    fn decode(r: &mut WireReader<'_>) -> Result<LimbOpening, WireError> {
        Ok(LimbOpening { slot: r.str()?.to_owned(), open: r.bool()? })
    }
}

/// The request port marker for a blow that a limb of the ridden mount took,
/// passed on to the boss.
///
/// Port card:
///
/// * **Operation** — the limb's lost health is healed. With `pass`, unless
///   the encounter refuses damage to the boss now (`BossConduct::invulnerable`),
///   the boss flashes and takes that much damage, and its encounter enters
///   `Death` when the damage kills it. Without `pass` the limb only heals: a
///   blow the module already passed on does not hurt the boss twice.
/// * **Owner** — `ambition_boss_encounter::conduct`.
/// * **Scope** — the boss the invocation ran for; the limb is `slot` of the
///   rig of the mount it rides. A boss that rides nothing, or a slot the rig
///   does not have, is refused with a warning. A limb that lost nothing
///   changes nothing.
/// * **Time** — `boss_conduct`, this tick.
pub struct LimbBlowPort;

#[derive(Clone, Debug, PartialEq)]
pub struct LimbBlow {
    pub slot: String,
    pub pass: bool,
}

impl Port for LimbBlowPort {
    const KEY: PortKey = PortKey::new("ambition.boss.limb_blow", 1);
    const ROLE: PortRole = PortRole::Request;
    type Value = LimbBlow;

    fn encode(v: &LimbBlow, out: &mut Vec<u8>) {
        wire::put_str(out, &v.slot);
        wire::put_bool(out, v.pass);
    }

    fn decode(r: &mut WireReader<'_>) -> Result<LimbBlow, WireError> {
        Ok(LimbBlow { slot: r.str()?.to_owned(), pass: r.bool()? })
    }
}

/// The request port marker for a damage box a limb of the mount a boss rides
/// carries.
///
/// Port card:
///
/// * **Operation** — the limb carries a damage box about its middle, held
///   while it is asked for each tick: a box not asked for in a tick is let
///   go, and a new `generation` replaces it. The box follows the limb, and a
///   portal pair the limb is in cuts it as it cuts the limb
///   (`docs/planning/engine/portals-a-body-in-two-places.md`): a fist through
///   a pair hurts at the far end. Each body it reaches is hit once per
///   generation.
/// * **Owner** — `ambition_boss_encounter::conduct`, which keeps the limb's
///   `LimbStrikeBox`.
/// * **Scope** — the boss the invocation ran for; the limb is `slot` of the
///   rig of the mount it rides. A boss that rides nothing, or a slot the rig
///   does not have, is refused with a warning.
/// * **Time** — `boss_conduct`; combat resolves the box this tick.
pub struct LimbStrikePort;

#[derive(Clone, Debug, PartialEq)]
pub struct LimbStrike {
    pub slot: String,
    pub generation: u32,
    pub half_extent: [f32; 2],
    pub damage: i32,
    /// A multiplier over the victim's standard feel-tuned launch.
    pub knockback: f32,
}

impl Port for LimbStrikePort {
    const KEY: PortKey = PortKey::new("ambition.boss.limb_strike", 1);
    const ROLE: PortRole = PortRole::Request;
    type Value = LimbStrike;

    fn encode(v: &LimbStrike, out: &mut Vec<u8>) {
        wire::put_str(out, &v.slot);
        wire::put_u32(out, v.generation);
        wire::put_vec2(out, v.half_extent);
        wire::put_i32(out, v.damage);
        wire::put_f32(out, v.knockback);
    }

    fn decode(r: &mut WireReader<'_>) -> Result<LimbStrike, WireError> {
        Ok(LimbStrike {
            slot: r.str()?.to_owned(),
            generation: r.u32()?,
            half_extent: r.vec2()?,
            damage: r.i32()?,
            knockback: r.f32()?,
        })
    }
}

/// The request port marker for a launch of the mount a boss rides.
///
/// Port card:
///
/// * **Operation** — launch the mount at `velocity` (world units per second,
///   +Y down), through the body's launch gateway (`BodyFlightState`): the
///   body step spends it. `flinchless` says the launch is the body's own (a
///   hop), not a hit: it does not tumble.
/// * **Owner** — `ambition_boss_encounter::conduct`.
/// * **Scope** — the boss the invocation ran for; the mount is the body it
///   rides. A boss that rides nothing is refused with a warning.
/// * **Time** — `boss_conduct`; the next body step moves the mount.
pub struct MountLaunchPort;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MountLaunch {
    pub velocity: [f32; 2],
    pub flinchless: bool,
}

impl Port for MountLaunchPort {
    const KEY: PortKey = PortKey::new("ambition.boss.mount_launch", 1);
    const ROLE: PortRole = PortRole::Request;
    type Value = MountLaunch;

    fn encode(v: &MountLaunch, out: &mut Vec<u8>) {
        wire::put_vec2(out, v.velocity);
        wire::put_bool(out, v.flinchless);
    }

    fn decode(r: &mut WireReader<'_>) -> Result<MountLaunch, WireError> {
        Ok(MountLaunch { velocity: r.vec2()?, flinchless: r.bool()? })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round<P: Port>(v: &P::Value) -> P::Value {
        let mut out = Vec::new();
        P::encode(v, &mut out);
        let mut r = WireReader::new(&out);
        let back = P::decode(&mut r).unwrap();
        r.finish().unwrap();
        back
    }

    #[test]
    fn the_conduct_values_survive_the_wire() {
        let conduct = BossConduct {
            position: [1.0, 2.0],
            velocity: [3.0, -4.0],
            facing: -1.0,
            side: 1.0,
            alive: true,
            telegraph: Some(LiveMove { key: "noodle_lash".into(), remaining: 0.5 }),
            active: None,
            target: Some([9.0, 8.0]),
            hall: Some(RoomHall { floor: 900.0, left: 10.0, right: 1500.0 }),
            driven: false,
            enraged: true,
            holding: true,
            minions: 3,
            between_phases: true,
            room: Some([1280.0, 720.0]),
            health: -2,
            invulnerable: true,
            target_body: Some(BodyBox {
                position: [9.5, 7.0],
                size: [24.0, 40.0],
                swing: Some(BodySwing { since: 0.1, hurts_in: 0.05, live: false, ends_in: 0.3 }),
            }),
            size: [40.0, 60.0],
            awake: true,
        };
        assert_eq!(round::<BossConductPort>(&conduct), conduct);
        let at_rest = BossConduct { target_body: Some(BodyBox { position: [1.0, 2.0], size: [3.0, 4.0], swing: None }), ..conduct.clone() };
        assert_eq!(round::<BossConductPort>(&at_rest), at_rest);
        let bare = BossConduct { telegraph: None, active: Some(LiveMove { key: "x".into(), remaining: 1.0 }), target: None, hall: None, target_body: None, ..conduct };
        assert_eq!(round::<BossConductPort>(&bare), bare);
        for guarded in [false, true] {
            assert_eq!(round::<BossGuardPort>(&BossGuard { guarded }), BossGuard { guarded });
        }
        for pose in [None, Some(Pose { position: [1.0, 1.0], velocity: [0.0, 2.0] })] {
            for side in [None, Some(-1.0)] {
                let v = ConductedPose { pose, side };
                assert_eq!(round::<ConductedPosePort>(&v), v);
            }
        }
        for on in [DrawnPart::Body, DrawnPart::Mount, DrawnPart::Limb("hand_left".into())] {
            for names in [Vec::new(), vec!["hit".to_string(), "rest".to_string()]] {
                let v = DrawnRow { on: on.clone(), names, elapsed: 0.25, looping: true };
                assert_eq!(round::<DrawnRowPort>(&v), v);
            }
        }
    }

    #[test]
    fn a_mount_view_survives_the_wire() {
        let fist = MountLimb {
            slot: "hand_left".into(),
            position: [700.0, 1000.0],
            size: [130.0, 100.0],
            home_offset: [-270.0, -70.0],
            lost_health: 3,
            held: true,
            contact: Some([700.0, 1050.0]),
        };
        let view = MountView {
            position: [900.0, 1040.0],
            velocity: [0.0, -520.0],
            size: [439.4, 414.7],
            facing: -1.0,
            unmirrored: true,
            back: Some([692.0, 1030.25, 1069.0, 1044.25]),
            hall: Some(RoomHall { floor: 1250.0, left: 48.0, right: 1800.0 }),
            limbs: vec![fist.clone(), MountLimb { slot: "hand_right".into(), held: false, lost_health: 0, contact: None, ..fist }],
        };
        assert_eq!(round::<MountViewPort>(&view), view);
        let bare = MountView { back: None, hall: None, limbs: Vec::new(), ..view };
        assert_eq!(round::<MountViewPort>(&bare), bare);
        for flinchless in [false, true] {
            let launch = MountLaunch { velocity: [0.0, -520.0], flinchless };
            assert_eq!(round::<MountLaunchPort>(&launch), launch);
        }
        for flag in [false, true] {
            let opening = LimbOpening { slot: "hand_left".into(), open: flag };
            assert_eq!(round::<LimbOpeningPort>(&opening), opening);
            let bond = LimbBond { slot: "hand_left".into(), start: flag.then_some([12.5, 840.25]) };
            assert_eq!(round::<LimbBondPort>(&bond), bond);
            let blow = LimbBlow { slot: "hand_right".into(), pass: flag };
            assert_eq!(round::<LimbBlowPort>(&blow), blow);
            let strike = LimbStrike {
                slot: "hand_right".into(),
                generation: 3,
                half_extent: [40.0, 30.0],
                damage: if flag { 2 } else { -1 },
                knockback: 1.4,
            };
            assert_eq!(round::<LimbStrikePort>(&strike), strike);
        }
        for hold in [None, Some(LimbHold { position: [630.0, 970.0], velocity: [12.0, -3.0], facing: -1.0 })] {
            let v = LimbPose { slot: "hand_left".into(), hold };
            assert_eq!(round::<LimbPosePort>(&v), v);
        }
    }
}
