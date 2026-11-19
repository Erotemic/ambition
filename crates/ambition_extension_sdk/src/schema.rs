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

    pub fn read(r: &mut crate::wire::WireReader<'_>) -> Result<Self, crate::wire::WireError> {
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
    /// One record for each scope an entry is invoked for: the body, or the
    /// room's root for a group (`group_think`). The record retires with its
    /// scope.
    Body,
    /// One record for each gameplay session, shared by every invocation of
    /// the module in the session, whatever body it runs for. It retires with
    /// the session.
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
#[derive(Clone, Debug, PartialEq)]
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
    /// An authored body or item name, resolved in the consumer's scope.
    /// This locator does not identify a live occurrence. It
    /// names something (it is not empty) in at most [`MAX_REF_LEN`] bytes; a
    /// reference that may be absent is an `Opt`. What it names may be gone:
    /// the reader resolves it.
    Ref(RefKind),
    /// UTF-8 text with a declared maximum byte length.
    Text { max_bytes: u32 },
    /// A value with its own tagged fields. The outer schema owns its lifetime.
    Record(Vec<FieldDecl>),
    /// A live occurrence address. Only transient records can retain it.
    Object,
}

/// What a [`FieldKind::Ref`] names. A body and an item are distinct kinds,
/// so a record cannot take one for the other.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RefKind {
    /// An authored body name (`SimId`).
    Body,
    /// An authored item name (`SimId`).
    Item,
}

impl RefKind {
    fn code(self) -> u8 {
        match self {
            Self::Body => 1,
            Self::Item => 2,
        }
    }
}

/// The longest reference, in bytes.
pub const MAX_REF_LEN: u32 = 128;

/// The largest number of nested field kinds in a schema.
pub const MAX_SCHEMA_DEPTH: usize = 32;

