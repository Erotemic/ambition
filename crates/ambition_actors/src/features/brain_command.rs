//! Runtime brain-switching authority for catalog-backed NPCs.
//!
//! [`BrainCommand`] is the ONE deterministic path for changing a catalog-backed
//! NPC's *autonomous* brain at runtime. It follows the established sim-command
//! pattern (`ambition_encounter::EncounterCommand`): a Bevy `Message` routed by
//! stable [`SimId`], drained by one reducer ([`apply_brain_commands`]) in the sim
//! schedule, grouped by target id in canonical order. Applying a command goes
//! through [`apply_brain_selection`] — the single helper that rebuilds the live
//! [`Brain`] from a catalog preset (via
//! [`CharacterCatalog::build_brain_from_preset`], the same seam spawn uses) AND
//! updates the actor's [`BrainBinding`] so the two agree atomically.
//!
//! Two invariants this authority upholds:
//! - **Authored home.** A rebuild uses the actor's [`AuthoredBrainContext`] (its
//!   spawn anchor + patrol radius), never its current pose, so a restored patrol
//!   brain recenters where it was authored, not wherever it wandered.
//! - **Temporary control is untouchable.** An actor under player possession
//!   (`Brain::Player`) or mount control (`Mounted`) is skipped: its autonomous
//!   selection is not the live brain, so switching it would corrupt live control.
//!
//! Provocation/challenge installs a hostile roster brain through its own
//! authority (`provoke_actor_in_place`); it records the archetype in the binding
//! as [`AutonomousSource::Provoked`](ambition_characters::actor::character_catalog::AutonomousSource::Provoked),
//! which lets snapshot reconciliation RERUN the roster construction to
//! reconstruct that mode rather than rebuild the catalog default over it.
//! Ordinary gameplay never replaces a character-backed NPC's `Brain` directly;
//! it emits a `BrainCommand` (autonomous catalog change) or routes through the
//! provoke authority (disposition change).

use crate::combat::CombatCapabilities;
use crate::features::ecs::actor_clusters::ActorConfig;
use crate::features::{ActorAggression, ActorDisposition, CombatKit};
use ambition_characters::actor::character_catalog::{
    qualify_preset_like, AuthoredBrainContext, BrainBinding, BrainBuildContext, BrainPresetId,
    CharacterCatalog,
};
use ambition_characters::actor::ActorPose;
use ambition_characters::brain::{ActionSet, Brain};
use ambition_platformer_primitives::sim_id::SimId;
use bevy::prelude::*;
use std::collections::{BTreeMap, BTreeSet};

/// A deterministic request to change an actor's selected autonomous brain, routed
/// by stable [`SimId`]. Cleared on snapshot restore (like every sim command
/// channel), so a command never double-applies across a rewind; replaying the
/// same inputs re-issues it. Applied by [`apply_brain_commands`].
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

/// A compound **release from provocation** — the "you are free" gameplay
/// operation (the inverse of a `<<challenge>>`). It invokes TWO distinct
/// authorities together but atomically from the operation's perspective:
///
/// 1. **Disposition authority** — pacify the actor (peaceful disposition, passive
///    aggression, grudge/target cleared) so it stops fighting and does not
///    re-aggro on sight.
/// 2. **Source authority** — restore the catalog-default autonomous source and its
///    complete peaceful config, by emitting a [`BrainCommand::restore_default`]
///    that [`apply_brain_commands`] (ordered after) applies through the one
///    brain-selection seam.
///
/// This keeps the two authorities distinct (a bare [`BrainCommand::RestoreDefault`]
/// never touches disposition), while giving "you are free" one deterministic,
/// rollback-safe command. Cleared on snapshot restore like every command channel.
#[derive(Message, Clone, Debug, PartialEq, Eq)]
pub struct ReleaseProvocation {
    /// Stable id of the actor being freed.
    pub target: SimId,
}

impl ReleaseProvocation {
    pub fn new(target: SimId) -> Self {
        Self { target }
    }
}

