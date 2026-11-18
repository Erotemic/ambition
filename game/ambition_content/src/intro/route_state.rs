//! Derived narrative save flags for the intro route.
//!
//! After flag effects are applied, this system emits missing target flags from
//! the pack's chains (`data/flag_chains.ron`). Targets then flow through the
//! ordinary flag-effect path on the next frame, including quest
//! notifications. The operation is idempotent because an already-present
//! target is skipped.

use bevy::prelude::*;

use ambition_combat::SetFlagsRequested;

/// The chains the selected pack states (`data/flag_chains.ron`).
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct FlagChains(pub Vec<ambition_content_schemas::flag_chains::FlagChain>);

/// Insert the chains the selected pack states. A pack with no chains file
/// has no chains.
pub(crate) fn register(app: &mut App) {
    let pack = crate::pack::select(app.world_mut());
    let chains = ambition_content_schemas::flag_chains::lowered_flag_chains(&pack).cloned().unwrap_or_default();
    app.insert_resource(FlagChains(chains));
}

/// Emit target flags whose triggers are present and whose targets are absent.
pub fn emit_intro_flag_chains(
    chains: Res<FlagChains>,
    save: Res<ambition_persistence::save::AmbitionGameSave>,
    mut effects: MessageWriter<SetFlagsRequested>,
) {
    let data = save.data();
    for chain in &chains.0 {
        if data.flag(&chain.when) && !data.flag(&chain.set) {
            effects.write(SetFlagsRequested {
                changes: vec![(chain.set.clone(), true)],
                reply: None,
                written: None,
            });
        }
    }
}

#[cfg(test)]
mod tests;
