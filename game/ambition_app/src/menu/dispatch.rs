//! Backend-agnostic menu action dispatcher.
//!
//! Maps [`MenuPageAction`] into game effects (equip/use, page changes, settings,
//! quit/reset, System drill-down) so the grid and cube backends call one path.

use bevy::prelude::*;

use ambition::menu::ActiveMenuPages;

use crate::menu::effects::{
    dispatch_item_confirm, MenuAction, MenuEffectManaQuery, MenuEffectPlayers,
};
use crate::menu::kaleidoscope_app::{
    back_edge_focus, close_system_entry, play_ui, rotate_sfx, KaleidoscopeCursor,
    KaleidoscopeSystemNav, SystemMenuParams,
};
use crate::menu::model::{
    system_rows_with_quality_prompt, MenuFocus, MenuPage, MenuPageAction, SystemRow,
};
use crate::menu::quality_confirm::VisualQualityConfirmState;
use ambition::actors::avatar::PlayerHealRequested;
use ambition::items::OwnedItems;
use ambition::persistence::settings::UserSettings;
use ambition::settings_menu::settings::{
    apply_settings_option, settings_menu_model, SettingsOptionId, SettingsOptionKind,
};
use ambition::settings_menu::system::SystemMenuAction;
use ambition::sfx::SfxWriter;

/// Dispatch a [`MenuPageAction`]. Item Equip/Use reuse the grid's shared
/// [`dispatch_item_confirm`] (no portal/equip/heal duplication); page-change sets
/// the active page so the lib rotates that face to the camera.
#[allow(clippy::too_many_arguments)]
pub(crate) fn dispatch_menu_action(
    action: MenuPageAction,
    pages: &mut ActiveMenuPages<MenuPage, MenuPageAction>,
    system_nav: &mut KaleidoscopeSystemNav,
    cursor: &mut KaleidoscopeCursor,
    owned: &mut OwnedItems,
    settings: &mut UserSettings,
    quality_confirm: &mut VisualQualityConfirmState,
    close_menu: &mut bool,
    commands: &mut Commands,
    players: &mut MenuEffectPlayers,
    mana_q: &mut MenuEffectManaQuery,
    heals: &mut MessageWriter<PlayerHealRequested>,
    sfx: &mut SfxWriter,
    system: &mut SystemMenuParams,
) {
    match action {
        MenuPageAction::Equip(item) | MenuPageAction::Use(item) => {
            let decided = dispatch_item_confirm(item, owned, commands, players, mana_q, heals);
            // Pick the confirm sound from the RESOLVED action so equip/unequip/use
            // are distinct, and a no-op (not owned / nothing to do) gives error feedback.
            let id = match decided {
                MenuAction::Equip(_) => ambition::sfx::ids::UI_MENU_EQUIP,
                MenuAction::Unequip(_) => ambition::sfx::ids::UI_MENU_UNEQUIP,
                MenuAction::UseConsumable(_) => ambition::sfx::ids::UI_MENU_ACCEPT,
                MenuAction::Inspect(_) | MenuAction::NotOwned(_) => {
                    ambition::sfx::ids::UI_MENU_ERROR
                }
            };
            play_ui(sfx, id);
            info!("cube action: {:?} \u{2192} {:?}", item, decided);
        }
        MenuPageAction::ChangePage(page) => {
            let from = pages.active;
            play_ui(sfx, rotate_sfx(from, page));
            pages.active = Some(page);
            // Fix 1: land the cursor on the new page's "back" edge button — the one
            // that turns BACK toward the page we came from — so an immediate select /
            // rotate goes home and the arriving control is highlighted.
            cursor.mark_keyboard(back_edge_focus(from, page));
            info!("cube page \u{2192} {:?}", page);
        }
        MenuPageAction::System(option) => {
            apply_system_option(option, settings, quality_confirm, close_menu, sfx);
        }
        MenuPageAction::SystemStep(option, dir) => {
            // Fix 2: a ◀ / ▶ click zone on a value row steps the setting in the given
            // direction through the SAME IR path the keyboard LEFT/RIGHT uses. Value
            // rows never close the menu. Visual quality is transactional: stepping it
            // only edits the pending profile until the user selects Apply.
            if option == SettingsOptionId::VisualQuality {
                quality_confirm.step_from(settings.video.quality.profile, dir);
            } else {
                let _ = apply_settings_option(option, dir, settings);
            }
            play_ui(sfx, ambition::sfx::ids::UI_SLIDER_TICK);
            info!("cube system step: {:?} dir {}", option, dir);
        }
        MenuPageAction::SystemOption(opt) => {
            // Radio / Language / Developer screen options apply against their live
            // resource (radio auditions + keeps the menu open; dev toggles mutate
            // DeveloperTools). The menu never closes from these.
            let id = system.apply_option(opt);
            play_ui(sfx, id);
            info!("cube system option: {:?}", opt);
        }
        MenuPageAction::ConfirmVisualQuality => {
            if let Some(profile) = quality_confirm.take_confirmed() {
                settings.video.quality.profile = profile;
                focus_visual_quality_setting_row(settings, system_nav, cursor, system);
                play_ui(sfx, ambition::sfx::ids::UI_MENU_ACCEPT);
                info!("cube system action: confirmed visual quality {:?}", profile);
            } else {
                play_ui(sfx, ambition::sfx::ids::UI_MENU_ERROR);
                info!("cube system action: confirm visual quality with no pending profile");
            }
        }
        MenuPageAction::CancelVisualQuality => {
            quality_confirm.cancel();
            focus_visual_quality_setting_row(settings, system_nav, cursor, system);
            play_ui(sfx, ambition::sfx::ids::UI_MENU_BACK);
            info!("cube system action: cancelled visual quality change");
        }
        MenuPageAction::SystemAction(SystemMenuAction::ResetSandbox) => {
            // Immediate, no-confirm: queue the reset and fold the menu shut.
            system.request_reset();
            *close_menu = true;
            play_ui(sfx, ambition::sfx::ids::UI_MENU_ACCEPT);
            info!("cube system action: reset sandbox");
        }
        MenuPageAction::SystemAction(SystemMenuAction::Quit) => {
            // Immediate: request application exit and fold the menu shut. Mirrors
            // the old pause-menu Quit row (which is removed in a later phase).
            commands.write_message(bevy::app::AppExit::Success);
            *close_menu = true;
            play_ui(sfx, ambition::sfx::ids::UI_MENU_ACCEPT);
            info!("cube system action: quit to desktop");
        }
        MenuPageAction::SystemAction(SystemMenuAction::ResetAllSettings) => {
            // Immediate, no-confirm: reset every persisted settings/dev resource
            // (the same set the pause menu's ResetAllSettings restores), then fold
            // the menu shut. The close also unpauses (the reset-pause fix).
            // `save_settings_on_change` then persists the defaulted `UserSettings`.
            quality_confirm.cancel();
            system.reset_all_settings(settings);
            *close_menu = true;
            play_ui(sfx, ambition::sfx::ids::UI_MENU_ACCEPT);
            info!("cube system action: reset all settings");
        }
        MenuPageAction::OpenSystemEntry(entry) => {
            // Drill INTO an entry: show its screen rows, land the cursor on the
            // first row. The republish picks up the new drill state + cursor.
            play_ui(sfx, ambition::sfx::ids::UI_TAB_CHANGE);
            system_nav.open_entry = Some(entry);
            cursor.mark_keyboard(MenuFocus::System(0));
            info!("cube system entry \u{2192} {:?}", entry);
        }
        MenuPageAction::CloseSystemEntry => {
            play_ui(sfx, ambition::sfx::ids::UI_MENU_BACK);
            close_system_entry(system_nav, cursor);
            info!("cube system entry \u{2192} (list)");
        }
    }
}

