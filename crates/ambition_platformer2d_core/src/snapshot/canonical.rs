//! Canonical serialization of immutable preparation data.

use serde::ser::{self, Serialize};

mod value;
pub use value::PreparedValue;

/// An immutable prepared value could not be serialized.
#[derive(Clone, Debug)]
pub struct Error(String);
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for Error {}
impl ser::Error for Error {
    fn custom<T: std::fmt::Display>(message: T) -> Self {
        Self(message.to_string())
    }
}

/// Encode all serialized fields with stable type tags and map order.
/// Float values use the snapshot float policy. This has no decode path.
pub fn encode<T: Serialize + ?Sized>(value: &T) -> Result<Vec<u8>, Error> {
    let mut out = b"ambition-prepared-value-v1\0".to_vec();
    field(&mut out, &project(value)?);
    Ok(out)
}

fn project<T: Serialize + ?Sized>(value: &T) -> Result<Vec<u8>, Error> {
    value.serialize(Encoder)
}

fn field(out: &mut Vec<u8>, bytes: &[u8]) {
    out.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
    out.extend_from_slice(bytes);
}
fn atom(tag: u8, bytes: &[u8]) -> Vec<u8> {
    let mut out = vec![tag];
    field(&mut out, bytes);
    out
}
fn named(tag: u8, name: &str) -> Vec<u8> {
    atom(tag, name.as_bytes())
}
fn variant(tag: u8, name: &str, index: u32, variant: &str) -> Vec<u8> {
    let mut out = named(tag, name);
    out.extend_from_slice(&index.to_le_bytes());
    field(&mut out, variant.as_bytes());
    out
}

struct Encoder;
struct Compound {
    head: Vec<u8>,
    fields: Vec<Vec<u8>>,
}
impl Compound {
    fn new(head: Vec<u8>) -> Self {
        Self {
            head,
            fields: vec![],
        }
    }
    fn push<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
        self.fields.push(project(value)?);
        Ok(())
    }
    fn finish(mut self) -> Vec<u8> {
        self.head
            .extend_from_slice(&(self.fields.len() as u64).to_le_bytes());
        for item in self.fields {
            field(&mut self.head, &item);
        }
        self.head
    }
}
struct Map {
    entries: Vec<(Vec<u8>, Vec<u8>)>,
    key: Option<Vec<u8>>,
}

