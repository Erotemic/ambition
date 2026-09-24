//! Where a persisted file's bytes go, per platform.
//!
//! The settings and save systems (`load_settings_at_startup`,
//! `save_settings_on_change`, `load_save_at_startup`, `autosave_sandbox_save`)
//! must compile on wasm too. If they are gated out of the web build, nothing
//! persists and nothing warns.
//!
//! The path is the address on both platforms. `PersistenceRoot` stays a
//! `PathBuf` and callers pass a `&Path`; only the last step differs. On the web,
//! two roots are two key prefixes, so `PersistenceRoot::isolated()` still works.
//!
//! Policy and bridge are split (as in `render_recovery`). The browser call
//! cannot run in tests, so all decisions live in [`storage_key`], [`read_from`],
//! and [`write_into`], which are tested against an in-memory map. Only the
//! `web_sys` lines are untested.

use std::path::Path;

/// The key a path becomes in a flat key/value store.
///
/// `localStorage` is one flat namespace per origin, so the full path, root
/// included, goes into the key. Otherwise two `PersistenceRoot`s collide and an
/// isolated App reads the player's settings.
///
/// The separator is normalised to `/`, so keys agree across platforms
/// (`Path::display` gives `\` on Windows).
///
/// The `ambition:` prefix namespaces the keys on the origin.
pub fn storage_key(path: &Path) -> String {
    let mut key = String::from("ambition:");
    let mut first = true;
    for component in path.components() {
        let text = component.as_os_str().to_string_lossy();
        if text.is_empty() {
            continue;
        }
        if !first {
            key.push('/');
        }
        first = false;
        key.push_str(text.trim_end_matches(['/', '\\']));
    }
    key
}

/// Read a persisted document, or `NotFound` when nothing is stored there.
///
/// The `NotFound` kind is required. Callers use it to tell "no file yet, start
/// fresh" from "a file exists and could not be read, do not overwrite it". See
/// `load_save` and `LoadedSave::preserve`.
pub fn read(path: &Path) -> std::io::Result<String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::fs::read_to_string(path)
    }
    #[cfg(target_arch = "wasm32")]
    {
        read_from(&browser_storage()?, path)
    }
}

/// Write a persisted document, replacing whatever was there.
pub fn write(path: &Path, body: &str) -> std::io::Result<()> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        // Native road: the callers own the temp-file dance, because their
        // recovery rules differ. This is only the plain write.
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, body)
    }
    #[cfg(target_arch = "wasm32")]
    {
        write_into(&browser_storage()?, path, body)
    }
}

/// Remove a persisted document. Absent is not an error.
pub fn remove(path: &Path) -> std::io::Result<()> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        match std::fs::remove_file(path) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            other => other,
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        remove_from(&browser_storage()?, path)
    }
}

// There is no `exists`, on purpose. A preflight check can consult the wrong
// store (the native filesystem on the browser build), and a bool cannot tell
// "nothing stored" from "storage is blocked". The save road needs that
// difference: one allows a write, the other forbids it.
//
// One read answers both questions. `read` returns `NotFound` for absence and
// any other kind for a refusal. Callers carry "was a document there" in their
// return value; see `LoadedSave::present`.

// ── The key/value policy, decided without a browser ────────────────────────
//
// These are compiled on every platform so the tests below run. A wasm-only
// gate would make the guard vacuous on the test machine.

/// A flat key/value store: `localStorage`, or a map in a test.
pub trait KeyValueStore {
    /// The value at `key`; `Ok(None)` when nothing is stored there.
    ///
    /// `Ok(None)` means absence only. A browser can refuse the read (site data
    /// blocked, cross-origin frame); that is `Err`. Only absence allows a write.
    fn get(&self, key: &str) -> Result<Option<String>, String>;
    /// Store `value` at `key`. `Err` carries a human-readable reason.
    fn set(&self, key: &str, value: &str) -> Result<(), String>;
    /// Remove `key`. Absent is not an error.
    fn remove(&self, key: &str) -> Result<(), String>;
}

/// [`read`] against any key/value store.
pub fn read_from(storage: &dyn KeyValueStore, path: &Path) -> std::io::Result<String> {
    let key = storage_key(path);
    match storage.get(&key) {
        Ok(Some(body)) => Ok(body),
        Ok(None) => Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("no stored value for {key}"),
        )),
        // NOT `NotFound`. The store is there and said no; the callers read that
        // as "a document may exist and I could not see it" and refuse to write.
        Err(reason) => Err(std::io::Error::other(format!(
            "could not read {key}: {reason}"
        ))),
    }
}

/// [`write`] against any key/value store.
pub fn write_into(storage: &dyn KeyValueStore, path: &Path, body: &str) -> std::io::Result<()> {
    // A browser can refuse this: `localStorage` has a quota of a few MB and
    // throws when it is full or site data is blocked. That becomes an ordinary
    // IO error, which callers log and continue past.
    storage
        .set(&storage_key(path), body)
        .map_err(std::io::Error::other)
}

/// [`remove`] against any key/value store.
pub fn remove_from(storage: &dyn KeyValueStore, path: &Path) -> std::io::Result<()> {
    storage
        .remove(&storage_key(path))
        .map_err(std::io::Error::other)
}

// ── The browser bridge: the only part no test here can reach ───────────────

#[cfg(target_arch = "wasm32")]
struct LocalStorage(web_sys::Storage);

