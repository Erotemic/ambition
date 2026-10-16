//! Drive scripted local input through the production participant pipeline.
//!
//! Scripted input runs after `InputSet::Route` and before
//! `PrimarySlotInputCommit`, so routed input cannot overwrite the script and the
//! script reaches the same publication boundary as live input. Ordering against
//! sets absent from a headless composition is a no-op.

use bevy::prelude::*;

use ambition_characters::control::{PlayerSlot, SeatRawFrames, SlotControls};
use ambition_platformer2d_core::ControlFrame;

/// What the fixture wants the local participant to be holding.
///
/// Written every frame, so a script that stops setting it holds the last frame
/// — the same thing a person holding a stick does.
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct ScriptedControls(pub ControlFrame);

/// Counts scripted input requested by the fixture and actually delivered by
/// the simulation's slot table.
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct ScriptedControlsObserved {
    /// Frames on which the script asked for something other than neutral.
    pub requested: u32,
    /// Frames on which the simulation's primary slot carried something other
    /// than neutral.
    pub delivered: u32,
}

impl ScriptedControlsObserved {
    /// Did any scripted press reach the simulation at all?
    pub fn reached_the_simulation(&self) -> bool {
        self.delivered > 0
    }

    /// Assert positive evidence that at least one scripted press reached the
    /// simulation. For same-frame routing checks, inspect `ControlFrame` directly
    /// after the update.
    #[track_caller]
    pub fn assert_the_script_reached_the_simulation(&self) {
        assert!(
            self.reached_the_simulation(),
            "the scripted controls never reached the simulation: {} frame(s) asked for a \
             press and {} arrived in the primary slot. Every 'nothing happened' assertion \
             in this test is therefore vacuous — it proves nobody pressed anything, not \
             that pressing did nothing.",
            self.requested,
            self.delivered,
        );
    }
}

/// Install the scripted participant stream on `app`.
///
/// Idempotent in the sense that matters: it inserts both resources with their
/// defaults, so a fixture may call it before or after it knows what the script
/// is.
pub fn drive_the_local_participant(app: &mut App) {
    app.init_resource::<ScriptedControls>();
    app.init_resource::<ScriptedControlsObserved>();
    app.add_systems(
        Update,
        write_scripted_controls
            .after(ambition_input::InputSet::Route)
            // Ordering against a system nobody composed is a silent no-op; ordering against
            // `PrimarySlotInputCommit` is the same guarantee on every host, which is why the
            // boundary is a set.
            //
            // ⛔⛔ ~~Both edges are stated; each is vacuous exactly where the other
            // applies.~~ THE TWO SYSTEM EDGES ARE GONE, AND THE BELT-AND-BRACES
            // READING WAS MEASURED RATHER THAN ARGUED. Both named systems are
            // installed INTO this set in production — `commit_seat_raw_frames` by
            // `ambition_platformer2d_host` and
            // `publish_seat_controls_when_nobody_else_does` by the runtime's
            // player schedule — so the set edge already covers every production
            // composition.
            //
            // ⚠ AND THE ONE FIXTURE THAT COMPOSES EITHER SYSTEM WITHOUT THE SET
            // DOES NOT INSTALL THIS ONE. `avatar/systems/tests.rs` names
            // `publish_seat_controls_when_nobody_else_does` and has zero
            // references to `write_scripted_controls`, `ScriptedControls` or
            // `drive_the_local_participant` — so the edge was vacuous there too,
            // not protective. A peer read the same rows and reached the opposite
            // conclusion from the target's registration alone; what settles it is
            // whether the ORDERED-FROM system is present, which is a different
            // question and the one that matters.
            .before(ambition_platformer2d_runtime::host_input::PrimarySlotInputCommit),
    );
    // `Last`, and it reads the SLOT TABLE. The observation has to come
    // from the far side of the pipeline or it would only be restating the write
    // — and the slot table is what the brains read, so a frame counted here is
    // a frame gameplay could act on.
    app.add_systems(Last, observe_delivered_controls);
}

