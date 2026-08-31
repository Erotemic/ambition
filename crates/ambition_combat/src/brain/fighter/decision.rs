//! Fighter decision rig: cadence, held intent, APM limiting, and deterministic noise.
//!
//! Every `FighterState` field affects future decisions and is rollback state.
//! APM is enforced at action emission, and the deterministic RNG advances only
//! when a sample is consumed.

use ambition_platformer2d_core::{self as ae, Vec2};

use super::recovery::{BodyKit, RecoveryLens};
use super::rollout::refine_by_rollout;
use ambition_characters::actor::control::ActorControlFrame;
use ambition_characters::brain::attack_kit::AttackCandidate;
use ambition_characters::brain::fighter::data::{
    FighterCfg, FighterState, FoeSample, PendingAttack,
};
use ambition_characters::brain::fighter::habit::Choice;
use ambition_characters::brain::fighter::options::{
    generate_options, MovementVerb, UtilityWeights,
};
use ambition_characters::brain::fighter::situation::{classify, Situation};
use ambition_characters::brain::BrainSnapshot;
use ambition_characters::perception::WorldView;

/// SplitMix64. One step per CONSUMED sample, which is what makes the stream
/// reproducible under rollback: a tick that reads no noise leaves the seed
/// exactly where it was.
fn split_mix_next(seed: &mut u64) -> u64 {
    *seed = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *seed;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// The next sample in `[-1, 1)`.
fn next_signed_unit(seed: &mut u64) -> f32 {
    let bits = split_mix_next(seed);
    // 53 bits into [0,1), then mapped — the same shape as an f64 uniform, at f32
    // precision, so the distribution is not lumpy at the ends.
    let unit = (bits >> 11) as f64 / (1u64 << 53) as f64;
    (unit * 2.0 - 1.0) as f32
}

/// One tick of the fighter brain.
///
/// Order matters and is the spec's: observe, emit the held intent, age the
/// clocks, mature a pending press, then — on a decision tick — think.
///
/// `view` is this tick's LIVE world, handed in by the integration layer. It goes
/// straight into the delay buffer and is never read directly; what the brain
/// reasons over is whatever the buffer hands back.
pub fn tick_fighter(
    cfg: &FighterCfg,
    state: &mut FighterState,
    snapshot: &BrainSnapshot,
    view: Option<&WorldView>,
    out: &mut ActorControlFrame,
) {
    if let Some(view) = view {
        state.perception.observe(view.clone());
    }

    // The APM window is wall time, not decision time: a brain that thinks slowly
    // and presses every thought is still pressing at that rate.
    state.apm.elapsed_ticks = state.apm.elapsed_ticks.saturating_add(1);

    // before the held frame is emitted and before the pending press matures:
    // a press decided while free is stale the moment a relationship exists, and
    // maturing it would spend an APM token on a button that cannot fire.
    if let Some(frame) = capture_context_frame(snapshot) {
        state.pending_press = None;
        state.held = frame.clone();
        *out = frame;
        return;
    }

    // Emit held intent, then clear every edge-triggered input before this tick's
    // decision. New press edges must be added to `clear_edges` or they will latch
    // across ticks.
    let mut frame = state.held.clone();
    frame.clear_edges();

    // THE HELD BUTTON IS DERIVED, NEVER LATCHED. `clear_edges` deliberately
    // leaves sustains alone, so a sustain written once would stay written;
    // spending the charge here means the only thing that can hold a button down
    // is a charge that has ticks left.
    //
    // ⛔ BOTH FIELDS, EVERY TICK. Writing only the one the current gesture wants
    // would leave the OTHER stuck down across a switch — a brain that charged a
    // smash and then chose a special would hold Attack forever.
    state.charge_hold_ticks = state.charge_hold_ticks.saturating_sub(1);
    hold_the_committed_button(state, &mut frame);

    if state.ticks_until_decision > 0 {
        state.ticks_until_decision -= 1;
    }

    match state.pending_press {
        Some(PendingAttack {
            ticks: 0,
            binding,
            hold_ticks: hold,
        }) => {
            state.pending_press = None;
            // THE ONE EMISSION POINT. A press with no APM token is DROPPED
            // and the held movement stays, which is what makes the humanity
            // histogram a measurement of behaviour rather than of intent.
            if state.apm.may_press(cfg.profile.apm_cap, cfg.tick_hz) {
                press_the_chosen_attack(binding, &mut frame);
                state.apm.presses = state.apm.presses.saturating_add(1);
                // The charge starts on the frame the press does, because the
                // move freezes at its authored hold point and asks what the
                // button is doing — a hold armed a tick later has already
                // missed the question.
                state.charge_hold_ticks = hold;
                state.charge_hold_gesture = charge_gesture_of(binding.verb);
                hold_the_committed_button(state, &mut frame);
            }
        }
        Some(pending) => {
            state.pending_press = Some(PendingAttack {
                ticks: pending.ticks - 1,
                ..pending
            })
        }
        None => {}
    }

    if state.ticks_until_decision == 0 {
        state.ticks_until_decision = cfg.interval();
        decide(cfg, state, snapshot, &mut frame);
    }

    state.held = frame.clone();

    // REELING IS A REFLEX, AND IT IS AN OVERLAY RATHER THAN AN INTENT.
    //
    // DI and the hitlag shift are read off the held stick on the frames a hit
    // resolves, which the decision cadence does not line up with: a brain that
    // thinks every five ticks would DI whatever it happened to be walking
    // toward. So the survival stick is recomputed every tick and written to the
    // emitted frame only.
    //
    // It is deliberately NOT stored in `state.held`. Held intent is what this
    // brain decided; a reflex is what its body is doing to it. Storing it would
    // leave the fighter drifting along the last launch for the rest of the
    // decision interval after hitstun ended.
    if let Some(view) = state.perception.perceive() {
        if let Some(stick) = super::reeling::survival_stick(view) {
            frame.locomotion = stick;
        }
        // The tech is on the same reflex clock and for the same reason: the
        // window is twenty frames wide and the decision cadence is five, so a
        // read made only on decision ticks would miss most landings. It spends
        // no APM — a tech is a hand reacting, not a plan.
        if super::reeling::tech_press(view) {
            frame.burst_pressed = true;
        }
    }
    *out = frame;
}

/// What a fighter inside a capture presses — `None` at neither end of one.
///
/// the same two fields a person's Attack button writes, and no capture API
/// at all. `trigger_moveset_moves` reads the RELATIONSHIP and turns a neutral
/// press into a pummel and a forward press into a throw; a brain that reached
/// for a capture-specific verb would be the CPU-only road this design exists
/// without.
///
/// the captive is not silent and the captor is not idle, which are the
/// two failures this replaces. A held body struggles — that is its whole agency
/// and the only thing it may ask for — and a holding body spends the hold rather
/// than standing in it until the clock runs out.
fn capture_context_frame(snapshot: &BrainSnapshot) -> Option<ActorControlFrame> {
    if snapshot.captured {
        let mut frame = ActorControlFrame::neutral();
        // no APM token. Mashing out of a grab is the one thing a person
        // really does at machine speed, and spending the decision budget on it
        // would make a fighter's escape compete with its next attack.
        if ambition_characters::control::struggling_this_tick(snapshot.captured_for, snapshot.dt) {
            frame.melee_pressed = true;
        }
        return Some(frame);
    }
    if !snapshot.holding_captive {
        return None;
    }
    let mut frame = ActorControlFrame::neutral();
    frame.melee_pressed = true;
    // Pummel once, then throw. deliberately the simplest policy that proves
    // the road: opponent percent, stage edge, kill potential and escape risk are
    // all real inputs it does not read.
    frame.attack_axis = if snapshot.pummels_landed >= 1 {
        // MIRRORED: `attack_dir_from_axis` reads `axis.x * facing`, so a bare
        // `+x` is *forward* only for a right-facing body. See `aim_the_stick`.
        let facing = if snapshot.actor_facing < 0.0 {
            -1.0
        } else {
            1.0
        };
        ae::LocalAxes::new(
            ambition_characters::actor::attack_gesture::TILT_DEFLECTION * facing,
            0.0,
        )
    } else {
        ae::LocalAxes::ZERO
    };
    Some(frame)
}

/// The decision tick: perceive, classify, generate, refine, translate.
fn decide(
    cfg: &FighterCfg,
    state: &mut FighterState,
    snapshot: &BrainSnapshot,
    frame: &mut ActorControlFrame,
) {
    let Some(view) = state.perception.perceive() else {
        return;
    };
    let situation = classify(view);

    // A press is armed at one decision and matures several ticks later, and the
    // situation can change in between — which on a platform stage it does, in
    // the one direction that matters. The trace caught it exactly: an attack
    // armed while airborne OVER the lip (`floor_edge=Some(45)`, still `Neutral`)
    // matured two decisions later with the body past the edge and asking to
    // `Recover`, and every attack in this engine LUNGES. So the fighter's own
    // queued swing carried it out at 700 px/s while its emitted input said left.
    //
    // and it is a DROP, not a ban — the distinction matters now that L2 offers a recovering
    // body its lifting moves. The stale press dies here; `generate_options` runs below and re-arms
    // from the Recovery option set in this same tick, so a body whose kit contains a way home
    // presses that instead of nothing.
    //
    // It no longer does — refusing was right about attacking and wrong about the repertoire,
    // since a genre fighter's answer to being offstage IS a move.)
    if situation == Situation::Recovery {
        state.pending_press = None;
    }

    // HABIT OBSERVATION IS PART OF THE DECISION TICK (§13.5). The foe's
    // observable choice since the last decision is fed to the model under the
    // situation that was live when it happened. This is FB5's missing writer —
    // until now the only thing that called `observe` was a test.
    let sample = foe_sample(view);
    if let (Some(previous), Some(current)) = (state.last_foe, sample) {
        state
            .habits
            .observe(situation, infer_choice(previous, current));
    }
    state.last_foe = sample;

    // THE KIT RIDES THE SNAPSHOT (§13.2). The brain cannot see the body's
    // moveset — `ambition_combat` depends on `ambition_characters` and not the
    // reverse — so the actors-side snapshot builder fills `attack_kit` from the
    // body's real `ActorMoveset`, exactly like `actor_aerial`. Body-derived truth
    // arriving through the world-in port.
    let options = generate_options(
        view,
        situation,
        &snapshot.attack_kit,
        &cfg.profile.utility_weights,
    );

    // THE ROLLOUT PREDICTS THE BODY IT IS IN, not a default one. The
    // config's tuning carries the foe assumptions and the hit response; the
    // MOVEMENT half comes from the body's own authored `MovementTuning` when the
    // snapshot carries it. Without this a character that authors its own gravity
    // or run speed is predicted as somebody else — and the shadow's copied
    // constants were three-for-three wrong for weeks under exactly that shape.
    let tuning = match snapshot.movement_tuning.as_ref() {
        Some(movement) => cfg.tuning.clone().with_movement(movement),
        None => cfg.tuning.clone(),
    };

    // THE RECOVERY LENS — the one real-kernel seam in the decision.
    //
    // Built once per decision (never per rolled line, and never per tick): the
    // world lowering allocates a block per perceived solid and does not change
    // between the lines of one decision. `None` — no kit on the snapshot, or a
    // view that names no stage — leaves L3 exactly as it was, which is what makes
    // this safe for every brain seat that is not a fighter on a stage.
    //
    // Both halves are body-derived truth from the world-in port, the same channel
    // `movement_tuning` and `attack_kit` arrive on. Nothing here interprets the
    // ability set; it is handed to the kernel, which owns what a body can do.
    //
    // THE ROUTES ARE PROPOSED, NOT CHOSEN. Every move in this body's kit that commands a
    // displacement becomes a candidate route, in `lifting_candidates`' deterministic order, and the
    // LENS decides which of them is useful from where the body actually is. Nothing here knows
    // whose body it is; the affordance is still derived from move geometry and never from an
    // identity.
    let route_moves =
        ambition_characters::brain::fighter::options::lifting_candidates(&snapshot.attack_kit);
    // ⭐ THE ROUTE ITSELF, not a lift reconstructed from three fields. A
    // reconstruction here could only ever describe a burst, which is how the
    // planner came to be blind to every other kind (D250).
    let routes: Vec<ambition_entity_catalog::RecoveryRoute> = route_moves
        .iter()
        .map(|c| c.frames.recovery_route)
        .collect();
    let lens = snapshot
        .abilities
        .zip(snapshot.movement_tuning.as_ref())
        .and_then(|(abilities, movement)| {
            RecoveryLens::from_view(
                &view,
                BodyKit {
                    abilities,
                    movement: *movement,
                },
                &routes,
                1.0 / cfg.tick_hz.max(1.0),
            )
        });

    // L3 refines L2's ranking when the profile pays for rollouts. `None` means
    // this profile does not, or there was nothing to refine.
    let refined = refine_by_rollout(
        view,
        situation,
        &options,
        &state.habits,
        &cfg.profile,
        &tuning,
        cfg.tick_hz,
        // How long this body is COMMITTED to whatever it decides: exactly until
        // it decides again.
        cfg.interval(),
        lens.as_ref(),
    );

    // MOVEMENT: the best verb the rollout did not veto.
    //
    // a verdict nothing consumes is not a verdict. L3 now rolls each
    // movement line and names the ones that end with this body out of the world;
    // if the rig still took `movement.first()`, that list would be a field in a
    // struct and the fighter would keep walking off the stage — which is the
    // exact defect class this codebase keeps rediscovering (a registration that
    // is inert, a seam that is unreachable, a refusal that cannot fire).
    //
    // L2 scores where the floor is NOW. The rollout is the only thing in the
    // brain that knows where the body will BE, so on this one question it
    // outranks the score rather than adjusting it.
    let vetoed = refined
        .as_ref()
        .map(|refined| refined.suicidal_movement.as_slice())
        .unwrap_or(&[]);
    // NO VERB HAS SPOKEN YET, SO THERE IS NO LATERAL INPUT YET. `frame`
    // arrives holding the last decision's answer; clearing here rather than
    // inside each verb makes "nothing was chosen" mean "nothing is pressed"
    // structurally, instead of depending on every branch below to remember.
    //
    // this replaced an explicit `halt()` on the empty case. An unreachable refusal reads as
    // protection while protecting nothing; `ladder_probe` confirmed it fires zero times across five
    // matches.
    frame.locomotion = ae::LocalAxes::ZERO;
    // ⛔⛔ AN UNMODELLED VERB IS NOT A SAFE ONE, AND THIS READ IT AS ONE.
    // `movement_intent` returns `None` for the verbs the shadow cannot
    // simulate — Dodge, Blink, Shield-as-motion — and says outright that
    // *"a rollout that reported every unknown as safe or as fatal would be
    // lying in one direction or the other"*. It reports neither: the option is
    // dropped from the rolled set, so it never appears in `vetoed`. A `find`
    // over "not vetoed" then promotes it above every verb the rollout DID judge,
    // and — worse — it stops `least_bad_movement` from ever firing, because that
    // fallback is gated on every OFFERED verb being vetoed and an unjudged one
    // never is.
    //
    // ⭐ MEASURED, seed 0 of `ladder_rig --sweep-below`, the two ticks before
    // level 6 dies at 0%:
    //
    // ```text
    // offered=[Approach, Dodge, Jump] vetoed=[Approach, Jump]
    //   unmodelled=[Dodge] chose=Some(Dodge) least_bad=Some(Approach)
    // ```
    //
    // The rollout had an answer for "everything I can judge is fatal" and it was
    // discarded for a verb nobody rolled. Two ticks later the body is off the
    // stage.
    //
    // ⭐ SO THE FALLBACK IS GATED ON THE JUDGED SET, not the offered one. Three
    // tiers, in the order the rollout's own contract implies:
    //
    // ```text
    // 1. judged and not vetoed   the rollout says this one lives
    // 2. least-bad               every judged line is fatal; this one dies latest
    // 3. unmodelled              nobody knows; better than a known death, and
    //                            worse than a measured survival
    // ```
    let unmodelled = refined
        .as_ref()
        .map(|refined| refined.unmodelled_movement.as_slice())
        .unwrap_or(&[]);
    let chosen = pick_movement(
        &options.movement,
        vetoed,
        unmodelled,
        refined
            .as_ref()
            .and_then(|refined| refined.least_bad_movement),
    );
    if let Some(verb) = chosen {
        apply_movement(verb, view, frame);
    }

    // A chosen attack carries its exact move binding through execution noise.
    // During recovery, the movement kernel has final authority: an endorsed
    // route is pressed, an already-regained state presses nothing, and a bounded
    // search miss falls back to the ordinary attack ranking.
    let endorsed_recovery = if situation == Situation::Recovery {
        lens.as_ref().map(|lens| {
            lens.best_route(super::recovery::RecoveryQuery {
                pos: view.self_view.pos,
                vel: view.self_view.vel,
                air_jumps_left: view.self_view.air_jumps_left,
            })
        })
    } else {
        None
    };
    // Keep the authored move id with the physical binding so traces can identify
    // the selected action even when multiple moves share a button/direction.
    let wants_attack: Option<(
        ambition_characters::brain::attack_kit::AttackBinding,
        String,
    )> = match endorsed_recovery {
        Some(verdict) if verdict.regained() => verdict
            .route
            .and_then(|index| route_moves.get(index))
            .map(|candidate| (candidate.binding, candidate.move_id.clone())),
        // A completed recovery search with no endorsed route is authoritative:
        // press no recovery attack. Movement steering continues and the next
        // decision re-evaluates from the new state.
        Some(_) => None,
        // An ordinary situation: no recovery search ran, and the ranking is
        // exactly right.
        None => refined
            .as_ref()
            .and_then(|refined| refined.binding.zip(refined.move_id.clone()))
            .or_else(|| {
                options
                    .attacks
                    .first()
                    .map(|attack| (attack.binding, attack.move_id.clone()))
            }),
    };
    // AIM THE STICK NOW, PRESS THE BUTTON LATER — because that is what a hand
    // does. The direction is a SUSTAIN (`clear_edges` leaves `attack_axis`
    // alone, exactly as it leaves `jump_held`) and the button is an EDGE, so the
    // decision that chooses a move sets the stick and the maturing press only
    // has to close the circuit. See [`aim_the_stick`] for the two bugs this
    // ordering deletes.
    if let Some((binding, _)) = wants_attack.as_ref() {
        aim_the_stick(*binding, view.self_view.facing, frame);
    }
    // A HAND HOLDING A BUTTON IS NOT PRESSING SOMETHING ELSE.
    //
    // Measured in a real match: smashes were armed with a full charge and paid
    // out at zero, because the next decision five ticks later armed another
    // attack and the emission reset the hold to that press's own — nearly always
    // a tap's. Seventeen smashes shared 283 held ticks between them and not one
    // reached its move's hold point. The charge in flight is a commitment, so it
    // owns the button until it is spent.
    let charging = state.charge_hold_ticks > 0;
    if let (Some((binding, _)), None, false) =
        (wants_attack.as_ref(), state.pending_press, charging)
    {
        let jitter = if cfg.profile.execution_noise > 0.0 {
            let sample = next_signed_unit(&mut state.noise).abs();
            (sample * cfg.profile.execution_noise * cfg.interval() as f32).round() as u32
        } else {
            0
        };
        // THE PRESS CANNOT OUTLIVE THE DECISION THAT MEANT IT. `cfg.interval()` is *"how
        // long this body is COMMITTED to whatever it decides: exactly until it decides again"*,
        // and the aimed stick above is held for exactly that long.
        let jitter = jitter.min(cfg.interval().saturating_sub(1));
        state.pending_press = Some(PendingAttack {
            ticks: jitter,
            binding: *binding,
            hold_ticks: match binding.verb {
                ambition_characters::brain::attack_kit::AttackVerb::Smash => {
                    super::charge::hold_ticks(
                        situation,
                        // ⛔ THE MOVE'S OWN HOLD POINT, not its startup. The charge
                        // begins where the timeline FREEZES, and a move that
                        // resolves no policy never freezes at all — its whole
                        // startup is the honest fallback, and holding a move that
                        // cannot charge is a no-op the same way a string hold on a
                        // move with no chain is.
                        options
                            .attacks
                            .iter()
                            .find(|attack| {
                                Some(&attack.move_id) == wants_attack.as_ref().map(|(_, id)| id)
                            })
                            .map_or(0.0, |attack| {
                                attack
                                    .frames
                                    .charge_hold_at_s
                                    .unwrap_or(attack.frames.startup_s)
                            }),
                        cfg.tick_hz,
                    )
                }
                // ⭐ A BASIC ATTACK HOLDS TOO, and for a different gesture: a
                // held Smash is a CHARGE, a held anything-else CONTINUES A
                // STRING. Both are the same button and the engine tells them
                // apart by the intent's strength, so the brain does not have to.
                //
                // ⛔ Why every basic and not only the ones with chains: a
                // continuation can reach nothing but a successor the playing
                // window already NAMES, so holding a move that authors no chain
                // is a no-op. Asking the brain which moves have chains would put
                // a second copy of the cancel table in the scorer.
                ambition_characters::brain::attack_kit::AttackVerb::Basic => {
                    super::charge::string_hold_ticks(
                        situation,
                        options
                            .attacks
                            .iter()
                            .find(|attack| {
                                Some(&attack.move_id) == wants_attack.as_ref().map(|(_, id)| id)
                            })
                            .map_or(0.0, |attack| attack.frames.startup_s),
                        cfg.tick_hz,
                    )
                }
                // ⭐ A SPECIAL HOLDS FOR THE SAME REASON A SMASH DOES, and
                // reaches the same decision: how long is worth holding HERE.
                // A move that resolves no charge policy reports no hold point
                // and the fallback holds through its startup, which for a
                // special that does not charge is a no-op — exactly as it is
                // for a smash that does not.
                ambition_characters::brain::attack_kit::AttackVerb::Special => {
                    super::charge::hold_ticks(
                        situation,
                        options
                            .attacks
                            .iter()
                            .find(|attack| {
                                Some(&attack.move_id) == wants_attack.as_ref().map(|(_, id)| id)
                            })
                            .map_or(0.0, |attack| {
                                attack
                                    .frames
                                    .charge_hold_at_s
                                    .unwrap_or(attack.frames.startup_s)
                            }),
                        cfg.tick_hz,
                    )
                }
                ambition_characters::brain::attack_kit::AttackVerb::Grab => 0,
            },
        });
    }

    trace_decision(
        view,
        &options,
        frame,
        snapshot.subject.as_deref(),
        DecisionSummary {
            situation,
            vetoed,
            chosen,
            // ⭐ WHY `chose` IS ALSO IN `vetoed`, when it is. Without these two
            // fields "every option was fatal and this one dies latest" and "an
            // unmodelled verb outranked the modelled ones" render identically.
            least_bad: refined
                .as_ref()
                .and_then(|refined| refined.least_bad_movement),
            unmodelled: refined
                .as_ref()
                .map(|refined| refined.unmodelled_movement.as_slice())
                .unwrap_or(&[]),
            attack: wants_attack.as_ref().map(|(_, id)| id.as_str()),
            recovery: endorsed_recovery,
            recovery_move: endorsed_recovery
                .and_then(|verdict| verdict.route)
                .and_then(|index| route_moves.get(index))
                .map(|candidate| candidate.move_id.as_str()),
            // the PROPOSALS, in probe order — without them a reader cannot
            // tell "the search rejected the grapple" from "the grapple was never
            // proposed", and those want opposite fixes. A borrow, so an untraced
            // decision costs nothing: the ids are rendered behind the early
            // return inside `trace_decision`.
            proposed_routes: &route_moves,
        },
    );
}

/// Everything one decision produced, in the terms [`trace_decision`]
/// publishes.
///
/// a struct rather than six more parameters, and not for tidiness: the trace
/// is the only consumer, so a field added here is a field the fact carries and
/// a field the rendered line shows. Splitting them across an argument list is
/// how the stderr half and the fact half drifted the first time.
struct DecisionSummary<'a, 'k> {
    /// L1's answer for this tick — the thing every other field is conditional on.
    situation: Situation,
    /// Movement verbs L3 struck off.
    vetoed: &'a [MovementVerb],
    /// The movement verb that survived.
    chosen: Option<MovementVerb>,
    /// The LEAST-BAD line, when every offered verb was vetoed and the choice
    /// fell to it.
    ///
    /// ⛔⛔ IT NEVER LEFT `refine_by_rollout` BEFORE, and that is the fact a
    /// reader most needs when `chose` is also in `vetoed`: "every option was
    /// fatal and this one dies latest" and "the veto was ignored" render
    /// identically without it. `None` here with a non-empty `vetoed` means a
    /// verb survived the veto on its own.
    least_bad: Option<MovementVerb>,
    /// Verbs the shadow model does not simulate, so the rollout judged neither
    /// safe nor fatal.
    ///
    /// ⚠ THE CALLER READS SILENCE AS SAFETY. `movement_intent` returns `None`
    /// for these and the option is dropped from the rolled set entirely, so an
    /// unmodelled verb is absent from `vetoed` and therefore outranks every
    /// modelled verb the moment the modelled ones are struck off. Publishing it
    /// is what makes that visible in a trace instead of inferable from source.
    unmodelled: &'a [MovementVerb],
    /// The authored move this decision will press, by id. `None` = no swing.
    attack: Option<&'a str>,
    /// The recovery search's verdict, when the situation made one run.
    recovery: Option<super::recovery::RouteVerdict>,
    /// The move the search got home on. `None` with a positive verdict means
    /// *"getting back without throwing anything"* — a different fact from a
    /// search that found nothing, which is why both are published.
    recovery_move: Option<&'a str>,
    /// Every route the repertoire PROPOSED, in the order the lens probes
    /// them ([`ambition_characters::brain::fighter::options::lifting_candidates`]). this is what separates
    /// *"the search rejected the grapple"* from *"the grapple was never
    /// proposed"*, and those two want opposite fixes. Held as candidates rather
    /// than strings so a run that is not tracing allocates nothing.
    proposed_routes: &'a [&'k AttackCandidate],
}

