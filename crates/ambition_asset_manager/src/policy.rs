//! Policy enums for handling missing assets and caching.
//!
//! These are decided per-[`crate::manifest::AssetEntry`], not per-profile.
//! The active [`crate::profile::AssetProfile`] influences whether a
//! location resolves at all; this layer governs what to do *after* a
//! resolution failure ("the location returned `Disabled` / the bytes
//! never arrived").

use serde::{Deserialize, Serialize};

/// What to do when the resolver hands back
/// [`crate::location::AssetLocation::Disabled`] or the bytes/handle
/// never finishes loading.
///
/// The resolver never panics on its own; callers consult this policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MissingAssetPolicy {
    /// Return a hard error. Use for assets the game truly cannot run
    /// without (the LDtk bootstrap, the default font on web).
    Error,
    /// Log a warning and substitute a runtime placeholder (Bevy
    /// fallback image, silent SFX, blank text). Default for most game
    /// content.
    WarnAndPlaceholder,
    /// Same as `WarnAndPlaceholder` but without the log line. Use for
    /// assets that are *expected* to be absent under stripped profiles
    /// (entity sprite PNGs when `--no-assets` is set).
    SilentPlaceholder,
}

impl MissingAssetPolicy {
    pub fn label(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::WarnAndPlaceholder => "warn_and_placeholder",
            Self::SilentPlaceholder => "silent_placeholder",
        }
    }

    /// `true` for [`Self::Error`] only. Helper so call sites that
    /// branch on "required vs optional" don't have to match on three
    /// variants.
    pub fn is_required(self) -> bool {
        matches!(self, Self::Error)
    }

    /// `true` if the policy logs when an asset is missing. Used by the
    /// resolver to keep `WarnAndPlaceholder` noisy and
    /// `SilentPlaceholder` quiet.
    pub fn should_warn(self) -> bool {
        matches!(self, Self::WarnAndPlaceholder)
    }
}

impl Default for MissingAssetPolicy {
    fn default() -> Self {
        Self::WarnAndPlaceholder
    }
}

/// Cache policy hint for the asset. The resolver does NOT enforce this;
/// it surfaces the value so a future Bevy asset-server tuning pass (or a
/// custom byte adapter) can respect it. Bevy already caches handles by
/// reference count; this is for the consumer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CachePolicy {
    /// Keep the asset alive for the lifetime of the process. Defaults
    /// for `audio.sfx_bank`, the LDtk project, fonts.
    LifetimeOfProcess,
    /// Keep alive while at least one consumer holds a handle. Default
    /// for room-tied sprites and per-encounter audio.
    Refcounted,
    /// Drop after a single read. Useful for one-shot manifest /
    /// configuration files.
    OneShot,
}

impl Default for CachePolicy {
    fn default() -> Self {
        Self::Refcounted
    }
}

impl CachePolicy {
    pub fn label(self) -> &'static str {
        match self {
            Self::LifetimeOfProcess => "lifetime_of_process",
            Self::Refcounted => "refcounted",
            Self::OneShot => "one_shot",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn required_only_for_error_policy() {
        assert!(MissingAssetPolicy::Error.is_required());
        assert!(!MissingAssetPolicy::WarnAndPlaceholder.is_required());
        assert!(!MissingAssetPolicy::SilentPlaceholder.is_required());
    }

    #[test]
    fn warn_only_for_warn_policy() {
        assert!(MissingAssetPolicy::WarnAndPlaceholder.should_warn());
        assert!(!MissingAssetPolicy::SilentPlaceholder.should_warn());
        assert!(!MissingAssetPolicy::Error.should_warn());
    }

    #[test]
    fn defaults_are_sensible() {
        assert_eq!(
            MissingAssetPolicy::default(),
            MissingAssetPolicy::WarnAndPlaceholder
        );
        assert_eq!(CachePolicy::default(), CachePolicy::Refcounted);
    }
}
