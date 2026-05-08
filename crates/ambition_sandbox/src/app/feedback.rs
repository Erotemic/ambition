#[allow(unused_imports)]
use super::cli::*;
#[allow(unused_imports)]
use super::dev_runtime::*;
#[allow(unused_imports)]
use super::hud::*;
#[allow(unused_imports)]
use super::input_systems::*;
#[allow(unused_imports)]
use super::phases::*;
#[allow(unused_imports)]
use super::plugins::*;
#[allow(unused_imports)]
use super::resources::*;
#[allow(unused_imports)]
use super::setup_systems::*;
#[allow(unused_imports)]
use super::update::*;
#[allow(unused_imports)]
use super::world_flow::*;
#[allow(unused_imports)]
use super::*;

/// Bundled `MessageWriter`s for the sim → presentation event channel.
///
/// `sandbox_update` outgrew Bevy's 16-system-param limit when individual
/// writers were passed; bundling them in a single `SystemParam` keeps the
/// sim system signature within budget while preserving the Vec-collector →
/// drain pattern documented in `docs/events_refactor_plan.md`. Adding new
/// channels to the sim → presentation seam happens here, not on the
/// `sandbox_update` signature.
#[derive(SystemParam)]
pub struct SandboxEventWriters<'w> {
    pub(super) sfx: MessageWriter<'w, SfxMessage>,
    pub(super) vfx: MessageWriter<'w, VfxMessage>,
    pub(super) debris: MessageWriter<'w, DebrisBurstMessage>,
    pub(super) died: MessageWriter<'w, PlayerDiedMessage>,
}

/// Mutable producer queues `sandbox_update` writes into during the
/// gameplay tick.
///
/// The encounter / feature pipeline is wide — switch presses, feature
/// events (door open, NPC death, breakable destroyed, …), and the
/// reset-feature messages all need `ResMut` access alongside everything
/// else `sandbox_update` already takes. Bundling them in a single
/// `SystemParam` keeps the system signature within Bevy's
/// 16-`SystemParam` budget (each `Res`/`ResMut` counts as one).
///
/// Producers (here):
/// - `switch_queue`: one switch activation enqueued per frame the
///   player toggles a Switch entity. Drained by
///   `crate::encounter::sync_encounter_controller_states`.
/// - `feature_bus`: aggregate of damage / heal / interaction events
///   produced by `crate::features::run_feature_logic`.
///
/// Add new sim → sim queues (NOT sim → presentation, which is
/// `SandboxEventWriters`) here when they grow naturally; resist the
/// urge to thread them through the system signature directly.
#[derive(SystemParam)]
pub struct SandboxQueues<'w> {
    pub switch_queue: ResMut<'w, crate::encounter::SwitchActivationQueue>,
    pub feature_bus: ResMut<'w, crate::features::FeatureEventBus>,
}

/// Read-only progression-state bundle for the HUD and pause menu.
///
/// Same `SystemParam`-packing trick as `SandboxQueues` — the HUD reads
/// from many independent registries (quests, cutscene state, bosses,
/// encounters, world map) and would otherwise blow the 16-param budget
/// when combined with windowing / camera / font handles. Grouping them
/// behind a single param both keeps the budget headroom and documents
/// the intentional read-only contract: HUD systems must not mutate
/// progression state. Mutators live in the producer side
/// (`sandbox_update`, `crate::quest`, `crate::boss_encounter`, etc.).
#[derive(SystemParam)]
pub struct ProgressionResources<'w> {
    pub quests: Res<'w, crate::quest::QuestRegistry>,
    pub cutscene: Res<'w, crate::cutscene::ActiveCutscene>,
    pub cutscene_request: Res<'w, crate::cutscene::CutsceneAdvanceRequest>,
    pub bosses: Res<'w, crate::boss_encounter::BossEncounterRegistry>,
    pub encounters: Res<'w, crate::encounter::EncounterRegistry>,
    pub map: Res<'w, crate::map_menu::MapMenuState>,
}

/// Per-frame Vec collectors for the sim → presentation event channels.
///
/// `sandbox_update` is the only producer; phase helpers append messages as
/// the gameplay loop runs and `flush_feedback` drains them into the
/// `MessageWriter`s at every return point. Keeping the collectors on a
/// single struct lets phase helpers take one parameter instead of three.
pub(super) struct FrameFeedback {
    pub(super) sfx: Vec<SfxMessage>,
    pub(super) vfx: Vec<VfxMessage>,
    pub(super) debris: Vec<DebrisBurstMessage>,
    pub(super) died: Vec<PlayerDiedMessage>,
}

impl FrameFeedback {
    pub(super) fn new() -> Self {
        Self {
            sfx: Vec::new(),
            vfx: Vec::new(),
            debris: Vec::new(),
            died: Vec::new(),
        }
    }
}

/// Local control-flow signal for `sandbox_update` phase helpers. `Return`
/// means the phase wants `sandbox_update` to flush feedback and stop the
/// frame here; `Continue` means proceed to the next phase.
#[must_use]
pub(super) enum PhaseOutcome {
    Continue,
    Return,
}

/// Drain the per-frame `FrameFeedback` into the bundled `MessageWriter`s.
/// Call at every `sandbox_update` return point so audio/fx/debris
/// subscribers see the messages this frame.
pub(super) fn flush_feedback(feedback: &mut FrameFeedback, writers: &mut SandboxEventWriters) {
    writers.sfx.write_batch(feedback.sfx.drain(..));
    writers.vfx.write_batch(feedback.vfx.drain(..));
    writers.debris.write_batch(feedback.debris.drain(..));
    writers.died.write_batch(feedback.died.drain(..));
}
