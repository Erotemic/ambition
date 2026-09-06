//! Small capability-integration sentinel: charge a pulse, fire it, and push
//! nearby bodies away.
//!
//! The crate owns the capability's behavior, authored schema, semantic action,
//! rollback state, and causal facts without a direct actor-monolith dependency.
//! `PulseBody`/`PulseAffected` keep the example independent of actor-domain types.
//! The action is declared, and since 2026-08-28 a registered action CAN carry a
//! device binding: a composition puts the registry-minted key in
//! `ambition_input::ProviderBindings`, and a press comes back as
//! `SemanticActionPressed`. What the composition still owns is the last hop —
//! which seat drives which body — because that is the one fact this crate refuses
//! to know. `PulseRequested` is unchanged and stays the seam either way: a
//! scripted sequence and an AI write it the same way a press does.

use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt;
use bevy::prelude::*;

mod schema;

pub use schema::{pulse_schema, PulseProfile, PulseProfiles, PULSE_SCHEMA};

/// The capability's name, used by the content compiler, the action registry and
/// the rollback owner label. One string, so a diagnostic from any of the three
/// names the same thing.
pub const PULSE_CAPABILITY: &str = "pulse";

/// The semantic action this capability contributes.
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

/// A body's pulse cooldown — the capability's own rollback state.
///
/// it MUST be rewound. A cooldown is a gate on an action, so a rewind that
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
/// The seam an input router writes — `PULSE_ACTION` can carry a device binding
/// now, so a composition maps `SemanticActionPressed { id: "pulse" }` onto this
/// for the body that seat drives. A game may also write it directly, which is how
/// a scripted sequence or an AI reaches the same mechanic.
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
/// a capability that keyed off `ambition_platformer2d_actor_monolith`' body components would have to
/// depend on it.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct PulseAffected;

/// Position and velocity, in this capability's own terms.
///
/// not `BodyKinematics`. Using the actor crate's cluster would be the
/// convenient choice and would make this crate depend on it, which is the one
/// thing the sentinel exists to avoid. A capability describes what it needs;
/// the composition adapts its bodies to that.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct PulseBody {
    pub pos: ambition_platformer2d_core::Vec2,
    pub vel: ambition_platformer2d_core::Vec2,
}

/// Install the mechanic's behavior.
///
/// The capability offers content, semantic action, and rollback declarations;
/// the host composition installs each declaration into the registries it owns.
#[derive(Debug, Default)]
pub struct PulsePlugin {
    /// What the composition compiled, or `None` for the built-in defaults.
    ///
    /// this field is the fix for an authority split the compiler program
    /// exists to prevent. The schema was
    /// registered, packs VALIDATED and LOWERED correctly, and the plugin then
    /// called `init_resource::<PulseProfiles>()` — the built-in defaults. So a
    /// game could author a radius, watch the compiler accept it, mount the
    /// capability, and pulse at the default radius forever. The compiler was
    /// validating content the runtime ignored, which is worse than not
    /// validating it.
    profiles: Option<PulseProfiles>,
}

impl PulsePlugin {
    /// Mount with the profiles a compiled pack prepared.
    ///
    /// Consumes the artifact `FacetOutcome::lower` produced — the same value the compiler validated
    /// — rather than re-reading the authored file.
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
            "pack '{}' prepared no `{PULSE_SCHEMA}` profiles. Either register              `capability_demo::pulse_schema()` and author one, or mount              `PulsePlugin::default()` and say that the defaults are intended",
            self.pack
        )
    }
}

impl std::error::Error for PulseContentMissing {}

