//! State schemas, logical values and records.
//!
//! A module declares each kind of state it owns as a [`StateSchema`]. The host
//! stores the values as [`Record`]s, takes snapshots of them for rollback and
//! hashes their canonical encoding. A module never writes a Rust layout, a
//! pointer, a `usize` or an entity bit pattern into a record.
//!
//! The field TAG is the identity of a field. The field NAME is a diagnostic: a
//! change to a name does not change the shape digest.

use crate::digest::Digest;
use crate::Name;

/// The identity of a schema: provider, local key and revision.
///
/// Two modules can both declare a local `state` key because the provider is
/// part of the key. A new revision must not reuse the shape digest of an old
/// one; the host refuses that.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SchemaKey {
    pub provider: Name,
    pub key: Name,
    pub revision: u32,
}

impl SchemaKey {
    pub const fn new(provider: &'static str, key: &'static str, revision: u32) -> Self {
        Self {
            provider: Name::Borrowed(provider),
            key: Name::Borrowed(key),
            revision,
        }
    }

    pub(crate) fn digest_into(&self, d: &mut Digest) {
        d.str(&self.provider).str(&self.key).u32(self.revision);
    }
}

impl SchemaKey {
    pub fn put(&self, out: &mut Vec<u8>) {
        crate::wire::put_str(out, &self.provider);
        crate::wire::put_str(out, &self.key);
        crate::wire::put_u32(out, self.revision);
    }

    pub fn read(r: &mut crate::wire::Reader<'_>) -> Result<Self, crate::wire::WireError> {
        Ok(Self {
            provider: r.str()?.to_owned().into(),
            key: r.str()?.to_owned().into(),
            revision: r.u32()?,
        })
    }
}

impl std::fmt::Display for SchemaKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}::{}@{}", self.provider, self.key, self.revision)
    }
}

/// Where the records of a schema live, and when they retire.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Attachment {
    /// One record for each body that an entry is invoked for. The record
    /// retires with the body.
    Body,
    /// One record for each gameplay session. It retires with the session.
    Session,
}

/// Which save road may keep the records. Rollback participation is NOT a
/// choice: all state that can change a later tick takes part in rollback.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SaveEligibility {
    /// The records exist only in the live simulation.
    Transient,
    /// A checkpoint keeps the records.
    Checkpoint,
    /// The durable world save keeps the records.
    Durable,
}

/// The type of one field.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum FieldKind {
    Bool,
    U32,
    I32,
    U64,
    /// A finite `f32`. NaN and infinity are refused.
    F32,
    /// Two finite `f32`, in world units.
    Vec2,
    Opt(Box<FieldKind>),
    /// A sequence with a declared maximum length. Order is meaning.
    Seq { elem: Box<FieldKind>, max_len: u32 },
}

impl FieldKind {
    pub fn opt(inner: FieldKind) -> Self {
        Self::Opt(Box::new(inner))
    }

    pub fn seq(elem: FieldKind, max_len: u32) -> Self {
        Self::Seq {
            elem: Box::new(elem),
            max_len,
        }
    }

    /// The deterministic initial value of a field of this kind.
    pub fn initial(&self) -> Value {
        match self {
            Self::Bool => Value::Bool(false),
            Self::U32 => Value::U32(0),
            Self::I32 => Value::I32(0),
            Self::U64 => Value::U64(0),
            Self::F32 => Value::F32(0.0),
            Self::Vec2 => Value::Vec2([0.0, 0.0]),
            Self::Opt(_) => Value::Opt(None),
            Self::Seq { .. } => Value::Seq(Vec::new()),
        }
    }

    /// The largest canonical encoding of a value of this kind, in bytes.
    pub fn max_encoded_len(&self) -> u64 {
        match self {
            Self::Bool => 1,
            Self::U32 | Self::I32 | Self::F32 => 4,
            Self::U64 | Self::Vec2 => 8,
            Self::Opt(inner) => 1 + inner.max_encoded_len(),
            Self::Seq { elem, max_len } => 4 + u64::from(*max_len) * elem.max_encoded_len(),
        }
    }

