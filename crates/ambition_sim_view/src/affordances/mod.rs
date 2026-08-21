//! Controlled-body affordances: "what would each input do right now?"
//!
//! The affordance table bridges participant input + controlled-body state +
//! world state to the verb each input would
//! invoke this frame. The HUD reads it to label each on-screen button;
//! gameplay code (today: nothing; future: the attack / jump / shield
//! subsystems) will read the same table so the HUD can never disagree
//! with what actually fires when a button is pressed.
//!
//! ## Shape
//!
//! - [`intent::PlayerIntent`] — pure player-driven input intent
//!   (directional aim today, motion-input history later).
//! - Per-verb variant enums in [`variants`] — closed sets describing
//!   every label/outcome a verb can take (`AttackVariant::DAir`,
//!   `JumpVariant::Climb`, …).
//! - Per-verb pure [`resolvers`] — `(intent, body, world) -> variant`,
//!   trivially unit-testable, callable by gameplay or HUD.
//! - [`interactable_proximity::NearestInteractable`] — frame-snapshot
//!   resource describing the nearest interactable's classification.
//! - [`PlayerAffordances`] resource (this module) — the denormalized
//!   table of variants for every verb, computed once per frame.
//! - [`AffordancesPlugin`] — wires the three compute systems
//!   (intent → proximity → affordances) into the schedule.
//!
//! ## What this replaces
//!
//! The previous design had a flat `PlayerActionContext` POD struct +
//! one growing `label_for` match. That model scaled poorly: each new
//! contextual rule grew both the struct (another `aim_back: bool`
//! field) and the match. The variants + resolvers shape is the same
//! information in a typed, queryable form — adding a new attack
//! variant is a one-arm change in the resolver and one variant in
//! the enum; the HUD updates for free because it just renders the
//! variant's `VariantLabel::text`.

//! ⛔⛔ **THE `portal` FEATURE GATES ARE GONE, and leaving them would have been
//! a silent regression.** In the monolith this code was gated because that crate
//! can be built without portals. This crate cannot: its dependency on the
//! monolith names `features = ["ldtk_runtime", "input", "portal"]`
//! unconditionally, and `ambition_portal2d` is a plain dependency here. So the
//! `not(feature = "portal")` arms were unreachable-but-selected — sim_view has
//! no `portal` feature of its own, so every gate resolved FALSE and
//! `portal_gun_active` became a hardcoded `false`. The HUD would have stopped
//! labelling the portal gun and nothing would have failed.

use ambition_platformer2d_shared_tangle::markers::ControlledSubject;
use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt;
use bevy::prelude::*;

pub mod intent;
pub mod interactable_proximity;
pub mod pogo_proximity;
pub mod resolvers;
pub mod variants;

pub use intent::{Aim, PlayerIntent, compute_aim, compute_controlled_actor_intent};
pub use interactable_proximity::{NearestInteractable, update_nearest_interactable};
pub use pogo_proximity::{PogoTargetBelow, update_pogo_target_below};
pub use resolvers::{
    PlayerBodyView, WorldView, resolve_attack, resolve_dash, resolve_interact, resolve_jump,
    resolve_shield, resolve_special,
};
pub use variants::{
    AttackVariant, DashVariant, InteractVariant, JumpVariant, ShieldVariant, SpecialVariant,
    VariantLabel,
};

/// The denormalized "what would each verb do right now" table.
///
/// Updated each frame by [`compute_player_affordances`] from the
/// current [`PlayerIntent`], the primary player's body, and
/// [`NearestInteractable`]. HUD systems read this resource and pass
/// each field to `VariantLabel::text()` (or, for `Interact`, the
/// `display()` helper that also handles `Custom` prompts).
///
/// Gameplay systems CAN read this resource directly, but the
/// canonical pattern is to call the corresponding `resolve_*` function
/// with whatever data the system already has; the affordance table is
/// the *cached* answer, not the only answer.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct PlayerAffordances {
    pub jump: JumpVariant,
    pub attack: AttackVariant,
    pub shield: ShieldVariant,
    pub dash: DashVariant,
    pub interact: InteractVariant,
    pub special: SpecialVariant,
}

