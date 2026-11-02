//! Cached System models shared by menu rendering and focus.

use super::kaleidoscope_app::{KaleidoscopeSystemNav, SystemMenuSnapshotParams};
use super::model::{system_rows_with_quality_prompt, SystemRow};
use super::quality_confirm::VisualQualityConfirmState;
use ambition_platformer2d::persistence::settings::{UserSettings, VisualQualityProfile};
use ambition_platformer2d::settings_menu::system::{
    DevSnapshot, RadioSnapshot, SystemMenuEntryId, SystemMenuModel,
};
use bevy::prelude::*;

/// The menu's System model and rows, shared by publication and pointer focus.
/// Compare input values so unchanged frames retain the model and its widgets.
#[derive(Component, Default)]
pub(crate) struct CachedSystemMenu {
    /// The System model, or `None` before the first cache update.
    pub(crate) model: Option<SystemMenuModel>,
    /// Rows for the current drill state.
    pub(crate) rows: Vec<SystemRow>,
    /// Snapshots used by page publication and the cache value key.
    pub(crate) radio: RadioSnapshot,
    pub(crate) dev: DevSnapshot,
    /// Pending visual-quality profile, if the Video screen is asking for confirmation.
    pub(crate) quality: Option<VisualQualityProfile>,
    /// The settings VALUE the cached model was built from. Compared, never
    /// tick-checked: a `ResMut<UserSettings>` taken by a navigation system
    /// dirties the tick every frame with the value unchanged.
    settings: Option<UserSettings>,
    #[cfg(feature = "input")]
    binding_state: Option<(
        ambition_platformer2d::input::ActionBindings,
        ambition_platformer2d::input::ActiveDevice,
    )>,
    /// The drill-down the cached rows were built for.
    open_entry: Option<SystemMenuEntryId>,
    /// How many times the System model was actually built. An instrument for the
    /// churn count D-CUBE-CHURN asks for: an open System face with no input must
    /// hold this at one.
    pub(crate) model_builds: u64,
}

impl CachedSystemMenu {
    /// A pointer observer can see a drill change before the next cache update.
    #[cfg(feature = "kaleidoscope_menu")]
    pub(crate) fn rows_are_current_for(&self, open_entry: Option<SystemMenuEntryId>) -> bool {
        self.model.is_some() && self.open_entry == open_entry
    }
    #[cfg(feature = "kaleidoscope_menu")]
    pub(crate) fn bindings_are_current_for(
        &self,
        menu: Entity,
        snapshot: &SystemMenuSnapshotParams,
    ) -> bool {
        #[cfg(feature = "input")]
        {
            return snapshot.bindings.state(menu)
                == self
                    .binding_state
                    .as_ref()
                    .map(|(bindings, device)| (bindings, *device));
        }
        #[cfg(not(feature = "input"))]
        {
            let _ = (menu, snapshot);
            true
        }
    }
}

/// Refresh each menu's System model only when its input values change.
pub(crate) fn cache_system_menu(
    mut menus: Query<(
        Entity,
        &KaleidoscopeSystemNav,
        &VisualQualityConfirmState,
        &mut CachedSystemMenu,
    )>,
    settings: Res<UserSettings>,
    snapshot: SystemMenuSnapshotParams,
) {
    let radio = snapshot.radio_snapshot();
    let dev = snapshot.dev_snapshot();
    for (menu, system_nav, quality_confirm, mut cache) in &mut menus {
        let quality = quality_confirm.pending();
        #[cfg(feature = "input")]
        let binding_state = snapshot.bindings.state(menu);
        #[cfg(feature = "input")]
        let bindings_changed = binding_state
            != cache
                .binding_state
                .as_ref()
                .map(|(bindings, device)| (bindings, *device));
        #[cfg(not(feature = "input"))]
        let bindings_changed = false;
        let stale = cache.model.is_none()
            || cache.settings.as_ref() != Some(&*settings)
            || cache.radio != radio
            || cache.dev != dev
            || cache.quality != quality
            || cache.open_entry != system_nav.open_entry
            || bindings_changed;
        if stale {
            let model = crate::menu::model::system_menu_model_with_pending_quality(
                &settings, &radio, &dev, quality,
            );
            #[cfg(feature = "input")]
            let model = snapshot.bindings.attach(model, menu);
            #[cfg(not(feature = "input"))]
            let _ = menu;
            cache.rows = system_rows_with_quality_prompt(&model, system_nav.open_entry, quality);
            cache.model = Some(model);
            cache.settings = Some(settings.clone());
            cache.open_entry = system_nav.open_entry;
            cache.model_builds += 1;
            #[cfg(feature = "input")]
            {
                cache.binding_state =
                    binding_state.map(|(bindings, device)| (bindings.clone(), device));
            }
        }
        cache.radio = radio.clone();
        cache.dev = dev.clone();
        cache.quality = quality;
    }
}

pub(crate) fn any_menu_visible(
    menus: Query<&ambition_platformer2d::inventory_ui::InventoryUiState>,
) -> bool {
    menus.iter().any(|menu| menu.visible)
}
