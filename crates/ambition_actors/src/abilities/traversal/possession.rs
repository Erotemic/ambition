//! Possession — Down + Interact transfers the player's controller brain onto a
//! nearby actor. Bosses are valid targets: the boss tick consumes
//! `Brain::Player`, driving boss movement AND its authored specials via a
//! deterministic input mapping over `BossCapability`.
//!
//! Possession is NOT input-copying. It is **brain transfer**. On possess we move
//! [`Brain::Player`]`(PlayerSlot::PRIMARY)` off the home avatar and onto the
//! target actor. The target then reads slot-0 input through the SAME
//! universal-brain path every player-controlled body uses:
//! `Brain::Player` → [`SlotControls`] → its own `ActorControl` → its own
//! `ActionSet`. It moves, attacks, and fires through its own body path — no
//! `Possessed` marker, no input mirror, no possession-specific override in the
//! actor tick.
//!
//! The home avatar, now without a player brain, is inert (a neutral
//! `ActorControl`, no local attack authority) until release restores its brain.
//!
//! Everything downstream — camera, portal viewer, nameplates, the melee
//! lifecycle — derives from [`ControlledSubject`], i.e. "who carries
//! `Brain::Player(PRIMARY)` this frame", never from a possession flag. That is
//! the whole point: possession is proof that control is actor-generic.
//!
//! Bosses are in scope: the boss tick (`crate::features::ecs::bosses::tick`)
//! handles a `Brain::Player` boss — reading slot input for movement and mapping
//! attack/special input onto its authored `BossCapability`. Restricting WHICH
//! boss is possessable (progression / design) is a targeting-policy gate to add
//! above this trigger, not a "bosses can never be controlled" barrier.

use bevy::prelude::*;

use ambition_characters::brain::{ActorControl, Brain, PlayerSlot};

use ambition_platformer_primitives::markers::ControlledSubject;

use crate::actor::BodyKinematics;
use crate::actor::PlayerEntity;
use crate::features::{CenteredAabb, FeatureSimEntity};

/// Brain-transfer bookkeeping for possession.
///
/// `controlled == None` means the local player drives the home avatar;
/// `Some(actor)` means slot-0's brain has been transferred to `actor`. The
/// remaining fields remember what to restore on release. This resource is
/// possession-INTERNAL: no gameplay/presentation system branches on it. Ask
/// [`ControlledSubject`] instead.
#[derive(Resource, Default)]
pub struct PossessionState {
    /// The actor currently possessed (its `Brain::Player(PRIMARY)` was
    /// transferred here), or `None` while driving the home avatar.
    pub possessed: Option<Entity>,
    /// The home avatar whose player brain was vacated, restored on release.
    pub home: Option<Entity>,
    /// The possessed actor's brain before transfer, restored on release.
    pub restore_brain: Option<Brain>,
}

/// Derive [`ControlledSubject`] from the ECS: the entity carrying
/// `Brain::Player(PRIMARY)`. Runs early each frame; there is exactly one such
/// entity during normal play (the home avatar, or the possessed actor while
/// possessing). A one-frame lag across a possess/release transition (commands
/// apply at a later sync point) is benign — no consumer double-acts, because
/// each body only emits actions for itself and only when it carries the brain.
pub fn resolve_controlled_subject(
    brains: Query<(Entity, &Brain)>,
    mut subject: ResMut<ControlledSubject>,
) {
    // HARD INVARIANT: exactly one entity carries `Brain::Player(PRIMARY)` during
    // normal play (zero only during a load/transition frame). Two is a bug the
    // whole architecture rests on NOT happening — a stale home brain that wasn't
    // vacated, or a double-assigned slot. Surface it loudly instead of silently
    // picking one and diverging.
    let mut chosen = None;
    let mut count = 0u32;
    for (entity, brain) in &brains {
        if brain.player_slot() == Some(PlayerSlot::PRIMARY) {
            count += 1;
            if chosen.is_none() {
                chosen = Some(entity);
            }
        }
    }
    debug_assert!(
        count <= 1,
        "control invariant violated: {count} entities carry Brain::Player(PRIMARY) \
         (expected exactly one); possession/vacate left a stale player brain"
    );
    if count > 1 {
        bevy::log::error!(
            "control invariant: {count} entities carry Brain::Player(PRIMARY); \
             using the first as the controlled subject"
        );
    }
    subject.0 = chosen;
}

