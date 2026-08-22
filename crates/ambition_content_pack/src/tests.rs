//! The compiler's own tests: one positive path, and one probe per hard error.
//!
//! The schema used here is deliberately tiny and fake. A real capability's
//! schema is tested by that capability, against its own authored content; what
//! this file owns is the PIPELINE — that each refusal fires, names its subject,
//! and reports the stage it stopped at.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::Arc;

use super::*;

// ── a toy schema, standing in for a capability's own ────────────────────────

const CRITTER: &str = "critter";
const HABITAT: &str = "habitat";
/// One file per burrow: the shape three real families have (boss encounters,
/// dialogue, worlds) and the reason aggregation exists.
const BURROW: &str = "burrow";
const ORDER: &str = "order";

/// Critters name a habitat, a `mood` preset defined in the same file, and a
/// sprite. That is enough shape to exercise a cross-source reference, a LOCAL
/// preset reference, and an asset — the three that fail differently.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct CritterFile {
    #[serde(default)]
    moods: Vec<String>,
    critters: Vec<CritterRow>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct CritterRow {
    name: String,
    habitat: String,
    mood: String,
    sprite: String,
    #[serde(default)]
    needs_capability: Option<String>,
}

struct CritterSchema;

impl ContentSchemaHandler for CritterSchema {
    fn check(&self, facet: &FacetSource<'_>, out: &mut FacetOutcome) {
        let file: CritterFile = match ron::from_str(facet.text) {
            Ok(file) => file,
            Err(error) => {
                // `deny_unknown_fields` turns an unconsumed authored field into
                // a TYPED ron variant, which is why UnknownField is reachable
                // at all — and why this matches the variant rather than
                // grepping the message, which changes between ron releases.
                let code = match error.code {
                    ron::Error::NoSuchStructField { .. } => DiagnosticCode::UnknownField,
                    _ => DiagnosticCode::MalformedSource,
                };
                out.report(facet.diagnostic(code, format!("{error}")));
                return;
            }
        };
        let moods: BTreeSet<&str> = file.moods.iter().map(String::as_str).collect();
        for row in &file.critters {
            let id = facet.content_id(&row.name);
            out.define(
                id.clone(),
                format!(
                    "habitat={} mood={} sprite={}",
                    row.habitat, row.mood, row.sprite
                ),
            );
            out.refer(PendingRef::new(
                SchemaId::new(HABITAT),
                &row.habitat,
                "habitat",
                id.clone(),
                "habitat",
            ));
            if !moods.contains(row.mood.as_str()) {
                // A LOCAL reference: the preset lives in this same file, so an
                // unknown one is a typo and reports as UnknownPreset.
                out.refer(
                    PendingRef::new(
                        facet.schema.clone(),
                        format!("mood:{}", row.mood),
                        "mood preset",
                        id.clone(),
                        "mood",
                    )
                    .local(),
                );
            }
            out.need_asset(AssetRequirement::new(&row.sprite, id.clone(), "sprite"));
            if let Some(capability) = &row.needs_capability {
                out.require(CapabilityId::new(capability));
            }
        }
    }
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct HabitatFile {
    habitats: Vec<String>,
}

struct HabitatSchema;

impl ContentSchemaHandler for HabitatSchema {
    fn check(&self, facet: &FacetSource<'_>, out: &mut FacetOutcome) {
        match ron::from_str::<HabitatFile>(facet.text) {
            Ok(file) => {
                for habitat in &file.habitats {
                    let id = facet.content_id(habitat);
                    out.define(id, "habitat");
                }
                // The toy pack's RUNTIME family: registered `Runtime`, so the
                // compiler requires an artifact and this produces one.
                out.lower(file.habitats.clone());
            }
            Err(error) => out.report(
                facet.diagnostic(DiagnosticCode::MalformedSource, format!("{error}")),
            ),
        }
    }
}

// ── the aggregating family ──────────────────────────────────────────────────
//
//  the FRAGMENT type and the ARTIFACT type are deliberately different. A
// toy where each file lowers a `Vec<String>` and the merge is a concatenated
// `Vec<String>` cannot tell a real aggregation from the compiler shortcutting a
// single source straight through — both downcast. `BurrowFile` in, `BTreeMap`
// out, so the single-source probe below can actually fail.

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct BurrowFile {
    dweller: String,
    depth: u32,
}

/// name → depth, assembled from every burrow file in the pack.
type BurrowMap = BTreeMap<String, u32>;

struct BurrowSchema;

impl ContentSchemaHandler for BurrowSchema {
    fn check(&self, facet: &FacetSource<'_>, out: &mut FacetOutcome) {
        match ron::from_str::<BurrowFile>(facet.text) {
            Ok(file) => {
                out.define(facet.content_id(&file.dweller), format!("depth={}", file.depth));
                out.lower(file);
            }
            Err(error) => out.report(
                facet.diagnostic(DiagnosticCode::MalformedSource, format!("{error}")),
            ),
        }
    }

