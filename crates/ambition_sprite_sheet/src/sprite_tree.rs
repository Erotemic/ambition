//! The published sprite tree: which files the sprite publisher wrote, and the
//! key each one is read by.
//!
//! `build.rs` and the crate use this one file (`build.rs` includes it with
//! `#[path]`), so a build that embeds the tree and a development build that
//! reads it at run time find the same files under the same keys. A second
//! scan would be a second answer to "which sheets are published".
//!
//! The tree is under the actor monolith's `assets/`: `sprites/` and its quality
//! tiers (`sprites_0_5x`, `sprites_0_25x`, `sprites_potato`), each with one
//! level of subdirectories (a boss publishes into its own), and
//! `sprite_packs/<tier>/ultrapack.json`.

use std::fs;
use std::path::{Path, PathBuf};

/// The quality-tier folders beside `sprites/`, and the marker a key carries
/// for each.
pub const TIERS: [(&str, &str); 3] = [("sprites_0_5x", "0_5x"), ("sprites_0_25x", "0_25x"), ("sprites_potato", "potato")];

/// The directories a sheet manifest can be in: `assets/sprites` and each tier.
pub fn sprite_dirs(owner: &Path) -> Vec<PathBuf> {
    std::iter::once("sprites".to_owned())
        .chain(TIERS.iter().map(|(dir, _)| (*dir).to_owned()))
        .map(|dir| owner.join("assets").join(dir))
        .collect()
}

/// `dir` and each directory directly under it.
fn dir_and_children(dir: &Path) -> Vec<PathBuf> {
    let mut dirs = vec![dir.to_path_buf()];
    if let Ok(children) = fs::read_dir(dir) {
        let mut children: Vec<PathBuf> = children.flatten().map(|entry| entry.path()).filter(|path| path.is_dir()).collect();
        children.sort();
        dirs.extend(children);
    }
    dirs
}

/// Every file in `dir` whose name ends with `suffix`, with the name before it.
fn files_ending(dir: &Path, suffix: &str) -> Vec<(String, PathBuf)> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut found: Vec<(String, PathBuf)> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter_map(|path| {
            let stem = path.file_name()?.to_str()?.strip_suffix(suffix)?.to_owned();
            Some((stem, path))
        })
        .collect();
    found.sort();
    found
}

/// The key of a sheet whose file root is `root`: the root, and the tier
/// marker of a tier folder (`<root>.<marker>`).
pub fn sheet_key(root: &str, path: &Path) -> String {
    let marker = path
        .components()
        .filter_map(|component| component.as_os_str().to_str())
        .find_map(|component| TIERS.iter().find(|(dir, _)| *dir == component).map(|(_, marker)| *marker));
    match marker {
        Some(marker) => format!("{root}.{marker}"),
        None => root.to_owned(),
    }
}

/// `entries`, sorted by key.
///
/// ⛔ A key that two files give is refused, and the refusal names both files.
/// The companion scan kept the first file of a key, and the sheet scan kept
/// both: the order of the scan chose which file a key was read from.
fn one_file_for_each_key(mut entries: Vec<(String, PathBuf)>, what: &str) -> Vec<(String, PathBuf)> {
    entries.sort();
    let collisions: Vec<String> = entries
        .windows(2)
        .filter(|pair| pair[0].0 == pair[1].0)
        .map(|pair| format!("`{}`: {} and {}", pair[0].0, pair[0].1.display(), pair[1].1.display()))
        .collect();
    assert!(
        collisions.is_empty(),
        "two published files give one {what} key; the publisher must write one: {}",
        collisions.join("; ")
    );
    entries
}

/// Every `*_spritesheet.ron`, keyed by [`sheet_key`], sorted by key. A key
/// that two files give is refused.
pub fn sheet_manifests(owner: &Path) -> Vec<(String, PathBuf)> {
    let entries: Vec<(String, PathBuf)> = sprite_dirs(owner)
        .iter()
        .flat_map(|dir| dir_and_children(dir))
        .flat_map(|dir| files_ending(&dir, "_spritesheet.ron"))
        .map(|(root, path)| (sheet_key(&root, &path), path))
        .collect();
    one_file_for_each_key(entries, "sprite sheet")
}

/// Every `<target><suffix>` file of `sprites/` (and, with `tiers`, of each
/// tier folder), keyed `<target>` or `<target>.<marker>`, sorted by key. A key
/// that two files give is refused.
pub fn companions(owner: &Path, suffix: &str, tiers: bool) -> Vec<(String, PathBuf)> {
    let mut roots = vec![(owner.join("assets/sprites"), None)];
    if tiers {
        roots.extend(TIERS.iter().map(|(dir, marker)| (owner.join("assets").join(dir), Some(*marker))));
    }
    let mut entries: Vec<(String, PathBuf)> = Vec::new();
    for (root, marker) in &roots {
        for dir in dir_and_children(root) {
            for (target, path) in files_ending(&dir, suffix) {
                let key = match marker {
                    Some(marker) => format!("{target}.{marker}"),
                    None => target,
                };
                entries.push((key, path));
            }
        }
    }
    one_file_for_each_key(entries, suffix)
}