// Moving it to the module that owns the tilt/smash distinction removes that edge without either
// brain naming the other.

/// Which movement verb this decision takes, given what the rollout judged.
///
/// ⛔⛔ AN UNMODELLED VERB IS NOT A SAFE ONE, AND THIS USED TO READ IT AS ONE.
/// `movement_intent` returns `None` for the verbs the shadow cannot simulate —
/// Dodge, Blink, Shield-as-motion — and says outright that *"a rollout that
/// reported every unknown as safe or as fatal would be lying in one direction or
/// the other"*. It reports neither: the option is dropped from the rolled set,
/// so it never appears in `vetoed`. A `find` over "not vetoed" therefore
/// promoted it above every verb the rollout DID judge — and, worse, stopped
/// `least_bad_movement` from ever firing, because that fallback was gated on
/// every OFFERED verb being vetoed and an unjudged one never is.
///
/// ⭐ MEASURED, seed 0 of `ladder_rig --sweep-below`, on the two ticks before
/// level 6 dies at 0% damage:
///
/// ```text
/// offered=[Approach, Dodge, Jump] vetoed=[Approach, Jump]
///   unmodelled=[Dodge] chose=Some(Dodge) least_bad=Some(Approach)
/// ```
///
/// The rollout had an answer for *"everything I can judge is fatal"* and it was
/// discarded for a verb nobody rolled. Two ticks later the body is off the stage.
///
/// ⭐ THREE TIERS, in the order the rollout's own contract implies:
///
/// ```text
/// 1. judged and not vetoed   the rollout modelled this line and it lives
/// 2. least-bad               every judged line is fatal; this one dies latest
/// 3. unmodelled              nobody knows — better than a known death, worse
///                            than a measured survival
/// ```
///
/// ⚠ TIER 3 IS ALSO THE NO-ROLLOUT PATH. With rollouts off nothing is judged and
/// nothing is vetoed, so L2's order comes straight through tier 3 unchanged.
fn pick_movement(
    movement: &[ambition_characters::brain::fighter::options::MoveOption],
    vetoed: &[MovementVerb],
    unmodelled: &[MovementVerb],
    least_bad: Option<MovementVerb>,
) -> Option<MovementVerb> {
    movement
        .iter()
        .find(|option| !vetoed.contains(&option.verb) && !unmodelled.contains(&option.verb))
        .map(|option| option.verb)
        .or(least_bad)
        .or_else(|| {
            movement
                .iter()
                .find(|option| !vetoed.contains(&option.verb))
                .map(|option| option.verb)
        })
}

