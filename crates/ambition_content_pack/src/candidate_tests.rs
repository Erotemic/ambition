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
/// ⛔ IT DEFINES AS WELL AS LOWERING. A schema whose authored values reach the
/// game without reaching the pack's IDENTITY is exactly the thing every arm here
/// would then be blind to, and this crate's lower-must-define rule refuses it.
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
/// ⛔ TWO IS THE MINIMUM THAT CAN EXPRESS THE DEFECT. A one-section pack makes
/// "the section changed" and "the pack changed" the same statement, so every arm
/// below would pass against exactly the reasoning this module exists to refuse.
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

const A: &str = "alpha-one";
const A2: &str = "alpha-two";
const B: &str = "beta-one";
const B2: &str = "beta-two";

/// ⭐ THE PREMISE. Without it every arm below compares two packs that are the
/// same pack, and "changing a section changes the identity" is vacuous.
#[test]
fn changing_either_section_changes_the_whole_packs_fingerprint() {
    let base = pack_of(A, B).fingerprint;
    assert_ne!(base, pack_of(A2, B).fingerprint, "alpha does not reach it");
    assert_ne!(base, pack_of(A, B2).fingerprint, "beta does not reach it");
    assert_eq!(base, pack_of(A, B).fingerprint, "it is not deterministic");
}

/// ⛔⛔ **THE DEFECT THIS MODULE EXISTS FOR.** One family's section is identical
/// and another's is not; the verdict must be `Publish`, because the WHOLE pack
/// changed. Inferring "unchanged" from the family that triggered the reload is
/// what let a candidate be selected while a subsystem believed nothing moved.
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

/// ⛔ A COMPLETE no-op: every section identical.
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

/// ⛔ STALENESS IS ASKED FIRST, and this is the arm that proves the ORDER rather
/// than the rule. The candidate is mechanically identical to what is live, and
/// it is still refused — because the base it was prepared against is gone, so
/// the caller's premise is false even though its bytes happen to match.
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

/// ⚠ `None` IS "NO CLAIM", not "against nothing". A caller that compiled and
/// published without yielding has nothing to be stale against, and treating a
/// missing base as a mismatch would refuse every such reload.
#[test]
fn a_candidate_that_makes_no_base_claim_is_not_stale() {
    let live = pack_of(A, B);
    let candidate = CandidateGeneration::prepared_against(pack_of(A, B2), None);
    assert!(matches!(
        candidate.verdict(Some(live.fingerprint)),
        CandidateVerdict::Publish { .. }
    ));
}

/// ⚠ AND A HOST THAT HAS SELECTED NOTHING IS A FIRST PUBLICATION, not a no-op.
#[test]
fn a_candidate_against_no_active_generation_publishes() {
    let candidate = CandidateGeneration::prepared_against(pack_of(A, B), None);
    assert!(matches!(
        candidate.verdict(None),
        CandidateVerdict::Publish { .. }
    ));
}