impl FieldKind {
    fn contains_object(&self) -> bool {
        match self {
            Self::Object => true,
            Self::Opt(inner) | Self::Seq { elem: inner, .. } => inner.contains_object(),
            Self::Record(fields) => fields.iter().any(|field| field.kind.contains_object()),
            _ => false,
        }
    }

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
            Self::Object => Value::Object(crate::ObjectRef::UNBOUND),
            Self::Bool => Value::Bool(false),
            Self::U32 => Value::U32(0),
            Self::I32 => Value::I32(0),
            Self::U64 => Value::U64(0),
            Self::F32 => Value::F32(0.0),
            Self::Vec2 => Value::Vec2([0.0, 0.0]),
            Self::Opt(_) => Value::Opt(None),
            Self::Seq { .. } => Value::Seq(Vec::new()),
            // Not a valid value: a bare reference field states its initial.
            Self::Ref(_) => Value::Ref(String::new()),
            Self::Text { .. } => Value::Text(String::new()),
            Self::Record(fields) => Value::Record(initial_fields(fields)),
        }
    }

    /// The largest canonical encoding of a value of this kind, in bytes.
    pub fn max_encoded_len(&self) -> u64 {
        match self {
            Self::Object => 21,
            Self::Bool => 1,
            Self::U32 | Self::I32 | Self::F32 => 4,
            Self::U64 | Self::Vec2 => 8,
            Self::Opt(inner) => 1u64.saturating_add(inner.max_encoded_len()),
            Self::Seq { elem, max_len } => 4u64.saturating_add(u64::from(*max_len).saturating_mul(elem.max_encoded_len())),
            Self::Ref(_) => 4 + u64::from(MAX_REF_LEN),
            Self::Text { max_bytes } => 4 + u64::from(*max_bytes),
            Self::Record(fields) => fields_max_encoded_len(fields),
        }
    }

    /// The kind on the wire: the same codes as the digest.
    pub fn put(&self, out: &mut Vec<u8>) {
        use crate::wire::{put_u32, put_u8};
        match self {
            Self::Object => put_u8(out, 12),
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
            Self::Ref(kind) => {
                put_u8(out, 9);
                put_u8(out, kind.code());
            }
            Self::Text { max_bytes } => {
                put_u8(out, 10);
                put_u32(out, *max_bytes);
            }
            Self::Record(fields) => {
                put_u8(out, 11);
                put_fields(fields, out);
            }
        }
    }

    pub fn read(r: &mut crate::wire::WireReader<'_>) -> Result<Self, crate::wire::WireError> {
        Self::read_at(r, 0)
    }

    fn read_at(r: &mut crate::wire::WireReader<'_>, depth: usize) -> Result<Self, crate::wire::WireError> {
        if depth >= MAX_SCHEMA_DEPTH {
            return Err(crate::wire::WireError::TooDeep);
        }
        Ok(match r.u8()? {
            1 => Self::Bool,
            2 => Self::U32,
            3 => Self::I32,
            4 => Self::U64,
            5 => Self::F32,
            6 => Self::Vec2,
            7 => Self::opt(Self::read_at(r, depth + 1)?),
            8 => {
                let max_len = r.u32()?;
                Self::seq(Self::read_at(r, depth + 1)?, max_len)
            }
            9 => Self::Ref(match r.u8()? {
                1 => RefKind::Body,
                2 => RefKind::Item,
                other => return Err(crate::wire::WireError::BadTag(other)),
            }),
            10 => Self::Text { max_bytes: r.u32()? },
            11 => Self::Record(read_fields(r, depth + 1)?),
            12 => Self::Object,
            other => return Err(crate::wire::WireError::BadTag(other)),
        })
    }

    fn digest_into(&self, d: &mut Digest) {
        match self {
            Self::Object => d.u8(12),
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
            Self::Ref(kind) => d.u8(9).u8(kind.code()),
            Self::Text { max_bytes } => d.u8(10).u32(*max_bytes),
            Self::Record(fields) => {
                d.u8(11);
                digest_fields(fields, d);
                d
            }
        };
    }

    /// A new kind can keep all values of the old kind. Bounds can grow.
    pub fn accepts_migration_from(&self, from: &Self) -> bool {
        match (self, from) {
            (Self::Text { max_bytes: to }, Self::Text { max_bytes: from }) => to >= from,
            (Self::Opt(to), Self::Opt(from)) => to.accepts_migration_from(from),
            (Self::Seq { elem: to, max_len }, Self::Seq { elem: from, max_len: old_max }) => {
                max_len >= old_max && to.accepts_migration_from(from)
            }
            (Self::Record(to), Self::Record(from)) => fields_accept_migration(to, from),
            _ => self == from,
        }
    }

    fn migrate_value(&self, from: &Self, value: &Value) -> Option<Value> {
        if !self.accepts_migration_from(from) || value.conforms(from, 0).is_err() {
            return None;
        }
        Some(match (self, from, value) {
            (Self::Record(to), Self::Record(from), Value::Record(record)) => {
                Value::Record(migrate_fields(to, from, record))
            }
            (Self::Opt(to), Self::Opt(from), Value::Opt(Some(value))) => {
                Value::some(to.migrate_value(from, value)?)
            }
            (Self::Seq { elem: to, .. }, Self::Seq { elem: from, .. }, Value::Seq(values)) => {
                Value::Seq(values.iter().map(|v| to.migrate_value(from, v)).collect::<Option<_>>()?)
            }
            _ => value.clone(),
        })
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
    /// A simulation identity ([`FieldKind::Ref`]).
    Ref(String),
    Text(String),
    Record(Record),
    Object(crate::ObjectRef),
}

/// Why a value or record does not match its schema.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SchemaError {
    /// The value has a different kind than the field declares.
    KindMismatch { field: u16, expected: String },
    /// A float is NaN or infinite.
    NotFinite { field: u16 },
    UnboundObjectReference { field: u16 },
    /// A sequence or text value is longer than its declared maximum.
    TooLong { field: u16, len: usize, max_len: u32 },
    /// The record has a different number of fields than the schema.
    FieldCount { expected: usize, found: usize },
    /// No field has this index.
    NoSuchField { index: usize },
    /// A reference is empty or longer than [`MAX_REF_LEN`].
    BadReference { field: u16, len: usize },
    Nested { field: u16, error: Box<SchemaError> },
}

impl std::fmt::Display for SchemaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::KindMismatch { field, expected } => {
                write!(f, "field tag {field}: the value is not a {expected}")
            }
            Self::UnboundObjectReference { field } => write!(f, "field tag {field}: the object reference is unbound; use an optional object field"),
            Self::NotFinite { field } => write!(f, "field tag {field}: the float is not finite"),
            Self::TooLong { field, len, max_len } => {
                write!(f, "field tag {field}: length {len} is more than {max_len}")
            }
            Self::FieldCount { expected, found } => {
                write!(f, "the record has {found} fields; the schema declares {expected}")
            }
            Self::NoSuchField { index } => write!(f, "no field has index {index}"),
            Self::Nested { field, error } => write!(f, "field tag {field}: {error}"),
            Self::BadReference { field, len } => {
                write!(f, "field tag {field}: a reference of {len} bytes names nothing (1..={MAX_REF_LEN})")
            }
        }
    }
}

