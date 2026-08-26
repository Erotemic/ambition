//! This crate's causal facts.
//!
//! Why did this body move this tick? — the inspector's first required
//! question, answered for a seated body.
//!
//! ## Observer by construction, not by discipline
//!
//! [`record_player_movement_intent`] takes every component immutably and holds
//! no mutable handle to anything but the log. It CANNOT affect the simulation,
//! and that is a property of its signature rather than a promise in a comment —
//! which matters here more than usual, because a rollback host resimulates and
//! an instrument that nudged state would desync exactly when it was being used.
//!
//! It runs AFTER the brain tick rather than inside it for the same reason: the
//! alternative was threading a recorder through `tick_controlled_brains`, and a
//! system that only reads cannot be the thing that broke the tick.
//!
//! ## The subject is the SEAT
//!
//! Not an `Entity` — indices are recycled and `to_bits` ordering is a trap this
//! repo has already been bitten by — and not a `SimId`, which bodies do not
//! carry. A seat is stable across death and respawn, which is precisely the
//! window an investigation spans: "why did seat 1 walk off the stage" survives
//! the three respawns in the middle of the answer.

use ambition_causal::{domains, CausalFact, CausalRecording, FactDetail, SubjectKey};
use bevy::prelude::*;

use crate::avatar::movement_components::{BodyGroundState, BodyKinematics};
use ambition_damage::{BodyHitResolution, BodyHitResolved, BodyReactionApplied};
use ambition_characters::control::{ActorControl};
use ambition_characters::control::{DrivingParticipant};

/// Publish one movement-intent fact per seated body per tick.
///
/// Records the intent the brain EMITTED alongside the body state it was emitted
/// from, which is what makes the fact answer "why" rather than "what": a body
/// that did not move because its brain asked for nothing is a different finding
/// from one that asked and was refused, and the two are indistinguishable from
/// a position sample.
pub fn record_player_movement_intent(
    log: Option<ResMut<CausalRecording>>,
    bodies: Query<(
        &BodyKinematics,
        &BodyGroundState,
        &DrivingParticipant,
        &ActorControl,
    )>,
) {
    let Some(mut log) = log else {
        return;
    };
    if !log.is_recording() {
        return;
    }
    for (kin, ground, driver, control) in &bodies {
        // Only seated bodies: the seat IS the identity, so a body without one
        // has nothing an explanation could be keyed on. Publishing it under a
        // recycled entity index would be worse than not publishing it. The
        // `&DrivingParticipant` in the query IS that filter.
        let slot = driver.0;
        let frame = &control.0;
        log.record(
            CausalFact::new(
                domains::MOVEMENT,
                0,
                FactDetail::new(
                    "movement_intent",
                    format!(
                        "seat {} asked for lateral {:+.2}{}",
                        slot.0,
                        frame.locomotion.x,
                        if frame.jump_pressed {
                            " and a jump"
                        } else {
                            ""
                        }
                    ),
                ),
            )
            .about(SubjectKey::Seat(slot.0))
            .by_participant(slot.0)
            .field("locomotion_x", frame.locomotion.x)
            .field("locomotion_y", frame.locomotion.y)
            .field("jump_pressed", frame.jump_pressed)
            .field("jump_held", frame.jump_held)
            .field("pos_x", kin.pos.x)
            .field("pos_y", kin.pos.y)
            .field("vel_x", kin.vel.x)
            .field("vel_y", kin.vel.y)
            .field("on_ground", ground.on_ground)
            .field("facing", kin.facing),
        );
    }
}

/// Movement operations emitted by the kernel for optional causal instrumentation.
///
/// The core movement crate does not depend on `ambition_causal`, so the host forwards
/// `FrameEvents::operations` through this message. Compositions without a causal
/// recorder may omit the writer entirely.
#[cfg(feature = "causal")]
#[derive(bevy::prelude::Message, Clone, Debug)]
pub struct BodyMovementOps {
    pub body: bevy::prelude::Entity,
    pub ops: Vec<ambition_platformer2d_core::MovementOp>,
}

