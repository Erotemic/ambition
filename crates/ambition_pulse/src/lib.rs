//! **A shockwave pulse — and the sentinel that proves a capability is a
//! capability.**
//!
//! The mechanic is small on purpose: a body charges a pulse, fires it, and
//! nearby bodies are pushed away from it. What matters is that this crate
//! contributes ALL FOUR halves of the capability contract without editing
//! anything central:
//!
//! ```text
//! behaviour            → this crate's systems
//! + authored schema    → `pulse_schema()`, registered with the content compiler
//! + semantic action    → `PULSE_ACTION`, registered with the action registry
//! + rollback state     → `PulseCooldown`, registered through `AmbitionRollbackApp`
//! + causal facts       → `pulse_fired`, published to the causal log
//! ```
//!
//! ⚠ **the Cargo manifest is half the proof, and only half.** There is no
//! DIRECT `ambition_actors` dependency, no game and no content crate, so this
//! crate's source cannot name an actor-crate item — the mechanic defines its own
//! [`PulseBody`] and [`PulseAffected`] rather than borrowing `BodyKinematics`,
//! and the compiler enforces that rather than a comment.
//!
//! ⛔ But `ambition_actors` IS in the TRANSITIVE closure, through
//! `ambition_runtime`, and pretending otherwise would make this sentinel lie.
//! Rollback registration lives in the runtime and only a crate above it can own
//! a schema directly, so a capability that wants its own rollback state links
//! the whole simulation whether it uses it or not. That is a measured cost of
//! the seam and a row of its own — see the program doc.
//!
//! ## What is honestly still wired by hand
//!
//! The semantic action is DECLARED and cannot yet carry a device binding of its
//! own — that waits on `InputMap<SemanticAction>` (see the program doc). So a
//! consumer fires a pulse by writing [`PulseRequested`], which is what an input
//! router will do once the binding exists. The declaration is real; the last
//! wire is not, and pretending otherwise would make this sentinel lie about the
//! thing it exists to measure.

use ambition_platformer_primitives::schedule::SimScheduleExt;
use bevy::prelude::*;

mod schema;

pub use schema::{PULSE_SCHEMA, PulseProfile, PulseProfiles, pulse_schema};

/// The capability's name, used by the content compiler, the action registry and
/// the rollback owner label. One string, so a diagnostic from any of the three
/// names the same thing.
pub const PULSE_CAPABILITY: &str = "pulse";

/// **The semantic action this capability contributes.**
///
/// Registered into `ActionRegistry` beside the engine's own vocabulary. Nothing
/// in `ambition_input` knows this exists — which is the point of the row.
pub const PULSE_ACTION: ambition_input::SemanticActionDef = ambition_input::SemanticActionDef {
    id: ambition_input::SemanticActionId("pulse"),
    capability: PULSE_CAPABILITY,
    kind: ambition_input::ActionControlKind::Button,
    contexts: &[ambition_input::GAMEPLAY_CONTEXT],
    doc: "Fire a shockwave that pushes nearby bodies away",
};

/// **A body's pulse cooldown — the capability's own rollback state.**
///
/// ⚠ it MUST be rewound. A cooldown is a gate on an action, so a rewind that
/// restored the body and not the gate would let a pulse fire twice from one
/// charge on the resimulated frame and desync. This is exactly the class the
/// repo's own note names: *"a stock count that is not registered rollback state
/// un-spends itself on a rewind"*.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PulseCooldown {
    pub remaining_ticks: u32,
}

impl PulseCooldown {
    pub fn ready(self) -> bool {
        self.remaining_ticks == 0
    }
}

/// A consumer asking this body to pulse.
///
/// The seam an input router will write once `PULSE_ACTION` can carry a device
/// binding. Until then a game writes it directly, which is also how a scripted
/// sequence or an AI would.
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub struct PulseRequested {
    pub body: Entity,
}

/// What a pulse did, for anyone who wants to react to it.
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub struct PulseFired {
    pub body: Entity,
    pub pushed: usize,
}

/// A body that can be pushed by a pulse. Deliberately this crate's OWN marker:
/// a capability that keyed off `ambition_actors`' body components would have to
/// depend on it.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct PulseAffected;

/// Position and velocity, in this capability's own terms.
///
/// ⚠ **not `BodyKinematics`.** Using the actor crate's cluster would be the
/// convenient choice and would make this crate depend on it, which is the one
/// thing the sentinel exists to avoid. A capability describes what it needs;
/// the composition adapts its bodies to that.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct PulseBody {
    pub pos: ambition_engine_core::Vec2,
    pub vel: ambition_engine_core::Vec2,
}

