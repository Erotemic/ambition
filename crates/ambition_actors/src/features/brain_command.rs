//! Runtime brain-switching authority + the closed actor-directive vocabulary.
//!
//! [`BrainCommand`] is the ONE deterministic path for changing a catalog-backed
//! NPC's selected brain at runtime. It follows the established sim-command
//! pattern (`ambition_encounter::EncounterCommand`): a Bevy `Message` routed by
//! stable [`SimId`], drained by one reducer ([`apply_brain_commands`]) in the sim
//! schedule, grouped by target id in canonical order. Applying a command rebuilds
//! the live [`Brain`] from a catalog preset via
//! [`CharacterCatalog::build_brain_from_preset`] — the same seam spawn uses — and
//! mutates the actor's [`BrainBinding`] so the selection is snapshot-safe.
//! Ordinary gameplay never replaces a character-backed NPC's `Brain` directly; it
//! emits a `BrainCommand`.
//!
//! [`ActorDirective`] is the small, closed vocabulary a dialogue/gameplay outcome
//! routes through. It keeps four distinct concerns distinct:
//! - **brain** directives change AUTONOMOUS behavior (which brain drives the actor),
//! - an **action** request performs a real gameplay ACTION (a jump that moves the body),
//! - an **animation** directive requests a visual PERFORMANCE (no gameplay effect),
//! - **disposition** directives change allegiance / targeting policy.
//!
//! [`route_actor_directives`] fans an [`ActorDirectiveRequest`] out to the
//! authoritative channel for each concern — one central, auditable seam. This is
//! NOT a general scripting DSL; it is a fixed set of routable intents.

use ambition_characters::actor::character_catalog::{
    BrainBinding, BrainBuildContext, BrainPresetId, CharacterCatalog,
};
use ambition_characters::actor::ActorPose;
use ambition_characters::brain::Brain;
use ambition_platformer_primitives::sim_id::SimId;
use bevy::prelude::*;
use std::collections::BTreeMap;

// ===== BrainCommand: the deterministic brain-switch authority ==============

/// A deterministic request to change an actor's selected brain, routed by stable
/// [`SimId`]. Cleared on snapshot restore (like every sim command channel), so a
/// command never double-applies across a rewind; replaying the same inputs
/// re-issues it. Applied by [`apply_brain_commands`].
#[derive(Message, Clone, Debug, PartialEq, Eq)]
pub struct BrainCommand {
    /// Stable id of the actor whose brain changes.
    pub target: SimId,
    pub kind: BrainCommandKind,
}

/// What a [`BrainCommand`] does to the target's brain.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BrainCommandKind {
    /// Switch to an explicit preset: `selection = Override(preset)` and the live
    /// `Brain` is rebuilt fresh from that preset.
    UsePreset(BrainPresetId),
    /// Return to the character default: `selection = Default` and the live `Brain`
    /// is rebuilt fresh from the binding's `default_preset`. Always a FRESH default
    /// brain — no hidden suspended brain instance is resumed.
    RestoreDefault,
}

impl BrainCommand {
    pub fn use_preset(target: SimId, preset: impl Into<BrainPresetId>) -> Self {
        Self {
            target,
            kind: BrainCommandKind::UsePreset(preset.into()),
        }
    }

    pub fn restore_default(target: SimId) -> Self {
        Self {
            target,
            kind: BrainCommandKind::RestoreDefault,
        }
    }
}

/// Drain [`BrainCommand`]s and apply them to catalog-backed NPCs. The single
/// authoritative writer of a runtime brain-switch.
///
/// Deterministic: commands are grouped by target id in a `BTreeMap` (canonical
/// order) and applied in arrival order; each command mutates exactly the one
/// entity whose `SimId` matches, so ECS iteration order is irrelevant. Rebuilding
/// a brain goes through the same catalog seam as spawn, so a preset resolves
/// identically here and at spawn.
pub fn apply_brain_commands(
    catalog: Res<CharacterCatalog>,
    mut commands_in: MessageReader<BrainCommand>,
    mut actors: Query<(&SimId, &mut Brain, &mut BrainBinding, &ActorPose)>,
) {
    let mut by_id: BTreeMap<&str, Vec<&BrainCommandKind>> = BTreeMap::new();
    for cmd in commands_in.read() {
        by_id
            .entry(cmd.target.as_str())
            .or_default()
            .push(&cmd.kind);
    }
    if by_id.is_empty() {
        return;
    }
    for (sim_id, mut brain, mut binding, pose) in &mut actors {
        let Some(kinds) = by_id.get(sim_id.as_str()) else {
            continue;
        };
        for kind in kinds {
            apply_brain_command(&catalog, sim_id, &mut brain, &mut binding, pose, kind);
        }
    }
}

