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

/// Every `*_spritesheet.ron`, keyed by [`sheet_key`], sorted by key.
pub fn sheet_manifests(owner: &Path) -> Vec<(String, PathBuf)> {
    let mut entries: Vec<(String, PathBuf)> = sprite_dirs(owner)
        .iter()
        .flat_map(|dir| dir_and_children(dir))
        .flat_map(|dir| files_ending(&dir, "_spritesheet.ron"))
        .map(|(root, path)| (sheet_key(&root, &path), path))
        .collect();
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    entries
}

/// Every `<target><suffix>` file of `sprites/` (and, with `tiers`, of each
/// tier folder), keyed `<target>` or `<target>.<marker>`, sorted by key. A key
/// found twice keeps its first file.
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
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    entries.dedup_by(|a, b| a.0 == b.0);
    entries
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
