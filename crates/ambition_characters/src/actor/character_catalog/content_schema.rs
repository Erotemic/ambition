//! The character capability's authored-content SCHEMA registration.
//!
//! This is what "a capability contributes authored schemas" means concretely:
//! `ambition_characters` owns the character catalog, so it — not a central
//! content enum, not the compiler — says how a `character_catalog.ron` is read,
//! what identities it mints, which references it needs resolved, and which
//! assets it depends on.
//!
//! ## One file, five identity kinds
//!
//! A catalog file's SCHEMA is `character_catalog`; the identities it defines are
//! `character`, `brain_preset`, `action_set_preset`, `axis_tuning_preset` and
//! `locomotion_preset`.
//! Those are different
//! questions and the distinction earns its keep immediately: a character naming
//! a missing `default_brain` is an [`DiagnosticCode::UnknownPreset`] pointing at
//! a preset kind, so the refusal can list the presets that DO exist rather than
//! the characters.
//!
//! ## Presets stay load-bearing
//!
//! `peaceful`, `striker_swipe` and the rest are shared across many rows and are
//! the reason the catalog is not one blob per character. Nothing here flattens
//! them: a preset is content with its own identity, and a character holds a
//! reference to one.

use std::sync::Arc;

use ambition_content_pack::{
    AssetRequirement, CapabilityId, ContentId, ContentKind, ContentSchemaHandler, DiagnosticCode,
    FacetOutcome, FacetSource, PendingRef, RuntimeDisposition, SchemaId, SchemaRegistration,
    SchemaVersion,
};

use super::entry::CharacterCatalogData;
use super::loader::try_parse_catalog;
use super::validator::{findings, Finding, FindingKind, PresetTable};

/// The capability that owns every schema in this module.
pub const CHARACTERS_CAPABILITY: &str = "characters";

/// The authored FILE kind.
pub const CHARACTER_CATALOG_SCHEMA: &str = "character_catalog";
/// One playable/spawnable character.
pub const CHARACTER_SCHEMA: &str = "character";
/// A named brain preset a character's `default_brain` points at.
pub const BRAIN_PRESET_SCHEMA: &str = "brain_preset";
/// A named action-set preset a character's `default_action_set` points at.
pub const ACTION_SET_PRESET_SCHEMA: &str = "action_set_preset";
/// A named movement feel a character's `axis_tuning_preset` points at.
pub const AXIS_TUNING_PRESET_SCHEMA: &str = "axis_tuning_preset";
/// A named gait a character's `locomotion_preset` points at.
pub const LOCOMOTION_PRESET_SCHEMA: &str = "locomotion_preset";

/// The schema version this handler reads. Bump it when the authored shape
/// changes meaning, never when a field is merely added with a default.
pub const CHARACTER_CATALOG_VERSION: SchemaVersion = SchemaVersion(1);

/// Typed reference markers, for shipped Rust consumers.
///
/// Both were silent.
pub struct Character;
impl ContentKind for Character {
    const SCHEMA: &'static str = CHARACTER_SCHEMA;
    const NOUN: &'static str = "character";
}

/// Zero-sized schema marker used by content-pack validation to resolve brain
/// preset references. `Facet` distinguishes this checker from authored reference
/// value types.
pub struct BrainPresetRefFacet;
impl ContentKind for BrainPresetRefFacet {
    const SCHEMA: &'static str = BRAIN_PRESET_SCHEMA;
    const NOUN: &'static str = "brain preset";
}

pub struct ActionSetPresetRef;
impl ContentKind for ActionSetPresetRef {
    const SCHEMA: &'static str = ACTION_SET_PRESET_SCHEMA;
    const NOUN: &'static str = "action-set preset";
}

struct CharacterCatalogSchema;

impl ContentSchemaHandler for CharacterCatalogSchema {
    fn check(&self, facet: &FacetSource<'_>, out: &mut FacetOutcome) {
        let catalog = match try_parse_catalog(facet.text) {
            Ok(catalog) => catalog,
            Err(error) => {
                // `deny_unknown_fields` on the catalog types is what makes an
                // unconsumed authored field reach here at all. Without it a
                // typo is silently dropped and the mechanic simply never fires
                // — everything looks authored and nothing happens.
                let code = if error.contains("Unexpected field") {
                    DiagnosticCode::UnknownField
                } else {
                    DiagnosticCode::MalformedSource
                };
                out.report(facet.diagnostic(code, error));
                return;
            }
        };
        declare(facet, &catalog, out);
        // LOWER: the runtime reads this instead of parsing the same bytes a
        // second time. Only published when the facet is clean — a caller must
        // never receive a runtime value out of a pack that was refused.
        if !out.failed() {
            out.lower(catalog);
        }
    }
}

