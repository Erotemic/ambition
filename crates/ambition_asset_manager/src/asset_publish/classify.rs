//! Classify a generated asset file by what it *is*, independent of where it
//! currently sits. The classifier is the shared brain behind both halves of
//! the publish boundary: the publisher uses it to select which staged files
//! belong in a runtime root, and the runtime-root hygiene validator uses it to
//! detect diagnostics that leaked into a runtime root.
//!
//! Classification is by filename/path shape only — no IO, no content parsing —
//! so it is cheap, deterministic, and usable from a build script, a test, or a
//! future publisher binary alike.

use std::path::Path;

/// The durable class of a generated asset file.
///
/// The first four variants are runtime artifacts the publisher installs. The
/// last two are the reason the boundary exists: [`Intermediate`] files are
/// author-time scaffolding that should not ship, and [`Diagnostic`] files are
/// human-only visual outputs that must never appear under a runtime root.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactClass {
    /// Runtime sprite-sheet metadata (`*_spritesheet.ron`). Baked into the
    /// binary by `build.rs`.
    SheetRecord,
    /// Runtime atlas page image (`*_spritesheet.png`, `*_spritesheet.1.png`, …).
    ImagePage,
    /// Transitional actor-contract sidecar (`*_actor.ron`). Installed today,
    /// not yet consumed by the sandbox.
    ActorSidecar,
    /// Forward-path entity-contract fragment (`*_entity.ron`). Reserved for the
    /// EntityCatalog migration; installed as a runtime candidate.
    EntityFragment,
    /// Runtime sprite-pack catalog (`ultrapack.json` under a pack tier root):
    /// the shared-page atlas index the ultrapacker emits per quality tier.
    /// JSON on purpose (Python-authored RON is the drift trap). Baked into the
    /// binary by `build.rs` alongside the sheet RONs.
    PackCatalog,
    /// Any other file the runtime genuinely loads: standalone prop/entity
    /// images (`props/axe.png`, `entities/…`), authored data RON
    /// (`data/character_catalog.ron`), and similar. Installed.
    RuntimeMisc,
    /// Author-time intermediate that is neither a runtime artifact nor a
    /// hard-error diagnostic: throwaway YAML sidecars, tooling manifests
    /// (`*.json`), and developer notes (`*.md`). A warning if found under a
    /// runtime root, never a hard error.
    Intermediate,
    /// Author-time visual diagnostic: canonical reference poses, labeled
    /// preview sheets, and pixel-grid debug overlays. These must live outside
    /// runtime roots; finding one under a runtime root is a hard error.
    Diagnostic,
}

impl ArtifactClass {
    /// Is this a runtime artifact the publisher installs into a runtime root?
    pub fn is_runtime(self) -> bool {
        matches!(
            self,
            ArtifactClass::SheetRecord
                | ArtifactClass::ImagePage
                | ArtifactClass::ActorSidecar
                | ArtifactClass::EntityFragment
                | ArtifactClass::PackCatalog
                | ArtifactClass::RuntimeMisc
        )
    }

    /// A stable manifest `kind` string for this class (see the PublishManifest
    /// `kind:` vocabulary in the planning doc).
    pub fn manifest_kind(self) -> &'static str {
        match self {
            ArtifactClass::SheetRecord => "sheet_record",
            ArtifactClass::ImagePage => "image_page",
            ArtifactClass::ActorSidecar => "actor_contract_sidecar",
            ArtifactClass::EntityFragment => "entity_contract_fragment",
            ArtifactClass::PackCatalog => "sprite_pack_catalog",
            ArtifactClass::RuntimeMisc => "runtime_misc",
            ArtifactClass::Intermediate => "intermediate",
            ArtifactClass::Diagnostic => "diagnostic",
        }
    }
}

/// True when any directory component of the path is a diagnostics container the
/// generator writes reference art into (e.g. the `canonicals/` gallery dir).
fn in_diagnostic_dir(rel_path: &Path) -> bool {
    rel_path
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .any(|c| c == "canonicals" || c == "diagnostics")
}

/// The visual-diagnostic filename suffixes the renderer emits alongside runtime
/// sheets. Kept in one place so the publisher and the hygiene validator agree.
const DIAGNOSTIC_SUFFIXES: &[&str] = &[
    "_canonical.png",
    "_canonical_transparent.png",
    "_preview_labeled.png",
    "_parts_debug.png",
    "_debug.png",
];

/// `true` for `foo_spritesheet.png` and its extra-page siblings
/// `foo_spritesheet.1.png`, `foo_spritesheet.2.png`, …
fn is_image_page(name: &str) -> bool {
    if name.ends_with("_spritesheet.png") {
        return true;
    }
    // `<stem>_spritesheet.<n>.png`
    let Some(rest) = name.strip_suffix(".png") else {
        return false;
    };
    let Some((head, tail)) = rest.rsplit_once('.') else {
        return false;
    };
    head.ends_with("_spritesheet") && !tail.is_empty() && tail.bytes().all(|b| b.is_ascii_digit())
}

