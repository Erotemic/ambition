//! Shared immutable versions of prepared data.

use super::Error;
use serde::Serialize;
use std::sync::{Arc, OnceLock};

/// One shared data version. Mutable access creates a separate version when needed.
pub struct PreparedValue<T>(Arc<Version<T>>);

#[derive(Clone)]
struct Version<T> {
    value: T,
    digest: OnceLock<Result<u64, Error>>,
}

impl<T> PreparedValue<T> {
    /// Freeze the supplied data as a new version.
    pub fn new(value: T) -> Self {
        Self(Arc::new(Version {
            value,
            digest: OnceLock::new(),
        }))
    }
}

impl<T: Serialize> PreparedValue<T> {
    /// The canonical data digest, computed once for this version.
    pub fn digest(&self) -> Result<u64, Error> {
        self.0
            .digest
            .get_or_init(|| {
                super::encode(&self.0.value).map(|bytes| {
                    crate::snapshot::PeerDigest::in_domain("prepared-value-v1")
                        .bytes(&bytes)
                        .finish()
                })
            })
            .clone()
    }
}

impl<T: Clone> PreparedValue<T> {
    /// Edit this value without changing versions retained by another owner.
    pub fn make_mut(&mut self) -> &mut T {
        let version = Arc::make_mut(&mut self.0);
        version.digest.take();
        &mut version.value
    }
}

impl<T> Clone for PreparedValue<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl<T: Default> Default for PreparedValue<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}
impl<T> From<T> for PreparedValue<T> {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}
impl<T> std::ops::Deref for PreparedValue<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.0.value
    }
}
impl<T: std::fmt::Debug> std::fmt::Debug for PreparedValue<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.value.fmt(f)
    }
}
impl<T: PartialEq> PartialEq for PreparedValue<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0.value == other.0.value
    }
}
impl<T: Eq> Eq for PreparedValue<T> {}
