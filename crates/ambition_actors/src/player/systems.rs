//! Player ECS systems.

use bevy::prelude::*;

use super::components::{LocalPlayer, PlayerEntity, PlayerInputFrame, PlayerSlot, PrimaryPlayer};
use super::events::PlayerHealRequested;
use super::movement_components::{BodyGroundState, BodyKinematics};
use crate::actor::BodyMelee;
use crate::features::ActorPose;
use ambition_characters::actor::{BodyCombat, BodyHealth};
use ambition_characters::brain::{ActorControl, Brain, BrainSnapshot, SlotControls};
use ambition_engine_core as ae;
use ambition_input::ControlFrame;

/// Publish the local device's finalized [`ControlFrame`] into the slot-based
/// controller model as [`PlayerSlot::PRIMARY`]. This is the ONE place local
/// input enters the canonical [`SlotControls`] resource; every controlled body
/// reads its slot's frame from there via `Brain::Player`. Co-op / netcode add
/// their own writers for higher slots without touching this one.
pub fn populate_slot_controls(frame: Res<ControlFrame>, mut slots: ResMut<SlotControls>) {
    slots.set(PlayerSlot::PRIMARY, *frame);
}

/// Mirror a controlled body's slot frame onto its [`PlayerInputFrame`] component.
///
/// `PlayerInputFrame` is the per-body view of "the input THIS body is receiving
/// as a controlled body" — read by player-specific ability systems (held-item
/// use, heal shrine, portal gun). It is sourced from [`SlotControls`] and gated
/// on brain ownership: a body only receives its slot's frame while it carries
/// `Brain::Player(slot)`. A vacated home avatar (its player brain transferred to
/// a possessed actor) therefore sees NEUTRAL input and has no local attack
/// authority — the mandate's "the vacated body must not act", derived from the
/// brain rather than a possession run-condition.
pub fn sync_local_player_input_frame(
    slots: Res<SlotControls>,
    mut players: Query<(&mut PlayerInputFrame, Option<&Brain>), With<LocalPlayer>>,
) {
    for (mut player_input, brain) in &mut players {
        player_input.frame = match brain.and_then(Brain::player_slot) {
            Some(slot) => slots.get(slot),
            // No player brain (vacated during possession): no local control.
            None => ControlFrame::default(),
        };
    }
}

/// Mirror authoritative player body state into the generic gameplay
/// [`ActorPose`] used by the brain/action resolver.
///
/// The player, NPCs, enemies, and bosses should all expose action origins
/// through gameplay pose data rather than presentation `Transform`s.
pub fn sync_player_actor_poses(
    mut players: Query<(&BodyKinematics, &mut ActorPose), With<PlayerEntity>>,
) {
    for (kin, mut pose) in &mut players {
        *pose = ActorPose::from_parts(kin.pos, kin.size * 0.5, kin.facing);
    }
}