fn preset_id(facet: &FacetSource<'_>, schema: &str, name: &str) -> ContentId {
    ContentId::new(facet.namespace, &SchemaId::new(schema), name)
}

fn declare(facet: &FacetSource<'_>, catalog: &CharacterCatalogData, out: &mut FacetOutcome) {
    for (name, preset) in &catalog.brain_presets {
        out.define(
            preset_id(facet, BRAIN_PRESET_SCHEMA, name),
            canonical(preset),
        );
    }
    for (name, preset) in &catalog.action_set_presets {
        out.define(
            preset_id(facet, ACTION_SET_PRESET_SCHEMA, name),
            canonical(preset),
        );
    }
    for (name, preset) in &catalog.axis_tuning_presets {
        out.define(
            preset_id(facet, AXIS_TUNING_PRESET_SCHEMA, name),
            canonical(preset),
        );
    }
    for (name, preset) in &catalog.locomotion_presets {
        out.define(
            preset_id(facet, LOCOMOTION_PRESET_SCHEMA, name),
            canonical(preset),
        );
    }

    for (name, entry) in &catalog.characters {
        let id = preset_id(facet, CHARACTER_SCHEMA, name);
        out.define(id.clone(), canonical(entry));

        // An empty path is a finding below, and is not asked of the asset
        // resolver: "" resolves to the asset root itself on most filesystems,
        // so a missing-asset check would pass on the one value that is
        // certainly wrong.
        let portrait = entry.portrait.as_ref();
        for (field, path) in [
            ("spritesheet", Some(&entry.spritesheet)),
            ("manifest", Some(&entry.manifest)),
            ("portrait.image", portrait.map(|portrait| &portrait.image)),
            ("portrait.manifest", portrait.map(|portrait| &portrait.manifest)),
        ] {
            if let Some(path) = path.filter(|path| !path.trim().is_empty()) {
                out.need_asset(AssetRequirement::new(path, id.clone(), field));
            }
        }
    }

    // The row rules are the validator's; this shows each one as a pack
    // diagnostic, and a preset reference as a pack reference.
    for Finding {
        character,
        field,
        kind,
    } in findings(catalog)
    {
        let id = preset_id(facet, CHARACTER_SCHEMA, character);
        let diagnostic = match kind {
            FindingKind::Reference { table, name } => {
                match preset_schema(table) {
                    // LOCAL: the preset is authored in this same catalog, so an
                    // unknown one is a typo, not a missing dependency, and the
                    // fix line says "define it here", not "install another pack".
                    Some((schema, noun)) => {
                        out.refer(
                            PendingRef::new(SchemaId::new(schema), name, noun, id, field).local(),
                        );
                        continue;
                    }
                    // A shared policy is not a content identity of its own, so
                    // the name is checked against this catalog's table here.
                    None if table.contains(catalog, name) => continue,
                    None => facet
                        .diagnostic(
                            DiagnosticCode::UnknownPreset,
                            format!(
                                "character `{character}` names `{name}` in `{field}`, which \
                                 `{}` does not state",
                                table.field()
                            ),
                        )
                        .fix(format!(
                            "state it in `{}`, or name one that is there",
                            table.field()
                        )),
                }
            }
            FindingKind::BothStated { inline_field, what } => facet
                .diagnostic(
                    DiagnosticCode::MalformedSource,
                    format!("character `{character}` states both `{inline_field}` and `{field}`"),
                )
                .fix(format!(
                    "state one {what}: the named one, or the row's own `{inline_field}`"
                )),
            FindingKind::Empty => {
                let diagnostic = facet.diagnostic(
                    DiagnosticCode::MalformedProviderBinding,
                    format!("character `{character}` has an empty `{field}`"),
                );
                match field {
                    "spritesheet" | "manifest" => {
                        diagnostic.fix("point it at a sheet, or remove the character")
                    }
                    "display_name" => diagnostic,
                    _ => diagnostic
                        .fix("fill all three portrait fields, or drop `portrait` entirely"),
                }
            }
            // One display name owned by two ids is an authority conflict: every
            // surface that shows a name has to pick, and they will not all pick
            // the same one.
            FindingKind::SharedDisplayName { first, display } => facet
                .diagnostic(
                    DiagnosticCode::ConflictingModuleContribution,
                    format!("characters `{first}` and `{character}` share display_name `{display}`"),
                )
                .fix(
                    "give one of them its own name — a display name is how a player \
                     identifies a character, so two owners is a conflict, not a duplicate",
                ),
        };
        out.report(diagnostic.about(id).at_field(field));
    }
}