/// Resolve a command's target catalog preset from the binding, or `None` (with a
/// warning) when it cannot be resolved. `RestoreDefault` needs a catalog default
/// preset — a source with none (a boss binding) rejects the command rather than
/// falling back to anything.
fn resolve_command_preset(
    sim_id: &SimId,
    binding: &BrainBinding,
    kind: &BrainCommandKind,
) -> Option<BrainPresetId> {
    match kind {
        // Qualify a (possibly raw) command preset into the actor's namespace — the
        // same one its default preset lives in — so authoring can use raw local
        // names while the assembled catalog keys presets by `provider::name`.
        BrainCommandKind::UsePreset(preset) => {
            let namespace = binding
                .default_preset
                .as_ref()
                .map(|p| p.as_str())
                .unwrap_or_else(|| preset.as_str());
            Some(BrainPresetId::new(qualify_preset_like(
                namespace,
                preset.as_str(),
            )))
        }
        BrainCommandKind::RestoreDefault => match &binding.default_preset {
            Some(default) => Some(default.clone()),
            None => {
                warn!(
                    target: "ambition_actors::brain_command",
                    "BrainCommand RestoreDefault for {}: binding has no catalog default preset \
                     (not a catalog-backed actor); command rejected",
                    sim_id.as_str(),
                );
                None
            }
        },
    }
}

/// The single authoritative catalog-brain selection: rebuild the live `Brain`
/// from the resolved preset and update the binding so the two agree. Returns
/// `false` (leaving both unchanged) when the preset is unknown — never a silent
/// fall back to the default or StandStill.
///
/// Both `UsePreset` and `RestoreDefault` rebuild using the actor's AUTHORED build
/// context, so a refreshed/restored patrol brain recenters on its authored home.
/// This is the shared helper the command reducer and (in spirit) any other
/// autonomous-selection site route through, so a preset resolves identically
/// wherever it is applied.
fn apply_brain_selection(
    catalog: &CharacterCatalog,
    sim_id: &SimId,
    brain: &mut Brain,
    binding: &mut BrainBinding,
    ctx: &BrainBuildContext,
    kind: &BrainCommandKind,
) -> bool {
    let Some(resolved_preset) = resolve_command_preset(sim_id, binding, kind) else {
        return false;
    };
    let Some(new_brain) = catalog.build_brain_from_preset(resolved_preset.as_str(), ctx) else {
        warn!(
            target: "ambition_actors::brain_command",
            "BrainCommand for {}: unknown brain preset `{}` (not in brain_presets); command rejected",
            sim_id.as_str(),
            resolved_preset,
        );
        return false;
    };
    *brain = new_brain;
    match kind {
        // Store the QUALIFIED name so a later snapshot/reconcile resolves it.
        BrainCommandKind::UsePreset(_) => binding.use_preset(resolved_preset),
        BrainCommandKind::RestoreDefault => binding.restore_default(),
    }
    true
}

