//! Compile sprite-part metadata before application startup.

pub mod parts;

pub const SCHEMA_VERSION: u32 = 3;

pub const SCHEMA_SOURCE: &str = include_str!("parts.rs");

/// The compiled product for one authored part table.
pub fn compiled_path(logical: &str) -> Option<String> {
    logical.strip_suffix("_parts.ron")
        .map(|stem| format!("__prepared/sprite_parts/{stem}_parts.bin"))
}

/// Compile a part table with the same wire format as the live reader.
pub fn compile_metadata(logical: &str, source: &[u8]) -> Result<Option<(String, Vec<u8>)>, String> {
    let Some(path) = compiled_path(logical) else { return Ok(None) };
    let text = std::str::from_utf8(source).map_err(|error| format!("{logical}: {error}"))?;
    let table = read_ron(text).map_err(|error| format!("{logical}: {error}"))?;
    if !(1..=SCHEMA_VERSION).contains(&table.schema_version) {
        return Err(format!("{logical}: unsupported part schema {}", table.schema_version));
    }
    let bytes = bincode::serialize(&table).map_err(|error| format!("{logical}: {error}"))?;
    Ok(Some((path, bytes)))
}

/// Read the authored wire format, including implicit optional fields.
pub fn read_ron(text: &str) -> Result<parts::Published, ron::error::SpannedError> {
    ron::Options::default()
        .with_default_extension(ron::extensions::Extensions::IMPLICIT_SOME)
        .from_str(text)
}
