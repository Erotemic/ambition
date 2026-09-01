//! Where a persisted file's BYTES actually go, per platform.
//!
//! ⛔⛔ THE BROWSER HAD NO PERSISTENCE AT ALL, SILENTLY. Until 2026-08-31 the
//! four systems that read and write settings and saves —
//! `load_settings_at_startup`, `save_settings_on_change`, `load_save_at_startup`
//! and `autosave_sandbox_save` — were each `#[cfg(not(target_arch = "wasm32"))]`.
//! On the web build they did not exist, so every setting a player changed was
//! forgotten on reload and no save was ever written. Nothing reported that,
//! because a system that is not compiled cannot warn.
//!
//! ⭐ THE PATH STAYS THE ADDRESS ON BOTH PLATFORMS. `PersistenceRoot` is still a
//! `PathBuf` and every caller still hands this module a `&Path`; only the last
//! step differs. That keeps `PersistenceRoot::isolated()` meaningful on the web
//! — two roots are two key prefixes exactly as they are two directories — and it
//! keeps the 60-odd call sites that treat the root as a path untouched.
//!
//! ⭐ POLICY AND BRIDGE ARE SPLIT, for the same reason `render_recovery` splits
//! them: the browser call cannot run in this test binary, so everything that
//! CAN be decided without a browser is decided in [`storage_key`] and in
//! [`read_from`] / [`write_into`], which are tested against an in-memory map.
//! What is left unverified is four lines of `web_sys`.

use std::path::Path;

/// The key a path becomes in a flat key/value store.
///
/// ⛔ A KEY/VALUE STORE HAS NO DIRECTORIES. `localStorage` is one flat namespace
/// per origin, so the whole path — root and all — has to survive into the key,
/// or two `PersistenceRoot`s would collide and an isolated App would read the
/// player's settings.
///
/// ⭐ THE SEPARATOR IS NORMALISED. A path built on one platform and a key read on
/// another must agree, and `Path::display` would emit `\` on Windows for the same
/// logical location. Everything becomes `/`.
///
/// The `ambition:` prefix namespaces us against anything else on the origin.
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
/// ⚠ THE `NotFound` KIND IS LOad-BEARING. Both callers distinguish "no file yet,
/// start fresh" from "a file exists and could not be read, do NOT overwrite it"
/// by matching on exactly this kind — see `load_save`, whose whole
/// `LoadedSave::preserve` road hangs off it.
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
        // ⭐ THE NATIVE ROAD IS UNCHANGED, INCLUDING THE TEMP-FILE DANCE. The
        // callers own it (they have different recovery rules for a failed
        // rename), so this is only the plain write they build on.
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

// ⛔⛔ THERE IS DELIBERATELY NO `exists`. The first draft had one, and both
// callers used it as a PREFLIGHT: ask whether the document is there, then read
// it. That shape has two defects and shipped both.
//
// The first is that a preflight is easy to write against the WRONG store —
// `load_save_at_startup` kept `path.exists()`, the NATIVE filesystem, after its
// read had moved to this module. On the browser that check answers "no" for a
// save that is sitting in `localStorage`, so a reload started fresh and the
// first autosave replaced the player's progress with defaults.
//
// The second is that a bool cannot carry WHY. `exists` returning `false` folded
// "nothing stored" together with "storage is blocked", and the save road
// distinguishes those for a living: one means write, the other means do NOT.
//
// ⇒ ONE READ ANSWERS BOTH QUESTIONS. `read` returns `NotFound` for absence and
// any other kind for a refusal, and the callers carry "was a document there" in
// the value they already return. See `LoadedSave::present`.

// ── The key/value policy, decided without a browser ────────────────────────
//
// ⭐ THESE THREE ARE COMPILED ON EVERY PLATFORM so the tests below actually run.
// A `#[cfg(target_arch = "wasm32")]` on them would make the guard vacuous on the
// machine that runs the suite, which is the shape this repository keeps catching.

/// A flat key/value store: `localStorage`, or a map in a test.
pub trait KeyValueStore {
    /// The value at `key`; `Ok(None)` when nothing is stored there.
    ///
    /// ⛔ `Ok(None)` MEANS ABSENCE AND NOTHING ELSE. A browser can refuse the
    /// read outright — site data blocked, a cross-origin frame — and that is
    /// `Err`. Collapsing the two is how a save gets overwritten: absence is the
    /// one answer that licenses a write.
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
    // ⚠ A BROWSER CAN REFUSE THIS. `localStorage` has a quota (a few MB) and
    // throws when it is exceeded or when the origin has site data blocked. That
    // surfaces as an ordinary IO error, which every caller already handles by
    // logging and carrying on rather than by losing the session.
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

/// ⚠ EVERY STEP HERE IS GENUINELY FALLIBLE. There may be no `window` (a worker),
/// and `local_storage()` itself returns `Err` when the origin has site data
/// blocked and `Ok(None)` in contexts that have no storage — three different
/// "no" answers, all of which must read as "cannot persist" rather than as a
/// panic in a player's browser.
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

/// ⭐ `pub(crate)` SO THE SAVE AND SETTINGS TESTS CAN DRIVE THE SAME MAP. Their
/// browser road has to be exercised against a key/value store, not against
/// another temporary directory — a filesystem test cannot fail the way the
/// browser did.
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

    impl MapStore {
        /// Seed the store the way a previous browser session would have.
        pub(crate) fn with(path: &Path, body: &str) -> Self {
            let store = Self::default();
            write_into(&store, path, body).expect("a fresh map accepts a write");
            store
        }
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
    /// ⛔⛔ THIS IS THE WHOLE REASON THE PATH STAYS THE ADDRESS. `localStorage`
    /// has no directories; if the key were built from the FILE name the settings
    /// of an isolated test App and of the player would be the same key, and the
    /// F spike rejected Bevy's `SettingsStore` for exactly this — losing
    /// `PersistenceRoot::isolated()`.
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
    /// ⛔ THE KIND IS THE CONTRACT. `load_save` reads `NotFound` as "fresh
    /// sandbox" and every OTHER error as "a save exists and I could not read it,
    /// so do NOT write over it". Collapsing the two loses a player's file.
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
    /// ⛔⛔ THIS IS THE ARM THAT DECIDES WHETHER A PLAYER KEEPS THEIR SAVE. If a
    /// blocked `localStorage` read came back as `NotFound`, `load_save` would
    /// take its `fresh()` road, `SaveFileWritable` would stay true, and the
    /// first autosave would write defaults over a save that was there all along.
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
    /// Premise guard for the arm above: without this, a store that silently
    /// dropped every write would still pass a round-trip test written against a
    /// store that never fails.
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