/// Possession reach (px): Down+Interact possesses the nearest candidate within this.
const POSSESS_RADIUS: f32 = 150.0;

/// Seconds the player must **hold** Down+Interact (with a candidate in range) to
/// commit a possession. A deliberate gesture so you don't possess by brushing
/// the button mid-fight; releasing fully is instant (a single press).
const POSSESS_HOLD_S: f32 = 2.0;

/// Stick deflection (gravity-resolved "down") past which the player counts as
/// holding **Down** for the possession gesture — the same threshold drop-through
/// uses.
pub const POSSESS_DOWN_THRESHOLD: f32 = 0.35;

/// True iff the player's stick is held "down" in the GRAVITY-resolved frame past
/// [`POSSESS_DOWN_THRESHOLD`]. The possession gesture is **Down + Interact**;
/// exposed so the interaction system can SUPPRESS a normal interact while Down is
/// held — i.e. Down+Interact is *claimed* by possession and never opens a door /
/// NPC. Sharing it keeps both systems agreeing on what "down" means under any
/// gravity orientation.
pub fn holding_descend(
    axis_x: f32,
    axis_y: f32,
    gravity_dir: ambition_engine_core::Vec2,
    movement_mode: ambition_engine_core::InputFrameMode,
) -> bool {
    ambition_engine_core::AccelerationFrame::new(gravity_dir)
        .resolve_input(movement_mode, axis_x, axis_y)
        .y
        > POSSESS_DOWN_THRESHOLD
}

