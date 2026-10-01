//! The boss domain's extension trigger port.
//!
//! Port card (`docs/planning/engine/extension-domain-contracts.md`):
//!
//! * **Operation** — whether a boss's pattern pressed `Special(<key>)` this
//!   tick. The entry's selector is the key.
//! * **Owner** — `ambition_boss_encounter::extension::queue_boss_special_casts`
//!   reads `ActorActionMessage::Special` and the boss's components.
//! * **Scope** — one invocation for EACH boss, EACH tick the phase runs, for
//!   each bound key, with [`BossCaster::pressed`]. A boss pattern presses a
//!   special on every tick of its beat, so a technique that acts once per
//!   beat must see the ticks with no press to know that a beat ended. (The
//!   simulation tick cannot tell it: it also advances while gameplay is
//!   suspended, and a beat continues across a pause.) Several presses of one
//!   key by one boss in one tick are one press; the last gives the move use.
//! * **Time** — `technique_execution`. Kinematics and the tracked target are
//!   this tick's settled values.
//! * **Read model** — world units, +Y DOWN (the engine's frame: top-left
//!   origin); `facing` is the sign of the body's
//!   facing (`1.0` or `-1.0`). `telegraphing` is true while the boss's
//!   pattern telegraphs THIS key, the ticks before the presses.
//! * **Version 2** (2026-10-01): adds `telegraphing`, and gives the authored
//!   `projectile_offset` itself instead of one technique's origin made from
//!   it — the echo fan adds it as is, the eye beam mirrors it by facing.
//! * **Version 3** (2026-10-01): adds the boss's combat box (`body_center`,
//!   `body_half_size`), the box its hits are judged against; a stomp's
//!   shock line stands on its bottom face.
//! * **Version 4** (2026-10-01): adds `room_size`, the size of the live room
//!   the boss is in (OW1 cut 7n: a technique sizes its volley by the boss's
//!   OWN room, never by "the" room).
//! * **Absence** — `target` is `None` when the boss tracks nothing.
//!   `room_size` is `None` when the boss's live room cannot be told.
//! * **Idle** — an invocation is IDLE when the key is neither pressed nor
//!   telegraphed. An entry that declares `IdlePolicy::ResetState` is not
//!   called on those ticks; its records go back to their initial values.
//! * **Replay** — the value is derived each tick from rollback state; the
//!   port keeps nothing between ticks.

use ambition_extension_sdk::wire::{self, WireReader, WireError};
use ambition_extension_sdk::{Port, PortKey, PortRole};

/// The trigger port marker.
pub struct BossSpecialCast;

impl Port for BossSpecialCast {
    const KEY: PortKey = PortKey::new("ambition.boss.special_cast", 4);
    const ROLE: PortRole = PortRole::Trigger;
    type Value = BossCaster;

    fn encode(v: &BossCaster, out: &mut Vec<u8>) {
        wire::put_bool(out, v.pressed);
        wire::put_bool(out, v.telegraphing);
        wire::put_bool(out, v.alive);
        wire::put_vec2(out, v.position);
        wire::put_f32(out, v.facing);
        wire::put_vec2(out, v.projectile_offset);
        wire::put_vec2(out, v.body_center);
        wire::put_vec2(out, v.body_half_size);
        wire::put_opt(out, v.target, wire::put_vec2);
        wire::put_opt(out, v.room_size, wire::put_vec2);
    }

    fn decode(r: &mut WireReader<'_>) -> Result<BossCaster, WireError> {
        Ok(BossCaster {
            pressed: r.bool()?,
            telegraphing: r.bool()?,
            alive: r.bool()?,
            position: r.vec2()?,
            facing: r.f32()?,
            projectile_offset: r.vec2()?,
            body_center: r.vec2()?,
            body_half_size: r.vec2()?,
            target: r.opt(WireReader::vec2)?,
            room_size: r.opt(WireReader::vec2)?,
        })
    }
}

/// The boss that pressed the special, at the read cut.
#[derive(Clone, Debug, PartialEq)]
pub struct BossCaster {
    /// True when the boss pressed the selector key this tick.
    pub pressed: bool,
    /// True while the boss's pattern telegraphs the selector key.
    pub telegraphing: bool,
    /// False when the boss has no health left.
    pub alive: bool,
    /// The body's position.
    pub position: [f32; 2],
    /// `1.0` faces +x, `-1.0` faces -x.
    pub facing: f32,
    /// The boss's authored projectile-origin offset from its position, as
    /// authored (not mirrored by facing).
    pub projectile_offset: [f32; 2],
    /// The centre of the boss's combat box.
    pub body_center: [f32; 2],
    /// The half size of the boss's combat box. Its feet are at
    /// `body_center.y + body_half_size.y` (+Y is down).
    pub body_half_size: [f32; 2],
    /// The centre of the body the boss tracks, or the tracked point when the
    /// target is not a body.
    pub target: Option<[f32; 2]>,
    /// The size of the live room the boss is in. Its playable area is
    /// `0..room_size` in world units.
    pub room_size: Option<[f32; 2]>,
}

