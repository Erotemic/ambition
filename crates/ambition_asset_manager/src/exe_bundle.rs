//! The asset tree inside the executable: one file to give a player.
//!
//! A packaged game is its executable with the asset tree written after it
//! (`scripts/package_single_exe.py`). The operating system loads the program
//! from the front of the file and ignores what follows, on Linux and on
//! Windows. The game opens its own file, reads the footer at its end, and
//! serves each asset from the bytes between.
//!
//! ```text
//! [ the program ][ payload: each file's bytes ][ index ][ footer, 32 bytes ]
//!
//! index:  u32 count, then for each file:
//!         u32 path length, the path (UTF-8, `/` between parts),
//!         u64 offset in the payload, u64 length
//! footer: u64 payload start, u64 index start, u64 index length, MAGIC
//! ```
//!
//! Each number is little-endian. The tree is the one flat `assets/` tree a
//! packaged build has (`scripts/package_asset_guard.py compose`), so the
//! default source and the `game://` source are the same reader.
//!
//! A file with no footer is a development build: [`ExeBundle::open`] answers
//! `None` and the game reads its asset roots from the disk as before.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// The last 8 bytes of an executable that carries its assets. The digits are
/// the version of the layout.
pub const FOOTER_MAGIC: [u8; 8] = *b"AMBNDL01";

/// The length of the footer.
pub const FOOTER_LEN: u64 = 32;

/// The asset tree inside one executable file.
pub struct ExeBundle {
    file: PathBuf,
    payload_start: u64,
    /// Path to (offset in the payload, length).
    entries: BTreeMap<String, (u64, u64)>,
    /// Each directory that holds a file, with no `/` at its end. The root is
    /// the empty string.
    directories: BTreeSet<String>,
}

/// The file and the number of entries. A bundle holds some thousand paths,
/// and each catalog that prints itself would print them all.
impl std::fmt::Debug for ExeBundle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExeBundle")
            .field("file", &self.file)
            .field("files", &self.entries.len())
            .finish()
    }
}

/// An asset path as the index writes it: parts joined by `/`, with no `.`
/// part, and each `..` part taken with the part before it. A Windows path
/// (`\`) and a path relative to a world file (`worlds/../sprites/a.png`) come
/// to the same key.
pub fn normalize(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for part in path.split(['/', '\\']) {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            part => parts.push(part),
        }
    }
    parts.join("/")
}

fn invalid(file: &Path, what: impl std::fmt::Display) -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        format!("the asset bundle of `{}` is damaged: {what}", file.display()),
    )
}

impl ExeBundle {
    /// The bundle at the end of `file`. `Ok(None)`: the file has no footer,
    /// so it carries no bundle. An error: it has a footer and the bundle is
    /// damaged (a download that was cut carries no footer; a file that was
    /// changed can).
    pub fn open(file: &Path) -> std::io::Result<Option<Self>> {
        let mut reader = std::fs::File::open(file)?;
        let file_len = reader.metadata()?.len();
        if file_len < FOOTER_LEN {
            return Ok(None);
        }
        let mut footer = [0u8; FOOTER_LEN as usize];
        reader.seek(SeekFrom::Start(file_len - FOOTER_LEN))?;
        reader.read_exact(&mut footer)?;
        if footer[24..] != FOOTER_MAGIC {
            return Ok(None);
        }
        let number = |at: usize| u64::from_le_bytes(footer[at..at + 8].try_into().expect("8 bytes"));
        let (payload_start, index_start, index_len) = (number(0), number(8), number(16));
        if payload_start > index_start || index_start.checked_add(index_len) != Some(file_len - FOOTER_LEN) {
            return Err(invalid(file, "its footer does not describe this file"));
        }
        let payload_len = index_start - payload_start;
        let mut index = vec![0u8; index_len as usize];
        reader.seek(SeekFrom::Start(index_start))?;
        reader.read_exact(&mut index)?;

        let mut at = 0usize;
        let mut take = |len: usize| -> std::io::Result<&[u8]> {
            let bytes = index.get(at..at + len).ok_or_else(|| invalid(file, "its index is cut short"))?;
            at += len;
            Ok(bytes)
        };
        let count = u32::from_le_bytes(take(4)?.try_into().expect("4 bytes"));
        let mut entries = BTreeMap::new();
        let mut directories = BTreeSet::from([String::new()]);
        for _ in 0..count {
            let path_len = u32::from_le_bytes(take(4)?.try_into().expect("4 bytes")) as usize;
            let path = std::str::from_utf8(take(path_len)?)
                .map_err(|_| invalid(file, "a path in its index is not UTF-8"))?
                .to_string();
            let offset = u64::from_le_bytes(take(8)?.try_into().expect("8 bytes"));
            let len = u64::from_le_bytes(take(8)?.try_into().expect("8 bytes"));
            if offset.checked_add(len).is_none_or(|end| end > payload_len) {
                return Err(invalid(file, format!("`{path}` is outside the payload")));
            }
            let mut directory = path.as_str();
            while let Some((parent, _)) = directory.rsplit_once('/') {
                directories.insert(parent.to_string());
                directory = parent;
            }
            entries.insert(path, (offset, len));
        }
        Ok(Some(Self {
            file: file.to_path_buf(),
            payload_start,
            entries,
            directories,
        }))
    }