/// `Down + Interact` controls possession: **hold ~2s** (with a candidate in
/// range) to transfer your controller brain onto the nearest non-boss actor;
/// press it again to release. `Down` is the gravity-resolved descend axis past
/// [`POSSESS_DOWN_THRESHOLD`]. The hold runs on real time (`raw_dt`) so
/// bullet-time doesn't change the feel.
///
/// The gesture belongs to slot 0, so it reads the local device frame
/// (`Res<ControlFrame>`) directly rather than any body's input — the home avatar
/// is inert (neutral input) while vacated, but the local device still drives the
/// release.
///
/// **This is the ONE sim system that holds the global `ControlFrame`, which makes
/// possession local-player-only: a second player could never possess anything.**
/// It is enumerated as the sole `Bridge::Slot0Gesture` in
/// `ambition_runtime/tests/control_frame_lint.rs`, whose allowlist doubles as the
/// N1 multiplayer checklist. The fix is to read the acting slot's
/// `SlotInteractionState` / `SlotControls`, exactly as `interaction_input_system`
/// already does for the interact buffer — a behavior change, not a refactor, so
/// it is deferred rather than hidden.
#[allow(clippy::too_many_arguments)]
pub fn possession_trigger_system(
    control: Res<ambition_input::ControlFrame>,
    gravity_field: Option<Res<crate::physics::GravityField>>,
    user_settings: Option<Res<ambition_persistence::settings::UserSettings>>,
    world_time: Res<ambition_time::WorldTime>,
    mut hold_timer: Local<f32>,
    mut prev_down_interact: Local<bool>,
    mut state: ResMut<PossessionState>,
    mut commands: Commands,
    // Home avatar kinematics: its position seeds the candidate search, and on
    // release it steps out to the vacated actor's spot (camera continuity).
    // SLOT-0 BY DESIGN: the HOME AVATAR is a real concept — the body slot 0 owns and
    // returns to on release. It is precisely the body that is NOT the controlled
    // subject while possession is active, so it cannot be found any other way.
    mut home_q: Query<(Entity, &mut BodyKinematics), crate::actor::PrimaryPlayerOnly>,
    // Possession candidates: any brain-driven feature body — INCLUDING bosses.
    // Bosses are valid controllable bodies (their tick consumes `Brain::Player`),
    // so there is no `Without<BossConfig>` barrier here. Restricting WHICH boss is
    // possessable (progression/design) is a targeting-policy gate to add above
    // this, not a "bosses can never be controlled" exclusion in the body model.
    candidates: Query<
        (Entity, &CenteredAabb),
        (
            With<FeatureSimEntity>,
            With<ActorControl>,
            With<Brain>,
            Without<PlayerEntity>,
        ),
    >,
    // The target's authored brain, snapshotted for restore on release. Its
    // faction is NOT touched — effective allegiance (`Brain::Player` ⇒ combat
    // treats it as Player) makes the possessed body fight its former allies
    // without mutating `ActorFaction`.
    target_data: Query<&Brain>,
    // Read-only AABB lookup for the vacate exit on release.
    actor_aabbs: Query<&CenteredAabb>,
) {
    let gravity_dir = crate::physics::gravity_dir_or_default(gravity_field.as_deref());
    let movement_mode = user_settings.as_deref().map_or(
        ambition_engine_core::InputFrameMode::DEFAULT_MOVEMENT,
        |s| s.gameplay.movement_frame_mode,
    );
    let down = holding_descend(control.axis_x, control.axis_y, gravity_dir, movement_mode);
    // The gesture is a HOLD, so it accumulates on the interact button being
    // HELD — not the single-frame `interact_pressed` edge (which doors / the
    // heal-shrine also consume, resetting the hold every frame). The release is
    // the rising edge of (down + held), tracked via `prev_down_interact`.
    let down_interact = down && control.interact_held;
    let release_edge = down_interact && !*prev_down_interact;
    *prev_down_interact = down_interact;

    // Already possessing → a fresh Down+Interact press releases (no hold).
    if let Some(target) = state.possessed {
        *hold_timer = 0.0;
        if release_edge {
            release_possession(&mut commands, &mut state, target, &actor_aabbs, &mut home_q);
        }
        return;
    }

    // Not possessing → accumulate the hold; commit at the threshold.
    if !down_interact {
        *hold_timer = 0.0;
        return;
    }
    *hold_timer += world_time.raw_dt;
    if *hold_timer < POSSESS_HOLD_S {
        return;
    }
    *hold_timer = 0.0;

    let Ok((home_entity, home_kin)) = home_q.single() else {
        return;
    };
    let home_pos = home_kin.pos;
    let nearest = candidates
        .iter()
        .map(|(entity, aabb)| (entity, (aabb.center - home_pos).length()))
        .filter(|(_, dist)| *dist <= POSSESS_RADIUS)
        .min_by(|a, b| a.1.total_cmp(&b.1));
    let Some((target, _)) = nearest else {
        return;
    };
    let Ok(target_brain) = target_data.get(target) else {
        return;
    };

    // BRAIN TRANSFER. Remember the target's brain to restore, then move the
    // player brain from the home avatar to the target. Both bodies get a fresh
    // neutral `ActorControl` so no stale edge-triggered intent (a held jump, a
    // pressed attack) leaks across the handover. The target's `ActorFaction` is
    // left untouched — effective allegiance handles its player-side combat.
    state.home = Some(home_entity);
    state.restore_brain = Some(target_brain.clone());
    state.possessed = Some(target);

    commands
        .entity(home_entity)
        .remove::<Brain>()
        .insert(ActorControl::default());
    commands
        .entity(target)
        .insert(Brain::Player(PlayerSlot::PRIMARY))
        .insert(ActorControl::default());
}

