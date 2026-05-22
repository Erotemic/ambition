//! Contextual button labels — what each in-game action button means
//! "right now."
//!
//! Today the on-screen / mobile / accessibility-prompt layers all
//! hard-code static labels ("Interact", "Shield", "Attack"). That
//! lies when the action's meaning has changed by context:
//! - On a ledge, **Shield** triggers a Smash-style roll, not a shield.
//! - Near an NPC, **Interact** talks; near a door, it opens it;
//!   near a chest it opens it.
//! - In the air with down-aim, **Attack** is a down-tilt / pogo.
//!
//! This module is the data-only side of "label this button correctly":
//!
//! 1. [`ContextualAction`] enumerates every button we'd like to label
//!    contextually.
//! 2. [`PlayerActionContext`] is a flat snapshot of the per-tick state
//!    that affects labels — `on_ledge`, current `interact_prompt`,
//!    `is_aerial`, `aim_down`, etc.
//! 3. [`label_for`] is a pure function `(action, ctx) -> &str` that
//!    returns the label.
//!
//! UI layers (touch HUD, accessibility prompts, tutorial overlays)
//! build a `PlayerActionContext` once per frame from their queries
//! and call `label_for` per button. No new component plumbing per
//! consumer; no per-call-site label switch.
//!
//! Adding a new contextual rule = adding a single match arm here.
//! Adding a new label-aware UI = building one
//! `PlayerActionContext` per frame and calling `label_for`.

use std::borrow::Cow;

/// The set of action buttons that can carry a context-dependent label.
///
/// Includes buttons that are "verbs" today: Interact, Shield, Attack,
/// Jump, Dash, Special. Movement axes (left/right/up/down) aren't
/// here — their label is the direction itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ContextualAction {
    Interact,
    Shield,
    Attack,
    Jump,
    Dash,
    /// Reserved for the "blink / fire projectile / special" button,
    /// whose effect already varies by held charge level.
    Special,
}

/// Per-tick snapshot of player state that affects what each
/// contextual action would do RIGHT NOW. Populated by a UI-side
/// system from existing components (`PlayerBody`, the engine's
/// `ledge_grab` field, the nearest-interactable query, etc.).
///
/// Stays a flat POD struct on purpose — easy to construct, easy
/// to test, no lifetimes on the consumer.
#[derive(Clone, Debug, Default)]
pub struct PlayerActionContext {
    /// True when [`crate::player::PlayerBody::ledge_grabbing`] is
    /// active (player hanging on a ledge or in the climb/roll
    /// transition). On a ledge, Shield rolls; Jump+Up climbs.
    pub on_ledge: bool,
    /// True when the player has a live interactable in range — e.g.
    /// an NPC bubble, a chest, a switch, an interactable door. UI
    /// systems source this from whatever proximity query already
    /// drives the "press E" prompt.
    pub has_interactable: bool,
    /// Optional override text for the **Interact** button — e.g.
    /// "Talk", "Open", "Read", "Use Switch". `None` falls back to
    /// the generic "Interact" label.
    pub interact_prompt: Option<Cow<'static, str>>,
    /// Player is aerial (not grounded). Affects which Attack reads
    /// as up-air / down-tilt / neutral-air, etc.
    pub is_aerial: bool,
    /// Aiming down (held down on the stick). Affects Attack →
    /// pogo / down-tilt.
    pub aim_down: bool,
    /// Aiming up. Affects Attack → up-tilt / up-air.
    pub aim_up: bool,
    /// Player is in morph-ball / crouched body mode. Affects
    /// Jump → "Unmorph" (since Jump exits the body mode).
    pub is_morphed: bool,
    /// Player is swimming. Affects Jump → "Stroke", Attack reads
    /// differently underwater.
    pub is_swimming: bool,
}

