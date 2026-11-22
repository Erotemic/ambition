//! Who owns a contribution to a body: independently revocable applications.
//!
//! A hold, a pose or another term on a body is supplied by an application:
//! one issuer's claim, which that issuer can withdraw without touching
//! another issuer's claim on the same body. The domains keep their own
//! reductions (any hold suppresses control; the highest pose wins). This
//! module owns only the identity and the per-application term storage.
//!
//! A key holds no `Entity`, component id, wall clock or insertion order, so
//! two peers and a rewind agree on it. The owner page is
//! `docs/planning/engine/actor-effects-and-modifier-composition.md` (6a).

use std::sync::Arc;

/// One live object occurrence: its session and its allocation (the room
/// that allocated it, if any, and the sequence there). Not a semantic
/// address: an object retired and made again under the same name is a new
/// occurrence.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OccurrenceKey {
    pub session: u64,
    pub room: Option<u32>,
    pub sequence: u64,
}

/// The owner that issued an application.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Issuer {
    /// A native mechanic, by its stable name (`transform_beat`).
    Native(&'static str),
    /// A behaviour attachment: the live occurrence of the state whose
    /// records it keeps, and its `provider::module` path. Two entries of one
    /// attachment are one issuer; the same module attached to two objects is
    /// two, and so is the same module on a remade object.
    Attachment { occurrence: OccurrenceKey, module: Arc<str> },
}

/// One independently revocable application: its issuer and the issuer's
/// own key for it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ApplicationKey {
    pub issuer: Issuer,
    pub local: u32,
}

impl ApplicationKey {
    /// A native mechanic's one application.
    pub const fn native(name: &'static str) -> Self {
        Self { issuer: Issuer::Native(name), local: 0 }
    }

    /// An attachment's one application of a kind (its hold, its pose).
    pub fn attachment(occurrence: OccurrenceKey, module: &str) -> Self {
        Self { issuer: Issuer::Attachment { occurrence, module: module.into() }, local: 0 }
    }

    /// The attachment that issued it, when an attachment did.
    pub fn attachment_issuer(&self) -> Option<(OccurrenceKey, &str)> {
        match &self.issuer {
            Issuer::Attachment { occurrence, module } => Some((*occurrence, module)),
            Issuer::Native(_) => None,
        }
    }

    fn checksum_into(&self, out: &mut Vec<u8>) {
        use crate::snapshot::{put_str, put_u32, put_u64};
        match &self.issuer {
            Issuer::Native(name) => {
                put_u32(out, 0);
                put_str(out, name);
            }
            Issuer::Attachment { occurrence, module } => {
                put_u32(out, 1);
                put_u64(out, occurrence.session);
                match occurrence.room {
                    Some(room) => {
                        put_u32(out, 1);
                        put_u32(out, room);
                    }
                    None => put_u32(out, 0),
                }
                put_u64(out, occurrence.sequence);
                put_str(out, module);
            }
        }
        put_u32(out, self.local);
    }
}

/// The `provider::module` part of an extension entry path
/// (`provider::module/entry`).
pub fn module_of_entry(entry: &str) -> &str {
    entry.split_once('/').map_or(entry, |(module, _)| module)
}

/// Terms on one body, one per application, in key order.
#[derive(Clone, Debug, PartialEq)]
pub struct OwnedTerms<T> {
    rows: Vec<(ApplicationKey, T)>,
}

impl<T> Default for OwnedTerms<T> {
    fn default() -> Self {
        Self { rows: Vec::new() }
    }
}

impl<T: PartialEq> OwnedTerms<T> {
    /// `key` supplies `term`, replacing its earlier term. False when it
    /// already supplied exactly this term.
    pub fn claim(&mut self, key: ApplicationKey, term: T) -> bool {
        match self.rows.binary_search_by(|(at, _)| at.cmp(&key)) {
            Ok(at) if self.rows[at].1 == term => false,
            Ok(at) => {
                self.rows[at].1 = term;
                true
            }
            Err(at) => {
                self.rows.insert(at, (key, term));
                true
            }
        }
    }
}

impl<T> OwnedTerms<T> {
    /// Withdraw `key`'s term, and only that term. False when it had none.
    pub fn release(&mut self, key: &ApplicationKey) -> bool {
        match self.rows.binary_search_by(|(at, _)| at.cmp(key)) {
            Ok(at) => {
                self.rows.remove(at);
                true
            }
            Err(_) => false,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn contains(&self, key: &ApplicationKey) -> bool {
        self.rows.binary_search_by(|(at, _)| at.cmp(key)).is_ok()
    }

    /// Every application and its term, in key order.
    pub fn iter(&self) -> impl Iterator<Item = (&ApplicationKey, &T)> {
        self.rows.iter().map(|(key, term)| (key, term))
    }

    /// Entity-free value projection; `term` writes one term's bytes.
    pub fn checksum(&self, term: impl Fn(&T, &mut Vec<u8>)) -> u64 {
        let mut bytes = Vec::new();
        crate::snapshot::put_u64(&mut bytes, self.rows.len() as u64);
        for (key, value) in &self.rows {
            key.checksum_into(&mut bytes);
            term(value, &mut bytes);
        }
        crate::snapshot::checksum_bytes(&bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BOB: OccurrenceKey = OccurrenceKey { session: 1, room: Some(0), sequence: 7 };

    #[test]
    fn a_release_withdraws_only_its_own_application() {
        let (a, b) = (ApplicationKey::attachment(BOB, "p::a"), ApplicationKey::attachment(BOB, "p::b"));
        let mut terms = OwnedTerms::default();
        assert!(terms.claim(a.clone(), 1));
        assert!(terms.claim(b.clone(), 2));
        assert!(!terms.claim(a.clone(), 1), "a repeated claim is one claim");
        assert!(terms.release(&a));
        assert!(!terms.release(&a));
        assert_eq!(terms.iter().map(|(key, term)| (key.clone(), *term)).collect::<Vec<_>>(), vec![(b, 2)]);
    }

    #[test]
    fn entries_of_one_attachment_are_one_issuer() {
        assert_eq!(module_of_entry("prov::shell/on_hit"), "prov::shell");
        assert_eq!(
            ApplicationKey::attachment(BOB, module_of_entry("prov::shell/start")),
            ApplicationKey::attachment(BOB, module_of_entry("prov::shell/finish")),
        );
    }

    #[test]
    fn a_remade_occurrence_is_another_issuer() {
        let remade = OccurrenceKey { sequence: 8, ..BOB };
        assert_ne!(ApplicationKey::attachment(BOB, "p::shell"), ApplicationKey::attachment(remade, "p::shell"));
    }
}
