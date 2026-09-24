//! Reusable developer-tooling state and simulation-side logic.
//!
//! Owns:
//!
//! - [`dev_tools`] — the [`DeveloperTools`](dev_tools::DeveloperTools) debug/
//!   gizmo toggle resource, the reflected editable player-tuning / ability /
//!   stats resources + their engine conversions, the movement/debug profile
//!   enums, and the inspector-visibility run conditions. Plus the live-edit
//!   sync systems that push inspector edits onto the authoritative player body
//!   (they name only the foundational `Body*` clusters + `PrimaryPlayerOnly`).
//! - [`profiling`] — the startup profiler marks (read by audio + setup).
//! - [`runtime_census`] — the profiling-only workload censuses (off unless
//!   `AMBITION_PROFILE_CENSUS` is set) and the clock every census samples on.
//! - [`persistence`] — `DeveloperTools` disk persistence (developer.ron).
//! - [`contribute_editable_ability_mask`] — the simulation system that offers the
//!   admitted ability mask to the primary player as a ceiling contribution.
//!
//! Presentation UI remains in `ambition_app`; gameplay tracing remains with the
//! simulation state it samples.

pub mod brain_override;
pub mod dev_tools;
/// The authored world file's mtime watch + reload status — the state half of
/// the Developer page's auto-apply row.
pub mod hot_reload;
pub mod persistence;
pub mod perception_extent;
pub mod population_cap;
pub mod profiling;
/// The profiling-only workload censuses and the shared census clock they sample on.
pub mod runtime_census;
pub mod sim_plugin;

pub use hot_reload::{poll_world_source_changes, WorldSourceHotReload};
pub use persistence::DeveloperPersistenceSchedulePlugin;
pub use sim_plugin::{DevInspectorMirrorSet, DevToolsSimPlugin};

use bevy::prelude::*;

use ambition_platformer2d_shared_tangle::markers::PrimaryPlayerOnly;
use dev_tools::EditableAbilitySet;

/// Marker type that owns the editable ability-set domain in
/// `PendingMechanicalEdits`. Identity is the type, not the label; see
/// `MechanicalDomain`.
pub struct EditableAbilitySetDomain;

pub fn ability_set_domain() -> ambition_platformer2d_core::MechanicalDomain {
    ambition_platformer2d_core::MechanicalDomain::of::<EditableAbilitySetDomain>(
        "editable_ability_set",
    )
}

/// Raise a changed developer ability selection as a proposal.
///
/// Runs in `PreUpdate`, not the sim schedule. Reading the live
/// `EditableAbilitySet` inside `GgrsSchedule` would apply the current inspector
/// value to body state that a rewind had just restored.
pub fn propose_editable_abilities(
    editable: Res<EditableAbilitySet>,
    mut pending: ResMut<ambition_platformer2d_core::PendingMechanicalEdits>,
) {
    // Exclude `is_added`: Bevy counts insertion as a change, and the mirror is
    // inserted before content seeds. Proposing that would stop the new session on
    // frame one.
    if !editable.is_changed() || editable.is_added() {
        return;
    }
    pending.propose(ability_set_domain());
}

/// Admit a developer ability selection. It touches no body.
///
/// Admission and projection are separate, as for `ActivePlayerBodyProfile`.
/// Admission belongs to the host frame and the rollback mutation boundary.
/// Projection must follow body existence, so a body rebuilt during the
/// simulation (reset, room load) gets its abilities on the same tick. See
/// [`contribute_editable_ability_mask`].
pub fn admit_editable_abilities(
    editable_abilities: Res<EditableAbilitySet>,
    mut active_mask: ResMut<dev_tools::ActiveEditableAbilityMask>,
    admission: Option<Res<ambition_platformer2d_core::MechanicalEditAdmission>>,
    mut pending: ResMut<ambition_platformer2d_core::PendingMechanicalEdits>,
) {
    let proposed = pending.is_pending(ability_set_domain());
    if proposed
        && matches!(
            admission.as_deref(),
            Some(ambition_platformer2d_core::MechanicalEditAdmission::Refuse)
        )
    {
        return;
    }
    if proposed {
        active_mask.0 = Some(editable_abilities.as_engine());
        pending.take(ability_set_domain());
    } else if active_mask.0.is_none() {
        // Seed the baseline when nothing is pending. The continuous `base ∩ mask`
        // reconciliation is not a mechanical edit and must work from frame one.
        active_mask.0 = Some(editable_abilities.as_engine());
    }
}

