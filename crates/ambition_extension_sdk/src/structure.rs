//! One structural description of a port's value (runtime authoring audit
//! P2): the declaration that gives the value's wire encoding, its script
//! form and a machine-readable [`Shape`].
//!
//! A [`Field`] codec says all four things about one Rust type: its shape, how
//! it is written to and read from the wire, and how it looks to a script and
//! is taken back from one. The SDK has codecs for its primitives; a record is
//! described once with [`port_record!`](crate::port_record), and a type the
//! SDK does not know (an enum, a maths type from another crate) gets one hand
//! written `Field` impl in the crate that owns it, so the four can still not
//! drift apart.
//!
//! A port then says `encode`/`decode` through its value's codec
//! ([`put`], [`read`]) and is made script-reachable with
//! [`script_port!`](crate::script_port).
//!
//! The wire encoding is the one [`crate::wire`] gives each primitive, field
//! by field in declaration order: describing a port this way does not change
//! its published bytes.

use crate::script::ScriptValue;
use crate::wire::{self, WireError, WireReader};

/// What a value is, for an author and for a tool.
#[derive(Clone, Debug, PartialEq)]
pub enum Shape {
    Bool,
    /// An integer in `0..=255`.
    U8,
    I32,
    U32,
    /// A finite `f32`: a script may not write NaN or an infinity.
    F32,
    /// `[x, y]`, two finite `f32`s.
    Vec2,
    Text,
    /// A finite logical value. Its consumer supplies the record schema.
    Data,
    /// The inner value, or `()` (in a record: the field may be absent).
    Opt(Box<Shape>),
    List(Box<Shape>),
    /// A table from text keys to values of one shape.
    Map(Box<Shape>),
    Record(Vec<FieldShape>),
    /// One of several cases; `script` says how a script writes each.
    Choice(Vec<CaseShape>),
}

/// One named field of a record.
#[derive(Clone, Debug, PartialEq)]
pub struct FieldShape {
    pub name: &'static str,
    pub shape: Shape,
    /// The field's documentation, as written above it.
    pub doc: &'static str,
    /// The value a script gets when it leaves the field out, in the
    /// script's own notation ([`ScriptValue::literal`]); `None` when the
    /// field is required.
    pub default: Option<String>,
}

/// One case of a [`Shape::Choice`].
#[derive(Clone, Debug, PartialEq)]
pub struct CaseShape {
    pub name: &'static str,
    pub payload: Option<Shape>,
    /// How a script writes this case.
    pub script: &'static str,
}