impl Value {
    pub fn as_object(&self) -> Option<crate::ObjectRef> {
        match self { Self::Object(value) if value.is_bound() => Some(*value), _ => None }
    }

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
            (Value::Object(value), FieldKind::Object) => {
                if value.is_bound() { Ok(()) } else { Err(SchemaError::UnboundObjectReference {field}) }
            },
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
            (Value::Ref(id), FieldKind::Ref(_)) => {
                if id.is_empty() || id.len() > MAX_REF_LEN as usize {
                    return Err(SchemaError::BadReference { field, len: id.len() });
                }
                Ok(())
            }
            (Value::Text(text), FieldKind::Text { max_bytes }) => {
                if text.len() > *max_bytes as usize {
                    return Err(SchemaError::TooLong { field, len: text.len(), max_len: *max_bytes });
                }
                Ok(())
            }
            (Value::Record(record), FieldKind::Record(fields)) => check_fields(fields, record)
                .map_err(|error| SchemaError::Nested { field, error: Box::new(error) }),
            _ => Err(mismatch()),
        }
    }

    /// Appends the canonical encoding. The schema gives the kind, so the
    /// encoding carries no type tags. Integers are little-endian. A float is
    /// its IEEE bit pattern, with `-0.0` written as `+0.0`.
    pub fn encode(&self, out: &mut Vec<u8>) {
        match self {
            Value::Object(v) => crate::structure::put(v, out),
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
            Value::Ref(id) | Value::Text(id) => {
                out.extend_from_slice(&(id.len() as u32).to_le_bytes());
                out.extend_from_slice(id.as_bytes());
            }
            Value::Record(record) => record.encode(out),
        }
    }

    /// The exact wire encoding: bit patterns kept as they are.
    pub fn put_wire(&self, out: &mut Vec<u8>) {
        use crate::wire::*;
        match self {
            Value::Object(v) => crate::structure::put(v, out),
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
            Value::Ref(id) | Value::Text(id) => put_str(out, id),
            Value::Record(record) => record.put_wire(out),
        }
    }

    /// Read a value of `kind`. Refuse collection bounds before allocation.
    /// The schema check also validates references and finite numbers.
    pub fn read_wire(
        kind: &FieldKind,
        r: &mut crate::wire::WireReader<'_>,
    ) -> Result<Self, crate::wire::WireError> {
        Ok(match kind {
            FieldKind::Object => Value::Object(crate::structure::read(r)?),
            FieldKind::Bool => Value::Bool(r.bool()?),
            FieldKind::U32 => Value::U32(r.u32()?),
            FieldKind::I32 => Value::I32(r.i32()?),
            FieldKind::U64 => Value::U64(r.u64()?),
            FieldKind::F32 => Value::F32(r.f32()?),
            FieldKind::Vec2 => Value::Vec2(r.vec2()?),
            FieldKind::Opt(inner) => Value::Opt(r.opt(|r| Self::read_wire(inner, r).map(Box::new))?),
            FieldKind::Seq { elem, max_len } => {
                let n = r.read_len()?;
                if n > *max_len as usize { return Err(crate::wire::WireError::TooLong(n as u64)); }
                Value::Seq((0..n).map(|_| Self::read_wire(elem, r)).collect::<Result<_, _>>()?)
            }
            FieldKind::Ref(_) => Value::Ref(r.str()?.to_owned()),
            FieldKind::Text { max_bytes } => {
                let text = r.str()?;
                if text.len() > *max_bytes as usize { return Err(crate::wire::WireError::TooLong(text.len() as u64)); }
                Value::Text(text.to_owned())
            }
            FieldKind::Record(fields) => Value::Record(read_record_fields(fields, r)?),
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

    /// The identity a reference names.
    pub fn as_ref_id(&self) -> Option<&str> {
        match self {
            Value::Ref(id) => Some(id),
            _ => None,
        }
    }

    pub fn some(inner: Value) -> Self {
        Value::Opt(Some(Box::new(inner)))
    }

    pub fn as_text(&self) -> Option<&str> {
        match self { Self::Text(text) => Some(text), _ => None }
    }

    pub fn as_record(&self) -> Option<&Record> {
        match self { Self::Record(record) => Some(record), _ => None }
    }

    pub fn as_record_mut(&mut self) -> Option<&mut Record> {
        match self { Self::Record(record) => Some(record), _ => None }
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
    DuplicateName { name: String },
    BadInitial(SchemaError),
    ObjectReferenceInSave { field: u16 },
    Empty,
    TooDeep,
}

impl StateSchema {
    /// Checks field identities, nesting and initial values.
    pub fn validate(&self) -> Result<(), SchemaDeclError> {
        validate_fields(&self.fields, 0)?;
        if self.save != SaveEligibility::Transient {
            for field in &self.fields {
                if field.kind.contains_object() {
                    return Err(SchemaDeclError::ObjectReferenceInSave { field: field.tag });
                }
            }
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
        digest_fields(&self.fields, &mut d);
        d.finish()
    }

    /// A new record with every field at its initial value.
    pub fn initial_record(&self) -> Record {
        initial_fields(&self.fields)
    }

    /// `record`, written under `from`, as a record of this schema: a field
    /// keeps the value of the field with its TAG in `from` when the kinds are
    /// compatible, and takes its initial value otherwise. This rule applies
    /// to nested records too. A name is not an identity.
    pub fn migrate(&self, from: &StateSchema, record: &Record) -> Record {
        migrate_fields(&self.fields, &from.fields, record)
    }

    /// Whether all retained fields can keep their old values.
    pub fn accepts_migration_from(&self, from: &StateSchema) -> bool {
        self.key == from.key && self.attachment == from.attachment && self.save == from.save
            && fields_accept_migration(&self.fields, &from.fields)
    }

    /// Checks that a record is a valid value of this schema.
    pub fn check(&self, record: &Record) -> Result<(), SchemaError> {
        check_fields(&self.fields, record)
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
        put_fields(&self.fields, out);
    }

    pub fn read(r: &mut crate::wire::WireReader<'_>) -> Result<Self, crate::wire::WireError> {
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
        let fields = read_fields(r, 0)?;
        Ok(Self {
            key,
            attachment,
            save,
            fields,
        })
    }

    /// A record of this schema on the wire.
    pub fn read_record(&self, r: &mut crate::wire::WireReader<'_>) -> Result<Record, crate::wire::WireError> {
        read_record_fields(&self.fields, r)
    }

    /// The largest canonical encoding of one record.
    pub fn max_encoded_len(&self) -> u64 {
        fields_max_encoded_len(&self.fields)
    }
}

fn validate_kind(kind: &FieldKind, depth: usize) -> Result<(), SchemaDeclError> {
    if depth >= MAX_SCHEMA_DEPTH {
        return Err(SchemaDeclError::TooDeep);
    }
    match kind {
        FieldKind::Opt(inner) | FieldKind::Seq { elem: inner, .. } => validate_kind(inner, depth + 1),
        FieldKind::Record(fields) => validate_fields(fields, depth + 1),
        _ => Ok(()),
    }
}

fn validate_fields(fields: &[FieldDecl], depth: usize) -> Result<(), SchemaDeclError> {
    if fields.is_empty() {
        return Err(SchemaDeclError::Empty);
    }
    let mut tags = std::collections::BTreeSet::new();
    let mut names = std::collections::BTreeSet::new();
    for field in fields {
        if !tags.insert(field.tag) {
            return Err(SchemaDeclError::DuplicateTag { tag: field.tag });
        }
        if !names.insert(field.name.as_ref()) {
            return Err(SchemaDeclError::DuplicateName { name: field.name.to_string() });
        }
        validate_kind(&field.kind, depth)?;
        field.initial.conforms(&field.kind, field.tag).map_err(SchemaDeclError::BadInitial)?;
    }
    Ok(())
}

fn initial_fields(fields: &[FieldDecl]) -> Record {
    Record::from_values(fields.iter().map(|f| f.initial.clone()).collect())
}

fn check_fields(fields: &[FieldDecl], record: &Record) -> Result<(), SchemaError> {
    if record.values.len() != fields.len() {
        return Err(SchemaError::FieldCount { expected: fields.len(), found: record.values.len() });
    }
    for (value, field) in record.values.iter().zip(fields) {
        value.conforms(&field.kind, field.tag)?;
    }
    Ok(())
}

fn fields_accept_migration(to: &[FieldDecl], from: &[FieldDecl]) -> bool {
    from.iter().all(|old| to.iter().find(|f| f.tag == old.tag)
        .is_none_or(|new| new.kind.accepts_migration_from(&old.kind)))
}

fn migrate_fields(to: &[FieldDecl], from: &[FieldDecl], record: &Record) -> Record {
    Record::from_values(to.iter().map(|field| {
        from.iter().position(|old| old.tag == field.tag)
            .and_then(|i| field.kind.migrate_value(&from[i].kind, record.values.get(i)?))
            .unwrap_or_else(|| field.initial.clone())
    }).collect())
}

fn fields_max_encoded_len(fields: &[FieldDecl]) -> u64 {
    fields.iter().fold(0u64, |n, f| n.saturating_add(f.kind.max_encoded_len()))
}

fn digest_fields(fields: &[FieldDecl], digest: &mut Digest) {
    digest.u32(fields.len() as u32);
    for field in fields {
        digest.u16(field.tag);
        field.kind.digest_into(digest);
        let mut initial = Vec::new();
        field.initial.encode(&mut initial);
        digest.u64(initial.len() as u64).bytes(&initial);
    }
}

fn put_fields(fields: &[FieldDecl], out: &mut Vec<u8>) {
    use crate::wire::*;
    put_u32(out, fields.len() as u32);
    for field in fields {
        put_u16(out, field.tag);
        put_str(out, &field.name);
        field.kind.put(out);
        field.initial.put_wire(out);
    }
}

fn read_fields(r: &mut crate::wire::WireReader<'_>, depth: usize) -> Result<Vec<FieldDecl>, crate::wire::WireError> {
    let n = r.read_len()?;
    let mut fields = Vec::with_capacity(n.min(256));
    for _ in 0..n {
        let tag = r.u16()?;
        let name = r.str()?.to_owned().into();
        let kind = FieldKind::read_at(r, depth)?;
        let initial = Value::read_wire(&kind, r)?;
        fields.push(FieldDecl { tag, name, kind, initial });
    }
    Ok(fields)
}

fn read_record_fields(fields: &[FieldDecl], r: &mut crate::wire::WireReader<'_>) -> Result<Record, crate::wire::WireError> {
    Ok(Record::from_values(fields.iter().map(|f| Value::read_wire(&f.kind, r)).collect::<Result<_, _>>()?))
}

/// The values of one record, in the order of its schema's fields.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Record {
    values: Vec<Value>,
}

impl Record {
    /// Construct values in schema order. The host checks them before commit.
    pub fn from_values(values: Vec<Value>) -> Self {
        Self { values }
    }

    pub fn get(&self, field: FieldRef) -> Result<&Value, SchemaError> {
        self.values
            .get(field.0)
            .ok_or(SchemaError::NoSuchField { index: field.0 })
    }

    pub fn get_mut(&mut self, field: FieldRef) -> Result<&mut Value, SchemaError> {
        self.values.get_mut(field.0).ok_or(SchemaError::NoSuchField { index: field.0 })
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

    fn nested_kind() -> FieldKind {
        FieldKind::seq(FieldKind::Record(vec![
            FieldDecl::new(1, "label", FieldKind::Text { max_bytes: 8 }),
            FieldDecl::new(2, "target", FieldKind::opt(FieldKind::Ref(RefKind::Body))),
            FieldDecl::new(3, "offset", FieldKind::Vec2),
        ]), 4)
    }

    #[test]
    fn nested_values_keep_wire_bits_and_canonical_checksums() {
        let kind = nested_kind();
        let value = Value::Seq(vec![Value::Record(Record::from_values(vec![
            Value::Text("écho".into()), Value::some(Value::Ref("placement:target".into())),
            Value::Vec2([-0.0, 2.0]),
        ]))]);
        assert_eq!(value.conforms(&kind, 1), Ok(()));
        let mut bytes = Vec::new();
        value.put_wire(&mut bytes);
        assert_eq!(crate::wire::decode_all(&bytes, |r| Value::read_wire(&kind, r)).unwrap(), value);
        let mut canonical = Vec::new();
        value.encode(&mut canonical);
        assert_ne!(bytes, canonical, "wire keeps the sign of zero");
        let plus_zero = crate::wire::decode_all(&canonical, |r| Value::read_wire(&kind, r)).unwrap();
        let mut again = Vec::new();
        plus_zero.encode(&mut again);
        assert_eq!(again, canonical);
        let mut encoded_kind = Vec::new();
        kind.put(&mut encoded_kind);
        assert_eq!(crate::wire::decode_all(&encoded_kind, FieldKind::read).unwrap(), kind);
        assert!(bytes.len() as u64 <= kind.max_encoded_len());
        assert!(Value::Text("ééé".into()).conforms(&FieldKind::Text { max_bytes: 5 }, 1).is_err(),
            "text bounds count UTF-8 bytes");
        assert!(Value::Ref(String::new()).conforms(&FieldKind::Ref(RefKind::Body), 1).is_err());
        assert!(Value::Text(String::new()).conforms(&FieldKind::Text { max_bytes: 0 }, 1).is_ok());
    }

    #[test]
    fn nested_migration_keeps_tags_and_refuses_narrower_contracts() {
        let old = FieldKind::opt(nested_kind());
        let new = FieldKind::opt(FieldKind::seq(FieldKind::Record(vec![
            FieldDecl::new(4, "new", FieldKind::Bool).with_initial(Value::Bool(true)),
            FieldDecl::new(2, "who", FieldKind::opt(FieldKind::Ref(RefKind::Body))),
            FieldDecl::new(1, "name", FieldKind::Text { max_bytes: 16 }),
        ]), 8));
        let value = Value::some(Value::Seq(vec![Value::Record(Record::from_values(vec![
            Value::Text("old".into()), Value::some(Value::Ref("slot:0/7".into())), Value::Vec2([2.0, 3.0]),
        ]))]));
        assert!(new.accepts_migration_from(&old));
        assert_eq!(new.migrate_value(&old, &value), Some(Value::some(Value::Seq(vec![
            Value::Record(Record::from_values(vec![Value::Bool(true),
                Value::some(Value::Ref("slot:0/7".into())), Value::Text("old".into())])),
        ]))));
        assert!(!old.accepts_migration_from(&new), "a smaller bound can lose saved values");
        assert!(!FieldKind::opt(FieldKind::U32).accepts_migration_from(&old));
    }

    #[test]
    fn empty_sequences_do_not_hide_invalid_nested_declarations() {
        let schema = |kind| StateSchema { key: SchemaKey::new("test", "nested", 1),
            attachment: Attachment::Body, save: SaveEligibility::Transient,
            fields: vec![FieldDecl::new(1, "entries", kind)] };
        let mut fields = vec![FieldDecl::new(1, "a", FieldKind::U32), FieldDecl::new(1, "b", FieldKind::Bool)];
        assert_eq!(schema(FieldKind::seq(FieldKind::Record(fields.clone()), 3)).validate(),
            Err(SchemaDeclError::DuplicateTag { tag: 1 }));
        fields[1].tag = 2;
        fields[1].name = "a".into();
        assert!(matches!(schema(FieldKind::seq(FieldKind::Record(fields), 3)).validate(),
            Err(SchemaDeclError::DuplicateName { .. })));
        let mut kind = FieldKind::U32;
        for _ in 0..MAX_SCHEMA_DEPTH { kind = FieldKind::opt(kind); }
        assert_eq!(schema(kind.clone()).validate(), Err(SchemaDeclError::TooDeep));
        let mut bytes = Vec::new();
        kind.put(&mut bytes);
        assert_eq!(FieldKind::read(&mut crate::wire::WireReader::new(&bytes)), Err(crate::wire::WireError::TooDeep));
    }

    /// A reference names something by its identity: it goes to the wire and
    /// back, its kind is part of the shape, and an empty or overlong one is
    /// refused.
    #[test]
    fn a_reference_names_something_and_keeps_its_kind() {
        let body = FieldKind::seq(FieldKind::Ref(RefKind::Body), 4);
        let value = Value::Seq(vec![Value::Ref("placement:a".into()), Value::Ref("placement:b".into())]);
        assert_eq!(value.conforms(&body, 1), Ok(()));
        let mut bytes = Vec::new();
        value.put_wire(&mut bytes);
        assert_eq!(Value::read_wire(&body, &mut crate::wire::WireReader::new(&bytes)).unwrap(), value);
        let mut kind = Vec::new();
        body.put(&mut kind);
        assert_eq!(FieldKind::read(&mut crate::wire::WireReader::new(&kind)).unwrap(), body);
        let item = FieldKind::seq(FieldKind::Ref(RefKind::Item), 4);
        let digest = |kind: &FieldKind| {
            let mut d = Digest::new();
            kind.digest_into(&mut d);
            d.finish()
        };
        assert_ne!(digest(&body), digest(&item), "a body reference and an item reference are different shapes");
        assert!(matches!(Value::Ref(String::new()).conforms(&FieldKind::Ref(RefKind::Body), 1), Err(SchemaError::BadReference { .. })));
        let long = Value::Ref("x".repeat(MAX_REF_LEN as usize + 1));
        assert!(matches!(long.conforms(&FieldKind::Ref(RefKind::Item), 1), Err(SchemaError::BadReference { .. })));
    }

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
