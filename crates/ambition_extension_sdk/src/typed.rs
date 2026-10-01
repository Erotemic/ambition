//! Typed records: a Rust struct for a state schema, with no handwritten
//! codec.
//!
//! [`record!`](crate::record) declares the struct and its schema together. The
//! generated type has `KEY`, `schema()`, `load(inv)` and `store(&self, inv)`.
//! A field's position in the declaration is its position in the record; its
//! tag is the stable identity a schema change keeps. The struct's `Default` is
//! the record's initial value (false, zero, `None`, empty).
//!
//! ```
//! use ambition_extension_sdk::{record, SchemaKey};
//!
//! record! {
//!     /// A volley's memory.
//!     pub struct Volley = SchemaKey::new("example", "volley", 1);
//!     1 samples: Vec<[f32; 2]> [max 5],
//!     2 sample_accum: f32,
//!     3 fired_this_strike: bool,
//! }
//!
//! assert_eq!(Volley::schema().fields.len(), 3);
//! assert_eq!(Volley::default().samples, Vec::<[f32; 2]>::new());
//! ```

use crate::schema::{FieldKind, Value};

/// A Rust type that a record field can hold.
pub trait RecordField: Sized {
    /// The field's kind. `max` is the declared maximum length; only a
    /// sequence uses it, and a sequence must have one.
    fn kind(max: Option<u32>) -> FieldKind;
    fn to_value(&self) -> Value;
    /// `None` when `value` is not of this field's kind.
    fn from_value(value: &Value) -> Option<Self>;
}

impl RecordField for bool {
    fn kind(_: Option<u32>) -> FieldKind {
        FieldKind::Bool
    }
    fn to_value(&self) -> Value {
        Value::Bool(*self)
    }
    fn from_value(value: &Value) -> Option<Self> {
        value.as_bool()
    }
}

impl RecordField for u32 {
    fn kind(_: Option<u32>) -> FieldKind {
        FieldKind::U32
    }
    fn to_value(&self) -> Value {
        Value::U32(*self)
    }
    fn from_value(value: &Value) -> Option<Self> {
        value.as_u32()
    }
}

impl RecordField for i32 {
    fn kind(_: Option<u32>) -> FieldKind {
        FieldKind::I32
    }
    fn to_value(&self) -> Value {
        Value::I32(*self)
    }
    fn from_value(value: &Value) -> Option<Self> {
        match value {
            Value::I32(v) => Some(*v),
            _ => None,
        }
    }
}

impl RecordField for u64 {
    fn kind(_: Option<u32>) -> FieldKind {
        FieldKind::U64
    }
    fn to_value(&self) -> Value {
        Value::U64(*self)
    }
    fn from_value(value: &Value) -> Option<Self> {
        value.as_u64()
    }
}

impl RecordField for f32 {
    fn kind(_: Option<u32>) -> FieldKind {
        FieldKind::F32
    }
    fn to_value(&self) -> Value {
        Value::F32(*self)
    }
    fn from_value(value: &Value) -> Option<Self> {
        value.as_f32()
    }
}

/// A point or a vector in world units.
impl RecordField for [f32; 2] {
    fn kind(_: Option<u32>) -> FieldKind {
        FieldKind::Vec2
    }
    fn to_value(&self) -> Value {
        Value::Vec2(*self)
    }
    fn from_value(value: &Value) -> Option<Self> {
        value.as_vec2()
    }
}

impl<T: RecordField> RecordField for Option<T> {
    fn kind(max: Option<u32>) -> FieldKind {
        FieldKind::opt(T::kind(max))
    }
    fn to_value(&self) -> Value {
        Value::Opt(self.as_ref().map(|v| Box::new(v.to_value())))
    }
    fn from_value(value: &Value) -> Option<Self> {
        match value.as_opt()? {
            None => Some(None),
            Some(inner) => T::from_value(inner).map(Some),
        }
    }
}

impl<T: RecordField> RecordField for Vec<T> {
    /// ⛔ A sequence must declare its maximum length (`[max N]` in
    /// [`record!`](crate::record)): a record is bounded state.
    fn kind(max: Option<u32>) -> FieldKind {
        let max = max.expect("a Vec field in a record must declare `[max N]`");
        FieldKind::seq(T::kind(None), max)
    }
    fn to_value(&self) -> Value {
        Value::Seq(self.iter().map(RecordField::to_value).collect())
    }
    fn from_value(value: &Value) -> Option<Self> {
        match value {
            Value::Seq(items) => items.iter().map(T::from_value).collect(),
            _ => None,
        }
    }
}

