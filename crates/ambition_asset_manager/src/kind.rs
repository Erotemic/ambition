//! [`AssetKind`] — coarse Ambition-side classification of an asset.
//!
//! `AssetKind` is the *type the catalog thinks the asset is*, not the
//! Rust type Bevy will hand back. Bevy keeps its own typed
//! `AssetServer::load::<T: Asset>` API — the catalog stores the kind so
//! resolver code can pick reasonable defaults (cache policy, fallback
//! handling, embed eligibility) without callers spelling it out at every
//! load site.
//!
//! Treat the variants as a closed set; add new ones only when a real
//! decision branches on the kind. `Other` exists for one-off bytes that
//! don't warrant a dedicated variant.

use serde::{Deserialize, Serialize};

/// Coarse asset category. Drives default cache policy and (later)
/// embed-eligibility heuristics. The Rust type Bevy resolves to is
/// independent of this and lives at the call site.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AssetKind {
    /// PNG / WebP / KTX texture loaded into Bevy's `Image` asset.
    Image,
    /// Short uncompressed / OGG sound clip. Long streamed tracks should
    /// still use `AudioClip`; `AudioBank` is the packed multi-clip case.
    AudioClip,
    /// `.sfxbank` packed multi-clip container (see `ambition_sfx_bank`).
    AudioBank,
    /// LDtk project JSON, parsed by `bevy_ecs_ldtk` (or our direct loader
    /// in `ambition_actors::ldtk_world::loading`).
    LdtkProject,
    /// `.ron` data file (tuning, manifest, encounter spec).
    RonData,
    /// `.json` data file.
    JsonData,
    /// `.ttf` / `.otf` font.
    Font,
    /// WGSL / SPIR-V shader source.
    Shader,
    /// Opaque bytes — small adapter-style loads (e.g. an SFX bank read
    /// directly into memory).
    Binary,
    /// Whole-directory entry. Useful for catalog entries that point at a
    /// folder of incremental content (sprite atlases) rather than one
    /// file. The resolver does not enumerate; consumers do.
    Directory,
    /// Catch-all for one-off assets that don't warrant a variant.
    Other,
}

impl AssetKind {
    /// Stable lower-snake-case label, used by tests + diagnostics.
    pub fn label(self) -> &'static str {
        match self {
            Self::Image => "image",
            Self::AudioClip => "audio_clip",
            Self::AudioBank => "audio_bank",
            Self::LdtkProject => "ldtk_project",
            Self::RonData => "ron_data",
            Self::JsonData => "json_data",
            Self::Font => "font",
            Self::Shader => "shader",
            Self::Binary => "binary",
            Self::Directory => "directory",
            Self::Other => "other",
        }
    }

    /// Whether Bevy's typed asset pipeline natively understands this
    /// kind. `AudioBank` is `false` because it's an Ambition-specific
    /// container — its bytes flow through a non-Bevy adapter
    /// (see [`crate::sfx_integration`]).
    pub fn bevy_native(self) -> bool {
        !matches!(self, Self::AudioBank | Self::Binary | Self::Directory)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_are_distinct() {
        let kinds = [
            AssetKind::Image,
            AssetKind::AudioClip,
            AssetKind::AudioBank,
            AssetKind::LdtkProject,
            AssetKind::RonData,
            AssetKind::JsonData,
            AssetKind::Font,
            AssetKind::Shader,
            AssetKind::Binary,
            AssetKind::Directory,
            AssetKind::Other,
        ];
        let labels: std::collections::HashSet<_> = kinds.iter().map(|k| k.label()).collect();
        assert_eq!(labels.len(), kinds.len());
    }

    #[test]
    fn bevy_native_excludes_byte_adapter_kinds() {
        assert!(AssetKind::Image.bevy_native());
        assert!(AssetKind::LdtkProject.bevy_native());
        assert!(!AssetKind::AudioBank.bevy_native());
        assert!(!AssetKind::Binary.bevy_native());
        assert!(!AssetKind::Directory.bevy_native());
    }
}
