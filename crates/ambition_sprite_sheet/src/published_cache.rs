//! Cache derived sprite metadata. Selected source files define each key.
//! The cache is temporary. A failed read or write costs a derivation.

use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

/// Bump when the layout of the cache itself changes.
const FORMAT: u32 = 1;

/// The shared temporary cache. No path depends on the engine checkout.
pub(crate) fn dir() -> PathBuf {
    std::env::temp_dir().join("ambition/published-metadata")
}

/// FNV-1a. The key is a file name; a clash gives bytes that the reader
/// checks, and the inputs are a few paths, sizes and times.
fn fold(hash: &mut u64, bytes: &[u8]) {
    for byte in bytes {
        *hash ^= u64::from(*byte);
        *hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    // A boundary, so `("ab", "c")` is not `("a", "bc")`.
    *hash ^= 0xff;
    *hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
}

/// The key of `kind` derived from `sources` as they are on disk now. `None`
/// when a source cannot be read: there is nothing to derive from.
fn key(kind: &str, schema: &[&str], sources: &[&Path]) -> Option<u64> {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    fold(&mut hash, kind.as_bytes());
    fold(&mut hash, &FORMAT.to_le_bytes());
    for part in schema {
        fold(&mut hash, part.as_bytes());
    }
    for source in sources {
        let meta = std::fs::metadata(source).ok()?;
        let modified = meta.modified().ok()?.duration_since(UNIX_EPOCH).ok()?.as_nanos();
        fold(&mut hash, source.as_os_str().as_encoded_bytes());
        fold(&mut hash, &meta.len().to_le_bytes());
        fold(&mut hash, &modified.to_le_bytes());
    }
    Some(hash)
}

/// What `kind` derives from `sources`: `decode` of the bytes a run before
/// this one derived from the same files, or of the bytes `derive` gives now
/// (which are kept for the next run).
///
/// `schema` is the SOURCE TEXT of what gives the bytes their meaning (the
/// modules of the structures that are written). bincode does not describe
/// itself, so bytes that an earlier build wrote with another structure must
/// not be read: a change of that source is a change of the key.
///
/// A file that `decode` refuses (cut short, or of a key that clashed) is
/// derived again.
pub(crate) fn derived_in<T>(
    dir: &Path,
    kind: &str,
    schema: &[&str],
    sources: &[&Path],
    decode: impl Fn(&[u8]) -> Option<T>,
    derive: impl FnOnce() -> Option<Vec<u8>>,
) -> Option<T> {
    if sources.is_empty() {
        return decode(&derive()?);
    }
    let Some(key) = key(kind, schema, sources) else {
        return decode(&derive()?);
    };
    let file = dir.join(format!("{kind}-{key:016x}.bin"));
    if let Some(value) = std::fs::read(&file).ok().and_then(|bytes| decode(&bytes)) {
        return Some(value);
    }
    let bytes = derive()?;
    // Written beside the file and renamed: a second game that starts now
    // reads a whole file or none.
    let partial = dir.join(format!("{kind}-{key:016x}.{}.partial", std::process::id()));
    if std::fs::create_dir_all(dir).is_ok() && std::fs::write(&partial, &bytes).is_ok() {
        if std::fs::rename(&partial, &file).is_err() {
            let _ = std::fs::remove_file(&partial);
        }
    }
    decode(&bytes)
}

/// Read or derive metadata in the temporary cache.
pub(crate) fn derived<T>(
    kind: &str,
    schema: &[&str],
    sources: &[&Path],
    decode: impl Fn(&[u8]) -> Option<T>,
    derive: impl FnOnce() -> Option<Vec<u8>>,
) -> Option<T> {
    // Embedded metadata has no file cache key and needs no filesystem path.
    if sources.is_empty() {
        return decode(&derive()?);
    }
    derived_in(&dir(), kind, schema, sources, decode, derive)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("ambition_published_cache_test_{name}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a scratch folder");
        dir
    }

    #[test]
    fn a_second_read_of_one_file_does_not_derive_and_a_changed_file_does() {
        let dir = scratch("reread");
        let source = dir.join("table.ron");
        std::fs::write(&source, "one").expect("the source");
        let derivations = Cell::new(0);
        let read = || {
            derived_in(&dir.join("cache"), "table", &["schema"], &[&source], |bytes| Some(bytes.to_vec()), || {
                derivations.set(derivations.get() + 1);
                std::fs::read(&source).ok()
            })
        };
        assert_eq!(read().as_deref(), Some(&b"one"[..]));
        assert_eq!(read().as_deref(), Some(&b"one"[..]));
        assert_eq!(derivations.get(), 1, "the second read derived the bytes again");

        // Another size: the key is of the file as it is now.
        std::fs::write(&source, "three").expect("the source");
        assert_eq!(read().as_deref(), Some(&b"three"[..]));
        assert_eq!(derivations.get(), 2, "a changed file was read from the cache of the old one");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn bytes_that_do_not_decode_are_derived_again_and_another_schema_is_another_key() {
        let dir = scratch("refused");
        let source = dir.join("table.ron");
        std::fs::write(&source, "good").expect("the source");
        let cache = dir.join("cache");
        let whole = |bytes: &[u8]| Some(bytes.to_vec());
        let first = derived_in(&cache, "table", &["schema"], &[&source], whole, || Some(b"cut".to_vec()));
        assert_eq!(first.as_deref(), Some(&b"cut"[..]));
        let only_good = |bytes: &[u8]| (bytes == b"good").then(|| bytes.to_vec());
        let second = derived_in(&cache, "table", &["schema"], &[&source], only_good, || Some(b"good".to_vec()));
        assert_eq!(second.as_deref(), Some(&b"good"[..]));

        // The bytes of one schema are not read by a build with another.
        let other = derived_in(&cache, "table", &["schema 2"], &[&source], whole, || Some(b"new".to_vec()));
        assert_eq!(other.as_deref(), Some(&b"new"[..]));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