/// Translate each controlled home body's slot frame into its `ActorControl`
/// frame.
///
/// This is the producer for the universal-brain seam on the home/player side —
/// the direct analogue of the `Brain::Player` branch in `tick_actor_brains`.
/// The INPUT AUTHORITY is [`SlotControls`] read by the body's own
/// `Brain::Player(slot)`, NOT `PlayerInputFrame`: the home body is controlled
/// because it carries the player brain for that slot, exactly like a possessed
/// actor. `PlayerInputFrame` is now only a compatibility mirror for player-
/// flavoured ability/UI systems (held item, heal shrine, portal gun) written by
/// `sync_local_player_input_frame`; gameplay brain input no longer depends on it.
///
/// The query requires `&mut Brain`, so a vacated home avatar (its player brain
/// transferred to a possessed actor by `possession`) carries no `Brain` and is
/// skipped — it stays inert with a neutral `ActorControl`. Iterates every home
/// body carrying a player brain; multi-player ready even though only one slot
/// exists today.
pub fn tick_player_brains(
    gravity_field: Option<Res<crate::physics::GravityField>>,
    user_settings: Option<Res<ambition_persistence::settings::UserSettings>>,
    slots: Res<SlotControls>,
    mut players: Query<(
        &BodyKinematics,
        &BodyGroundState,
        &mut Brain,
        &mut ActorControl,
    )>,
) {
    let control_down = crate::physics::gravity_dir_or_default(gravity_field.as_deref());
    let control_frame_modes = user_settings
        .as_deref()
        .map_or(ae::ControlFrameModes::default(), |s| {
            s.gameplay.control_frame_modes()
        });

    for (kin, ground, mut brain, mut control) in &mut players {
        // INPUT AUTHORITY: this body's OWN slot frame, keyed by the brain it
        // carries — the SAME `Brain::Player(slot)` → `SlotControls` path a
        // possessed actor reads. A body whose brain isn't a player brain is
        // skipped (its `ActorControl` is owned by an AI tick, not this one).
        let Some(slot) = brain.player_slot() else {
            continue;
        };
        let input = slots.get(slot);
        // Build the snapshot from the player's cluster components plus
        // the per-tick slot frame. The input is what makes
        // Brain::Player's translation deterministic: same input +
        // same body snapshot → same ActorControlFrame.
        let snapshot = BrainSnapshot {
            actor_pos: kin.pos,
            actor_vel: kin.vel,
            actor_facing: kin.facing,
            control_down,
            movement_frame_mode: control_frame_modes.movement,
            aim_frame_mode: control_frame_modes.aim,
            actor_on_ground: ground.on_ground,
            // The player brain reads input, not the Smash aerial path; grounded
            // locomotion semantics regardless of fly mode.
            actor_aerial: false,
            alive: true,
            target_pos: kin.pos,
            target_alive: true,
            // The player brain doesn't regroup on damage; full-health is inert here.
            health_fraction: 1.0,
            sim_time: 0.0,
            dt: 0.0,
            // Player brain emits an already-normalized stick; capability is
            // applied on the player integration side, so this is don't-care here.
            max_run_speed: 0.0,
            attack_cooldown_remaining: 0.0,
            attack_windup_remaining: 0.0,
            attack_active_remaining: 0.0,
            attack_recover_remaining: 0.0,
            stun_remaining: 0.0,
            wall_contact: None,
            // BossPattern-only inputs — inert for the player body.
            boss_encounter_phase: None,
            world_size: ae::Vec2::ZERO,
            front_wall_clearance: None,
            player_input: Some(input),
            // Player brain doesn't consult these fields; leave them
            // None so the snapshot builder doesn't pay for queries
            // the brain ignores.
            crowding: None,
            terrain: None,
            // Player brain reads its own air-jump state via the
            // PlayerInputFrame / engine path, not via the snapshot.
            air_jumps_remaining: 0,
        };
        let mut frame = ambition_characters::actor::control::ActorControlFrame::neutral();
        brain.tick(&snapshot, &mut frame);
        control.0 = frame;
    }
}

/// Write the player's read-model fields on [`BodyCombat`] each frame — the
/// symmetric counterpart to the actor's `sync_actor_components_from_cluster`.
///
/// - `attacking` mirrors `BodyMelee::is_swinging()` (a swing in flight, any phase).
/// - `alive` mirrors the body's liveness AUTHORITY, `BodyHealth`. For actors this
///   field is owned by the per-frame sync from their cluster `status.alive`; the
///   player has no such cluster, so without this it kept its spawn default
///   (`false`) forever — a silent "the player is dead" that made every enemy's
///   `target_alive` read false and idle their brain. Owning it here keeps the
///   field correct for every `BodyCombat` reader (HUD / nameplate / health bar /
///   perception / damage gates), so none of them are a footgun for the player.
///   (Liveness-CRITICAL gameplay should still read `BodyHealth` directly — the
///   authority — rather than this once-per-frame mirror, to avoid a tick of lag.)
pub fn write_player_ecs_components(
    mut players: Query<(&BodyMelee, &BodyHealth, &mut BodyCombat), With<PlayerEntity>>,
) {
    for (attack, health, mut combat) in &mut players {
        combat.attacking = attack.is_swinging();
        combat.alive = health.current() > 0;
    }
}

/// Apply heal messages to the authoritative `BodyHealth` ECS component.
///
/// A heal targets either a specific player entity (`heal.target ==
/// Some(entity)`) or the primary player as a fallback (`None`). The
/// fallback path keeps existing call sites — cutscene heals, dev-tool
/// heals — working with no change. Per-player producers like pickup
/// collection should set the target explicitly so a non-primary
/// player who walked into the heart actually gets healed.
pub fn apply_player_heal_requests(
    mut heals: MessageReader<PlayerHealRequested>,
    mut players: Query<&mut BodyHealth, With<PlayerEntity>>,
    primary_q: Query<Entity, (With<PlayerEntity>, With<PrimaryPlayer>)>,
) {
    let primary = primary_q.single().ok();
    for heal in heals.read() {
        if heal.amount <= 0 {
            continue;
        }
        let target = heal.target.or(primary);
        let Some(target) = target else {
            // No player entity yet (startup or headless): drop the
            // heal silently so the queue still drains.
            continue;
        };
        if let Ok(mut health) = players.get_mut(target) {
            health.heal(heal.amount);
        }
    }
}

