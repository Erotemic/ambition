//! Enumerate files from one accepted sprite publication.
//!
//! Build scripts and runtime code use the same selection reader. Logical
//! paths name assets. Physical paths name immutable bundle members.

use std::path::{Path, PathBuf};

/// The quality-tier folders beside `sprites/`, and the marker a key carries
/// for each.
pub const TIERS: [(&str, &str); 3] = [("sprites_0_5x", "0_5x"), ("sprites_0_25x", "0_25x"), ("sprites_potato", "potato")];

#[path = "../../ambition_asset_manager/src/art_publication.rs"]
mod publication;
pub use publication::Selection;

pub fn selection(owner: &Path) -> Result<Selection, String> {
    Selection::read(&publication::store_for_assets(&owner.join("assets")))
}

/// The publication directory contains the atomic selection file.
pub fn sprite_dirs(owner: &Path) -> Vec<PathBuf> {
    vec![publication::store_for_assets(&owner.join("assets"))]
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

/// The files of one kind in the tree, sorted by key, and a refusal for each
/// key that two files give.
///
/// ⛔ A key that two files give is refused, and the refusal names both files.
/// The companion scan kept the first file of a key, and the sheet scan kept
/// both: the order of the scan chose which file a key was read from. The scan
/// itself does not stop: a running game reads the tree again on each edit,
/// and its watch must refuse the table and keep the running sheets. Each
/// reader decides with [`KeyedFiles::one_for_each_key`].
pub struct KeyedFiles {
    /// Every file found, sorted by key. A key that two files give is here
    /// twice, so a watch sees the second file come and go.
    pub files: Vec<(String, PathBuf)>,
    /// `(key, why)` for each key that two files give, with both files named.
    pub refused: Vec<(String, String)>,
}

impl KeyedFiles {
    /// The files, one for each key, or the refusal of each key that two
    /// files give.
    pub fn one_for_each_key(self) -> Result<Vec<(String, PathBuf)>, Vec<(String, String)>> {
        if self.refused.is_empty() {
            Ok(self.files)
        } else {
            Err(self.refused)
        }
    }

    /// The files, for a reader that has no table to keep (the build, a
    /// test): it stops at a refusal.
    pub fn expect_one_for_each_key(self) -> Vec<(String, PathBuf)> {
        self.one_for_each_key().unwrap_or_else(|refused| {
            let each: Vec<String> = refused.iter().map(|(key, why)| format!("{key}: {why}")).collect();
            panic!("the publisher must write one file for each key: {}", each.join("; "))
        })
    }
}

fn keyed(mut entries: Vec<(String, PathBuf)>, what: &str) -> KeyedFiles {
    entries.sort();
    let refused = entries
        .windows(2)
        .filter(|pair| pair[0].0 == pair[1].0)
        .map(|pair| {
            (
                format!("{what} key `{}`", pair[0].0),
                format!("two published files give it: {} and {}", pair[0].1.display(), pair[1].1.display()),
            )
        })
        .collect();
    KeyedFiles { files: entries, refused }
}

/// Every `*_spritesheet.ron`, keyed by [`sheet_key`], sorted by key. A key
/// that two files give is refused.
pub fn sheet_manifests(owner: &Path) -> KeyedFiles {
    match selection(owner) {
        Ok(selected) => sheet_manifests_in(&selected),
        Err(error) => KeyedFiles { files: Vec::new(), refused: vec![("publication".into(), error)] },
    }
}

pub fn sheet_manifests_in(selected: &Selection) -> KeyedFiles {
    keyed(selected.entries().filter(|(logical, _)| logical.ends_with("_spritesheet.ron"))
        .map(|(logical, path)| {
            let stem = Path::new(logical).file_name().unwrap().to_str().unwrap().trim_end_matches("_spritesheet.ron");
            (sheet_key(stem, Path::new(logical)), path)
        }).collect(), "sprite sheet")
}

pub fn companions(owner: &Path, suffix: &str, tiers: bool) -> KeyedFiles {
    match selection(owner) {
        Ok(selected) => companions_in(&selected, suffix, tiers),
        Err(error) => KeyedFiles { files: Vec::new(), refused: vec![("publication".into(), error)] },
    }
}

pub fn companions_in(selected: &Selection, suffix: &str, tiers: bool) -> KeyedFiles {
    keyed(selected.entries().filter(|(logical, _)| logical.ends_with(suffix) && (tiers || logical.starts_with("sprites/")))
        .map(|(logical, path)| {
            let stem = Path::new(logical).file_name().unwrap().to_str().unwrap().trim_end_matches(suffix);
            (sheet_key(stem, Path::new(logical)), path)
        }).collect(), suffix)
}

pub fn portrait_manifests(owner: &Path) -> Vec<(String, PathBuf)> {
    selection(owner).map(|selected| portraits_in(&selected)).unwrap_or_default()
}

pub fn portraits_in(selected: &Selection) -> Vec<(String, PathBuf)> {
    selected.entries().filter(|(logical, _)| logical.ends_with("_portraits.ron"))
        .map(|(logical, path)| (logical.to_owned(), path)).collect()
}

pub fn pack_catalogs(owner: &Path) -> Vec<(String, PathBuf)> {
    selection(owner).map(|selected| packs_in(&selected)).unwrap_or_default()
}

pub fn packs_in(selected: &Selection) -> Vec<(String, PathBuf)> {
    selected.entries().filter(|(logical, _)| logical.starts_with("sprite_packs/") && logical.ends_with("/ultrapack.json"))
        .map(|(logical, path)| (logical.split('/').nth(1).unwrap().to_owned(), path)).collect()
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
    // The four corners of a draw's quad, from the feet: the part turned
    // (radians, clockwise with +y down) and scaled about its pivot, its pivot
    // at `at`. A table at another texel scale states its rect in its texels.
    let corners = |draw: &super::published::PublishedDraw| -> Option<[(f32, f32); 4]> {
        let part = published.parts.get(usize::from(draw.part))?;
        let scale = published.texel_scale.max(f32::EPSILON);
        let (w, h) = (part.rect.2 as f32 / scale, part.rect.3 as f32 / scale);
        let (px, py) = (part.pivot.0 / scale, part.pivot.1 / scale);
        let (c, s) = (draw.rotation.cos(), draw.rotation.sin());
        Some([(0.0, 0.0), (w, 0.0), (0.0, h), (w, h)].map(|(x, y)| {
            let (lx, ly) = ((x - px) * draw.scale.0, (y - py) * draw.scale.1);
            (draw.at.0 + c * lx - s * ly, draw.at.1 + s * lx + c * ly)
        }))
    };
    let mut clips = std::collections::BTreeMap::new();
    let mut boxes = std::collections::BTreeMap::new();
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
        let framed: Vec<lp::BakedBoxFrame> = clip
            .frames
            .iter()
            .map(|frame| {
                lp::box_frame(frame.iter().filter_map(|draw| Some((track_name(draw.track)?.as_str(), corners(draw)?))))
            })
            .collect();
        if framed.iter().flatten().any(Option::is_some) {
            any = true;
            boxes.insert(name.clone(), framed);
        }
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
        boxes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// An owner in a temporary folder with `files` (relative to `assets/`).
    fn tree(name: &str, files: &[&str]) -> PathBuf {
        let owner = std::env::temp_dir().join(format!("ambition_sprite_tree_{name}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&owner);
        for file in files {
            let path = owner.join("assets").join(file);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, "").unwrap();
        }
        let selected: std::collections::BTreeMap<_, _> = files.iter().map(|file| (file.to_string(), serde_json::json!({
            "path":format!("../assets/{file}"), "sha256":"fixture", "role":"runtime", "tier":"full"
        }))).collect();
        let store = owner.join("art_publication");
        fs::create_dir_all(&store).unwrap();
        let mut selected = selected;
        for member in selected.values_mut() {
            let relative = member["path"].as_str().unwrap().trim_start_matches("../assets/").to_owned();
            let target = store.join("files").join(&relative);
            fs::create_dir_all(target.parent().unwrap()).unwrap();
            fs::copy(owner.join("assets").join(&relative), &target).unwrap();
            member["path"] = format!("files/{relative}").into();
        }
        fs::write(store.join("published.json"), serde_json::json!({"schema":publication::SCHEMA,"generation":"fixture","files":selected}).to_string()).unwrap();
        owner
    }

    /// The refusal of a scan, which still lists each file it found.
    fn refusal(scan: KeyedFiles, files: usize) -> String {
        assert_eq!(scan.files.len(), files, "the scan lists each file it found");
        let refused = scan.one_for_each_key().expect_err("the scan refuses the tree");
        refused.iter().map(|(key, why)| format!("{key}: {why}")).collect::<Vec<_>>().join("; ")
    }

    /// Q84: each quality tier's portrait manifest is in the tree, under its
    /// own path, beside the full one.
    #[test]
    fn the_portrait_manifests_of_each_tier_are_in_the_tree() {
        let owner = tree(
            "portraits",
            &["sprites/alice_portraits.ron", "sprites_0_5x/alice_portraits.ron", "sprites_potato/alice_portraits.ron", "sprites/alice_spritesheet.ron"],
        );
        let keys: Vec<String> = portrait_manifests(&owner).into_iter().map(|(key, _)| key).collect();
        assert_eq!(keys, ["sprites/alice_portraits.ron", "sprites_0_5x/alice_portraits.ron", "sprites_potato/alice_portraits.ron"]);
    }

    /// ⭐ C1: two files of one key are refused with both paths, in each scan
    /// and in a quality tier. One file for each key is not.
    #[test]
    fn a_key_that_two_files_give_is_refused() {
        // The control: one file of each key, in `sprites/` and in a tier.
        let owner = tree("one", &["sprites/hero/hero_parts.ron", "sprites_0_5x/hero_parts.ron"]);
        let keys: Vec<String> = companions(&owner, "_parts.ron", true).expect_one_for_each_key().into_iter().map(|(key, _)| key).collect();
        assert_eq!(keys, ["hero", "hero.0_5x"]);

        for (name, files) in [
            ("two", ["sprites/a/hero_parts.ron", "sprites/b/hero_parts.ron"]),
            ("tier", ["sprites_0_5x/hero_parts.ron", "sprites_0_5x/boss/hero_parts.ron"]),
        ] {
            let owner = tree(name, &files);
            let message = refusal(companions(&owner, "_parts.ron", true), 2);
            assert!(
                files.iter().all(|file| message.contains(&owner.join("art_publication/files").join(file).display().to_string())),
                "{name}: the refusal names both files: {message}"
            );
        }

        let files = ["sprites/a/hero_spritesheet.ron", "sprites/b/hero_spritesheet.ron"];
        let owner = tree("sheets", &files);
        let message = refusal(sheet_manifests(&owner), 2);
        assert!(message.contains("`hero`"), "the sheet scan refuses it too: {message}");
    }
}
