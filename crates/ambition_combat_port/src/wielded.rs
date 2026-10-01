//! The held-item domain's extension ports: a body uses the item it holds,
//! pays for it from its resource bank, and is heard doing it.

use ambition_extension_sdk::wire::{self, WireError, WireReader};
use ambition_extension_sdk::{Port, PortKey, PortRole};

/// The trigger port marker for a wielded item's use.
///
/// Port card (`docs/planning/engine/extension-domain-contracts.md`):
///
/// * **Operation** — a body holds an item and may use it. The entry's
///   selector is the held item's id (`HeldItemSpec::id`, for example
///   `shockwave`).
/// * **Owner** — `ambition_abilities::extension`, which reads the body's
///   resolved control frame (`ActorControl`, written alike by a player's input
///   and by an NPC brain), its kinematics, gravity frame and resource bank.
/// * **Scope** — one invocation for EACH body holding a bound item, EACH tick
///   the phase runs. An invocation is IDLE when [`Wielder::pressed`] is false.
/// * **Time** — `wielded_use`; every value is this tick's settled value.
/// * **Read model** — world units, +Y down. `frame_side` and `frame_down` are
///   the body's gravity basis: a body-local vector `(x, y)` is
///   `frame_side * x + frame_down * y` in the world ([`Wielder::to_world`]).
///   `aim_local` is body-local: the aim stick, else the movement stick, else
///   the facing.
/// * **Absence** — `mana` is `None` when the body has no mana pool.
///   `names_spawns` is false when the body has no simulation identity or no
///   mint stream: a spawn it asks for is refused (ADR 0030).
/// * **Replay** — derived each tick from rollback state.
pub struct WieldedUsePort;

/// The body that holds the item, at the read cut.
#[derive(Clone, Debug, PartialEq)]
pub struct Wielder {
    /// Attack pressed this tick, and not as the throw gesture (Shield held).
    pub pressed: bool,
    /// True when a seat drives the body (a player), not a brain.
    pub driven: bool,
    pub position: [f32; 2],
    /// The body's full size.
    pub size: [f32; 2],
    /// `1.0` faces +x (body-local), `-1.0` faces -x.
    pub facing: f32,
    pub frame_side: [f32; 2],
    pub frame_down: [f32; 2],
    pub aim_local: [f32; 2],
    /// The body's mana now, before any use this tick.
    pub mana: Option<f32>,
    /// The body can name what it spawns (it has a `SimId` and a
    /// `SimIdCounter`). A module that asked is never refused by
    /// `ambition.world.spawn_module_entity`.
    pub names_spawns: bool,
}

/// A bank pays a cost when it holds at least the cost less this. The bank's
/// own rule (`ActorResources::can_pay`); `ambition_abilities::extension` tests
/// that the two agree.
pub const PAY_EPSILON: f32 = 1e-6;

impl Wielder {
    /// A body-local vector in the world.
    pub fn to_world(&self, local: [f32; 2]) -> [f32; 2] {
        [
            self.frame_side[0] * local[0] + self.frame_down[0] * local[1],
            self.frame_side[1] * local[0] + self.frame_down[1] * local[1],
        ]
    }

    /// A body-local box half size in the world (the axis-aligned box that
    /// holds the turned box).
    pub fn to_world_half(&self, half: [f32; 2]) -> [f32; 2] {
        [
            (self.frame_side[0] * half[0]).abs() + (self.frame_down[0] * half[1]).abs(),
            (self.frame_side[1] * half[0]).abs() + (self.frame_down[1] * half[1]).abs(),
        ]
    }

    /// True when the body's mana pays `cost`.
    pub fn can_pay_mana(&self, cost: f32) -> bool {
        self.mana.is_some_and(|mana| mana + PAY_EPSILON >= cost)
    }
}

impl Port for WieldedUsePort {
    const KEY: PortKey = PortKey::new("ambition.items.wielded_use", 2);
    const ROLE: PortRole = PortRole::Trigger;
    type Value = Wielder;

