//! Pure resolver functions — one per verb.
//!
//! Each `resolve_*` takes the minimum data needed to decide which
//! variant would fire if the verb's button were pressed RIGHT NOW.
//! They are pure: no Bevy, no queries, no `Res<…>` — just arguments
//! in, variant out. That makes them trivially unit-testable AND
//! reusable by gameplay code: when the attack subsystem migrates to
//! variants, it can call `resolve_attack` itself instead of growing
//! its own duplicate branching.
//!
//! The compute layer in [`super`] is the only place that knows
//! about ECS; it bridges queries to these resolvers.

use super::intent::Aim;
use super::variants::*;

/// State of the player's body that the resolvers care about. A small
/// view bundle is more honest than passing six bools positionally;
/// when a new dimension is needed (e.g. `is_charging`, `is_invincible`)
/// it gets a field here and a branch in whichever resolver consumes
/// it, without disturbing the others.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PlayerBodyView {
    pub is_aerial: bool,
    pub on_ledge: bool,
    pub is_morphed: bool,
    pub is_swimming: bool,
}

/// What's nearby in the world that affects affordance resolution.
/// Today this only carries the nearest-interactable's variant +
/// optional authored prompt. Future fields (pogo-target-below, wall
/// at the player's back for wall-jump hint, water current direction)
/// slot in alongside without changing the resolver signatures of
/// unrelated verbs.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WorldView {
    pub nearest_interactable: InteractVariant,
    /// True when a pogo-able target sits beneath the player's feet
    /// within pogo range. Populated by a Phase 3 downward query;
    /// today it's always `false`, which means `D-Air` never upgrades
    /// to `Pogo` until that wiring lands.
    pub pogo_target_below: bool,
    /// True when the player holds an active portal gun, which rebinds
    /// Interact to "Mode Switch" (toggle blue/orange) when there's no
    /// genuine interactable nearby.
    pub portal_gun_active: bool,
}

/// What pressing Attack would do right now.
pub fn resolve_attack(aim: Aim, body: PlayerBodyView, world: &WorldView) -> AttackVariant {
    if body.is_aerial {
        if aim.is_down() {
            if world.pogo_target_below {
                AttackVariant::Pogo
            } else {
                AttackVariant::DAir
            }
        } else if aim.is_up() {
            AttackVariant::UAir
        } else if aim.is_back() {
            AttackVariant::BAir
        } else {
            AttackVariant::NAir
        }
    } else if aim.is_down() {
        AttackVariant::DTilt
    } else if aim.is_up() {
        AttackVariant::UTilt
    } else {
        // Forward / Back / Neutral grounded all read as Jab today —
        // F-Smash / B-Smash are reserved for a future held-charge
        // input that doesn't exist yet.
        AttackVariant::Jab
    }
}

/// What pressing Jump would do right now.
pub fn resolve_jump(body: PlayerBodyView) -> JumpVariant {
    if body.on_ledge {
        JumpVariant::Climb
    } else if body.is_morphed {
        JumpVariant::Unmorph
    } else if body.is_swimming {
        JumpVariant::Stroke
    } else {
        JumpVariant::Jump
    }
}

/// What pressing Shield would do right now.
pub fn resolve_shield(body: PlayerBodyView) -> ShieldVariant {
    if body.on_ledge {
        ShieldVariant::Roll
    } else {
        ShieldVariant::Shield
    }
}

/// What pressing Dash would do right now.
pub fn resolve_dash(body: PlayerBodyView) -> DashVariant {
    if body.is_aerial {
        DashVariant::Dodge
    } else {
        DashVariant::Dash
    }
}

/// What pressing Interact would do right now. Just forwards the
/// world view's classification; the proximity query is what does
/// the real work of picking a variant.
pub fn resolve_interact(world: &WorldView) -> InteractVariant {
    // A genuine interactable always wins (Talk / Open / Activate / …).
    if !matches!(world.nearest_interactable, InteractVariant::None) {
        return world.nearest_interactable.clone();
    }
    // Otherwise an item may rebind Interact — the portal gun toggles mode.
    if world.portal_gun_active {
        return InteractVariant::ModeSwitch;
    }
    // Nothing to do: the HUD shows "Context".
    InteractVariant::None
}

/// What pressing Special would do right now. Smash-style four-way
/// dispatch on the current stick aim: neutral / side / up / down
/// specials are distinct variants in the HUD even though they all
/// invoke the same gameplay outcome (fireball) today.
///
/// The Hadouken seam (quarter-circle-forward → `Hadouken`) lives in
/// the future: when an input-history buffer detects QCF, the
/// resolver picks `Hadouken` over `NeutralSpecial` / `SideSpecial`.
/// Until that lands the resolver is aim-only.
pub fn resolve_special(aim: Aim, _body: PlayerBodyView) -> SpecialVariant {
    // Diagonals fall through to whichever axis dominates the read.
    // Up wins over forward/back so a `ForwardUp` aim still picks
    // `UpSpecial` (the Smash convention).
    if aim.is_up() {
        SpecialVariant::UpSpecial
    } else if aim.is_down() {
        SpecialVariant::DownSpecial
    } else if aim.is_forward() || aim.is_back() {
        SpecialVariant::SideSpecial
    } else {
        SpecialVariant::NeutralSpecial
    }
}

#[cfg(test)]
mod tests;
