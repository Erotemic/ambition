use super::*;
use std::collections::BTreeSet;

fn valid_id(value: &str) -> bool {
    !value.is_empty() && value.trim() == value
}
fn digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}
pub(super) fn relative(value: &str) -> bool {
    !value.is_empty()
        && !value.contains('\\')
        && !value.starts_with('/')
        && value
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != ".." && !part.contains(':'))
}
fn near(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-7
}

impl MusicPlaybackDescription {
    pub(super) fn validate(&self) -> Result<(), String> {
        if self.schema != PLAYBACK_SCHEMA || self.evidence != "published_products" {
            return Err(
                "native music needs published_products with schema ambition.music_playback.v1"
                    .into(),
            );
        }
        if !valid_id(&self.cue_id)
            || !digest(&self.compiled_score_fingerprint)
            || !self.render_fingerprint.as_deref().is_some_and(digest)
        {
            return Err("music product needs cue, compiled-score, and render identities".into());
        }
        if !matches!(self.product_family.as_str(), "adaptive" | "playback")
            || self.sections.is_empty()
        {
            return Err("music product needs a supported family and playable sections".into());
        }
        let mut sections = BTreeSet::new();
        for section in &self.sections {
            if !valid_id(&section.id) || !sections.insert(&section.id) {
                return Err(format!(
                    "invalid or duplicate music section {:?}",
                    section.id
                ));
            }
            let groups: BTreeSet<_> = section
                .product_roles
                .iter()
                .filter(|(_, basis)| **basis == MusicGainBasis::GroupedPreMaster)
                .map(|(id, _)| id)
                .collect();
            if groups != self.layer_groups.keys().collect() {
                return Err(format!(
                    "section {:?}: grouped products differ from the layer plan",
                    section.id
                ));
            }
            section
                .validate()
                .map_err(|error| format!("section {:?}: {error}", section.id))?;
        }
        Ok(())
    }
}

