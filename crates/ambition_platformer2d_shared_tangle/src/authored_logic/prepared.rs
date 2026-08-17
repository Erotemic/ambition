//! **Preparation: how authored text becomes a call the simulation can make
//! without ever seeing the text.**
//!
//! [The condition contract](super) lets a domain publish a question; [the
//! command contract](super::commands) lets it publish a verb. Both take
//! [`AuthoredArg`] values — *prepared* arguments — and neither says where a
//! prepared argument comes from. Every consumer so far built its own: a lock
//! wall rebuilt `AuthoredArg::Name(wall.gated_by.clone())` from a `String` on
//! every tick, and the Yarn command bridge grew its own text conversion.
//!
//! This is that step, once. Authored source in, a [`PreparedCondition`] or a
//! [`PreparedCommand`] out, and a refusal with a reason in between.
//!
//! # ⭐⭐ The four properties, and why each is STRUCTURAL rather than promised
//!
//! **1. Validation cannot be skipped, because there is no other way in.** Both
//! prepared types have private fields and no public constructor. The only
//! functions that produce one are [`ConditionCatalog::prepare`] and
//! [`CommandCatalog::prepare`], which check the id against the published
//! catalog, the argument count against the descriptor's arity, and every value
//! against its declared [`ParamKind`]. ⇒ **a prepared call that was never
//! validated is not a state this program can be in** — it is unconstructible,
//! the same way an unpublishable catalog row is.
//!
//! **2. The runtime parses nothing, because the runtime holds no text.** The
//! authored source is consumed by `prepare` and is **not stored** on the
//! prepared value. What survives is a [`ConditionId`]/[`CommandId`] and a
//! `Vec<AuthoredArg>`. ⛔ there is no accessor that returns the source line,
//! which is what makes *"nothing parses an expression string during
//! simulation"* a shape rather than a rule somebody has to keep.
//!
//! **3. Program data is immutable, because nothing can mutate it.** No `&mut`
//! accessor, no public field, no interior mutability. A holder may REPLACE a
//! prepared call with another validated one; nothing can edit one in place. That
//! is the same argument [`CommandCatalog`]'s private `publish` makes, adapted to
//! a value rather than a registry: the catalogs are safe because a tick cannot
//! reach the door, and a prepared call is safe because no door exists.
//!
//! **4. A reference is a [`SimId`], minted by `SimId`'s own constructors.** ⛔
//! never [`SimId::from_snapshot`], which that module reserves for rebuilding an
//! id from a snapshot blob. The authored text names its namespace —
//! `encounter:symmetry_attunement` — and preparation dispatches to
//! [`SimId::encounter`] / [`SimId::placement`], so the escaping that keeps the
//! id encoding injective happens exactly as it does everywhere else.
//!
//! ⚠ **the author spelling the namespace is a deliberate choice, and the
//! alternative is written down so widening later is a decision.** The other
//! design puts the namespace in the [`ParamSpec`], so a `.ldtk` field could say
//! just `symmetry_attunement`. That needs `ParamKind::Reference` to carry a
//! payload, which breaks the one-line kind check both catalogs share, for the
//! benefit of a shorter string. And an authored reference that names its
//! namespace is *readable in the level*: `placement:kernel` and
//! `encounter:kernel` are two different things and an agent reading the world
//! can tell which one it is looking at.
//!
//! # ⛔ What preparation is NOT
//!
//! ⛔ **not an expression language.** The authored form is `<id> <arg>…` —
//! whitespace-separated, no operators, no nesting, no precedence. An argument
//! containing a space is not expressible, which is a limit worth having: every
//! argument any published condition or command takes is an id, a key, a number
//! or a truth.
//!
//! ⛔ **no program counter, and that is the point.** A prepared call is one
//! call. Nothing here sequences, latches, waits or resumes, so there is no
//! cursor to give rollback semantics to — which is the cheapest possible answer
//! to M0's Finding 4 (*the tree ships three different answers to "is a program
//! counter rollback state?"*). ⚠ **the day a customer genuinely needs a cursor,
//! that is a decision for a human**, not a thing to add here because it was
//! convenient.
//!
//! ⛔ **not a condition/command pairing either.** A `when … then …` rule form
//! was written and cut: the one customer that pays for this — a `Switch` that
//! names a verb — has an EMPTY condition list in all four of its rows, and a
//! shipped `when` with zero adopters is the wrapper this program's own falsifier
//! 2 refuses. The two halves prepare separately and the domain that owns a
//! trigger decides when to ask; see `world::authored_switch_commands`.

use bevy::prelude::World;

use crate::sim_id::SimId;

use super::{
    AuthoredArg, CommandCatalog, CommandId, ConditionCatalog, ConditionId, ConditionOutcome,
    ParamKind, ParamSpec, RunAuthoredCommand,
};