/// Install the mechanic's behaviour.
///
/// ⛔ **it does NOT register rollback state, and that is the fix rather than an
/// omission.** The first version did, through `AmbitionRollbackApp`, and the
/// cost was a dependency on `ambition_runtime` — the whole simulation, dragged
/// into a mechanic that uses none of it, because the registration trait lives up
/// there.
///
/// It also broke the pattern the other two contributions already follow. A
/// capability OFFERS a content schema ([`pulse_schema`]) and OFFERS a semantic
/// action ([`PULSE_ACTION`]); the composition installs them, because the
/// registry belongs to whoever is composing. Rollback is the same kind of thing.
/// [`ROLLBACK_STATE`] is the offer; a host with the trait in scope installs it,
/// which is one line and is shown in this crate's tests.
#[derive(Debug, Default)]
pub struct PulsePlugin {
    /// What the composition compiled, or `None` for the built-in defaults.
    ///
    /// ⛔ **this field is the fix for an authority split the compiler program
    /// exists to prevent** (GPT 5.6, 2026-08-01, finding 2). The schema was
    /// registered, packs VALIDATED and LOWERED correctly, and the plugin then
    /// called `init_resource::<PulseProfiles>()` — the built-in defaults. So a
    /// game could author a radius, watch the compiler accept it, mount the
    /// capability, and pulse at the default radius forever. The compiler was
    /// validating content the runtime ignored, which is worse than not
    /// validating it.
    profiles: Option<PulseProfiles>,
}

impl PulsePlugin {
    /// **Mount with the profiles a compiled pack prepared.**
    ///
    /// Consumes the artifact `FacetOutcome::lower` produced — the same value the
    /// compiler validated — rather than re-reading the authored file. A second
    /// parse would be a second authority over the same bytes, which is the
    /// defect `P1e` removed from the character catalog.
    ///
    /// `Err` when the pack prepared no pulse profiles: a composition that asked
    /// for authored tuning and got none should hear about it, not silently run
    /// the defaults. That silence is precisely what this constructor replaces.
    pub fn from_prepared(
        pack: &ambition_content_pack::PreparedContentPack,
    ) -> Result<Self, PulseContentMissing> {
        let lowered = pack
            .lowered::<Vec<PulseProfile>>(&ambition_content_pack::SchemaId::new(PULSE_SCHEMA))
            .ok_or(PulseContentMissing {
                pack: pack.namespace.0.clone(),
            })?;
        Ok(Self {
            profiles: Some(PulseProfiles::from_prepared(lowered.clone())),
        })
    }
}

/// A pack that prepared no pulse profiles, named so the refusal is actionable.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PulseContentMissing {
    pub pack: String,
}

impl std::fmt::Display for PulseContentMissing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "pack '{}' prepared no `{PULSE_SCHEMA}` profiles. Either register              `ambition_pulse::pulse_schema()` and author one, or mount              `PulsePlugin::default()` and say that the defaults are intended",
            self.pack
        )
    }
}

impl std::error::Error for PulseContentMissing {}

impl Plugin for PulsePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<PulseRequested>()
            .add_message::<PulseFired>();
        // The composition's compiled profiles WIN. `init_resource` would not
        // overwrite, and that asymmetry is how the defaults used to survive.
        match self.profiles.clone() {
            Some(profiles) => app.insert_resource(profiles),
            None => app.init_resource::<PulseProfiles>(),
        };
        // **THE AUTHORITATIVE SIMULATION SCHEDULE, through the public seam.**
        //
        // ⛔ this was bare `Update`, which is the ordinary Bevy habit and is
        // exactly what this crate's own recipe tells a capability author NOT to
        // do — the worked example contradicted the documentation it exists to
        // illustrate (GPT 5.6, 2026-08-01, finding 1). Two concrete failures:
        //
        // * **fixed-tick host**: cooldowns aged once per RENDER update, so pulse
        //   timing followed the frame rate rather than the tick rate.
        // * **rollback host**: these systems are not part of what GGRS replays,
        //   so a rewind could restore `PulseCooldown` without re-running the
        //   behaviour that produced the surrounding result. Snapshotting the
        //   state does not help if the systems that move it never resimulate.
        //
        // `sim_schedule()` asks the HOST which schedule is authoritative and
        // seals the answer; nothing here names GGRS or fixed-tick.
        let sim = app.sim_schedule();
        app.add_systems(
            sim,
            // One explicit, stable phase for the whole chain: cooldown
            // progression, then action consumption, then force application.
            // `GameplayEffects` is where a gameplay consequence lands on bodies,
            // which is what a pulse is.
            (tick_pulse_cooldowns, fire_pulses)
                .chain()
                .in_set(ambition_platformer_primitives::schedule::SandboxSet::GameplayEffects),
        );
    }
}

/// **What this capability needs rewound**, named for whoever installs it.
///
/// A capability cannot register rollback state without linking the simulation,
/// so it says what it needs instead. A composition does:
///
/// ```ignore
/// use ambition_runtime::rollback::AmbitionRollbackApp;
/// app.rollback_component_clone_probed::<PulseCooldown>(
///     ambition_pulse::PULSE_CAPABILITY,
///     ambition_pulse::ROLLBACK_STATE,
///     |cooldown| u64::from(cooldown.remaining_ticks),
/// );
/// ```
///
/// ⚠ **omitting it is a desync, not a missing feature.** A cooldown is a gate
/// on an action: a rewind that restored the body and not the gate lets a pulse
/// fire twice from one charge on the resimulated frame. [`REQUIRED_ROLLBACK`]
/// is how a host finds that out at assembly rather than at a desync.
pub const ROLLBACK_STATE: &str = "pulse.cooldown";