    /// The kind on the wire: the same codes as the digest.
    pub fn put(&self, out: &mut Vec<u8>) {
        use crate::wire::{put_u32, put_u8};
        match self {
            Self::Bool => put_u8(out, 1),
            Self::U32 => put_u8(out, 2),
            Self::I32 => put_u8(out, 3),
            Self::U64 => put_u8(out, 4),
            Self::F32 => put_u8(out, 5),
            Self::Vec2 => put_u8(out, 6),
            Self::Opt(inner) => {
                put_u8(out, 7);
                inner.put(out);
            }
            Self::Seq { elem, max_len } => {
                put_u8(out, 8);
                put_u32(out, *max_len);
                elem.put(out);
            }
        }
    }

    pub fn read(r: &mut crate::wire::Reader<'_>) -> Result<Self, crate::wire::WireError> {
        Ok(match r.u8()? {
            1 => Self::Bool,
            2 => Self::U32,
            3 => Self::I32,
            4 => Self::U64,
            5 => Self::F32,
            6 => Self::Vec2,
            7 => Self::opt(Self::read(r)?),
            8 => {
                let max_len = r.u32()?;
                Self::seq(Self::read(r)?, max_len)
            }
            other => return Err(crate::wire::WireError::BadTag(other)),
        })
    }

    fn digest_into(&self, d: &mut Digest) {
        match self {
            Self::Bool => d.u8(1),
            Self::U32 => d.u8(2),
            Self::I32 => d.u8(3),
            Self::U64 => d.u8(4),
            Self::F32 => d.u8(5),
            Self::Vec2 => d.u8(6),
            Self::Opt(inner) => {
                d.u8(7);
                inner.digest_into(d);
                d
            }
            Self::Seq { elem, max_len } => {
                d.u8(8).u32(*max_len);
                elem.digest_into(d);
                d
            }
        };
    }
}

/// A logical value. The canonical encoding of a value depends only on the
/// value, never on how it is stored.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Bool(bool),
    U32(u32),
    I32(i32),
    U64(u64),
    F32(f32),
    Vec2([f32; 2]),
    Opt(Option<Box<Value>>),
    Seq(Vec<Value>),
}

/// Why a value or record does not match its schema.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SchemaError {
    /// The value has a different kind than the field declares.
    KindMismatch { field: u16, expected: String },
    /// A float is NaN or infinite.
    NotFinite { field: u16 },
    /// A sequence is longer than its declared maximum.
    TooLong { field: u16, len: usize, max_len: u32 },
    /// The record has a different number of fields than the schema.
    FieldCount { expected: usize, found: usize },
    /// No field has this index.
    NoSuchField { index: usize },
}

impl std::fmt::Display for SchemaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::KindMismatch { field, expected } => {
                write!(f, "field tag {field}: the value is not a {expected}")
            }
            Self::NotFinite { field } => write!(f, "field tag {field}: the float is not finite"),
            Self::TooLong { field, len, max_len } => {
                write!(f, "field tag {field}: length {len} is more than {max_len}")
            }
            Self::FieldCount { expected, found } => {
                write!(f, "the record has {found} fields; the schema declares {expected}")
            }
            Self::NoSuchField { index } => write!(f, "no field has index {index}"),
        }
    }
}

