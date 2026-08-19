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

use crate::features::TemporaryControl;
use ambition_platformer2d_shared_tangle::lifecycle::{
    RoomScopedEntity, SessionScopeId, SessionScopedEntity,
};
use ambition_platformer2d_shared_tangle::markers::ControlledSubject;
use ambition_platformer2d_shared_tangle::sim_id::SimId;

use crate::actor::PlayerEntity;
use crate::features::{CenteredAabb, FeatureSimEntity};

/// Exact lifecycle ownership a possessed body had before control transfer.
/// Release restores this value rather than assuming every candidate began as a
/// room fixture.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PossessionRestoreScope {
    #[default]
    Unscoped,
    Room,
    Session(SessionScopeId),
}

/// Brain-transfer bookkeeping for possession.
///
/// `controlled == None` means the local player drives the home avatar;
/// `Some(actor)` means slot-0's brain has been transferred to `actor`. The
/// remaining fields remember what to restore on release. This resource is
/// possession-INTERNAL: no gameplay/presentation system branches on it. Ask
/// [`ControlledSubject`] instead.
#[derive(Resource, Clone, Default)]
pub struct PossessionState {
    /// The actor currently possessed (its `Brain::Player(PRIMARY)` was
    /// transferred here), or `None` while driving the home avatar.
    pub possessed: Option<Entity>,
    /// The home avatar whose player brain was vacated, restored on release.
    pub home: Option<Entity>,
    /// The possessed actor's brain before transfer, restored on release.
    pub restore_brain: Option<Brain>,
    /// Exact lifecycle scope to restore on release.
    pub restore_scope: PossessionRestoreScope,
    /// How long Down+Interact has been held toward the possess threshold.
    ///
    /// Lives HERE rather than in a `Local<f32>` on the trigger system because
    /// this resource is registered rollback state and a `Local` is not: GGRS
    /// cannot save or restore per-system state, so a rewind would rewind the
    /// possession decision while leaving the charge that produced it at its
    /// predicted value (deep review 2026-07-19 §2.4).
    pub hold_timer: f32,
    /// Previous frame's Down+Interact, for rising-edge release detection. Same
    /// reasoning as `hold_timer`: edge state must rewind with the decision.
    pub prev_down_interact: bool,
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
    // Write only on an actual change of subject: an unconditional store marks
    // the resource changed every frame, which defeats change detection for
    // every downstream consumer (the control-prompt rebuild gates on it).
    if subject.0 != chosen {
        subject.0 = chosen;
    }
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
    gravity_dir: ambition_platformer2d_core::Vec2,
    movement_mode: ambition_platformer2d_core::InputFrameMode,
) -> bool {
    ambition_platformer2d_core::AccelerationFrame::new(gravity_dir)
        .resolve_input(
            movement_mode,
            ambition_platformer2d_core::ScreenAxes::new(axis_x, axis_y),
        )
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
/// `ambition_platformer2d_runtime/tests/control_frame_lint.rs`, whose allowlist doubles as the
/// N1 multiplayer checklist. The fix is to read the acting slot's
/// `SlotInteractionState` / `SlotControls`, exactly as `interaction_input_system`
/// already does for the interact buffer — a behavior change, not a refactor, so
/// it is deferred rather than hidden.
#[allow(clippy::too_many_arguments)]
pub fn possession_trigger_system(
    control: Res<ambition_input::ControlFrame>,
    controlled: Option<Res<ambition_platformer2d_shared_tangle::markers::ControlledSubject>>,
    frames: Query<&crate::physics::ResolvedMotionFrame>,
    user_settings: Option<Res<ambition_persistence::settings::UserSettings>>,
    world_time: Res<ambition_time::WorldTime>,
    // The active session's lifetime scope. Possessing an actor PROMOTES it out of
    // room scope into this scope (see the possess handover below), so the body you
    // took over survives room transitions and can be walked anywhere — it is your
    // character now, not a fixture of the room you found it in (Jon).
    mut state: ResMut<PossessionState>,
    mut commands: Commands,
    // Home avatar kinematics: its position seeds the candidate search, and on
    // release it steps out to the vacated actor's spot (camera continuity).
    // SLOT-0 BY DESIGN: the HOME AVATAR is a real concept — the body slot 0 owns and
    // returns to on release. It is precisely the body that is NOT the controlled
    // subject while possession is active, so it cannot be found any other way.
    mut home_q: Query<
        (
            Entity,
            ambition_platformer2d_core::BodyClusterQueryData,
            &mut crate::features::MotionModel,
        ),
        crate::actor::PrimaryPlayerOnly,
    >,
    // Possession candidates: any brain-driven feature body — INCLUDING bosses.
    // Bosses are valid controllable bodies (their tick consumes `Brain::Player`),
    // so there is no `Without<BossConfig>` barrier here. Restricting WHICH boss is
    // possessable (progression/design) is a targeting-policy gate to add above
    // this, not a "bosses can never be controlled" exclusion in the body model.
    candidates: Query<
        (
            Entity,
            &CenteredAabb,
            Option<&ambition_characters::actor::BodyHealth>,
        ),
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
    target_data: Query<(
        &Brain,
        Option<&RoomScopedEntity>,
        Option<&SessionScopedEntity>,
    )>,
    // Read-only AABB lookup for the vacate exit on release.
    actor_aabbs: Query<&CenteredAabb>,
) {
    // The CONTROLLED body's resolved frame decides what "down" means for the
    // gesture — while possessing, that is the possessed body's frame.
    let gravity_dir = crate::control::controlled_frame_down(
        controlled.as_deref(),
        home_q.single().map(|(entity, _, _)| entity).ok(),
        &frames,
    );
    let movement_mode = user_settings.as_deref().map_or(
        ambition_platformer2d_core::InputFrameMode::DEFAULT_MOVEMENT,
        |s| s.gameplay.resolved_movement_frame_mode(),
    );
    let down = holding_descend(control.axis_x, control.axis_y, gravity_dir, movement_mode);
    // The gesture is a HOLD, so it accumulates on the interact button being
    // HELD — not the single-frame `interact_pressed` edge (which doors / the
    // heal-shrine also consume, resetting the hold every frame). The release is
    // the rising edge of (down + held), tracked via `prev_down_interact`.
    let down_interact = down && control.interact_held;
    let release_edge = down_interact && !state.prev_down_interact;
    state.prev_down_interact = down_interact;

    // Already possessing → a fresh Down+Interact press releases (no hold).
    if let Some(target) = state.possessed {
        state.hold_timer = 0.0;
        if release_edge {
            release_possession(&mut commands, &mut state, target, &actor_aabbs, &mut home_q);
        }
        return;
    }

    // Not possessing → accumulate the hold; commit at the threshold.
    if !down_interact {
        state.hold_timer = 0.0;
        return;
    }
    state.hold_timer += world_time.raw_dt;
    if state.hold_timer < POSSESS_HOLD_S {
        return;
    }
    state.hold_timer = 0.0;

    let Ok((home_entity, home_clusters, _)) = home_q.single() else {
        return;
    };
    let home_pos = home_clusters.kinematics.pos;
    let nearest = candidates
        .iter()
        // Structural tangibility gate (Jon 2026-07-22): a dead body is an
        // intangible corpse — you cannot possess a corpse. Excluded BEFORE
        // distance selection so a nearer corpse never shadows a farther live body.
        .filter(|(_, _, health)| !crate::combat::util::body_is_corpse(*health))
        .map(|(entity, aabb, _)| (entity, (aabb.center - home_pos).length()))
        .filter(|(_, dist)| *dist <= POSSESS_RADIUS)
        .min_by(|a, b| a.1.total_cmp(&b.1));
    let Some((target, _)) = nearest else {
        return;
    };
    let Ok((target_brain, room_scope, session_scope)) = target_data.get(target) else {
        return;
    };

    // BRAIN TRANSFER. Remember the target's brain to restore, then move the
    // player brain from the home avatar to the target. Both bodies get a fresh
    // neutral `ActorControl` so no stale edge-triggered intent (a held jump, a
    // pressed attack) leaks across the handover. The target's `ActorFaction` is
    // left untouched — effective allegiance handles its player-side combat.
    state.home = Some(home_entity);
    state.restore_brain = Some(target_brain.clone());
    state.restore_scope = if let Some(scope) = session_scope {
        PossessionRestoreScope::Session(scope.0)
    } else if room_scope.is_some() {
        PossessionRestoreScope::Room
    } else {
        PossessionRestoreScope::Unscoped
    };
    state.possessed = Some(target);

    commands
        .entity(home_entity)
        .remove::<Brain>()
        .insert(ActorControl::default());
    let mut target_cmds = commands.entity(target);
    target_cmds
        .insert(Brain::Player(PlayerSlot::PRIMARY))
        .insert(ActorControl::default())
        // Record the possession by stable id so a snapshot restores the control
        // MODE across a rewind (the home avatar is always the primary player).
        .insert(TemporaryControl::Player {
            controller: SimId::player_slot(0),
        });
    // PROMOTE the possessed body out of room scope. A room-scoped actor despawns
    // on every room load; the home avatar you drive is session-scoped precisely so
    // it survives transitions and can navigate anywhere.
    //
    // ⛔⛔ **AND THE ITEM DOMAIN DEPENDS ON THIS WITHOUT NAMING IT (2026-08-19).**
    // `items::pickup::project_custody_onto_residency` decides whether a HELD
    // object travels by asking whether its HOLDER is a `RoomResident` — so the
    // custody marker below, which suspends this body's residency, is also the
    // only reason an object in a possessed body's hand is not retired at the
    // door. Two subsystems that never reference each other agree by way of one
    // component write. `an_item_carried_by_a_possessed_body_survives_the_door_too`
    // is the guard, and it is not theoretical: it went RED when this stopped
    // swapping the lifetime and stayed red until that projection stopped asking
    // `Has<RoomScopedEntity>` as a proxy for residency. Possession makes the
    // target the body you drive, so it takes the same lifetime — otherwise it
    // would vanish the instant you carried it through an exit (or die during a
    // rollback-confirmed transition's delay, which is exactly the substitution
    // hazard GPT review #1 named). `release_possession` reverts it. The scope
    // markers are rollback-snapshot state, so this rewinds with the possession
    // decision. Absent an active session (a minimal test) it stays as it is.
    //
    // ⭐⭐ **AND IT DOES THAT BY SUSPENDING RESIDENCY, NOT BY CHANGING THE
    // LIFETIME — which is what `InCustodyOf`'s own doc has said all along:**
    // *"the LIFETIME is unchanged, and that is deliberate. The entity keeps
    // `RoomScopedEntity`; the marker is never taken away, so no query that
    // requires the scope silently loses sight of it"*, and *"`0` is whatever
    // entity took custody: a couch seat, A POSSESSED ACTOR, an NPC."*
    //
    // ⛔⛔ **this used to swap the scope instead, and that is precisely the
    // failure that doc warns about.** `project_custody_onto_authored_occurrences`
    // reads `(With<InCustodyOf>, With<RoomScopedEntity>)` — so a possessed body
    // with its room scope removed was invisible to the occurrence ledger, its
    // home room was never told the occurrence was in somebody's hands, and
    // re-entering that room AUTHORED A SECOND COPY behind the same
    // `SimId::placement(..)`. Measured, not reasoned:
    // `an_authored_actor_carried_out_of_its_room_and_back_does_not_meet_a_copy`
    // counted two.
    //
    // ⭐ the retirement half is unchanged and needs no promotion, because
    // `RoomResident` is `(With<RoomScopedEntity>, Without<InCustodyOf>)` — the
    // custody marker already excludes this body from the sweep a room change
    // runs. One mechanism instead of two, and the ledger joins for free.
    //
    // ⚠ `restore_scope` below still records and restores the exact scope. It is
    // a no-op on this path now (nothing is removed), and it is kept because it
    // is part of a rollback-registered resource: retiring the field is a schema
    // change and belongs to a bump, not to a bug fix.
    //
    // ⛔⛔ **AND THE MARKER IS NOT WRITTEN HERE, because it is DERIVED.**
    // `InCustodyOf` is declared to rollback as
    // *"room residency reprojected from `ItemCustody` every tick"* — a
    // justification for not snapshotting it that is only true while something
    // reprojects it. A possessed body has no `ItemCustody`, so inserting the
    // marker at this site would create the one population nothing re-derives:
    // a rewind past the possession would drop it and never put it back, and the
    // driven body would become a `RoomResident` again and be retired at the next
    // door. [`project_possession_onto_custody`] is the deriver, reading
    // `PossessionState` — which IS rollback state — so the declaration's
    // justification stays true for both populations.
    // ⭐ the `ActiveSessionScope` parameter is GONE with the promotion it served:
    // its scope id was needed to mint `SessionScopedEntity(scope_id)`, and
    // nothing here mints a scope any more. A `let _ = &param` suppression would
    // have kept an authority this system no longer consults, which is exactly
    // what makes a later reader think it matters.
}

/// Restore the home avatar's player brain and the target's authored brain plus
/// exact pre-possession lifecycle scope, then step the home body out to the
/// vacated actor's position so the camera does not snap back.
fn release_possession(
    commands: &mut Commands,
    state: &mut PossessionState,
    target: Entity,
    actor_aabbs: &Query<&CenteredAabb>,
    // SLOT-0 BY DESIGN: the home avatar (see `possession_trigger_system`).
    home_q: &mut Query<
        (
            Entity,
            ambition_platformer2d_core::BodyClusterQueryData,
            &mut crate::features::MotionModel,
        ),
        crate::actor::PrimaryPlayerOnly,
    >,
) {
    state.possessed = None;

    // Restore the actor's authored brain, clearing stale edges. Its faction was
    // never touched (effective allegiance), so there is nothing to restore. The
    // cached `restore_brain` is kept in sync with the actor's autonomous SOURCE
    // (refreshed by `BrainCommand` if it switched during possession), so releasing
    // resumes the CURRENT selected source. Its temporary-control record returns to
    // `Autonomous`.
    if let Some(brain) = state.restore_brain.take() {
        if let Ok(mut ec) = commands.get_entity(target) {
            // Taken INSIDE the guard: the recorded scope is only consumed on the
            // path that actually restores it. Taking it above would, on any
            // inconsistent state where the brain is already gone, discard the
            // record while leaving the body wearing the possession-time session
            // scope forever.
            // ⭐⭐ **RELEASE TOUCHES NO SCOPE, because possession touched none.**
            // Dropping the custody marker is the whole of it, and residency
            // resumes in whatever room is active NOW — the marker's own
            // contract: `RoomScopedEntity` carries no room id, so a body
            // released two rooms later is resident THERE and the next transition
            // out retires it correctly. Nothing has to remember where it came
            // from.
            //
            // ⛔ **and removing-then-restoring the scope would now be a BUG, not
            // a no-op.** It used to be the point; today nothing has changed the
            // scope during possession, so a remove/re-insert pair would silently
            // revert any scope write some OTHER system made while the body was
            // being driven.
            //
            // ⚠ `restore_scope` is still recorded above and is now vestigial. It
            // is a field of a rollback-registered resource, so retiring it is a
            // schema change and belongs to a bump rather than to a bug fix; it is
            // read nowhere.
            let _ = &state.restore_scope;
            ec.insert(brain)
                .insert(ActorControl::default())
                .insert(TemporaryControl::Autonomous);
        }
    }

    // Restore the home avatar's player brain and vacate-exit to the actor's spot.
    if let Some(home) = state.home.take() {
        if let Ok(mut ec) = commands.get_entity(home) {
            ec.insert(Brain::Player(PlayerSlot::PRIMARY))
                .insert(ActorControl::default());
        }
        if let (Ok(aabb), Ok((_, mut cluster_item, mut motion_model))) =
            (actor_aabbs.get(target), home_q.get_mut(home))
        {
            // THE discrete-transit authority: the vacate-exit is a scripted
            // teleport arriving at rest (ADR 0024 authority model).
            let mut clusters = cluster_item.as_clusters_mut();
            ambition_platformer2d_core::movement::transit_body(
                &mut motion_model,
                &mut clusters,
                aabb.center,
                ambition_platformer2d_core::movement::TransitVelocity::Zero,
            );
        }
    }
}

/// **A DRIVEN BODY IS IN THE PARTICIPANT'S CUSTODY, and that marker is a
/// PROJECTION of [`PossessionState`] rather than a write at the possess site.**
///
/// ⭐ **possession is custody of a BODY**, so it uses the same vocabulary a
/// carried object does — `InCustodyOf`, whose own doc says *"the LIFETIME is
/// unchanged, and that is deliberate"* and names *"a possessed actor"* among the
/// custodians. Possession used to express the same fact by SWAPPING the body's
/// lifetime (room scope out, session scope in), which hid it from every query
/// that requires the room scope — including the occurrence ledger's, so a home
/// room re-authored a SECOND copy of the body being driven.
///
/// ⛔⛔ **IT IS A DERIVE AND NOT A FOLLOW-UP CALL, for a rollback reason.**
/// `InCustodyOf` is registered as a DERIVED component on the strength of one
/// sentence — *"room residency reprojected from `ItemCustody` every tick"* — and
/// that sentence is what excuses it from the snapshot. A possessed body has no
/// `ItemCustody`; writing the marker at the possess site would create a
/// population nothing reprojects, and a rewind past the possession would drop it
/// with nothing to put it back. Reading `PossessionState`, which IS rollback
/// state, keeps the excuse true for both populations.
///
/// ⚠ **the retraction arm is scoped by `Without<GroundItem>`**, because the item
/// domain owns the marker on objects and reprojects it from its own authority.
/// A blanket retraction here would fight
/// [`project_custody_onto_residency`](crate::items::pickup::project_custody_onto_residency)
/// every tick.
///
/// ⚠ **compared before writing**, like its item sibling: an unconditional insert
/// would mark the component changed on every tick of a possession, and change
/// ticks do not rewind.
pub fn project_possession_onto_custody(
    mut commands: Commands,
    state: Res<PossessionState>,
    driven: Query<
        (
            Entity,
            &ambition_platformer2d_shared_tangle::lifecycle::InCustodyOf,
        ),
        Without<crate::items::pickup::GroundItem>,
    >,
) {
    use ambition_platformer2d_shared_tangle::lifecycle::InCustodyOf;
    let wanted = state.possessed.zip(state.home);
    for (entity, custody) in &driven {
        let agrees =
            matches!(wanted, Some((possessed, home)) if possessed == entity && custody.0 == home);
        if !agrees {
            commands.entity(entity).remove::<InCustodyOf>();
        }
    }
    if let Some((possessed, home)) = wanted {
        if driven.get(possessed).map(|(_, custody)| custody.0) != Ok(home) {
            commands.entity(possessed).try_insert(InCustodyOf(home));
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
    state.restore_scope = PossessionRestoreScope::Unscoped;
}

#[cfg(test)]
mod tests;

impl bevy::ecs::entity::MapEntities for PossessionState {
    fn map_entities<M: bevy::ecs::entity::EntityMapper>(&mut self, mapper: &mut M) {
        if let Some(entity) = self.possessed.as_mut() {
            *entity = mapper.get_mapped(*entity);
        }
        if let Some(entity) = self.home.as_mut() {
            *entity = mapper.get_mapped(*entity);
        }
    }
}
