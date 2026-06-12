//! Small adapter between the asset catalog and [`ambition_sfx::BankProvider`].
//!
//! The catalog resolves an `AudioBank` id to an [`crate::location::AssetLocation`];
//! this adapter handles synchronous locations (`LocalPath`, `Embedded` with
//! caller-supplied bytes). HTTP / IPFS remain async loader responsibilities.
//! Embedded bytes must be supplied by the consuming crate so `include_bytes!`
//! can see a literal path.

#![cfg(feature = "sfx")]

use std::path::Path;

use thiserror::Error;

use ambition_sfx::{BankProvider, SfxError};

use crate::location::AssetLocation;
use crate::resolver::ResolvedAsset;

/// Errors the SFX adapter can return on top of [`SfxError`].
#[derive(Debug, Error)]
pub enum SfxBankResolveError {
    /// The catalog returned `Disabled` for this id under the active
    /// profile — the SFX system should fall back to
    /// [`ambition_sfx::SilentProvider`] or layered defaults.
    #[error("asset disabled under the active profile")]
    Disabled,
    /// The catalog produced a location the synchronous adapter can't
    /// handle (HTTP / IPFS). Use an async Bevy loader instead.
    #[error("location {0:?} is async-only; use a Bevy AssetServer load")]
    AsyncOnlyLocation(AssetLocation),
    /// `Embedded` location was returned but the caller passed
    /// `None` for the byte slice — only the consumer can `include_bytes!`
    /// the literal path.
    #[error("embedded location {0:?} needs caller-supplied bytes; pass them via build_provider_from_resolved")]
    EmbeddedBytesMissing(AssetLocation),
    /// Underlying SFX backend error (bad bank format, IO failure, ...).
    #[error("sfx backend: {0}")]
    Sfx(#[from] SfxError),
}

/// Build a [`BankProvider`] from a [`ResolvedAsset`].
///
/// `embedded_bytes` is consulted when the resolved location is
/// `Embedded(_)` — caller must pass `Some(include_bytes!("..."))` from
/// their own crate, where the literal path is visible.
///
/// Returns `Err(SfxBankResolveError::Disabled)` for disabled locations
/// so the SFX system can layer a [`ambition_sfx::SilentProvider`] on top
/// instead of fataling.
pub fn build_provider_from_resolved(
    resolved: &ResolvedAsset,
    embedded_bytes: Option<&[u8]>,
) -> Result<BankProvider, SfxBankResolveError> {
    match &resolved.location {
        AssetLocation::Disabled => Err(SfxBankResolveError::Disabled),
        AssetLocation::LocalPath(path) => Ok(BankProvider::from_path(path)?),
        AssetLocation::Embedded(_) => match embedded_bytes {
            Some(bytes) => Ok(BankProvider::from_bytes(bytes.to_vec())?),
            None => Err(SfxBankResolveError::EmbeddedBytesMissing(
                resolved.location.clone(),
            )),
        },
        loc @ (AssetLocation::BevyPath(_)
        | AssetLocation::BevySourcePath { .. }
        | AssetLocation::HttpUrl(_)
        | AssetLocation::IpfsGateway { .. }) => {
            Err(SfxBankResolveError::AsyncOnlyLocation(loc.clone()))
        }
    }
}

/// Convenience: build a [`BankProvider`] directly from a local path,
/// bypassing the catalog. Mostly here so existing call sites can
/// migrate one helper at a time.
pub fn build_provider_from_path(path: &Path) -> Result<BankProvider, SfxBankResolveError> {
    Ok(BankProvider::from_path(path)?)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::id::AssetId;
    use crate::kind::AssetKind;
    use crate::policy::{CachePolicy, MissingAssetPolicy};
    use crate::profile::{AssetProfile, AssetSourceProfile};

    fn resolved(location: AssetLocation) -> ResolvedAsset {
        ResolvedAsset {
            id: AssetId::new("audio.sfx_bank"),
            kind: AssetKind::AudioBank,
            profile: AssetProfile::DesktopDevLoose,
            location,
            missing_policy: MissingAssetPolicy::WarnAndPlaceholder,
            cache_policy: CachePolicy::LifetimeOfProcess,
            preload_group: None,
            source_used: Some(AssetSourceProfile::LooseFilesystem),
            authored_candidate: true,
        }
    }

    /// `BankProvider` does not implement `Debug`, so we can't call
    /// `.unwrap_err()` on a `Result<BankProvider, _>`. Match instead
    /// and assert via `matches!` on the error arm.
    fn expect_err(result: Result<BankProvider, SfxBankResolveError>) -> SfxBankResolveError {
        match result {
            Ok(_) => panic!("expected SfxBankResolveError, got Ok(BankProvider)"),
            Err(e) => e,
        }
    }

    #[test]
    fn disabled_returns_disabled_error() {
        let err = expect_err(build_provider_from_resolved(
            &resolved(AssetLocation::Disabled),
            None,
        ));
        assert!(matches!(err, SfxBankResolveError::Disabled));
    }

    #[test]
    fn http_returns_async_only_error() {
        let err = expect_err(build_provider_from_resolved(
            &resolved(AssetLocation::HttpUrl("https://x/y".into())),
            None,
        ));
        assert!(matches!(err, SfxBankResolveError::AsyncOnlyLocation(_)));
    }

    #[test]
    fn ipfs_returns_async_only_error() {
        let err = expect_err(build_provider_from_resolved(
            &resolved(AssetLocation::IpfsGateway {
                gateway: "https://gw".into(),
                cid: "bafy".into(),
                path: "sfx.bank".into(),
            }),
            None,
        ));
        assert!(matches!(err, SfxBankResolveError::AsyncOnlyLocation(_)));
    }

    #[test]
    fn embedded_without_bytes_returns_specific_error() {
        let err = expect_err(build_provider_from_resolved(
            &resolved(AssetLocation::embedded("audio/sfx.bank")),
            None,
        ));
        assert!(matches!(err, SfxBankResolveError::EmbeddedBytesMissing(_)));
    }

    #[test]
    fn local_path_to_missing_file_returns_underlying_sfx_error() {
        // Resolve to a path that definitely doesn't exist; the SFX
        // backend produces an Io error which we wrap as
        // SfxBankResolveError::Sfx. The point is that the adapter
        // doesn't silently swallow the failure.
        let err = expect_err(build_provider_from_resolved(
            &resolved(AssetLocation::LocalPath(PathBuf::from(
                "/definitely/not/a/real/path/sfx.bank",
            ))),
            None,
        ));
        assert!(matches!(err, SfxBankResolveError::Sfx(_)));
    }
}