/// This crate's key in the primary player's [`AbilityContributions`].
///
/// [`AbilityContributions`]: ambition_platformer2d_core::AbilityContributions
pub const EDITABLE_ABILITY_MASK: &str = "dev.editable_ability_mask";

/// Contribute the admitted mask to the primary player, if any, as a ceiling
/// over its verbs. `project_body_abilities` turns it into the effective set.
/// A mask can only turn a verb off, never add one the character was not
/// authored with.
///
/// It runs in the simulation, where bodies are built, so a new body gets its
/// admitted abilities on the same tick. It reads the mask, not the editor
/// resource (see [`dev_tools::ActiveEditableAbilityMask`]). While an edit
/// awaits admission the previous contribution stays, so a refused edit reaches
/// no body.
pub fn contribute_editable_ability_mask(
    active_mask: Res<dev_tools::ActiveEditableAbilityMask>,
    pending: Res<ambition_platformer2d_core::PendingMechanicalEdits>,
    mut player_q: Query<&mut ambition_platformer2d_core::AbilityContributions, PrimaryPlayerOnly>,
) {
    if pending.is_pending(ability_set_domain()) {
        return;
    }
    let Some(mask) = active_mask.0 else {
        return;
    };
    let Ok(mut contributions) = player_q.single_mut() else {
        return;
    };
    let ceiling = ambition_platformer2d_core::AbilityContribution::Ceiling(mask);
    if contributions.get(EDITABLE_ABILITY_MASK) != Some(ceiling) {
        contributions.set(EDITABLE_ABILITY_MASK, ceiling);
    }
}

/// Developer/debug state: debug flags and the HUD flash timer. Keyboard preset
/// selection is owned by persisted user settings, not duplicated here.
#[derive(Resource)]
pub struct DeveloperRuntimeState {
    pub debug: bool,
    pub slowmo: bool,
    /// How far slow-motion slows the sim clock when [`Self::slowmo`] is on.
    pub slowmo_scale: f32,
    pub preset_flash: f32,
}

/// Ask the clock to slow while developer slow-motion is on.
///
/// The dev crate sends a `ClockRequester::DevTool` request; the kernel does
/// not read developer state. `RegimePolicy` grants it in `Solo` and denies it
/// in `RLDeterministic` and `Cinematic`. `apply_clock_scale_requests` takes
/// the `min`, so the strongest slowdown wins and order does not matter.
///
/// Write every frame, not on the toggle edge: requests are reduced per frame,
/// so a one-shot write is replaced by the default on the next tick.
pub fn request_developer_slow_motion(
    dev_state: bevy::prelude::Res<DeveloperRuntimeState>,
    mut writer: bevy::prelude::MessageWriter<ambition_time::time_control::ClockScaleRequest>,
) {
    if !dev_state.slowmo {
        return;
    }
    writer.write(ambition_time::time_control::ClockScaleRequest {
        domain: ambition_time::ClockDomain::SimClock,
        scale: dev_state.slowmo_scale,
        requester: ambition_time::time_control::ClockRequester::DevTool,
        reason: "dev_slowmo",
    });
}

/// Wind down the HUD's preset flash.
///
/// The value is written by a room commit and read by the app's HUD; nothing in
/// the sim reads it.
///
/// Runs in `Update` on the render clock, not in the sim schedule. It is not
/// rollback state, so in a rewinding schedule each resimulated tick would
/// decay it again.
pub fn decay_developer_presentation_flash(
    time: ambition_time::PresentationTime,
    mut dev_state: bevy::prelude::ResMut<DeveloperRuntimeState>,
) {
    dev_state.preset_flash = (dev_state.preset_flash - time.wall_dt()).max(0.0);
}

