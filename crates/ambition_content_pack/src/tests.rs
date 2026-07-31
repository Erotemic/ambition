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
                for habitat in file.habitats {
                    let id = facet.content_id(&habitat);
                    out.define(id, "habitat");
                }
            }
            Err(error) => out.report(
                facet.diagnostic(DiagnosticCode::MalformedSource, format!("{error}")),
            ),
        }
    }
}

fn registration(id: &str, handler: Arc<dyn ContentSchemaHandler>) -> SchemaRegistration {
    SchemaRegistration {
        id: SchemaId::new(id),
        version: SchemaVersion(1),
        capability: CapabilityId::new("critters"),
        disposition: RuntimeDisposition::Runtime,
        doc: "test schema",
        handler,
    }
}

fn registry() -> SchemaRegistry {
    let mut registry = SchemaRegistry::new();
    registry
        .register(registration(CRITTER, Arc::new(CritterSchema)))
        .expect("fresh registry");
    registry
        .register(registration(HABITAT, Arc::new(HabitatSchema)))
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
    // Everything after schema resolution genuinely could not run, and the
    // failure says so rather than letting a partial list look complete.
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
    // The manifest is honest and the CONTENT is not: a row asks for a
    // capability nobody declared. Only the handler can see this, so it is
    // caught after facet validation — and still before anything is assembled.
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
    let mut clashing = registration(CRITTER, Arc::new(CritterSchema));
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
