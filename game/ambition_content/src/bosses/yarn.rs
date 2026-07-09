//! Cut-rope boss Yarn vocabulary + mirror feed.
//!
//! The generic dialog runtime owns the shared commands/functions
//! (`set_flag`, `inventory_has`, …); this module supplies the
//! Smirking Behemoth's named vocabulary through the
//! [`YarnContentBindings`] installer seam and mirrors the boss-room
//! state into the [`YarnStateMirrorData::extras`] map so the authored
//! scripts can branch on it without the dialog runtime naming content.

use bevy::prelude::*;
use bevy_yarnspinner::prelude::DialogueRunner;
use std::sync::Arc;

use ambition_dialog::{YarnStateMirror, YarnStateMirrorData};
use ambition_combat::SetFlagRequested;
use ambition_persistence::save::SandboxSave;

use super::{CutRopeHeavyObjectCycle, PendingCutRopeRoomReplay};

/// Mirror-extras key for the heavy object currently hanging in the
/// cut-rope room.
const HEAVY_OBJECT_KEY: &str = "cut_rope_heavy_object";

/// `<<watch_cut_rope_video>>` — authored post-boss branch placeholder.
///
/// TODO(web-open): desktop builds could optionally open the URL in the user's
/// browser after a settings/privacy opt-in. For now the Yarn line presents the
/// link and this command records the choice as a save flag for traceability.
pub fn cmd_watch_cut_rope_video(mut effects: MessageWriter<SetFlagRequested>) {
    info!(
        target: "ambition_actors::dialog::yarn",
        "watch_cut_rope_video: TODO optional browser launch for https://www.youtube.com/watch?v=ucLGm27DDL0",
    );
    effects.write(SetFlagRequested {
        id: "smirking_behemoth_video_suggested".into(),
        on: true,
    });
}

/// `<<reset_cut_rope_room>>` — replay the Smirking Behemoth room from the start.
///
/// The command is reached by Yarn immediately after the NPC's final line is
/// presented, before the player has dismissed that line. Latch a pending replay
/// resource instead of resetting immediately; the simulation emits the real
/// replay request once `DialogState` is inactive.
pub fn cmd_reset_cut_rope_room(mut pending: ResMut<PendingCutRopeRoomReplay>) {
    pending.requested = true;
    info!(
        target: "ambition_actors::dialog::yarn",
        "reset_cut_rope_room: latched Smirking Behemoth room replay until dialogue closes",
    );
}

/// [`YarnContentBindings`](ambition_dialog::YarnContentBindings)
/// installer: register the cut-rope commands + the
/// `cut_rope_heavy_object_is(id)` library function on the runner.
pub fn install_cut_rope_yarn_bindings(
    commands: &mut Commands,
    runner: &mut DialogueRunner,
    mirror: &YarnStateMirror,
) {
    let watch_id = commands.register_system(cmd_watch_cut_rope_video);
    let reset_id = commands.register_system(cmd_reset_cut_rope_room);
    let cmds = runner.commands_mut();
    cmds.add_command("watch_cut_rope_video", watch_id);
    cmds.add_command("reset_cut_rope_room", reset_id);
    // cut_rope_heavy_object_is(id) -> bool: lets authored Yarn branch on the
    // runtime-selected heavy object without reaching into Bevy resources.
    let m = Arc::clone(&mirror.0);
    runner
        .library_mut()
        .add_function("cut_rope_heavy_object_is", move |id: String| -> bool {
            m.read()
                .map(
                    |snap: std::sync::RwLockReadGuard<'_, YarnStateMirrorData>| {
                        snap.extras.get(HEAVY_OBJECT_KEY).map(String::as_str) == Some(id.as_str())
                    },
                )
                .unwrap_or(false)
        });
}

/// Per-frame mirror feed: publish the cut-rope room's current heavy
/// object into the Yarn mirror extras. Runs after the generic
/// `refresh_yarn_state_mirror` and keeps the old save-gated behavior
/// (no save → no value, so `cut_rope_heavy_object_is(...)` is false).
pub fn mirror_cut_rope_heavy_object(
    save: Option<Res<SandboxSave>>,
    cycle: Option<Res<CutRopeHeavyObjectCycle>>,
    mirror: Res<YarnStateMirror>,
) {
    let mut snap = mirror.0.write().expect("YarnStateMirror poisoned");
    if save.is_none() {
        snap.extras.remove(HEAVY_OBJECT_KEY);
        return;
    }
    let id = cycle
        .map(|cycle| cycle.current_dialogue_id())
        .unwrap_or("anvil");
    snap.extras
        .insert(HEAVY_OBJECT_KEY.to_string(), id.to_string());
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The extras feed must reproduce the old inline behavior: with a
    /// save present the current cycle id (or the "anvil" default) is
    /// readable; without a save the key is absent → predicate false.
    #[test]
    fn heavy_object_extra_mirrors_cycle_and_save_gating() {
        let mut app = App::new();
        app.init_resource::<YarnStateMirror>();
        app.add_systems(Update, mirror_cut_rope_heavy_object);

        // No save resource: key absent.
        app.update();
        {
            let mirror = app.world().resource::<YarnStateMirror>();
            let snap = mirror.0.read().unwrap();
            assert!(snap.extras.get(HEAVY_OBJECT_KEY).is_none());
        }

        // Save present, no cycle resource: defaults to "anvil".
        app.insert_resource(SandboxSave::default());
        app.update();
        {
            let mirror = app.world().resource::<YarnStateMirror>();
            let snap = mirror.0.read().unwrap();
            assert_eq!(
                snap.extras.get(HEAVY_OBJECT_KEY).map(String::as_str),
                Some("anvil")
            );
        }
    }
}
