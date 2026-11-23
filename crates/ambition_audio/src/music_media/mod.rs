//! Import playback facts and media from one accepted library snapshot.

use ambition_asset_manager::content_library::{LibraryFileData, LibrarySnapshot};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

mod clock;
mod facts;
mod validation;
pub use clock::{MusicFrameOrigin, PreparedMusicClock};
pub use facts::*;

#[derive(Clone, Debug, PartialEq)]
pub struct SelectedMusicMedia {
    pub description: MusicPlaybackDescription,
    pub description_path: String,
    sources: BTreeMap<(String, String), Result<String, String>>,
}

/// The selected media needed by one authored music catalog.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MusicMediaCatalog {
    selected: BTreeMap<String, SelectedMusicMedia>,
    ungenerated: BTreeSet<String>,
}

impl MusicMediaCatalog {
    pub fn from_snapshot<'a>(
        snapshot: &LibrarySnapshot,
        references: impl IntoIterator<Item = &'a str>,
    ) -> Result<Self, Vec<String>> {
        let mut catalog = BTreeMap::new();
        let mut ungenerated = BTreeSet::new();
        let mut errors = Vec::new();
        for path in references.into_iter().collect::<BTreeSet<_>>() {
            if !snapshot.files.contains_key(path) && generated_reference(snapshot, path) {
                ungenerated.insert(path.to_owned());
                continue;
            }
            match SelectedMusicMedia::read(snapshot, path) {
                Ok(media) => {
                    catalog.insert(path.to_owned(), media);
                }
                Err(error) => errors.push(format!("music media {path:?}: {error}")),
            }
        }
        if errors.is_empty() {
            Ok(Self {
                selected: catalog,
                ungenerated,
            })
        } else {
            Err(errors)
        }
    }

    pub fn get(&self, reference: &str) -> Option<&SelectedMusicMedia> {
        self.selected.get(reference)
    }

    pub fn is_ungenerated(&self, reference: &str) -> bool {
        self.ungenerated.contains(reference)
    }
}

fn generated_reference(snapshot: &LibrarySnapshot, reference: &str) -> bool {
    let Some((library, logical)) = reference
        .strip_prefix("content://")
        .and_then(|path| path.split_once('/'))
    else {
        return false;
    };
    validation::relative(logical)
        && snapshot.libraries.get(library).is_some_and(|library| {
            !library.is_embedded() && library.producer_of(logical) == Some("music")
        })
}

impl SelectedMusicMedia {
    fn read(snapshot: &LibrarySnapshot, reference: &str) -> Result<Self, String> {
        let selected = snapshot
            .files
            .get(reference)
            .ok_or("the selected libraries do not supply the playback description")?;
        let bytes = selected.read()?;
        if let Some(expected) = &selected.sha256 {
            if !format!("{:x}", Sha256::digest(&bytes)).eq_ignore_ascii_case(expected) {
                return Err(
                    "playback description differs from its accepted publication digest".into(),
                );
            }
        } else if matches!(selected.data, LibraryFileData::Native(_)) {
            return Err("playback description has no accepted publication identity".into());
        }

        // The outer render report belongs to the producer. The inner contract is versioned.
        #[derive(serde::Deserialize)]
        struct RenderReport {
            music_playback: MusicPlaybackDescription,
        }
        let report: RenderReport =
            serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
        let description = report.music_playback;
        description.validate()?;
        let directory = selected
            .logical
            .rsplit_once('/')
            .map_or(String::new(), |(directory, _)| format!("{directory}/"));
        let mut sources = BTreeMap::new();
        for section in &description.sections {
            for product in &section.products {
                let path = format!("content://{}/{directory}{}", selected.library, product.path);
                let address = (|| {
                    let file = snapshot.files.get(&path).ok_or_else(|| {
                        format!(
                            "section {:?} layer {:?}: selected product {path:?} is absent",
                            section.id, product.layer
                        )
                    })?;
                    let matches = match (&file.sha256, &file.data) {
                        (Some(digest), _) => digest.eq_ignore_ascii_case(&product.sha256),
                        (None, LibraryFileData::Embedded(Some(bytes))) => {
                            format!("{:x}", Sha256::digest(bytes))
                                .eq_ignore_ascii_case(&product.sha256)
                        }
                        // The build fixes served files and metadata in one embedded library.
                        (None, LibraryFileData::Embedded(None)) => true,
                        (None, LibraryFileData::Native(_)) => {
                            return Err(format!("{path:?} has no accepted publication identity"))
                        }
                    };
                    if !matches {
                        return Err(format!("section {:?} layer {:?}: selected media digest differs from the playback description", section.id, product.layer));
                    }
                    Ok(file.asset_path().to_owned())
                })();
                sources.insert((section.id.clone(), product.layer.clone()), address);
            }
        }
        Ok(Self {
            description,
            description_path: selected.asset_path().to_owned(),
            sources,
        })
    }

    pub fn source_path(&self, section: &str, layer: &str) -> Result<&str, &str> {
        self.sources
            .get(&(section.to_owned(), layer.to_owned()))
            .ok_or("selected product has no immutable address")?
            .as_deref()
            .map_err(String::as_str)
    }
}

#[cfg(test)]
pub(crate) mod tests;