/// The codec of values of type `T`: their shape, wire encoding and script
/// form. Implemented by `T` itself for the SDK's primitives and for records
/// ([`port_record!`](crate::port_record)); by a marker type when the value
/// type is another crate's (a maths vector).
pub trait Field<T> {
    fn shape() -> Shape;
    fn put(value: &T, out: &mut Vec<u8>);
    fn read(r: &mut WireReader<'_>) -> Result<T, WireError>;
    fn to_script(value: &T) -> ScriptValue;
    fn from_script(value: &ScriptValue) -> Result<T, String>;
}

/// Encode `value` with its own codec.
pub fn put<T: Field<T>>(value: &T, out: &mut Vec<u8>) {
    T::put(value, out)
}

/// Decode a value with its own codec.
pub fn read<T: Field<T>>(r: &mut WireReader<'_>) -> Result<T, WireError> {
    T::read(r)
}

impl Field<bool> for bool {
    fn shape() -> Shape {
        Shape::Bool
    }
    fn put(v: &bool, out: &mut Vec<u8>) {
        wire::put_bool(out, *v)
    }
    fn read(r: &mut WireReader<'_>) -> Result<bool, WireError> {
        r.bool()
    }
    fn to_script(v: &bool) -> ScriptValue {
        ScriptValue::Bool(*v)
    }
    fn from_script(v: &ScriptValue) -> Result<bool, String> {
        v.as_bool()
    }
}

macro_rules! integer_field {
    ($ty:ty, $shape:ident, $put:path, $read:ident) => {
        impl Field<$ty> for $ty {
            fn shape() -> Shape {
                Shape::$shape
            }
            fn put(v: &$ty, out: &mut Vec<u8>) {
                $put(out, *v)
            }
            fn read(r: &mut WireReader<'_>) -> Result<$ty, WireError> {
                r.$read()
            }
            fn to_script(v: &$ty) -> ScriptValue {
                ScriptValue::Int(i64::from(*v))
            }
            fn from_script(v: &ScriptValue) -> Result<$ty, String> {
                let n = v.as_int()?;
                <$ty>::try_from(n).map_err(|_| format!("{n} is out of range for {}", stringify!($ty)))
            }
        }
    };
}

integer_field!(u8, U8, wire::put_u8, u8);
integer_field!(i32, I32, wire::put_i32, i32);
integer_field!(u32, U32, wire::put_u32, u32);

fn finite(n: f32) -> Result<f32, String> {
    if n.is_finite() {
        Ok(n)
    } else {
        Err(format!("{n} is not a finite number"))
    }
}

impl Field<f32> for f32 {
    fn shape() -> Shape {
        Shape::F32
    }
    fn put(v: &f32, out: &mut Vec<u8>) {
        wire::put_f32(out, *v)
    }
    fn read(r: &mut WireReader<'_>) -> Result<f32, WireError> {
        r.f32()
    }
    fn to_script(v: &f32) -> ScriptValue {
        ScriptValue::Num(*v)
    }
    fn from_script(v: &ScriptValue) -> Result<f32, String> {
        finite(v.as_num()?)
    }
}

impl Field<[f32; 2]> for [f32; 2] {
    fn shape() -> Shape {
        Shape::Vec2
    }
    fn put(v: &[f32; 2], out: &mut Vec<u8>) {
        wire::put_vec2(out, *v)
    }
    fn read(r: &mut WireReader<'_>) -> Result<[f32; 2], WireError> {
        r.vec2()
    }
    fn to_script(v: &[f32; 2]) -> ScriptValue {
        ScriptValue::vec2(*v)
    }
    fn from_script(v: &ScriptValue) -> Result<[f32; 2], String> {
        let [x, y] = v.as_vec2()?;
        Ok([finite(x)?, finite(y)?])
    }
}

/// Exact unsigned values use decimal text in scripts.
impl Field<u64> for u64 {
    fn shape() -> Shape { Shape::Text }
    fn put(value: &u64, out: &mut Vec<u8>) { wire::put_u64(out, *value); }
    fn read(r: &mut WireReader<'_>) -> Result<u64, WireError> { r.u64() }
    fn to_script(value: &u64) -> ScriptValue { ScriptValue::Text(value.to_string()) }
    fn from_script(value: &ScriptValue) -> Result<u64, String> {
        value.as_text()?.parse().map_err(|_| "expected an unsigned decimal u64 value".into())
    }
}

/// Four finite coordinates, such as a box or an RGBA colour.
impl Field<[f32; 4]> for [f32; 4] {
    fn shape() -> Shape { Shape::List(Box::new(Shape::F32)) }
    fn put(value: &[f32; 4], out: &mut Vec<u8>) {
        for value in value { wire::put_f32(out, *value); }
    }
    fn read(r: &mut WireReader<'_>) -> Result<[f32; 4], WireError> {
        Ok([r.f32()?, r.f32()?, r.f32()?, r.f32()?])
    }
    fn to_script(value: &[f32; 4]) -> ScriptValue {
        ScriptValue::List(value.iter().copied().map(ScriptValue::Num).collect())
    }
    fn from_script(value: &ScriptValue) -> Result<[f32; 4], String> {
        let values = value.as_list()?;
        let [a, b, c, d] = values else { return Err("expected four coordinates".into()); };
        Ok([finite(a.as_num()?)?, finite(b.as_num()?)?, finite(c.as_num()?)?, finite(d.as_num()?)?])
    }
}

impl Field<String> for String {
    fn shape() -> Shape {
        Shape::Text
    }
    fn put(v: &String, out: &mut Vec<u8>) {
        wire::put_str(out, v)
    }
    fn read(r: &mut WireReader<'_>) -> Result<String, WireError> {
        Ok(r.str()?.to_owned())
    }
    fn to_script(v: &String) -> ScriptValue {
        ScriptValue::Text(v.clone())
    }
    fn from_script(v: &ScriptValue) -> Result<String, String> {
        Ok(v.as_text()?.to_owned())
    }
}

impl<T, C: Field<T>> Field<Option<T>> for Option<C> {
    fn shape() -> Shape {
        Shape::Opt(Box::new(C::shape()))
    }
    fn put(v: &Option<T>, out: &mut Vec<u8>) {
        match v {
            None => wire::put_u8(out, 0),
            Some(v) => {
                wire::put_u8(out, 1);
                C::put(v, out);
            }
        }
    }
    fn read(r: &mut WireReader<'_>) -> Result<Option<T>, WireError> {
        r.opt(|r| C::read(r))
    }
    fn to_script(v: &Option<T>) -> ScriptValue {
        v.as_ref().map_or(ScriptValue::Unit, C::to_script)
    }
    fn from_script(v: &ScriptValue) -> Result<Option<T>, String> {
        match v {
            ScriptValue::Unit => Ok(None),
            v => C::from_script(v).map(Some),
        }
    }
}

impl<T, C: Field<T>> Field<Vec<T>> for Vec<C> {
    fn shape() -> Shape {
        Shape::List(Box::new(C::shape()))
    }
    fn put(v: &Vec<T>, out: &mut Vec<u8>) {
        wire::put_u32(out, v.len() as u32);
        for item in v {
            C::put(item, out);
        }
    }
    fn read(r: &mut WireReader<'_>) -> Result<Vec<T>, WireError> {
        let count = r.read_len()?;
        let mut items = Vec::with_capacity(count);
        for _ in 0..count {
            items.push(C::read(r)?);
        }
        Ok(items)
    }
    fn to_script(v: &Vec<T>) -> ScriptValue {
        ScriptValue::List(v.iter().map(C::to_script).collect())
    }
    fn from_script(v: &ScriptValue) -> Result<Vec<T>, String> {
        v.as_list()?
            .iter()
            .enumerate()
            .map(|(n, item)| C::from_script(item).map_err(|e| format!("[{n}]: {e}")))
            .collect()
    }
}

/// The codec of exactly two values, each with codec `C`: the two in order on
/// the wire, a list of two in a script (the two ends of a pair).
pub struct Pair<C>(std::marker::PhantomData<C>);

impl<T, C: Field<T>> Field<[T; 2]> for Pair<C> {
    fn shape() -> Shape {
        Shape::List(Box::new(C::shape()))
    }
    fn put(v: &[T; 2], out: &mut Vec<u8>) {
        C::put(&v[0], out);
        C::put(&v[1], out);
    }
    fn read(r: &mut WireReader<'_>) -> Result<[T; 2], WireError> {
        Ok([C::read(r)?, C::read(r)?])
    }
    fn to_script(v: &[T; 2]) -> ScriptValue {
        ScriptValue::List(v.iter().map(C::to_script).collect())
    }
    fn from_script(v: &ScriptValue) -> Result<[T; 2], String> {
        let [a, b] = v.as_list()? else { return Err("expected a list of two".into()); };
        Ok([C::from_script(a).map_err(|e| format!("[0]: {e}"))?, C::from_script(b).map_err(|e| format!("[1]: {e}"))?])
    }
}

/// The codec of a table from text keys to values: `(key, value)` pairs on
/// the wire (a count, then each key and value), a map in a script.
pub struct TextMap<C>(std::marker::PhantomData<C>);

impl<T, C: Field<T>> Field<Vec<(String, T)>> for TextMap<C> {
    fn shape() -> Shape {
        Shape::Map(Box::new(C::shape()))
    }
    fn put(v: &Vec<(String, T)>, out: &mut Vec<u8>) {
        wire::put_u32(out, v.len() as u32);
        for (key, value) in v {
            wire::put_str(out, key);
            C::put(value, out);
        }
    }
    fn read(r: &mut WireReader<'_>) -> Result<Vec<(String, T)>, WireError> {
        let count = r.read_len()?;
        let mut items = Vec::with_capacity(count);
        for _ in 0..count {
            let key = r.str()?.to_owned();
            items.push((key, C::read(r)?));
        }
        Ok(items)
    }
    fn to_script(v: &Vec<(String, T)>) -> ScriptValue {
        ScriptValue::Map(v.iter().map(|(k, value)| (k.clone(), C::to_script(value))).collect())
    }
    fn from_script(v: &ScriptValue) -> Result<Vec<(String, T)>, String> {
        let ScriptValue::Map(fields) = v else {
            return Err(format!("expected a table, got {}", v.kind()));
        };
        fields
            .iter()
            .map(|(k, value)| Ok((k.clone(), C::from_script(value).map_err(|e| format!("`{k}`: {e}"))?)))
            .collect()
    }
}

/// A record's fields from a script's table: the table must be a map, and a
/// field it names that the record does not declare is an error naming it (a
/// misspelt field must not pass silently). Used by
/// [`port_record!`](crate::port_record).
pub fn record_fields<'a>(v: &'a ScriptValue, declared: &[&str]) -> Result<&'a [(String, ScriptValue)], String> {
    let ScriptValue::Map(fields) = v else {
        return Err(format!("expected a table, got {}", v.kind()));
    };
    if let Some((stray, _)) = fields.iter().find(|(k, _)| !declared.contains(&k.as_str())) {
        return Err(format!("`{stray}`: no such field (the fields are {declared:?})"));
    }
    Ok(fields)
}

