//! References to one live object occurrence in a gameplay session.

use crate::{FieldKind, Value, typed::RecordField};

/// This address retains its allocation namespace when the object changes rooms.
/// Native owners validate the session and the live occurrence before use.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObjectRef {
    pub session: u64,
    pub allocation_room: Option<u32>,
    pub sequence: u64,
}

impl ObjectRef {
    /// An absent initial address. Use `Option<ObjectRef>` for optional state.
    pub const UNBOUND: Self = Self {
        session: u64::MAX, allocation_room: None, sequence: u64::MAX,
    };

    pub fn is_bound(self) -> bool { self != Self::UNBOUND }
}

impl Default for ObjectRef {
    fn default() -> Self { Self::UNBOUND }
}

crate::port_record! { ObjectRef {
    /// Exact session identity, as unsigned decimal text in scripts.
    session: u64,
    /// Allocation room, or absent for a session-lifetime object.
    allocation_room: Option<u32>,
    /// Exact occurrence sequence, as unsigned decimal text in scripts.
    sequence: u64,
} }

impl RecordField for ObjectRef {
    fn kind(_: Option<u32>) -> FieldKind { FieldKind::Object }
    fn to_value(&self) -> Value { Value::Object(*self) }
    fn from_value(value: &Value) -> Option<Self> { value.as_object() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Attachment, FieldDecl, Record, SaveEligibility, SchemaKey, StateSchema,
        script::{record_to_script, record_from_script}, wire::{decode_all, WireReader}};

    #[test]
    fn nested_object_addresses_keep_exact_values_and_reject_save_capture() {
        let address = ObjectRef {session: (1 << 60) + 9, allocation_room: Some(17), sequence: (1 << 61) + 5};
        let kind = FieldKind::seq(FieldKind::opt(FieldKind::Object), 3);
        let schema = StateSchema {key: SchemaKey::new("test", "objects", 1),
            attachment: Attachment::Body, save: SaveEligibility::Transient,
            fields: vec![FieldDecl::new(1, "objects", kind)]};
        schema.validate().unwrap();
        let record = Record::from_values(vec![Value::Seq(vec![Value::some(Value::Object(address)), Value::Opt(None)])]);
        let script = record_to_script(&schema, &record).unwrap();
        assert_eq!(record_from_script(&schema, &script).unwrap(), record);
        let mut bytes = Vec::new();
        record.put_wire(&mut bytes);
        assert_eq!(schema.read_record(&mut WireReader::new(&bytes)).unwrap(), record);
        let mut canonical = Vec::new(); record.encode(&mut canonical);
        assert_eq!(canonical, bytes);
        bytes.clear(); crate::structure::put(&address, &mut bytes);
        assert_eq!(decode_all(&bytes, crate::structure::read::<ObjectRef>).unwrap(), address);
        for save in [SaveEligibility::Checkpoint, SaveEligibility::Durable] {
            let schema = StateSchema {save, ..schema.clone()};
            assert_eq!(schema.validate(), Err(crate::schema::SchemaDeclError::ObjectReferenceInSave {field: 1}));
        }
        assert!(Value::Object(ObjectRef::UNBOUND).conforms(&FieldKind::Object, 1).is_err());
    }
}