/// **Why one authored line did not become a prepared call.**
///
/// ⚠ **it carries the source**, because the caller that reports this is usually
/// a loader iterating many authored rows and *"takes 2 arguments, got 1"* with
/// no line in it is a diagnostic an author cannot act on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreparationError {
    source: String,
    reason: String,
}

impl PreparationError {
    fn new(source: &str, reason: impl Into<String>) -> Self {
        Self {
            source: source.to_string(),
            reason: reason.into(),
        }
    }

    /// The authored text that failed. ⚠ this is the ERROR's copy — a diagnostic —
    /// and is deliberately the only place the source survives preparation.
    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn reason(&self) -> &str {
        &self.reason
    }
}

impl std::fmt::Display for PreparationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.source, self.reason)
    }
}

/// **One validated question, ready to be asked.**
///
/// ⭐ private fields and no public constructor — see this module's header on why
/// that is what makes *"validation happens before runtime"* structural.
#[derive(Clone, Debug, PartialEq)]
pub struct PreparedCondition {
    id: ConditionId,
    args: Vec<AuthoredArg>,
}

impl PreparedCondition {
    pub fn id(&self) -> &ConditionId {
        &self.id
    }

    pub fn args(&self) -> &[AuthoredArg] {
        &self.args
    }
}

/// **One validated verb, ready to be requested.**
#[derive(Clone, Debug, PartialEq)]
pub struct PreparedCommand {
    id: CommandId,
    args: Vec<AuthoredArg>,
}

impl PreparedCommand {
    pub fn id(&self) -> &CommandId {
        &self.id
    }

    pub fn args(&self) -> &[AuthoredArg] {
        &self.args
    }
}

impl RunAuthoredCommand {
    /// **Ask for a prepared verb.** The only bridge from preparation to the
    /// request channel, so a requester never assembles arguments by hand.
    pub fn prepared(call: &PreparedCommand) -> Self {
        Self::new(call.id.clone(), call.args.clone())
    }
}

impl ConditionCatalog {
    /// **Prepare one question from authored source.**
    ///
    /// The id is already parsed because a caller that spells its own question in
    /// Rust (a lock wall asking `world.flag_set`) should not have to round-trip
    /// through a string to reach validation. Authored source that names the id
    /// too goes through [`ConditionCatalog::prepare_line`].
    pub fn prepare(
        &self,
        id: ConditionId,
        args: &[&str],
    ) -> Result<PreparedCondition, PreparationError> {
        let source = describe_source(id.as_str(), args);
        let Some(descriptor) = self.describe(&id) else {
            return Err(PreparationError::new(
                &source,
                format!(
                    "no condition `{id}` is published; the installed engine knows {} others",
                    self.len()
                ),
            ));
        };
        let args = prepare_args(&source, id.as_str(), descriptor.params, args)?;
        Ok(PreparedCondition { id, args })
    }

    /// **Prepare one question from a whole authored line** — `"world.flag_set
    /// bob_field_survey_received"`.
    pub fn prepare_line(&self, source: &str) -> Result<PreparedCondition, PreparationError> {
        let (raw_id, args) = split_line(source)?;
        let Some(id) = ConditionId::parse(raw_id) else {
            return Err(PreparationError::new(
                source,
                format!("{raw_id:?} is not a `domain.question` id"),
            ));
        };
        self.prepare(id, &args)
    }

    /// **Ask a prepared question.**
    ///
    /// ⭐ the point of the prepared form: the tick evaluates, and everything that
    /// could have been wrong about the call was already wrong at prepare time.
    pub fn ask(&self, world: &World, prepared: &PreparedCondition) -> ConditionOutcome {
        self.evaluate(world, &prepared.id, &prepared.args)
    }
}

impl CommandCatalog {
    /// **Prepare one verb from authored source.**
    pub fn prepare(
        &self,
        id: CommandId,
        args: &[&str],
    ) -> Result<PreparedCommand, PreparationError> {
        let source = describe_source(id.as_str(), args);
        let Some(descriptor) = self.describe(&id) else {
            return Err(PreparationError::new(
                &source,
                format!(
                    "no command `{id}` is published; the installed engine knows {} others",
                    self.len()
                ),
            ));
        };
        let args = prepare_args(&source, id.as_str(), descriptor.params, args)?;
        Ok(PreparedCommand { id, args })
    }

    /// **Prepare one verb from a whole authored line** — `"encounter.signal
    /// encounter:symmetry_attunement gravity_down"`.
    ///
    /// ⭐ this is the form an authored FIELD carries, because a level author
    /// writes one string and the number of arguments is the verb's business
    /// rather than the field's.
    pub fn prepare_line(&self, source: &str) -> Result<PreparedCommand, PreparationError> {
        let (raw_id, args) = split_line(source)?;
        let Some(id) = CommandId::parse(raw_id) else {
            return Err(PreparationError::new(
                source,
                format!("{raw_id:?} is not a `domain.verb` id"),
            ));
        };
        self.prepare(id, &args)
    }
}

