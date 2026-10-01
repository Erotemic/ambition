//! The host-owned store for module state attached to a body.
//!
//! One [`BodyRecords`] component holds every admitted record of one body, in
//! schema-key order. It is rollback state: the component is cloned for a
//! snapshot (a deep copy of the logical values) and its checksum is the
//! canonical logical checksum of every record. The records retire with the
//! body, because they are a component of the body.

use ambition_extension_sdk::digest::Digest;
use ambition_extension_sdk::{Record, SchemaKey};
use ambition_platformer2d_core::snapshot::RollbackRegistrar;
use bevy::prelude::*;

/// One stored record with the shape digest of the schema it was written
/// under. The digest makes the checksum self-contained: a record written
/// under another shape never compares equal.
#[derive(Clone, Debug, PartialEq)]
pub struct StoredRecord {
    pub key: SchemaKey,
    pub shape: u64,
    pub record: Record,
}

/// Every module record attached to one body.
#[derive(Component, Clone, Debug, Default, PartialEq)]
pub struct BodyRecords {
    records: Vec<StoredRecord>,
}

impl BodyRecords {
    pub fn get(&self, key: &SchemaKey) -> Option<&Record> {
        self.records
            .binary_search_by(|r| r.key.cmp(key))
            .ok()
            .map(|i| &self.records[i].record)
    }

    pub fn put(&mut self, key: SchemaKey, shape: u64, record: Record) {
        match self.records.binary_search_by(|r| r.key.cmp(&key)) {
            Ok(i) => {
                self.records[i].shape = shape;
                self.records[i].record = record;
            }
            Err(i) => self.records.insert(i, StoredRecord { key, shape, record }),
        }
    }

    pub fn records(&self) -> &[StoredRecord] {
        &self.records
    }

    /// The canonical logical checksum: every record's shape and values, in
    /// key order. Not a presence probe.
    pub fn checksum(&self) -> u64 {
        let mut d = Digest::new();
        d.u32(self.records.len() as u32);
        for stored in &self.records {
            d.u64(stored.record.checksum(stored.shape));
        }
        d.finish()
    }
}

const OWNER: &str = "ambition_extension_host";

/// Adds the host's store to the rollback contract.
pub fn register_rollback_state(registrar: &mut impl RollbackRegistrar) {
    registrar.rollback_component_clone_checksum::<BodyRecords>(
        OWNER,
        "extension.body_records",
        "every module record of the body: schema shape digest and canonical values",
        BodyRecords::checksum,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_extension_sdk::{
        Attachment, FieldDecl, FieldKind, FieldRef, SaveEligibility, StateSchema, Value,
    };

    fn schema(key: &'static str) -> StateSchema {
        StateSchema {
            key: SchemaKey::new("test", key, 1),
            attachment: Attachment::Body,
            save: SaveEligibility::Transient,
            fields: vec![FieldDecl::new(1, "n", FieldKind::U32)],
        }
    }

    #[test]
    fn the_checksum_sees_a_value_and_a_shape() {
        let a = schema("a");
        let mut one = BodyRecords::default();
        one.put(a.key.clone(), a.shape_digest(), a.initial_record());
        let mut two = one.clone();
        assert_eq!(one.checksum(), two.checksum());

        let mut changed = a.initial_record();
        changed.set(FieldRef(0), Value::U32(1)).unwrap();
        two.put(a.key.clone(), a.shape_digest(), changed);
        assert_ne!(one.checksum(), two.checksum());

        let mut reshaped = one.clone();
        reshaped.put(a.key.clone(), a.shape_digest() ^ 1, a.initial_record());
        assert_ne!(one.checksum(), reshaped.checksum());
    }

    #[test]
    fn records_stay_in_key_order_whatever_the_write_order() {
        let (a, b) = (schema("a"), schema("b"));
        let mut ab = BodyRecords::default();
        ab.put(a.key.clone(), a.shape_digest(), a.initial_record());
        ab.put(b.key.clone(), b.shape_digest(), b.initial_record());
        let mut ba = BodyRecords::default();
        ba.put(b.key.clone(), b.shape_digest(), b.initial_record());
        ba.put(a.key.clone(), a.shape_digest(), a.initial_record());
        assert_eq!(ab, ba);
        assert_eq!(ab.checksum(), ba.checksum());
    }
}
