//! The registry one composition installs, and the CLI's argument handling.
//!
//! Both live in the lib rather than the bin so a test can call them without
//! spawning a process — and so the CLI provably runs the SAME registry and the
//! SAME `compile` a standard test runs. One validator implementation is the
//! rule; two would disagree, and the disagreement is worse than either being
//! wrong.

use std::path::PathBuf;

use ambition_content_pack::{
    compile_dir, AdvisoryAssets, AssetSource, AssetsUnchecked, CapabilityId, CompileFailure,
    DirectoryAssets, PreparedContentPack, SchemaRegistry,
};

/// The schemas and capabilities this tool composes.
///
/// Adding a capability here is what makes its authored content validatable —
/// which is the same act, in the same vocabulary, as installing it in an app.
pub fn default_registry() -> SchemaRegistry {
    let mut registry = SchemaRegistry::new();
    registry
        .register(ambition_characters::actor::character_catalog::character_catalog_schema())
        .expect("the default registry installs each schema once");
    registry
        .register(ambition_items::content_schema::item_catalog_schema())
        .expect("the default registry installs each schema once");
    registry
        .register(
            ambition_characters::brain::fighter::content_schema::fighter_brain_ladder_schema(),
        )
        .expect("the default registry installs each schema once");
    registry
        .register(ambition_characters::smash_fighter::content_schema::smash_fighter_schema())
        .expect("the default registry installs each schema once");
    registry
        .register(ambition_encounter::content_schema::encounter_waves_schema())
        .expect("the default registry installs each schema once");
    registry
        .register(ambition_boss_encounter::pattern::content_schema::boss_seed_library_schema())
        .expect("the default registry installs each schema once");
    registry
        .register(ambition_boss_encounter::pattern::content_schema::boss_validator_bands_schema())
        .expect("the default registry installs each schema once");
    registry
        .register(ambition_boss_encounter::pattern::content_schema::boss_profiles_schema())
        .expect("the default registry installs each schema once");
    registry
        .register(ambition_boss_encounter::pattern::content_schema::boss_encounter_schema())
        .expect("the default registry installs each schema once");
    registry
        .register(ambition_audio::content_schema::music_registry_schema())
        .expect("the default registry installs each schema once");
    registry
        .register(ambition_audio::content_schema::sfx_registry_schema())
        .expect("the default registry installs each schema once");
    registry
}

/// What the CLI was asked to do.
#[derive(Debug)]
pub struct Invocation {
    pub pack_root: PathBuf,
    pub asset_roots: Vec<PathBuf>,
    /// Capabilities installed beyond the ones the linked schemas bring. Lets a
    /// caller validate a pack against a LARGER composition than this binary
    /// links, which is how a game with its own capabilities uses this tool
    /// before it has its own.
    pub extra_capabilities: Vec<CapabilityId>,
    /// Skip asset existence checks — explicit, so a pack validated this way is
    /// visibly not making a claim about its assets.
    pub skip_asset_check: bool,
    /// Report a missing asset as a WARNING and still compile. The right default
    /// for interactive authoring on a checkout whose git-ignored art was never
    /// generated; `--strict-assets` is the packaging/release answer.
    pub advisory_assets: bool,
    pub list_schemas: bool,
    /// Print only the fingerprint, for a build script or a cache key.
    pub fingerprint_only: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ArgError {
    MissingPackRoot,
    MissingValue(String),
    Unknown(String),
}

impl std::fmt::Display for ArgError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingPackRoot => f.write_str("no pack directory given"),
            Self::MissingValue(flag) => write!(f, "{flag} needs a value"),
            Self::Unknown(flag) => write!(f, "unknown option {flag}"),
        }
    }
}

pub const USAGE: &str = "\
ambition_content — validate a content pack

USAGE:
    ambition_content <pack-dir> [options]

OPTIONS:
    --asset-root <dir>    Look for authored assets under <dir>. Repeatable;
                          roots are tried in order and the winner is recorded
                          as the asset's provenance.
    --no-asset-check      Do not check that assets exist. The prepared pack
                          records `<unchecked>` provenance so it is visibly not
                          claiming they are present.
    --advisory-assets     Report a missing asset as a warning instead of
                          refusing. Use it while authoring on a checkout whose
                          git-ignored art was never generated; the default
                          refuses, which is what packaging wants.
    --capability <id>     Treat <id> as installed. Use it to validate against a
                          composition larger than this binary links.
    --fingerprint         Print only the content fingerprint.
    --list-schemas        List the installed schemas and exit.
    -h, --help            This text.