/// The boss domain's summon request port.
///
/// Port card (`docs/planning/engine/extension-domain-contracts.md`):
///
/// * **Operation** — bring a character into the boss's encounter: a minion.
/// * **Owner** — `ambition_boss_encounter::extension`, which lowers it into
///   `EffectRequest` → `Effect::Summon`, the summon executor's one road.
/// * **Scope and grant** — only a BOSS can summon: a request for a body
///   that is not a boss is refused. The minion joins the boss's encounter
///   and fights on the encounter's enemy side; a module chooses neither.
/// * **Identity** — the minion's id is `<label>:<boss id>:<serial…>`, joined
///   with `:`. The module gives the label and the serial (a strike counter,
///   from its own records, so a rollback replays the same ids); the boss id
///   comes from the boss. An empty label, or one with a `:`, is refused.
/// * **Time** — offered in `technique_execution`. The summon executor makes
///   the minion this tick.
/// * **Read model** — world units, +Y down.
/// * **Result** — submitted is not applied: a `character_id` that names no
///   character makes a generic body. No acknowledgement port yet.
pub struct BossSummonPort;

/// A minion a boss module asks for.
#[derive(Clone, Debug, PartialEq)]
pub struct BossSummon {
    /// The id namespace: a stable name of the technique's summons.
    pub label: String,
    /// The minion's number in that namespace, for the boss.
    pub serial: Vec<u32>,
    pub position: [f32; 2],
    pub half_size: [f32; 2],
    /// The character the minion is made from.
    pub character_id: String,
    /// Health for this minion; `None` keeps the character's own.
    pub health: Option<u32>,
    /// Whether the minion keeps the character's contact hazard.
    pub keeps_contact_damage: bool,
    /// The minion fights on the boss's own side (`HitSide::Boss`): the boss's
    /// own volumes pass through it. False: it is an enemy like any other.
    pub on_boss_side: bool,
}

impl Port for BossSummonPort {
    const KEY: PortKey = PortKey::new("ambition.boss.summon", 2);
    const ROLE: PortRole = PortRole::Request;
    type Value = BossSummon;

    fn encode(v: &BossSummon, out: &mut Vec<u8>) {
        wire::put_str(out, &v.label);
        wire::put_u32(out, v.serial.len() as u32);
        for n in &v.serial {
            wire::put_u32(out, *n);
        }
        wire::put_vec2(out, v.position);
        wire::put_vec2(out, v.half_size);
        wire::put_str(out, &v.character_id);
        wire::put_opt(out, v.health, wire::put_u32);
        wire::put_bool(out, v.keeps_contact_damage);
        wire::put_bool(out, v.on_boss_side);
    }

    fn decode(r: &mut WireReader<'_>) -> Result<BossSummon, WireError> {
        let label = r.str()?.to_owned();
        let len = r.u32()?;
        let serial = (0..len).map(|_| r.u32()).collect::<Result<Vec<_>, _>>()?;
        Ok(BossSummon {
            label,
            serial,
            position: r.vec2()?,
            half_size: r.vec2()?,
            character_id: r.str()?.to_owned(),
            health: r.opt(WireReader::u32)?,
            keeps_contact_damage: r.bool()?,
            on_boss_side: r.bool()?,
        })
    }
}

impl BossSummon {
    /// The minion's id for the boss `boss_id`, or `None` when the label is
    /// refused.
    pub fn id(&self, boss_id: &str) -> Option<String> {
        if self.label.is_empty() || self.label.contains(':') {
            return None;
        }
        let mut id = format!("{}:{boss_id}", self.label);
        for n in &self.serial {
            id.push(':');
            id.push_str(&n.to_string());
        }
        Some(id)
    }
}

pub mod conduct;
pub use conduct::{
    BossConduct, BossConductPort, ConductedPose, ConductedPosePort, DrawnRow, DrawnRowPort, LiveMove, Pose, RoomHall,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_summon_id_is_label_boss_and_serial() {
        let mut s = BossSummon {
            label: "gradient_sentinel_cascade".into(),
            serial: vec![3, 1],
            position: [0.0, 0.0],
            half_size: [1.0, 1.0],
            character_id: "npc_ai_slop".into(),
            health: None,
            keeps_contact_damage: true,
            on_boss_side: false,
        };
        assert_eq!(s.id("gs").as_deref(), Some("gradient_sentinel_cascade:gs:3:1"));
        s.label = "a:b".into();
        assert_eq!(s.id("gs"), None);
        s.label = String::new();
        assert_eq!(s.id("gs"), None);
    }

    #[test]
    fn a_summon_survives_the_wire() {
        let s = BossSummon {
            label: "x".into(),
            serial: vec![7],
            position: [1.5, -2.0],
            half_size: [3.0, 4.0],
            character_id: "npc".into(),
            health: Some(9),
            keeps_contact_damage: false,
            on_boss_side: true,
        };
        let mut out = Vec::new();
        BossSummonPort::encode(&s, &mut out);
        let mut r = WireReader::new(&out);
        assert_eq!(BossSummonPort::decode(&mut r).unwrap(), s);
        r.finish().unwrap();
    }
}