impl Default for DeveloperRuntimeState {
    fn default() -> Self {
        Self {
            debug: false,
            slowmo: false,
            // The value it carried in the feel table.
            slowmo_scale: 0.25,
            preset_flash: 1.2,
        }
    }
}

impl DeveloperRuntimeState {
    pub fn debug_enabled(&self) -> bool {
        self.debug
    }
}

/// Which layers of the combat overlay to draw.
///
/// The layers are independent because the questions are. "Is this volume
/// inside the sprite?" needs the art; "where does this reach?" is easier
/// without it; "why did this miss?" wants hurtboxes without strikes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CombatOverlayLayers {
    /// The rendered character art. Off draws the world grid instead, which is
    /// what `hide_sprites` has always meant.
    pub art: bool,
    /// The coarse collision envelope and the effective hurtboxes.
    pub hurtboxes: bool,
    /// Live strike volumes and the move/timing readout.
    pub strikes: bool,
}

impl Default for CombatOverlayLayers {
    /// Everything: the preset a tool asking for "the combat overlay" means.
    fn default() -> Self {
        Self {
            art: true,
            hurtboxes: true,
            strikes: true,
        }
    }
}

/// Turn the combat overlay on, everywhere it is gated.
///
/// The gizmo pass has three gates: the debug flag, the gizmo toggle, and the
/// per-view fields. All are off in a plain build, and missing any one gives a
/// picture with no hit volumes. Tools that want combat geometry call this, so
/// the gates are listed in one place.
///
/// Idempotent: safe to call every frame, which is what a capture tool must do —
/// settings load and the developer-tools default both write this state, so a
/// startup-only write is a race against whichever of them runs last.
pub fn force_combat_overlay(
    state: &mut DeveloperRuntimeState,
    tools: &mut dev_tools::DeveloperTools,
    layers: CombatOverlayLayers,
) {
    if !state.debug {
        state.debug = true;
    }
    if !tools.gizmos_enabled {
        tools.gizmos_enabled = true;
    }
    if tools.debug_view_mode != dev_tools::DebugViewMode::Combat {
        tools.apply_debug_view_mode(dev_tools::DebugViewMode::Combat, false);
    }
    // The preset turns on the combined gate, which draws both halves whatever
    // the per-layer fields say, so clear it. `draw_combat_geometry_view` reads
    // `show_player_hitbox || show_feature_hitboxes` for the hurt half and
    // `show_combat_preview || show_feature_hitboxes` for the strikes.
    tools.show_feature_hitboxes = false;
    tools.show_player_hitbox = layers.hurtboxes;
    tools.show_combat_preview = layers.strikes;
    tools.hide_sprites = !layers.art;
}

#[cfg(test)]
mod ability_admission_tests {
    use super::*;

    /// Admission must not depend on a body existing. Otherwise an edit proposed
    /// while the primary player is absent stays pending and re-enters the
    /// admission decision every frame. The fixture has no player on purpose. The
    /// projection half is covered by `avatar::starting_character::tests::live_refresh`.
    #[test]
    fn an_ability_edit_is_admitted_with_no_player_to_project_onto() {
        let mut app = App::new();
        app.init_resource::<EditableAbilitySet>();
        app.init_resource::<dev_tools::ActiveEditableAbilityMask>();
        app.init_resource::<ambition_platformer2d_core::PendingMechanicalEdits>();
        app.init_resource::<ambition_platformer2d_core::MechanicalEditAdmission>();
        app.add_systems(Update, (admit_editable_abilities, contribute_editable_ability_mask).chain());

        // A mask that differs from the default, so "it arrived" is a real
        // difference rather than two defaults agreeing.
        let default_mask = EditableAbilitySet::default();
        {
            let mut editable = app.world_mut().resource_mut::<EditableAbilitySet>();
            editable.double_jump = !default_mask.double_jump;
        }
        let proposed = app.world().resource::<EditableAbilitySet>().as_engine();
        assert_ne!(
            proposed,
            default_mask.as_engine(),
            "the fixture proposed the default mask, so the assertion below would \
             hold whether or not anything was admitted",
        );
        app.world_mut()
            .resource_mut::<ambition_platformer2d_core::PendingMechanicalEdits>()
            .propose(ability_set_domain());

        app.update();

        assert!(
            !app.world()
                .resource::<ambition_platformer2d_core::PendingMechanicalEdits>()
                .is_pending(ability_set_domain()),
            "the proposal is STILL pending with no player in the world, so it \
             re-enters the admission/rebase decision on every frame until a body \
             happens to exist",
        );
        assert_eq!(
            app.world()
                .resource::<dev_tools::ActiveEditableAbilityMask>()
                .0,
            Some(proposed),
            "nothing recorded WHAT was admitted, so a body built later projects \
             whatever the editor panel holds at that moment instead",
        );
    }