/// Turn the kernel's operation list into facts the explainer can join.
#[cfg(feature = "causal")]
pub fn record_movement_operations(
    log: Option<ResMut<CausalRecording>>,
    mut ops: bevy::prelude::MessageReader<BodyMovementOps>,
    identities: Query<&ambition_combat::components::ActorIdentity>,
    tick: Option<Res<ambition_time::SimTick>>,
) {
    let Some(mut log) = log else {
        ops.clear();
        return;
    };
    if !log.is_recording() {
        ops.clear();
        return;
    }
    for applied in ops.read() {
        let Ok(identity) = identities.get(applied.body) else {
            continue;
        };
        for op in &applied.ops {
            log.record(
                CausalFact::new(
                    domains::MOVEMENT,
                    tick.as_deref().map_or(0, |t| t.get()),
                    FactDetail::new(
                        "movement_operation",
                        format!("the movement kernel performed {op:?}"),
                    ),
                )
                .about(SubjectKey::Sim(identity.id.clone()))
                .field("operation", format!("{op:?}")),
            );
        }
    }
}

pub fn record_body_control_frame(
    log: Option<ResMut<CausalRecording>>,
    bodies: Query<(
        // The READ-MODEL, not `ActorConfig`. `sync_actor_read_model` copies
        // `config.id` into it verbatim, so the join with the brain's fact holds
        // — and an observer reading the read-model instead of the authored
        // cluster is the same discipline the render side follows.
        &ambition_combat::components::ActorIdentity,
        &BodyKinematics,
        &BodyGroundState,
        &ActorControl,
        &ambition_platformer2d_core::BodyDashState,
        Option<&DrivingParticipant>,
        // A plain two-CPU match shows seven consecutive ticks pinned at exactly `-86` while the
        // brain asks `+0.65` — neither obeying nor decelerating, so it is not a turnaround. The
        // three candidates all live here, and without them the log can show that the body disagreed
        // and never why.
        //
        // `Option`, because a body without a combat cluster is a legal body
        // and must not vanish from the log for lacking one.
        Option<&ambition_characters::actor::BodyCombat>,
        // AC3.1.B: the melee AUTHORITY.
        Option<&crate::actor::BodyMelee>,
        // THE INTEGRATOR'S OWN INPUTS, added after six candidates were
        // eliminated one at a time and the cause was still not found (S51). The
        // unauthored steps are a near-constant `-99`/tick, which is an
        // ACCELERATION, and `integration.rs` adds exactly two:
        // `gravity_acceleration` and `external_acceleration`. Printing both
        // turns "which term produced -5940 px/s²?" from a search into a read.
        //
        // `Option` for a body without a resolved frame (bare test bodies).
        Option<&ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame>,
    )>,
) {
    let Some(mut log) = log else {
        return;
    };
    if !log.is_recording() {
        return;
    }
    for (identity, kin, ground, control, dash, driver, combat, melee, motion_frame) in &bodies {
        // A seated body is already covered by `record_player_movement_intent`,
        // under its SEAT — which is the better key there, because a seat
        // survives death and respawn and an actor id does not.
        if driver.is_some() {
            continue;
        }
        let frame = &control.0;
        log.record(
            CausalFact::new(
                domains::MOVEMENT,
                0,
                FactDetail::new(
                    "control_frame_received",
                    format!(
                        "body holds lateral {:+.2} while moving {:+.0}/s",
                        frame.locomotion.x, kin.vel.x
                    ),
                ),
            )
            .about(SubjectKey::Sim(identity.id.clone()))
            // The pair the whole instrument exists for: what was asked, and
            // which way the body is actually going. A sign disagreement between
            // these two is the finding, and nothing else in the log shows it.
            .field("locomotion_x", frame.locomotion.x)
            .field("vel_x", kin.vel.x)
            .field("locomotion_y", frame.locomotion.y)
            .field("vel_y", kin.vel.y)
            .field("pos_x", kin.pos.x)
            .field("pos_y", kin.pos.y)
            .field("on_ground", ground.on_ground)
            .field("facing", kin.facing)
            .field("burst_pressed", frame.burst_pressed)
            // the dash state is here for a NAMED suspicion: the trace showed
            // the body reaching 750/s, which is dash speed (a run caps at 270),
            // while `Dash` appeared zero times in the brain's decisions. A dash
            // armed by something other than the decision shows up as a spent
            // charge and a live cooldown with `burst_pressed` false.
            .field("dash_charges", i64::from(dash.charges_available))
            .field("dash_cooldown", dash.cooldown)
            // A HARD lock means the body has no input authority at all this
            // tick — a disagreement under one is the system working.
            // `hitstun_timer` is the partial-control penalty, and `attacking` is
            // a move owning the body's motion.
            .field(
                "hard_lock",
                combat.map_or(0.0, ambition_characters::actor::BodyCombat::hard_lock_timer),
            )
            .field("hitstun", combat.map_or(0.0, |c| c.hitstun_timer))
            .field("attacking", melee.is_some_and(|m| m.is_swinging()))
            // The two acceleration terms the integrator adds, in world units per
            // second squared. At 60Hz a `-99`/tick step needs `-5940` here.
            .field(
                "gravity_accel_x",
                motion_frame.map_or(0.0, |f| f.get().gravity_acceleration().x),
            )
            .field(
                "external_accel_x",
                motion_frame.map_or(0.0, |f| f.get().external_acceleration().x),
            ),
        );
    }
}

