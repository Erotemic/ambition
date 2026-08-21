//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

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

    // Holding the portal gun rebinds Interact to "Mode Switch"…
    let world = WorldView {
        portal_gun_active: true,
        ..WorldView::default()
    };
    assert_eq!(resolve_interact(&world), InteractVariant::ModeSwitch);
    // …but a genuine interactable still wins.
    let world = WorldView {
        nearest_interactable: InteractVariant::Talk,
        portal_gun_active: true,
        ..WorldView::default()
    };
    assert_eq!(resolve_interact(&world), InteractVariant::Talk);
    // Nothing nearby and no gun → None (the HUD shows "Context").
    assert_eq!(
        resolve_interact(&WorldView::default()),
        InteractVariant::None
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