/// Drain [`BrainCommand`]s and apply them to catalog-backed NPCs. The single
/// authoritative writer of a runtime autonomous-brain switch.
///
/// Deterministic: commands are grouped by target id in a `BTreeMap` (canonical
/// order) and applied in arrival order; each command mutates exactly the one
/// entity whose `SimId` matches, so ECS iteration order is irrelevant. An actor
/// under temporary control (player possession or mounted) is skipped — its live
/// brain is not its autonomous selection, and overwriting it would corrupt
/// control.
pub fn apply_brain_commands(
    catalog: Res<CharacterCatalog>,
    mut commands_in: MessageReader<BrainCommand>,
    mut possession: ResMut<crate::abilities::traversal::possession::PossessionState>,
    mut actors: Query<(
        Entity,
        &SimId,
        &mut Brain,
        &mut BrainBinding,
        Option<&AuthoredBrainContext>,
        Option<&mut ActorConfig>,
        &ActorPose,
        Has<crate::features::ecs::Mounted>,
        Option<&CombatKit>,
        Option<&mut CombatCapabilities>,
        Option<&mut ActionSet>,
    )>,
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
    for (
        entity,
        sim_id,
        mut brain,
        mut binding,
        authored,
        config,
        pose,
        mounted,
        kit,
        caps,
        action_set,
    ) in &mut actors
    {
        let Some(kinds) = by_id.get(sim_id.as_str()) else {
            continue;
        };
        // Rebuild around the AUTHORED home, not the current pose. (A catalog NPC
        // always carries `AuthoredBrainContext`; the pose is a defensive fallback.)
        let ctx = authored
            .map(AuthoredBrainContext::build_context)
            .unwrap_or_else(|| BrainBuildContext::at(pose.origin().x));

        // Under temporary control (player possession / mount) the live `Brain` is
        // the controller's, not the autonomous selection — so a switch updates only
        // the SOURCE that resumes when control ends, and is NEVER silently lost. We
        // do NOT touch any mount cache (that is the MOUNTED mode, not the autonomous
        // resume mode) — the suspended-autonomous-runtime pass owns resumption. The
        // possession resume-brain is kept agreeing with the new source so a LIVE
        // release resumes the newly selected mode; a snapshot restore reconstructs
        // it from the source directly (`reconcile_autonomous_actors`).
        if brain.is_player() || mounted {
            let mut changed = false;
            for kind in kinds {
                changed |= update_source_only(&catalog, sim_id, &mut binding, kind);
            }
            if changed && possession.possessed == Some(entity) {
                if let Some(resumed) = binding
                    .active_preset()
                    .and_then(|preset| catalog.build_brain_from_preset(preset.as_str(), &ctx))
                {
                    possession.restore_brain = Some(resumed);
                }
            }
            continue;
        }

        let mut changed = false;
        for kind in kinds {
            changed |=
                apply_brain_selection(&catalog, sim_id, &mut brain, &mut binding, &ctx, kind);
        }
        if changed {
            apply_catalog_mode(&catalog, &brain, config, kit, caps, action_set);
        }
    }
}

/// Restore the COMPLETE catalog-default actor mode after a live autonomous switch
/// (`UsePreset` / `RestoreDefault`) — not just the live brain. A prior provocation
/// may have installed hostile tuning / capabilities / action set / sprite override;
/// "you are free" (and any catalog switch) must revert ALL of it so the peaceful
/// actor is coherent LIVE, matching what a snapshot reconcile reconstructs from the
/// source. Uses the SHARED [`peaceful_config`](crate::features::ecs::snapshot_reconcile::peaceful_config)
/// projection, so live and reconcile can never drift. `config.brain` is derived
/// from the live brain inside that projection.
///
/// When the actor carries no combat kit to rebuild the full mode from, this falls
/// back to keeping only the `config.brain` read-model in sync (the prior behavior).
fn apply_catalog_mode(
    catalog: &CharacterCatalog,
    brain: &Brain,
    config: Option<Mut<ActorConfig>>,
    kit: Option<&CombatKit>,
    caps: Option<Mut<CombatCapabilities>>,
    action_set: Option<Mut<ActionSet>>,
) {
    let character_id = config.as_ref().and_then(|c| c.sprite_character_id.clone());
    let Some(kit) = kit else {
        if let Some(mut config) = config {
            config.brain = config_brain_for(brain);
        }
        return;
    };
    let peaceful = crate::features::ecs::snapshot_reconcile::peaceful_config(
        catalog,
        character_id.as_deref(),
        kit,
        brain,
    );
    if let Some(mut config) = config {
        config.tuning = peaceful.tuning;
        config.brain_spec = peaceful.brain_spec;
        config.brain = peaceful.config_brain;
        config.sprite_override_npc_name = None;
    }
    if let Some(mut caps) = caps {
        *caps = peaceful.capabilities;
    }
    if let Some(mut action_set) = action_set {
        *action_set = peaceful.action_set;
    }
}