fn focus_visual_quality_setting_row(
    settings: &UserSettings,
    system_nav: &KaleidoscopeSystemNav,
    cursor: &mut KaleidoscopeCursor,
    system: &SystemMenuParams,
) {
    let model = system.model(settings);
    let rows = system_rows_with_quality_prompt(&model, system_nav.open_entry, None);
    if let Some(idx) = rows
        .iter()
        .position(|row| *row == SystemRow::Setting(SettingsOptionId::VisualQuality))
    {
        cursor.mark_keyboard(MenuFocus::System(idx));
    }
}

/// Apply a System-face option (SELECT/confirm) by mutating `UserSettings` through
/// the shared settings IR ([`apply_settings_option`]): toggles flip, cycles +
/// sliders advance one step (confirm = next), and `Close` folds the menu. The SFX
/// is chosen from the option's IR `kind` (toggle on/off, slider tick, close).
/// Persistence is NOT re-implemented here: the existing `save_settings_on_change`
/// system writes `settings.ron` whenever `UserSettings` changes, so mutating the
/// resource is the whole job.
fn apply_system_option(
    option: SettingsOptionId,
    settings: &mut UserSettings,
    quality_confirm: &mut VisualQualityConfirmState,
    close_menu: &mut bool,
    sfx: &mut SfxWriter,
) {
    if option == SettingsOptionId::VisualQuality {
        quality_confirm.step_from(settings.video.quality.profile, 1);
        play_ui(sfx, ambition::sfx::ids::UI_SLIDER_TICK);
        info!(
            "cube system pending visual quality: {:?}",
            quality_confirm.pending()
        );
        return;
    }

    // Resolve the option's kind BEFORE mutating, so a toggle reports its NEW state
    // and a slider/cycle gets a tick. `Close` is the only kind that folds the menu.
    let kind = settings_menu_model(settings)
        .categories
        .iter()
        .flat_map(|c| c.options.iter())
        .find(|o| o.id == option)
        .map(|o| o.kind)
        .unwrap_or(SettingsOptionKind::Action);

    // Confirm advances like Next (dir 0 == next/toggle/up in the IR).
    let closed = apply_settings_option(option, 0, settings);
    if closed {
        *close_menu = true;
        play_ui(sfx, ambition::sfx::ids::UI_MENU_CLOSE);
        info!("cube system option: {:?}", option);
        return;
    }

    match kind {
        SettingsOptionKind::Toggle(_) => {
            // Read the now-current state from the rebuilt model for the on/off SFX.
            let on = settings_menu_model(settings)
                .categories
                .iter()
                .flat_map(|c| c.options.iter())
                .find(|o| o.id == option)
                .map(|o| matches!(o.kind, SettingsOptionKind::Toggle(true)))
                .unwrap_or(false);
            play_ui(
                sfx,
                if on {
                    ambition::sfx::ids::UI_TOGGLE_ON
                } else {
                    ambition::sfx::ids::UI_TOGGLE_OFF
                },
            );
        }
        SettingsOptionKind::Cycle { .. } | SettingsOptionKind::Slider { .. } => {
            play_ui(sfx, ambition::sfx::ids::UI_SLIDER_TICK);
        }
        SettingsOptionKind::Action => {}
    }
    info!("cube system option: {:?}", option);
}
