//! Module-owned entities: a module asks the world for an entity of a kind it
//! names (a turret, a well), and the world ticks each such entity through
//! the module, until the entity's lifetime ends.
//!
//! The world owns the entity: its identity, its side, its session, its
//! position and its lifetime. The module owns what the entity DOES, and keeps
//! its own state (a cooldown) in records scoped to the entity.

use ambition_extension_sdk::wire::{self, WireError, WireReader};
use ambition_extension_sdk::{Port, PortKey, PortRole};

use crate::special::{number_at, put_params, read_params, text_at};
use crate::{ContactBody, SpecialParam};

/// The request port marker for a new module-owned entity.
///
/// Port card (`docs/planning/engine/extension-domain-contracts.md`):
///
/// * **Operation** — put an entity of kind `kind` at `at`, for `lifetime_s`
///   gameplay seconds. The entity ticks through the entries bound to
///   [`ModuleEntityTickPort`] with the selector `kind`. `half_extents` is
///   the size of the box that bodies touch (zero: a point, which no body
///   touches). `params` are the entity's authored values (a move's params,
///   as [`crate::FighterSpecial`] gives them); the world keeps them on the
///   entity unchanged and gives them back at each tick.
/// * **Owner** — `ambition_abilities::extension`.
/// * **Scope** — the spawner: the body the invocation ran for. The entity
///   takes the spawner's effective side, match team, session and
///   presentation source, frozen at the spawn (the entity outlives its
///   spawner). Its identity is minted from the spawner's identity, and it is
///   in the spawner's live room.
/// * **Time** — `wielded_use` and `technique_execution`. The entity exists at
///   `module_entity_tick` of the same tick.
/// * **Result** — a spawner that cannot name the entity (no `SimId` or no
///   `SimIdCounter`) spawns nothing, and the refusal is logged. A module
///   that asked `Wielder::names_spawns` is never refused.
pub struct SpawnModuleEntityPort;

#[derive(Clone, Debug, PartialEq)]
pub struct ModuleEntitySpawn {
    pub kind: String,
    pub at: [f32; 2],
    pub lifetime_s: f32,
    pub half_extents: [f32; 2],
    pub params: Vec<(String, SpecialParam)>,
}

impl Port for SpawnModuleEntityPort {
    const KEY: PortKey = PortKey::new("ambition.world.spawn_module_entity", 2);
    const ROLE: PortRole = PortRole::Request;
    type Value = ModuleEntitySpawn;

    fn encode(v: &ModuleEntitySpawn, out: &mut Vec<u8>) {
        wire::put_str(out, &v.kind);
        wire::put_vec2(out, v.at);
        wire::put_f32(out, v.lifetime_s);
        wire::put_vec2(out, v.half_extents);
        put_params(out, &v.params);
    }

    fn decode(r: &mut WireReader<'_>) -> Result<ModuleEntitySpawn, WireError> {
        Ok(ModuleEntitySpawn {
            kind: r.str()?.to_owned(),
            at: r.vec2()?,
            lifetime_s: r.f32()?,
            half_extents: r.vec2()?,
            params: read_params(r)?,
        })
    }
}

/// The trigger port marker for one tick of a module-owned entity.
///
/// Port card:
///
/// * **Operation** — a module-owned entity lives this tick. The entry's
///   selector is the entity's kind.
/// * **Owner** — `ambition_abilities::extension`, which ages each entity by
///   the gameplay step first, and removes it (with no invocation) when its
///   lifetime ends.
/// * **Scope** — the entity. One invocation for EACH living entity of a bound
///   kind, EACH tick with a gameplay step greater than zero, in the order of
///   (position, remaining lifetime, identity). Records are the entity's and
///   go with it.
/// * **Time** — `module_entity_tick`, after `wielded_use` in the same tick.
///   Requests are consumed this tick.
/// * **Read model** — world units, +Y down. `nearest_enemy` is the centre of
///   the nearest body IN THE ENTITY'S OWN LIVE ROOM whose effective side is
///   `Enemy` and that can be hit (in play, on the playable plane), with ties
///   broken by identity. It is the `Enemy` side, NOT the side hostile to the
///   entity's own side.
///   `params` are the ones of the spawn. `touchers` are the bodies of the
///   entity's own live room whose box meets the entity's box (side to side
///   and top to bottom; a body that stands on a box meets it), that can be
///   hit (in play, on the playable plane), of any side, in identity order.
/// * **Absence** — `nearest_enemy` is `None` when there is no such body.
/// * **Replay** — derived each tick from rollback state.
pub struct ModuleEntityTickPort;

