//! The ports of a reaction to a hit: what a body does about how this tick's
//! hits came out, decided by a module in `hit_reaction`
//! (`ambition_extension_sdk::phases::HIT_REACTION`).
//!
//! A counter is the first: a stance's window sustains an effect, a parry is
//! caught inside it, and the stance's module decides the answer. A strike
//! that carries an effect is the second ([`StruckPort`]): the module of the
//! effect decides what it leaves on the body it struck.

use ambition_extension_sdk::wire::{WireError, WireReader};
use ambition_extension_sdk::{Port, PortKey, PortRole};

use crate::special::SpecialParam;

/// The trigger port marker for a parry a body caught.
///
/// Port card:
///
/// * **Operation** — the scope body caught a parry this tick while the window
///   under its move's clock sustains the effect whose key is the selector
///   (`smash.counter`). A parry with no such window invokes nothing: an
///   ordinary parry is complete on its own.
/// * **Owner** — `ambition_combat::extension::reaction`, which reads
///   `ParriedBodyHit`.
/// * **Scope** — the defender. One invocation for each parry, in the order the
///   parries were caught.
/// * **Time** — `hit_reaction`: the parry is this tick's.
/// * **Read model** — world units, +Y down. `parry` names this parry for
///   [`AnswerParryPort`] (this tick only). `params` are the sustained
///   effect's authored params, each leaf by its path, as
///   [`crate::FighterSpecial`] gives a move's.
pub struct ParriedPort;

#[derive(Clone, Debug, PartialEq)]
pub struct Parried {
    pub parry: u32,
    pub contact: [f32; 2],
    pub params: Vec<(String, SpecialParam)>,
}

impl Parried {
    /// The number at `path` of the stance's params.
    pub fn number(&self, path: &str) -> Option<f32> {
        crate::special::number_at(&self.params, path)
    }

    /// The text at `path` of the stance's params.
    pub fn text(&self, path: &str) -> Option<&str> {
        crate::special::text_at(&self.params, path)
    }

    /// The flag at `path` of the stance's params.
    pub fn flag(&self, path: &str) -> Option<bool> {
        crate::special::flag_at(&self.params, path)
    }
}

impl Port for ParriedPort {
    const KEY: PortKey = PortKey::new("ambition.combat.parried", 1);
    const ROLE: PortRole = PortRole::Trigger;
    type Value = Parried;

    fn encode(v: &Parried, out: &mut Vec<u8>) {
        ambition_extension_sdk::structure::put(v, out)
    }

    fn decode(r: &mut WireReader<'_>) -> Result<Parried, WireError> {
        ambition_extension_sdk::structure::read(r)
    }
}

/// The request port marker for an answer to a parry.
///
/// Port card:
///
/// * **Operation** — the technique `key` is pressed for the defender, or with
///   `on_attacker` for the body whose attack was parried, with the params
///   found at `params_at` in the stance's authored params, whole (a path in
///   the syntax of [`Parried::params`]: `response_params`,
///   `response_params[0]`; empty: no params). A path the stance states
///   nothing at is refused. The technique is the game's to answer, a
///   module's or native, as for any press.
/// * **Owner** — `ambition_combat::extension::reaction`, which writes
///   `ActorActionMessage`.
/// * **Scope and grant** — the defender, and only for a parry of its own: a
///   `parry` that is not one this tick gave the scope is refused, so a module
///   names only an attacker it was given.
/// * **Time** — `hit_reaction`; the press is read on the next tick.
pub struct AnswerParryPort;

#[derive(Clone, Debug, PartialEq)]
pub struct AnswerParry {
    pub parry: u32,
    pub key: String,
    pub params_at: String,
    pub on_attacker: bool,
}

impl Port for AnswerParryPort {
    const KEY: PortKey = PortKey::new("ambition.combat.answer_parry", 1);
    const ROLE: PortRole = PortRole::Request;
    type Value = AnswerParry;

    fn encode(v: &AnswerParry, out: &mut Vec<u8>) {
        ambition_extension_sdk::structure::put(v, out)
    }

    fn decode(r: &mut WireReader<'_>) -> Result<AnswerParry, WireError> {
        ambition_extension_sdk::structure::read(r)
    }
}