/// Every `*_portraits.ron` under `sprites/`, at any depth, keyed by its path
/// relative to `assets/` with `/` separators, sorted by key.
pub fn portrait_manifests(owner: &Path) -> Vec<(String, PathBuf)> {
    fn walk(dir: &Path, assets: &Path, out: &mut Vec<(String, PathBuf)>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for path in entries.flatten().map(|entry| entry.path()) {
            if path.is_dir() {
                walk(&path, assets, out);
            } else if path.file_name().and_then(|name| name.to_str()).is_some_and(|name| name.ends_with("_portraits.ron")) {
                if let Ok(relative) = path.strip_prefix(assets) {
                    out.push((relative.to_string_lossy().replace('\\', "/"), path));
                }
            }
        }
    }
    let assets = owner.join("assets");
    let mut entries = Vec::new();
    walk(&assets.join("sprites"), &assets, &mut entries);
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    entries
}

/// Each quality tier's ultrapack catalog (`sprite_packs/<tier>/ultrapack.json`),
/// keyed by the tier's folder name, sorted by key.
pub fn pack_catalogs(owner: &Path) -> Vec<(String, PathBuf)> {
    let Ok(entries) = fs::read_dir(owner.join("assets/sprite_packs")) else {
        return Vec::new();
    };
    let mut tiers: Vec<(String, PathBuf)> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter_map(|tier| {
            let catalog = tier.join("ultrapack.json");
            let name = tier.file_name()?.to_str()?.to_owned();
            (tier.is_dir() && catalog.is_file()).then_some((name, catalog))
        })
        .collect();
    tiers.sort_by(|a, b| a.0.cmp(&b.0));
    tiers
}

/// Read a published part flipbook with the schema the crate reads it with.
pub fn read_part_flipbook(text: &str) -> Result<super::published::Published, ron::error::SpannedError> {
    ron::Options::default()
        .with_default_extension(ron::extensions::Extensions::IMPLICIT_SOME)
        .from_str(text)
}

/// The landmark table of one published part flipbook: the place of each
/// landmark track in each frame of each clip. `None` when the flipbook tracks
/// no landmark, so that such a target has no table.
///
/// A mirror row (`<row>~mirrored`) is not read: a body that faces the other
/// way mirrors the row's points about its feet.
pub fn landmarks_of(published: &super::published::Published) -> Option<super::landmarks_published::BakedLandmarks> {
    use super::landmarks_published as lp;
    let track_name = |track: Option<u16>| track.and_then(|track| published.tracks.get(usize::from(track)));
    let mut clips = std::collections::BTreeMap::new();
    let mut any = false;
    for (name, clip) in &published.clips {
        if name.contains('~') {
            continue;
        }
        let frames: Vec<lp::BakedLandmarkFrame> = clip
            .frames
            .iter()
            .map(|frame| lp::landmark_frame(frame.iter().filter_map(|draw| track_name(draw.track).map(|name| (name.as_str(), draw.at)))))
            .collect();
        any |= frames.iter().flatten().any(Option::is_some);
        clips.insert(
            name.clone(),
            lp::BakedLandmarkClip {
                frame_duration_s: clip.frame_duration_s,
                frames,
            },
        );
    }
    any.then_some(lp::BakedLandmarks {
        frame_height: published.frame_size.1,
        clips,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An owner in a temporary folder with `files` (relative to `assets/`).
    fn tree(name: &str, files: &[&str]) -> PathBuf {
        let owner = std::env::temp_dir().join(format!("ambition_sprite_tree_{name}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&owner);
        for file in files {
            let path = owner.join("assets").join(file);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, "").unwrap();
        }
        owner
    }

    fn refusal(scan: impl FnOnce() -> Vec<(String, PathBuf)> + std::panic::UnwindSafe) -> String {
        let payload = std::panic::catch_unwind(scan).expect_err("the scan refuses the tree");
        payload.downcast_ref::<String>().cloned().unwrap_or_default()
    }

    /// ⭐ C1: two files of one key are refused with both paths, in each scan
    /// and in a quality tier. One file for each key is not.
    #[test]
    fn a_key_that_two_files_give_is_refused() {
        // The control: one file of each key, in `sprites/` and in a tier.
        let owner = tree("one", &["sprites/hero/hero_parts.ron", "sprites_0_5x/hero_parts.ron"]);
        let keys: Vec<String> = companions(&owner, "_parts.ron", true).into_iter().map(|(key, _)| key).collect();
        assert_eq!(keys, ["hero", "hero.0_5x"]);

        for (name, files) in [
            ("two", ["sprites/a/hero_parts.ron", "sprites/b/hero_parts.ron"]),
            ("tier", ["sprites_0_5x/hero_parts.ron", "sprites_0_5x/boss/hero_parts.ron"]),
        ] {
            let owner = tree(name, &files);
            let message = refusal(|| companions(&owner, "_parts.ron", true));
            assert!(
                files.iter().all(|file| message.contains(&owner.join("assets").join(file).display().to_string())),
                "{name}: the refusal names both files: {message}"
            );
        }

        let files = ["sprites/a/hero_spritesheet.ron", "sprites/b/hero_spritesheet.ron"];
        let owner = tree("sheets", &files);
        let message = refusal(|| sheet_manifests(&owner));
        assert!(message.contains("`hero`"), "the sheet scan refuses it too: {message}");
    }
}
