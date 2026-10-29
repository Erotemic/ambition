//! Accept body control from scripts, abilities, moves, and room rules.

use ambition_characters::actor::BodyHealth;
use ambition_characters::brain::ActionSet;
use ambition_characters::control::{ActorControl, ControlHolds, DrivingParticipant};
use ambition_combat::moveset::ActorMoveset;
use ambition_platformer2d_core::BodyKinematics;
use bevy::prelude::*;

/// Blank scripted bodies after brain production and before control consumers.
///
/// Device state is untouched, so held input resumes when scripted control ends.
pub fn blank_scripted_control_frames(mut bodies: Query<&mut ActorControl, With<ControlHolds>>) {
    for mut control in &mut bodies {
        control.0 = ambition_characters::actor::control::ActorControlFrame::neutral();
    }
}

/// Walk every body a script commands to its mark
/// ([`ambition_characters::control::CommandedMove`]): the one road for a
/// player, an NPC and a boss.
///
/// After the blank, so a held body is walked by the script and not by the
/// stick. A boss that is lured starts no attack: the walk clears its attack
/// intent and the presses of its frame, so a windup in progress is
/// interrupted and a committed strike runs out. A dead body is not walked.
pub fn drive_commanded_moves(
    mut bodies: Query<(
        &BodyKinematics,
        &ambition_characters::control::CommandedMove,
        &mut ActorControl,
        Option<&ambition_platformer2d_core::MotionModel>,
        Option<&ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame>,
        Option<&mut ambition_characters::brain::boss_pattern::BossAttackIntent>,
        Option<&BodyHealth>,
    )>,
) {
    for (kin, command, mut control, motion_model, frame, attack_intent, health) in &mut bodies {
        if health.is_some_and(|health| !health.alive()) {
            continue;
        }
        if let Some(mut attack_intent) = attack_intent {
            attack_intent.clear();
            control.0.melee_pressed = false;
            control.0.special_pressed = false;
        }
        let walker = motion_model.map(|model| {
            (
                model.commanded_top_speed(),
                frame.map_or_else(
                    || ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame::default().get(),
                    |frame| frame.get(),
                ),
            )
        });
        command.steer(kin.pos, kin.facing, walker, &mut control.0);
    }
}

/// Order systems around body action-scheme acceptance.
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BodyControlGate;

/// Apply body capabilities and room grants to the current control frame.
/// Only a body with `DrivingParticipant` gets the room's driven techniques.
/// Route technique edges and clear unsupported verbs before body execution.
pub fn gate_body_control(
    // The driven body also wears the techniques its own room's rules give it
    // (OW1: THE live room's rules were the rules of no room while two rooms
    // were live).
    driven: ambition_combat::governing_rules::RulesOf<
        ambition_characters::action_scheme::DrivenTechniques,
    >,
    mut bodies: Query<(
        Entity,
        bevy::prelude::Ref<ActionSet>,
        // The body's live combat/ability authorities — the SAME inputs the
        // control-prompt read-model derives its labels from. The gate resolves
        // them through the shared `derive_action_scheme` so what a slot GATES
        // here and what the prompt SHOWS are one derivation (no UI drift).
        // `Ref`: the scheme is derived again only when one of them changed.
        bevy::prelude::Ref<ambition_platformer2d_core::BodyAbilities>,
        Option<bevy::prelude::Ref<ActorMoveset>>,
        Option<bevy::prelude::Ref<ambition_characters::action_scheme::ActorTechniques>>,
        // A worn row can put a technique on a slot.
        Option<bevy::prelude::Ref<ambition_characters::equipment::WornEquipment>>,
        &mut ambition_characters::action_scheme::DerivedActionScheme,
        &mut ambition_characters::control::ActorControl,
        // Sanctioned technique edges: when a slot resolves to `Technique`, the
        // gate routes the slot's device edge here (and clears the raw verb),
        // so a content technique reads THIS instead of intercepting a raw
        // combat press. `ActionSet` requires it.
        &mut ambition_characters::action_scheme::ResolvedTechniqueEdges,
        Has<ambition_characters::brain::ChargesProjectiles>,
        // Holding an item REPURPOSES the attack verb (the pickup stashes the
        // melee kit precisely so item-use fires instead), so the persona
        // gate must not eat melee/shield presses while an item is held —
        // by IDENTITY, not by racing the item systems in schedule order.
        Has<ambition_combat::held_items::HeldItem>,
        Has<DrivingParticipant>,
    )>,
) {
    use ambition_characters::action_scheme::{
        derive_action_scheme, resolve_control_slots, techniques_of,
    };
    use bevy::prelude::DetectChanges as _;

    for (
        entity,
        actions,
        abilities,
        moveset,
        techniques,
        worn,
        mut derived,
        mut control,
        mut edges,
        has_charge_marker,
        holds_item,
        is_driven,
    ) in &mut bodies
    {
        let driven_here = if is_driven { driven.of(entity) } else { None };
        let present = [moveset.is_some(), techniques.is_some(), worn.is_some()];
        // The authorities change at loading, a pickup or a re-wear, not each
        // tick (findings F16): a quiet tick reuses the scheme.
        let stale = derived.scheme.is_none()
            || actions.is_changed()
            || abilities.is_changed()
            || moveset.as_ref().is_some_and(|m| m.is_changed())
            || techniques.as_ref().is_some_and(|t| t.is_changed())
            || worn.as_ref().is_some_and(|w| w.is_changed())
            || derived.present != present
            || derived.driven.as_ref() != driven_here;
        if stale {
            // THE shared resolver — byte-identical to the call the ControlPrompt
            // producer makes on the same immediate authorities.
            let techniques = techniques_of(techniques.as_deref(), driven_here, worn.as_deref());
            let scheme = derive_action_scheme(
                &abilities.abilities,
                moveset.as_ref().map(|m| &m.0),
                Some(&*actions),
                &techniques,
            );
            *derived = ambition_characters::action_scheme::DerivedActionScheme {
                scheme: Some(scheme),
                driven: driven_here.cloned(),
                present,
            };
        }
        let Some(scheme) = derived.scheme.as_ref() else {
            continue;
        };

        // Per-slot dispatch: route every technique to its sanctioned edge, strip
        // the verbs the scheme doesn't own (Attack/Special/Projectile), and keep
        // the moveset `Move`s.
        let unroutable = resolve_control_slots(scheme, &mut control.0, &mut *edges, holds_item);
        debug_assert!(
            unroutable.is_empty(),
            "action scheme declared a technique on a slot the combat gate cannot route \
             yet (needs the Phase-3 kernel re-key): {unroutable:?}",
        );

        // the question now lives with every other slot's, inside
        // `resolve_control_slots` above: the Shield slot is present iff the body
        // has the shield ability, and the held-item exception (shield+attack is
        // the throw gesture) moved there with it. `sustain_bubble_shield` runs
        // AFTER this system and still forces the guard up for the folded
        // bubble-shield special, so a special MAY raise a guard — it is simply no
        // longer the only way any body has.

        // HOW THIS BODY FIRES is its `ChargesProjectiles` marker, installed
        // from the character's `ranged_execution` by every road that builds or
        // re-wears it; a body without it has no charge path to feed.
        if !has_charge_marker {
            control.0.projectile_pressed = false;
            control.0.projectile_held = false;
            control.0.projectile_released = false;
        }
    }
}

#[cfg(test)]
mod tests;
