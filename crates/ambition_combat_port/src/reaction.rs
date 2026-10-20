//! The ports of a reaction to a hit: what a body does about how this tick's
//! hits came out, decided by a module in `hit_reaction`
//! (`ambition_extension_sdk::phases::HIT_REACTION`).
//!
//! A counter is the first: a stance's window sustains an effect, a parry is
//! caught inside it, and the stance's module decides the answer.

use ambition_extension_sdk::wire::{self, WireError, WireReader};
use ambition_extension_sdk::{Port, PortKey, PortRole};

use crate::special::{put_params, read_params, SpecialParam};

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
        wire::put_u32(out, v.parry);
        wire::put_vec2(out, v.contact);
        put_params(out, &v.params);
    }

    fn decode(r: &mut WireReader<'_>) -> Result<Parried, WireError> {
        Ok(Parried { parry: r.u32()?, contact: r.vec2()?, params: read_params(r)? })
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
        wire::put_u32(out, v.parry);
        wire::put_str(out, &v.key);
        wire::put_str(out, &v.params_at);
        wire::put_bool(out, v.on_attacker);
    }

    fn decode(r: &mut WireReader<'_>) -> Result<AnswerParry, WireError> {
        Ok(AnswerParry { parry: r.u32()?, key: r.str()?.to_owned(), params_at: r.str()?.to_owned(), on_attacker: r.bool()? })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip<P: Port>(value: P::Value) -> P::Value {
        let mut bytes = Vec::new();
        P::encode(&value, &mut bytes);
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
    }
}
