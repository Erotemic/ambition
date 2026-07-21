//! **The external-effect quarantine, against the real simulation.**
//!
//! `ambition_runtime::external_effects`' own tests drive the four systems
//! directly and prove the *rule*: a re-simulated frame replaces what the
//! abandoned pass produced, confirmed frames release in order, each intent
//! exactly once. What they cannot prove is that the plugin's schedule placement
//! carries that rule through a real host — that the outbox clear really runs
//! before gameplay, the journal really runs after it, and the release really
//! lands ahead of presentation.
//!
//! So this boots the actual Ambition sim on the actual GGRS host and asks the
//! sharpest question available without a network:
//!
//! > **Does rolling back change what presentation observes?**
//!
//! It must not. The same input script is played twice through the same host —
//! once with `check_distance = 0` (GGRS never rewinds, so every effect is
//! ground truth) and once with `check_distance = 4` (GGRS rewinds and
//! resimulates every single step). The two runs must produce the *same effects
//! in the same order*. Without the quarantine the rewinding run emits each sound
//! roughly five times over, which is the bug in its original observable form.
//!
//! Deliberately NOT claimed here: a mispredicted remote input. A sync test
//! resimulates with the *same* inputs, so its correction always equals its
//! prediction and A-versus-B cannot arise. That case is proven against the real
//! systems in `external_effects/tests.rs`
//! (`a_corrected_frame_replaces_what_the_prediction_produced` and the
//! produces-nothing variant); proving it against a live two-peer session is owed
//! when a Matchbox transport lands, and is tracked in `tracks.md` §1.

#![cfg(feature = "rl_sim")]

use ambition::runtime::external_effects::ExternalEffectJournal;
use ambition::sfx::{OwnedSfxMessage, SfxMessage};
use ambition_app::rl_sim::{AgentAction, AmbitionSim, SandboxSim, SandboxSimOptions, TimestepMode};
use bevy::ecs::message::{MessageCursor, Messages};

/// How far past the end to keep stepping so the rewinding run's pending tail
/// confirms. Must exceed the largest `check_distance` used below.
const FLUSH_STEPS: usize = 12;

fn sim_with_rewind_distance(check_distance: usize) -> SandboxSim {
    SandboxSim::new_with_options(
        SandboxSimOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_sync_test_rollback_settings(check_distance, 10),
    )
    .expect("Ambition GGRS sync-test harness builds")
}

/// Inputs chosen to emit sound often: jumps, dashes and attacks all publish
/// through `SfxWriter`.
fn noisy_action(frame: usize) -> AgentAction {
    AgentAction {
        move_x: if frame % 24 < 12 { 1.0 } else { -1.0 },
        jump: frame % 7 == 0,
        jump_held: frame % 7 < 3,
        dash: frame % 11 == 4,
        attack: frame % 5 == 2,
        ..AgentAction::default()
    }
}

/// A released effect, reduced to something comparable across runs. Positions
/// are quantized because the two runs advance the same simulation but the
/// harness's own float bookkeeping need not be bit-identical for this claim.
fn fingerprint(message: &OwnedSfxMessage) -> String {
    let (name, pos) = match message.request {
        SfxMessage::Jump { pos } => ("jump", pos),
        SfxMessage::DoubleJump { pos } => ("double_jump", pos),
        SfxMessage::Dash { pos } => ("dash", pos),
        SfxMessage::Blink { pos, .. } => ("blink", pos),
        SfxMessage::Pogo { pos } => ("pogo", pos),
        SfxMessage::Land { pos } => ("land", pos),
        SfxMessage::Slash { pos } => ("slash", pos),
        SfxMessage::Hit { pos } => ("hit", pos),
        SfxMessage::Death { pos } => ("death", pos),
        SfxMessage::Reset { pos } => ("reset", pos),
        SfxMessage::Play { id, pos } => return format!("play:{id:?}@{:.0},{:.0}", pos.x, pos.y),
    };
    format!("{name}@{:.0},{:.0}", pos.x, pos.y)
}

/// Play the script and return every effect presentation would have observed,
/// in the order it would have observed it, plus the journal's own exactly-once
/// counter.
fn observed_effects(check_distance: usize, steps: usize) -> (Vec<String>, u64) {
    let mut sim = sim_with_rewind_distance(check_distance);
    let mut cursor = MessageCursor::<OwnedSfxMessage>::default();
    let mut seen = Vec::new();

    for frame in 0..(steps + FLUSH_STEPS) {
        // Keep driving real input only for the scripted window; the flush tail
        // is idle so both runs end in the same place.
        let action = if frame < steps {
            noisy_action(frame)
        } else {
            AgentAction::default()
        };
        sim.step(action);
        sim.rollback_health()
            .unwrap_or_else(|error| panic!("frame {frame}: {error}"));

        let messages = sim.world().resource::<Messages<OwnedSfxMessage>>();
        seen.extend(cursor.read(messages).map(fingerprint));
    }

    let released = sim
        .world()
        .get_resource::<ExternalEffectJournal<OwnedSfxMessage>>()
        .map_or(0, ExternalEffectJournal::released);
    (seen, released)
}

