//! [`AssetId`] — stable logical identifier for an asset entry.
//!
//! The wire form is a dotted lowercase string, e.g.
//! `sprite.entity.chest_closed`, `world.sandbox_ldtk`, or
//! `audio.sfx_bank`. The string is the canonical id: equality, hashing,
//! manifest lookup, and serialization all key off the string.
//!
//! The id intentionally does NOT encode the location, profile, file
//! extension, or whether the asset is required. Those are properties of
//! the [`crate::manifest::AssetEntry`] keyed by id.
//!
//! Keep ids stable across underlying file renames; the manifest redirects
//! locations while gameplay keeps referencing the same id.

use serde::{Deserialize, Serialize};

/// Stable logical identifier for an asset entry.
///
/// Construct with [`AssetId::new`] for runtime strings or
/// [`AssetId::from_static`] for `&'static str` literals (the latter
/// avoids an allocation but is otherwise equivalent).
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AssetId(String);

impl AssetId {
    /// Construct from any string-like value.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Construct from a `'static` literal. Same behavior as `new`, but useful
    /// for stable catalog ids declared as constants.
    pub fn from_static(value: &'static str) -> Self {
        Self(value.to_string())
    }

    /// Borrow the underlying canonical string.
    #[inline]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume into the owned string.
    pub fn into_string(self) -> String {
        self.0
    }
}

impl std::fmt::Display for AssetId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for AssetId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for AssetId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl std::str::FromStr for AssetId {
    type Err = std::convert::Infallible;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::new(s))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equality_is_string_equality() {
        assert_eq!(
            AssetId::new("sprite.entity.chest_closed"),
            AssetId::from_static("sprite.entity.chest_closed")
        );
        assert_ne!(
            AssetId::new("sprite.entity.chest_closed"),
            AssetId::new("sprite.entity.chest_open"),
        );
    }

    #[test]
    fn display_round_trips_canonical_form() {
        let id = AssetId::new("audio.sfx_bank");
        assert_eq!(format!("{id}"), "audio.sfx_bank");
        assert_eq!(id.as_str(), "audio.sfx_bank");
    }

    #[test]
    fn from_str_is_infallible() {
        let id: AssetId = "world.sandbox_ldtk".parse().unwrap();
        assert_eq!(id, AssetId::new("world.sandbox_ldtk"));
    }

    #[test]
    fn serde_round_trips_as_transparent_string() {
        let id = AssetId::new("sprite.entity.chest_closed");
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"sprite.entity.chest_closed\"");
        let back: AssetId = serde_json::from_str(&json).unwrap();
        assert_eq!(back, id);
    }
}