EXIT CODE:
    0  the pack compiled
    1  the pack was refused (every problem the failing stage could see is
       printed, along with the checks that did not run)
    2  the invocation itself was wrong
";

pub fn parse_args(args: impl IntoIterator<Item = String>) -> Result<Invocation, ArgError> {
    let mut pack_root = None;
    let mut asset_roots = Vec::new();
    let mut extra_capabilities = Vec::new();
    let mut skip_asset_check = false;
    let mut advisory_assets = false;
    let mut list_schemas = false;
    let mut fingerprint_only = false;

    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--asset-root" => asset_roots
                .push(PathBuf::from(args.next().ok_or_else(|| {
                    ArgError::MissingValue("--asset-root".into())
                })?)),
            "--capability" => extra_capabilities
                .push(CapabilityId::new(args.next().ok_or_else(|| {
                    ArgError::MissingValue("--capability".into())
                })?)),
            "--no-asset-check" => skip_asset_check = true,
            "--advisory-assets" => advisory_assets = true,
            "--list-schemas" => list_schemas = true,
            "--fingerprint" => fingerprint_only = true,
            other if other.starts_with('-') => return Err(ArgError::Unknown(other.to_string())),
            other => pack_root = Some(PathBuf::from(other)),
        }
    }

    if list_schemas {
        return Ok(Invocation {
            pack_root: pack_root.unwrap_or_default(),
            asset_roots,
            extra_capabilities,
            skip_asset_check,
            advisory_assets,
            list_schemas,
            fingerprint_only,
        });
    }

    Ok(Invocation {
        pack_root: pack_root.ok_or(ArgError::MissingPackRoot)?,
        asset_roots,
        extra_capabilities,
        skip_asset_check,
        advisory_assets,
        list_schemas,
        fingerprint_only,
    })
}

impl Invocation {
    /// Run it. The registry is built here, so the capabilities named on the
    /// command line and the ones the binary links go through one path.
    pub fn run(&self) -> Result<PreparedContentPack, CompileFailure> {
        let mut registry = default_registry();
        for capability in &self.extra_capabilities {
            registry.install_capability(capability.clone());
        }
        let assets: Box<dyn AssetSource> = if self.skip_asset_check {
            Box::new(AssetsUnchecked)
        } else {
            // No `--asset-root` means the pack's own directory, which is the right default: a
            // pack's assets live in the pack.
            let roots = if self.asset_roots.is_empty() {
                vec![self.pack_root.clone()]
            } else {
                self.asset_roots.clone()
            };
            if self.advisory_assets {
                Box::new(AdvisoryAssets::new(roots))
            } else {
                Box::new(DirectoryAssets::new(roots))
            }
        };
        compile_dir(&self.pack_root, &registry, assets.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn the_default_registry_installs_the_character_capability() {
        let registry = default_registry();
        assert!(registry.has_capability(&CapabilityId::new("characters")));
        assert!(registry
            .get(&ambition_content_pack::SchemaId::new("character_catalog"))
            .is_some());
    }

    #[test]
    fn arguments_parse_into_one_invocation() {
        let invocation = parse_args(args(&[
            "packs/cast",
            "--asset-root",
            "assets",
            "--asset-root",
            "shared",
            "--capability",
            "combat",
        ]))
        .expect("parses");
        assert_eq!(invocation.pack_root, PathBuf::from("packs/cast"));
        assert_eq!(invocation.asset_roots.len(), 2);
        assert_eq!(
            invocation.extra_capabilities,
            vec![CapabilityId::new("combat")]
        );
        assert!(!invocation.skip_asset_check && !invocation.advisory_assets);
    }

    #[test]
    fn a_bad_invocation_is_refused_rather_than_defaulted() {
        assert_eq!(
            parse_args(args(&[])).unwrap_err(),
            ArgError::MissingPackRoot
        );
        assert_eq!(
            parse_args(args(&["pack", "--asset-root"])).unwrap_err(),
            ArgError::MissingValue("--asset-root".into())
        );
        assert_eq!(
            parse_args(args(&["pack", "--verify-everything"])).unwrap_err(),
            ArgError::Unknown("--verify-everything".into())
        );
    }
}
