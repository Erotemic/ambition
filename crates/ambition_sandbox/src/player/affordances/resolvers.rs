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
    world.nearest_interactable.clone()
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
mod tests {
    use super::*;
    use std::borrow::Cow;

    fn grounded() -> PlayerBodyView {
        PlayerBodyView {
            is_aerial: false,
            on_ledge: false,
            is_morphed: false,
            is_swimming: false,
        }
    }

    fn aerial() -> PlayerBodyView {
        PlayerBodyView {
            is_aerial: true,
            ..grounded()
        }
    }

    fn empty_world() -> WorldView {
        WorldView::default()
    }

    #[test]
    fn attack_grounded_neutral_is_jab() {
        assert_eq!(
            resolve_attack(Aim::Neutral, grounded(), &empty_world()),
            AttackVariant::Jab
        );
        assert_eq!(
            resolve_attack(Aim::Forward, grounded(), &empty_world()),
            AttackVariant::Jab
        );
    }

    #[test]
    fn attack_grounded_down_is_dtilt() {
        assert_eq!(
            resolve_attack(Aim::Down, grounded(), &empty_world()),
            AttackVariant::DTilt
        );
        // Diagonal still counts as "has down component."
        assert_eq!(
            resolve_attack(Aim::ForwardDown, grounded(), &empty_world()),
            AttackVariant::DTilt
        );
    }

    #[test]
    fn attack_grounded_up_is_utilt() {
        assert_eq!(
            resolve_attack(Aim::Up, grounded(), &empty_world()),
            AttackVariant::UTilt
        );
    }

    #[test]
    fn attack_aerial_neutral_is_nair() {
        assert_eq!(
            resolve_attack(Aim::Neutral, aerial(), &empty_world()),
            AttackVariant::NAir
        );
        // Forward stick + aerial is still NAir until F-Air is a
        // gameplay variant; the resolver's branches reflect what the
        // sim actually does today.
        assert_eq!(
            resolve_attack(Aim::Forward, aerial(), &empty_world()),
            AttackVariant::NAir
        );
    }

    #[test]
    fn attack_aerial_back_is_bair() {
        assert_eq!(
            resolve_attack(Aim::Back, aerial(), &empty_world()),
            AttackVariant::BAir
        );
    }

    #[test]
    fn attack_aerial_up_is_uair() {
        assert_eq!(
            resolve_attack(Aim::Up, aerial(), &empty_world()),
            AttackVariant::UAir
        );
        assert_eq!(
            resolve_attack(Aim::ForwardUp, aerial(), &empty_world()),
            AttackVariant::UAir
        );
    }

    #[test]
    fn attack_aerial_down_is_dair_without_pogo_target() {
        assert_eq!(
            resolve_attack(Aim::Down, aerial(), &empty_world()),
            AttackVariant::DAir
        );
    }

    #[test]
    fn attack_aerial_down_promotes_to_pogo_with_target_below() {
        let world = WorldView {
            pogo_target_below: true,
            ..WorldView::default()
        };
        assert_eq!(
            resolve_attack(Aim::Down, aerial(), &world),
            AttackVariant::Pogo
        );
        // Pogo only triggers on a down-aim — neutral D-air without
        // aim_down stays as N-Air.
        assert_eq!(
            resolve_attack(Aim::Neutral, aerial(), &world),
            AttackVariant::NAir
        );
    }

    #[test]
    fn jump_priority_ledge_over_morph_over_swim() {
        let mut body = grounded();
        body.on_ledge = true;
        body.is_morphed = true;
        body.is_swimming = true;
        // Ledge wins — Climb.
        assert_eq!(resolve_jump(body), JumpVariant::Climb);

        body.on_ledge = false;
        // Morph wins over swim.
        assert_eq!(resolve_jump(body), JumpVariant::Unmorph);

        body.is_morphed = false;
        assert_eq!(resolve_jump(body), JumpVariant::Stroke);

        body.is_swimming = false;
        assert_eq!(resolve_jump(body), JumpVariant::Jump);
    }

    #[test]
    fn shield_on_ledge_is_roll() {
        let mut body = grounded();
        body.on_ledge = true;
        assert_eq!(resolve_shield(body), ShieldVariant::Roll);
    }

    #[test]
    fn shield_off_ledge_is_shield() {
        assert_eq!(resolve_shield(grounded()), ShieldVariant::Shield);
    }

    #[test]
    fn dash_aerial_is_dodge() {
        assert_eq!(resolve_dash(aerial()), DashVariant::Dodge);
        assert_eq!(resolve_dash(grounded()), DashVariant::Dash);
    }

    #[test]
    fn interact_forwards_world_view() {
        let world = WorldView {
            nearest_interactable: InteractVariant::Talk,
            ..WorldView::default()
        };
        assert_eq!(resolve_interact(&world), InteractVariant::Talk);

        let world = WorldView {
            nearest_interactable: InteractVariant::Custom(Cow::Borrowed("Read note")),
            ..WorldView::default()
        };
        assert_eq!(
            resolve_interact(&world),
            InteractVariant::Custom(Cow::Borrowed("Read note"))
        );
    }

    #[test]
    fn special_neutral_is_n_special() {
        assert_eq!(
            resolve_special(Aim::Neutral, grounded()),
            SpecialVariant::NeutralSpecial
        );
    }

    #[test]
    fn special_up_wins_over_forward_back() {
        // Up cardinal.
        assert_eq!(
            resolve_special(Aim::Up, grounded()),
            SpecialVariant::UpSpecial
        );
        // Diagonal up (forward-up or back-up) also picks UpSpecial —
        // matches the Smash convention that "up special" fires on any
        // stick read with an upward component.
        assert_eq!(
            resolve_special(Aim::ForwardUp, grounded()),
            SpecialVariant::UpSpecial
        );
        assert_eq!(
            resolve_special(Aim::BackUp, grounded()),
            SpecialVariant::UpSpecial
        );
    }

    #[test]
    fn special_down_picks_d_special() {
        assert_eq!(
            resolve_special(Aim::Down, grounded()),
            SpecialVariant::DownSpecial
        );
        assert_eq!(
            resolve_special(Aim::ForwardDown, grounded()),
            SpecialVariant::DownSpecial
        );
    }

    #[test]
    fn special_horizontal_picks_s_special() {
        assert_eq!(
            resolve_special(Aim::Forward, grounded()),
            SpecialVariant::SideSpecial
        );
        assert_eq!(
            resolve_special(Aim::Back, grounded()),
            SpecialVariant::SideSpecial
        );
    }
}
