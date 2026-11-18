//! Typed records: a Rust struct for a state schema, with no handwritten
//! codec.
//!
//! [`record!`](crate::record) declares the struct and its schema together. The
//! generated type has `KEY`, `schema()`, `load(inv)` and `store(&self, inv)`.
//! A field's position in the declaration is its position in the record; its
//! tag is the stable identity a schema change keeps. The struct's `Default` is
//! the record's initial value (false, zero, `None`, empty).
//!
//! A record is attached to the body the entry runs for. `= KEY, per session;`
//! declares one record for the whole session instead, shared by every
//! invocation of the module (a session tally, a cursor across bodies).
//!
//! A record is transient: it lives in the simulation only. `, kept durable;`
//! after the attachment asks the durable world save to keep it (a session
//! record only, and only in a composition that installs that save road; the
//! host refuses it elsewhere).
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
    /// sequence or text value uses it, and each must have one.
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

impl RecordField for String {
    fn kind(max: Option<u32>) -> FieldKind {
        FieldKind::Text { max_bytes: max.expect("a String field must declare `[max N]`") }
    }
    fn to_value(&self) -> Value { Value::Text(self.clone()) }
    fn from_value(value: &Value) -> Option<Self> { value.as_text().map(str::to_owned) }
}

/// Declare a reusable value with tagged fields and no separate lifetime.
/// Use this type inside a `record!`, an option or a bounded sequence.
#[macro_export]
macro_rules! record_value {
    (
        $(#[$meta:meta])* $vis:vis struct $name:ident;
        $( $(#[$fmeta:meta])* $tag:literal $field:ident : $ty:ty $([max $max:expr])? ),* $(,)?
    ) => {
        $(#[$meta])*
        #[derive(Clone, Debug, Default, PartialEq)]
        $vis struct $name { $( $(#[$fmeta])* pub $field: $ty, )* }

        impl $name {
            pub fn fields() -> Vec<$crate::FieldDecl> {
                vec![$(
                    $crate::FieldDecl::new($tag, stringify!($field),
                        <$ty as $crate::typed::RecordField>::kind($crate::record!(@max $($max)?)))
                ),*]
            }

            pub fn to_record_value(&self) -> $crate::Record {
                $crate::Record::from_values(vec![$(
                    <$ty as $crate::typed::RecordField>::to_value(&self.$field)
                ),*])
            }

            #[allow(unused_assignments)]
            pub fn from_record_value(record: &$crate::Record) -> Option<Self> {
                if record.values().len() != Self::fields().len() { return None; }
                let mut index = 0usize;
                Some(Self { $( $field: {
                    let value = record.get($crate::FieldRef(index)).ok()?;
                    index += 1;
                    <$ty as $crate::typed::RecordField>::from_value(value)?
                }, )* })
            }
        }

        impl $crate::typed::RecordField for $name {
            fn kind(_: Option<u32>) -> $crate::FieldKind { $crate::FieldKind::Record(Self::fields()) }
            fn to_value(&self) -> $crate::Value { $crate::Value::Record(self.to_record_value()) }
            fn from_value(value: &$crate::Value) -> Option<Self> {
                Self::from_record_value(value.as_record()?)
            }
        }
    };
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
        $vis:vis struct $name:ident = $key:expr $(, per $attach:ident)? $(, kept $save:ident)?;
        $(
            $(#[$fmeta:meta])*
            $tag:literal $field:ident : $ty:ty $([max $max:expr])?
        ),* $(,)?
    ) => {
        $crate::record_value! {
            $(#[$meta])* $vis struct $name;
            $( $(#[$fmeta])* $tag $field: $ty $([max $max])?, )*
        }

        impl $name {
            pub const KEY: $crate::SchemaKey = $key;

            /// The schema: one record for each body (or, with `per session`,
            /// one for the session), transient unless `kept` names a save.
            pub fn schema() -> $crate::StateSchema {
                $crate::StateSchema {
                    key: Self::KEY,
                    attachment: $crate::record!(@attach $($attach)?),
                    save: $crate::record!(@save $($save)?),
                    fields: Self::fields(),
                }
            }

            /// This invocation's record, as the struct.
            pub fn load(inv: &mut $crate::Invocation<'_>) -> Result<Self, $crate::Fault> {
                let record = inv.state(&Self::KEY)?;
                Self::from_record(record)
            }

            /// A stored record, as the struct: what an inspector or a test
            /// reads from the host's store.
            pub fn from_record(record: &$crate::Record) -> Result<Self, $crate::Fault> {
                Self::schema().check(record)
                    .map_err(|error| $crate::Fault::Schema { schema: Self::KEY, error })?;
                Self::from_record_value(record).ok_or_else(|| $crate::Fault::Module(
                    format!("record does not match {}", Self::KEY).into()))
            }

            /// Convert this value to schema order.
            pub fn to_record(&self) -> $crate::Record { self.to_record_value() }

            /// Write the struct back as this invocation's record.
            pub fn store(&self, inv: &mut $crate::Invocation<'_>) -> Result<(), $crate::Fault> {
                *inv.state(&Self::KEY)? = self.to_record_value();
                Ok(())
            }
        }
    };
    (@max $max:expr) => { Some($max) };
    (@max) => { None };
    (@attach session) => { $crate::Attachment::Session };
    (@attach body) => { $crate::Attachment::Body };
    (@attach) => { $crate::Attachment::Body };
    (@save durable) => { $crate::SaveEligibility::Durable };
    (@save checkpoint) => { $crate::SaveEligibility::Checkpoint };
    (@save) => { $crate::SaveEligibility::Transient };
}

#[cfg(test)]
#[allow(dead_code)] // `load` and `store` run through the host; see the modules
mod tests {
    use crate::{SchemaKey, StateSchema};

    record_value! {
        struct Task;
        1 title: String [max 32],
        2 progress: u32,
    }

    record! {
        struct Tasks = SchemaKey::new("test", "tasks", 1), per session, kept durable;
        1 active: Option<Task>,
        2 queue: Vec<Task> [max 4],
    }

    #[test]
    fn reusable_values_compose_in_typed_and_script_records() {
        let value = Tasks { active: Some(Task { title: "build".into(), progress: 3 }),
            queue: vec![Task { title: "guard".into(), progress: 7 }] };
        let schema = Tasks::schema();
        schema.validate().unwrap();
        let record = value.to_record();
        schema.check(&record).unwrap();
        assert_eq!(Tasks::from_record(&record).unwrap(), value);
        let mut script = crate::script::record_to_script(&schema, &record).unwrap();
        assert_eq!(crate::script::record_from_script(&schema, &script).unwrap(), record);
        if let crate::script::ScriptValue::Map(fields) = &mut script {
            if let crate::script::ScriptValue::Map(active) = &mut fields[0].1 {
                active[0].0 = "typo".into();
            }
        }
        let error = crate::script::record_from_script(&schema, &script).unwrap_err();
        assert!(error.contains("active") && error.contains("typo"), "{error}");
        let mut too_long = value;
        too_long.queue[0].title = "x".repeat(33);
        assert!(schema.check(&too_long.to_record()).is_err());
    }

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