    fn aggregate(
        &self,
        fragments: &[LoweredFragment<'_>],
        out: &mut AggregateOutcome,
    ) -> Aggregation {
        let mut map = BurrowMap::new();
        let mut sources: BTreeMap<String, String> = BTreeMap::new();
        for fragment in fragments {
            let Some(file) = fragment.get::<BurrowFile>() else {
                continue;
            };
            if let Some(first) = sources.get(&file.dweller) {
                out.report(
                    AggregateOutcome::refusal(
                        DiagnosticCode::ConflictingModuleContribution,
                        format!(
                            "`{}` is burrowed in `{first}` and in `{}`",
                            file.dweller, fragment.declared_path
                        ),
                    )
                    .in_source(fragment.declared_path),
                );
                continue;
            }
            sources.insert(file.dweller.clone(), fragment.declared_path.to_string());
            map.insert(file.dweller.clone(), file.depth);
        }
        if !out.failed() {
            out.lower(map);
        }
        Aggregation::Defined
    }
}

/// The order fragments arrived in, as one string — for the declared-order probe.
struct OrderSchema;

impl ContentSchemaHandler for OrderSchema {
    fn check(&self, facet: &FacetSource<'_>, out: &mut FacetOutcome) {
        //  it DEFINES as well as lowering, and it did not at first — the
        // lower-must-define rule caught this toy the moment the rule existed,
        // which is the population it is for: a schema whose authored values
        // reach the game without reaching the pack's identity.
        let word = facet.text.trim().to_string();
        out.define(facet.content_id(&word), word.clone());
        out.lower(word);
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

fn registration(
    id: &str,
    disposition: RuntimeDisposition,
    handler: Arc<dyn ContentSchemaHandler>,
) -> SchemaRegistration {
    SchemaRegistration {
        id: SchemaId::new(id),
        version: SchemaVersion(1),
        capability: CapabilityId::new("critters"),
        disposition,
        doc: "test schema",
        handler,
    }
}

fn registry() -> SchemaRegistry {
    let mut registry = SchemaRegistry::new();
    // `critter` is AUTHORING-ONLY here on purpose: it is the family these tests
    // use to exercise references, assets and duplicate identities, and several
    // of those need TWO sources of one schema — which a `Runtime` schema now
    // refuses (it has not said how two artifacts combine). `habitat` is the
    // runtime family, and it lowers.
    registry
        .register(registration(
            CRITTER,
            RuntimeDisposition::AuthoringOnly,
            Arc::new(CritterSchema),
        ))
        .expect("fresh registry");
    registry
        .register(registration(
            HABITAT,
            RuntimeDisposition::Runtime,
            Arc::new(HabitatSchema),
        ))
        .expect("fresh registry");
    // The many-sources family. Installed but unauthored by every other pack
    // here, which is the ordinary state of a capability nobody used.
    registry
        .register(registration(
            BURROW,
            RuntimeDisposition::Runtime,
            Arc::new(BurrowSchema),
        ))
        .expect("fresh registry");
    registry
        .register(registration(
            ORDER,
            RuntimeDisposition::Runtime,
            Arc::new(OrderSchema),
        ))
        .expect("fresh registry");
    registry
}

// ── a pack on disk ──────────────────────────────────────────────────────────

struct Pack {
    root: PathBuf,
}

impl Pack {
    /// A well-formed two-source pack. Every negative test starts from this and
    /// changes ONE thing, so what the test proves is the change.
    fn valid(name: &str) -> Self {
        let pack = Self::empty(name);
        pack.write(
            "habitats.ron",
            "(habitats: [\"burrow\", \"canopy\"])",
        );
        pack.write(
            "critters.ron",
            r#"(
                moods: ["placid", "cross"],
                critters: [
                    (name: "mole", habitat: "burrow", mood: "placid", sprite: "mole.png"),
                    (name: "shrike", habitat: "canopy", mood: "cross", sprite: "shrike.png"),
                ],
            )"#,
        );
        pack.manifest(
            r#"(
                id: "critters",
                version: "1.0.0",
                namespace: "test",
                sources: [
                    (path: "habitats.ron", schema: "habitat", version: 1),
                    (path: "critters.ron", schema: "critter", version: 1),
                ],
            )"#,
        );
        pack
    }