/// Restore the home avatar's player brain and the target's authored brain +
/// faction, then step the home body out to the vacated actor's position so the
/// camera (which was following the actor) doesn't snap back.
fn release_possession(
    commands: &mut Commands,
    state: &mut PossessionState,
    target: Entity,
    actor_aabbs: &Query<&CenteredAabb>,
    // SLOT-0 BY DESIGN: the home avatar (see `possession_trigger_system`).
    home_q: &mut Query<(Entity, &mut BodyKinematics), crate::actor::PrimaryPlayerOnly>,
) {
    state.possessed = None;

    // Restore the actor's authored brain, clearing stale edges. Its faction was
    // never touched (effective allegiance), so there is nothing to restore.
    if let Some(brain) = state.restore_brain.take() {
        if let Ok(mut ec) = commands.get_entity(target) {
            ec.insert(brain).insert(ActorControl::default());
        }
    }

    // Restore the home avatar's player brain and vacate-exit to the actor's spot.
    if let Some(home) = state.home.take() {
        if let Ok(mut ec) = commands.get_entity(home) {
            ec.insert(Brain::Player(PlayerSlot::PRIMARY))
                .insert(ActorControl::default());
        }
        if let (Ok(aabb), Ok((_, mut kin))) = (actor_aabbs.get(target), home_q.get_mut(home)) {
            kin.pos = aabb.center;
            kin.vel = ambition_engine_core::Vec2::ZERO;
        }
    }
}

