//! A schema for one file of rules: a RON value that its owner reads whole.
//!
//! Most of a game's tuning is this shape: a struct of numbers, each checked
//! for a range. [`rules_schema`] gives the struct a complete handler, so the
//! owner writes the struct and its range check and nothing else: the parse,
//! the refusal of an unknown field, the problems named by field and the
//! lowered value are the same for every such file.

use std::fmt::Debug;
use std::marker::PhantomData;
use std::sync::Arc;

use serde::de::DeserializeOwned;

use crate::{
    CapabilityId, ContentSchemaHandler, DiagnosticCode, FacetOutcome, FacetSource,
    PreparedContentPack, RuntimeDisposition, SchemaId, SchemaRegistration, SchemaVersion,
};

/// The schema of a file of rules `T`, owned by `capability`.
///
/// `problems` names each value the owner cannot use (for example
/// "`rounds_to_win` is 0; ..."). Each one refuses the file at admission, with
/// the file named. Give `T` `#[serde(deny_unknown_fields)]`: a field the
/// owner does not read is then refused as
/// [`DiagnosticCode::UnknownField`], and a typing mistake is not a rule that
/// never applies.
///
/// Read the lowered value with [`lowered_rules`].
pub fn rules_schema<T>(
    id: &'static str,
    version: u32,
    capability: &str,
    doc: &'static str,
    problems: fn(&T) -> Vec<String>,
) -> SchemaRegistration
where
    T: DeserializeOwned + Debug + Send + Sync + 'static,
{
    SchemaRegistration {
        id: SchemaId::new(id),
        version: SchemaVersion(version),
        capability: CapabilityId::new(capability),
        disposition: RuntimeDisposition::Runtime,
        doc,
        handler: Arc::new(RulesSchema::<T> { id, problems, read: PhantomData }),
    }
}

/// The rules `T` the pack states in its `id` source, or `None` when it
/// declares no such source.
pub fn lowered_rules<'pack, T: Send + Sync + 'static>(
    pack: &'pack PreparedContentPack,
    id: &str,
) -> Option<&'pack T> {
    pack.lowered::<T>(&SchemaId::new(id))
}

struct RulesSchema<T> {
    id: &'static str,
    problems: fn(&T) -> Vec<String>,
    read: PhantomData<fn() -> T>,
}

impl<T> ContentSchemaHandler for RulesSchema<T>
where
    T: DeserializeOwned + Debug + Send + Sync + 'static,
{
    fn check(&self, facet: &FacetSource<'_>, out: &mut FacetOutcome) {
        let authored = match facet.parse::<T>() {
            Ok(authored) => authored,
            Err(diagnostic) => {
                out.report(diagnostic);
                return;
            }
        };
        let id = facet.content_id_in(self.id, "rules");
        out.define(id.clone(), format!("{authored:?}"));
        for problem in (self.problems)(&authored) {
            out.report(
                facet
                    .diagnostic(DiagnosticCode::MalformedSource, problem)
                    .about(id.clone()),
            );
        }
        if !out.failed() {
            out.lower(authored);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{compile, AssetsUnchecked, CompileFailure, ContentPackDraft, SchemaRegistry};

    #[derive(Debug, PartialEq, serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Rounds {
        to_win: u8,
    }

    fn problems(rounds: &Rounds) -> Vec<String> {
        if rounds.to_win == 0 {
            vec!["`to_win` is 0; a match is taken by at least one round".to_string()]
        } else {
            Vec::new()
        }
    }

    fn compiled(text: &str) -> Result<PreparedContentPack, CompileFailure> {
        let mut registry = SchemaRegistry::new();
        registry
            .register(rules_schema::<Rounds>("probe_rounds", 1, "probe", "a probe's rounds", problems))
            .expect("one registration");
        let draft = ContentPackDraft::from_manifest_ron(
            r#"(id: "probe", version: "0.1.0", namespace: "probe", requires: [],
                sources: [(path: "rounds.ron", schema: "probe_rounds", version: 1)])"#,
            [("rounds.ron".to_string(), text.to_string())],
        )?;
        compile(&draft, &registry, &AssetsUnchecked)
    }

    /// The file lowers to its struct; a value the owner refuses and a field
    /// it does not read are each refused at admission, with the file named.
    #[test]
    fn a_rules_file_lowers_and_each_refusal_names_the_file() {
        let pack = compiled("(to_win: 3)").unwrap_or_else(|failure| panic!("{}", failure.render()));
        assert_eq!(lowered_rules::<Rounds>(&pack, "probe_rounds"), Some(&Rounds { to_win: 3 }));

        for (text, code, says) in [
            ("(to_win: 0)", DiagnosticCode::MalformedSource, "`to_win` is 0"),
            ("(to_win: 2, to_lose: 1)", DiagnosticCode::UnknownField, "to_lose"),
        ] {
            let failure = compiled(text).expect_err(text);
            let rendered = failure.render();
            assert!(failure.has(code), "{text}: {rendered}");
            assert!(rendered.contains(says) && rendered.contains("rounds.ron"), "{text}: {rendered}");
        }
    }
}