/// The strongest stable subject a body has — and never `None`.
///
/// Strongest first: the SEAT (survives death and respawn), then the actor's
/// stable id, then an explicitly UNSTABLE entity key. the unstable variant is
/// a recorded API leak and still beats global: a recycled index can mislead one
/// later query; a world fact misleads every query forever.
fn body_subject(
    drivers: &Query<&DrivingParticipant>,
    identities: &Query<&ambition_combat::components::ActorIdentity>,
    body: Entity,
) -> (SubjectKey, Option<u8>) {
    if let Some(seat) = drivers.get(body).ok().map(|driver| driver.0 .0) {
        return (SubjectKey::Seat(seat), Some(seat));
    }
    if let Ok(identity) = identities.get(body) {
        return (SubjectKey::Sim(identity.id.clone()), None);
    }
    (SubjectKey::Unstable(body.to_bits()), None)
}

/// Why was this hit accepted or rejected? — the inspector's damage question.
///
/// Reads `BodyHitResolved`, which both hit paths announce from the value the
/// SHARED resolver already produced. The outcome vocabulary is the answer:
/// `Ignored` is an i-frame or a corpse, `Blocked` is a raised shield, `Armored`
/// and `WalletShielded` are a spent defence, `Damaged` carries the amount and
/// whether it killed.
///
/// the raw damage travels beside the outcome, because "asked for 30 and
/// dealt 0" and "asked for 0" are different findings and the outcome alone
/// cannot tell them apart.
pub fn record_hit_resolutions(
    log: Option<ResMut<CausalRecording>>,
    mut hits: MessageReader<BodyHitResolved>,
    bodies: Query<&DrivingParticipant>,
    identities: Query<&ambition_combat::components::ActorIdentity>,
) {
    let Some(mut log) = log else {
        // Drain regardless — a backlog surfacing on the frame somebody enables
        // the instrument would be stamped with the WRONG tick.
        hits.clear();
        return;
    };
    if !log.is_recording() {
        hits.clear();
        return;
    }
    for hit in hits.read() {
        let (outcome, damage, died) = match hit.resolution {
            BodyHitResolution::Ignored => ("ignored", 0, false),
            BodyHitResolution::Blocked => ("blocked", 0, false),
            BodyHitResolution::Armored => ("armored", 0, false),
            BodyHitResolution::WalletShielded { spent } => ("wallet_shielded", spent, false),
            BodyHitResolution::Damaged { damage, died } => ("damaged", damage, died),
        };
        let mut fact = CausalFact::new(
            domains::DAMAGE,
            0,
            FactDetail::new(
                "hit_resolved",
                match hit.resolution {
                    BodyHitResolution::Ignored => {
                        "hit ignored — invulnerable, or already dead".to_string()
                    }
                    BodyHitResolution::Blocked => "hit blocked by a raised shield".to_string(),
                    BodyHitResolution::Armored => "hit absorbed by worn armor".to_string(),
                    BodyHitResolution::WalletShielded { spent } => {
                        format!("hit absorbed by a wallet shield, {spent} spent")
                    }
                    BodyHitResolution::Damaged { damage, died } => {
                        if died {
                            format!("took {damage} and died")
                        } else {
                            format!("took {damage}")
                        }
                    }
                },
            ),
        )
        .field("outcome", outcome)
        .field("damage", i64::from(damage))
        .field("raw_damage", i64::from(hit.raw_damage))
        .field("died", died)
        .field("source", format!("{:?}", hit.source));
        let (subject, seat) = body_subject(&bodies, &identities, hit.body);
        fact = fact.about(subject);
        if let Some(seat) = seat {
            fact = fact.by_participant(seat);
        }
        log.record(fact);
    }
}