/// **What this capability requires rewound**, for a host to check against its
/// registry.
///
/// The offer alone left a hole: nothing made a composition accept it. A host
/// closes it in one line —
///
/// ```ignore
/// let missing = registry.missing_required_state(ambition_pulse::REQUIRED_ROLLBACK);
/// assert!(missing.is_empty(), "{missing:?}");
/// ```
///
/// — and gets the `why` back rather than a bare name, because a host hitting
/// this needs to know whether it is looking at a desync or an optional extra,
/// and only the capability knows which.
pub const REQUIRED_ROLLBACK: &[ambition_engine_core::snapshot::RequiredRollbackState] =
    &[ambition_engine_core::snapshot::RequiredRollbackState {
        owner: PULSE_CAPABILITY,
        name: ROLLBACK_STATE,
        why: "a pulse cooldown that is not rewound lets the action fire twice from one charge \
              on a resimulated frame",
    }];

/// Age every cooldown by one tick.
pub fn tick_pulse_cooldowns(mut cooldowns: Query<&mut PulseCooldown>) {
    for mut cooldown in &mut cooldowns {
        cooldown.remaining_ticks = cooldown.remaining_ticks.saturating_sub(1);
    }
}

/// Fire the requested pulses, push what is in range, and explain it.
#[allow(clippy::type_complexity)]
pub fn fire_pulses(
    mut requests: MessageReader<PulseRequested>,
    mut fired: MessageWriter<PulseFired>,
    profiles: Res<PulseProfiles>,
    mut log: Option<ResMut<ambition_causal::CausalRecording>>,
    mut bodies: Query<(Entity, &mut PulseBody, Option<&mut PulseCooldown>)>,
    affected: Query<Entity, With<PulseAffected>>,
) {
    let profile = profiles.active();
    for request in requests.read() {
        // A body on cooldown asks for nothing. Reported as a FACT rather than
        // silently dropped: "I pressed it and nothing happened" is the question
        // an inspector exists to answer.
        let ready = bodies
            .get(request.body)
            .map(|(_, _, cooldown)| cooldown.is_none_or(|c| c.ready()))
            .unwrap_or(false);
        let origin = bodies
            .get(request.body)
            .map(|(_, body, _)| body.pos)
            .unwrap_or_default();
        if !ready {
            publish(
                log.as_deref_mut(),
                request.body,
                "pulse_refused",
                "on cooldown",
                0,
                &profile,
            );
            continue;
        }

        let targets: Vec<Entity> = affected.iter().collect();
        let mut pushed = 0usize;
        for target in targets {
            if target == request.body {
                continue;
            }
            let Ok((_, mut body, _)) = bodies.get_mut(target) else {
                continue;
            };
            let away = body.pos - origin;
            let distance = away.length();
            if distance > profile.radius || distance == 0.0 {
                continue;
            }
            // Linear falloff: full force at the centre, nothing at the rim.
            let scale = 1.0 - (distance / profile.radius);
            body.vel += away / distance * profile.force * scale;
            pushed += 1;
        }

        if let Ok((_, _, Some(mut cooldown))) = bodies.get_mut(request.body) {
            cooldown.remaining_ticks = profile.cooldown_ticks;
        }
        fired.write(PulseFired {
            body: request.body,
            pushed,
        });
        publish(
            log.as_deref_mut(),
            request.body,
            "pulse_fired",
            "fired",
            pushed,
            &profile,
        );
    }
}

/// The capability's causal facts.
///
/// ⚠ it publishes the PROFILE it used. "Which content supplied the active
/// value" is one of the inspector's required questions, and a pulse that felt
/// wrong is almost always a profile question rather than a code one.
fn publish(
    log: Option<&mut ambition_causal::CausalRecording>,
    body: Entity,
    kind: &'static str,
    summary: &'static str,
    pushed: usize,
    profile: &PulseProfile,
) {
    let Some(log) = log else {
        return;
    };
    if !log.is_recording() {
        return;
    }
    log.record(
        ambition_causal::CausalFact::new(
            ambition_causal::domains::MOVEMENT,
            0,
            ambition_causal::FactDetail::new(kind, summary),
        )
        // No stable subject of its own: this capability does not know how the
        // composition identifies bodies. `Unstable` says so rather than
        // pretending an entity index is an identity.
        .about(ambition_causal::SubjectKey::Unstable(body.to_bits()))
        .from_content(format!("pulse:pulse_profile/{}", profile.name))
        .field("pushed", pushed as i64)
        .field("radius", profile.radius)
        .field("force", profile.force)
        .field("cooldown_ticks", i64::from(profile.cooldown_ticks)),
    );
}

/// Everything a composition installs to get this capability, in one call.
///
/// The content schema is returned rather than registered, because the compiler's
/// registry belongs to whoever is compiling — a capability offers a schema, it
/// does not decide which pack uses it.
pub fn register_actions(
    registry: &mut ambition_input::ActionRegistry,
) -> Result<(), ambition_input::ActionConflict> {
    registry.register(PULSE_ACTION)
}

#[cfg(test)]
mod tests;
