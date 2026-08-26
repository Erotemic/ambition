//! Unified body kinematics for every controllable platformer body.
//!
//! Systems that hold multiple mutable [`BodyKinematics`] queries must prove
//! them disjoint with marker filters (`With<PlayerEntity>`, `With<ActorConfig>`,
//! `With<BossConfig>`, plus `Without<...>` guards where needed). Do that with
//! filters, never by re-splitting the component.

// TODO(compat-remove): migrate callers to `ambition_platformer2d_core::BodyKinematics`, then
// remove this path-preservation re-export.
pub use ambition_platformer2d_core::BodyKinematics;

use bevy::prelude::*;

/// Marks the single body whose position drives the room's live gravity
/// resolution (the active player). The runtime's `resolve_active_gravity`
/// queries `(&BodyKinematics, With<PrimaryBody>)` so it stays content-free; the
/// host (`ambition_platformer2d_actor_monolith`) adds this marker to its primary player entity.
///
/// Distinct from [`crate::markers::PrimaryPlayer`]: `PrimaryBody` is the
/// gravity-relevant body, `PrimaryPlayer` is the presentation/HUD-followed
/// player. The spawn bundle attaches both to the same entity today, but gravity
/// filters only on `PrimaryBody` so it never depends on the player-specific
/// marker.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct PrimaryBody;

/// Emitted the frame a mount dies and its rider dismounts (the
/// `(dead-mount, still-mounted)` dissolution the mount coupling enforces).
/// Carries both entities so a consumer can react to either side.
///
/// This is a body FACT crossing out of the mount coupling — deliberately NOT
/// routed through the `EncounterGate` script bus (that channel is
/// script-vocabulary). The boss-encounter bridge subscribes to turn it into a
/// `mount_died` external phase trigger — the boss whose mount died fights on
/// foot in an authored mini-phase (ADR 0020; Q19). Any other system may
/// subscribe to the same message later without touching this one.
///
/// it lives HERE, below the domains, because two of them share it. The
/// writer is the mount coupling in the actor monolith and the reader is
/// `ambition_boss_encounter`; a message owned by one of the two would make the
/// other depend on it for a type carrying nothing but a pair of entities. Same
/// shape, and the same reason, as `FeatureInteractionSet` being put here so a
/// carved module could still name the ordering it participates in.
#[derive(Message, Clone, Copy, Debug)]
pub struct MountDied {
    pub mount: Entity,
    pub rider: Entity,
}

/// HOW HEAVY THIS BODY IS, and `1.0` is the reference.
///
/// ⭐⭐ IT LIVES HERE FOR THE REASON `MountDied` DOES, one type up: two domains
/// share it. The WRITER is the character runtime's physical baseline — mass
/// arrives with a character's authored vitals — and the READER is the mount
/// coupling's mass-weighted centre of gravity. A physics fact owned by one
/// mechanic makes every other consumer depend on that mechanic to say how heavy
/// something is.
///
/// ⛔ IT USED TO LIVE IN `features::ecs::mount`, and 27 references outside that
/// module spelled it `crate::features::Mass` — a generic fact wearing one
/// mechanic's address. ⇒ imported, never re-exported, exactly as `MountDied`'s
/// own note demands: a `pub use` would let callers keep the old spelling and
/// hide whose type it is.
#[derive(Component, Clone, Copy, Debug)]
pub struct Mass(pub f32);

impl Default for Mass {
    /// The reference body. ⛔ NOT zero — mass is a divisor in the mount pair's
    /// centre of gravity, and a default that made a body weightless would move
    /// that centre onto whichever body forgot to author one.
    fn default() -> Self {
        Self(1.0)
    }
}

/// THIS BODY IS SOLID TO OTHER BODIES THAT ARE ALSO SOLID.
///
/// presence is the whole opt-in. A body without this component is not resisted and does not
/// resist, and the movement kernel resolves it byte for byte as it did before body contact existed
/// (`ambition_platformer2d_core::movement::body_contact`).
///
/// it is not jostle, and it must not be renamed to that. A platform
/// fighter grants it to its cast and calls the result jostle; a co-op platformer
/// might grant it so two partners cannot stand at the same point on a switch.
/// The mechanism is *one body's motion constrained by the bodies it is touching*
/// and nothing about that sentence is a genre.
///
/// the number is a KNOB because the GAMES differ. Smash-likes let fighters squeeze past
/// each other; a beat-em-up may want a wall.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct BodyContact {
    /// How hard OTHER bodies resist this one, `0.0` (not at all) to `1.0` (a
    /// solid wall). See `BodyContactField::resistance`.
    pub resistance: f32,
}

