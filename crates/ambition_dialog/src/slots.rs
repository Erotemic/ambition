//! Conversations beside the main one (SIMULATION-ISLANDS N2).
//!
//! The main runner presents through the [`DialogState`] resource. Each further
//! live conversation runs on its own entity: a [`DialogSlot`] holds its
//! presentation state, and the Yarn bridge gives the entity a `DialogueRunner`
//! of its own. The runners share one Yarn variable store.

use bevy::prelude::*;

use crate::runtime::DialogState;

/// A runner beside the main one, and the presentation state of the
/// conversation it runs. The same fields and requests as the main
/// [`DialogState`]: a slot is started, advanced and closed the same way.
#[derive(Component, Clone, Debug, Default)]
pub struct DialogSlot(pub DialogState);

/// The runner whose Yarn command or function is running now. The bridge sets
/// it around each call, so a command learns which conversation fired it
/// without the runner passing it. `None` outside a call.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SpeakingRunner(pub Option<Entity>);

/// Run condition of the slots' input and reveal: a slot's conversation is open,
/// or its hold mark is still up.
pub fn a_slot_conversation_is_open_or_held(slots: Query<&DialogSlot>) -> bool {
    slots.iter().any(|slot| slot.0.active() || slot.0.held)
}

/// The reveal of each slot's line, as [`crate::dialog_reveal_tick`] does for the
/// main one.
pub fn slot_dialog_reveal_tick(
    time: Res<Time>,
    voice_catalog: Option<Res<ambition_sfx::voice::DialogueVoiceCatalog>>,
    mut slots: Query<&mut DialogSlot>,
    mut sfx: ambition_sfx::SfxWriter,
) {
    for slot in &mut slots {
        crate::systems::reveal(time.delta_secs(), voice_catalog.as_deref(), slot.map_unchanged(|slot| &mut slot.0), &mut sfx);
    }
}
