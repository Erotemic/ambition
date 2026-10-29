//! Build one sprite generation from explicitly selected libraries.

use ambition_asset_manager::content_library::{ContentLibraries, LibrarySnapshot, LibraryFileData};
use crate::{baked_sheet_rons::SheetTexts, baked_part_flipbooks::BakedPartFlipbook, published_tree::kept};

fn product_key(library: &str, logical: &str, suffix: &str) -> String {
    let stem = logical.strip_suffix(suffix).expect("suffix checked");
    let mut path = stem.to_owned();
    for (folder, marker) in crate::sprite_tree::TIERS {
        if let Some(rest) = stem.strip_prefix(&format!("{folder}/")) {
            path = format!("sprites/{rest}.{marker}");
            break;
        }
    }
    format!("content://{library}/{path}")
}

impl SheetTexts {
    pub fn from_libraries(libraries: &ContentLibraries) -> Result<Self, String> {
        Self::from_snapshot(&libraries.snapshot()?, None)
    }

    pub fn from_snapshot(snapshot: &LibrarySnapshot, previous: Option<&crate::SpriteContent>) -> Result<Self, String> {
        let libraries = &snapshot.libraries;
        let all_files = &snapshot.files;
        let mut old = std::collections::HashMap::new();
        if let Some(content) = previous {
            let generation = crate::baked_sheet_rons::current(content);
            for (kind, table) in [("sheet", generation.rons()), ("rig", generation.body_rigs()), ("portrait", generation.portrait_rons()), ("pack", generation.pack_catalog_texts())] {
                old.extend(table.iter().map(|(key, text)| (format!("{kind}:{key}"), (*key, text.clone()))));
            }
            old.extend(generation.part_tables().rows().iter().map(|(key, table, _)| (format!("part:{key}"), (*key, match table {
                BakedPartFlipbook::Ron(text) => text.clone(), _ => std::sync::Arc::from(""),
            }))));
        }
        let mut texts = Self::default();
        for library in libraries.iter() {
            let prefix = format!("content://{}/", library.manifest.id);
            for (_, file) in all_files.iter().filter(|(key, file)| key.strip_prefix(&prefix) == Some(file.logical.as_str())) {
                let logical = &file.logical;
                let address = format!("content://{}/{}", library.manifest.id, logical);
                if logical == "data/room_themes.json" {
                    let themes = serde_json::from_slice(&file.read()?).map_err(|error| format!("{address}: {error}"))?;
                    texts.room_themes.merge(themes)?;
                    continue;
                }
                if logical.ends_with(".png") {
                    let immutable = file.asset_path().to_owned();
                    texts.image_paths.insert(address.clone(), immutable.clone());
                    if all_files.get(logical).is_some_and(|candidate| candidate.library == library.manifest.id) {
                        texts.image_paths.insert(logical.clone(), immutable);
                    }
                    continue;
                }
                let suffix = ["_spritesheet.ron", "_body_rig.ron", "_parts.ron", "_portraits.ron"]
                    .into_iter().find(|suffix| logical.ends_with(suffix));
                let pack = logical.starts_with("sprite_packs/") && logical.ends_with("/ultrapack.json");
                if suffix.is_none() && !pack { continue; }
                let kind = match suffix { Some("_spritesheet.ron") => "sheet", Some("_body_rig.ron") => "rig", Some("_parts.ron") => "part", Some("_portraits.ron") => "portrait", _ => "pack" };
                let key = match suffix {
                    Some("_portraits.ron") => address.clone(),
                    Some(suffix) => product_key(&library.manifest.id, logical, suffix),
                    None => format!("content://{}/{}", library.manifest.id, logical.split('/').nth(1).unwrap()),
                };
                let held = old.get(&format!("{kind}:{key}")).cloned();
                let key = held.as_ref().map(|row| row.0).unwrap_or_else(|| kept(key));
                if kind == "part" && file.generation.is_some() {
                    if let LibraryFileData::Native(path) = &file.data {
                        let path = kept(path.to_string_lossy().into_owned());
                        texts.part_flipbooks.push((key, BakedPartFlipbook::File(path), path));
                        continue;
                    }
                }
                let text = match &file.data {
                    LibraryFileData::Native(path) => {
                        let text = std::fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
                        match held { Some((_, previous)) if previous.as_ref() == text => previous, _ => std::sync::Arc::from(text) }
                    },
                    LibraryFileData::Embedded(Some(bytes)) => std::sync::Arc::from(std::str::from_utf8(bytes).map_err(|error| format!("{address}: {error}"))?),
                    LibraryFileData::Embedded(None) => return Err(format!("Library metadata is not embedded: {address}")),
                };
                match kind {
                    "sheet" => texts.sheets.push((key, text)),
                    "rig" => texts.body_rigs.push((key, text)),
                    "portrait" => texts.portraits.push((key, text)),
                    "part" => texts.part_flipbooks.push((key, BakedPartFlipbook::Ron(text), kept(address))),
                    _ => texts.pack_catalogs.push((key, text)),
                }
            }
        }
        texts.sheets.sort_by_key(|row| row.0);
        texts.body_rigs.sort_by_key(|row| row.0);
        texts.part_flipbooks.sort_by_key(|row| row.0);
        texts.portraits.sort_by_key(|row| row.0);
        texts.pack_catalogs.sort_by_key(|row| row.0);
        Ok(texts)
    }
}

/// Keep the source of a metadata reference when it names an image.
pub(crate) fn image_in(source: &str, image: &str) -> String {
    if image.contains("://") { return image.to_owned(); }
    let Some((scheme, rest)) = source.split_once("://") else { return image.to_owned() };
    if !image.contains('/') {
        let directory = source.rsplit_once('/').map(|(directory, _)| directory).unwrap_or(source);
        return format!("{directory}/{image}");
    }
    if scheme == "content" {
        let Some((library, _)) = rest.split_once('/') else { return image.to_owned() };
        format!("content://{library}/{image}")
    } else {
        format!("{scheme}://{image}")
    }
}

#[cfg(test)]
mod tests {
    use super::image_in;

    #[test]
    fn an_image_keeps_its_metadata_source_or_its_explicit_owner() {
        for source in ["game://sprites/body.ron", "content://shared/sprites/body.ron"] {
            assert_eq!(image_in(source, "body.png"), source.replace(".ron", ".png"));
            assert_eq!(image_in(source, "content://other/art/body.png"), "content://other/art/body.png");
        }
        assert_eq!(image_in("game://sprites/body.ron", "sprites/body.png"), "game://sprites/body.png");
        assert_eq!(image_in("content://shared/sprites/body.ron", "sprites/body.png"), "content://shared/sprites/body.png");
    }
}