/// The content identity kind a preset table mints, with its noun, or `None`
/// for a table whose entries are not content identities of their own.
fn preset_schema(table: PresetTable) -> Option<(&'static str, &'static str)> {
    match table {
        PresetTable::Brain => Some((BRAIN_PRESET_SCHEMA, "brain preset")),
        PresetTable::ActionSet => Some((ACTION_SET_PRESET_SCHEMA, "action-set preset")),
        PresetTable::AxisTuning => Some((AXIS_TUNING_PRESET_SCHEMA, "axis-tuning preset")),
        PresetTable::Locomotion => Some((LOCOMOTION_PRESET_SCHEMA, "locomotion preset")),
        PresetTable::AutonomousProfile => None,
    }
}

/// The canonical form an entry contributes to the pack fingerprint.
///
/// Round-tripped through RON rather than hashing the authored bytes: reflowing
/// a comment or reordering two fields must NOT move the fingerprint, and
/// changing a value must. Only the type knows which differences are semantic.
fn canonical<T: serde::Serialize>(value: &T) -> String {
    ron::ser::to_string(value).unwrap_or_else(|error| format!("<uncanonicalizable: {error}>"))
}

/// The catalog a prepared pack lowered to, if it carries one.
///
/// This is the runtime's load path. `ambition_content` composes its cast from
/// here rather than from its own `parse_catalog`, so the bytes the compiler
/// validated and the bytes the game runs are the same read.
pub fn lowered_catalog(
    pack: &ambition_content_pack::PreparedContentPack,
) -> Option<&CharacterCatalogData> {
    pack.lowered::<CharacterCatalogData>(&SchemaId::new(CHARACTER_CATALOG_SCHEMA))
}

