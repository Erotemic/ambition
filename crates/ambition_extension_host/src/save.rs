//! The records a save road keeps, as save rows, and their adoption on a load.
//!
//! The host names no save file. A composition's road adapter
//! ([`crate::SaveRoad`]) reads the rows from the live store, keeps them in its
//! save, and gives them back to [`adopt_saved_rows`] when the save loads.
//!
//! A row carries the schema it was written under, so a load after a module
//! edit migrates the record by field tag (`StateSchema::migrate`), as a reload
//! does. A row whose schema this composition did not admit stays in the save
//! untouched: a module that is not loaded today does not lose its world.

use ambition_extension_sdk::wire::WireReader;
use ambition_extension_sdk::{SchemaKey, StateSchema};

use crate::admission::{Admitted, SaveRoad};
use crate::store::RecordSet;

/// One saved record: the schema it was written under and its values, each
/// in the exact wire encoding, and the schema key as text for a reader.
///
/// The schema key in `schema` is the identity of the row. `key` is a label
/// and an index: a row whose label names another schema is refused.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SavedRow {
    pub key: String,
    pub schema: Vec<u8>,
    pub record: Vec<u8>,
}

/// The admitted schemas that road `road` keeps.
fn kept_by(admitted: &Admitted, road: SaveRoad) -> impl Iterator<Item = &SchemaKey> {
    admitted
        .schemas
        .iter()
        .filter(move |(_, s)| SaveRoad::of(s.schema.attachment, s.schema.save) == Some(road))
        .map(|(key, _)| key)
}

/// The rows road `road` keeps from `records`, in key order. A record that is
/// not stored is at its initial value, and has no row.
pub fn saved_rows(admitted: &Admitted, road: SaveRoad, records: &RecordSet) -> Vec<SavedRow> {
    let mut rows: Vec<SavedRow> = kept_by(admitted, road)
        .filter_map(|key| {
            let stored = records.get_stored(key)?;
            let mut schema = Vec::new();
            admitted.schemas[key].schema.put(&mut schema);
            let mut record = Vec::new();
            stored.record.put_wire(&mut record);
            Some(SavedRow { key: key.to_string(), schema, record })
        })
        .collect();
    rows.sort();
    rows
}

/// The schema key written in `row`, if its schema decodes.
fn written_key(row: &SavedRow) -> Option<SchemaKey> {
    StateSchema::read(&mut WireReader::new(&row.schema)).ok().map(|schema| schema.key)
}

/// The rows a save keeps after road `road` writes `live` into `saved`: the
/// live rows, and each saved row of a schema the road does not keep in this
/// composition (a module that is not loaded), unchanged.
///
/// ⛔ A saved row is dropped when EITHER its label or the schema written in
/// it is one the road keeps. It kept a row by its label only, so a row
/// labelled as a module that is not loaded, with the schema of a module that
/// is, stayed in the save beside the live row, and the next load adopted it
/// over that live record.
pub fn merged_rows(admitted: &Admitted, road: SaveRoad, saved: &[SavedRow], live: Vec<SavedRow>) -> Vec<SavedRow> {
    let kept: Vec<String> = kept_by(admitted, road).map(ToString::to_string).collect();
    let mut rows: Vec<SavedRow> = saved
        .iter()
        .filter(|row| {
            !kept.contains(&row.key) && written_key(row).is_none_or(|key| !kept.contains(&key.to_string()))
        })
        .cloned()
        .chain(live)
        .collect();
    rows.sort();
    rows
}

/// Put each saved row that road `road` keeps into `records`, migrated to the
/// admitted shape. Returns one line for each row that could not be read; that
/// row is not adopted, and the live record keeps its value.
///
/// A row is refused when its label names another schema than the one
/// written in it, and every row of a schema that two rows give is refused:
/// the save does not say which is the record, and the last one in the list
/// won.
pub fn adopt_saved_rows(admitted: &Admitted, road: SaveRoad, records: &mut RecordSet, saved: &[SavedRow]) -> Vec<String> {
    let mut problems = Vec::new();
    let keys: Vec<Option<SchemaKey>> = saved.iter().map(written_key).collect();
    for (row, key) in saved.iter().zip(&keys) {
        if let Some(key) = key {
            if row.key != key.to_string() {
                problems.push(format!("saved record {}: its schema is {key}, not the schema its label names", row.key));
                continue;
            }
            if keys.iter().filter(|other| other.as_ref() == Some(key)).count() > 1 {
                problems.push(format!("saved record {}: the save has more than one row of {key}", row.key));
                continue;
            }
        }
        let written = match StateSchema::read(&mut WireReader::new(&row.schema)) {
            Ok(schema) => schema,
            Err(error) => {
                problems.push(format!("saved record {}: its schema does not decode: {error:?}", row.key));
                continue;
            }
        };
        let Some(admitted_schema) = admitted.schemas.get(&written.key) else {
            continue;
        };
        if SaveRoad::of(admitted_schema.schema.attachment, admitted_schema.schema.save) != Some(road) {
            continue;
        }
        let mut reader = WireReader::new(&row.record);
        let record = match written.read_record(&mut reader).map(|record| (reader.finish(), record)) {
            Ok((Ok(()), record)) if written.check(&record).is_ok() => record,
            _ => {
                problems.push(format!("saved record {}: its values do not decode under its schema", row.key));
                continue;
            }
        };
        let record = if written.shape_digest() == admitted_schema.shape {
            record
        } else {
            admitted_schema.schema.migrate(&written, &record)
        };
        records.put(written.key, admitted_schema.shape, record);
    }
    problems
}