impl MusicMediaSection {
    fn validate(&self) -> Result<(), String> {
        let frame = self
            .frame_plan
            .as_ref()
            .ok_or("published section has no frame plan")?;
        let origin = MusicFrameOrigin {
            sample_rate: frame.sample_rate,
            score_seconds: self.start_seconds,
            score_frame: frame.score_origin_frame,
        };
        let global_frame =
            |seconds: f64| (seconds * f64::from(frame.sample_rate)).round_ties_even();
        let file_end = if self.looped {
            self.end_seconds
        } else {
            self.tail_end_seconds.unwrap_or(self.end_seconds)
        };
        if frame.sample_rate == 0
            || frame.musical_end_frame == 0
            || frame.file_frames < frame.musical_end_frame
            || ![
                self.start_seconds,
                self.end_seconds,
                self.start_beat,
                self.end_beat,
                self.musical_end_seconds,
                self.file_duration_seconds,
            ]
            .iter()
            .all(|value| value.is_finite())
            || self.start_seconds < 0.0
            || self.end_seconds <= self.start_seconds
            || self.end_beat <= self.start_beat
            || !near(
                self.musical_end_seconds,
                self.end_seconds - self.start_seconds,
            )
            || frame.score_origin_frame as f64
                != (self.start_seconds * f64::from(frame.sample_rate)).round_ties_even()
            || frame.musical_end_frame as f64
                != global_frame(self.end_seconds) - frame.score_origin_frame as f64
            || frame.file_frames as f64 != global_frame(file_end) - frame.score_origin_frame as f64
            || !near(self.file_duration_seconds, file_end - self.start_seconds)
        {
            return Err("invalid encoded frame origin, body, or file span".into());
        }
        if frame.loop_frames != self.looped.then_some(frame.musical_end_frame)
            || (self.looped && frame.file_frames != frame.musical_end_frame)
        {
            return Err("loop period does not match its encoded body".into());
        }
        if self.material.independent_tail || self.material.scheduled_pickup {
            return Err(
                "this product schema does not supply independent tails or scheduled pickups".into(),
            );
        }
        let tail = if self.tail.is_none() {
            MusicTailConstruction::None
        } else if self.looped {
            MusicTailConstruction::Folded
        } else {
            MusicTailConstruction::Appended
        };
        let lead = if self.lead_in.is_none() {
            MusicLeadConstruction::None
        } else {
            MusicLeadConstruction::Folded
        };
        if self.material.tail != tail
            || self.material.lead_in != lead
            || frame.tail_source_audio_frames > frame.tail_source_frames
            || self.material.tail_excludes_pickup
                != (self.tail_audio_end_seconds != self.tail_end_seconds)
        {
            return Err("retained material does not match its construction".into());
        }
        for (seconds, expected, from_end) in [
            (self.tail_end_seconds, frame.tail_source_frames, true),
            (
                self.tail_audio_end_seconds,
                frame.tail_source_audio_frames,
                true,
            ),
            (
                self.lead_in_start_seconds,
                frame.lead_in_source_frames,
                false,
            ),
        ] {
            if seconds.is_some_and(|value| !value.is_finite() || value < 0.0) {
                return Err("invalid source-material time".into());
            }
            let actual = seconds.map_or(0.0, |seconds| {
                if from_end {
                    (global_frame(seconds) - frame.score_origin_frame as f64)
                        - frame.musical_end_frame as f64
                } else {
                    -(global_frame(seconds) - frame.score_origin_frame as f64)
                }
            });
            if actual != expected as f64 {
                return Err("source-material frame span differs from its clock".into());
            }
        }
        if !self.looped && frame.file_frames != frame.musical_end_frame + frame.tail_source_frames {
            return Err("one-shot file span does not include its declared tail".into());
        }
        if frame
            .bar_frames
            .iter()
            .any(|bar| *bar > frame.musical_end_frame)
            || frame.bar_frames.windows(2).any(|pair| pair[0] >= pair[1])
        {
            return Err("bar frames must advance inside the musical body".into());
        }
        match &self.clock {
            MusicMediaClock::Constant {
                bpm,
                beats_per_bar,
                bar_origin_seconds,
            } => {
                let period = beats_per_bar * 60.0 / bpm;
                if !bpm.is_finite()
                    || *bpm <= 0.0
                    || !beats_per_bar.is_finite()
                    || *beats_per_bar <= 0.0
                    || !bar_origin_seconds.is_finite()
                    || !period.is_finite()
                    || period * f64::from(frame.sample_rate) < 1.0
                {
                    return Err("invalid constant music clock".into());
                }
                let first =
                    bar_origin_seconds + (-bar_origin_seconds / period).ceil().max(0.0) * period;
                let count = frame.bar_frames.len();
                let next = first + count as f64 * period;
                let last = first + count.saturating_sub(1) as f64 * period;
                if next < self.musical_end_seconds - 1e-9
                    || (count > 0 && last > self.musical_end_seconds + 1e-9)
                    || frame.bar_frames.iter().enumerate().any(|(index, bar)| {
                        (*bar as f64 - origin.local_frame(first + index as f64 * period)).abs()
                            > 1.0
                    })
                {
                    return Err("constant clock differs from its encoded bar frames".into());
                }
            }
            MusicMediaClock::Boundaries { bar_seconds } => {
                if bar_seconds.len() != frame.bar_frames.len()
                    || bar_seconds
                        .iter()
                        .zip(&frame.bar_frames)
                        .any(|(seconds, bar)| {
                            !seconds.is_finite()
                                || *seconds < 0.0
                                || (origin.local_frame(*seconds) - *bar as f64).abs() > 1.0
                        })
                {
                    return Err("variable clock differs from its encoded bar frames".into());
                }
            }
        }
        let mut markers = BTreeSet::new();
        if self.markers.len() != frame.markers.len() {
            return Err("marker and frame counts differ".into());
        }
        for (marker, encoded) in self.markers.iter().zip(&frame.markers) {
            if !valid_id(&marker.id)
                || !markers.insert(&marker.id)
                || !marker.seconds.is_finite()
                || !marker.beat.is_finite()
                || marker.seconds < 0.0
                || marker.beat < self.start_beat
                || marker.beat > self.end_beat
                || encoded.id != marker.id
                || encoded.role != marker.role
                || encoded.frame > frame.musical_end_frame
                || (encoded.frame as f64 - origin.local_frame(marker.seconds)).abs() > 1.0
            {
                return Err("invalid typed marker or frame origin".into());
            }
        }
        if !frame
            .markers
            .iter()
            .any(|marker| marker.role == MusicMarkerRole::Entry && marker.frame == 0)
        {
            return Err("section has no entry marker at its frame origin".into());
        }
        let mut layers = BTreeSet::new();
        let mut paths = BTreeSet::new();
        if self.available != !self.products.is_empty() {
            return Err("product availability differs from its files".into());
        }
        let grouped = self
            .products
            .iter()
            .find(|product| product.gain_basis == MusicGainBasis::GroupedPreMaster);
        for product in &self.products {
            if product.gain_basis == MusicGainBasis::GroupedPreMaster
                && grouped.is_some_and(|first| {
                    first.runtime_stem_gain_mode != product.runtime_stem_gain_mode
                        || first.applied_runtime_gain_db != product.applied_runtime_gain_db
                })
            {
                return Err("grouped products use incompatible runtime gain bases".into());
            }
            if !valid_id(&product.layer)
                || !layers.insert(&product.layer)
                || !relative(&product.path)
                || !paths.insert(&product.path)
                || !digest(&product.sha256)
                || product.sample_rate != frame.sample_rate
                || product.frames != frame.file_frames
                || !matches!(product.channels, 1 | 2)
                || product.format != "OGG"
                || product.subtype != "VORBIS"
                || product.frame_observer != "libsndfile"
                || self.product_roles.get(&product.layer) != Some(&product.gain_basis)
                || !product.applied_runtime_gain_db.is_finite()
            {
                return Err(format!(
                    "invalid or incompatible encoded product {:?}",
                    product.layer
                ));
            }
            match product.gain_basis {
                MusicGainBasis::MasteredFull
                    if product.layer == "full"
                        && product.runtime_stem_gain_mode.is_none()
                        && product.applied_runtime_gain_db == 0.0
                        && matches!(
                            product.mastering.as_deref(),
                            Some("composition_master" | "legacy_section_postprocess")
                        ) => {}
                MusicGainBasis::GroupedPreMaster
                    if product.mastering.is_none()
                        && matches!(
                            product.runtime_stem_gain_mode.as_deref(),
                            Some("native" | "shared")
                        )
                        && (product.runtime_stem_gain_mode.as_deref() != Some("native")
                            || product.applied_runtime_gain_db == 0.0) => {}
                _ => {
                    return Err(format!(
                        "invalid gain basis for product {:?}",
                        product.layer
                    ))
                }
            }
        }
        if self.available && layers.len() != self.product_roles.len() {
            return Err("published section omits a declared product".into());
        }
        Ok(())
    }

