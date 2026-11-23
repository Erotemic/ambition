use super::*;
use crate::music_media::{MusicGainBasis, MusicMediaCatalog, PreparedMusicClock};

pub(super) struct LinkedMusicCue<'a> {
    pub spec: &'a MusicCueSpec,
    pub sections: Vec<PreparedMusicSection>,
    pub identity: Option<PreparedMusicMediaIdentity>,
    pub mastered_full: bool,
    pub slots: BTreeMap<&'a str, usize>,
}

pub(super) fn link<'a>(
    cue: &'a MusicCueSpec,
    catalog: &MusicMediaCatalog,
) -> Result<LinkedMusicCue<'a>, String> {
    let slots = super::validation::layer_slots(&cue.layers)?;
    let source = |layer: &str,
                  path: String,
                  product: Option<crate::music_media::MusicEncodedProduct>|
     -> Result<PreparedMusicSource, String> {
        let slot = slots
            .get(layer)
            .ok_or_else(|| format!("unknown source layer {layer:?}"))?;
        Ok(PreparedMusicSource {
            layer_id: layer.to_owned(),
            slot: *slot,
            path,
            product,
        })
    };
    let (sections, identity, mastered_full) = match &cue.media {
        MusicCueMedia::Inline {
            asset_root: _,
            bpm,
            beats_per_bar,
            sections,
        } => {
            let bar = *beats_per_bar * 60.0 / bpm;
            if !bpm.is_finite()
                || *bpm <= 0.0
                || !beats_per_bar.is_finite()
                || *beats_per_bar <= 0.0
                || !bar.is_finite()
                || bar <= 0.0
            {
                return Err("tempo and bar length must be finite and above zero".into());
            }
            let mut prepared = Vec::new();
            for section in sections {
                let duration = f64::from(section.duration_beats) * 60.0 / f64::from(*bpm);
                if !section.duration_beats.is_finite()
                    || section.duration_beats <= 0.0
                    || !duration.is_finite()
                    || duration <= 0.0
                {
                    return Err(format!(
                        "section {:?}: duration must be finite and above zero",
                        section.id
                    ));
                }
                let mut sources = Vec::new();
                for item in &section.sources {
                    if item.path.trim().is_empty() {
                        return Err(format!(
                            "section {:?}: empty source path for {:?}",
                            section.id, item.layer_id
                        ));
                    }
                    sources.push(source(&item.layer_id, cue.media.source_path(item), None)?);
                }
                prepared.push(PreparedMusicSection {
                    id: section.id.clone(),
                    duration_seconds: duration,
                    file_duration_seconds: None,
                    looped: section.looped,
                    sources,
                    clock: PreparedMusicClock::Constant {
                        seconds_per_bar: f64::from(*beats_per_bar) * 60.0 / f64::from(*bpm),
                        first_bar_seconds: 0.0,
                        frames: None,
                        bar_count: None,
                    },
                    markers: Vec::new(),
                    retained_material: None,
                });
            }
            (
                prepared,
                None,
                cue.layers.len() == 1 && slots.contains_key("full"),
            )
        }
        MusicCueMedia::Published {
            description,
            products,
        } => {
            let media = catalog.get(description).ok_or_else(|| {
                format!("playback description {description:?} is not in the selected media catalog")
            })?;
            let basis = match products {
                MusicProductSet::MasteredFull => MusicGainBasis::MasteredFull,
                MusicProductSet::GroupedLayers => MusicGainBasis::GroupedPreMaster,
            };
            let mut prepared = Vec::new();
            for section in &media.description.sections {
                let layers: std::collections::BTreeSet<_> = section
                    .product_roles
                    .iter()
                    .filter(|(_, role)| **role == basis)
                    .map(|(layer, _)| layer.as_str())
                    .collect();
                if layers.is_empty() || layers != slots.keys().copied().collect() {
                    return Err(format!(
                        "section {:?}: policy layers do not match the selected {products:?} product set",
                        section.id
                    ));
                }
                if !section.available {
                    return Err(format!(
                        "section {:?}: selected products are not available",
                        section.id
                    ));
                }
                let mut sources = Vec::new();
                for product in &section.products {
                    if product.gain_basis != basis {
                        continue;
                    }
                    let path = media
                        .source_path(&section.id, &product.layer)
                        .map_err(|error| {
                            format!(
                                "section {:?} layer {:?}: {error}",
                                section.id, product.layer
                            )
                        })?;
                    sources.push(source(
                        &product.layer,
                        path.to_owned(),
                        Some(product.clone()),
                    )?);
                }
                sources.sort_by_key(|source| source.slot);
                let frames = section
                    .frame_plan
                    .as_ref()
                    .expect("validated published section");
                prepared.push(PreparedMusicSection {
                    id: section.id.clone(),
                    duration_seconds: frames.musical_end_frame as f64
                        / f64::from(frames.sample_rate),
                    file_duration_seconds: Some(
                        frames.file_frames as f64 / f64::from(frames.sample_rate),
                    ),
                    looped: section.looped,
                    sources,
                    clock: section.prepared_clock(),
                    markers: frames.markers.clone(),
                    retained_material: Some(section.material.clone()),
                });
            }
            (
                prepared,
                Some(PreparedMusicMediaIdentity {
                    cue_id: media.description.cue_id.clone(),
                    description_path: media.description_path.clone(),
                    compiled_score_fingerprint: media
                        .description
                        .compiled_score_fingerprint
                        .clone(),
                    render_fingerprint: media
                        .description
                        .render_fingerprint
                        .clone()
                        .expect("validated render identity"),
                }),
                basis == MusicGainBasis::MasteredFull,
            )
        }
    };
    Ok(LinkedMusicCue {
        spec: cue,
        sections,
        identity,
        mastered_full,
        slots,
    })
}