/// Declare a typed record: the struct, its schema, and its load and store.
/// See the [module docs](crate::typed).
#[macro_export]
macro_rules! record {
    (
        $(#[$meta:meta])*
        $vis:vis struct $name:ident = $key:expr;
        $(
            $(#[$fmeta:meta])*
            $tag:literal $field:ident : $ty:ty $([max $max:expr])?
        ),* $(,)?
    ) => {
        $(#[$meta])*
        #[derive(Clone, Debug, Default, PartialEq)]
        $vis struct $name {
            $( $(#[$fmeta])* pub $field: $ty, )*
        }

        impl $name {
            pub const KEY: $crate::SchemaKey = $key;

            /// The schema: one body record, transient.
            pub fn schema() -> $crate::StateSchema {
                $crate::StateSchema {
                    key: Self::KEY,
                    attachment: $crate::Attachment::Body,
                    save: $crate::SaveEligibility::Transient,
                    fields: vec![
                        $(
                            $crate::FieldDecl::new(
                                $tag,
                                stringify!($field),
                                <$ty as $crate::typed::RecordField>::kind(
                                    $crate::record!(@max $($max)?),
                                ),
                            ),
                        )*
                    ],
                }
            }

            /// This invocation's record, as the struct.
            #[allow(unused_assignments)]
            pub fn load(inv: &mut $crate::Invocation<'_>) -> Result<Self, $crate::Fault> {
                let record = inv.state(&Self::KEY)?;
                let mut index = 0usize;
                Ok(Self {
                    $(
                        $field: {
                            let value = record.get($crate::FieldRef(index)).map_err(|error| {
                                $crate::Fault::Schema { schema: Self::KEY, error }
                            })?;
                            index += 1;
                            <$ty as $crate::typed::RecordField>::from_value(value).ok_or_else(|| {
                                $crate::Fault::Module(
                                    format!(
                                        "field {} of {} is not a {}",
                                        stringify!($field),
                                        Self::KEY,
                                        stringify!($ty)
                                    )
                                    .into(),
                                )
                            })?
                        },
                    )*
                })
            }

            /// Write the struct back as this invocation's record.
            #[allow(unused_assignments)]
            pub fn store(&self, inv: &mut $crate::Invocation<'_>) -> Result<(), $crate::Fault> {
                let record = inv.state(&Self::KEY)?;
                let mut index = 0usize;
                $(
                    record
                        .set(
                            $crate::FieldRef(index),
                            $crate::typed::RecordField::to_value(&self.$field),
                        )
                        .map_err(|error| $crate::Fault::Schema { schema: Self::KEY, error })?;
                    index += 1;
                )*
                let _ = index;
                Ok(())
            }
        }
    };
    (@max $max:expr) => { Some($max) };
    (@max) => { None };
}

#[cfg(test)]
#[allow(dead_code)] // `load` and `store` run through the host; see the modules
mod tests {
    use crate::{SchemaKey, StateSchema};

    record! {
        struct Everything = SchemaKey::new("test", "everything", 1);
        1 flag: bool,
        2 count: u32,
        3 delta: i32,
        4 big: u64,
        5 t: f32,
        7 at: [f32; 2],
        8 lock: Option<[f32; 2]>,
        9 trail: Vec<[f32; 2]> [max 4],
    }

    #[test]
    fn the_struct_default_is_the_records_initial_value() {
        let schema: StateSchema = Everything::schema();
        schema.validate().expect("a valid schema");
        assert_eq!(schema.fields.iter().map(|f| f.tag).collect::<Vec<_>>(), vec![1, 2, 3, 4, 5, 7, 8, 9]);
        let initial = schema.initial_record();
        let value = Everything::default();
        for (i, field) in schema.fields.iter().enumerate() {
            let _ = field;
            assert_eq!(
                initial.get(crate::FieldRef(i)).unwrap(),
                &match i {
                    0 => crate::typed::RecordField::to_value(&value.flag),
                    1 => crate::typed::RecordField::to_value(&value.count),
                    2 => crate::typed::RecordField::to_value(&value.delta),
                    3 => crate::typed::RecordField::to_value(&value.big),
                    4 => crate::typed::RecordField::to_value(&value.t),
                    5 => crate::typed::RecordField::to_value(&value.at),
                    6 => crate::typed::RecordField::to_value(&value.lock),
                    _ => crate::typed::RecordField::to_value(&value.trail),
                },
                "field {i}"
            );
        }
    }

    #[test]
    fn a_value_round_trips_through_its_field_kind() {
        use crate::typed::RecordField;
        let lock: Option<[f32; 2]> = Some([1.5, -2.0]);
        assert_eq!(Option::<[f32; 2]>::from_value(&lock.to_value()), Some(lock));
        let trail = vec![[1.0, 2.0], [3.0, 4.0]];
        assert_eq!(Vec::<[f32; 2]>::from_value(&trail.to_value()), Some(trail));
        assert_eq!(u32::from_value(&true.to_value()), None, "a kind mismatch is refused");
    }
}