/// Split `"<id> <arg>…"` on whitespace.
///
/// ⚠ **it never repairs and never quotes.** An argument containing a space is
/// not expressible; adding quoting would be the first inch of the expression
/// language this module's header refuses.
fn split_line(source: &str) -> Result<(&str, Vec<&str>), PreparationError> {
    let mut parts = source.split_whitespace();
    let Some(raw_id) = parts.next() else {
        return Err(PreparationError::new(
            source,
            "an authored call needs an id; this line is blank",
        ));
    };
    Ok((raw_id, parts.collect()))
}

/// What the diagnostic calls the line, whether or not there was one.
fn describe_source(id: &str, args: &[&str]) -> String {
    let mut source = id.to_string();
    for arg in args {
        source.push(' ');
        source.push_str(arg);
    }
    source
}

/// **Turn authored text into the arguments the published descriptor declares.**
///
/// ⭐⭐ **the descriptor decides the kind; the authored text only has to fit.**
/// The alternative — guess the kind from the text — is the lossy conversion that
/// silently turns a flag named `"1"` into a number, and it is the reason this
/// lives beside the descriptors rather than in each consumer.
fn prepare_args(
    source: &str,
    id: &str,
    params: &[ParamSpec],
    texts: &[&str],
) -> Result<Vec<AuthoredArg>, PreparationError> {
    if texts.len() != params.len() {
        return Err(PreparationError::new(
            source,
            format!(
                "`{id}` takes {} argument(s) ({}), got {}",
                params.len(),
                params
                    .iter()
                    .map(|param| param.name)
                    .collect::<Vec<_>>()
                    .join(", "),
                texts.len()
            ),
        ));
    }
    params
        .iter()
        .zip(texts)
        .map(|(param, text)| {
            prepare_one(id, param, text).map_err(|reason| PreparationError::new(source, reason))
        })
        .collect()
}

fn prepare_one(id: &str, param: &ParamSpec, text: &str) -> Result<AuthoredArg, String> {
    match param.kind {
        ParamKind::Name => Ok(AuthoredArg::Name(text.to_string())),
        ParamKind::Number => text.parse::<f64>().map(AuthoredArg::Number).map_err(|_| {
            format!(
                "`{id}` argument `{}` is a Number, and the authored value {text:?} is not one",
                param.name
            )
        }),
        // ⚠ **exactly `true` / `false`, with no `1`, `yes` or `on`.** A verb that
        // accepted four spellings of truth would accept a fifth by accident, and a
        // mistyped one would read as `false` — which is a flag being CLEARED when
        // the author meant to set it.
        ParamKind::Truth => match text {
            "true" => Ok(AuthoredArg::Truth(true)),
            "false" => Ok(AuthoredArg::Truth(false)),
            other => Err(format!(
                "`{id}` argument `{}` is a Truth, and the authored value {other:?} is \
                 neither `true` nor `false`",
                param.name
            )),
        },
        ParamKind::Reference => prepare_reference(id, param, text),
    }
}

/// **`<namespace>:<id>` → the matching [`SimId`] constructor.**
///
/// ⛔ **never [`SimId::from_snapshot`]**, which is reserved for rebuilding an id
/// from a snapshot blob. Going through the real constructor is what applies the
/// escaping that keeps the id encoding injective — so an authored
/// `encounter:a:b` prepares to `SimId::encounter("a:b")` and cannot collide with
/// anything else the vocabulary can mint.
///
/// ⚠ **only the two namespaces an AUTHOR can name.** `slot:`, a spawned id and a
/// strike volume are minted by the running simulation from facts no level
/// knows — a placement and an encounter are the two an authored world actually
/// contains.
fn prepare_reference(id: &str, param: &ParamSpec, text: &str) -> Result<AuthoredArg, String> {
    let Some((namespace, body)) = text.split_once(':') else {
        return Err(format!(
            "`{id}` argument `{}` is a prepared reference, and the authored value \
             {text:?} names no namespace — write `placement:<id>` or `encounter:<id>`",
            param.name
        ));
    };
    if body.is_empty() {
        return Err(format!(
            "`{id}` argument `{}`: the authored reference {text:?} names a namespace \
             and nothing else",
            param.name
        ));
    }
    match namespace {
        "placement" => Ok(AuthoredArg::Reference(SimId::placement(body))),
        "encounter" => Ok(AuthoredArg::Reference(SimId::encounter(body))),
        other => Err(format!(
            "`{id}` argument `{}`: {other:?} is not an authorable identity namespace; \
             an authored world names `placement:<id>` or `encounter:<id>`",
            param.name
        )),
    }
}

#[cfg(test)]
mod tests;