    /// A refused edit is not admitted. This fails if admission drained the domain
    /// unconditionally.
    #[test]
    fn a_refused_ability_edit_stays_pending_and_admits_nothing() {
        let mut app = App::new();
        app.init_resource::<EditableAbilitySet>();
        app.init_resource::<dev_tools::ActiveEditableAbilityMask>();
        app.init_resource::<ambition_platformer2d_core::PendingMechanicalEdits>();
        app.insert_resource(ambition_platformer2d_core::MechanicalEditAdmission::Refuse);
        app.add_systems(Update, (admit_editable_abilities, contribute_editable_ability_mask).chain());
        app.world_mut()
            .resource_mut::<ambition_platformer2d_core::PendingMechanicalEdits>()
            .propose(ability_set_domain());

        app.update();

        assert!(
            app.world()
                .resource::<ambition_platformer2d_core::PendingMechanicalEdits>()
                .is_pending(ability_set_domain()),
            "a REFUSED edit was drained: the refusal became a front door the \
             unadmitted value walks through",
        );
        assert_eq!(
            app.world()
                .resource::<dev_tools::ActiveEditableAbilityMask>()
                .0,
            None,
            "a refused edit was recorded as the admitted mask",
        );
    }
}

#[cfg(test)]
mod developer_runtime_state_tests {
    use super::*;

    #[test]
    fn debug_overlay_defaults_off_for_every_game() {
        assert!(!DeveloperRuntimeState::default().debug);
    }

    /// The HUD flash winds down.
    ///
    /// The decay itself is tested here. Other `DevToolsSimPlugin` systems need
    /// resources from crates this one does not depend on, so the registration is
    /// checked in the shipped app (see below).
    #[test]
    fn the_developer_flash_decays_through_a_system_this_crate_registers() {
        use bevy::prelude::*;

        // 1. The decay, and its floor. A HUD that tests `> 0.0` needs the clamp.
        let mut world = World::new();
        world.insert_resource(DeveloperRuntimeState {
            preset_flash: 0.05,
            ..Default::default()
        });
        let mut time = Time::<()>::default();
        time.advance_by(std::time::Duration::from_millis(20));
        world.insert_resource(time);
        world.insert_resource(ambition_time::ClockState::default());
        let mut run =
            bevy::ecs::system::IntoSystem::into_system(decay_developer_presentation_flash);
        run.initialize(&mut world);
        run.run((), &mut world).expect("the decay system runs");
        let once = world.resource::<DeveloperRuntimeState>().preset_flash;
        assert!(once < 0.05, "the flash did not wind down at all: {once}");
        run.run((), &mut world).expect("the decay system runs");
        run.run((), &mut world).expect("the decay system runs");
        assert_eq!(
            world.resource::<DeveloperRuntimeState>().preset_flash,
            0.0,
            "the flash ran past zero, so a HUD asking `> 0.0` never stops drawing it"
        );

        // 2. The registration guard is `the_developer_hud_flash_still_winds_down`
        //    in `game/ambition_app/tests/`, which boots a real app. A bare
        //    `App::new()` cannot run this plugin's schedule, and without a Bevy debug
        //    feature the graph does not report system names.
    }
}
