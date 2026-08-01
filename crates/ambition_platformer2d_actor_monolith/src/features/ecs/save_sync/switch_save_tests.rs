//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod switch_save_tests` block (test-organization campaign, 2026-07-10).
//! Pure move: same test names + logic, now an adjacent child module (a direct
//! sibling, so `super` path depth is unchanged) with `use super::*;`.

//! sync_ecs_switches_from_save authoritatively restores each switch's
//! on/off from the save flag keyed by its FeatureId — so a save load
//! (or a reset that rewrote flags) re-derives switch visuals/state.
use super::*;
use crate::encounter::SwitchActivation;
use ambition_persistence::save::SandboxSave;
use bevy::prelude::{App, Update};

#[test]
fn switches_restore_their_on_state_from_the_save() {
    let mut app = App::new();
    let mut save = SandboxSave::default();
    save.data_mut().set_switch("on_switch", true);
    app.insert_resource(save);
    app.add_systems(Update, sync_ecs_switches_from_save);

    // Start each switch at the OPPOSITE of its saved value to prove the
    // restore is authoritative, not an OR.
    let on = app
        .world_mut()
        .spawn((
            FeatureId::new("on_switch"),
            SwitchOn(false),
            SwitchFeature::new(SwitchActivation::default()),
        ))
        .id();
    let off = app
        .world_mut()
        .spawn((
            FeatureId::new("off_switch"), // never set in the save -> false
            SwitchOn(true),
            SwitchFeature::new(SwitchActivation::default()),
        ))
        .id();

    app.update();

    assert!(
        app.world().get::<SwitchOn>(on).unwrap().0,
        "a saved-on switch restores to on"
    );
    assert!(
        !app.world().get::<SwitchOn>(off).unwrap().0,
        "an unsaved switch is authoritatively set off"
    );
}