    fn empty(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!("ambition_content_pack_test/{name}"));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("temp pack dir");
        Self { root }
    }

    fn write(&self, path: &str, text: &str) {
        let full = self.root.join(path);
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent).expect("parent dir");
        }
        std::fs::write(full, text).expect("write source");
    }

    fn manifest(&self, text: &str) {
        self.write("pack.ron", text);
    }

    fn assets() -> FixedAssets {
        FixedAssets::new(["mole.png", "shrike.png"])
    }

    fn compile(&self) -> Result<PreparedContentPack, CompileFailure> {
        compile_dir(&self.root, &registry(), &Self::assets())
    }

    fn compile_with(
        &self,
        registry: &SchemaRegistry,
        assets: &dyn AssetSource,
    ) -> Result<PreparedContentPack, CompileFailure> {
        compile_dir(&self.root, registry, assets)
    }

    fn refuse(&self) -> CompileFailure {
        self.compile().expect_err("this pack must be refused")
    }
}

// ── the positive path ───────────────────────────────────────────────────────

#[test]
fn a_valid_pack_compiles_to_ordered_content_with_a_stable_fingerprint() {
    let pack = Pack::valid("valid").compile().expect("valid pack compiles");

    assert_eq!(pack.content_count(), 4, "two critters and two habitats");
    // Canonical ordering is (namespace, schema, name) — NOT manifest order and
    // not filesystem order, so two machines agree.
    let ids: Vec<String> = pack.content.keys().map(ToString::to_string).collect();
    assert_eq!(
        ids,
        vec![
            "test:critter/mole",
            "test:critter/shrike",
            "test:habitat/burrow",
            "test:habitat/canopy",
        ]
    );
    assert_eq!(pack.resolved_references.len(), 2, "two habitat references");
    assert_eq!(pack.assets.len(), 2);
    assert!(pack.required_capabilities.contains(&CapabilityId::new("critters")));

    // Recompiling the same bytes gives the same fingerprint; that is the whole
    // contract, and it is checked against a SECOND read rather than against
    // itself so a cached value cannot pass it.
    let again = Pack::valid("valid_again").compile().expect("compiles");
    assert_eq!(
        pack.fingerprint, again.fingerprint,
        "identical content fingerprints identically"
    );

    // And it MOVES when a value moves. A fingerprint that never changes is a
    // constant with extra steps.
    let edited = Pack::valid("valid_edited");
    edited.write(
        "critters.ron",
        r#"(
            moods: ["placid", "cross"],
            critters: [
                (name: "mole", habitat: "canopy", mood: "placid", sprite: "mole.png"),
                (name: "shrike", habitat: "canopy", mood: "cross", sprite: "shrike.png"),
            ],
        )"#,
    );
    assert_ne!(
        pack.fingerprint,
        edited.compile().expect("still valid").fingerprint,
        "moving a critter to another habitat must move the fingerprint"
    );
}

#[test]
fn a_typed_reference_resolves_only_against_content_the_pack_defines() {
    struct Critter;
    impl ContentKind for Critter {
        const SCHEMA: &'static str = CRITTER;
        const NOUN: &'static str = "critter";
    }

    let pack = Pack::valid("typed_refs").compile().expect("compiles");
    let mole = pack.resolve::<Critter>("mole").expect("mole is defined");
    assert_eq!(mole.name(), "mole");
    assert_eq!(mole.target().to_string(), "test:critter/mole");
    assert!(
        pack.resolve::<Critter>("wyvern").is_none(),
        "a ResolvedContentRef cannot be minted for content nobody authored — holding one IS the proof"
    );
}