/// Mana regenerated per second (clamped to the meter max).
const MANA_REGEN_PER_SEC: f32 = 14.0;

/// Mana slowly regenerates so it's a genuine spendable resource. Uses
/// `ResourceMeter::refill` (clamped) rather than the meter's own `regen_rate`
/// field so we don't change `BodyMana::default` (and any test that relies on
/// it). Scaled by sim dt, so bullet-time / pause slow it with the world.
///
/// Refills the *controlled subject's* mana — the body actually spending it on
/// charge attacks / the fireball — so possessing an actor regenerates that
/// actor's meter, not the vacated home avatar's. (Moved from the render HUD
/// module, E4: a sim mutator never lives in presentation.)
pub fn regen_player_mana(
    time: Res<ambition_time::WorldTime>,
    controlled: Option<Res<ambition_platformer_primitives::markers::ControlledSubject>>,
    mut manas: Query<&mut crate::actor::BodyMana>,
    primary: Query<Entity, (With<PlayerEntity>, With<PrimaryPlayer>)>,
) {
    let dt = time.sim_dt();
    if dt <= 0.0 {
        return;
    }
    let Some(subject) = controlled
        .as_deref()
        .and_then(|subject| subject.0)
        .or_else(|| primary.single().ok())
    else {
        return;
    };
    if let Ok(mut mana) = manas.get_mut(subject) {
        mana.meter.refill(MANA_REGEN_PER_SEC * dt);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_characters::brain::ActorControl;

    #[test]
    fn mana_regenerates_over_time_but_clamps_to_max() {
        let mut app = App::new();
        app.insert_resource(ambition_time::WorldTime {
            raw_dt: 1.0,
            scaled_dt: 1.0,
        });
        app.add_systems(Update, regen_player_mana);
        let player = app
            .world_mut()
            .spawn((
                PlayerEntity,
                PrimaryPlayer,
                crate::actor::BodyMana::default(),
            ))
            .id();
        // Drain it, then let it tick back up.
        app.world_mut()
            .get_mut::<crate::actor::BodyMana>(player)
            .unwrap()
            .meter
            .try_spend(60.0);
        let before = app
            .world()
            .get::<crate::actor::BodyMana>(player)
            .unwrap()
            .meter
            .current;
        app.update();
        let after = app
            .world()
            .get::<crate::actor::BodyMana>(player)
            .unwrap()
            .meter
            .current;
        assert!(
            after > before,
            "mana should regenerate ({before} -> {after})"
        );

        // Many ticks can't exceed max.
        for _ in 0..20 {
            app.update();
        }
        let m = app
            .world()
            .get::<crate::actor::BodyMana>(player)
            .unwrap()
            .meter;
        assert!(m.current <= m.max + 1e-3, "mana clamps to max");
    }

    #[test]
    fn wallet_add_clamps_and_spend_respects_balance() {
        let mut wallet = ambition_characters::actor::BodyWallet::default();
        assert_eq!(wallet.balance, 0);
        wallet.add(50);
        wallet.add(-100); // can't drive below zero
        assert_eq!(wallet.balance, 0);
        wallet.add(30);
        assert!(wallet.try_spend(20));
        assert_eq!(wallet.balance, 10);
        assert!(!wallet.try_spend(99), "can't overspend");
        assert_eq!(wallet.balance, 10);
    }

    /// Default player ActionSet derives from AbilitySet — when
    /// `attack` is on, the ActionSet has a Swipe melee; when off,
    /// melee is None and the resolver emits nothing for melee
    /// presses. Pins the ability-gated capability invariant.
    #[test]
    fn player_action_set_melee_disabled_when_attack_ability_off() {
        use ambition_characters::brain::ActionSet;
        let mut player = crate::player::primary_player_scratch(
            ae::Vec2::new(0.0, 0.0),
            ae::AbilitySet::sandbox_all(),
        );
        // Force-disable the attack ability.
        player.abilities.abilities.attack = false;
        let bundle = crate::player::PlayerSimulationBundle::from_scratch(
            player,
            ambition_characters::actor::Health::new(10),
        );
        // ActionSet on the bundle reflects the disabled ability.
        let action_set: &ActionSet = &bundle.action_set;
        assert!(
            action_set.melee.is_none(),
            "ActionSet.melee should be None when AbilitySet.attack is off"
        );
    }

    /// Similarly: with shield ability off, special slot is None.
    /// Pins the same gating discipline for special-ability slots.
    #[test]
    fn player_action_set_special_disabled_when_shield_ability_off() {
        use ambition_characters::brain::ActionSet;
        let mut player = crate::player::primary_player_scratch(
            ae::Vec2::new(0.0, 0.0),
            ae::AbilitySet::sandbox_all(),
        );
        player.abilities.abilities.shield = false;
        let bundle = crate::player::PlayerSimulationBundle::from_scratch(
            player,
            ambition_characters::actor::Health::new(10),
        );
        let action_set: &ActionSet = &bundle.action_set;
        assert!(
            action_set.special.is_none(),
            "ActionSet.special should be None when AbilitySet.shield is off"
        );
    }

    /// Default player ActionSet has a Swipe melee + Bolt ranged +
    /// `bubble_shield` special when the player has all abilities. Pins
    /// the sandbox_all() default — EFFECTS consumers
    /// can rely on these slots being filled.
    #[test]
    fn player_action_set_has_full_moveset_with_sandbox_all_abilities() {
        use ambition_characters::brain::{
            ActionSet, MeleeActionSpec, RangedActionSpec, SpecialActionSpec,
        };
        let player = crate::player::primary_player_scratch(
            ae::Vec2::new(0.0, 0.0),
            ae::AbilitySet::sandbox_all(),
        );
        let bundle = crate::player::PlayerSimulationBundle::from_scratch(
            player,
            ambition_characters::actor::Health::new(10),
        );
        let action_set: &ActionSet = &bundle.action_set;
        assert!(matches!(action_set.melee, Some(MeleeActionSpec::Swipe(_))));
        assert!(matches!(
            action_set.ranged,
            Some(RangedActionSpec::Bolt { .. })
        ));
        assert!(matches!(
            action_set.special,
            Some(SpecialActionSpec::Special(ref key)) if key == "bubble_shield"
        ));
    }

    /// End-to-end: player releases the projectile charge →
    /// tick_player_brains fills frame.fire → resolver emits a
    /// Ranged action message with the player's Bolt spec. Pins
    /// the ranged side of the seam alongside the melee test below.
    #[test]
    fn player_projectile_release_emits_ranged_bolt_action_message_end_to_end() {
        use ambition_characters::brain::{
            emit_brain_action_messages, ActionRequest, ActorActionMessage, RangedActionSpec,
        };
        use bevy::transform::components::Transform;
        let mut app = App::new();
        app.init_resource::<ControlFrame>();
        app.init_resource::<SlotControls>();
        app.add_message::<ActorActionMessage>();
        let mut player = crate::player::primary_player_scratch(
            ae::Vec2::new(40.0, 60.0),
            ae::AbilitySet::sandbox_all(),
        );
        ae::refresh_movement_resources_clusters(
            &player.abilities,
            &mut player.dash,
            &mut player.jump,
            ae::DEFAULT_TUNING,
        );
        let bundle = crate::player::PlayerSimulationBundle::from_scratch(
            player,
            ambition_characters::actor::Health::new(10),
        );
        app.world_mut()
            .spawn((bundle, Transform::from_xyz(40.0, 60.0, 0.0)));
        app.add_systems(
            Update,
            (
                populate_slot_controls,
                sync_local_player_input_frame,
                tick_player_brains,
                emit_brain_action_messages,
            )
                .chain(),
        );
        {
            let mut cf = app.world_mut().resource_mut::<ControlFrame>();
            cf.projectile_released = true;
            // aim diagonally up-right; brain reads aim when present
            cf.aim_x = 0.8;
            cf.aim_y = -0.6;
        }
        app.update();
        let mut messages = app
            .world_mut()
            .resource_mut::<bevy::ecs::message::Messages<ActorActionMessage>>();
        let received: Vec<_> = messages.drain().collect();
        let ranged: Vec<_> = received
            .into_iter()
            .filter(|m| matches!(m.request, ActionRequest::Ranged { .. }))
            .collect();
        assert_eq!(ranged.len(), 1, "expected exactly one Ranged message");
        match ranged[0].request.clone() {
            ActionRequest::Ranged {
                spec: RangedActionSpec::Bolt { speed, .. },
                dir,
                dir_policy,
                ..
            } => {
                assert!(speed > 0.0, "Bolt has positive speed");
                // dir is the controlled-body-local aim vector normalized.
                assert!(dir.x > 0.0 && dir.y < 0.0, "aim diagonally up-right");
                assert_eq!(dir_policy, ae::GameplayFramePolicy::ControlledBodyLocal);
            }
            other => panic!("expected Ranged::Bolt, got {:?}", other),
        }
    }

    /// End-to-end: player presses attack → tick_player_brains fills
    /// ActorControl → emit_brain_action_messages produces an
    /// ActorActionMessage with a Swipe request. Pins the full
    /// player-side universal-brain seam from input to resolved
    /// concrete action.
    #[test]
    fn player_attack_press_emits_swipe_action_message_end_to_end() {
        use ambition_characters::brain::{
            emit_brain_action_messages, ActionRequest, ActorActionMessage, MeleeActionSpec,
        };
        use bevy::transform::components::Transform;
        let mut app = App::new();
        app.init_resource::<ControlFrame>();
        app.init_resource::<SlotControls>();
        app.add_message::<ActorActionMessage>();
        let mut player = crate::player::primary_player_scratch(
            ae::Vec2::new(40.0, 60.0),
            ae::AbilitySet::sandbox_all(),
        );
        ae::refresh_movement_resources_clusters(
            &player.abilities,
            &mut player.dash,
            &mut player.jump,
            ae::DEFAULT_TUNING,
        );
        // Use the canonical bundle so the player's ActionSet is the
        // production default (Swipe melee + Bolt ranged). Bundle
        // already includes a PlayerBody synced off the authority.
        let bundle = crate::player::PlayerSimulationBundle::from_scratch(
            player,
            ambition_characters::actor::Health::new(10),
        );
        app.world_mut()
            .spawn((bundle, Transform::from_xyz(40.0, 60.0, 0.0)));
        app.add_systems(
            Update,
            (
                populate_slot_controls,
                sync_local_player_input_frame,
                tick_player_brains,
                emit_brain_action_messages,
            )
                .chain(),
        );

        {
            let mut cf = app.world_mut().resource_mut::<ControlFrame>();
            cf.attack_pressed = true;
            cf.axis_x = 1.0;
        }
        app.update();
        let mut messages = app
            .world_mut()
            .resource_mut::<bevy::ecs::message::Messages<ActorActionMessage>>();
        let received: Vec<_> = messages.drain().collect();
        assert_eq!(received.len(), 1, "expected one Swipe message");
        match received[0].request.clone() {
            ActionRequest::Melee {
                spec: MeleeActionSpec::Swipe(_),
                facing,
                origin,
                ..
            } => {
                assert!(facing > 0.0, "facing should be right (+1)");
                assert_eq!(origin, ae::Vec2::new(40.0, 60.0));
            }
            other => panic!("expected Melee::Swipe, got {:?}", other),
        }
    }

    /// End-to-end: spawn a player entity with the brain components,
    /// populate ControlFrame, run sync_local_player_input_frame +
    /// tick_player_brains, assert ActorControl reflects the input.
    /// Pins the universal-brain seam on the player side.
    #[test]
    fn player_brain_seam_translates_control_frame_to_actor_control() {
        let mut app = App::new();
        app.init_resource::<ControlFrame>();
        app.init_resource::<SlotControls>();
        let mut player = crate::player::primary_player_scratch(
            ae::Vec2::new(100.0, 100.0),
            ae::AbilitySet::sandbox_all(),
        );
        ae::refresh_movement_resources_clusters(
            &player.abilities,
            &mut player.dash,
            &mut player.jump,
            ae::DEFAULT_TUNING,
        );
        // `PlayerSimulationBundle` carries the same cluster components
        // that `PlayerMovementAuthority` + `PlayerBody` used to be
        // synthesized from. `Brain` / `ActorControl` are bundle fields
        // too, so no extra spawn-tuple state is needed.
        let bundle = crate::player::PlayerSimulationBundle::from_scratch(
            player,
            ambition_characters::actor::Health::new(10),
        );
        app.world_mut().spawn(bundle);
        app.add_systems(
            Update,
            (
                populate_slot_controls,
                sync_local_player_input_frame,
                tick_player_brains,
            )
                .chain(),
        );

        // Stamp the control frame with a known input.
        {
            let mut cf = app.world_mut().resource_mut::<ControlFrame>();
            cf.axis_x = 1.0;
            cf.jump_pressed = true;
            cf.attack_pressed = true;
            cf.shield_held = true;
        }
        app.update();

        let mut q = app
            .world_mut()
            .query_filtered::<&ActorControl, With<PlayerEntity>>();
        let control = q
            .iter(app.world())
            .next()
            .expect("player entity should have ActorControl");
        // axis_x → desired_vel.x, jump_pressed → jump_pressed, etc.
        assert_eq!(control.0.locomotion.x, 1.0);
        assert!(control.0.jump_pressed);
        assert!(control.0.melee_pressed);
        assert!(control.0.shield_held);
        assert_eq!(control.0.facing, 1.0);
    }
}