/// AIM THE ATTACK STICK — the direction half of a chosen move, written at
/// DECISION time and held until the next decision, the way a hand holds a stick.
///
/// the axis is in the body's gravity-local frame, the same frame
/// [`ActorControlFrame::locomotion`] is in and the same one a human's stick
/// arrives in — `attack_dir_from_axis` multiplies `axis.x` by the body's
/// `facing` to recover *forward*, so the CALLER owes it a facing-independent
/// vector. Up is NEGATIVE y, the screen convention `InputState` carries.
///
/// * the mirror. This wrote `Forward` as `+x` — a FACING-relative vector into
///   a gravity-local field — so the resolver multiplied by facing a second time
///   and every forward/back attack chosen while the body faced LEFT came out
///   reversed. George Booul's side special was selected 19–24 times per match
///   and performed zero times: `special_forward` mirrored to `Back`, no
///   `special_back` verb exists, and the chain fell back to `special` — which is
///   why the move ledger recorded two `bivalence` presses the decision log never
///   selected. That disagreement is the falsifier; nothing else produces it.
/// * the accidental smash. See
///   [`ambition_characters::actor::attack_gesture::TILT_DEFLECTION`].
///
/// a `Neutral` direction is a CENTRED stick, and centring it re-arms the
/// flick detector — which is correct: the next directional press is then a fresh
/// gesture rather than the tail of this one.
fn aim_the_stick(
    binding: ambition_characters::brain::attack_kit::AttackBinding,
    facing: f32,
    frame: &mut ActorControlFrame,
) {
    use ambition_characters::actor::attack_gesture::AttackDir;
    use ambition_characters::brain::attack_kit::AttackVerb;

    // A body whose facing has not been established yet still has to aim
    // somewhere; `+1` keeps `Forward` meaning `+x` rather than collapsing the
    // whole gesture to a centred stick.
    let facing = if facing < 0.0 { -1.0 } else { 1.0 };
    // A SMASH is the full shove that the body reads as a flick; everything else
    // is the partial deflection a tilt/aerial is made of.
    let push = match binding.verb {
        AttackVerb::Smash => 1.0,
        // A grab takes no stick at all: its only direction is Neutral, so this
        // multiplies a zero. Stated rather than folded into the arm below,
        // because "a grab is aimed like a tilt" would be a false sentence.
        AttackVerb::Grab => 0.0,
        // a SPECIAL has no tilt/smash distinction — `move_for_directional_verb`
        // only needs the direction to clear the deadzone — but it takes the same
        // partial deflection so that a special press can never leave a flick
        // armed behind it and turn the FOLLOWING tilt into a smash.
        AttackVerb::Basic | AttackVerb::Special => {
            ambition_characters::actor::attack_gesture::TILT_DEFLECTION
        }
    };
    frame.attack_axis = match binding.direction {
        AttackDir::Neutral => ae::LocalAxes::ZERO,
        AttackDir::Forward => ae::LocalAxes::new(push * facing, 0.0),
        AttackDir::Back => ae::LocalAxes::new(-push * facing, 0.0),
        AttackDir::Up => ae::LocalAxes::new(0.0, -push),
        AttackDir::Down => ae::LocalAxes::new(0.0, push),
    };
}