// ── one probe per hard error ────────────────────────────────────────────────

#[test]
fn an_unknown_schema_fails_before_assembly_and_offers_the_installed_ones() {
    let pack = Pack::valid("unknown_schema");
    pack.manifest(
        r#"(
            id: "critters", version: "1.0.0", namespace: "test",
            sources: [
                (path: "habitats.ron", schema: "habitat", version: 1),
                (path: "critters.ron", schema: "critterr", version: 1),
            ],
        )"#,
    );
    let failure = pack.refuse();
    assert_eq!(failure.stage, CompileStage::SchemaResolution);
    assert!(failure.has(DiagnosticCode::UnknownSchema));
    let rendered = failure.render();
    assert!(
        rendered.contains("did you mean `critter`?"),
        "a typo is answered, not only rejected:\n{rendered}"
    );
    assert!(
        failure
            .stopped_before()
            .contains(&CompileStage::ReferenceResolution)
    );
}

#[test]
fn a_schema_version_the_composition_does_not_install_is_refused() {
    let pack = Pack::valid("schema_version");
    pack.manifest(
        r#"(
            id: "critters", version: "1.0.0", namespace: "test",
            sources: [
                (path: "habitats.ron", schema: "habitat", version: 1),
                (path: "critters.ron", schema: "critter", version: 2),
            ],
        )"#,
    );
    let failure = pack.refuse();
    assert_eq!(failure.stage, CompileStage::SchemaResolution);
    assert!(failure.has(DiagnosticCode::SchemaVersionMismatch));
}

#[test]
fn an_uninstalled_required_capability_fails_before_any_facet_is_read() {
    let pack = Pack::valid("missing_capability");
    pack.manifest(
        r#"(
            id: "critters", version: "1.0.0", namespace: "test",
            requires: ["weather"],
            sources: [
                (path: "habitats.ron", schema: "habitat", version: 1),
                (path: "critters.ron", schema: "critter", version: 1),
            ],
        )"#,
    );
    let failure = pack.refuse();
    assert_eq!(failure.stage, CompileStage::CapabilityValidation);
    assert!(failure.has(DiagnosticCode::MissingCapability));
    assert!(
        failure.render().contains("installed capabilities: critters"),
        "the refusal lists what IS installed — a refusal that only says no costs a rebuild to \
         answer:\n{}",
        failure.render()
    );
}

#[test]
fn a_capability_only_the_authored_content_needs_is_still_refused() {
    // The manifest is honest and the CONTENT is not: a row asks for a capability nobody
    // declared.
    let pack = Pack::valid("facet_capability");
    pack.write(
        "critters.ron",
        r#"(
            moods: ["placid"],
            critters: [
                (name: "mole", habitat: "burrow", mood: "placid", sprite: "mole.png",
                 needs_capability: Some("weather")),
            ],
        )"#,
    );
    let failure = pack.refuse();
    assert_eq!(failure.stage, CompileStage::CapabilityValidation);
    assert!(failure.has(DiagnosticCode::MissingCapability));
    assert!(
        failure.render().contains("declare it in pack.ron's `requires`"),
        "and it says how to make the check happen EARLIER next time"
    );
}

#[test]
fn an_unknown_preset_is_refused_and_named_as_a_preset() {
    let pack = Pack::valid("unknown_preset");
    pack.write(
        "critters.ron",
        r#"(
            moods: ["placid"],
            critters: [
                (name: "mole", habitat: "burrow", mood: "plcid", sprite: "mole.png"),
            ],
        )"#,
    );
    let failure = pack.refuse();
    assert_eq!(failure.stage, CompileStage::ReferenceResolution);
    assert!(
        failure.has(DiagnosticCode::UnknownPreset),
        "a preset lives in the same file, so it reports as a typo rather than as a missing \
         dependency: {:?}",
        failure.codes()
    );
}