/// Recompute [`PlayerAffordances`] from the CONTROLLED subject's state
/// and the current world view. Skips writing when the table hasn't
/// changed so Bevy's change-detection bit only flips on actual
/// updates — relevant for downstream HUD systems that filter on
/// `Changed<PlayerAffordances>`.
///
/// The body it reads is the [`ControlledSubject`] (the entity carrying
/// `DrivingParticipant(PRIMARY)` — the home avatar, or a possessed actor while
/// possessing), falling back to the primary player when nothing is possessed.
/// The button hints therefore describe what the body you are DRIVING would do,
/// not the vacated home avatar — the same relativity rule the camera, input,
/// and the interact prompt ([`update_nearest_interactable`]) already follow.
pub fn compute_player_affordances(
    intent: Res<PlayerIntent>,
    proximity: Res<NearestInteractable>,
    pogo: Res<PogoTargetBelow>,
    controlled: Option<Res<ControlledSubject>>,
    primary: Query<
        Entity,
        (
            With<ambition_platformer2d_shared_tangle::markers::PlayerEntity>,
            With<ambition_platformer2d_shared_tangle::markers::PrimaryPlayer>,
        ),
    >,
    player_q: Query<(
        &ambition_platformer2d_core::BodyGroundState,
        &ambition_platformer2d_core::BodyMotionFacts,
        &ambition_platformer2d_core::BodyModeState,
        &ambition_platformer2d_core::BodyEnvironmentContact,
        Option<&ambition_portal2d::PortalGun>,
    )>,
    mut affordances: ResMut<PlayerAffordances>,
) {
    // The driven body: possessed subject if any, else the home avatar.
    let subject = controlled
        .and_then(|subject| subject.0)
        .or_else(|| primary.single().ok());
    let Some(subject) = subject else {
        // No player yet (e.g. boot-up before `setup_simulation_system` runs).
        // Leave affordances at their defaults; the HUD renders "Jump / Attack /
        // Shield / Dash / Interact / Special" which is the correct cold-start
        // label.
        return;
    };
    let Ok((ground, facts, body_mode, env_contact, portal_gun)) = player_q.get(subject) else {
        return;
    };
    let body = PlayerBodyView {
        is_aerial: !ground.on_ground,
        on_ledge: facts.ledge.is_some(),
        is_morphed: matches!(
            body_mode.body_mode,
            ambition_platformer2d_core::BodyMode::MorphBall
        ),
        is_swimming: env_contact.water.is_some(),
    };
    let world = WorldView {
        nearest_interactable: proximity.0.clone(),
        pogo_target_below: pogo.0,
        portal_gun_active: portal_gun.is_some_and(|g| g.active),
    };

    let next = PlayerAffordances {
        jump: resolve_jump(body),
        attack: resolve_attack(intent.aim, body, &world),
        shield: resolve_shield(body),
        dash: resolve_dash(body),
        interact: resolve_interact(&world),
        special: resolve_special(intent.aim, body),
    };
    if *affordances != next {
        *affordances = next;
    }
}

/// SystemSet for the affordance compute chain. HUD systems should run
/// `.after(AffordancesSystemSet::Compute)` so they see this frame's
/// values; gameplay systems that consume affordances (currently none)
/// would too.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AffordancesSystemSet {
    /// The three-stage compute chain: intent → proximity → affordances.
    Compute,
}

/// Bevy plugin wiring the affordances pipeline. Registers the three
/// resources and three systems in `Update`, chained so they execute in
/// the right order each frame.
pub struct AffordancesPlugin;

impl Plugin for AffordancesPlugin {
    fn build(&self, app: &mut App) {
        let sim = app.sim_schedule();
        app.init_resource::<PlayerIntent>()
            .init_resource::<NearestInteractable>()
            .init_resource::<PogoTargetBelow>()
            .init_resource::<PlayerAffordances>()
            .add_systems(
                sim,
                (
                    compute_controlled_actor_intent,
                    update_nearest_interactable,
                    update_pogo_target_below,
                    compute_player_affordances,
                )
                    .chain()
                    // `compute_controlled_actor_intent` reads the controlled
                    // body's slot frame (`SlotControls`, published by
                    // `populate_slot_controls` inside the `PlayerInput` set)
                    // rather than the global `Res<ControlFrame>`. Pin the
                    // chain after primary-slot publication so the intent reflects
                    // this frame's finalized input without an entity-local copy.
                    .after(ambition_platformer2d_actor_monolith::control::PrimarySlotInputCommit)
                    .in_set(AffordancesSystemSet::Compute),
            );
        // The device-presentation half that lived here (`ActiveInputMethod`,
        // `detect_active_input_method`, `glyph_for`) moved to
        // `ambition_input`: the per-seat `SeatActiveDevices` is the one
        // active-device authority, and `ambition_input::glyph_for` draws from
        // it. Every input those items took was input-crate vocabulary; the
        // touch overlay no longer names this crate for glyphs.
    }
}

#[cfg(test)]
mod tests;