/// Drain [`ReleaseProvocation`]s ("you are free"): pacify each target (the
/// disposition authority) and emit a [`BrainCommand::restore_default`] so
/// [`apply_brain_commands`] restores its catalog-default source + complete peaceful
/// config (the source authority). Ordered BEFORE `apply_brain_commands` so the
/// emitted command applies the same frame.
///
/// Pacifying resets the aggression to fully passive (no grudge, no target, no
/// accumulated strikes) and the disposition to peaceful, so a freed actor stops
/// fighting immediately and does not re-aggro on sight — the deliberate "you are
/// free" semantic, distinct from the target-liveness stand-down (which keeps the
/// aggression mode so a duelist re-engages when a foe reappears).
pub fn apply_release_provocations(
    mut releases: MessageReader<ReleaseProvocation>,
    mut brain_commands: MessageWriter<BrainCommand>,
    mut actors: Query<(&SimId, &mut ActorDisposition, &mut ActorAggression)>,
) {
    let targets: BTreeSet<String> = releases
        .read()
        .map(|r| r.target.as_str().to_string())
        .collect();
    if targets.is_empty() {
        return;
    }
    for (sim_id, mut disposition, mut aggression) in &mut actors {
        if !targets.contains(sim_id.as_str()) {
            continue;
        }
        // Disposition authority: pacify.
        *aggression = ActorAggression::passive();
        *disposition = ActorDisposition::Peaceful;
        // Source authority: restore the catalog-default autonomous mode.
        brain_commands.write(BrainCommand::restore_default(sim_id.clone()));
    }
}

/// Update only the autonomous SOURCE of a binding (no live-`Brain` rebuild), for a
/// command that arrives while the body is under temporary control. Returns whether
/// the preset resolved (an unknown preset is rejected, never silently applied).
fn update_source_only(
    catalog: &CharacterCatalog,
    sim_id: &SimId,
    binding: &mut BrainBinding,
    kind: &BrainCommandKind,
) -> bool {
    let Some(resolved) = resolve_command_preset(sim_id, binding, kind) else {
        return false;
    };
    // Validate the preset resolves before recording it, so control never resumes
    // into an unknown brain.
    if catalog
        .build_brain_from_preset(resolved.as_str(), &BrainBuildContext::at(0.0))
        .is_none()
    {
        warn!(
            target: "ambition_actors::brain_command",
            "BrainCommand for {} (under temporary control): unknown preset `{}`; source unchanged",
            sim_id.as_str(),
            resolved,
        );
        return false;
    }
    match kind {
        BrainCommandKind::UsePreset(_) => binding.use_preset(resolved),
        BrainCommandKind::RestoreDefault => binding.restore_default(),
    }
    true
}

/// The `ActorConfig.brain` read-model derived from a live autonomous brain, shared
/// by the spawn plan, the runtime switch, and the post-restore reconcile so the
/// classification can never disagree with the actual brain.
pub(crate) fn config_brain_for(
    brain: &Brain,
) -> ambition_entity_catalog::placements::CharacterBrain {
    use ambition_characters::brain::StateMachineCfg;
    if matches!(brain, Brain::StateMachine(StateMachineCfg::Patrol { .. })) {
        // The `path_id` is cosmetic in the read-model (no read site inspects it —
        // the real path is a separate `ActorMotionPath`), so a derived one is None.
        ambition_entity_catalog::placements::CharacterBrain::Patrol { path_id: None }
    } else {
        ambition_entity_catalog::placements::CharacterBrain::Passive
    }
}

/// Registers the [`BrainCommand`] + [`ReleaseProvocation`] channels and their
/// reducers. Runs in the gameplay effects window of the sim schedule.
pub struct BrainCommandPlugin;

impl Plugin for BrainCommandPlugin {
    fn build(&self, app: &mut App) {
        use ambition_platformer_primitives::schedule::SimScheduleExt;
        use bevy::prelude::IntoScheduleConfigs;

        app.add_message::<BrainCommand>();
        app.add_message::<ReleaseProvocation>();

        let sim = app.sim_schedule();
        app.add_systems(
            sim,
            (
                // Release runs first so the `BrainCommand` it emits is applied by
                // `apply_brain_commands` in the same frame.
                apply_release_provocations.before(apply_brain_commands),
                apply_brain_commands,
            )
                .in_set(crate::schedule::SandboxSet::GameplayEffects),
        );
    }
}

#[cfg(test)]
mod tests;
