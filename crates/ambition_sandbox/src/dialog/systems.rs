use ambition_engine as ae;
use bevy::prelude::*;

use super::content::DialogMode;
use super::ui::DialogChoiceSlot;
use crate::game_mode::GameMode;
#[cfg(feature = "input")]
use crate::input::MenuControlFrame;
#[cfg(feature = "input")]
use crate::ui_nav::apply_vertical_scroll;

#[cfg(feature = "input")]
pub fn dialog_pointer_input(
    mut runtime: ResMut<crate::SandboxRuntime>,
    mode: Res<State<GameMode>>,
    mut next_mode: ResMut<NextState<GameMode>>,
    choices: Query<(&Interaction, &DialogChoiceSlot), Changed<Interaction>>,
) {
    if !runtime.dialogue.active() {
        return;
    }
    if !matches!(mode.get(), GameMode::Dialogue) {
        return;
    }

    let option_count = runtime.dialogue.options().len();
    for (interaction, slot) in &choices {
        let valid_slot = if option_count == 0 {
            slot.index == 0
        } else {
            slot.index < option_count
        };
        if !valid_slot {
            continue;
        }

        match interaction {
            Interaction::Hovered => {
                let index = slot.index.min(option_count.saturating_sub(1));
                if runtime.dialogue.selected_option != index {
                    runtime.dialogue.pointer_armed = None;
                }
                runtime.dialogue.selected_option = index;
            }
            Interaction::Pressed => {
                let index = slot.index.min(option_count.saturating_sub(1));

                #[cfg(target_os = "android")]
                {
                    let confirm = runtime.dialogue.selected_option == index
                        && runtime.dialogue.pointer_armed == Some(index);
                    runtime.dialogue.selected_option = index;
                    if confirm {
                        runtime.dialogue.pointer_armed = None;
                        let closed = runtime.dialogue.confirm_or_advance();
                        if closed {
                            next_mode.set(GameMode::Playing);
                        }
                    } else {
                        runtime.dialogue.pointer_armed = Some(index);
                    }
                }

                #[cfg(not(target_os = "android"))]
                {
                    runtime.dialogue.selected_option = index;
                    let closed = runtime.dialogue.confirm_or_advance();
                    if closed {
                        next_mode.set(GameMode::Playing);
                    }
                }
                return;
            }
            Interaction::None => {}
        }
    }
}

#[cfg(not(feature = "input"))]
pub fn dialog_pointer_input() {}

#[cfg(feature = "input")]
pub fn dialog_input(
    menu: Res<MenuControlFrame>,
    mut runtime: ResMut<crate::SandboxRuntime>,
    mode: Res<State<GameMode>>,
    mut next_mode: ResMut<NextState<GameMode>>,
) {
    if !runtime.dialogue.active() {
        return;
    }
    if !matches!(mode.get(), GameMode::Dialogue) {
        return;
    }
    if menu.back || menu.start {
        runtime.dialogue.close();
        next_mode.set(GameMode::Playing);
        return;
    }
    let mut frame = crate::input::MenuInputFrame {
        up: menu.up,
        down: menu.down,
        left: menu.left,
        right: menu.right,
        select: menu.select,
        back: menu.back,
        start: menu.start,
    };
    apply_vertical_scroll(&mut frame, menu.vertical_scroll_steps());
    if frame.up {
        runtime.dialogue.select_delta(-1);
    }
    if frame.down {
        runtime.dialogue.select_delta(1);
    }
    if frame.select {
        let closed = runtime.dialogue.confirm_or_advance();
        if closed {
            next_mode.set(GameMode::Playing);
        }
    }
}

#[cfg(not(feature = "input"))]
pub fn dialog_input() {}

/// Swap the live dialog branch when world state has progressed past
/// the conversation's original prompt. Today this only affects the
/// pirate cove: once the mockingbird's encounter is `Cleared`, the
/// admiral and raider both pivot from "go kill the bird" to "the bird
/// is dead, here is your reward / banter."
///
/// GENERALIZATION PLAN: this is the second piece of pirate-specific
/// glue (the first is `boss_encounter::sync_mockingbird_treasure_chest`).
/// When a third quest needs post-completion dialog, lift this into a
/// data table — `(trigger_mode, gate_predicate, target_mode)` triples
/// registered by content code — and let the system iterate. Until
/// then, the pair-of-conditions is small enough to inline.
///
/// Runs each frame `.after(sandbox_update).before(sync_dialog_ui)` so
/// the redirected mode is the one the renderer reads.
pub fn redirect_post_quest_dialog(
    mut runtime: ResMut<crate::SandboxRuntime>,
    save: Res<crate::save::SandboxSave>,
) {
    if !runtime.dialogue.active() {
        return;
    }
    let bird_dead = matches!(
        save.data()
            .boss(crate::boss_encounter::MOCKINGBIRD_ENCOUNTER_ID),
        ae::PersistedEncounterState::Cleared,
    );
    if !bird_dead {
        return;
    }
    let new_mode = match runtime.dialogue.mode() {
        DialogMode::PirateAdmiral => Some(DialogMode::PirateAdmiralAfterTreasure),
        DialogMode::PirateRaider => Some(DialogMode::PirateRaiderAfterTreasure),
        _ => None,
    };
    if let Some(mode) = new_mode {
        runtime.dialogue.set_mode(mode);
    }
}