    /// The bundle of the program that is running, read one time. `None` for a
    /// development build, which carries none.
    ///
    /// A damaged bundle is `None` also, with one line on standard error: the
    /// game then looks for its assets on the disk, and says what it did not
    /// find.
    pub fn of_running_exe() -> Option<&'static ExeBundle> {
        static BUNDLE: OnceLock<Option<ExeBundle>> = OnceLock::new();
        BUNDLE
            .get_or_init(|| {
                let exe = std::env::current_exe().ok()?;
                match Self::open(&exe) {
                    Ok(bundle) => bundle,
                    Err(error) => {
                        eprintln!("{error}");
                        None
                    }
                }
            })
            .as_ref()
    }

    /// The executable that holds the bundle.
    pub fn file(&self) -> &Path {
        &self.file
    }

    /// The number of files in the bundle.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Each path in the bundle, in order.
    pub fn paths(&self) -> impl Iterator<Item = &str> {
        self.entries.keys().map(String::as_str)
    }

    /// Whether the bundle holds the file `path`.
    pub fn contains(&self, path: &str) -> bool {
        self.entries.contains_key(&normalize(path))
    }

    /// Whether `path` is a directory of the bundle: a file is under it.
    pub fn is_directory(&self, path: &str) -> bool {
        self.directories.contains(&normalize(path))
    }

    /// The files and the directories that are in the directory `path`, as
    /// paths from the root of the bundle. `None`: not a directory.
    pub fn children(&self, path: &str) -> Option<Vec<String>> {
        let directory = normalize(path);
        if !self.directories.contains(&directory) {
            return None;
        }
        let is_child = |candidate: &str| match candidate.rsplit_once('/') {
            Some((parent, _)) => parent == directory,
            None => directory.is_empty() && !candidate.is_empty(),
        };
        let mut children: BTreeSet<&str> = BTreeSet::new();
        children.extend(self.entries.keys().map(String::as_str).filter(|path| is_child(path)));
        children.extend(self.directories.iter().map(String::as_str).filter(|path| is_child(path)));
        Some(children.into_iter().map(str::to_string).collect())
    }

    /// The bytes of the file `path`. `Ok(None)`: the bundle does not hold it.
    pub fn read(&self, path: &str) -> std::io::Result<Option<Vec<u8>>> {
        let Some(&(offset, len)) = self.entries.get(&normalize(path)) else {
            return Ok(None);
        };
        // One open for each read: reads come from many threads, and a shared
        // handle would need a lock around its seek.
        let mut reader = std::fs::File::open(&self.file)?;
        reader.seek(SeekFrom::Start(self.payload_start + offset))?;
        let mut bytes = vec![0u8; len as usize];
        reader.read_exact(&mut bytes)?;
        Ok(Some(bytes))
    }
}

#[cfg(feature = "bevy")]
mod source {
    use super::ExeBundle;
    use bevy::asset::io::{AssetReader, AssetReaderError, AssetSourceBuilder, PathStream, VecReader};
    use std::path::{Path, PathBuf};

    /// Reads each asset from the bundle of the running program.
    struct ExeBundleReader(&'static ExeBundle);

    fn key(path: &Path) -> String {
        path.to_string_lossy().into_owned()
    }

    impl AssetReader for ExeBundleReader {
        async fn read<'a>(&'a self, path: &'a Path) -> Result<VecReader, AssetReaderError> {
            match self.0.read(&key(path)) {
                Ok(Some(bytes)) => Ok(VecReader::new(bytes)),
                Ok(None) => Err(AssetReaderError::NotFound(path.to_path_buf())),
                Err(error) => Err(AssetReaderError::Io(std::sync::Arc::new(error))),
            }
        }

        /// A bundle holds no `.meta` file: each asset loads with the defaults
        /// of its loader, as it does from a tree that has none.
        async fn read_meta<'a>(&'a self, path: &'a Path) -> Result<VecReader, AssetReaderError> {
            Err(AssetReaderError::NotFound(path.to_path_buf()))
        }