#[derive(Clone, Debug, PartialEq)]
pub struct ModuleEntityTick {
    pub position: [f32; 2],
    /// Seconds of lifetime left, after this tick's aging. Always above zero.
    pub remaining_s: f32,
    pub nearest_enemy: Option<[f32; 2]>,
    pub params: Vec<(String, SpecialParam)>,
    pub touchers: Vec<ContactBody>,
}

impl ModuleEntityTick {
    /// The number at `path` of the spawn's params.
    pub fn number(&self, path: &str) -> Option<f32> {
        number_at(&self.params, path)
    }

    /// The text at `path` of the spawn's params.
    pub fn text(&self, path: &str) -> Option<&str> {
        text_at(&self.params, path)
    }
}

impl Port for ModuleEntityTickPort {
    const KEY: PortKey = PortKey::new("ambition.world.module_entity_tick", 2);
    const ROLE: PortRole = PortRole::Trigger;
    type Value = ModuleEntityTick;

    fn encode(v: &ModuleEntityTick, out: &mut Vec<u8>) {
        wire::put_vec2(out, v.position);
        wire::put_f32(out, v.remaining_s);
        wire::put_opt(out, v.nearest_enemy, wire::put_vec2);
        put_params(out, &v.params);
        wire::put_u32(out, v.touchers.len() as u32);
        for toucher in &v.touchers {
            wire::put_vec2(out, toucher.position);
            wire::put_vec2(out, toucher.size);
            wire::put_vec2(out, toucher.velocity);
        }
    }

    fn decode(r: &mut WireReader<'_>) -> Result<ModuleEntityTick, WireError> {
        let position = r.vec2()?;
        let remaining_s = r.f32()?;
        let nearest_enemy = r.opt(WireReader::vec2)?;
        let params = read_params(r)?;
        let count = r.read_len()?;
        let mut touchers = Vec::with_capacity(count);
        for _ in 0..count {
            touchers.push(ContactBody { position: r.vec2()?, size: r.vec2()?, velocity: r.vec2()? });
        }
        Ok(ModuleEntityTick { position, remaining_s, nearest_enemy, params, touchers })
    }
}

/// The request port marker for ending a module entity before its lifetime.
///
/// Port card:
///
/// * **Operation** — remove the module entity the invocation runs for. Its
///   lifetime is the latest it lives; a module that keeps its own clock (a
///   well that pulls and THEN ages) ends it at the tick it chooses.
/// * **Owner** — `ambition_abilities::extension`.
/// * **Scope** — the invocation's scope. A scope that is not a module entity
///   is not removed, and the refusal is logged.
/// * **Time** — `module_entity_tick`; the entity is gone before the next
///   tick's `module_entity_tick`.
pub struct EndModuleEntityPort;

#[derive(Clone, Debug, PartialEq)]
pub struct EndModuleEntity;

impl Port for EndModuleEntityPort {
    const KEY: PortKey = PortKey::new("ambition.world.end_module_entity", 1);
    const ROLE: PortRole = PortRole::Request;
    type Value = EndModuleEntity;

    fn encode(_: &EndModuleEntity, _: &mut Vec<u8>) {}

    fn decode(_: &mut WireReader<'_>) -> Result<EndModuleEntity, WireError> {
        Ok(EndModuleEntity)
    }
}

/// The request port marker for a pull toward a point.
///
/// Port card:
///
/// * **Operation** — move each body that the pull reaches toward `center` by
///   the fraction `min(rate * dt, 1)` of its distance, this tick. A body is
///   reached when its centre is within `radius` of `center`, its effective
///   side is `Enemy`, it can be hit (in play, on the playable plane), and it is
///   in the puller's own live room. The
///   move is an external kinematic constraint (ADR 0024): the body's
///   collision step resolves a wall the pull pushes it into.
/// * **Owner** — `ambition_abilities::extension`.
/// * **Time** — `module_entity_tick`, in the body-path `Carry` set: the move
///   is travel the path readers see this tick. Two pulls on one tick apply in
///   request order.
pub struct PullBodiesPort;

#[derive(Clone, Debug, PartialEq)]
pub struct PullBodies {
    pub center: [f32; 2],
    pub radius: f32,
    /// The fraction of the remaining gap closed per second.
    pub rate: f32,
}

