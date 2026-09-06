//! Validated authored calls used by the simulation.
//!
//! [`ConditionCatalog::prepare`] and [`CommandCatalog::prepare`] validate the id,
//! arity, and argument kinds before constructing a [`PreparedCondition`] or
//! [`PreparedCommand`]. Their fields are private, so unchecked calls cannot be
//! constructed. Prepared values contain only ids and typed [`AuthoredArg`] values;
//! runtime code does not retain or parse the authored source text. References are
//! explicit namespaced [`SimId`] values.
//!
//! Preparation handles one whitespace-delimited call at a time. It is not an
//! expression language, rule sequencer, or condition/command pairing.

use bevy::prelude::World;

use crate::sim_id::SimId;

use super::{
    AuthoredArg, CommandCatalog, CommandId, ConditionCatalog, ConditionId, ConditionOutcome,
    ParamKind, ParamSpec, RunAuthoredCommand,
};

/// Why one authored line did not become a prepared call.
///
/// it carries the source, because the caller that reports this is usually
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

    /// The authored text that failed. this is the ERROR's copy — a diagnostic —
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

/// One validated question, ready to be asked.
///
/// private fields and no public constructor — see this module's header on why
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

/// One validated verb, ready to be requested.
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
    /// Ask for a prepared verb. The only bridge from preparation to the
    /// request channel, so a requester never assembles arguments by hand.
    pub fn prepared(call: &PreparedCommand) -> Self {
        Self::new(call.id.clone(), call.args.clone())
    }
}

impl ConditionCatalog {
    /// Prepare one question from authored source.
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

    /// Prepare one question from a whole authored line — `"world.flag_set
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

    /// Ask a prepared question.
    ///
    /// the point of the prepared form: the tick evaluates, and everything that
    /// could have been wrong about the call was already wrong at prepare time.
    pub fn ask(&self, world: &World, prepared: &PreparedCondition) -> ConditionOutcome {
        self.evaluate(world, &prepared.id, &prepared.args)
    }
}

impl CommandCatalog {
    /// Prepare one verb from authored source.
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

    /// Prepare one verb from a whole authored line — `"encounter.signal
    /// encounter:symmetry_attunement gravity_down"`.
    ///
    /// this is the form an authored FIELD carries, because a level author
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
/// it never repairs and never quotes. An argument containing a space is
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

/// Turn authored text into the arguments the published descriptor declares.
///
/// the descriptor decides the kind; the authored text only has to fit.
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

/// Convert ONE authored argument text into the [`AuthoredArg`] its declared
/// [`ParamSpec`] asks for.
///
/// ⭐⭐ THE ONE AUTHORITY ON WHAT AN AUTHORED VALUE MEANS — what spellings of
/// truth are accepted, what a Number will parse, how a reference is minted. It is
/// public because a SECOND surface needs it: authored dialogue hands a command
/// its parameters as untyped text and had grown a byte-identical copy of the
/// Name/Number/Truth arms, comment included. Two authorities on "what `true`
/// means" is exactly the defect the Truth arm's own comment warns about — a
/// fifth spelling accepted by accident on one road and not the other.
///
/// ⛔ A CALLER THAT CANNOT EXPRESS AN IDENTITY REFUSES `ParamKind::Reference`
/// ITSELF, before delegating, so its refusal can say WHY in its own terms. This
/// function mints the reference for callers that can.
pub fn prepare_authored_arg(
    id: &str,
    param: &ParamSpec,
    text: &str,
) -> Result<AuthoredArg, String> {
    prepare_one(id, param, text)
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
        // exactly `true` / `false`, with no `1`, `yes` or `on`. A verb that
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

/// `<namespace>:<id>` → the matching [`SimId`] constructor.
///
/// never [`SimId::from_snapshot`], which is reserved for rebuilding an id
/// from a snapshot blob. Going through the real constructor is what applies the
/// escaping that keeps the id encoding injective — so an authored
/// `encounter:a:b` prepares to `SimId::encounter("a:b")` and cannot collide with
/// anything else the vocabulary can mint.
///
/// only the two namespaces an AUTHOR can name. `slot:`, a spawned id and a
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
