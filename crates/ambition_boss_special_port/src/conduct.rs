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
///   moves against the room itself.
/// * **Absence** — `target` is `None` when the boss tracks nothing; `hall`
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
    const KEY: PortKey = PortKey::new("ambition.boss.conduct", 4);
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
///   `side` is the side the boss faces while its pose is held, and the side
///   the next tick's trigger reports.
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
    pub side: f32,
}

impl Port for ConductedPosePort {
    const KEY: PortKey = PortKey::new("ambition.boss.conducted_pose", 1);
    const ROLE: PortRole = PortRole::Request;
    type Value = ConductedPose;

    fn encode(v: &ConductedPose, out: &mut Vec<u8>) {
        wire::put_bool(out, v.pose.is_some());
        if let Some(p) = v.pose {
            wire::put_vec2(out, p.position);
            wire::put_vec2(out, p.velocity);
        }
        wire::put_f32(out, v.side);
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
        Ok(ConductedPose { pose, side: r.f32()? })
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

/// The request port marker for the row a body is drawn with.
///
/// Port card:
///
/// * **Operation** — draw the body with the named sheet row, `elapsed`
///   seconds in, looping or holding its last frame; with no `name`, draw it
///   as the engine would. Presentation: no simulation state reads it, and it
///   is derived again each tick (the module's own clock is the elapsed time).
/// * **Owner** — `ambition_boss_encounter::extension` (`PinnedRow`).
/// * **Scope** — the body the invocation ran for.
/// * **Time** — `boss_conduct`.
pub struct DrawnRowPort;

#[derive(Clone, Debug, PartialEq)]
pub struct DrawnRow {
    pub name: Option<String>,
    pub elapsed: f32,
    pub looping: bool,
}

impl Port for DrawnRowPort {
    const KEY: PortKey = PortKey::new("ambition.presentation.drawn_row", 1);
    const ROLE: PortRole = PortRole::Request;
    type Value = DrawnRow;

    fn encode(v: &DrawnRow, out: &mut Vec<u8>) {
        wire::put_bool(out, v.name.is_some());
        if let Some(name) = &v.name {
            wire::put_str(out, name);
        }
        wire::put_f32(out, v.elapsed);
        wire::put_bool(out, v.looping);
    }

    fn decode(r: &mut WireReader<'_>) -> Result<DrawnRow, WireError> {
        let name = if r.bool()? { Some(r.str()?.to_owned()) } else { None };
        Ok(DrawnRow {
            name,
            elapsed: r.f32()?,
            looping: r.bool()?,
        })
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
        };
        assert_eq!(round::<BossConductPort>(&conduct), conduct);
        let bare = BossConduct { telegraph: None, active: Some(LiveMove { key: "x".into(), remaining: 1.0 }), target: None, hall: None, ..conduct };
        assert_eq!(round::<BossConductPort>(&bare), bare);
        for guarded in [false, true] {
            assert_eq!(round::<BossGuardPort>(&BossGuard { guarded }), BossGuard { guarded });
        }
        for pose in [None, Some(Pose { position: [1.0, 1.0], velocity: [0.0, 2.0] })] {
            let v = ConductedPose { pose, side: -1.0 };
            assert_eq!(round::<ConductedPosePort>(&v), v);
        }
        for name in [None, Some("drift".to_string())] {
            let v = DrawnRow { name, elapsed: 0.25, looping: true };
            assert_eq!(round::<DrawnRowPort>(&v), v);
        }
    }
}