impl Port for PullBodiesPort {
    const KEY: PortKey = PortKey::new("ambition.world.pull_bodies", 1);
    const ROLE: PortRole = PortRole::Request;
    type Value = PullBodies;

    fn encode(v: &PullBodies, out: &mut Vec<u8>) {
        wire::put_vec2(out, v.center);
        wire::put_f32(out, v.radius);
        wire::put_f32(out, v.rate);
    }

    fn decode(r: &mut WireReader<'_>) -> Result<PullBodies, WireError> {
        Ok(PullBodies {
            center: r.vec2()?,
            radius: r.f32()?,
            rate: r.f32()?,
        })
    }
}

/// The request port marker for a throw of one toucher.
///
/// Port card:
///
/// * **Operation** — the toucher at index `toucher` of this tick's
///   [`ModuleEntityTick::touchers`] leaves at `velocity` (world units per
///   second, +Y down). The velocity is set, not added: a body that falls fast
///   onto the entity is thrown as far as one that walks onto it.
/// * **Owner** — `ambition_abilities::extension`.
/// * **Scope** — the module entity the invocation ran for, and only its
///   touchers of this tick. An index that is not a toucher is refused, and
///   the refusal is logged.
/// * **Time** — `module_entity_tick`; the body integrates the velocity this
///   tick.
pub struct LaunchToucherPort;

#[derive(Clone, Debug, PartialEq)]
pub struct LaunchToucher {
    pub toucher: u32,
    pub velocity: [f32; 2],
}

impl Port for LaunchToucherPort {
    const KEY: PortKey = PortKey::new("ambition.world.launch_toucher", 1);
    const ROLE: PortRole = PortRole::Request;
    type Value = LaunchToucher;

    fn encode(v: &LaunchToucher, out: &mut Vec<u8>) {
        wire::put_u32(out, v.toucher);
        wire::put_vec2(out, v.velocity);
    }

    fn decode(r: &mut WireReader<'_>) -> Result<LaunchToucher, WireError> {
        Ok(LaunchToucher { toucher: r.u32()?, velocity: r.vec2()? })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_entity_values_survive_the_wire() {
        let spawn = ModuleEntitySpawn {
            kind: "sentry".into(),
            at: [3.0, -4.0],
            lifetime_s: 5.0,
            half_extents: [20.0, 6.0],
            params: vec![
                ("uses".to_owned(), SpecialParam::Number(3.0)),
                ("vfx".to_owned(), SpecialParam::Text("spring_pop".to_owned())),
            ],
        };
        let mut out = Vec::new();
        SpawnModuleEntityPort::encode(&spawn, &mut out);
        let mut r = WireReader::new(&out);
        assert_eq!(SpawnModuleEntityPort::decode(&mut r).unwrap(), spawn);
        r.finish().unwrap();

        for nearest_enemy in [None, Some([7.0, 8.0])] {
            let tick = ModuleEntityTick {
                position: [1.0, 2.0],
                remaining_s: 0.5,
                nearest_enemy,
                params: spawn.params.clone(),
                touchers: vec![ContactBody { position: [1.0, -10.0], size: [28.0, 40.0], velocity: [0.0, 90.0] }],
            };
            assert_eq!(tick.number("uses"), Some(3.0));
            assert_eq!(tick.text("vfx"), Some("spring_pop"));
            let mut out = Vec::new();
            ModuleEntityTickPort::encode(&tick, &mut out);
            let mut r = WireReader::new(&out);
            assert_eq!(ModuleEntityTickPort::decode(&mut r).unwrap(), tick);
            r.finish().unwrap();
        }

        let pull = PullBodies {
            center: [5.0, 6.0],
            radius: 220.0,
            rate: 5.0,
        };
        let mut out = Vec::new();
        PullBodiesPort::encode(&pull, &mut out);
        let mut r = WireReader::new(&out);
        assert_eq!(PullBodiesPort::decode(&mut r).unwrap(), pull);
        r.finish().unwrap();

        let launch = LaunchToucher { toucher: 2, velocity: [0.0, -900.0] };
        let mut out = Vec::new();
        LaunchToucherPort::encode(&launch, &mut out);
        let mut r = WireReader::new(&out);
        assert_eq!(LaunchToucherPort::decode(&mut r).unwrap(), launch);
        r.finish().unwrap();
    }
}
