use super::*;

pub(super) fn apply_runtime_balance_overrides(
    cue: &MusicCueSpec,
    state: &MusicStateSpec,
    gains: &mut LayerGains,
    master: f32,
) {
    // Cues that play one mastered `full` layer per section let the
    // renderer/YAML own loudness; state.gains stay near unity and the
    // authored per-stem balance table (if any) must not overwrite the
    // `full` layer's slot.
    if cue.layers.len() == 1 && cue.layer("full").is_some() {
        return;
    }

    let Some(over) = cue
        .runtime_balance_overrides
        .iter()
        .find(|over| over.state_id == state.id)
    else {
        return;
    };

    for (layer_id, gain) in &over.layer_gains {
        if let Some(layer) = cue.layer(layer_id) {
            let slot = layer.slot.min(MAX_LAYERS - 1);
            gains[slot] = gain.max(0.0) * master;
        }
    }
}

pub(super) fn gains_for_state(
    cue: &MusicCueSpec,
    state: &MusicStateSpec,
    settings: &MusicMix,
) -> LayerGains {
    let mut gains = [0.0; MAX_LAYERS];
    let master = settings.effective_music() * cue.relative_volume;
    for layer_gain in &state.gains {
        if let Some(layer) = cue.layer(&layer_gain.layer_id) {
            let slot = layer.slot.min(MAX_LAYERS - 1);
            gains[slot] = layer_gain.gain.max(0.0) * master;
        }
    }
    apply_runtime_balance_overrides(cue, state, &mut gains, master);
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