    pub(crate) fn prepared_clock(&self) -> PreparedMusicClock {
        let frame = self
            .frame_plan
            .as_ref()
            .expect("validated product frame plan");
        let origin = MusicFrameOrigin {
            sample_rate: frame.sample_rate,
            score_seconds: self.start_seconds,
            score_frame: frame.score_origin_frame,
        };
        if let MusicMediaClock::Constant {
            bpm,
            beats_per_bar,
            bar_origin_seconds,
        } = &self.clock
        {
            let period = beats_per_bar * 60.0 / bpm;
            let first =
                bar_origin_seconds + (-bar_origin_seconds / period).ceil().max(0.0) * period;
            if frame.bar_frames.iter().enumerate().all(|(index, bar)| {
                *bar as f64 == origin.local_frame(first + index as f64 * period)
            }) {
                return PreparedMusicClock::Constant {
                    seconds_per_bar: period,
                    first_bar_seconds: *bar_origin_seconds,
                    frames: Some(origin),
                    bar_count: Some(frame.bar_frames.len()),
                };
            }
        }
        // Use the integer frame facts. Float arithmetic can move a rounding tie by one frame.
        PreparedMusicClock::Boundaries {
            bar_seconds: frame
                .bar_frames
                .iter()
                .map(|frame_index| *frame_index as f64 / f64::from(frame.sample_rate))
                .collect(),
        }
    }
}