impl BodyContact {
    /// The value a platform fighter wants: two fighters walking into each other
    /// stall where they meet, and a determined one still squeezes past.
    pub const FIRM: Self = Self { resistance: 0.85 };
}

impl Default for BodyContact {
    fn default() -> Self {
        Self::FIRM
    }
}

/// EVERY SOLID BODY'S CONTACT BOX, SAMPLED ONCE BEFORE ANY OF THEM MOVES.
///
/// the snapshot is the fairness argument, not an optimisation. Two
/// bodies resolved in sequence against each other's LIVE poses would each see
/// the other somewhere different — the first at its entry pose, the second at
/// the first's already-integrated one — so whichever the query yielded first
/// would win the contest. Under rollback that is a desync; on a couch it is one
/// player being harder to push than the other for no reason anybody authored.
///
/// grounded bodies only, first slice. An airborne fighter passing over
/// another one is not in its way, and STANDING on a body is `footstool`, which
/// already exists and means something else.
#[derive(Resource, Default, Clone, Debug)]
pub struct BodyContactSnapshot {
    bodies: Vec<(
        Entity,
        ambition_platformer2d_core::movement::BodyContactBlocker,
        f32,
    )>,
}

impl BodyContactSnapshot {
    pub fn clear(&mut self) {
        self.bodies.clear();
    }

    /// the VELOCITY is not decoration. It is what lets two bodies closing
    /// on one gap divide it instead of each spending all of it; see
    /// `constrain_motion`. Sampled from the same pre-integration pass as the
    /// pose, because a split derived from two different instants is the
    /// order-dependence this snapshot exists to remove.
    pub fn push(
        &mut self,
        body: Entity,
        contact_box: ambition_platformer2d_core::Aabb,
        velocity: ambition_platformer2d_core::Vec2,
        resistance: f32,
    ) {
        self.bodies.push((
            body,
            ambition_platformer2d_core::movement::BodyContactBlocker::new(contact_box, velocity),
            resistance,
        ));
    }

    pub fn is_empty(&self) -> bool {
        self.bodies.is_empty()
    }

    pub fn len(&self) -> usize {
        self.bodies.len()
    }

    /// THIS BODY'S CONTACT FIELD: the boxes of every OTHER solid body, and
    /// this body's own resistance to them.
    ///
    /// a body that is not in the snapshot gets an INERT field, which is
    /// the whole opt-in expressed once. It is not in the snapshot because it
    /// carries no [`BodyContact`], or because it is airborne — and neither of
    /// those needs a second question at the call site.
    ///
    /// a caller-owned scratch buffer, not a returned `Vec`. This runs per
    /// body per tick inside the integrator; allocating there would be a
    /// per-frame allocation for a capability most bodies do not have.
    pub fn field_for<'s>(
        &self,
        body: Entity,
        out: &'s mut Vec<ambition_platformer2d_core::movement::BodyContactBlocker>,
    ) -> ambition_platformer2d_core::movement::BodyContactField<'s> {
        out.clear();
        let Some((own, resistance)) = self
            .bodies
            .iter()
            .find(|(entity, _, _)| *entity == body)
            .map(|(_, blocker, resistance)| (*blocker, *resistance))
        else {
            return ambition_platformer2d_core::movement::BodyContactField::NONE;
        };
        out.extend(
            self.bodies
                .iter()
                .filter(|(other, _, _)| *other != body)
                .map(|(_, blocker, _)| *blocker),
        );
        // this body's own snapshot velocity travels with the field, so
        // both halves of a contacting pair divide one gap by the same two
        // numbers. See `constrain_motion`.
        ambition_platformer2d_core::movement::BodyContactField::moving(
            out,
            resistance,
            own.entry_velocity,
        )
    }
}