/// Press the move the brain chose — the BUTTON half only; the stick was
/// aimed by the decision that chose the move ([`aim_the_stick`]).
///
/// The verb picks the button and the held stick picks the variant, which is
/// exactly what `resolve_attack_gesture` reads and `move_for_directional_verb`
/// resolves — so a fighter reaches its up-tilt the same way a player does, and a
/// move with no binding was never in the kit to be chosen.
fn press_the_chosen_attack(
    binding: ambition_characters::brain::attack_kit::AttackBinding,
    frame: &mut ActorControlFrame,
) {
    use ambition_characters::brain::attack_kit::AttackVerb;

    match binding.verb {
        AttackVerb::Basic => {
            frame.melee_pressed = true;
            // ⛔ `Auto`, not `Tilt`: a CPU's basic attack asks the interpreter to
            // read its own stick as a person's would. Forcing `Tilt` here would
            // delete the fighter brain's ability to smash by flicking.
            frame.melee_strength_hint = ambition_platformer2d_core::AttackStrengthHint::Auto;
        }
        AttackVerb::Smash => {
            frame.melee_pressed = true;
            frame.melee_strength_hint = ambition_platformer2d_core::AttackStrengthHint::Smash;
        }
        AttackVerb::Special => {
            frame.special_pressed = true;
        }
        AttackVerb::Grab => {
            // The same edge a person's Grab button writes. Everything
            // downstream — the authored grab move, eligibility, arbitration,
            // the relationship — reads one answer.
            frame.grab_pressed = true;
        }
    }
}

