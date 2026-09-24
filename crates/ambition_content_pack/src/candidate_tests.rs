use super::*;
use crate::{
    compile, AggregateOutcome, Aggregation, AssetsUnchecked, CapabilityId, ContentPackDraft,
    ContentPackManifest, ContentSchemaHandler, FacetOutcome, FacetSource, LoweredFragment,
    ModuleNamespace, PackId, PackVersion, RuntimeDisposition, SchemaId, SchemaRegistration,
    SchemaRegistry, SchemaVersion, SourceDeclaration,
};
use std::sync::Arc;

/// A schema that defines one row per source and lowers its text verbatim.
///
/// It defines as well as lowers; the lower-must-define rule refuses a schema
/// whose values reach the game but not the pack's identity.
struct EchoSchema {
    name: &'static str,
}

impl ContentSchemaHandler for EchoSchema {
    fn check(&self, facet: &FacetSource<'_>, out: &mut FacetOutcome) {
        let text = facet.text.trim().to_string();
        out.define(facet.content_id(self.name), text.clone());
        out.lower(text);
    }

    fn aggregate(
        &self,
        fragments: &[LoweredFragment<'_>],
        out: &mut AggregateOutcome,
    ) -> Aggregation {
        let joined: Vec<String> = fragments
            .iter()
            .filter_map(|f| f.get::<String>().cloned())
            .collect();
        out.lower(joined.join(","));
        Aggregation::Defined
    }
}

fn registry() -> SchemaRegistry {
    let mut registry = SchemaRegistry::new();
    for name in ["alpha", "beta"] {
        registry
            .register(SchemaRegistration {
                id: SchemaId::new(name),
                version: SchemaVersion(1),
                capability: CapabilityId::new("probe"),
                disposition: RuntimeDisposition::Runtime,
                doc: "candidate probe schema",
                handler: Arc::new(EchoSchema { name }),
            })
            .expect("fresh registry");
    }
    registry
}

/// Two sections, so a test can change ONE and ask what the whole says.
///
/// Two is the minimum: with one section, "the section changed" and "the pack
/// changed" mean the same thing.
fn pack_of(alpha: &str, beta: &str) -> Arc<PreparedContentPack> {
    let draft = ContentPackDraft::from_sources(
        ContentPackManifest {
            id: PackId("candidate_probe".into()),
            version: PackVersion("1.0.0".into()),
            namespace: ModuleNamespace("probe".into()),
            requires: Vec::new(),
            sources: vec![
                SourceDeclaration {
                    path: "alpha.ron".into(),
                    schema: SchemaId::new("alpha"),
                    version: SchemaVersion(1),
                },
                SourceDeclaration {
                    path: "beta.ron".into(),
                    schema: SchemaId::new("beta"),
                    version: SchemaVersion(1),
                },
            ],
        },
        [
            ("alpha.ron".to_string(), alpha.to_string()),
            ("beta.ron".to_string(), beta.to_string()),
        ],
    )
    .expect("the probe draft assembles");
    Arc::new(compile(&draft, &registry(), &AssetsUnchecked).expect("the probe pack compiles"))
}

/// A pack declaring ONLY the alpha source — a whole family absent.
fn alpha_only(alpha: &str) -> Arc<PreparedContentPack> {
    let draft = ContentPackDraft::from_sources(
        ContentPackManifest {
            id: PackId("candidate_probe".into()),
            version: PackVersion("1.0.0".into()),
            namespace: ModuleNamespace("probe".into()),
            requires: Vec::new(),
            sources: vec![SourceDeclaration {
                path: "alpha.ron".into(),
                schema: SchemaId::new("alpha"),
                version: SchemaVersion(1),
            }],
        },
        [("alpha.ron".to_string(), alpha.to_string())],
    )
    .expect("the probe draft assembles");
    Arc::new(compile(&draft, &registry(), &AssetsUnchecked).expect("the probe pack compiles"))
}

const A: &str = "alpha-one";
const A2: &str = "alpha-two";
const B: &str = "beta-one";
const B2: &str = "beta-two";

/// Premise: changing a section changes the identity.
#[test]
fn changing_either_section_changes_the_whole_packs_fingerprint() {
    let base = pack_of(A, B).fingerprint;
    assert_ne!(base, pack_of(A2, B).fingerprint, "alpha does not reach it");
    assert_ne!(base, pack_of(A, B2).fingerprint, "beta does not reach it");
    assert_eq!(base, pack_of(A, B).fingerprint, "it is not deterministic");
}

/// One family's section is identical and another's is not. The verdict is
/// `Publish`, because the whole pack changed.
#[test]
fn an_identical_section_does_not_make_a_changed_pack_a_no_op() {
    let live = pack_of(A, B);
    let candidate = CandidateGeneration::prepared_against(pack_of(A, B2), Some(live.fingerprint));
    assert_eq!(
        candidate.pack().content.len(),
        live.content.len(),
        "the two packs do not even carry the same sections"
    );
    assert_eq!(
        candidate.verdict(Some(live.fingerprint)),
        CandidateVerdict::Publish {
            fingerprint: candidate.fingerprint()
        },
        "alpha was identical, so the whole pack was called unchanged"
    );
}

/// A complete no-op: every section identical.
#[test]
fn a_mechanically_identical_candidate_is_a_complete_no_op() {
    let live = pack_of(A, B);
    let candidate = CandidateGeneration::prepared_against(pack_of(A, B), Some(live.fingerprint));
    assert_eq!(
        candidate.verdict(Some(live.fingerprint)),
        CandidateVerdict::Unchanged {
            fingerprint: live.fingerprint
        }
    );
}

/// Staleness is checked first: a candidate identical to what is live is still
/// refused when its base is gone.
#[test]
fn a_stale_candidate_is_refused_even_when_it_is_mechanically_identical() {
    let older = pack_of(A2, B2);
    let live = pack_of(A, B);
    let candidate = CandidateGeneration::prepared_against(pack_of(A, B), Some(older.fingerprint));
    assert_eq!(
        candidate.verdict(Some(live.fingerprint)),
        CandidateVerdict::Stale {
            prepared_against: older.fingerprint,
            active: live.fingerprint,
        },
        "a candidate whose base disappeared was reported as a no-op"
    );
}

/// `None` means "no claim", not "against nothing". Treating it as a mismatch
/// would refuse every reload that did not yield.
#[test]
fn a_candidate_that_makes_no_base_claim_is_not_stale() {
    let live = pack_of(A, B);
    let candidate = CandidateGeneration::prepared_against(pack_of(A, B2), None);
    assert!(matches!(
        candidate.verdict(Some(live.fingerprint)),
        CandidateVerdict::Publish { .. }
    ));
}

/// A host that has selected nothing gets a first publication, not a no-op.
#[test]
fn a_candidate_against_no_active_generation_publishes() {
    let candidate = CandidateGeneration::prepared_against(pack_of(A, B), None);
    assert!(matches!(
        candidate.verdict(None),
        CandidateVerdict::Publish { .. }
    ));
}

/// Premise for the domain tests: editing one family's source moves only that
/// family.
#[test]
fn editing_one_domain_changes_only_that_domain() {
    let base = pack_of(A, B);
    let changed = changed_domains(&base, &pack_of(A2, B));
    assert_eq!(
        changed.iter().map(|s| s.0.as_str()).collect::<Vec<_>>(),
        vec!["alpha"],
        "editing alpha reported {changed:?}"
    );
    let changed = changed_domains(&base, &pack_of(A, B2));
    assert_eq!(
        changed.iter().map(|s| s.0.as_str()).collect::<Vec<_>>(),
        vec!["beta"]
    );
}

/// An identical pack changes no domain.
#[test]
fn an_identical_pack_changes_no_domain() {
    assert!(changed_domains(&pack_of(A, B), &pack_of(A, B)).is_empty());
}

/// Both changes at once: the diff must not stop at the first difference.
#[test]
fn editing_two_domains_reports_both() {
    let changed = changed_domains(&pack_of(A, B), &pack_of(A2, B2));
    assert_eq!(
        changed.iter().map(|s| s.0.as_str()).collect::<Vec<_>>(),
        vec!["alpha", "beta"]
    );
}

/// A domain present in only one pack counts as changed. This test guards
/// `.chain(candidate.keys())` in `changed_domains`.
///
/// Test both directions: iterating only the base misses an added family, and
/// iterating only the candidate misses a removed one.
#[test]
fn a_domain_added_or_removed_is_a_changed_domain() {
    let two = pack_of(A, B);
    let one = alpha_only(A);

    let removed = changed_domains(&two, &one);
    assert_eq!(
        removed.iter().map(|s| s.0.as_str()).collect::<Vec<_>>(),
        vec!["beta"],
        "a REMOVED family was not reported as changed"
    );
    let added = changed_domains(&one, &two);
    assert_eq!(
        added.iter().map(|s| s.0.as_str()).collect::<Vec<_>>(),
        vec!["beta"],
        "an ADDED family was not reported as changed"
    );
}