impl Value {
    /// Checks that this value is a valid value of `kind`.
    pub fn conforms(&self, kind: &FieldKind, field: u16) -> Result<(), SchemaError> {
        let mismatch = || SchemaError::KindMismatch {
            field,
            expected: format!("{kind:?}"),
        };
        match (self, kind) {
            (Value::Bool(_), FieldKind::Bool)
            | (Value::U32(_), FieldKind::U32)
            | (Value::I32(_), FieldKind::I32)
            | (Value::U64(_), FieldKind::U64) => Ok(()),
            (Value::F32(v), FieldKind::F32) => finite(*v, field),
            (Value::Vec2([x, y]), FieldKind::Vec2) => {
                finite(*x, field)?;
                finite(*y, field)
            }
            (Value::Opt(None), FieldKind::Opt(_)) => Ok(()),
            (Value::Opt(Some(inner)), FieldKind::Opt(kind)) => inner.conforms(kind, field),
            (Value::Seq(items), FieldKind::Seq { elem, max_len }) => {
                if items.len() > *max_len as usize {
                    return Err(SchemaError::TooLong {
                        field,
                        len: items.len(),
                        max_len: *max_len,
                    });
                }
                items.iter().try_for_each(|item| item.conforms(elem, field))
            }
            _ => Err(mismatch()),
        }
    }

    /// Appends the canonical encoding. The schema gives the kind, so the
    /// encoding carries no type tags. Integers are little-endian. A float is
    /// its IEEE bit pattern, with `-0.0` written as `+0.0`.
    pub fn encode(&self, out: &mut Vec<u8>) {
        match self {
            Value::Bool(v) => out.push(u8::from(*v)),
            Value::U32(v) => out.extend_from_slice(&v.to_le_bytes()),
            Value::I32(v) => out.extend_from_slice(&v.to_le_bytes()),
            Value::U64(v) => out.extend_from_slice(&v.to_le_bytes()),
            Value::F32(v) => out.extend_from_slice(&canonical_f32(*v).to_le_bytes()),
            Value::Vec2([x, y]) => {
                out.extend_from_slice(&canonical_f32(*x).to_le_bytes());
                out.extend_from_slice(&canonical_f32(*y).to_le_bytes());
            }
            Value::Opt(None) => out.push(0),
            Value::Opt(Some(inner)) => {
                out.push(1);
                inner.encode(out);
            }
            Value::Seq(items) => {
                out.extend_from_slice(&(items.len() as u32).to_le_bytes());
                for item in items {
                    item.encode(out);
                }
            }
        }
    }

    /// The exact wire encoding: bit patterns kept as they are.
    pub fn put_wire(&self, out: &mut Vec<u8>) {
        use crate::wire::*;
        match self {
            Value::Bool(v) => put_bool(out, *v),
            Value::U32(v) => put_u32(out, *v),
            Value::I32(v) => put_i32(out, *v),
            Value::U64(v) => put_u64(out, *v),
            Value::F32(v) => put_f32(out, *v),
            Value::Vec2(v) => put_vec2(out, *v),
            Value::Opt(None) => put_u8(out, 0),
            Value::Opt(Some(inner)) => {
                put_u8(out, 1);
                inner.put_wire(out);
            }
            Value::Seq(items) => {
                put_u32(out, items.len() as u32);
                for item in items {
                    item.put_wire(out);
                }
            }
        }
    }