/// The trigger port marker for a strike that carried an effect to a body.
///
/// Port card:
///
/// * **Operation** — a strike landed on the scope body this tick, and the
///   volume that landed authors an on-hit effect whose key is the selector
///   (`smash.mark_body`). `params` are that effect's authored params, each
///   leaf by its path, as [`crate::FighterSpecial`] gives a move's.
/// * **Owner** — `ambition_combat::extension::reaction`, which reads
///   `OnHitEffectMessage`.
/// * **Scope** — the body that was struck, so the module's records and
///   statuses are the struck body's. One invocation for each strike, in the
///   order the strikes landed.
/// * **Time** — `hit_reaction`: the strike is this tick's.
/// * **Read model** — world units, +Y down. `striker` is the credit of the
///   body whose move struck: a token that names it durably (across a rewind,
///   on every peer) for a later verb that credits it
///   ([`crate::BlastPort`]); `0` when the striker has no simulation identity,
///   so it cannot be credited.
pub struct StruckPort;

#[derive(Clone, Debug, PartialEq)]
pub struct Struck {
    pub contact: [f32; 2],
    pub striker: u64,
    pub params: Vec<(String, SpecialParam)>,
}

impl Struck {
    /// The number at `path` of the effect's params.
    pub fn number(&self, path: &str) -> Option<f32> {
        crate::special::number_at(&self.params, path)
    }

    /// The text at `path` of the effect's params.
    pub fn text(&self, path: &str) -> Option<&str> {
        crate::special::text_at(&self.params, path)
    }

    /// The flag at `path` of the effect's params.
    pub fn flag(&self, path: &str) -> Option<bool> {
        crate::special::flag_at(&self.params, path)
    }
}

impl Port for StruckPort {
    const KEY: PortKey = PortKey::new("ambition.combat.struck", 1);
    const ROLE: PortRole = PortRole::Trigger;
    type Value = Struck;

    fn encode(v: &Struck, out: &mut Vec<u8>) {
        ambition_extension_sdk::structure::put(v, out)
    }

    fn decode(r: &mut WireReader<'_>) -> Result<Struck, WireError> {
        ambition_extension_sdk::structure::read(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip<P: Port + ambition_extension_sdk::script::ScriptPort>(value: P::Value) -> P::Value {
        let mut bytes = Vec::new();
        P::encode(&value, &mut bytes);
        let scripted = P::from_script(&P::to_script(&value)).unwrap();
        let mut script_bytes = Vec::new();
        P::encode(&scripted, &mut script_bytes);
        assert_eq!(script_bytes, bytes);
        P::decode(&mut WireReader::new(&bytes)).unwrap()
    }

    /// The reaction values decode to what was encoded.
    #[test]
    fn reaction_values_cross_the_wire_unchanged() {
        let parried = Parried {
            parry: 2,
            contact: [1.0, -3.0],
            params: vec![("answers_the_attacker".into(), SpecialParam::Flag(true))],
        };
        assert_eq!(round_trip::<ParriedPort>(parried.clone()), parried);
        let answer = AnswerParry {
            parry: 2,
            key: "smash.time_dilation".into(),
            params_at: "response_params".into(),
            on_attacker: true,
        };
        assert_eq!(round_trip::<AnswerParryPort>(answer.clone()), answer);
        let struck = Struck {
            contact: [4.0, 5.0],
            striker: u64::MAX - 3,
            params: vec![("fuse_s".into(), SpecialParam::Number(1.4))],
        };
        assert_eq!(round_trip::<StruckPort>(struck.clone()), struck);
    }
}

ambition_extension_sdk::port_record! {
    Parried {
        /// Phase-local parry index for answer_parry.
        parry: u32,
        contact: [f32; 2],
        params: Vec<(String, SpecialParam)> as ambition_extension_sdk::structure::TextMap<SpecialParam>,
    }
}
ambition_extension_sdk::port_record! {
    AnswerParry {
        parry: u32,
        key: String,
        params_at: String = String::new(),
        on_attacker: bool = false,
    }
}
ambition_extension_sdk::port_record! {
    Struck {
        contact: [f32; 2],
        /// Opaque credit token of the striker, or "0" when absent.
        striker: u64 as crate::blast::CreditToken,
        params: Vec<(String, SpecialParam)> as ambition_extension_sdk::structure::TextMap<SpecialParam>,
    }
}
ambition_extension_sdk::script_port!(ParriedPort);
ambition_extension_sdk::script_port!(AnswerParryPort);
ambition_extension_sdk::script_port!(StruckPort);