#[test]
fn an_unresolved_content_reference_names_the_declarer_and_the_field() {
    let pack = Pack::valid("unresolved_ref");
    pack.write(
        "critters.ron",
        r#"(
            moods: ["placid"],
            critters: [
                (name: "mole", habitat: "abyss", mood: "placid", sprite: "mole.png"),
            ],
        )"#,
    );
    let failure = pack.refuse();
    assert_eq!(failure.stage, CompileStage::ReferenceResolution);
    assert!(failure.has(DiagnosticCode::UnresolvedReference));
    let diagnostic = failure
        .diagnostics
        .iter()
        .find(|d| d.code == DiagnosticCode::UnresolvedReference)
        .expect("the reference failure");
    assert_eq!(
        diagnostic.subject.as_ref().map(ToString::to_string),
        Some("test:critter/mole".to_string()),
        "the finding is ABOUT the content that declared it, not about the file"
    );
    assert_eq!(diagnostic.field.as_deref(), Some("habitat"));
}

#[test]
fn a_missing_asset_is_refused_with_the_root_it_looked_under() {
    let pack = Pack::valid("missing_asset");
    let failure = pack
        .compile_with(&registry(), &FixedAssets::new(["mole.png"]))
        .expect_err("shrike.png is absent");
    assert_eq!(failure.stage, CompileStage::ReferenceResolution);
    assert!(failure.has(DiagnosticCode::MissingAsset));
    assert!(failure.render().contains("shrike.png"));
}

#[test]
fn a_duplicate_canonical_identity_names_both_sources() {
    let pack = Pack::valid("duplicate_identity");
    pack.write(
        "more_critters.ron",
        r#"(
            moods: ["placid"],
            critters: [
                (name: "mole", habitat: "burrow", mood: "placid", sprite: "mole.png"),
            ],
        )"#,
    );
    pack.manifest(
        r#"(
            id: "critters", version: "1.0.0", namespace: "test",
            sources: [
                (path: "habitats.ron", schema: "habitat", version: 1),
                (path: "critters.ron", schema: "critter", version: 1),
                (path: "more_critters.ron", schema: "critter", version: 1),
            ],
        )"#,
    );
    let failure = pack.refuse();
    assert_eq!(failure.stage, CompileStage::ConflictDetection);
    assert!(failure.has(DiagnosticCode::DuplicateIdentity));
    let rendered = failure.render();
    assert!(
        rendered.contains("critters.ron") && rendered.contains("more_critters.ron"),
        "both sides are named — a duplicate reported from one side is half an answer:\n{rendered}"
    );
}

#[test]
fn an_unknown_authored_field_is_an_error_and_not_a_shrug() {
    // The most expensive content bug there is: everything looks authored and
    // nothing happens.
    let pack = Pack::valid("unknown_field");
    pack.write(
        "critters.ron",
        r#"(
            moods: ["placid"],
            critters: [
                (name: "mole", habitat: "burrow", mood: "placid", sprite: "mole.png",
                 favourite_snack: "worms"),
            ],
        )"#,
    );
    let failure = pack.refuse();
    assert_eq!(failure.stage, CompileStage::FacetValidation);
    assert!(
        failure.has(DiagnosticCode::UnknownField),
        "got {:?}",
        failure.codes()
    );
}

#[test]
fn two_capabilities_claiming_one_schema_is_refused_at_registration() {
    let mut registry = registry();
    let mut clashing = registration(
        CRITTER,
        RuntimeDisposition::AuthoringOnly,
        Arc::new(CritterSchema),
    );
    clashing.capability = CapabilityId::new("beasts");
    let diagnostic = registry
        .register(clashing)
        .expect_err("one schema, one owner");
    assert_eq!(diagnostic.code, DiagnosticCode::AmbiguousSchemaOwnership);
    assert!(
        diagnostic.message.contains("critters") && diagnostic.message.contains("beasts"),
        "both claimants are named"
    );
}