/// The user-facing label for `action` given the current `ctx`.
///
/// Returned as a `Cow` so authors can provide either a literal
/// `"Roll"` or a borrowed prompt from the context without
/// allocating in the common case. UI consumers can pass directly
/// into `Text::new(...)`.
pub fn label_for(action: ContextualAction, ctx: &PlayerActionContext) -> Cow<'static, str> {
    use ContextualAction as A;
    match action {
        A::Interact => {
            if ctx.has_interactable {
                ctx.interact_prompt
                    .clone()
                    .unwrap_or(Cow::Borrowed("Interact"))
            } else {
                Cow::Borrowed("Interact")
            }
        }
        A::Shield => {
            if ctx.on_ledge {
                Cow::Borrowed("Roll")
            } else {
                Cow::Borrowed("Shield")
            }
        }
        A::Attack => {
            if ctx.is_aerial && ctx.aim_down {
                Cow::Borrowed("Pogo")
            } else if ctx.is_aerial && ctx.aim_up {
                Cow::Borrowed("Up Air")
            } else if !ctx.is_aerial && ctx.aim_down {
                Cow::Borrowed("Down Tilt")
            } else if !ctx.is_aerial && ctx.aim_up {
                Cow::Borrowed("Up Tilt")
            } else {
                Cow::Borrowed("Attack")
            }
        }
        A::Jump => {
            if ctx.on_ledge {
                Cow::Borrowed("Climb")
            } else if ctx.is_morphed {
                Cow::Borrowed("Unmorph")
            } else if ctx.is_swimming {
                Cow::Borrowed("Stroke")
            } else {
                Cow::Borrowed("Jump")
            }
        }
        A::Dash => {
            if ctx.is_aerial {
                Cow::Borrowed("Dodge")
            } else {
                Cow::Borrowed("Dash")
            }
        }
        A::Special => Cow::Borrowed("Special"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shield_on_ledge_reads_as_roll() {
        let ctx = PlayerActionContext {
            on_ledge: true,
            ..PlayerActionContext::default()
        };
        assert_eq!(label_for(ContextualAction::Shield, &ctx), "Roll");
    }

    #[test]
    fn shield_off_ledge_reads_as_shield() {
        let ctx = PlayerActionContext::default();
        assert_eq!(label_for(ContextualAction::Shield, &ctx), "Shield");
    }

    #[test]
    fn jump_on_ledge_reads_as_climb() {
        let ctx = PlayerActionContext {
            on_ledge: true,
            ..PlayerActionContext::default()
        };
        assert_eq!(label_for(ContextualAction::Jump, &ctx), "Climb");
    }

    #[test]
    fn interact_uses_prompt_when_provided() {
        let ctx = PlayerActionContext {
            has_interactable: true,
            interact_prompt: Some(Cow::Borrowed("Read the news board")),
            ..PlayerActionContext::default()
        };
        assert_eq!(
            label_for(ContextualAction::Interact, &ctx),
            "Read the news board",
        );
    }

    #[test]
    fn interact_falls_back_to_generic_label_when_no_target() {
        let ctx = PlayerActionContext::default();
        assert_eq!(label_for(ContextualAction::Interact, &ctx), "Interact");
    }

    #[test]
    fn attack_down_air_reads_as_pogo() {
        let ctx = PlayerActionContext {
            is_aerial: true,
            aim_down: true,
            ..PlayerActionContext::default()
        };
        assert_eq!(label_for(ContextualAction::Attack, &ctx), "Pogo");
    }

    #[test]
    fn attack_down_grounded_reads_as_down_tilt() {
        let ctx = PlayerActionContext {
            is_aerial: false,
            aim_down: true,
            ..PlayerActionContext::default()
        };
        assert_eq!(label_for(ContextualAction::Attack, &ctx), "Down Tilt");
    }

    #[test]
    fn jump_in_morphed_state_unmorphs() {
        let ctx = PlayerActionContext {
            is_morphed: true,
            ..PlayerActionContext::default()
        };
        assert_eq!(label_for(ContextualAction::Jump, &ctx), "Unmorph");
    }
}
