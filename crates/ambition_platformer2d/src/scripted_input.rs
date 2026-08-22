//! **Drive the local participant from a script, through the production input
//! pipeline.**
//!
//! ⛔⛔ **THIS EXISTS BECAUSE EIGHT FIXTURES INDEPENDENTLY LEARNED THE SAME
//! ORDERING, AND FIVE OF THEM LEARNED IT THE HARD WAY.** A scripted control
//! frame written in `PreUpdate` is overwritten by the participant pipeline in
//! `Update` — the pipeline lives behind `ambition_platformer2d/input`, which
//! workspace feature unification turns on from `ambition_app`'s defaults
//! whatever the test's own crate asked for. So the fixtures compiled, the
//! script ran, and the body never moved.
//!
//! The dangerous half was not the tests that went red. It was the NEGATIVE
//! ones: a test asserting *"this ability is not available"* stayed green
//! because the button never reached the simulation at all, which is a proof
//! that nobody pressed anything dressed up as a proof that pressing did
//! nothing.
//!
//! ⭐ **so the ordering is stated ONCE, against the authority rather than
//! against a guess:**
//!
//! ```text
//! after   InputSet::Route              where the participant pipeline declares
//!                                      its raw-frame writers
//! before  PrimarySlotInputCommit      the publication boundary every host
//!                                      has — a write that misses it never
//!                                      reaches gameplay however late in
//!                                      `Update` it lands
//! ```
//!
//! ⚠ ordering against a set or system nobody composed is a no-op, so a headless
//! frame-stepped composition (which has no latch) is unaffected by both edges.
//!
//! ⚠ **the local participant, singular — and it now says WHICH seat rather than
//! implying one.** This writes
//! [`PlayerSlot::PRIMARY`](ambition_characters::control::PlayerSlot::PRIMARY)'s row
//! of [`SeatRawFrames`](ambition_characters::control::SeatRawFrames), the seat every
//! one of these fixtures scripts. It used to write the global `ControlFrame`,
//! where "the primary seat" was not stated anywhere — it was what that resource
//! happened to mean. Scripting a second seat is one line from here the day a
//! fixture wants it; until then the limit is visible instead of structural.

use bevy::prelude::*;

use ambition_characters::control::{PlayerSlot, SeatRawFrames, SlotControls};
use ambition_platformer2d_core::ControlFrame;

/// **What the fixture wants the local participant to be holding.**
///
/// Written every frame, so a script that stops setting it holds the last frame
/// — the same thing a person holding a stick does.
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct ScriptedControls(pub ControlFrame);

/// **What the SIMULATION actually received** — the other half of the pair, and
/// the reason a false-green negative test is harder to write now.
///
/// ⭐ it counts what the sim's own slot table carried, not what the fixture
/// asked for. A script whose write is overwritten before the sim sees it
/// requests plenty and delivers nothing, which is exactly the state that used
/// to read as a passing negative proof.
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

    /// **Positive evidence to pair with a negative assertion.**
    ///
    /// ⛔ call this in any test whose claim is that something did NOT happen.
    /// Without it, "the ability never fired" and "the button never arrived" are
    /// the same green.
    ///
    /// ⚠⚠ **DELIVERY IS OBSERVED ONE FRAME LATE, so this is an END-OF-RUN check
    /// and not a first-frame precondition.** Bevy runs `FixedUpdate` BEFORE
    /// `Update` within a frame, so the slot table this reads was published from
    /// the frame written on the PREVIOUS `app.update()`. Asserting it
    /// immediately after the first scripted press reports `1 requested, 0
    /// delivered` on a perfectly healthy pipeline — which is how it was first
    /// misused, and the reason the contract is written down here rather than
    /// left for each caller to rediscover. For the same-frame question ("did my
    /// write survive routing?"), read `ControlFrame` right after the update.
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

/// **Install the scripted participant stream on `app`.**
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
            // ⛔⛔ **the BOUNDARY SET, never a named system.** This said
            // `.before(commit_seat_raw_frames)`, and that system exists only on a
            // host with a frame→tick latch — so on a frame-stepped composition
            // the edge was vacuous and the scripted write raced the publish,
            // landing a frame late or not at all. Ordering against a system
            // nobody composed is a silent no-op; ordering against
            // `PrimarySlotInputCommit` is the same guarantee on every host,
            // which is why the boundary is a set.
            .before(ambition_platformer2d_runtime::host_input::PrimarySlotInputCommit)
            // ⚠ **and the latch fold by name, because it is in ANOTHER
            // schedule.** On a fixed-tick or rollback host the commit set lives
            // in the sim schedule while the fold runs in `Update`, and a
            // cross-schedule set edge orders nothing. Both edges are stated; each
            // is vacuous exactly where the other applies.
            .before(ambition_platformer2d_runtime::host_input::commit_seat_raw_frames)
            .before(
                ambition_platformer2d_runtime::host_input::publish_seat_controls_when_nobody_else_does,
            ),
    );
    // ⚠ **`Last`, and it reads the SLOT TABLE.** The observation has to come
    // from the far side of the pipeline or it would only be restating the write
    // — and the slot table is what the brains read, so a frame counted here is
    // a frame gameplay could act on.
    app.add_systems(Last, observe_delivered_controls);
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