#[test]
fn the_same_file_reached_twice_is_collapsed_deterministically() {
    let pack = Pack::valid("aliased_source");
    // `./critters.ron` and `critters.ron` are the same file spelled two ways —
    // the cheap, portable stand-in for the symlink case, and it exercises the
    // same canonicalisation.
    pack.manifest(
        r#"(
            id: "critters", version: "1.0.0", namespace: "test",
            sources: [
                (path: "habitats.ron", schema: "habitat", version: 1),
                (path: "critters.ron", schema: "critter", version: 1),
                (path: "./critters.ron", schema: "critter", version: 1),
            ],
        )"#,
    );
    let compiled = pack.compile().expect("an alias is not an error by itself");
    assert_eq!(
        compiled.sources.len(),
        2,
        "the alias collapsed rather than defining every critter twice"
    );
    assert_eq!(
        compiled.collapsed_aliases,
        vec![("./critters.ron".to_string(), "critters.ron".to_string())],
        "and it is RECORDED — a pack that silently reports fewer sources than its manifest \
         declares is indistinguishable from one that dropped a file"
    );
    assert_eq!(compiled.content_count(), 4);
}

#[test]
fn the_same_file_declared_under_two_schemas_is_a_clear_error() {
    let pack = Pack::valid("aliased_conflict");
    pack.manifest(
        r#"(
            id: "critters", version: "1.0.0", namespace: "test",
            sources: [
                (path: "habitats.ron", schema: "habitat", version: 1),
                (path: "critters.ron", schema: "critter", version: 1),
                (path: "./critters.ron", schema: "habitat", version: 1),
            ],
        )"#,
    );
    let failure = pack.refuse();
    assert_eq!(failure.stage, CompileStage::Parse);
    assert!(
        failure.has(DiagnosticCode::ConflictingSourceAlias),
        "deduplicating here would mean guessing what the file MEANS: {:?}",
        failure.codes()
    );
}

#[test]
fn a_declared_source_that_is_not_there_is_refused_at_parse() {
    let pack = Pack::valid("missing_source");
    pack.manifest(
        r#"(
            id: "critters", version: "1.0.0", namespace: "test",
            sources: [
                (path: "habitats.ron", schema: "habitat", version: 1),
                (path: "ghosts.ron", schema: "critter", version: 1),
            ],
        )"#,
    );
    let failure = pack.refuse();
    assert_eq!(failure.stage, CompileStage::Parse);
    assert!(failure.has(DiagnosticCode::MalformedSource));
}

#[test]
fn a_schema_with_no_instances_in_this_pack_is_fine() {
    // A capability offering something nobody authored is the ordinary state of
    // a library. Only an authored facet with no complete handler is an error.
    let pack = Pack::empty("no_instances");
    pack.write("habitats.ron", "(habitats: [\"burrow\"])");
    pack.manifest(
        r#"(
            id: "critters", version: "1.0.0", namespace: "test",
            sources: [(path: "habitats.ron", schema: "habitat", version: 1)],
        )"#,
    );
    let compiled = pack.compile().expect("a pack may use one of two schemas");
    assert_eq!(compiled.content_count(), 1);
    assert!(
        !compiled.schemas.contains_key(&SchemaId::new(CRITTER)),
        "an unused installed schema is not recorded as a dependency of this pack"
    );
}

#[test]
fn an_unchecked_asset_source_says_so_instead_of_looking_verified() {
    let pack = Pack::valid("unchecked_assets");
    let compiled = pack
        .compile_with(&registry(), &AssetsUnchecked)
        .expect("asset checks declined");
    assert!(
        compiled
            .assets
            .values()
            .all(|provenance| provenance.root == "<unchecked>"),
        "provenance records that nothing verified these — a pack prepared this way is visibly \
         not making a claim about its assets"
    );
}

///  A `Runtime` schema that lowers nothing is refused.
///
/// Otherwise a pack compiles while carrying authored runtime content with no
/// runtime representation — "validated and then ignored", which is the one thing
/// a content compiler must never certify. `AuthoringOnly` is how a schema says
/// it deliberately reaches no runtime, and saying it explicitly is the point.
#[test]
fn a_runtime_schema_that_lowers_nothing_is_refused() {
    struct Inert;
    impl ContentSchemaHandler for Inert {
        fn check(&self, facet: &FacetSource<'_>, out: &mut FacetOutcome) {
            // Validates fine, defines content, and never lowers.
            out.define(facet.content_id("something"), "inert");
        }
    }

    let pack = Pack::empty("runtime_without_lowering");
    pack.write("inert.ron", "()");
    pack.manifest(
        r#"(
            id: "critters", version: "1.0.0", namespace: "test",
            sources: [(path: "inert.ron", schema: "inert", version: 1)],
        )"#,
    );

    let mut runtime = SchemaRegistry::new();
    runtime
        .register(registration(
            "inert",
            RuntimeDisposition::Runtime,
            Arc::new(Inert),
        ))
        .expect("fresh registry");
    let failure = pack
        .compile_with(&runtime, &AssetsUnchecked)
        .expect_err("a Runtime schema owes a runtime artifact");
    assert_eq!(failure.stage, CompileStage::FacetValidation);
    assert!(failure.render().contains("validate and then be ignored"));

    // The SAME handler registered `AuthoringOnly` compiles, because reaching no
    // runtime is then what it means rather than what it forgot.
    let mut authoring = SchemaRegistry::new();
    authoring
        .register(registration(
            "inert",
            RuntimeDisposition::AuthoringOnly,
            Arc::new(Inert),
        ))
        .expect("fresh registry");
    assert!(
        pack.compile_with(&authoring, &AssetsUnchecked).is_ok(),
        "the disposition is the difference, not the handler"
    );
}