#[cfg(target_arch = "wasm32")]
impl KeyValueStore for LocalStorage {
    fn get(&self, key: &str) -> Result<Option<String>, String> {
        self.0
            .get_item(key)
            .map_err(|error| format!("localStorage refused the read: {error:?}"))
    }
    fn set(&self, key: &str, value: &str) -> Result<(), String> {
        self.0
            .set_item(key, value)
            .map_err(|error| format!("localStorage refused the write: {error:?}"))
    }
    fn remove(&self, key: &str) -> Result<(), String> {
        self.0
            .remove_item(key)
            .map_err(|error| format!("localStorage refused the removal: {error:?}"))
    }
}

/// Every step can fail. There may be no `window` (a worker); `local_storage()`
/// returns `Err` when site data is blocked and `Ok(None)` where there is no
/// storage. All must read as "cannot persist", not panic.
#[cfg(target_arch = "wasm32")]
fn browser_storage() -> std::io::Result<LocalStorage> {
    let window = web_sys::window()
        .ok_or_else(|| std::io::Error::other("no browser window; cannot persist"))?;
    let storage = window
        .local_storage()
        .map_err(|error| std::io::Error::other(format!("localStorage unavailable: {error:?}")))?
        .ok_or_else(|| std::io::Error::other("this context has no localStorage"))?;
    Ok(LocalStorage(storage))
}

/// `pub(crate)` so the save and settings tests can drive the same map. The
/// browser road must be tested against a key/value store, not a directory.
#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::path::PathBuf;

    #[derive(Default)]
    pub(crate) struct MapStore {
        pub(crate) entries: RefCell<HashMap<String, String>>,
        pub(crate) refuse: bool,
    }

    impl KeyValueStore for MapStore {
        fn get(&self, key: &str) -> Result<Option<String>, String> {
            if self.refuse {
                return Err("site data is blocked".to_string());
            }
            Ok(self.entries.borrow().get(key).cloned())
        }
        fn set(&self, key: &str, value: &str) -> Result<(), String> {
            if self.refuse {
                return Err("quota exceeded".to_string());
            }
            self.entries
                .borrow_mut()
                .insert(key.to_string(), value.to_string());
            Ok(())
        }
        fn remove(&self, key: &str) -> Result<(), String> {
            self.entries.borrow_mut().remove(key);
            Ok(())
        }
    }

    /// Two roots are two key prefixes, so isolation survives the flat namespace.
    ///
    /// This is why the path stays the address. If the key came from the file
    /// name only, an isolated test App and the player would share a key, and
    /// `PersistenceRoot::isolated()` would not work.
    #[test]
    fn two_persistence_roots_are_two_keys() {
        let mine = PathBuf::from("/tmp/ambition-app-state/7-0").join("ambition/settings.ron");
        let players = PathBuf::from("/home/someone/.local/share").join("ambition/settings.ron");
        assert_ne!(
            storage_key(&mine),
            storage_key(&players),
            "an isolated App must not address the player's settings"
        );
        assert!(
            storage_key(&mine).starts_with("ambition:"),
            "the origin is shared with whatever else the page stores"
        );
    }

    /// The same logical location is the same key however the path was spelled.
    #[test]
    fn the_separator_does_not_change_the_key() {
        assert_eq!(
            storage_key(Path::new("/a/b/settings.ron")),
            storage_key(Path::new("/a//b/settings.ron")),
            "a doubled separator names the same place"
        );
    }

    /// A stored document reads back; a missing one is `NotFound`, not an error.
    ///
    /// The kind is the contract. `load_save` reads `NotFound` as "fresh sandbox"
    /// and every other error as "do not write over it".
    #[test]
    fn a_missing_key_is_not_found_and_a_stored_one_round_trips() {
        let store = MapStore::default();
        let path = Path::new("/root/ambition/save.ron");

        let missing = read_from(&store, path).expect_err("nothing is stored yet");
        assert_eq!(
            missing.kind(),
            std::io::ErrorKind::NotFound,
            "a first run must read as NotFound, or the save road takes the \
             preserve branch and never writes"
        );

        write_into(&store, path, "(hello: 1)").expect("the store accepted it");
        assert_eq!(
            read_from(&store, path).expect("it is there now"),
            "(hello: 1)"
        );
    }

    /// A store that refuses the READ is not an absence.
    ///
    /// If a blocked `localStorage` read returned `NotFound`, `load_save` would
    /// start fresh, `SaveFileWritable` would stay true, and the first autosave
    /// would overwrite an existing save.
    #[test]
    fn a_refused_read_is_not_not_found() {
        let store = MapStore {
            refuse: true,
            ..Default::default()
        };
        let error =
            read_from(&store, Path::new("/root/save.ron")).expect_err("the store refused the read");
        assert_ne!(
            error.kind(),
            std::io::ErrorKind::NotFound,
            "a refusal must not read as absence; absence is the answer that \
             licenses writing over the key"
        );
        assert!(
            error.to_string().contains("site data is blocked"),
            "the reason has to survive to the log line: {error}"
        );
    }

    /// A browser that refuses the write is an IO error, not a panic.
    ///
    /// Premise guard for the test above: a store that silently dropped every
    /// write would still pass a round-trip test against a store that never fails.
    #[test]
    fn a_refused_write_surfaces_as_an_error() {
        let store = MapStore {
            refuse: true,
            ..Default::default()
        };
        let error = write_into(&store, Path::new("/root/settings.ron"), "body")
            .expect_err("the store refused");
        assert_ne!(
            error.kind(),
            std::io::ErrorKind::NotFound,
            "a refusal is not an absence; the caller logs it and plays on"
        );
        assert!(
            error.to_string().contains("quota"),
            "the reason has to survive to the log line: {error}"
        );
    }
}