/// Which button a chosen verb holds down while it charges.
fn charge_gesture_of(
    verb: ambition_characters::brain::attack_kit::AttackVerb,
) -> ambition_entity_catalog::ChargeGesture {
    match verb {
        // A held Special is a charge shot. Every other verb holds Attack — a
        // smash charges on it and a basic continues a string on it — or holds
        // nothing at all, in which case the counter is zero and neither field
        // goes down.
        ambition_characters::brain::attack_kit::AttackVerb::Special => {
            ambition_entity_catalog::ChargeGesture::Special
        }
        _ => ambition_entity_catalog::ChargeGesture::Smash,
    }
}

/// Write BOTH sustains from the charge in flight. See the call site's note.
fn hold_the_committed_button(state: &FighterState, frame: &mut ActorControlFrame) {
    let holding = state.charge_hold_ticks > 0;
    let gesture = state.charge_hold_gesture;
    frame.melee_held = holding && gesture == ambition_entity_catalog::ChargeGesture::Smash;
    frame.special_held = holding && gesture == ambition_entity_catalog::ChargeGesture::Special;
}

/// Publish the decision as a structured causal fact — and render one line of
/// it when `AMBITION_FIGHTER_TRACE=1`.
///
/// It is a FACT now, for three reasons the text line could not meet:
///
/// * it is queryable. `explanation.first("fighter_decision").get("chose")`
///   is a field lookup; the same thing over stderr is a regex over prose that
///   breaks when somebody improves the wording.
/// * it correlates. A fact carries a tick, a subject and a generation, so
///   the verb this brain chose can be joined to the movement it produced and
///   the damage that followed. Two unrelated `eprintln!`s cannot be joined at
///   all.
/// * it labels a repeat. The old docstring conceded it was *"not
///   rollback-safe and does not pretend to be"* — under a rollback host a
///   resimulated frame decided again and printed again, and two identical lines
///   are indistinguishable from one decision made twice. `Execution` says which.
///
/// one authority. The stderr line is RENDERED from the fact, so the two cannot drift.
///
/// the tick is stamped by the scope owner, not by this function. A brain
/// five hops below the ECS does not know the world's clock, and a decision
/// counter guessed here would be a second clock that no other domain could join
/// against. `CausalLog::set_tick` is the one place with the answer.
fn trace_decision(
    view: ambition_characters::perception::Perceived<'_>,
    options: &ambition_characters::brain::fighter::options::OptionSet,
    frame: &ActorControlFrame,
    // Which body this is, from the snapshot's world-in port. `None` publishes an
    // unattributed fact — honest for a fixture, and useless on a stage with two
    // fighters, which is why the integration layer fills it.
    subject: Option<&str>,
    summary: DecisionSummary<'_, '_>,
) {
    static ENABLED: std::sync::LazyLock<bool> = std::sync::LazyLock::new(|| {
        std::env::var("AMBITION_FIGHTER_TRACE").is_ok_and(|value| value != "0")
    });

    #[cfg(feature = "causal")]
    let publishing = ambition_causal::recording();
    #[cfg(not(feature = "causal"))]
    let publishing = false;

    if !*ENABLED && !publishing {
        return;
    }

    let me = view.self_view;
    let offered: Vec<_> = options.movement.iter().map(|option| option.verb).collect();
    let DecisionSummary {
        situation,
        vetoed,
        chosen,
        least_bad,
        unmodelled,
        attack,
        recovery,
        recovery_move,
        proposed_routes,
    } = summary;
    // Built here rather than at the call site: the early return above means an
    // untraced decision never allocates it.
    let proposed: Vec<&str> = proposed_routes
        .iter()
        .map(|candidate| candidate.move_id.as_str())
        .collect();
    // What the recovery search DID, as three separate answers. A caller that
    // collapses them loses the interesting one: "already getting home, saved the
    // move" and "searched and found nothing" are both `recovery_move = none`, and
    // only the first is a fighter playing well.
    let recovery_searched = recovery.is_some();
    let recovery_regained = recovery.is_some_and(|verdict| verdict.regained());
    // a negative is a claim about the SEARCHER, so the search that produced
    // it is published beside it — reading `NoSupportFoundBy` as "cannot recover"
    // is the misreading `RecoveryLens`' own header spends four paragraphs on.
    let recovery_bounded_by = recovery
        .and_then(|verdict| verdict.outlook.bounded_by())
        .map(|probe| probe.policy.name);
    // the subject leads the line for the same reason the fact carries one: two
    // fighters on a stage produced two interleaved streams with nothing to tell
    // them apart, and this trace exists because reasoning about that failed.
    let line = format!(
        "[fighter{}] situation={situation:?} x={:.0} vx={:.0} ground={} phase={:?} stage={} [{:.0}..{:.0}] floor_edge={:?} terrain={} supported={} offered={:?} vetoed={:?} unmodelled={:?} chose={:?} least_bad={:?} attack={} routes={:?} recovery={} bounded_by={} emit_x={:.1}",
        match subject {
            Some(id) => format!(" {id}"),
            None => String::new(),
        },
        me.pos.x,
        me.vel.x,
        me.on_ground,
        me.phase,
        view.stage.is_known(),
        view.stage.bounds.min.x,
        view.stage.bounds.max.x,
        view.floor_edge_distance().map(|d| d.round()),
        // ⭐ WHAT THE BODY CAN SEE UNDER ITSELF. `on_ground` is KERNEL truth and
        // `terrain` is viewport-limited PERCEPTION, so the two can contradict —
        // measured on the l1 recovery fixture, 6 of 6 grounded decisions had
        // `ground=true` with `floor_edge=None`. Without these two numbers a
        // reader cannot tell "no terrain reached me at all" from "terrain
        // reached me and none of it is under my feet", and those want opposite
        // fixes.
        view.terrain.len(),
        view.supporting_floor().is_some(),
        offered,
        vetoed,
        unmodelled,
        chosen,
        least_bad,
        attack.unwrap_or("none"),
        proposed,
        match (recovery_searched, recovery_regained, recovery_move) {
            (false, _, _) => "not-searched",
            (true, true, Some(id)) => id,
            (true, true, None) => "home-already",
            (true, false, _) => "no-route-found",
        },
        recovery_bounded_by.unwrap_or("-"),
        frame.locomotion.x,
    );

    #[cfg(feature = "causal")]
    if publishing {
        use ambition_causal::{domains, CausalFact, FactDetail, SubjectKey};
        // The summary is the same line a human reads; every value a TOOL would
        // want is a field beside it, so nothing has to be parsed back out.
        //
        // The SUBJECT comes from the snapshot, because the brain cannot know
        // which body it is and must not: an unattributed decision fact cannot
        // answer "why did THIS fighter do that" the moment a second fighter is
        // on the stage, which for a fighting game is every interesting tick.
        let mut fact = CausalFact::new(
            domains::BRAIN,
            0,
            FactDetail::new(
                "fighter_decision",
                match chosen {
                    Some(verb) => format!("chose {verb:?}"),
                    None => "chose nothing — every verb was vetoed".to_string(),
                },
            ),
        )
        .field("chose", format!("{chosen:?}"))
        .field("offered", format!("{offered:?}"))
        .field("vetoed", format!("{vetoed:?}"))
        .field("vetoed_count", vetoed.len() as i64)
        // ⭐ WHY `chose` IS ALSO IN `vetoed`, when it is: either the least-bad
        // fallback fired, or an UNMODELLED verb was promoted by the caller's
        // reading of silence as safety. Two different fixes, and without these
        // two fields the fact renders them identically.
        .field("least_bad", format!("{least_bad:?}"))
        .field("unmodelled", format!("{unmodelled:?}"))
        .field("unmodelled_count", unmodelled.len() as i64)
        // L1's answer, so every other field is readable as conditional on
        // it. `first("fighter_decision")` could say which verb a brain took
        // and never which QUESTION it was answering, so the histogram this
        // instrument exists for — `Situation::Recovery → the action selected` —
        // could not be grouped at all.
        .field("situation", format!("{situation:?}"))
        // The authored move this decision presses, by id. `"none"` is a real
        // answer (no swing), distinct from a move named "none" existing.
        .field("attack", attack.unwrap_or("none").to_string())
        .field("recovery_searched", recovery_searched)
        .field("recovery_regained", recovery_regained)
        .field("recovery_move", recovery_move.unwrap_or("none").to_string())
        .field(
            "recovery_bounded_by",
            recovery_bounded_by.unwrap_or("-").to_string(),
        )
        // the PROPOSAL list, so a `recovery_move` of "none" is readable: an
        // empty list means the repertoire offered nothing, a non-empty one means
        // the kernel declined everything it was offered.
        .field("recovery_routes", format!("{proposed:?}"))
        .field("pos_x", me.pos.x)
        .field("vel_x", me.vel.x)
        .field("on_ground", me.on_ground)
        .field("phase", format!("{:?}", me.phase))
        .field("stage_known", view.stage.is_known())
        .field(
            "floor_edge_distance",
            view.floor_edge_distance().unwrap_or(f32::INFINITY),
        )
        .field("emit_locomotion_x", frame.locomotion.x);
        if let Some(subject) = subject {
            fact = fact.about(SubjectKey::Sim(subject.to_string()));
        }
        ambition_causal::record(fact);
    }

    if *ENABLED {
        eprintln!("{line}");
    }
}