impl ser::Serializer for Encoder {
    type Ok = Vec<u8>;
    type Error = Error;
    type SerializeSeq = Compound;
    type SerializeTuple = Compound;
    type SerializeTupleStruct = Compound;
    type SerializeTupleVariant = Compound;
    type SerializeMap = Map;
    type SerializeStruct = Compound;
    type SerializeStructVariant = Compound;
    fn serialize_bool(self, v: bool) -> Result<Vec<u8>, Error> {
        Ok(atom(0, &[u8::from(v)]))
    }
    fn serialize_i8(self, v: i8) -> Result<Vec<u8>, Error> {
        Ok(atom(1, &v.to_le_bytes()))
    }
    fn serialize_i16(self, v: i16) -> Result<Vec<u8>, Error> {
        Ok(atom(2, &v.to_le_bytes()))
    }
    fn serialize_i32(self, v: i32) -> Result<Vec<u8>, Error> {
        Ok(atom(3, &v.to_le_bytes()))
    }
    fn serialize_i64(self, v: i64) -> Result<Vec<u8>, Error> {
        Ok(atom(4, &v.to_le_bytes()))
    }
    fn serialize_i128(self, v: i128) -> Result<Vec<u8>, Error> {
        Ok(atom(5, &v.to_le_bytes()))
    }
    fn serialize_u8(self, v: u8) -> Result<Vec<u8>, Error> {
        Ok(atom(6, &v.to_le_bytes()))
    }
    fn serialize_u16(self, v: u16) -> Result<Vec<u8>, Error> {
        Ok(atom(7, &v.to_le_bytes()))
    }
    fn serialize_u32(self, v: u32) -> Result<Vec<u8>, Error> {
        Ok(atom(8, &v.to_le_bytes()))
    }
    fn serialize_u64(self, v: u64) -> Result<Vec<u8>, Error> {
        Ok(atom(9, &v.to_le_bytes()))
    }
    fn serialize_u128(self, v: u128) -> Result<Vec<u8>, Error> {
        Ok(atom(10, &v.to_le_bytes()))
    }
    fn serialize_f32(self, v: f32) -> Result<Vec<u8>, Error> {
        let mut bytes = vec![];
        super::put_f32(&mut bytes, v);
        Ok(atom(11, &bytes))
    }
    fn serialize_f64(self, v: f64) -> Result<Vec<u8>, Error> {
        let bits = if v.is_nan() {
            f64::NAN.to_bits()
        } else {
            v.to_bits()
        };
        Ok(atom(12, &bits.to_le_bytes()))
    }
    fn serialize_char(self, v: char) -> Result<Vec<u8>, Error> {
        Ok(atom(13, &(v as u32).to_le_bytes()))
    }
    fn serialize_str(self, v: &str) -> Result<Vec<u8>, Error> {
        Ok(atom(14, v.as_bytes()))
    }
    fn serialize_bytes(self, v: &[u8]) -> Result<Vec<u8>, Error> {
        Ok(atom(15, v))
    }
    fn serialize_none(self) -> Result<Vec<u8>, Error> {
        Ok(vec![16])
    }
    fn serialize_some<T: Serialize + ?Sized>(self, v: &T) -> Result<Vec<u8>, Error> {
        Ok(atom(17, &project(v)?))
    }
    fn serialize_unit(self) -> Result<Vec<u8>, Error> {
        Ok(vec![18])
    }
    fn serialize_unit_struct(self, name: &'static str) -> Result<Vec<u8>, Error> {
        Ok(named(19, name))
    }
    fn serialize_unit_variant(
        self,
        name: &'static str,
        index: u32,
        name_variant: &'static str,
    ) -> Result<Vec<u8>, Error> {
        Ok(variant(20, name, index, name_variant))
    }
    fn serialize_newtype_struct<T: Serialize + ?Sized>(
        self,
        name: &'static str,
        value: &T,
    ) -> Result<Vec<u8>, Error> {
        let mut out = named(21, name);
        field(&mut out, &project(value)?);
        Ok(out)
    }
    fn serialize_newtype_variant<T: Serialize + ?Sized>(
        self,
        name: &'static str,
        index: u32,
        name_variant: &'static str,
        value: &T,
    ) -> Result<Vec<u8>, Error> {
        let mut out = variant(22, name, index, name_variant);
        field(&mut out, &project(value)?);
        Ok(out)
    }
    fn serialize_seq(self, _: Option<usize>) -> Result<Compound, Error> {
        Ok(Compound::new(vec![23]))
    }
    fn serialize_tuple(self, _: usize) -> Result<Compound, Error> {
        Ok(Compound::new(vec![24]))
    }
    fn serialize_tuple_struct(self, name: &'static str, _: usize) -> Result<Compound, Error> {
        Ok(Compound::new(named(25, name)))
    }
    fn serialize_tuple_variant(
        self,
        name: &'static str,
        index: u32,
        name_variant: &'static str,
        _: usize,
    ) -> Result<Compound, Error> {
        Ok(Compound::new(variant(26, name, index, name_variant)))
    }
    fn serialize_map(self, _: Option<usize>) -> Result<Map, Error> {
        Ok(Map {
            entries: vec![],
            key: None,
        })
    }
    fn serialize_struct(self, name: &'static str, _: usize) -> Result<Compound, Error> {
        Ok(Compound::new(named(28, name)))
    }
    fn serialize_struct_variant(
        self,
        name: &'static str,
        index: u32,
        name_variant: &'static str,
        _: usize,
    ) -> Result<Compound, Error> {
        Ok(Compound::new(variant(29, name, index, name_variant)))
    }
    fn is_human_readable(&self) -> bool {
        false
    }
}
impl ser::SerializeSeq for Compound {
    type Ok = Vec<u8>;
    type Error = Error;
    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
        self.push(value)
    }
    fn end(self) -> Result<Vec<u8>, Error> {
        Ok(self.finish())
    }
}
impl ser::SerializeTuple for Compound {
    type Ok = Vec<u8>;
    type Error = Error;
    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
        self.push(value)
    }
    fn end(self) -> Result<Vec<u8>, Error> {
        Ok(self.finish())
    }
}
impl ser::SerializeTupleStruct for Compound {
    type Ok = Vec<u8>;
    type Error = Error;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
        self.push(value)
    }
    fn end(self) -> Result<Vec<u8>, Error> {
        Ok(self.finish())
    }
}
impl ser::SerializeTupleVariant for Compound {
    type Ok = Vec<u8>;
    type Error = Error;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
        self.push(value)
    }
    fn end(self) -> Result<Vec<u8>, Error> {
        Ok(self.finish())
    }
}
impl ser::SerializeStruct for Compound {
    type Ok = Vec<u8>;
    type Error = Error;
    fn serialize_field<T: Serialize + ?Sized>(
        &mut self,
        name: &'static str,
        value: &T,
    ) -> Result<(), Error> {
        self.push(&(name, value))
    }
    fn end(self) -> Result<Vec<u8>, Error> {
        Ok(self.finish())
    }
}
impl ser::SerializeStructVariant for Compound {
    type Ok = Vec<u8>;
    type Error = Error;
    fn serialize_field<T: Serialize + ?Sized>(
        &mut self,
        name: &'static str,
        value: &T,
    ) -> Result<(), Error> {
        self.push(&(name, value))
    }
    fn end(self) -> Result<Vec<u8>, Error> {
        Ok(self.finish())
    }
}
impl ser::SerializeMap for Map {
    type Ok = Vec<u8>;
    type Error = Error;
    fn serialize_key<T: Serialize + ?Sized>(&mut self, key: &T) -> Result<(), Error> {
        if self.key.is_some() {
            return Err(ser::Error::custom("map key has no value"));
        }
        self.key = Some(project(key)?);
        Ok(())
    }
    fn serialize_value<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
        let key = self
            .key
            .take()
            .ok_or_else(|| <Error as ser::Error>::custom("map value has no key"))?;
        self.entries.push((key, project(value)?));
        Ok(())
    }
    fn end(mut self) -> Result<Vec<u8>, Error> {
        if self.key.is_some() {
            return Err(ser::Error::custom("map key has no value"));
        }
        self.entries.sort_unstable();
        let mut out = vec![27];
        out.extend_from_slice(&(self.entries.len() as u64).to_le_bytes());
        for (key, value) in self.entries {
            field(&mut out, &key);
            field(&mut out, &value);
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::encode;

    #[test]
    fn unordered_registry_rows_have_the_same_projection_as_ordered_rows() {
        let ordered: std::collections::BTreeMap<_, _> = (0..32)
            .map(|id| (format!("actor-{id}"), (id, id as f32 * 1.5)))
            .collect();
        let unordered: std::collections::HashMap<_, _> = ordered
            .iter()
            .rev()
            .map(|(id, row)| (id.clone(), *row))
            .collect();
        assert_eq!(encode(&ordered).unwrap(), encode(&unordered).unwrap());
    }

    #[test]
    fn float_projection_keeps_signed_zero_and_normalizes_nan_payloads() {
        assert_ne!(encode(&0.0_f32).unwrap(), encode(&-0.0_f32).unwrap());
        assert_ne!(
            encode(&f32::INFINITY).unwrap(),
            encode(&f32::NEG_INFINITY).unwrap()
        );
        assert_eq!(
            encode(&f32::NAN).unwrap(),
            encode(&f32::from_bits(0xffc0_0021)).unwrap()
        );
        assert_ne!(encode(&0.0_f64).unwrap(), encode(&-0.0_f64).unwrap());
        assert_ne!(encode(&f64::INFINITY).unwrap(), encode(&f64::NAN).unwrap());
        assert_eq!(
            encode(&f64::NAN).unwrap(),
            encode(&f64::from_bits(0xfff8_0000_0000_0021)).unwrap()
        );
    }

    #[test]
    fn adjacent_names_cannot_change_their_field_boundaries() {
        assert_ne!(encode(&("ab", "c")).unwrap(), encode(&("a", "bc")).unwrap());
        assert_ne!(encode(&Some(0_u32)).unwrap(), encode(&None::<u32>).unwrap());
    }
}