/// `true` for the ultrapacker's shared page images: `ultrapack_0.png`,
/// `ultrapack_41.png`, … (one uniform-size page of a quality-tier pack).
fn is_pack_page(name: &str) -> bool {
    name.strip_prefix("ultrapack_")
        .and_then(|rest| rest.strip_suffix(".png"))
        .is_some_and(|digits| !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()))
}

/// Classify a generated file by its path relative to a staging or runtime root.
///
/// Only the path shape is inspected; the file need not exist.
pub fn classify(rel_path: &Path) -> ArtifactClass {
    let name = rel_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();

    // Diagnostics first: a file is diagnostic if it sits in a diagnostics dir,
    // carries a known diagnostic suffix, or is the contact-sheet gallery.
    if in_diagnostic_dir(rel_path)
        || name == "canonicals_contact_sheet.png"
        || DIAGNOSTIC_SUFFIXES.iter().any(|s| name.ends_with(s))
    {
        return ArtifactClass::Diagnostic;
    }

    if name.ends_with("_spritesheet.ron") {
        return ArtifactClass::SheetRecord;
    }
    if name.ends_with("_actor.ron") {
        return ArtifactClass::ActorSidecar;
    }
    if name.ends_with("_entity.ron") {
        return ArtifactClass::EntityFragment;
    }
    if is_image_page(name) || is_pack_page(name) {
        return ArtifactClass::ImagePage;
    }
    if name == "ultrapack.json" {
        return ArtifactClass::PackCatalog;
    }

    // Author-time scaffolding that isn't a runtime artifact but isn't a
    // hard-error diagnostic either.
    if name.ends_with(".yaml") || name.ends_with(".json") || name.ends_with(".md") {
        return ArtifactClass::Intermediate;
    }

    // Everything else the runtime genuinely loads: standalone prop/entity
    // images, authored data RON, and any not-yet-categorized runtime file.
    ArtifactClass::RuntimeMisc
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn c(p: &str) -> ArtifactClass {
        classify(&PathBuf::from(p))
    }

    #[test]
    fn runtime_artifacts_classify_as_installable() {
        assert_eq!(c("goblin_spritesheet.ron"), ArtifactClass::SheetRecord);
        assert_eq!(c("goblin_spritesheet.png"), ArtifactClass::ImagePage);
        assert_eq!(c("goblin_spritesheet.1.png"), ArtifactClass::ImagePage);
        assert_eq!(c("goblin_spritesheet.12.png"), ArtifactClass::ImagePage);
        assert_eq!(c("goblin_actor.ron"), ArtifactClass::ActorSidecar);
        assert_eq!(c("goblin_entity.ron"), ArtifactClass::EntityFragment);
        assert_eq!(c("base/ultrapack_0.png"), ArtifactClass::ImagePage);
        assert_eq!(c("potato/ultrapack_13.png"), ArtifactClass::ImagePage);
        assert_eq!(c("base/ultrapack.json"), ArtifactClass::PackCatalog);
        assert_eq!(c("props/axe.png"), ArtifactClass::RuntimeMisc);
        assert_eq!(c("data/character_catalog.ron"), ArtifactClass::RuntimeMisc);
        for cls in [
            c("goblin_spritesheet.ron"),
            c("props/axe.png"),
            c("goblin_actor.ron"),
        ] {
            assert!(cls.is_runtime());
        }
    }

    #[test]
    fn visual_diagnostics_are_never_runtime() {
        for path in [
            "shrine_canonical.png",
            "shrine_canonical_transparent.png",
            "gnu_ton_boss/gnu_ton_boss_preview_labeled.png",
            "mockingbird_boss/mockingbird_boss_parts_debug.png",
            "boss_spritesheet_debug.png",
            "canonicals/alice_canonical.png",
            "canonicals_contact_sheet.png",
            "props/shrine_canonical.png",
        ] {
            let cls = classify(&PathBuf::from(path));
            assert_eq!(
                cls,
                ArtifactClass::Diagnostic,
                "{path} should be diagnostic"
            );
            assert!(!cls.is_runtime(), "{path} must not be installable");
        }
    }

    #[test]
    fn intermediates_warn_not_error() {
        assert_eq!(c("goblin_spritesheet.yaml"), ArtifactClass::Intermediate);
        assert_eq!(c("ldtk_sprite_manifest.json"), ArtifactClass::Intermediate);
        assert_eq!(
            c("mockingbird_boss/sources_and_inspirations.md"),
            ArtifactClass::Intermediate
        );
        assert!(!ArtifactClass::Intermediate.is_runtime());
    }
}