///  Two sources lowering one schema is REFUSED, never last-wins — unless
/// the schema has SAID how they combine.
///
/// Silently overwriting means the content INDEX knows about both sources while
/// the runtime artifact holds only the last — validation and the running game
/// seeing different content. Only the HANDLER knows whether two of its artifacts
/// union, override or conflict, so a generic merge here would be the compiler
/// guessing.
///
/// `habitat` never said, so it is still refused; the refusal now names BOTH
/// files, because "which two" is the first thing a reader asks.
#[test]
fn two_sources_lowering_one_schema_is_refused_rather_than_silently_last_wins() {
    let pack = Pack::empty("two_runtime_sources");
    pack.write("a.ron", "(habitats: [\"burrow\"])");
    pack.write("b.ron", "(habitats: [\"canopy\"])");
    pack.manifest(
        r#"(
            id: "critters", version: "1.0.0", namespace: "test",
            sources: [
                (path: "a.ron", schema: "habitat", version: 1),
                (path: "b.ron", schema: "habitat", version: 1),
            ],
        )"#,
    );
    let failure = pack.refuse();
    assert_eq!(failure.stage, CompileStage::Aggregation);
    assert!(
        failure.has(DiagnosticCode::ConflictingModuleContribution),
        "got {:?}",
        failure.codes()
    );
    let rendered = failure.render();
    assert!(
        rendered.contains("how two of its artifacts combine"),
        "and it names the missing contract rather than just refusing:\n{rendered}"
    );
    assert!(
        rendered.contains("a.ron") && rendered.contains("b.ron"),
        "both files, so the author knows which two collided:\n{rendered}"
    );
}