/// The env var that names the game whose player plays itself, by its hub game
/// id: `AMBITION_AUTOPLAY=mary_o`. For unattended profiling and soak runs.
///
/// A host calls [`install_autoplay_from_env`]. Each game owns its policy: a system in
/// [`AutoplayPolicy`] gated by [`autoplay_is`] with its own id, which writes
/// the [`crate::autopilot::AutopilotOrder`]: a goal the autopilot goes to by
/// navigation, or a press of the game's own. A game with no policy is not
/// driven.
pub const AUTOPLAY_ENV: &str = "AMBITION_AUTOPLAY";

/// The game [`AUTOPLAY_ENV`] names, if any: read at run time, or, for a build
/// with no environment at run time (an APK), at compile time. Every host
/// reads it here.
pub fn autoplay_from_env() -> Option<String> {
    std::env::var(AUTOPLAY_ENV)
        .ok()
        .or_else(|| option_env!("AMBITION_AUTOPLAY").map(str::to_owned))
        .map(|game| game.trim().to_owned())
        .filter(|game| !game.is_empty())
}

/// [`install_autoplay`] for the game [`autoplay_from_env`] names, if any.
pub fn install_autoplay_from_env(app: &mut App) {
    if let Some(game) = autoplay_from_env() {
        eprintln!("[census] config autoplay={game}");
        install_autoplay(app, game);
    }
}

/// The game the scripted participant plays, from [`AUTOPLAY_ENV`].
#[derive(Resource, Clone, Debug, PartialEq, Eq)]
pub struct Autoplay(pub String);

/// Where a game's autoplay policy runs: each frame, before the script is
/// written into the seat.
#[derive(bevy::ecs::schedule::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AutoplayPolicy;

/// Run condition of a game's autoplay policy: true while [`Autoplay`] names `game`.
pub fn autoplay_is(game: &'static str) -> impl Fn(Option<Res<Autoplay>>) -> bool + Clone {
    move |autoplay: Option<Res<Autoplay>>| autoplay.is_some_and(|autoplay| autoplay.0 == game)
}

/// Let `game`'s autoplay policy drive the local participant. While it runs,
/// the script owns the primary seat: device and touch input are not read.
pub fn install_autoplay(app: &mut App, game: impl Into<String>) {
    drive_the_local_participant(app);
    app.insert_resource(Autoplay(game.into()));
    app.configure_sets(Update, AutoplayPolicy.before(write_scripted_controls));
    // A policy gives a goal; the autopilot goes there by navigation.
    crate::autopilot::install(app);
    app.configure_sets(Update, crate::autopilot::AutopilotDrive.before(write_scripted_controls));
}

/// Set what the local participant is holding from here on.
pub fn hold(app: &mut App, frame: ControlFrame) {
    app.world_mut().resource_mut::<ScriptedControls>().0 = frame;
}

/// Let go of everything.
pub fn release(app: &mut App) {
    hold(app, ControlFrame::default());
}

/// What the simulation received so far.
pub fn observed(app: &App) -> ScriptedControlsObserved {
    *app.world().resource::<ScriptedControlsObserved>()
}

fn write_scripted_controls(
    script: Res<ScriptedControls>,
    mut raw: ResMut<SeatRawFrames>,
    mut observed: ResMut<ScriptedControlsObserved>,
) {
    raw.set(PlayerSlot::PRIMARY, script.0);
    if script.0 != ControlFrame::default() {
        observed.requested = observed.requested.saturating_add(1);
    }
}

fn observe_delivered_controls(
    slots: Option<Res<SlotControls>>,
    mut observed: ResMut<ScriptedControlsObserved>,
) {
    let Some(slots) = slots else {
        return;
    };
    if slots.get(PlayerSlot::PRIMARY) != ControlFrame::default() {
        observed.delivered = observed.delivered.saturating_add(1);
    }
}
