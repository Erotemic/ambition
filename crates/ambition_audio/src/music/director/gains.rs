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

pub(super) fn gains_for_state(cue: &MusicCueSpec, state: &MusicStateSpec) -> LayerGains {
    let mut gains = [0.0; MAX_LAYERS];
    let master = cue.relative_volume;
    for layer_gain in &state.gains {
        if let Some(layer) = cue.layer(&layer_gain.layer_id) {
            let slot = layer.slot.min(MAX_LAYERS - 1);
            gains[slot] = layer_gain.gain.max(0.0) * master;
        }
    }
    apply_runtime_balance_overrides(cue, state, &mut gains, master);
    gains
}
