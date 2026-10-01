//! Module-owned entities: a module asks the world for an entity of a kind it
//! names (a turret, a well), and the world ticks each such entity through
//! the module, until the entity's lifetime ends.
//!
//! The world owns the entity: its identity, its side, its session, its
//! position and its lifetime. The module owns what the entity DOES, and keeps
//! its own state (a cooldown) in records scoped to the entity.

use ambition_extension_sdk::wire::{self, WireError, WireReader};
use ambition_extension_sdk::{Port, PortKey, PortRole};

/// The request port marker for a new module-owned entity.
///
/// Port card (`docs/planning/engine/extension-domain-contracts.md`):
///
/// * **Operation** — put an entity of kind `kind` at `at`, for `lifetime_s`
///   gameplay seconds. The entity ticks through the entries bound to
///   [`ModuleEntityTickPort`] with the selector `kind`.
/// * **Owner** — `ambition_abilities::extension`.
/// * **Scope** — the spawner: the body the invocation ran for. The entity
///   takes the spawner's effective side, match team, session and
///   presentation source, frozen at the spawn (the entity outlives its
///   spawner). Its identity is minted from the spawner's identity.
/// * **Time** — `wielded_use`. The entity exists at `module_entity_tick` of
///   the same tick.
/// * **Result** — a spawner that cannot name the entity (no `SimId` or no
///   `SimIdCounter`) spawns nothing, and the refusal is logged. A module
///   that asked `Wielder::names_spawns` is never refused.
pub struct SpawnModuleEntityPort;

#[derive(Clone, Debug, PartialEq)]
pub struct ModuleEntitySpawn {
    pub kind: String,
    pub at: [f32; 2],
    pub lifetime_s: f32,
}

impl Port for SpawnModuleEntityPort {
    const KEY: PortKey = PortKey::new("ambition.world.spawn_module_entity", 1);
    const ROLE: PortRole = PortRole::Request;
    type Value = ModuleEntitySpawn;

    fn encode(v: &ModuleEntitySpawn, out: &mut Vec<u8>) {
        wire::put_str(out, &v.kind);
        wire::put_vec2(out, v.at);
        wire::put_f32(out, v.lifetime_s);
    }

    fn decode(r: &mut WireReader<'_>) -> Result<ModuleEntitySpawn, WireError> {
        Ok(ModuleEntitySpawn {
            kind: r.str()?.to_owned(),
            at: r.vec2()?,
            lifetime_s: r.f32()?,
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
///   the nearest body whose effective side is `Enemy` and that can be hit (in
///   play, on the playable plane), with ties broken by identity. It is the
///   `Enemy` side, NOT the side hostile to the entity's own side.
/// * **Absence** — `nearest_enemy` is `None` when there is no such body.
/// * **Replay** — derived each tick from rollback state.
pub struct ModuleEntityTickPort;

#[derive(Clone, Debug, PartialEq)]
pub struct ModuleEntityTick {
    pub position: [f32; 2],
    /// Seconds of lifetime left, after this tick's aging. Always above zero.
    pub remaining_s: f32,
    pub nearest_enemy: Option<[f32; 2]>,
}

impl Port for ModuleEntityTickPort {
    const KEY: PortKey = PortKey::new("ambition.world.module_entity_tick", 1);
    const ROLE: PortRole = PortRole::Trigger;
    type Value = ModuleEntityTick;

    fn encode(v: &ModuleEntityTick, out: &mut Vec<u8>) {
        wire::put_vec2(out, v.position);
        wire::put_f32(out, v.remaining_s);
        wire::put_opt(out, v.nearest_enemy, wire::put_vec2);
    }

    fn decode(r: &mut WireReader<'_>) -> Result<ModuleEntityTick, WireError> {
        Ok(ModuleEntityTick {
            position: r.vec2()?,
            remaining_s: r.f32()?,
            nearest_enemy: r.opt(WireReader::vec2)?,
        })
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
            };
            let mut out = Vec::new();
            ModuleEntityTickPort::encode(&tick, &mut out);
            let mut r = WireReader::new(&out);
            assert_eq!(ModuleEntityTickPort::decode(&mut r).unwrap(), tick);
            r.finish().unwrap();
        }
    }
}