    /// Read a value of `kind`. Bounds are the schema check's job, not the
    /// reader's; the reader only refuses lengths past [`crate::wire::MAX_LEN`].
    pub fn read_wire(
        kind: &FieldKind,
        r: &mut crate::wire::Reader<'_>,
    ) -> Result<Self, crate::wire::WireError> {
        Ok(match kind {
            FieldKind::Bool => Value::Bool(r.bool()?),
            FieldKind::U32 => Value::U32(r.u32()?),
            FieldKind::I32 => Value::I32(r.i32()?),
            FieldKind::U64 => Value::U64(r.u64()?),
            FieldKind::F32 => Value::F32(r.f32()?),
            FieldKind::Vec2 => Value::Vec2(r.vec2()?),
            FieldKind::Opt(inner) => Value::Opt(r.opt(|r| Self::read_wire(inner, r).map(Box::new))?),
            FieldKind::Seq { elem, .. } => {
                let n = r.read_len()?;
                Value::Seq((0..n).map(|_| Self::read_wire(elem, r)).collect::<Result<_, _>>()?)
            }
        })
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_u32(&self) -> Option<u32> {
        match self {
            Value::U32(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Value::U64(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_f32(&self) -> Option<f32> {
        match self {
            Value::F32(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_vec2(&self) -> Option<[f32; 2]> {
        match self {
            Value::Vec2(v) => Some(*v),
            _ => None,
        }
    }

    /// `Some(None)` for an empty option, `None` when this is not an option.
    pub fn as_opt(&self) -> Option<Option<&Value>> {
        match self {
            Value::Opt(v) => Some(v.as_deref()),
            _ => None,
        }
    }

    pub fn some(inner: Value) -> Self {
        Value::Opt(Some(Box::new(inner)))
    }
}

fn finite(v: f32, field: u16) -> Result<(), SchemaError> {
    if v.is_finite() {
        Ok(())
    } else {
        Err(SchemaError::NotFinite { field })
    }
}

fn canonical_f32(v: f32) -> u32 {
    if v == 0.0 {
        0
    } else {
        v.to_bits()
    }
}

/// One declared field.
#[derive(Clone, Debug, PartialEq)]
pub struct FieldDecl {
    /// The identity of the field inside its schema.
    pub tag: u16,
    /// A diagnostic name. It is not part of the shape digest.
    pub name: Name,
    pub kind: FieldKind,
    /// The deterministic initial value.
    pub initial: Value,
}

impl FieldDecl {
    /// A field whose initial value is the kind's default.
    pub fn new(tag: u16, name: &'static str, kind: FieldKind) -> Self {
        let initial = kind.initial();
        Self {
            tag,
            name: Name::Borrowed(name),
            kind,
            initial,
        }
    }

    pub fn with_initial(mut self, initial: Value) -> Self {
        self.initial = initial;
        self
    }
}

/// The index of a field in its schema's field list. A module declares one
/// constant per field next to its schema.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FieldRef(pub usize);

/// A complete schema declaration.
#[derive(Clone, Debug, PartialEq)]
pub struct StateSchema {
    pub key: SchemaKey,
    pub attachment: Attachment,
    pub save: SaveEligibility,
    pub fields: Vec<FieldDecl>,
}

/// Why a schema declaration is not valid.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SchemaDeclError {
    DuplicateTag { tag: u16 },
    BadInitial(SchemaError),
    Empty,
}

impl StateSchema {
    /// Checks tag uniqueness and the initial values.
    pub fn validate(&self) -> Result<(), SchemaDeclError> {
        if self.fields.is_empty() {
            return Err(SchemaDeclError::Empty);
        }
        let mut tags: Vec<u16> = self.fields.iter().map(|f| f.tag).collect();
        tags.sort_unstable();
        if let Some(pair) = tags.windows(2).find(|w| w[0] == w[1]) {
            return Err(SchemaDeclError::DuplicateTag { tag: pair[0] });
        }
        for field in &self.fields {
            field
                .initial
                .conforms(&field.kind, field.tag)
                .map_err(SchemaDeclError::BadInitial)?;
        }
        Ok(())
    }

    /// The canonical shape digest: key, attachment, save policy, and each
    /// field's tag, kind and initial value. Field names do not take part.
    pub fn shape_digest(&self) -> u64 {
        let mut d = Digest::new();
        self.key.digest_into(&mut d);
        d.u8(match self.attachment {
            Attachment::Body => 1,
            Attachment::Session => 2,
        });
        d.u8(match self.save {
            SaveEligibility::Transient => 1,
            SaveEligibility::Checkpoint => 2,
            SaveEligibility::Durable => 3,
        });
        d.u32(self.fields.len() as u32);
        for field in &self.fields {
            d.u16(field.tag);
            field.kind.digest_into(&mut d);
            let mut initial = Vec::new();
            field.initial.encode(&mut initial);
            d.u64(initial.len() as u64).bytes(&initial);
        }
        d.finish()
    }

    /// A new record with every field at its initial value.
    pub fn initial_record(&self) -> Record {
        Record {
            values: self.fields.iter().map(|f| f.initial.clone()).collect(),
        }
    }

    /// Checks that a record is a valid value of this schema.
    pub fn check(&self, record: &Record) -> Result<(), SchemaError> {
        if record.values.len() != self.fields.len() {
            return Err(SchemaError::FieldCount {
                expected: self.fields.len(),
                found: record.values.len(),
            });
        }
        for (value, field) in record.values.iter().zip(&self.fields) {
            value.conforms(&field.kind, field.tag)?;
        }
        Ok(())
    }

    pub fn put(&self, out: &mut Vec<u8>) {
        use crate::wire::*;
        self.key.put(out);
        put_u8(
            out,
            match self.attachment {
                Attachment::Body => 1,
                Attachment::Session => 2,
            },
        );
        put_u8(
            out,
            match self.save {
                SaveEligibility::Transient => 1,
                SaveEligibility::Checkpoint => 2,
                SaveEligibility::Durable => 3,
            },
        );
        put_u32(out, self.fields.len() as u32);
        for field in &self.fields {
            put_u16(out, field.tag);
            put_str(out, &field.name);
            field.kind.put(out);
            field.initial.put_wire(out);
        }
    }

    pub fn read(r: &mut crate::wire::Reader<'_>) -> Result<Self, crate::wire::WireError> {
        use crate::wire::WireError;
        let key = SchemaKey::read(r)?;
        let attachment = match r.u8()? {
            1 => Attachment::Body,
            2 => Attachment::Session,
            other => return Err(WireError::BadTag(other)),
        };
        let save = match r.u8()? {
            1 => SaveEligibility::Transient,
            2 => SaveEligibility::Checkpoint,
            3 => SaveEligibility::Durable,
            other => return Err(WireError::BadTag(other)),
        };
        let n = r.read_len()?;
        let mut fields = Vec::with_capacity(n.min(256));
        for _ in 0..n {
            let tag = r.u16()?;
            let name: Name = r.str()?.to_owned().into();
            let kind = FieldKind::read(r)?;
            let initial = Value::read_wire(&kind, r)?;
            fields.push(FieldDecl {
                tag,
                name,
                kind,
                initial,
            });
        }
        Ok(Self {
            key,
            attachment,
            save,
            fields,
        })
    }

    /// A record of this schema on the wire.
    pub fn read_record(&self, r: &mut crate::wire::Reader<'_>) -> Result<Record, crate::wire::WireError> {
        Ok(Record {
            values: self
                .fields
                .iter()
                .map(|f| Value::read_wire(&f.kind, r))
                .collect::<Result<_, _>>()?,
        })
    }

    /// The largest canonical encoding of one record.
    pub fn max_encoded_len(&self) -> u64 {
        self.fields.iter().map(|f| f.kind.max_encoded_len()).sum()
    }
}

/// The values of one record, in the order of its schema's fields.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Record {
    values: Vec<Value>,
}

impl Record {
    pub fn get(&self, field: FieldRef) -> Result<&Value, SchemaError> {
        self.values
            .get(field.0)
            .ok_or(SchemaError::NoSuchField { index: field.0 })
    }

    /// Replaces one value. The host checks the record against its schema
    /// before it commits the write.
    pub fn set(&mut self, field: FieldRef, value: Value) -> Result<(), SchemaError> {
        let slot = self
            .values
            .get_mut(field.0)
            .ok_or(SchemaError::NoSuchField { index: field.0 })?;
        *slot = value;
        Ok(())
    }

    pub fn values(&self) -> &[Value] {
        &self.values
    }

    /// The exact wire encoding of the record (its schema gives the kinds).
    pub fn put_wire(&self, out: &mut Vec<u8>) {
        for value in &self.values {
            value.put_wire(out);
        }
    }

    /// The canonical encoding of the record.
    pub fn encode(&self, out: &mut Vec<u8>) {
        for value in &self.values {
            value.encode(out);
        }
    }

    /// The canonical checksum of the record under its schema's shape.
    pub fn checksum(&self, shape_digest: u64) -> u64 {
        let mut bytes = Vec::new();
        self.encode(&mut bytes);
        Digest::new().u64(shape_digest).bytes(&bytes).finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn latch() -> StateSchema {
        StateSchema {
            key: SchemaKey::new("test", "latch", 1),
            attachment: Attachment::Body,
            save: SaveEligibility::Transient,
            fields: vec![
                FieldDecl::new(1, "last_tick", FieldKind::opt(FieldKind::U64)),
                FieldDecl::new(2, "aims", FieldKind::seq(FieldKind::Vec2, 2)),
            ],
        }
    }

    #[test]
    fn a_field_name_is_a_diagnostic_and_a_tag_is_an_identity() {
        let base = latch();
        let mut renamed = latch();
        renamed.fields[0].name = Name::Borrowed("previous_tick");
        assert_eq!(base.shape_digest(), renamed.shape_digest());

        let mut retagged = latch();
        retagged.fields[0].tag = 9;
        assert_ne!(base.shape_digest(), retagged.shape_digest());

        let mut longer = latch();
        longer.fields[1].kind = FieldKind::seq(FieldKind::Vec2, 3);
        assert_ne!(base.shape_digest(), longer.shape_digest());

        let mut revised = latch();
        revised.key.revision = 2;
        assert_ne!(base.shape_digest(), revised.shape_digest());
    }

    #[test]
    fn a_record_refuses_a_nan_an_overlong_sequence_and_a_wrong_kind() {
        let schema = latch();
        let mut record = schema.initial_record();
        assert_eq!(schema.check(&record), Ok(()));

        record
            .set(FieldRef(1), Value::Seq(vec![Value::Vec2([f32::NAN, 0.0])]))
            .unwrap();
        assert_eq!(schema.check(&record), Err(SchemaError::NotFinite { field: 2 }));

        record
            .set(FieldRef(1), Value::Seq(vec![Value::Vec2([0.0, 0.0]); 3]))
            .unwrap();
        assert!(matches!(schema.check(&record), Err(SchemaError::TooLong { .. })));

        record.set(FieldRef(1), Value::Seq(Vec::new())).unwrap();
        record.set(FieldRef(0), Value::U32(4)).unwrap();
        assert!(matches!(
            schema.check(&record),
            Err(SchemaError::KindMismatch { field: 1, .. })
        ));
    }

    #[test]
    fn the_checksum_sees_every_field_and_ignores_the_sign_of_zero() {
        let schema = latch();
        let shape = schema.shape_digest();
        let a = schema.initial_record();
        let mut b = schema.initial_record();
        b.set(FieldRef(0), Value::some(Value::U64(0))).unwrap();
        assert_ne!(a.checksum(shape), b.checksum(shape));

        let mut c = schema.initial_record();
        c.set(FieldRef(1), Value::Seq(vec![Value::Vec2([-0.0, 1.0])]))
            .unwrap();
        let mut d = schema.initial_record();
        d.set(FieldRef(1), Value::Seq(vec![Value::Vec2([0.0, 1.0])]))
            .unwrap();
        assert_eq!(c.checksum(shape), d.checksum(shape));
    }

    #[test]
    fn a_duplicate_tag_is_refused() {
        let mut schema = latch();
        schema.fields[1].tag = 1;
        assert_eq!(
            schema.validate(),
            Err(SchemaDeclError::DuplicateTag { tag: 1 })
        );
    }
}