/// Apply one command to one actor. Rebuilds the live brain from the resolved
/// preset and updates the binding so the two agree. An unknown runtime preset is
/// REJECTED with a diagnostic — never a silent fall back to the default or
/// StandStill; the binding + brain are left unchanged.
fn apply_brain_command(
    catalog: &CharacterCatalog,
    sim_id: &SimId,
    brain: &mut Brain,
    binding: &mut BrainBinding,
    pose: &ActorPose,
    kind: &BrainCommandKind,
) {
    // A runtime switch carries no placement patrol params: a switched-in patrol
    // re-centers its lane on the actor's current position.
    let ctx = BrainBuildContext::at(pose.origin().x);
    // Qualify a (possibly raw) command preset into the actor's namespace — the
    // same one its default preset lives in — so authoring can use raw local names
    // while the assembled catalog keys presets by `provider::name`.
    let resolved_preset: BrainPresetId = match kind {
        BrainCommandKind::UsePreset(preset) => BrainPresetId::new(
            ambition_characters::actor::character_catalog::qualify_preset_like(
                binding.default_preset.as_str(),
                preset.as_str(),
            ),
        ),
        BrainCommandKind::RestoreDefault => binding.default_preset.clone(),
    };
    let Some(new_brain) = catalog.build_brain_from_preset(resolved_preset.as_str(), &ctx) else {
        warn!(
            target: "ambition_actors::brain_command",
            "BrainCommand for {}: unknown brain preset `{}` (not in brain_presets); command rejected",
            sim_id.as_str(),
            resolved_preset,
        );
        return;
    };
    *brain = new_brain;
    match kind {
        // Store the QUALIFIED name so a later snapshot/reconcile resolves it.
        BrainCommandKind::UsePreset(_) => binding.use_preset(resolved_preset),
        BrainCommandKind::RestoreDefault => binding.restore_default(),
    }
}

// ===== ActorDirective: the closed dialogue/gameplay directive vocabulary ====

/// A one-shot gameplay ACTION an actor can be asked to perform. A real action
/// that moves/affects the body — distinct from a pure animation. Closed and
/// small; extended as concrete consumers land.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActorActionKind {
    /// A real jump: the body leaves the ground.
    Jump,
    /// A real attack: the body swings/fires through its action set.
    Attack,
}

/// A disposition/allegiance change directive kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DispositionDirectiveKind {
    /// Become hostile toward the player (e.g. "fight me").
    SetHostile,
    /// Return to the default (peaceful) disposition (e.g. "you are free").
    RestoreDefault,
}

/// The closed vocabulary of actor-facing directives a dialogue/gameplay outcome
/// can request. Distinct concerns stay distinct (see the module docs). Not a
/// general scripting DSL.
#[derive(Clone, Debug, PartialEq)]
pub enum ActorDirective {
    /// Switch autonomous behavior to an explicit brain preset.
    UseBrainPreset(BrainPresetId),
    /// Return autonomous behavior to the character's catalog default brain.
    RestoreDefaultBrain,
    /// Perform a real gameplay action (a jump MOVES the body, an attack HITS).
    RequestAction(ActorActionKind),
    /// Play a one-shot animation clip as pure visual performance — no gameplay
    /// action occurs. A jump-in-place flourish, NOT a real jump.
    PlayAnimation(String),
    /// Set the actor's disposition/faction stance.
    SetDisposition(DispositionDirectiveKind),
}

