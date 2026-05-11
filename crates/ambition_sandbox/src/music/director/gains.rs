use super::*;

pub(super) fn apply_first_goblin_runtime_balance_overrides(
    cue: &MusicCueSpec,
    state: &MusicStateSpec,
    gains: &mut LayerGains,
    master: f32,
) {
    if cue.id != FIRST_GOBLIN_CUE_ID {
        return;
    }

    // The current first_goblin_tune_v2 runtime plays one mastered `full` layer
    // per section. In that mode, state.gains are intentionally near unity and
    // the renderer/YAML owns section-to-section loudness. Do not apply the old
    // per-stem runtime balance table: doing so hides generator problems and can
    // overwrite the slot used by the `full` layer.
    if cue.layers.len() == 1 && cue.layer("full").is_some() {
        return;
    }

    let overrides: &[(&str, f32)] = match state.id.as_str() {
        "intro" => &[("full", 0.95)],
        "outro" => &[("full", 0.85)],
        "cleared_bridge" => &[
            ("strings", 0.40),
            ("winds", 0.36),
            ("mallets", 0.10),
            ("percussion", 0.06),
            ("brass", 0.10),
            ("choir_pad", 0.04),
        ],
        "wave1" => &[
            ("strings", 0.95),
            ("winds", 1.00),
            ("mallets", 0.18),
            ("percussion", 0.08),
            ("brass", 0.00),
            ("choir_pad", 0.00),
        ],
        "wave2" => &[
            ("strings", 0.95),
            ("winds", 1.00),
            ("mallets", 0.14),
            ("percussion", 0.42),
            ("brass", 0.44),
            ("choir_pad", 0.04),
        ],
        "wave2_brute" => &[
            ("strings", 0.95),
            ("winds", 1.00),
            ("mallets", 0.12),
            ("percussion", 0.50),
            ("brass", 0.58),
            ("choir_pad", 0.06),
        ],
        "wave3" => &[
            ("strings", 0.90),
            ("winds", 1.00),
            ("mallets", 0.08),
            ("percussion", 0.58),
            ("brass", 0.62),
            ("choir_pad", 0.06),
        ],
        "recap_loop" => &[
            ("strings", 0.90),
            ("winds", 0.90),
            ("mallets", 0.10),
            ("percussion", 0.12),
            ("brass", 0.16),
            ("choir_pad", 0.02),
        ],
        _ => return,
    };

    for (layer_id, gain) in overrides {
        if let Some(layer) = cue.layer(layer_id) {
            let slot = layer.slot.min(MAX_LAYERS - 1);
            gains[slot] = gain.max(0.0) * master;
        }
    }
}

pub(super) fn gains_for_state(
    cue: &MusicCueSpec,
    state: &MusicStateSpec,
    settings: &UserSettings,
) -> LayerGains {
    let mut gains = [0.0; MAX_LAYERS];
    let master = settings.audio.effective_music() * cue.relative_volume;
    for layer_gain in &state.gains {
        if let Some(layer) = cue.layer(&layer_gain.layer_id) {
            let slot = layer.slot.min(MAX_LAYERS - 1);
            gains[slot] = layer_gain.gain.max(0.0) * master;
        }
    }
    apply_first_goblin_runtime_balance_overrides(cue, state, &mut gains, master);
    gains
}

pub(super) fn set_bank_targets(
    director: &mut MusicDirectorState,
    bank: MusicBank,
    gains: LayerGains,
) {
    director.target_gains[bank.index()] = gains;
}

pub(super) fn zero_all_targets(director: &mut MusicDirectorState) {
    director.target_gains = [[0.0; MAX_LAYERS]; 2];
}

pub(super) fn zero_all_current_and_targets(director: &mut MusicDirectorState) {
    director.current_gains = [[0.0; MAX_LAYERS]; 2];
    director.target_gains = [[0.0; MAX_LAYERS]; 2];
}

pub(super) fn update_gain_smoothing(
    director: &mut MusicDirectorState,
    channels: &MusicLayerChannels,
    dt: f32,
) {
    let alpha = if STEM_GAIN_BLEND_SECONDS <= 0.0 {
        1.0
    } else {
        1.0 - (-dt / STEM_GAIN_BLEND_SECONDS).exp()
    };
    for bank in [MusicBank::A, MusicBank::B] {
        for slot in 0..MAX_LAYERS {
            let current = director.current_gains[bank.index()][slot];
            let target = director.target_gains[bank.index()][slot];
            let next = current + (target - current) * alpha;
            director.current_gains[bank.index()][slot] =
                if next.abs() < 0.0005 { 0.0 } else { next };
            channels.set_layer_volume(bank, slot, director.current_gains[bank.index()][slot]);
        }
    }

    if let Some(fading_bank) = director.fading_bank {
        director.fade_stop_seconds -= dt;
        if director.fade_stop_seconds <= 0.0 {
            channels.stop_bank(fading_bank, 120);
            director.current_gains[fading_bank.index()] = [0.0; MAX_LAYERS];
            director.target_gains[fading_bank.index()] = [0.0; MAX_LAYERS];
            director.fading_bank = None;
        }
    }
}