/// The gate. A host that rewinds and resimulates every step must deliver the
/// same sounds, in the same order, as a host that never rewinds at all.
#[test]
fn rewinding_does_not_change_what_presentation_observes() {
    const STEPS: usize = 48;

    let (never_rewinds, _) = observed_effects(0, STEPS);
    let (rewinds_constantly, released) = observed_effects(4, STEPS);

    assert!(
        !never_rewinds.is_empty(),
        "the input script must actually make noise, or this proves nothing"
    );
    assert_eq!(
        rewinds_constantly, never_rewinds,
        "a rollback host delivered a different sequence of sounds than a host \
         that never rewound the same simulation"
    );
    assert_eq!(
        released as usize,
        rewinds_constantly.len(),
        "the journal's own exactly-once counter must agree with what was observed"
    );
}

/// Guards the test above from passing for the wrong reason. If `check_distance`
/// 4 stopped producing rewinds, `rewinding_does_not_change_...` would compare
/// two identical non-rewinding runs and pass while proving nothing.
#[test]
fn the_rewinding_run_really_rewinds() {
    let mut sim = sim_with_rewind_distance(4);
    for frame in 0..24 {
        sim.step(noisy_action(frame));
    }

    let stats = sim
        .rollback_execution_stats()
        .expect("GGRS instrumentation is installed");
    assert!(
        stats.load_runs > 0,
        "no LoadWorld requests — nothing was ever rewound: {stats:?}"
    );
    assert!(
        stats.advance_runs > 24 * 2,
        "resimulation must re-run frames many times over, or duplicate effects \
         could not have been observed in the first place: {stats:?}"
    );
}

/// Nothing may be left stranded. Once the host has confirmed everything it
/// simulated, the journal must be empty rather than holding effects that will
/// never be released.
#[test]
fn the_journal_drains_once_every_frame_confirms() {
    let mut sim = sim_with_rewind_distance(4);
    for frame in 0..32 {
        sim.step(noisy_action(frame));
    }
    for _ in 0..FLUSH_STEPS {
        sim.step(AgentAction::default());
    }

    let journal = sim
        .world()
        .get_resource::<ExternalEffectJournal<OwnedSfxMessage>>()
        .expect("the rollback host installs the audio journal");
    assert!(
        journal.released() > 0,
        "the run must have released something for the drain claim to mean anything"
    );
    // `check_distance` + 1, and the +1 is `bevy_ggrs`, not slack. It computes a
    // sync test's confirmed frame as `current - check_distance` using the frame
    // counter read *before* the advance increments it, so while frame F is
    // being simulated the reported confirmed frame is `F - 5`, not `F - 4`. The
    // quarantine is therefore one frame more conservative than it strictly
    // needs to be — safe in the direction that matters, and cheaper to document
    // than to correct against an upstream detail that may change.
    assert!(
        journal.depth() <= 5,
        "at most `check_distance` (+1, see above) frames may still be awaiting \
         confirmation, got {} — the journal is retaining effects it should have \
         released",
        journal.depth()
    );
}

/// **The classification, as an executable claim.**
///
/// The quarantine work-list named `EffectRequest` as VFX exposure. It is not:
/// all three of its readers are simulation-side (`apply_effects` spawns
/// hitboxes, `apply_summon_effects` spawns minions,
/// `apply_enemy_projectile_effects` spawns projectiles), as is
/// `SpawnProjectile`'s. Deferring one of those to the confirmed boundary would
/// not quarantine an external effect — it would change what the simulation
/// computes, because the consumer would miss the message on the pass that
/// produced it and see it again on a frame it does not belong to.
///
/// So the split is not "effect-shaped name" but "who reads it", and getting it
/// wrong in the permissive direction is a desync rather than a duplicate sound.
#[test]
fn only_presentation_facing_effects_are_quarantined() {
    use ambition::vfx::vfx::DebrisBurstMessage;
    use ambition::vfx::{EffectRequest, ExplosionRequest, FireworksRequest, VfxMessage};

    let sim = sim_with_rewind_distance(4);
    let world = sim.world();

    macro_rules! assert_quarantined {
        ($($ty:ty),+ $(,)?) => {$(
            assert!(
                world.contains_resource::<ExternalEffectJournal<$ty>>(),
                concat!(
                    stringify!($ty),
                    " is read by presentation and must be held to the confirmed \
                     boundary, but no journal was installed for it"
                )
            );
        )+};
    }
    macro_rules! assert_not_quarantined {
        ($($ty:ty),+ $(,)?) => {$(
            assert!(
                !world.contains_resource::<ExternalEffectJournal<$ty>>(),
                concat!(
                    stringify!($ty),
                    " is read by the SIMULATION. Deferring it past the frame that \
                     produced it changes what the simulation computes — that is a \
                     desync, not a quarantine"
                )
            );
        )+};
    }

    assert_quarantined!(
        OwnedSfxMessage,
        VfxMessage,
        ExplosionRequest,
        FireworksRequest,
        DebrisBurstMessage,
    );
    assert_not_quarantined!(EffectRequest, ambition::projectiles::SpawnProjectile);
}
