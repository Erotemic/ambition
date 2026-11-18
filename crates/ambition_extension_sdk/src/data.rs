//! Bounded logical data for schema-selected record initializers.

use crate::{script::ScriptValue, structure::{Field, Shape}, wire::{self, WireError, WireReader}};

const MAX_DEPTH: usize = crate::MAX_SCHEMA_DEPTH + 1;
const MAX_NODES: usize = 65_536;
const MAX_DATA_BYTES: usize = 1 << 20;

fn check(value: &ScriptValue, depth: usize, left: &mut usize, bytes: &mut usize) -> Result<(), String> {
    if depth >= MAX_DEPTH || *left == 0 { return Err("data exceeds its nesting or node bound".into()); }
    *left -= 1;
    let own = 1usize.saturating_add(match value {
        ScriptValue::Unit => 0, ScriptValue::Bool(_) => 1, ScriptValue::Int(_) => 8,
        ScriptValue::Num(_) => 4, ScriptValue::Text(text) => 4usize.saturating_add(text.len()),
        ScriptValue::List(_) => 4,
        ScriptValue::Map(fields) => fields.iter().fold(4usize, |n, (key, _)| n.saturating_add(4).saturating_add(key.len())),
    });
    *bytes = bytes.checked_sub(own).ok_or("data exceeds its byte bound")?;
    match value {
        ScriptValue::Num(n) if !n.is_finite() => return Err("data contains a non-finite number".into()),
        ScriptValue::Text(text) if text.len() > MAX_DATA_BYTES => return Err("data text exceeds its byte bound".into()),
        ScriptValue::List(values) => for value in values { check(value, depth + 1, left, bytes)?; },
        ScriptValue::Map(fields) => {
            let mut keys = std::collections::BTreeSet::new();
            for (key, value) in fields {
                if key.len() > MAX_DATA_BYTES || !keys.insert(key) { return Err("data has an overlong or duplicate key".into()); }
                check(value, depth + 1, left, bytes)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn put(value: &ScriptValue, out: &mut Vec<u8>) {
    match value {
        ScriptValue::Unit => wire::put_u8(out, 0),
        ScriptValue::Bool(value) => { wire::put_u8(out, 1); wire::put_bool(out, *value); }
        ScriptValue::Int(value) => { wire::put_u8(out, 2); out.extend_from_slice(&value.to_le_bytes()); }
        ScriptValue::Num(value) => { wire::put_u8(out, 3); wire::put_f32(out, *value); }
        ScriptValue::Text(value) => { wire::put_u8(out, 4); wire::put_str(out, value); }
        ScriptValue::List(values) => {
            wire::put_u8(out, 5); wire::put_u32(out, values.len() as u32);
            for value in values { put(value, out); }
        }
        ScriptValue::Map(fields) => {
            wire::put_u8(out, 6); wire::put_u32(out, fields.len() as u32);
            let mut fields: Vec<_> = fields.iter().collect();
            fields.sort_by(|a, b| a.0.cmp(&b.0));
            for (key, value) in fields { wire::put_str(out, key); put(value, out); }
        }
    }
}

fn read(reader: &mut WireReader<'_>, depth: usize, left: &mut usize, start: usize) -> Result<ScriptValue, WireError> {
    if reader.position().saturating_sub(start) >= MAX_DATA_BYTES { return Err(WireError::TooLong(MAX_DATA_BYTES as u64 + 1)); }
    if depth >= MAX_DEPTH { return Err(WireError::TooDeep); }
    if *left == 0 { return Err(WireError::TooLong(MAX_NODES as u64 + 1)); }
    *left -= 1;
    let text = |reader: &mut WireReader<'_>| {
        let text = reader.str()?;
        if reader.position().saturating_sub(start) > MAX_DATA_BYTES { return Err(WireError::TooLong(reader.position().saturating_sub(start) as u64)); }
        Ok(text.to_owned())
    };
    Ok(match reader.u8()? {
        0 => ScriptValue::Unit,
        1 => ScriptValue::Bool(reader.bool()?),
        2 => ScriptValue::Int(reader.u64()? as i64),
        3 => {
            let value = reader.f32()?;
            if !value.is_finite() { return Err(WireError::BadTag(3)); }
            ScriptValue::Num(value)
        }
        4 => ScriptValue::Text(text(reader)?),
        tag @ (5 | 6) => {
            let count = reader.read_len()?;
            if count > *left { return Err(WireError::TooLong(count as u64)); }
            if tag == 5 {
                ScriptValue::List((0..count).map(|_| read(reader, depth + 1, left, start)).collect::<Result<_, _>>()?)
            } else {
                let mut keys = std::collections::BTreeSet::new();
                let mut fields = Vec::new();
                for _ in 0..count {
                    let key = text(reader)?;
                    if !keys.insert(key.clone()) { return Err(WireError::BadString); }
                    fields.push((key, read(reader, depth + 1, left, start)?));
                }
                ScriptValue::Map(fields)
            }
        }
        other => return Err(WireError::BadTag(other)),
    })
}

impl Field<ScriptValue> for ScriptValue {
    fn shape() -> Shape { Shape::Data }
    fn put(value: &Self, out: &mut Vec<u8>) { put(value, out); }
    fn read(reader: &mut WireReader<'_>) -> Result<Self, WireError> {
        let mut left = MAX_NODES;
        let start = reader.position();
        let value = read(reader, 0, &mut left, start)?;
        let len = reader.position().saturating_sub(start);
        if len > MAX_DATA_BYTES { return Err(WireError::TooLong(len as u64)); }
        Ok(value)
    }
    fn to_script(value: &Self) -> Self { value.clone() }
    fn from_script(value: &Self) -> Result<Self, String> {
        let mut left = MAX_NODES;
        let mut bytes = MAX_DATA_BYTES;
        check(value, 0, &mut left, &mut bytes)?;
        Ok(value.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn logical_data_has_exact_integers_canonical_keys_and_bounded_depth() {
        let value = ScriptValue::Map(vec![
            ("z".into(), ScriptValue::Int(i64::MIN)),
            ("a".into(), ScriptValue::List(vec![ScriptValue::Unit, ScriptValue::Text("écho".into())])),
        ]);
        let mut bytes = Vec::new(); <ScriptValue as Field<_>>::put(&value, &mut bytes);
        let decoded = wire::decode_all(&bytes, <ScriptValue as Field<_>>::read).unwrap();
        let ScriptValue::Map(fields) = &decoded else { panic!("a table") };
        assert_eq!(fields[0].0, "a");
        assert_eq!(fields[1].1, ScriptValue::Int(i64::MIN));
        let mut again = Vec::new(); <ScriptValue as Field<_>>::put(&decoded, &mut again);
        assert_eq!(again, bytes);
        assert!(ScriptValue::from_script(&ScriptValue::Num(f32::NAN)).is_err());
        let mut deep = ScriptValue::Unit;
        for _ in 0..MAX_DEPTH { deep = ScriptValue::List(vec![deep]); }
        assert!(ScriptValue::from_script(&deep).is_err());
        bytes.clear(); <ScriptValue as Field<_>>::put(&deep, &mut bytes);
        assert_eq!(wire::decode_all(&bytes, <ScriptValue as Field<_>>::read), Err(WireError::TooDeep));
        let large = ScriptValue::List(vec![ScriptValue::Text("x".repeat(MAX_DATA_BYTES / 2)); 2]);
        assert!(ScriptValue::from_script(&large).is_err());
        bytes.clear(); <ScriptValue as Field<_>>::put(&large, &mut bytes);
        assert!(matches!(wire::decode_all(&bytes, <ScriptValue as Field<_>>::read), Err(WireError::TooLong(_))));
    }
}