/// Why did knockback have this magnitude and direction?
///
/// The last of the inspector's required damage questions, and the one the
/// velocity alone cannot answer: a short launch is a weak hit, a well-DI'd one,
/// or a hit that carried no knockback at all, and those are three different
/// findings. The fact carries all three inputs beside the result.
pub fn record_hit_reactions(
    log: Option<ResMut<CausalRecording>>,
    mut reactions: MessageReader<BodyReactionApplied>,
    bodies: Query<&DrivingParticipant>,
    identities: Query<&ambition_combat::components::ActorIdentity>,
) {
    let Some(mut log) = log else {
        reactions.clear();
        return;
    };
    if !log.is_recording() {
        reactions.clear();
        return;
    }
    for applied in reactions.read() {
        let r = applied.reaction;
        let mut fact = CausalFact::new(
            domains::DAMAGE,
            0,
            FactDetail::new(
                "knockback_applied",
                if r.had_knockback {
                    format!(
                        "launched at {:.0} px/s for {:.2}s of hitstun",
                        r.velocity.length(),
                        r.hitstun
                    )
                } else {
                    "hurt but not launched — the hit carried no knockback".to_string()
                },
            ),
        )
        .field("speed", r.velocity.length())
        .field("velocity_x", r.velocity.x)
        .field("velocity_y", r.velocity.y)
        .field("hitstun", r.hitstun)
        .field("had_knockback", r.had_knockback)
        // DI is the victim's own contribution. ZERO means it did not steer,
        // which is a different finding from steering and being overruled.
        .field("di_x", r.di_input_local.x)
        .field("di_y", r.di_input_local.y)
        .field("steered", r.di_input_local.length() > 0.0);
        let (subject, seat) = body_subject(&bodies, &identities, applied.body);
        fact = fact.about(subject);
        if let Some(seat) = seat {
            fact = fact.by_participant(seat);
        }
        log.record(fact);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_causal::{FactValue, RecordingPolicy};
    use ambition_characters::brain::{Brain};
use ambition_characters::control::{PlayerSlot};

    fn app() -> App {
        let mut app = App::new();
        app.init_resource::<CausalRecording>();
        app.world_mut()
            .resource_mut::<CausalRecording>()
            .set_policy(RecordingPolicy::All);
        app.add_systems(Update, record_player_movement_intent);
        app
    }

    fn seated_body(app: &mut App, slot: u8, locomotion_x: f32) {
        let mut control = ActorControl::default();
        control.0.locomotion.x = locomotion_x;
        app.world_mut().spawn((
            BodyKinematics {
                pos: ambition_platformer2d_core::Vec2::new(120.0, 300.0),
                ..Default::default()
            },
            BodyGroundState::default(),
            DrivingParticipant(PlayerSlot(slot)),
            control,
        ));
    }

    #[test]
    fn each_seated_body_explains_its_own_movement() {
        let mut app = app();
        seated_body(&mut app, 0, 1.0);
        seated_body(&mut app, 1, -1.0);
        app.world_mut()
            .resource_mut::<CausalRecording>()
            .set_tick(30);
        app.update();

        let log = app.world().resource::<CausalRecording>();
        for (slot, expected) in [(0u8, 1.0_f32), (1, -1.0)] {
            let explanation = log.explain(30, &SubjectKey::Seat(slot));
            let intent = explanation
                .first("movement_intent")
                .unwrap_or_else(|| panic!("seat {slot} published its intent"));
            assert_eq!(
                intent.get("locomotion_x"),
                Some(&FactValue::Float(expected.into())),
                "seat {slot} explains ITS OWN movement, not another seat's"
            );
            assert_eq!(intent.participant, Some(slot));
        }
    }

    #[test]
    fn a_body_with_no_seat_publishes_nothing_rather_than_a_recycled_index() {
        let mut app = app();
        app.world_mut().spawn((
            BodyKinematics::default(),
            BodyGroundState::default(),
            Brain::stand_still(),
            ActorControl::default(),
        ));
        app.update();
        assert!(
            app.world().resource::<CausalRecording>().is_empty(),
            "an unseated body has no stable identity, and an entity index is not one — \
             indices are recycled, so a later body would inherit this one's explanation"
        );
    }

    #[test]
    fn the_intent_distinguishes_asking_for_nothing_from_being_refused() {
        // A position sample cannot tell these apart, which is the whole reason
        // the fact records the EMITTED intent beside the body state.
        let mut app = app();
        seated_body(&mut app, 0, 0.0);
        app.update();
        let log = app.world().resource::<CausalRecording>();
        let intent = log
            .explain(0, &SubjectKey::Seat(0))
            .first("movement_intent")
            .cloned()
            .expect("a still body still explains itself");
        assert_eq!(intent.get("locomotion_x"), Some(&FactValue::Float(0.0)));
        assert_eq!(
            intent.get("vel_x"),
            Some(&FactValue::Float(0.0)),
            "asked for nothing, moving at nothing — the pair is the finding"
        );
    }
}

#[cfg(test)]
mod damage_tests {
    use super::*;
    use ambition_causal::{FactValue, RecordingPolicy};
    use ambition_characters::brain::{Brain};
use ambition_characters::control::{PlayerSlot};

    fn app() -> App {
        let mut app = App::new();
        app.init_resource::<CausalRecording>();
        app.add_message::<BodyHitResolved>();
        app.add_systems(Update, record_hit_resolutions);
        app
    }

    fn announce(app: &mut App, body: Entity, resolution: BodyHitResolution, raw: i32) {
        app.world_mut().write_message(BodyHitResolved {
            body,
            resolution,
            source: ambition_combat::HitSource::Projectile,
            raw_damage: raw,
        });
    }

    /// Every outcome the resolver can reach is a distinguishable answer.
    ///
    /// "Why did nothing happen" is the question people actually bring here, and
    /// an i-frame, a shield, spent armor and a real zero are four different
    /// reasons for the same visible result.
    #[test]
    fn each_hit_outcome_explains_itself_differently() {
        let mut app = app();
        app.world_mut()
            .resource_mut::<CausalRecording>()
            .set_policy(RecordingPolicy::All)
            .set_tick(12);
        let body = app
            .world_mut()
            .spawn(DrivingParticipant(PlayerSlot(2)))
            .id();

        for (resolution, expected) in [
            (BodyHitResolution::Ignored, "ignored"),
            (BodyHitResolution::Blocked, "blocked"),
            (BodyHitResolution::Armored, "armored"),
            (
                BodyHitResolution::WalletShielded { spent: 7 },
                "wallet_shielded",
            ),
            (
                BodyHitResolution::Damaged {
                    damage: 30,
                    died: false,
                },
                "damaged",
            ),
        ] {
            announce(&mut app, body, resolution, 30);
            app.update();
            let why = app
                .world()
                .resource::<CausalRecording>()
                .explain(12, &SubjectKey::Seat(2));
            let last = why.all("hit_resolved").last().expect("recorded");
            assert_eq!(
                last.get("outcome"),
                Some(&FactValue::Text(expected.into())),
                "{resolution:?} must be its own answer, not merged into a neighbour"
            );
        }
    }

    /// asked for 30 and dealt 0 is not the same finding as asked for 0.
    /// The outcome alone cannot tell them apart, which is why the raw damage
    /// travels beside it.
    #[test]
    fn a_blocked_hit_still_records_what_it_asked_for() {
        let mut app = app();
        app.world_mut()
            .resource_mut::<CausalRecording>()
            .set_policy(RecordingPolicy::All)
            .set_tick(13);
        let body = app
            .world_mut()
            .spawn(DrivingParticipant(PlayerSlot(0)))
            .id();
        announce(&mut app, body, BodyHitResolution::Blocked, 42);
        app.update();

        let why = app
            .world()
            .resource::<CausalRecording>()
            .explain(13, &SubjectKey::Seat(0));
        let hit = why.first("hit_resolved").expect("recorded");
        assert_eq!(hit.get("damage"), Some(&FactValue::Int(0)));
        assert_eq!(
            hit.get("raw_damage"),
            Some(&FactValue::Int(42)),
            "the shield is only interesting next to what it stopped"
        );
    }

    /// A lethal hit says so, so a death is explainable from the damage domain
    /// alone even where no ruleset owns the consequence.
    #[test]
    fn a_lethal_hit_records_that_it_killed() {
        let mut app = app();
        app.world_mut()
            .resource_mut::<CausalRecording>()
            .set_policy(RecordingPolicy::All)
            .set_tick(14);
        let body = app
            .world_mut()
            .spawn(DrivingParticipant(PlayerSlot(1)))
            .id();
        announce(
            &mut app,
            body,
            BodyHitResolution::Damaged {
                damage: 99,
                died: true,
            },
            99,
        );
        app.update();
        let hit = app
            .world()
            .resource::<CausalRecording>()
            .explain(14, &SubjectKey::Seat(1))
            .first("hit_resolved")
            .cloned()
            .expect("recorded");
        assert_eq!(hit.get("died"), Some(&FactValue::Bool(true)));
    }
}

#[cfg(test)]
mod knockback_tests {
    use super::*;
    use ambition_damage::BodyReaction;
    use ambition_causal::{FactValue, RecordingPolicy};
    use ambition_characters::brain::{Brain};
use ambition_characters::control::{PlayerSlot};

    fn reaction_app() -> App {
        let mut app = App::new();
        app.init_resource::<CausalRecording>();
        app.add_message::<BodyReactionApplied>();
        app.add_systems(Update, record_hit_reactions);
        app.world_mut()
            .resource_mut::<CausalRecording>()
            .set_policy(RecordingPolicy::All)
            .set_tick(21);
        app
    }

    /// A short launch has three different causes, and the velocity cannot
    /// tell them apart. That is the whole reason this fact exists.
    #[test]
    fn a_short_launch_distinguishes_a_weak_hit_from_a_well_steered_one() {
        let mut app = reaction_app();
        let body = app
            .world_mut()
            .spawn(DrivingParticipant(PlayerSlot(1)))
            .id();

        // Steered: the victim held DI and the launch came out short.
        app.world_mut().write_message(BodyReactionApplied {
            body,
            reaction: BodyReaction {
                velocity: ambition_platformer2d_core::Vec2::new(40.0, 0.0),
                di_input_local: ambition_platformer2d_core::Vec2::new(-1.0, 0.0),
                hitstun: 0.2,
                had_knockback: true,
            },
        });
        app.update();
        let steered = app
            .world()
            .resource::<CausalRecording>()
            .explain(21, &SubjectKey::Seat(1))
            .first("knockback_applied")
            .cloned()
            .expect("recorded");
        assert_eq!(steered.get("steered"), Some(&FactValue::Bool(true)));
        assert_eq!(steered.get("had_knockback"), Some(&FactValue::Bool(true)));

        // Not launched at all: hurt, no knockback. Same visible stillness, a
        // completely different finding.
        let mut app = reaction_app();
        let body = app
            .world_mut()
            .spawn(DrivingParticipant(PlayerSlot(1)))
            .id();
        app.world_mut().write_message(BodyReactionApplied {
            body,
            reaction: BodyReaction {
                velocity: ambition_platformer2d_core::Vec2::ZERO,
                di_input_local: ambition_platformer2d_core::Vec2::ZERO,
                hitstun: 0.0,
                had_knockback: false,
            },
        });
        app.update();
        let unlaunched = app
            .world()
            .resource::<CausalRecording>()
            .explain(21, &SubjectKey::Seat(1))
            .first("knockback_applied")
            .cloned()
            .expect("recorded");
        assert_eq!(
            unlaunched.get("had_knockback"),
            Some(&FactValue::Bool(false))
        );
        assert_eq!(unlaunched.get("steered"), Some(&FactValue::Bool(false)));
        assert!(
            unlaunched.detail.summary.contains("no knockback"),
            "and it SAYS so, so a reader chasing a launch stops here: {}",
            unlaunched.detail.summary
        );
    }

    /// The speed and hitstun are fields, so "launched far" and "launched long"
    /// are separately checkable.
    #[test]
    fn the_launch_records_its_speed_and_its_hitstun() {
        let mut app = reaction_app();
        let body = app
            .world_mut()
            .spawn(DrivingParticipant(PlayerSlot(0)))
            .id();
        app.world_mut().write_message(BodyReactionApplied {
            body,
            reaction: BodyReaction {
                velocity: ambition_platformer2d_core::Vec2::new(300.0, 400.0),
                di_input_local: ambition_platformer2d_core::Vec2::ZERO,
                hitstun: 0.45,
                had_knockback: true,
            },
        });
        app.update();
        let fact = app
            .world()
            .resource::<CausalRecording>()
            .explain(21, &SubjectKey::Seat(0))
            .first("knockback_applied")
            .cloned()
            .expect("recorded");
        assert_eq!(fact.get("speed"), Some(&FactValue::Float(500.0)));
        assert_eq!(
            fact.get("hitstun"),
            Some(&FactValue::Float(0.45_f32.into()))
        );
    }
}