impl Plugin for PulsePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<PulseRequested>()
            .add_message::<PulseFired>();
        match self.profiles.clone() {
            Some(profiles) => app.insert_resource(profiles),
            None => app.init_resource::<PulseProfiles>(),
        };
        // THE AUTHORITATIVE SIMULATION SCHEDULE, through the public seam.
        //
        // this was bare `Update`, which is the ordinary Bevy habit and is
        // exactly what this crate's own recipe tells a capability author NOT to
        // do — the worked example contradicted the documentation it exists to
        // illustrate. Two concrete failures:
        //
        // * fixed-tick host: cooldowns aged once per RENDER update, so pulse
        //   timing followed the frame rate rather than the tick rate.
        // * rollback host: these systems are not part of what GGRS replays,
        //   so a rewind could restore `PulseCooldown` without re-running the
        //   behaviour that produced the surrounding result. Snapshotting the
        //   state does not help if the systems that move it never resimulate.
        let sim = app.sim_schedule();
        app.add_systems(
            sim,
            // One explicit, stable phase for the whole chain: cooldown
            // progression, then action consumption, then force application.
            // `GameplayEffects` is where a gameplay consequence lands on bodies,
            // which is what a pulse is.
            (tick_pulse_cooldowns, fire_pulses)
                .chain()
                .in_set(ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhaseMonolith::GameplayEffects),
        );
    }
}

/// What this capability needs rewound, named for whoever installs it.
///
/// A capability cannot register rollback state without linking the simulation,
/// so it says what it needs instead. A composition does:
///
/// ```ignore
/// use ambition_platformer2d_rollback_ggrs::AmbitionRollbackApp;
/// app.rollback_component_clone_probed::<PulseCooldown>(
///     capability_demo::PULSE_CAPABILITY,
///     capability_demo::ROLLBACK_STATE,
///     |cooldown| u64::from(cooldown.remaining_ticks),
/// );
/// ```
///
/// omitting it is a desync, not a missing feature. A cooldown is a gate
/// on an action: a rewind that restored the body and not the gate lets a pulse
/// fire twice from one charge on the resimulated frame. [`REQUIRED_ROLLBACK`]
/// is how a host finds that out at assembly rather than at a desync.
pub const ROLLBACK_STATE: &str = "pulse.cooldown";

/// What this capability requires rewound, for a host to check against its
/// registry.
///
/// The offer alone left a hole: nothing made a composition accept it. A host
/// closes it in one line —
///
/// ```ignore
/// let missing = registry.missing_required_state(capability_demo::REQUIRED_ROLLBACK);
/// assert!(missing.is_empty(), "{missing:?}");
/// ```
///
/// — and gets the `why` back rather than a bare name, because a host hitting
/// this needs to know whether it is looking at a desync or an optional extra,
/// and only the capability knows which.
pub const REQUIRED_ROLLBACK: &[ambition_platformer2d_core::snapshot::RequiredRollbackState] = &[
    ambition_platformer2d_core::snapshot::RequiredRollbackState {
        owner: PULSE_CAPABILITY,
        name: ROLLBACK_STATE,
        why: "a pulse cooldown that is not rewound lets the action fire twice from one charge \
              on a resimulated frame",
    },
    // `fire_pulses` mutates `PulseBody::vel` — that is the mechanic's entire observable effect
    // — so a rewind that restored the cooldown and not the push would resimulate from a body
    // that is still moving from a pulse it is about to fire again. A contract that names only
    // the cheap half is worse than none: the check passes and the desync remains.
    ambition_platformer2d_core::snapshot::RequiredRollbackState {
        owner: PULSE_CAPABILITY,
        name: BODY_ROLLBACK_STATE,
        why: "a pulse pushes bodies by changing their velocity; a rewind that does not restore \
              that velocity resimulates from a body still carrying the old push",
    },
];

/// The pushed body's state — the other half of what a rewind must restore.
///
/// a composition that adapts its own bodies to [`PulseBody`] should register
/// whichever component is AUTHORITATIVE for that motion, under this name. What
/// must not happen is registering neither because the sentinel only mentioned
/// the cooldown.
pub const BODY_ROLLBACK_STATE: &str = "pulse.body";

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
/// it publishes the PROFILE it used. "Which content supplied the active
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