/// Dialogue/gameplay ingress: "apply this directive to this actor". The router
/// ([`route_actor_directives`]) fans it out to the authoritative channel for its
/// concern. Routed by stable [`SimId`].
#[derive(Message, Clone, Debug, PartialEq)]
pub struct ActorDirectiveRequest {
    pub target: SimId,
    pub directive: ActorDirective,
}

/// Gameplay one-shot action channel (a REAL action: jump/attack). Distinct from
/// [`ActorAnimationDirective`] so an action can never be confused with a mere
/// visual. Effect wiring (injecting the jump/attack into the actor's control this
/// tick) is a documented follow-up; the channel + routing are established here.
#[derive(Message, Clone, Debug, PartialEq, Eq)]
pub struct ActorActionRequest {
    pub target: SimId,
    pub action: ActorActionKind,
}

/// Presentation channel: play a one-shot animation clip with NO gameplay effect.
/// The visual-only counterpart to [`ActorActionRequest`].
#[derive(Message, Clone, Debug, PartialEq)]
pub struct ActorAnimationDirective {
    pub target: SimId,
    pub clip: String,
}

/// Allegiance / targeting-policy channel. Effect wiring to the aggression /
/// faction seam is the documented immediate follow-up; the channel + routing are
/// established here so disposition stays a DISTINCT concern from brain choice.
#[derive(Message, Clone, Debug, PartialEq, Eq)]
pub struct DispositionDirective {
    pub target: SimId,
    pub kind: DispositionDirectiveKind,
}

/// Fan an [`ActorDirectiveRequest`] out to the authoritative channel for its
/// concern. The single, auditable routing seam: brain directives → [`BrainCommand`],
/// action requests → [`ActorActionRequest`], animation directives →
/// [`ActorAnimationDirective`], disposition directives → [`DispositionDirective`].
/// A gameplay action and a pure animation land on DIFFERENT channels — the
/// distinction is structural, not by convention.
pub fn route_actor_directives(
    mut requests: MessageReader<ActorDirectiveRequest>,
    mut brain_out: MessageWriter<BrainCommand>,
    mut action_out: MessageWriter<ActorActionRequest>,
    mut anim_out: MessageWriter<ActorAnimationDirective>,
    mut disposition_out: MessageWriter<DispositionDirective>,
) {
    for req in requests.read() {
        let target = req.target.clone();
        match &req.directive {
            ActorDirective::UseBrainPreset(preset) => {
                brain_out.write(BrainCommand::use_preset(target, preset.clone()));
            }
            ActorDirective::RestoreDefaultBrain => {
                brain_out.write(BrainCommand::restore_default(target));
            }
            ActorDirective::RequestAction(action) => {
                action_out.write(ActorActionRequest {
                    target,
                    action: *action,
                });
            }
            ActorDirective::PlayAnimation(clip) => {
                anim_out.write(ActorAnimationDirective {
                    target,
                    clip: clip.clone(),
                });
            }
            ActorDirective::SetDisposition(kind) => {
                disposition_out.write(DispositionDirective {
                    target,
                    kind: *kind,
                });
            }
        }
    }
}

// ===== Plugin ===============================================================

/// Registers the brain-command + actor-directive channels and their two sim
/// systems (`route_actor_directives` → `apply_brain_commands`, chained so a
/// directive-issued `BrainCommand` applies the same frame). Runs in the gameplay
/// effects window of the sim schedule.
pub struct BrainCommandPlugin;

impl Plugin for BrainCommandPlugin {
    fn build(&self, app: &mut App) {
        use ambition_platformer_primitives::schedule::SimScheduleExt;
        use bevy::prelude::IntoScheduleConfigs;

        app.add_message::<BrainCommand>();
        app.add_message::<ActorDirectiveRequest>();
        app.add_message::<ActorActionRequest>();
        app.add_message::<ActorAnimationDirective>();
        app.add_message::<DispositionDirective>();

        let sim = app.sim_schedule();
        app.add_systems(
            sim,
            (route_actor_directives, apply_brain_commands)
                .chain()
                .in_set(crate::schedule::SandboxSet::GameplayEffects),
        );
    }
}

#[cfg(test)]
mod tests;