/// What the foe looks like from across the stage, or `None` when there is no foe.
fn foe_sample(view: ambition_characters::perception::Perceived<'_>) -> Option<FoeSample> {
    let foe = view.nearest_hostile()?;
    let toward = foe.pos - view.self_view.pos;
    Some(FoeSample {
        attacking: matches!(
            foe.phase,
            ambition_characters::perception::BodyPhase::AttackStartup
                | ambition_characters::perception::BodyPhase::AttackActive
        ),
        on_ground: foe.on_ground,
        shielding: foe.shield_raised,
        // Positive when the foe's velocity points at me.
        closing: -(foe.vel.x * toward.x.signum()),
    })
}

/// The foe's observable choice between two samples, in §13.5's order.
fn infer_choice(previous: FoeSample, current: FoeSample) -> Choice {
    if current.attacking && !previous.attacking {
        Choice::Attack
    } else if !current.on_ground && previous.on_ground {
        Choice::Jump
    } else if current.shielding {
        Choice::Shield
    } else if current.closing > 0.0 {
        Choice::Approach
    } else if current.closing < 0.0 {
        Choice::Retreat
    } else {
        Choice::Wait
    }
}

/// Translate a movement verb into control-frame fields.
///
/// the sign comes from the perceived foe, not from the actor's facing.
/// Facing is what the body currently shows and lags a decision; the direction
/// that makes `Approach` mean approach is the one toward the thing being
/// approached.
fn apply_movement(
    verb: MovementVerb,
    view: ambition_characters::perception::Perceived<'_>,
    frame: &mut ActorControlFrame,
) {
    // nothing caught it because the conversion is a REINTERPRETATION:
    // `LocalAxes::from_vec(self.locomotion)` copies the components and renames
    // the type, so the type asserts a transform nobody performed.
    //
    // and nothing SAW it because the two conventions agree in the only
    // configuration that gets played: under screen-down gravity `side` is world
    // `+x` and `to_local` is the identity, so this change is byte-identical
    // there. It diverges exactly where this brain already reasons correctly —
    // `fighter_from_self(view, gravity_down)` builds the shadow model in the
    // gravity frame, and `is_punishable(foe, me.gravity_down)` reads it. The
    // rollout was frame-aware and the emit was not.
    let frame_axes = view.self_view.acceleration_frame();
    // `f32::signum(0.0)` is `1.0`, not `0.0` — so a delta that lies exactly along the
    // body's gravity axis (nothing to the side at all) would come back as FULL THROTTLE
    // sideways. The deadzone is the same one `smash/emit.rs::signum_or` uses.
    let side_toward = |world_delta: Vec2| {
        let side = frame_axes.to_local(world_delta).x;
        if side.abs() < 0.001 {
            0.0
        } else {
            side.signum()
        }
    };
    let toward = view
        .nearest_hostile()
        .map(|foe| side_toward(foe.pos - view.self_view.pos))
        .unwrap_or(0.0);
    frame.shield_held = false;
    // Behaviour is identical because nothing in this brain has ever written `.y` and `held` starts
    // at `neutral()`, so it was 0.0 for the frame's whole life; the point is that the rule and the
    // code now say the same thing. Facing is deliberately not cleared: which way a body looks
    // between decisions is the held intent doing its job.
    //
    // A held jump is a real input (it is what buys height), which is exactly why it has to be
    // re-stated by whichever verb is chosen rather than inherited by whichever verb is not.
    frame.jump_held = false;
    match verb {
        MovementVerb::Approach => {
            frame.locomotion = ae::LocalAxes::new(toward, 0.0);
            frame.facing = toward;
        }
        MovementVerb::Retreat => {
            frame.locomotion = ae::LocalAxes::new(-toward, 0.0);
            frame.facing = toward;
        }
        MovementVerb::Jump => {
            frame.jump_pressed = true;
            frame.jump_held = true;
        }
        MovementVerb::Dash => {
            frame.burst_pressed = true;
            frame.locomotion = ae::LocalAxes::new(toward, 0.0);
            frame.facing = toward;
        }
        MovementVerb::Dodge => {
            // THE SAME BUTTON AS `Dash`, AND THE BODY TURNS IT INTO A ROLL
            // (or an air dodge off the ground). The brain does not get to pick
            // which — `apply_dodge` claims the buffer first on any body that
            // owns the ability — so all this verb decides is the DIRECTION, and
            // the stick is what carries it: `apply_dodge` rolls along
            // `local_stick.x`, falling back to facing when the stick is neutral.
            //
            // away from a swing, into everything else, which is the whole
            // of what separates the genre's two uses of the roll. A roll is
            // i-frames plus travel: spent AWAY from an attack it is the evade,
            // spent TOWARD a standing opponent it is the approach that cannot be
            // poked out of. The read is perceivable — is anybody swinging at me
            // — so a human watching the same stage could make it too, which is
            // the no-cheat contract this brain is held to.
            let threatened = view
                .actors
                .iter()
                .any(|actor| actor.hostile_to_self && actor.alive && actor.phase.is_attacking());
            let roll = if threatened { -toward } else { toward };
            frame.burst_pressed = true;
            frame.locomotion = ae::LocalAxes::new(roll, 0.0);
            // facing tracks the FOE, not the roll. A body that rolls away
            // while turning its back would come out of the roll facing the
            // blastzone, and its next swing would point at nothing.
            frame.facing = toward;
        }
        MovementVerb::Shield => {
            frame.locomotion = ae::LocalAxes::ZERO;
            frame.shield_held = true;
        }
        MovementVerb::Blink => {
            frame.blink_pressed = true;
        }
        MovementVerb::Recover => {
            // Recover toward the stage center, resolving that world-space direction into the
            // body's local side axis, and jump for vertical gain.
            let centre = Vec2::new(
                (view.stage.bounds.min.x + view.stage.bounds.max.x) * 0.5,
                view.self_view.pos.y,
            );
            let home = side_toward(centre - view.self_view.pos);
            frame.locomotion = ae::LocalAxes::new(home, 0.0);
            frame.facing = home;
            frame.jump_pressed = true;
            frame.jump_held = true;
        }
    }
}

/// Which situation the brain last classified — read by the ladder rig and the
/// humanity checks, both of which need to know what the brain thought it was
/// doing rather than only what it emitted.
pub fn situation_of(state: &FighterState) -> Option<Situation> {
    state.perception.perceive().map(classify)
}

/// The kit an option generator needs, as the snapshot carries it.
pub type AttackKit = Vec<AttackCandidate>;

/// The utility weights a profile plays under, exposed so a fixture can assert
/// the rig uses the PROFILE's rather than a default.
pub fn weights_of(cfg: &FighterCfg) -> &UtilityWeights {
    &cfg.profile.utility_weights
}

// A CHILD of the decision module: its tests reach `FighterState`'s fields and
// the private clocks, which is the design rather than an accident.
#[cfg(test)]
#[path = "decision/tests.rs"]
mod tests;