        async fn read_directory<'a>(&'a self, path: &'a Path) -> Result<Box<PathStream>, AssetReaderError> {
            let children = self
                .0
                .children(&key(path))
                .ok_or_else(|| AssetReaderError::NotFound(path.to_path_buf()))?;
            Ok(Box::new(futures_lite::stream::iter(
                children.into_iter().map(PathBuf::from).collect::<Vec<_>>(),
            )))
        }

        async fn is_directory<'a>(&'a self, path: &'a Path) -> Result<bool, AssetReaderError> {
            Ok(self.0.is_directory(&key(path)))
        }
    }

    /// An asset source that reads from `bundle`. It has no writer and no
    /// watcher: a packaged game does not change its assets.
    pub fn exe_bundle_asset_source(bundle: &'static ExeBundle) -> AssetSourceBuilder {
        AssetSourceBuilder::new(move || Box::new(ExeBundleReader(bundle)))
    }
}

#[cfg(feature = "bevy")]
pub use source::exe_bundle_asset_source;

/// Write `files` after the bytes of `program` as a bundle, to `out`. For the
/// tests of this crate; `scripts/package_single_exe.py` writes the same
/// layout for a real package.
#[cfg(test)]
pub(crate) fn write_bundle_for_test(program: &[u8], files: &[(&str, &[u8])], out: &Path) {
    let mut bytes = program.to_vec();
    let payload_start = bytes.len() as u64;
    let mut index = (files.len() as u32).to_le_bytes().to_vec();
    let mut offset = 0u64;
    for (path, content) in files {
        bytes.extend_from_slice(content);
        index.extend_from_slice(&(path.len() as u32).to_le_bytes());
        index.extend_from_slice(path.as_bytes());
        index.extend_from_slice(&offset.to_le_bytes());
        index.extend_from_slice(&(content.len() as u64).to_le_bytes());
        offset += content.len() as u64;
    }
    let index_start = bytes.len() as u64;
    bytes.extend_from_slice(&index);
    bytes.extend_from_slice(&payload_start.to_le_bytes());
    bytes.extend_from_slice(&index_start.to_le_bytes());
    bytes.extend_from_slice(&(index.len() as u64).to_le_bytes());
    bytes.extend_from_slice(&FOOTER_MAGIC);
    std::fs::write(out, bytes).expect("the bundle is written");
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROGRAM: &[u8] = b"\x7fELF this stands for the program";
    const FILES: &[(&str, &[u8])] = &[
        ("sprites/robot.png", b"robot bytes"),
        ("sprites/props/crate.png", b"crate"),
        ("worlds/sandbox.ldtk", b"{}"),
        ("fonts/empty.ttf", b""),
    ];

    fn bundle() -> (tempfile::TempDir, ExeBundle) {
        let dir = tempfile::tempdir().expect("a temp dir");
        let exe = dir.path().join("game");
        write_bundle_for_test(PROGRAM, FILES, &exe);
        let bundle = ExeBundle::open(&exe).expect("the file is read").expect("the file carries a bundle");
        (dir, bundle)
    }

    /// The packager (`scripts/package_single_exe.py`) writes the layout and
    /// this module reads it: two programs, one layout. This file is the bytes
    /// the packager writes for a small tree, and the Python arm
    /// `test_the_writer_makes_the_bytes_the_game_reads` proves that it is. A
    /// change to the layout on one side fails one of the two arms.
    #[test]
    fn the_packager_s_bundle_reads_back() {
        let dir = tempfile::tempdir().expect("a temp dir");
        let exe = dir.path().join("game");
        std::fs::write(&exe, include_bytes!("../tests/data/exe_bundle_golden.bin")).expect("the file is written");
        let bundle = ExeBundle::open(&exe).expect("the file is read").expect("the file carries a bundle");
        assert_eq!(
            bundle.paths().collect::<Vec<_>>(),
            ["audio/music/a.txt", "sprites/b.bin", "worlds/empty.ldtk"]
        );
        assert_eq!(bundle.read("audio/music/a.txt").unwrap().as_deref(), Some(&b"first file\n"[..]));
        assert_eq!(bundle.read("sprites/b.bin").unwrap().as_deref(), Some(&[0u8, 1, 2, 3, 4, 5, 6][..]));
        assert_eq!(bundle.read("worlds/empty.ldtk").unwrap().as_deref(), Some(&b""[..]));
    }

    #[test]
    fn each_file_reads_back_with_its_own_bytes() {
        let (_dir, bundle) = bundle();
        assert_eq!(bundle.len(), FILES.len());
        for (path, content) in FILES {
            assert!(bundle.contains(path), "{path}");
            assert_eq!(bundle.read(path).unwrap().as_deref(), Some(*content), "{path}");
        }
        // The control: a path the bundle does not hold.
        assert!(!bundle.contains("sprites/none.png"));
        assert_eq!(bundle.read("sprites/none.png").unwrap(), None);
    }

    /// A Windows path, and a path that a world file names relative to its own
    /// directory, find the same file.
    #[test]
    fn a_path_is_found_however_it_is_spelled() {
        let (_dir, bundle) = bundle();
        for spelled in [
            "sprites\\robot.png",
            "./sprites/robot.png",
            "worlds/../sprites/robot.png",
            "worlds\\..\\sprites\\robot.png",
            "sprites//robot.png",
        ] {
            assert_eq!(bundle.read(spelled).unwrap().as_deref(), Some(&b"robot bytes"[..]), "{spelled}");
        }
        assert_eq!(normalize("a/b/../../c"), "c");
        // A path that climbs above the root stays in the bundle.
        assert_eq!(normalize("../../sprites/robot.png"), "sprites/robot.png");
    }

    #[test]
    fn directories_are_the_parents_of_the_files() {
        let (_dir, bundle) = bundle();
        assert_eq!(
            bundle.children("").unwrap(),
            vec!["fonts".to_string(), "sprites".to_string(), "worlds".to_string()]
        );
        assert_eq!(
            bundle.children("sprites").unwrap(),
            vec!["sprites/props".to_string(), "sprites/robot.png".to_string()]
        );
        assert!(bundle.is_directory("sprites/props") && !bundle.is_directory("sprites/robot.png"));
        assert_eq!(bundle.children("sprites/robot.png"), None);
        assert_eq!(bundle.children("music"), None);
    }

    /// A program with nothing after it carries no bundle, and that is not an
    /// error: it is each development build.
    #[test]
    fn a_plain_program_carries_no_bundle() {
        let dir = tempfile::tempdir().expect("a temp dir");
        let exe = dir.path().join("game");
        for program in [&b""[..], &b"short"[..], PROGRAM, &[0u8; 4096][..]] {
            std::fs::write(&exe, program).unwrap();
            assert!(ExeBundle::open(&exe).unwrap().is_none(), "{} bytes", program.len());
        }
    }

    /// A bundle that was changed after it was written is an error that names
    /// the file, not a bundle that serves wrong bytes.
    #[test]
    fn a_damaged_bundle_is_refused() {
        let dir = tempfile::tempdir().expect("a temp dir");
        let exe = dir.path().join("game");
        write_bundle_for_test(PROGRAM, FILES, &exe);
        let good = std::fs::read(&exe).unwrap();
        let footer = good.len() - FOOTER_LEN as usize;

        // The footer points past the file.
        let mut moved = good.clone();
        moved[footer + 8..footer + 16].copy_from_slice(&u64::MAX.to_le_bytes());
        // A byte is taken out of the payload: the footer is of another file.
        let mut shorter = good.clone();
        shorter.remove(PROGRAM.len() + 1);
        // An entry is longer than the payload.
        let mut long_entry = good.clone();
        let index_start = u64::from_le_bytes(good[footer + 8..footer + 16].try_into().unwrap()) as usize;
        let first_len = index_start + 4 + 4 + FILES[0].0.len() + 8;
        long_entry[first_len..first_len + 8].copy_from_slice(&1_000_000u64.to_le_bytes());

        for (what, bytes) in [("footer", moved), ("shorter", shorter), ("entry", long_entry)] {
            std::fs::write(&exe, bytes).unwrap();
            let error = ExeBundle::open(&exe).expect_err(what);
            assert!(error.to_string().contains("is damaged"), "{what}: {error}");
        }
        // The control: the file as it was written opens.
        std::fs::write(&exe, &good).unwrap();
        assert!(ExeBundle::open(&exe).unwrap().is_some());
    }

    #[cfg(feature = "bevy")]
    #[test]
    fn the_asset_reader_serves_the_bundle() {
        use bevy::asset::io::{AssetReader as _, AssetReaderError, AssetSourceId, Reader as _};
        use futures_lite::{future::block_on, StreamExt as _};

        let (_dir, bundle) = bundle();
        let bundle: &'static ExeBundle = Box::leak(Box::new(bundle));
        let source = exe_bundle_asset_source(bundle).build(AssetSourceId::Default, false, false);
        let reader = source.reader();
        block_on(async {
            let mut bytes = Vec::new();
            reader.read(Path::new("worlds/../sprites/robot.png")).await.unwrap().read_to_end(&mut bytes).await.unwrap();
            assert_eq!(bytes, b"robot bytes");
            assert!(matches!(
                reader.read(Path::new("sprites/none.png")).await,
                Err(AssetReaderError::NotFound(_))
            ));
            assert!(matches!(
                reader.read_meta(Path::new("sprites/robot.png")).await,
                Err(AssetReaderError::NotFound(_))
            ));
            assert!(reader.is_directory(Path::new("sprites")).await.unwrap());
            let listed: Vec<_> = reader.read_directory(Path::new("sprites")).await.unwrap().collect().await;
            assert_eq!(listed, vec![PathBuf::from("sprites/props"), PathBuf::from("sprites/robot.png")]);
        });
    }
}