/// If the possessed actor is gone (despawned / removed), hand control back to
/// the home avatar so the player isn't stranded driving nothing. The actor's
/// brain can't be restored (it's gone); only the home brain is re-attached.
pub fn release_possession_if_target_lost(
    mut state: ResMut<PossessionState>,
    mut commands: Commands,
    still_present: Query<(), With<Brain>>,
) {
    let Some(target) = state.possessed else {
        return;
    };
    if still_present.get(target).is_ok() {
        return;
    }
    // Target vanished mid-possession.
    if let Some(home) = state.home.take() {
        if let Ok(mut ec) = commands.get_entity(home) {
            ec.insert(Brain::Player(PlayerSlot::PRIMARY))
                .insert(ActorControl::default());
        }
    }
    state.possessed = None;
    state.restore_brain = None;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actor::BodyBaseSize;
    use crate::actor::PrimaryPlayer;
    use crate::features::ActorFaction;
    use ambition_characters::brain::{PlayerSlot, StateMachineCfg};

    fn vec2(x: f32, y: f32) -> ambition_engine_core::Vec2 {
        ambition_engine_core::Vec2::new(x, y)
    }

    /// App with the trigger + 1s/frame real time, so 2 held frames clear the 2s hold.
    fn trigger_app() -> App {
        let mut app = App::new();
        app.insert_resource(ambition_input::ControlFrame::default());
        app.insert_resource(ambition_time::WorldTime {
            raw_dt: 1.0,
            scaled_dt: 1.0,
        });
        app.init_resource::<PossessionState>();
        app.add_systems(
            Update,
            (possession_trigger_system, release_possession_if_target_lost).chain(),
        );
        app
    }

    fn spawn_home(app: &mut App) -> Entity {
        app.world_mut()
            .spawn((
                PlayerEntity,
                PrimaryPlayer,
                Brain::Player(PlayerSlot::PRIMARY),
                ActorControl::default(),
                BodyKinematics {
                    pos: vec2(0.0, 0.0),
                    vel: vec2(0.0, 0.0),
                    size: vec2(24.0, 40.0),
                    facing: 1.0,
                },
                BodyBaseSize {
                    base_size: vec2(24.0, 40.0),
                },
            ))
            .id()
    }

    fn spawn_candidate(app: &mut App, pos: ambition_engine_core::Vec2) -> Entity {
        app.world_mut()
            .spawn((
                FeatureSimEntity,
                CenteredAabb::new(pos, vec2(12.0, 16.0)),
                Brain::StateMachine(StateMachineCfg::StandStill),
                ActorControl::default(),
                ActorFaction::Enemy,
            ))
            .id()
    }

    fn brain_slot(app: &App, e: Entity) -> Option<PlayerSlot> {
        app.world().get::<Brain>(e).and_then(|b| b.player_slot())
    }

    fn faction_of(app: &App, e: Entity) -> ActorFaction {
        *app.world().get::<ActorFaction>(e).unwrap()
    }

    fn hold_down_interact(app: &mut App, held: bool) {
        let mut control = app
            .world_mut()
            .resource_mut::<ambition_input::ControlFrame>();
        control.axis_y = if held { 1.0 } else { 0.0 };
        control.interact_held = held;
    }

    #[test]
    fn possession_transfers_the_player_brain_and_release_restores_it() {
        let mut app = trigger_app();
        let home = spawn_home(&mut app);
        let actor = spawn_candidate(&mut app, vec2(80.0, 0.0)); // in range

        // Before possession: home carries the player brain; the actor its own.
        assert_eq!(brain_slot(&app, home), Some(PlayerSlot::PRIMARY));
        assert_eq!(brain_slot(&app, actor), None);

        // Hold Down+Interact: 1s, then 2s → crosses the threshold → possess.
        hold_down_interact(&mut app, true);
        app.update(); // hold_timer = 1.0
        assert_eq!(brain_slot(&app, actor), None, "not possessed mid-hold");
        app.update(); // hold_timer = 2.0 ≥ threshold → transfer

        // After possession: the ACTOR carries the player brain; the home avatar
        // no longer does; the actor is player-aligned; its old brain is stashed.
        assert_eq!(brain_slot(&app, actor), Some(PlayerSlot::PRIMARY));
        assert_eq!(
            brain_slot(&app, home),
            None,
            "home avatar's player brain is vacated"
        );
        assert!(app.world().get::<Brain>(home).is_none());
        // Effective allegiance: the target's AUTHORED faction is NOT mutated by
        // possession (it stays Enemy). Combat treats it as Player because it
        // carries `Brain::Player` — verified by the targeting/damage tests — so
        // there is no flip to bookkeep and no restore on release.
        assert_eq!(
            faction_of(&app, actor),
            ActorFaction::Enemy,
            "possession must NOT overwrite the authored faction"
        );
        assert_eq!(
            app.world().resource::<PossessionState>().possessed,
            Some(actor)
        );
        // The REPORTED BUG's root cause is gone: the vacated home avatar has a
        // neutral `ActorControl` and no brain to repopulate it, so it emits no
        // melee/attack this frame or any frame while possessed — attack authority
        // can only originate from the body carrying `Brain::Player`.
        assert_eq!(
            app.world().get::<ActorControl>(home).map(|c| c.0),
            Some(ambition_characters::actor::control::ActorControlFrame::neutral()),
            "vacated home avatar's control frame is cleared — no attack authority"
        );

        // Release: a fresh Down+Interact press hands control back.
        hold_down_interact(&mut app, false);
        app.update();
        hold_down_interact(&mut app, true);
        app.update();

        assert_eq!(
            brain_slot(&app, home),
            Some(PlayerSlot::PRIMARY),
            "release restores the home avatar's player brain"
        );
        assert_eq!(
            brain_slot(&app, actor),
            None,
            "release restores the actor's autonomous brain"
        );
        assert_eq!(
            faction_of(&app, actor),
            ActorFaction::Enemy,
            "authored faction unchanged across the whole possess/release cycle"
        );
        assert!(app
            .world()
            .resource::<PossessionState>()
            .possessed
            .is_none());
        // Vacate exit: the home avatar stepped out where the actor stood.
        let home_pos = app
            .world_mut()
            .query_filtered::<&BodyKinematics, With<PlayerEntity>>()
            .single(app.world())
            .unwrap()
            .pos;
        assert_eq!(home_pos, vec2(80.0, 0.0));
    }

    #[test]
    fn exactly_one_body_carries_the_player_brain_before_and_after() {
        let mut app = trigger_app();
        app.init_resource::<ControlledSubject>();
        app.add_systems(Update, resolve_controlled_subject);
        let home = spawn_home(&mut app);
        let actor = spawn_candidate(&mut app, vec2(80.0, 0.0));
        app.update();
        assert_eq!(app.world().resource::<ControlledSubject>().0, Some(home));

        hold_down_interact(&mut app, true);
        app.update(); // hold_timer = 1.0
        app.update(); // hold_timer = 2.0 → brain transfer commands queued
        app.update(); // transfer applied; resolver re-derives the subject
        assert_eq!(
            app.world().resource::<ControlledSubject>().0,
            Some(actor),
            "controlled subject follows the player brain onto the possessed actor"
        );
    }

    #[test]
    fn a_brief_tap_does_not_possess() {
        let mut app = trigger_app();
        let _home = spawn_home(&mut app);
        let actor = spawn_candidate(&mut app, vec2(80.0, 0.0));
        hold_down_interact(&mut app, true);
        app.update();
        hold_down_interact(&mut app, false);
        app.update();
        assert_eq!(brain_slot(&app, actor), None, "a brief tap doesn't possess");
    }

    #[test]
    fn out_of_range_actors_are_not_possessed() {
        let mut app = trigger_app();
        let _home = spawn_home(&mut app);
        let actor = spawn_candidate(&mut app, vec2(900.0, 0.0)); // far out of range
        hold_down_interact(&mut app, true);
        app.update();
        app.update();
        app.update();
        assert_eq!(
            brain_slot(&app, actor),
            None,
            "nothing in range → no transfer"
        );
    }

    /// The mandate's headline invariant: while controlling a possessed target,
    /// pressing attack emits `ActorActionMessage` for the TARGET, and the vacated
    /// home avatar emits nothing. The collapse of the bug: attack authority
    /// follows the body carrying `Brain::Player`, resolved by the SAME
    /// `emit_brain_action_messages` stream for every body.
    #[test]
    fn attack_while_controlling_target_emits_only_for_the_target() {
        use ambition_characters::actor::ActorPose;
        use ambition_characters::brain::{
            emit_brain_action_messages, ActionSet, ActorActionMessage, MeleeActionSpec, SwipeSpec,
        };

        let mut app = App::new();
        app.add_message::<ActorActionMessage>();
        let kit = ActionSet {
            melee: Some(MeleeActionSpec::Swipe(SwipeSpec::STRIKER_DEFAULT)),
            ..Default::default()
        };
        // Vacated home avatar: neutral control (its brain was transferred away),
        // but it still owns a melee ActionSet + a pose.
        let home = app
            .world_mut()
            .spawn((ActorControl::default(), kit.clone(), ActorPose::default()))
            .id();
        // Possessed target: its `Brain::Player` produced a melee-pressed frame.
        let mut frame = ambition_characters::actor::control::ActorControlFrame::neutral();
        frame.melee_pressed = true;
        frame.facing = 1.0;
        let target = app
            .world_mut()
            .spawn((ActorControl(frame), kit, ActorPose::default()))
            .id();

        app.add_systems(Update, emit_brain_action_messages);
        app.update();

        let msgs: Vec<_> = app
            .world_mut()
            .resource_mut::<bevy::ecs::message::Messages<ActorActionMessage>>()
            .drain()
            .collect();
        let melee: Vec<_> = msgs.iter().filter(|m| m.is_melee()).collect();
        assert_eq!(melee.len(), 1, "exactly one melee action this frame");
        assert_eq!(
            melee[0].actor, target,
            "the attack originates from the possessed target"
        );
        assert!(
            melee.iter().all(|m| m.actor != home),
            "the vacated home avatar emits no attack"
        );
    }

    #[test]
    fn losing_the_target_hands_control_back_to_home() {
        let mut app = trigger_app();
        let home = spawn_home(&mut app);
        let actor = spawn_candidate(&mut app, vec2(80.0, 0.0));
        hold_down_interact(&mut app, true);
        app.update();
        app.update();
        assert_eq!(brain_slot(&app, actor), Some(PlayerSlot::PRIMARY));
        // The possessed actor despawns (died / left the room).
        app.world_mut().entity_mut(actor).despawn();
        app.update();
        assert_eq!(
            brain_slot(&app, home),
            Some(PlayerSlot::PRIMARY),
            "the home avatar reclaims control when the possessed body is lost"
        );
        assert!(app
            .world()
            .resource::<PossessionState>()
            .possessed
            .is_none());
    }
}