    fn encode(v: &Wielder, out: &mut Vec<u8>) {
        wire::put_bool(out, v.pressed);
        wire::put_bool(out, v.driven);
        wire::put_vec2(out, v.position);
        wire::put_vec2(out, v.size);
        wire::put_f32(out, v.facing);
        wire::put_vec2(out, v.frame_side);
        wire::put_vec2(out, v.frame_down);
        wire::put_vec2(out, v.aim_local);
        wire::put_opt(out, v.mana, wire::put_f32);
        wire::put_bool(out, v.names_spawns);
    }

    fn decode(r: &mut WireReader<'_>) -> Result<Wielder, WireError> {
        Ok(Wielder {
            pressed: r.bool()?,
            driven: r.bool()?,
            position: r.vec2()?,
            size: r.vec2()?,
            facing: r.f32()?,
            frame_side: r.vec2()?,
            frame_down: r.vec2()?,
            aim_local: r.vec2()?,
            mana: r.opt(WireReader::f32)?,
            names_spawns: r.bool()?,
        })
    }
}

/// The request port marker for paying mana.
///
/// Port card:
///
/// * **Operation** — take `amount` from the body's mana pool.
/// * **Owner** — `ambition_abilities::extension` (the bank's `pay`).
/// * **Scope** — the body the invocation ran for.
/// * **Time** — `wielded_use`, this tick.
/// * **Result** — a body that cannot pay pays nothing, and the refusal is
///   logged. A module that read [`Wielder::mana`] and asked
///   [`Wielder::can_pay_mana`] is never refused: the read and the payment are
///   one tick and one spender.
pub struct SpendManaPort;

#[derive(Clone, Debug, PartialEq)]
pub struct SpendMana {
    pub amount: f32,
}

impl Port for SpendManaPort {
    const KEY: PortKey = PortKey::new("ambition.resources.spend_mana", 1);
    const ROLE: PortRole = PortRole::Request;
    type Value = SpendMana;

    fn encode(v: &SpendMana, out: &mut Vec<u8>) {
        wire::put_f32(out, v.amount);
    }

    fn decode(r: &mut WireReader<'_>) -> Result<SpendMana, WireError> {
        Ok(SpendMana { amount: r.f32()? })
    }
}

/// The request port marker for a sound a body makes.
///
/// Port card:
///
/// * **Operation** — play the cue `cue` (an authored sound id, for example
///   `world.rock.hit`) at `at`, as the body's sound.
/// * **Owner** — `ambition_abilities::extension` (the body's sound writer).
/// * **Time** — `wielded_use`. Presentation: no simulation state reads it.
pub struct BodySoundPort;

#[derive(Clone, Debug, PartialEq)]
pub struct BodySound {
    pub cue: String,
    pub at: [f32; 2],
}

impl Port for BodySoundPort {
    const KEY: PortKey = PortKey::new("ambition.feedback.body_sound", 1);
    const ROLE: PortRole = PortRole::Request;
    type Value = BodySound;

    fn encode(v: &BodySound, out: &mut Vec<u8>) {
        wire::put_str(out, &v.cue);
        wire::put_vec2(out, v.at);
    }

    fn decode(r: &mut WireReader<'_>) -> Result<BodySound, WireError> {
        Ok(BodySound {
            cue: r.str()?.to_owned(),
            at: r.vec2()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_wielder_survives_the_wire() {
        let w = Wielder {
            pressed: true,
            driven: false,
            position: [1.0, 2.0],
            size: [20.0, 30.0],
            facing: -1.0,
            frame_side: [0.0, -1.0],
            frame_down: [1.0, 0.0],
            aim_local: [0.5, 0.5],
            mana: Some(42.0),
            names_spawns: true,
        };
        let mut out = Vec::new();
        WieldedUsePort::encode(&w, &mut out);
        let mut r = WireReader::new(&out);
        assert_eq!(WieldedUsePort::decode(&mut r).unwrap(), w);
        r.finish().unwrap();
        // Gravity to +x: body-local "down" is world +x.
        assert_eq!(w.to_world([0.0, 1.0]), [1.0, 0.0]);
        assert_eq!(w.to_world_half([10.0, 2.0]), [2.0, 10.0]);
    }
}