/// Aggregating schemas merge all declared sources into one runtime artifact.
#[test]
fn an_aggregating_schema_merges_every_source_into_one_artifact() {
    let pack = Pack::empty("aggregate_many");
    pack.write("mole.ron", r#"(dweller: "mole", depth: 3)"#);
    pack.write("shrike.ron", r#"(dweller: "shrike", depth: 1)"#);
    pack.manifest(
        r#"(
            id: "critters", version: "1.0.0", namespace: "test",
            sources: [
                (path: "mole.ron", schema: "burrow", version: 1),
                (path: "shrike.ron", schema: "burrow", version: 1),
            ],
        )"#,
    );
    let prepared = pack.compile().expect("two burrows merge");
    let burrows = prepared
        .lowered::<BurrowMap>(&SchemaId::new(BURROW))
        .expect("the merged artifact is what a Runtime schema lowered");
    assert_eq!(burrows.get("mole"), Some(&3));
    assert_eq!(burrows.get("shrike"), Some(&1));
    assert_eq!(burrows.len(), 2, "both, not the last one");
}

/// Aggregation runs even for one source so the runtime artifact type cannot
/// depend on how many files the author declared. The fixture deliberately uses
/// different fragment and aggregate types.
#[test]
fn an_aggregating_schema_is_asked_to_merge_even_a_single_source() {
    let pack = Pack::empty("aggregate_one");
    pack.write("mole.ron", r#"(dweller: "mole", depth: 3)"#);
    pack.manifest(
        r#"(
            id: "critters", version: "1.0.0", namespace: "test",
            sources: [(path: "mole.ron", schema: "burrow", version: 1)],
        )"#,
    );
    let prepared = pack.compile().expect("one burrow is a pack too");
    assert!(
        prepared
            .lowered::<BurrowMap>(&SchemaId::new(BURROW))
            .is_some_and(|burrows| burrows.get("mole") == Some(&3)),
        "one source must lower the same TYPE nine sources do"
    );
}

/// A merge is also the first place a CROSS-SOURCE invariant can be checked — a
/// per-facet handler sees one file and cannot know another claimed the name.
#[test]
fn an_aggregation_refuses_what_no_single_facet_could_have_seen() {
    let pack = Pack::empty("aggregate_conflict");
    pack.write("deep.ron", r#"(dweller: "mole", depth: 9)"#);
    pack.write("shallow.ron", r#"(dweller: "mole", depth: 1)"#);
    pack.manifest(
        r#"(
            id: "critters", version: "1.0.0", namespace: "test",
            sources: [
                (path: "deep.ron", schema: "burrow", version: 1),
                (path: "shallow.ron", schema: "burrow", version: 1),
            ],
        )"#,
    );
    let failure = pack.refuse();
    assert_eq!(failure.stage, CompileStage::Aggregation);
    assert!(
        failure.render().contains("burrowed in `deep.ron`"),
        "the handler's own merge rule speaks, naming both files:\n{}",
        failure.render()
    );
    assert!(
        failure
            .stopped_before()
            .contains(&CompileStage::ReferenceResolution),
        "and a refusal here says what never ran"
    );
}

///  fragments arrive in DECLARED order, not map order and not filesystem
/// order. A merge with override semantics needs a defined order, and the one an
/// author can see and diff is the manifest, top to bottom. The paths here sort
/// the other way on purpose, so an accidental `BTreeMap` of sources would fail.
#[test]
fn fragments_reach_the_merge_in_the_order_the_manifest_declares() {
    let pack = Pack::empty("aggregate_order");
    pack.write("z_first.ron", "one");
    pack.write("a_second.ron", "two");
    pack.manifest(
        r#"(
            id: "critters", version: "1.0.0", namespace: "test",
            sources: [
                (path: "z_first.ron", schema: "order", version: 1),
                (path: "a_second.ron", schema: "order", version: 1),
            ],
        )"#,
    );
    let prepared = pack.compile().expect("both lower");
    assert_eq!(
        prepared
            .lowered::<String>(&SchemaId::new(ORDER))
            .map(String::as_str),
        Some("one,two"),
        "declared order, not the alphabetical order of the paths"
    );
}

///  The same logical pack fingerprints identically from two directories.
///
/// The fingerprint is content identity: a cache key, a packaging input, a
/// session-compatibility check, the thing that says whether two builds carry the
/// same content. A value that moves with the checkout path answers none of
/// those. Asset PROVENANCE (where a file resolved) stays available for
/// diagnostics and stays out of identity.
#[test]
fn the_fingerprint_does_not_move_with_the_checkout_path() {
    fn pack_with_art(name: &str) -> Pack {
        let pack = Pack::valid(name);
        // Real files under THIS pack's own root, so each run resolves its assets
        // somewhere different and provenance actually records two roots.
        pack.write("mole.png", "");
        pack.write("shrike.png", "");
        pack
    }

    let here = pack_with_art("fingerprint_here");
    let elsewhere = pack_with_art("some/deeper/fingerprint_elsewhere");

    let compile_at = |pack: &Pack| {
        compile_dir(
            &pack.root,
            &registry(),
            &DirectoryAssets::new([pack.root.clone()]),
        )
        .expect("art is present under this pack's own root")
    };
    let first = compile_at(&here);
    let second = compile_at(&elsewhere);

    let root_of = |pack: &PreparedContentPack| {
        pack.assets.values().next().map(|p| p.root.clone())
    };
    assert_ne!(
        root_of(&first),
        root_of(&second),
        "the two runs really did resolve against different roots — without this the test \
         would pass for the wrong reason"
    );
    assert_eq!(
        first.fingerprint, second.fingerprint,
        "identical content, different checkout: ONE identity. A fingerprint that moved with \
         the path would answer none of the questions it exists for — caching, packaging, \
         session compatibility, comparing two builds."
    );
}