/// The character capability's registration, for a composition to install.
pub fn character_catalog_schema() -> SchemaRegistration {
    SchemaRegistration {
        id: SchemaId::new(CHARACTER_CATALOG_SCHEMA),
        version: CHARACTER_CATALOG_VERSION,
        capability: CapabilityId::new(CHARACTERS_CAPABILITY),
        disposition: RuntimeDisposition::Runtime,
        doc: "A character roster with its shared brain and action-set presets. Defines \
              `character`, `brain_preset` and `action_set_preset` identities.",
        handler: Arc::new(CharacterCatalogSchema),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_content_pack::{
        compile, AssetsUnchecked, CompileStage, ContentPackDraft, ContentPackManifest, FixedAssets,
        ModuleNamespace, PackId, PackVersion, SchemaRegistry, SourceDeclaration,
    };

    const SMALL_CATALOG: &str = r#"(
        brain_presets: { "peaceful": StandStill },
        action_set_presets: { "striker_swipe": (move_style: Walk) },
        characters: {
            "mole": (
                display_name: "Mole",
                spritesheet: "mole.png",
                manifest: "mole.ron",
                tier: MainHall,
                body_kind: Standard,
                composition: None,
                default_brain: "peaceful",
                default_action_set: "striker_swipe",
                tags: [],
            ),
        },
    )"#;

    fn registry() -> SchemaRegistry {
        let mut registry = SchemaRegistry::new();
        registry
            .register(character_catalog_schema())
            .expect("fresh registry");
        registry
    }

    fn draft(name: &str, catalog: &str) -> ContentPackDraft {
        let root = std::env::temp_dir().join(format!("ambition_character_schema_test/{name}"));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("temp dir");
        std::fs::write(root.join("catalog.ron"), catalog).expect("write catalog");
        ContentPackDraft::read_manifest(
            root,
            ContentPackManifest {
                id: PackId("test_cast".into()),
                version: PackVersion("1.0.0".into()),
                namespace: ModuleNamespace("test".into()),
                requires: Vec::new(),
                sources: vec![SourceDeclaration {
                    path: "catalog.ron".into(),
                    schema: SchemaId::new(CHARACTER_CATALOG_SCHEMA),
                    version: CHARACTER_CATALOG_VERSION,
                }],
            },
        )
        .expect("draft reads")
    }

    /// Each row rule, broken once, is refused by both readers, at the field the
    /// rule names. The rules are one list (`validator::findings`); this holds
    /// each reader to showing every kind of finding in it.
    #[test]
    fn every_row_rule_is_refused_by_both_readers_at_its_field() {
        const ROWS: &str = r#"(
            autonomous_profiles: { "striker": (template: Smash) },
            axis_tuning_presets: { "classic": () },
            locomotion_presets: { "hall_walk": (run_speed: 210.0, move_style: Walk) },
            brain_presets: { "peaceful": StandStill },
            action_set_presets: { "striker_swipe": (move_style: Walk) },
            characters: {
                "mole": (
                    display_name: "Mole", spritesheet: "mole.png", manifest: "mole.ron",
                    tier: MainHall, body_kind: Standard, composition: None,
                    default_brain: "peaceful", default_action_set: "striker_swipe", tags: [],
                    MORE
                ),
                "vole": (
                    display_name: "Vole", spritesheet: "vole.png", manifest: "vole.ron",
                    tier: MainHall, body_kind: Standard, composition: None,
                    default_brain: "peaceful", default_action_set: "striker_swipe", tags: [],
                ),
            },
        )"#;
        // (case, the mole row's text replaced, its replacement, the field refused)
        let cases: &[(&str, &str, &str, &str)] = &[
            ("empty_name", r#""Mole""#, r#""""#, "display_name"),
            ("shared_name", r#""Mole""#, r#""Vole""#, "display_name"),
            ("empty_sheet", r#""mole.png""#, r#""""#, "spritesheet"),
            ("empty_manifest", r#""mole.ron""#, r#""""#, "manifest"),
            (
                "partial_portrait",
                "MORE",
                r#"portrait: Some((image: "", manifest: "p.ron", default_clip: "idle")),"#,
                "portrait.image",
            ),
            (
                "unknown_brain",
                r#"default_brain: "peaceful""#,
                r#"default_brain: "peacful""#,
                "default_brain",
            ),
            (
                "unknown_action_set",
                r#"default_action_set: "striker_swipe""#,
                r#"default_action_set: "striker_swip""#,
                "default_action_set",
            ),
            (
                "unknown_feel",
                "MORE",
                r#"axis_tuning_preset: Some("modern"),"#,
                "axis_tuning_preset",
            ),
            (
                "two_feels",
                "MORE",
                r#"axis_tuning_preset: Some("classic"), axis_tuning: Some(()),"#,
                "axis_tuning_preset",
            ),
            (
                "unknown_gait",
                "MORE",
                r#"locomotion_preset: Some("run"),"#,
                "locomotion_preset",
            ),
            (
                "two_gaits",
                "MORE",
                r#"locomotion_preset: Some("hall_walk"), locomotion: Some((run_speed: 1.0, move_style: Walk)),"#,
                "locomotion_preset",
            ),
            (
                "unknown_policy",
                "MORE",
                r#"named_autonomous_profile: Some("brute"),"#,
                "named_autonomous_profile",
            ),
            (
                "two_policies",
                "MORE",
                r#"named_autonomous_profile: Some("striker"), autonomous_profile: Some((template: Smash)),"#,
                "named_autonomous_profile",
            ),
            (
                "unknown_provoked",
                "MORE",
                r#"provoked_profile: Some("brute"),"#,
                "provoked_profile",
            ),
        ];

        // The control: the rows as written break no rule in either reader.
        let clean = ROWS.replace("MORE", "");
        assert_eq!(
            super::super::validator::validate(&try_parse_catalog(&clean).expect("parses")),
            Vec::<String>::new()
        );
        compile(&draft("rule_control", &clean), &registry(), &AssetsUnchecked)
            .expect("the control compiles");

        for (case, from, to, field) in cases {
            let text = ROWS.replacen(from, to, 1).replace("MORE", "");
            assert_ne!(text, clean, "case `{case}` changed nothing");
            let lines = super::super::validator::validate(&try_parse_catalog(&text).expect("parses"));
            assert!(
                lines.iter().any(|line| line.contains(field)),
                "the RON reader did not refuse `{case}` at `{field}`: {lines:?}"
            );
            let failure = compile(&draft(&format!("rule_{case}"), &text), &registry(), &AssetsUnchecked)
                .expect_err(case);
            assert!(
                failure
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.field.as_deref() == Some(*field)),
                "the pack compiler did not refuse `{case}` at `{field}`:\n{}",
                failure.render()
            );
        }
    }

    #[test]
    fn a_compiled_pack_carries_the_catalog_the_runtime_will_load() {
        // The row this closes: before it, the compiler proved the content
        // correct and the game parsed the same file again through a different
        // function. Two readers of one file, with nothing guaranteeing they
        // agreed.
        let draft = draft("lowering", SMALL_CATALOG);
        let pack = compile(&draft, &registry(), &AssetsUnchecked).expect("compiles");
        let catalog = lowered_catalog(&pack).expect("a Runtime schema lowers its artifact");
        assert!(catalog.characters.contains_key("mole"));
        assert!(catalog.brain_presets.contains_key("peaceful"));
        assert_eq!(
            catalog.characters["mole"].display_name, "Mole",
            "the runtime value is the one the compiler validated, not a re-parse"
        );
    }

    #[test]
    fn a_refused_pack_hands_out_no_runtime_value() {
        //  the direction that matters. A caller must never get a usable
        // catalog out of a pack that failed — that is how invalid content
        // reaches a running game while a validator says it refused.
        let draft = draft(
            "refused_lowering",
            &SMALL_CATALOG.replace(r#"tags: [],"#, r#"tags: [], favourite_snack: "worms","#),
        );
        assert!(compile(&draft, &registry(), &AssetsUnchecked).is_err());
    }

    #[test]
    fn one_catalog_file_mints_three_kinds_of_identity() {
        let draft = draft("three_kinds", SMALL_CATALOG);
        let pack = compile(
            &draft,
            &registry(),
            &FixedAssets::new(["mole.png", "mole.ron"]),
        )
        .expect("a well-formed catalog compiles");

        let ids: Vec<String> = pack.content.keys().map(ToString::to_string).collect();
        assert_eq!(
            ids,
            vec![
                "test:action_set_preset/striker_swipe",
                "test:brain_preset/peaceful",
                "test:character/mole",
            ],
            "a character, and the two shared presets it points at, are three identities"
        );
        // Both preset references resolved, and they are recorded so an
        // inspector can answer "which authored value supplied this".
        assert_eq!(pack.resolved_references.len(), 2);
        assert!(
            pack.resolve::<Character>("mole").is_some()
                && pack.resolve::<BrainPresetRefFacet>("peaceful").is_some()
        );
        assert!(
            pack.resolve::<Character>("peaceful").is_none(),
            "a preset is not a character — the typed ref is what stops that being a runtime \
             lookup that silently misses"
        );
    }

    #[test]
    fn a_character_naming_a_preset_that_does_not_exist_is_refused_as_a_preset() {
        let draft = draft(
            "missing_preset",
            &SMALL_CATALOG.replace(
                r#"default_brain: "peaceful""#,
                r#"default_brain: "peacful""#,
            ),
        );
        let failure = compile(&draft, &registry(), &AssetsUnchecked).expect_err("refused");
        assert_eq!(failure.stage, CompileStage::ReferenceResolution);
        assert!(failure.has(DiagnosticCode::UnknownPreset));
        assert!(
            failure.render().contains("did you mean `peaceful`?"),
            "the refusal answers the typo:\n{}",
            failure.render()
        );
    }

    #[test]
    fn a_missing_sprite_sheet_is_refused_and_names_the_character() {
        let draft = draft("missing_sheet", SMALL_CATALOG);
        let failure = compile(&draft, &registry(), &FixedAssets::new(["mole.ron"]))
            .expect_err("mole.png is absent");
        assert_eq!(failure.stage, CompileStage::ReferenceResolution);
        let diagnostic = failure
            .diagnostics
            .iter()
            .find(|d| d.code == DiagnosticCode::MissingAsset)
            .expect("the asset failure");
        assert_eq!(
            diagnostic.subject.as_ref().map(ToString::to_string),
            Some("test:character/mole".to_string())
        );
        assert_eq!(diagnostic.field.as_deref(), Some("spritesheet"));
    }

    #[test]
    fn two_characters_sharing_one_display_name_is_an_authority_conflict() {
        let two = SMALL_CATALOG.replace(
            r#""mole": ("#,
            r#""vole": (
                display_name: "Mole",
                spritesheet: "mole.png",
                manifest: "mole.ron",
                tier: MainHall,
                body_kind: Standard,
                composition: None,
                default_brain: "peaceful",
                default_action_set: "striker_swipe",
                tags: [],
            ),
            "mole": ("#,
        );
        let draft = draft("shared_display_name", &two);
        let failure = compile(&draft, &registry(), &AssetsUnchecked).expect_err("refused");
        assert_eq!(failure.stage, CompileStage::FacetValidation);
        assert!(failure.has(DiagnosticCode::ConflictingModuleContribution));
    }

    #[test]
    fn an_authored_field_the_schema_does_not_consume_is_refused() {
        let draft = draft(
            "unknown_field",
            &SMALL_CATALOG.replace(r#"tags: [],"#, r#"tags: [], favourite_snack: "worms","#),
        );
        let failure = compile(&draft, &registry(), &AssetsUnchecked).expect_err("refused");
        assert_eq!(failure.stage, CompileStage::FacetValidation);
        assert!(
            failure.has(DiagnosticCode::UnknownField),
            "an unconsumed field is the most expensive content bug there is — everything looks \
             authored and nothing happens: {:?}",
            failure.codes()
        );
    }
}
