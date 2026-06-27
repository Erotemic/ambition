//! Player affordances: "what would each button do right now?"
//!
//! The affordance table is the single source of truth bridging player
//! input + player state + world state to the verb each input would
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

use bevy::prelude::*;

pub mod devices;
pub mod intent;
pub mod interactable_proximity;
pub mod pogo_proximity;
pub mod resolvers;
pub mod variants;

pub use devices::{
    detect_active_input_method, glyph_for, ActiveInputMethod, GamepadKind, InputMethod,
};
pub use intent::{compute_aim, compute_controlled_actor_intent, Aim, PlayerIntent};
pub use interactable_proximity::{update_nearest_interactable, NearestInteractable};
pub use pogo_proximity::{update_pogo_target_below, PogoTargetBelow};
pub use resolvers::{
    resolve_attack, resolve_dash, resolve_interact, resolve_jump, resolve_shield, resolve_special,
    PlayerBodyView, WorldView,
};
pub use variants::{
    AttackVariant, DashVariant, IconId, InteractVariant, JumpVariant, ShieldVariant,
    SpecialVariant, VariantLabel,
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

/// Recompute [`PlayerAffordances`] from the primary player's state
/// and the current world view. Skips writing when the table hasn't
/// changed so Bevy's change-detection bit only flips on actual
/// updates — relevant for downstream HUD systems that filter on
/// `Changed<PlayerAffordances>`.
pub fn compute_player_affordances(
    intent: Res<PlayerIntent>,
    proximity: Res<NearestInteractable>,
    pogo: Res<PogoTargetBelow>,
    #[cfg(feature = "portal")] player_q: Query<
        (
            &crate::actor::BodyGroundState,
            &crate::actor::BodyLedgeState,
            &crate::actor::BodyModeState,
            &crate::actor::BodyEnvironmentContact,
            Option<&crate::portal::PortalGun>,
        ),
        (
            With<crate::actor::PlayerEntity>,
            With<crate::actor::PrimaryPlayer>,
        ),
    >,
    #[cfg(not(feature = "portal"))] player_q: Query<
        (
            &crate::actor::BodyGroundState,
            &crate::actor::BodyLedgeState,
            &crate::actor::BodyModeState,
            &crate::actor::BodyEnvironmentContact,
        ),
        (
            With<crate::actor::PlayerEntity>,
            With<crate::actor::PrimaryPlayer>,
        ),
    >,
    mut affordances: ResMut<PlayerAffordances>,
) {
    #[cfg(feature = "portal")]
    let Ok((ground, ledge, body_mode, env_contact, portal_gun)) = player_q.single() else {
        // No primary player yet (e.g. boot-up before
        // `setup_simulation_system` runs). Leave affordances at their
        // defaults; the HUD renders "Jump / Attack / Shield / Dash /
        // Interact / Special" which is the correct cold-start label.
        return;
    };
    #[cfg(not(feature = "portal"))]
    let Ok((ground, ledge, body_mode, env_contact)) = player_q.single() else {
        return;
    };
    let body = PlayerBodyView {
        is_aerial: !ground.on_ground,
        on_ledge: ledge.grab.is_some(),
        is_morphed: matches!(body_mode.body_mode, ambition_engine_core::BodyMode::MorphBall),
        is_swimming: env_contact.water.is_some(),
    };
    let world = WorldView {
        nearest_interactable: proximity.0.clone(),
        pogo_target_below: pogo.0,
        #[cfg(feature = "portal")]
        portal_gun_active: portal_gun.is_some_and(|g| g.active),
        #[cfg(not(feature = "portal"))]
        portal_gun_active: false,
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
        app.init_resource::<PlayerIntent>()
            .init_resource::<NearestInteractable>()
            .init_resource::<PogoTargetBelow>()
            .init_resource::<PlayerAffordances>()
            .init_resource::<ActiveInputMethod>()
            .add_systems(
                Update,
                (
                    compute_controlled_actor_intent,
                    update_nearest_interactable,
                    update_pogo_target_below,
                    compute_player_affordances,
                )
                    .chain()
                    // `compute_controlled_actor_intent` now reads the
                    // actor-local `PlayerInputFrame` (mirrored inside the
                    // `PlayerInput` set) rather than the global
                    // `Res<ControlFrame>` (populated before
                    // `CoreSimulation`). Pin the chain after the sync so
                    // the intent still reflects this frame's input.
                    .after(crate::player::sync_local_player_input_frame)
                    .in_set(AffordancesSystemSet::Compute),
            )
            // Active-input-method detection runs unchained because it
            // only reads input resources and writes its own resource —
            // no ordering dependency on the affordance compute chain.
            // HUD systems that consume both `PlayerAffordances` and
            // `ActiveInputMethod` should pin `.after(AffordancesSystemSet::Compute)`
            // (which transitively orders after detection too, since
            // Bevy's scheduler is allowed to reorder within a frame).
            .add_systems(Update, detect_active_input_method);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_engine_core as ae;

    /// Minimal app harness: spawns a primary player + drives one
    /// `app.update()` so the affordance compute chain runs end-to-end
    /// without pulling in the whole sandbox plugin graph.
    fn build_test_app() -> (App, Entity) {
        use ambition_input::ControlFrame;
        use crate::actor::{BodyKinematics, PlayerEntity, PrimaryPlayer};
        use crate::actor::{BodyModeState, BodyEnvironmentContact, BodyGroundState, BodyLedgeState};
        use crate::player::{PlayerInputFrame};

        let mut app = App::new();
        // `detect_active_input_method` reads `Res<ButtonInput<KeyCode>>`
        // and `Res<Touches>`; Bevy normally creates them via
        // `InputPlugin`. Initialise them directly so the test app
        // doesn't depend on the full input plugin graph. `ControlFrame`
        // is no longer read by the compute chain (it reads the actor's
        // `PlayerInputFrame`), but keep it so the harness still mirrors
        // the production resource set.
        app.init_resource::<ControlFrame>()
            .init_resource::<bevy::input::ButtonInput<KeyCode>>()
            .init_resource::<bevy::input::touch::Touches>()
            .add_plugins(AffordancesPlugin);
        // The affordance compute reads exactly these four cluster
        // components: ground (on_ground), ledge (grab), body_mode
        // (body_mode), env_contact (water). Plus kinematics for the
        // intent system's facing read and `PlayerInputFrame` for the
        // actor-local aim. Start with grounded baseline + neutral input.
        let entity = app
            .world_mut()
            .spawn((
                PlayerEntity,
                PrimaryPlayer,
                PlayerInputFrame::default(),
                BodyKinematics::default(),
                BodyGroundState {
                    on_ground: true,
                    ..Default::default()
                },
                BodyLedgeState::default(),
                BodyModeState::default(),
                BodyEnvironmentContact::default(),
            ))
            .id();
        (app, entity)
    }

    fn read_affordances(app: &App) -> PlayerAffordances {
        app.world().resource::<PlayerAffordances>().clone()
    }

    /// Stamp the controlled actor's local input axes (the intent compute
    /// reads `PlayerInputFrame`, not the global `Res<ControlFrame>`).
    fn set_axis(app: &mut App, player: Entity, x: f32, y: f32) {
        let mut input = app
            .world_mut()
            .get_mut::<crate::player::PlayerInputFrame>(player)
            .unwrap();
        input.frame.axis_x = x;
        input.frame.axis_y = y;
    }

    #[test]
    fn default_grounded_neutral_player_reads_baseline_labels() {
        let (mut app, _) = build_test_app();
        app.update();
        let aff = read_affordances(&app);
        assert_eq!(aff.jump, JumpVariant::Jump);
        assert_eq!(aff.attack, AttackVariant::Jab);
        assert_eq!(aff.shield, ShieldVariant::Shield);
        assert_eq!(aff.dash, DashVariant::Dash);
        assert_eq!(aff.interact, InteractVariant::None);
        // Neutral aim → `NeutralSpecial` (today: fireball under the
        // hood). The resolver's neutral arm is the cold-start label
        // a player sees before pushing the stick.
        assert_eq!(aff.special, SpecialVariant::NeutralSpecial);
    }

    #[test]
    fn special_dispatches_on_aim_direction() {
        let (mut app, player_entity) = build_test_app();
        // Push axis_y down → DownSpecial.
        set_axis(&mut app, player_entity, 0.0, 1.0);
        app.update();
        assert_eq!(read_affordances(&app).special, SpecialVariant::DownSpecial);

        // Push axis_y up → UpSpecial.
        set_axis(&mut app, player_entity, 0.0, -1.0);
        app.update();
        assert_eq!(read_affordances(&app).special, SpecialVariant::UpSpecial);

        // Side-stick (forward relative to right-facing) → SideSpecial.
        {
            let mut entity = app.world_mut().entity_mut(player_entity);
            let mut kin = entity.get_mut::<crate::actor::BodyKinematics>().unwrap();
            kin.facing = 1.0;
        }
        set_axis(&mut app, player_entity, 1.0, 0.0);
        app.update();
        assert_eq!(read_affordances(&app).special, SpecialVariant::SideSpecial);
    }

    #[test]
    fn airborne_player_with_down_aim_reads_as_dair() {
        let (mut app, player_entity) = build_test_app();
        // Push axis_y down (sim convention: +Y is down).
        set_axis(&mut app, player_entity, 0.0, 1.0);
        // Lift the player off the ground.
        {
            let mut entity = app.world_mut().entity_mut(player_entity);
            let mut ground = entity
                .get_mut::<crate::actor::BodyGroundState>()
                .unwrap();
            ground.on_ground = false;
        }
        app.update();
        let aff = read_affordances(&app);
        assert_eq!(aff.attack, AttackVariant::DAir);
        // Dash also flips when aerial.
        assert_eq!(aff.dash, DashVariant::Dodge);
    }

    #[test]
    fn ledge_grab_flips_jump_and_shield() {
        let (mut app, player_entity) = build_test_app();
        {
            let mut entity = app.world_mut().entity_mut(player_entity);
            let mut ledge = entity.get_mut::<crate::actor::BodyLedgeState>().unwrap();
            ledge.grab = Some(ae::LedgeGrabState::hanging(ae::LedgeContact {
                wall_normal_x: 1.0,
                anchor: ae::Vec2::ZERO,
                climb_target: ae::Vec2::ZERO,
            }));
        }
        app.update();
        let aff = read_affordances(&app);
        assert_eq!(aff.jump, JumpVariant::Climb);
        assert_eq!(aff.shield, ShieldVariant::Roll);
    }

    #[test]
    fn b_air_fires_when_aim_opposes_facing_aerial() {
        let (mut app, player_entity) = build_test_app();
        {
            let mut ent = app.world_mut().entity_mut(player_entity);
            ent.get_mut::<crate::actor::BodyGroundState>()
                .unwrap()
                .on_ground = false;
            ent.get_mut::<crate::actor::BodyKinematics>()
                .unwrap()
                .facing = 1.0;
        }
        // Push stick left (negative X) — opposing facing-right.
        set_axis(&mut app, player_entity, -1.0, 0.0);
        app.update();
        let aff = read_affordances(&app);
        assert_eq!(aff.attack, AttackVariant::BAir);
    }
}