/// Describe a record type once: its fields, in wire order, each with its
/// codec (the field's own type unless `as Codec` names another) and an
/// optional default a script may leave the field out for. It implements
/// [`Field`] for the record: the shape, the wire encoding (each field's, in
/// this order), and the script form (a table; a stray field is an error, a
/// missing one is an error unless it has a default).
///
/// ```ignore
/// port_record! {
///     BodySound {
///         /// The sound's cue id.
///         cue: String,
///         /// Where it plays.
///         at: Place,
///     }
/// }
/// ```
///
/// Use `Value as Codec` with a local codec for a value owned by another crate.
/// Every field of the struct must be listed: the read builds the struct from
/// exactly these fields, so a field the list lacks or a misspelt one does
/// not compile.
#[macro_export]
macro_rules! port_record {
    ($ty:ident { $($fields:tt)* }) => {
        $crate::port_record!($ty as $ty { $($fields)* });
    };
    ($ty:ident as $record_codec:ty { $( $(#[doc = $doc:literal])* $field:ident : $fty:ty $(as $codec:ty)? $(= $default:expr)? ),* $(,)? }) => {
        impl $crate::structure::Field<$ty> for $record_codec {
            fn shape() -> $crate::structure::Shape {
                $crate::structure::Shape::Record(vec![$(
                    $crate::structure::FieldShape {
                        name: stringify!($field),
                        shape: <$crate::port_record!(@codec $fty $(, $codec)?) as $crate::structure::Field<$fty>>::shape(),
                        doc: concat!($($doc, "\n",)*),
                        default: $crate::port_record!(@default $fty, $crate::port_record!(@codec $fty $(, $codec)?) $(, $default)?),
                    },
                )*])
            }

            fn put(v: &$ty, out: &mut Vec<u8>) {
                $(<$crate::port_record!(@codec $fty $(, $codec)?) as $crate::structure::Field<$fty>>::put(&v.$field, out);)*
            }

            fn read(
                r: &mut $crate::wire::WireReader<'_>,
            ) -> Result<$ty, $crate::wire::WireError> {
                Ok($ty {
                    $($field: <$crate::port_record!(@codec $fty $(, $codec)?) as $crate::structure::Field<$fty>>::read(r)?,)*
                })
            }

            fn to_script(v: &$ty) -> $crate::script::ScriptValue {
                $crate::script::ScriptValue::Map(vec![$(
                    (
                        stringify!($field).to_owned(),
                        <$crate::port_record!(@codec $fty $(, $codec)?) as $crate::structure::Field<$fty>>::to_script(&v.$field),
                    ),
                )*])
            }

            fn from_script(v: &$crate::script::ScriptValue) -> Result<$ty, String> {
                let fields = $crate::structure::record_fields(v, &[$(stringify!($field)),*])?;
                Ok($ty {
                    $($field: match fields.iter().find(|(k, _)| k == stringify!($field)) {
                        Some((_, value)) => <$crate::port_record!(@codec $fty $(, $codec)?) as $crate::structure::Field<$fty>>::from_script(value)
                            .map_err(|e| format!("`{}`: {e}", stringify!($field)))?,
                        None => $crate::port_record!(@missing $field $($default)?),
                    },)*
                })
            }
        }
    };
    (@codec $fty:ty) => { $fty };
    (@codec $fty:ty, $codec:ty) => { $codec };
    (@default $fty:ty, $codec:ty) => { None };
    (@default $fty:ty, $codec:ty, $default:expr) => {
        Some(<$codec as $crate::structure::Field<$fty>>::to_script(&$default).literal())
    };
    (@missing $field:ident) => { return Err(format!("`{}`: missing", stringify!($field))) };
    (@missing $field:ident $default:expr) => { $default };
}

/// Make port `$port` reachable from scripts through its value's codec
/// ([`Field`]): the script form and the shape are the codec's.
/// Use `Port as Codec` when the port's value needs a separate codec type.
#[macro_export]
macro_rules! script_port {
    ($port:ty) => {
        $crate::script_port!($port as <$port as $crate::Port>::Value);
    };
    ($port:ty as $codec:ty) => {
        impl $crate::script::ScriptPort for $port {
            fn to_script(value: &<$port as $crate::Port>::Value) -> $crate::script::ScriptValue {
                <$codec as $crate::structure::Field<<$port as $crate::Port>::Value>>::to_script(value)
            }
            fn from_script(value: &$crate::script::ScriptValue) -> Result<<$port as $crate::Port>::Value, String> {
                <$codec as $crate::structure::Field<<$port as $crate::Port>::Value>>::from_script(value)
            }
            fn shape() -> Option<$crate::structure::Shape> {
                Some(<$codec as $crate::structure::Field<<$port as $crate::Port>::Value>>::shape())
            }
        }
    };
}

impl Shape {
    /// The shape as a one-line type, as an author reads it (`[x, y]`,
    /// `number?`, `{ cue: text, at: ... }`).
    pub fn summary(&self) -> String {
        match self {
            Shape::Bool => "bool".into(),
            Shape::U8 => "integer 0..=255".into(),
            Shape::I32 => "integer (i32)".into(),
            Shape::U32 => "integer (u32)".into(),
            Shape::F32 => "number".into(),
            Shape::Vec2 => "[x, y]".into(),
            Shape::Text => "text".into(),
            Shape::Data => "schema-selected data".into(),
            Shape::Opt(inner) => format!("{} or ()", inner.summary()),
            Shape::List(inner) => format!("list of {}", inner.summary()),
            Shape::Map(inner) => format!("table of text to {}", inner.summary()),
            Shape::Record(fields) => {
                let fields: Vec<String> = fields.iter().map(|f| format!("{}: {}", f.name, f.shape.summary())).collect();
                format!("{{ {} }}", fields.join(", "))
            }
            Shape::Choice(cases) => {
                let cases: Vec<&str> = cases.iter().map(|c| c.script).collect();
                format!("one of: {}", cases.join(" | "))
            }
        }
    }

    /// The shape as JSON (no dependency: the SDK has none).
    pub fn to_json(&self) -> String {
        let mut out = String::new();
        self.write_json(&mut out);
        out
    }

    fn write_json(&self, out: &mut String) {
        let simple = |out: &mut String, kind: &str| out.push_str(&format!("{{\"kind\":{}}}", json_str(kind)));
        match self {
            Shape::Bool => simple(out, "bool"),
            Shape::U8 => simple(out, "u8"),
            Shape::I32 => simple(out, "i32"),
            Shape::U32 => simple(out, "u32"),
            Shape::F32 => simple(out, "f32"),
            Shape::Vec2 => simple(out, "vec2"),
            Shape::Text => simple(out, "text"),
            Shape::Data => simple(out, "data"),
            Shape::Opt(inner) | Shape::List(inner) | Shape::Map(inner) => {
                let kind = match self {
                    Shape::Opt(_) => "opt",
                    Shape::List(_) => "list",
                    _ => "map",
                };
                out.push_str(&format!("{{\"kind\":{},\"of\":", json_str(kind)));
                inner.write_json(out);
                out.push('}');
            }
            Shape::Record(fields) => {
                out.push_str("{\"kind\":\"record\",\"fields\":[");
                for (n, f) in fields.iter().enumerate() {
                    if n > 0 {
                        out.push(',');
                    }
                    out.push_str(&format!("{{\"name\":{},\"doc\":{},", json_str(f.name), json_str(f.doc.trim())));
                    if let Some(default) = &f.default {
                        out.push_str(&format!("\"default\":{},", json_str(default)));
                    }
                    out.push_str("\"shape\":");
                    f.shape.write_json(out);
                    out.push('}');
                }
                out.push_str("]}");
            }
            Shape::Choice(cases) => {
                out.push_str("{\"kind\":\"choice\",\"cases\":[");
                for (n, c) in cases.iter().enumerate() {
                    if n > 0 {
                        out.push(',');
                    }
                    out.push_str(&format!("{{\"name\":{},\"script\":{}", json_str(c.name), json_str(c.script)));
                    if let Some(payload) = &c.payload {
                        out.push_str(",\"payload\":");
                        payload.write_json(out);
                    }
                    out.push('}');
                }
                out.push_str("]}");
            }
        }
    }
}

/// `s` as a JSON string literal.
pub fn json_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
