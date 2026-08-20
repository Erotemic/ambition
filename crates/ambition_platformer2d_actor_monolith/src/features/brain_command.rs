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
//! - **A DISPLACED brain is untouchable.** An actor under mount control
//!   (`Mounted`) is skipped: the mount cached its policy and put a controller's
//!   in its place, so switching the live one would corrupt live control.
//!   ⭐ **possession is no longer on that list, and that is the point of the
//!   `Brain::Player` deletion.** A possessed body keeps its own policy the whole
//!   time — the seat moved, the brain did not — so a switch mid-possession
//!   applies LIVE, and release resumes it because it was never displaced. This
//!   used to be a two-step dance: update the SOURCE only, then re-derive
//!   `PossessionState::restore_brain` so a release would resume the new
//!   selection. Both halves are gone with the field.
//!
//! Provocation/challenge installs a hostile brain through its own authority
//! (`provoke_actor_in_place`); it records WHICH policy in the binding — the
//! engine's default, or one the creature authored — which lets snapshot
//! reconciliation rebuild that mode rather than the catalog default over it.
//!
//! ⚠ this said "installs a hostile ROSTER brain" and "RERUN the roster
//! construction", naming a variant that carried an archetype id. Neither is true
//! since 2026-08-12: provocation states a policy and rebuilds no body.
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
use ambition_platformer2d_shared_tangle::sim_id::SimId;
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
                .preset()
                .map(|p| p.as_str())
                .unwrap_or_else(|| preset.as_str());
            Some(BrainPresetId::new(qualify_preset_like(
                namespace,
                preset.as_str(),
            )))
        }
        BrainCommandKind::RestoreDefault => match &binding.default_preset {
            ambition_characters::actor::character_catalog::AutonomousDefault::Preset(default) => {
                Some(default.clone())
            }
            // ⭐ **NOT A PRESET AND NOT A FAILURE.** A character whose default is
            // its own authored `BrainProfile` is restored by the profile road in
            // `apply_brain_selection`, which has the body this lowering needs;
            // this function only answers the preset question, so it says *not
            // mine* rather than warning about a preset nobody named.
            ambition_characters::actor::character_catalog::AutonomousDefault::CharacterProfile => {
                None
            }
            ambition_characters::actor::character_catalog::AutonomousDefault::None => {
                warn!(
                    target: "ambition_platformer2d_actor_monolith::brain_command",
                    "BrainCommand RestoreDefault for {}: binding has no autonomous default \
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
#[allow(clippy::too_many_arguments)]
fn apply_brain_selection(
    catalog: &CharacterCatalog,
    sim_id: &SimId,
    brain: &mut Brain,
    binding: &mut BrainBinding,
    ctx: &BrainBuildContext,
    kind: &BrainCommandKind,
    // **The body**, for a default that is the character's own `BrainProfile`:
    // §4.7 pairs a policy's normalized effort with the body's own top speed, so
    // the lowering cannot happen without one.
    profile_body: Option<&ActorConfig>,
    // **The character's own policy, resolved by IDENTITY** — see
    // [`character_policy`](crate::features::ecs::character_policy). `None` only
    // where no cast can answer, and then the body's current policy stands in.
    character_profile: Option<ambition_characters::brain::BrainProfile>,
    abilities: ambition_platformer2d_core::AbilitySet,
) -> bool {
    // ⭐⭐ **RESTORING A CHARACTER'S OWN POLICY IS NOT A PRESET LOOKUP.**
    //
    // ⛔ before this arm, a body whose default was its character's authored
    // `BrainProfile` carried `default_preset: Some("")`, and every
    // `RestoreDefault` — every possession release, every "you are free" — landed
    // on *"unknown brain preset ``"* and was REJECTED. The body kept whichever
    // mind it happened to have, silently, for the rest of the session.
    //
    // The lowering is the same one the spawn road and the rewind road use, and
    // it reads the profile off the body's own config, so all three agree by
    // construction rather than by three matching implementations.
    if matches!(kind, BrainCommandKind::RestoreDefault)
        && matches!(
            binding.default_preset,
            ambition_characters::actor::character_catalog::AutonomousDefault::CharacterProfile
        )
    {
        let Some(config) = profile_body else {
            warn!(
                target: "ambition_platformer2d_actor_monolith::brain_command",
                "BrainCommand RestoreDefault for {}: its character's own policy is the                  default, but the body carries no ActorConfig to lower it against;                  command rejected",
                sim_id.as_str(),
            );
            return false;
        };
        // ⛔⛔ **THE POLICY COMES FROM THE CHARACTER, NOT FROM THE BODY'S CURRENT
        // ONE.** This lowered `config.brain_profile` directly, and provocation
        // WRITES that field — so "you are free" rebuilt the provoked mind and
        // then labelled the binding `CharacterProfile`. The body kept hunting
        // you while every piece of state agreed it had been released.
        //
        // ⛔ **and there is no fallback to that field.** A binding saying
        // `default = CharacterProfile` is a claim the character can answer;
        // falling back to the live policy when it cannot is the same bug with a
        // longer fuse. Reject, loudly, like every other unresolvable command.
        let Some(profile) = character_profile else {
            warn!(
                target: "ambition_platformer2d_actor_monolith::brain_command",
                "BrainCommand RestoreDefault for {}: its binding says the CHARACTER \
                 owns the default policy, and the character could not be resolved \
                 (no WornCharacter, or no prepared cast containing it, or it \
                 authors no autonomous_profile); command rejected",
                sim_id.as_str(),
            );
            return false;
        };
        *brain =
            crate::features::ecs::character_policy::brain_from_profile(config, profile, abilities);
        binding.restore_default();
        return true;
    }
    let Some(resolved_preset) = resolve_command_preset(sim_id, binding, kind) else {
        return false;
    };
    let Some(new_brain) = catalog.build_brain_from_preset(resolved_preset.as_str(), ctx) else {
        warn!(
            target: "ambition_platformer2d_actor_monolith::brain_command",
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
/// entity whose `SimId` matches, so ECS iteration order is irrelevant. A MOUNTED
/// actor is skipped — the mount displaced its policy, so its live brain is not
/// its autonomous selection and overwriting it would corrupt control. A POSSESSED
/// actor is NOT skipped: nothing displaced its policy.
pub fn apply_brain_commands(
    catalog: Res<CharacterCatalog>,
    // **The cast**, for a body whose autonomous default is its own character's
    // policy: that policy is recovered by identity, never from the mutable
    // `ActorConfig::brain_profile` a provocation has overwritten. `Option`
    // because compositions that register no cast are ordinary.
    prepared: Option<Res<crate::character_runtime::PreparedCharacterRegistry>>,
    mut commands_in: MessageReader<BrainCommand>,
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
        // **The body's own verbs**, for a default that is the character's
        // authored policy — the lowering asks what this body can actually do.
        Option<&ambition_platformer2d_core::BodyAbilities>,
        // **Which character this body IS** — the gameplay identity its durable
        // autonomous policy is recovered through.
        Option<&ambition_characters::actor::WornCharacter>,
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
        // ⚠ **the entity handle is no longer read**, and the reason is the
        // `Brain::Player` deletion: this used to compare it against
        // `PossessionState::possessed` to decide whether to refresh a resume
        // brain. Nothing is displaced by a possession any more, so there is
        // nothing to refresh and nobody to identify.
        _entity,
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
        body_abilities,
        worn,
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
        let abilities = body_abilities
            .map(|abilities| abilities.abilities)
            .unwrap_or_default();
        // The durable answer to "what does this character normally do", resolved
        // once per body. `None` where no cast can answer; the lowering then
        // falls back to the body's current policy, which is the fixture road.
        let character_profile = prepared.as_deref().zip(worn).and_then(|(registry, worn)| {
            crate::features::ecs::character_policy::character_autonomous_profile(registry, worn)
        });

        // Under MOUNT control the live `Brain` is the controller's, not the
        // autonomous selection — so a switch updates only the SOURCE that resumes
        // when control ends, and is NEVER silently lost. We do NOT touch any
        // mount cache (that is the MOUNTED mode, not the autonomous resume mode)
        // — the suspended-autonomous-runtime pass owns resumption.
        //
        // ⭐⭐ **POSSESSION IS NOT HERE ANY MORE.** It used to share this arm
        // because possession moved `Brain::Player` onto the body and destroyed
        // whatever policy was there, so the only honest thing a switch could do
        // was update the source and re-derive `PossessionState::restore_brain`.
        // A possessed body keeps its own brain now: the switch applies LIVE
        // below, the human's input still drives the body through its seat, and
        // the release resumes the switched policy because it was never displaced.
        // The `restore_brain` re-derivation that made a provoke → possess →
        // release-provocation → release sequence resume the PROVOKED mind cannot
        // exist, because there is no cached mind.
        if mounted {
            let mut changed = false;
            for kind in kinds {
                changed |= update_source_only(&catalog, sim_id, &mut binding, kind);
            }
            // ⚠ the return value is the "did anything actually change" signal the
            // resume-brain refresh consumed; nothing consumes it on this arm.
            let _ = changed;
            continue;
        }

        let mut changed = false;
        for kind in kinds {
            changed |= apply_brain_selection(
                &catalog,
                sim_id,
                &mut brain,
                &mut binding,
                &ctx,
                kind,
                config.as_deref(),
                character_profile,
                abilities,
            );
        }
        if changed {
            apply_catalog_mode(
                &catalog,
                prepared.as_deref(),
                &brain,
                config,
                kit,
                caps,
                action_set,
                character_profile,
            );
        }
    }
}

/// Restore the COMPLETE catalog-default actor mode after a live autonomous switch
/// (`UsePreset` / `RestoreDefault`) — not just the live brain. A prior provocation
/// may have installed hostile tuning / capabilities / action set / sprite override;
/// "you are free" (and any catalog switch) must revert ALL of it so the peaceful
/// actor is coherent LIVE, matching what a snapshot reconcile reconstructs from the
/// source. Uses the SHARED [`peaceful_config`](crate::features::ecs::autonomous_reconcile::peaceful_config)
/// projection, so live and reconcile can never drift. `config.brain` is derived
/// from the live brain inside that projection.
///
/// When the actor carries no combat kit to rebuild the full mode from, this falls
/// back to keeping only the `config.brain` read-model in sync (the prior behavior).
fn apply_catalog_mode(
    catalog: &CharacterCatalog,
    // **The prepared cast, so the peaceful projection asks the CHARACTER whether
    // it flies before it asks the catalog's silhouette.** See `peaceful_config`.
    prepared: Option<&crate::character_runtime::PreparedCharacterRegistry>,
    brain: &Brain,
    config: Option<Mut<ActorConfig>>,
    kit: Option<&CombatKit>,
    caps: Option<Mut<CombatCapabilities>>,
    action_set: Option<Mut<ActionSet>>,
    // See the `Some` arm below: a character that states its own policy states
    // its own BODY too, and this reconstruction is not for it.
    character_profile: Option<ambition_characters::brain::BrainProfile>,
) {
    // ⭐⭐ **A CHARACTER-FIRST BODY IS RESTORED IN THE MIND ONLY.**
    //
    // ⛔ the projection below is the peaceful-NPC seed: default capabilities and
    // `brain_profile: BrainProfile::default()`. It is the correct answer for a
    // catalog-default NPC, whose whole body IS that seed. Over a body whose
    // character authored its kit it is a silent downgrade wearing a controller
    // change — and the policy it zeroed was the field the CharacterProfile
    // restoration then read back as the character's default.
    //
    // ⚠ **it said `max_health: 1, max_run_speed: MAX_RUN_SPEED` here and neither
    // is true any more.** The `1` moved to `DEFAULT_UNAUTHORED_BODY_HEALTH` at
    // the spawn seeds (D101), and `peaceful_config` reads the prepared
    // character's blueprint for BOTH numbers since 2026-08-13 (P2.19) — because
    // this guard keys on the character's POLICY while the downgrade was to its
    // BODY, so a character that authored locomotion and no profile fell straight
    // through it. That population is empty today and the trap is closed at the
    // projection, which is the layer that can see the character.
    if let Some(profile) = character_profile {
        if let Some(mut config) = config {
            config.brain_profile = profile;
            config.brain = config_brain_for(brain);
            config.sprite_override_npc_name = None;
        }
        return;
    }
    let character_id = config.as_ref().and_then(|c| c.sprite_character_id.clone());
    let Some(kit) = kit else {
        if let Some(mut config) = config {
            config.brain = config_brain_for(brain);
        }
        return;
    };
    let peaceful = crate::features::ecs::autonomous_reconcile::peaceful_config(
        catalog,
        prepared,
        character_id.as_deref(),
        kit,
        brain,
    );
    if let Some(mut config) = config {
        config.tuning = peaceful.tuning;
        config.brain_profile = peaceful.brain_profile;
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
    // ⭐⭐ **A CHARACTER-FIRST DEFAULT HAS NO PRESET TO VALIDATE, AND THAT IS NOT
    // A REJECTION.**
    //
    // ⛔ `resolve_command_preset` answers *not mine* for `CharacterProfile`, and
    // this read that `None` as *unresolvable* and left the binding untouched. So
    // a `RestoreDefault` arriving while the body was possessed or mounted was
    // silently dropped: provoke → possess → release-provocation → release
    // possession resumed the PROVOKED policy, because the release never reached
    // the source it was supposed to change. The lowering needs a body this
    // function does not have — but recording the source does not, and recording
    // the source is this function's entire job.
    if matches!(kind, BrainCommandKind::RestoreDefault)
        && matches!(
            binding.default_preset,
            ambition_characters::actor::character_catalog::AutonomousDefault::CharacterProfile
        )
    {
        binding.restore_default();
        return true;
    }
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
            target: "ambition_platformer2d_actor_monolith::brain_command",
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
        use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt;
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
                .in_set(crate::schedule::Platformer2dSimulationPhaseMonolith::GameplayEffects),
        );
    }
}

#[cfg(test)]
mod tests;
