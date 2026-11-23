//! Capture is a persistent relationship between two bodies, separate from hit
//! resolution and [`MovePlayback`](crate::moveset::MovePlayback).
//!
//! Acquisition selects the captive once. Pummels target that relationship
//! without reacquiring through collision, and throws end it at their authored
//! release frame.

pub mod systems;

/// **A HIT BREAKS A GRAB — the composition slot that says a host runs that rule.**
///
/// ⛔⛤ PUBLISHED BECAUSE ITS ABSENCE WAS INVISIBLE. Measured 2026-09-12: every
/// host except `ambition_demo_smash` installed this mechanic's acquisition,
/// pummel, throw, carry, pose, tick and escape and **left out
/// [`systems::release_interrupted_captures`]**, so a hold ended only on the
/// escape timer, a throw, or a despawned captor — no damage to either fighter
/// freed the captive, for the 17 shipped movesets that author
/// `smash.capture_attempt`.
///
/// ⚠ A NAME CANNOT BE THE INSTRUMENT HERE. `System::name()` reports
/// `"<Enable the debug feature to see the name>"` in this workspace's build, so
/// a guard that filtered the schedule by system name would count ZERO forever
/// and pass as a green check that had tested nothing. A published set is a node
/// the graph can be ASKED about, and `ScheduleGraph::systems_in_set` answers
/// both *"is the rule installed"* and *"is it installed twice"*.
///
/// ⇒ It lives beside the relation rather than in a composition because the rule
/// is combat's fact; a composition supplies only the schedule and the phase.
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GrabInterruptionApplied;

/// **A HIT BEATS A GRAB ON ONE TICK — the slot acquisition runs in.**
///
/// Acquisition ends the captive's move. Before damage resolves, that took
/// away a hit the captive's move landed on the same tick, so a grab won every
/// tie with a hit. The genre's rule is the other way: an ordinary hit beats
/// an ordinary grab (S0.2). So the installer puts this set after damage
/// resolves and after [`GrabInterruptionApplied`], and acquisition refuses a
/// body a hit is moving (`BodyCombat::is_knocked`), at either end.
///
/// A set, not a system name, for the reason [`GrabInterruptionApplied`]
/// gives: the guard asks the schedule graph about this node.
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CapturesAcquired;

use bevy::prelude::{Component, Entity, Message, Query};

use ambition_platformer2d_core as ae;

/// Run condition of a capture pass that visits the holds: a body is held.
/// With none, the pass has nothing to pose, tick, sample or release.
pub fn a_capture_stands(captives: bevy::prelude::Query<(), bevy::prelude::With<CapturedBy>>) -> bool {
    !captives.is_empty()
}

/// Who is holding this body; the sole authority for the capture relationship.
///
/// The inverse is derived with [`captive_of`] rather than stored separately. A
/// live `Entity` is appropriate because captures are short-lived match state;
/// [`MapEntities`](bevy::ecs::entity::MapEntities) remaps it for rollback.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct CapturedBy {
    /// The body holding this one.
    pub captor: Entity,
    /// Where this body is held, in the CAPTOR's body-local frame: `+x` = the
    /// captor's committed facing, `+y` = gravity-down. Resolved against the
    /// captor's live facing and motion frame every tick, so a capture survives
    /// the captor turning around and survives arbitrary gravity.
    ///
    /// Measured from the captor's position, or from [`Self::hold_point`]
    /// when the hold rides one.
    pub hold_offset_local: ae::Vec2,
    /// The named point of the CAPTOR's body rig the hold rides (a jaw, a
    /// hand). The art package states the point on a joint; the captor's pose
    /// places it each tick (`BodyRigPose`), so the held body is where the
    /// captor's jaw is DRAWN, in each frame of its clip. `None`: the hold is
    /// measured from the captor's position, a deliberate root anchor.
    pub hold_point: Option<HoldPoint>,
}

/// A named point of a captor's rig, bound to the slot it had when the hold
/// was made (body anchors, A2).
///
/// The NAME is the meaning; the slot is a cache of where the name was. A rig
/// replaced in use (a redraw, a reload, a retracted rig) can give that slot to
/// another point, so the slot is used only while it still carries the name,
/// and a rig that no longer states the name ends the hold rather than holding
/// the body somewhere else ([`HoldPoint::in_rig`]).
///
/// The name is kept as a stable key (FNV-1a of the name's bytes) so the
/// relation stays `Copy` and restores identically on every peer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct HoldPoint {
    pub slot: u16,
    pub name: u64,
}

impl HoldPoint {
    /// The stable key of an attachment name.
    pub fn key(name: &str) -> u64 {
        name.bytes().fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3))
    }

    /// The point `name` of `rig`, or `None` when the rig does not state it.
    pub fn of(rig: &ambition_characters::actor::PreparedBodyRig, name: &str) -> Option<Self> {
        let slot = u16::try_from(rig.attachment_index(name)?).ok()?;
        Some(Self { slot, name: Self::key(name) })
    }

    /// This point's slot in `rig` NOW: its bound slot while that still carries
    /// the name, else the slot that does (a rig redrawn in another order),
    /// else `None` (the rig no longer states the point). The scan runs only
    /// when the bound slot stopped naming the point.
    pub fn in_rig(self, rig: &ambition_characters::actor::PreparedBodyRig) -> Option<Self> {
        let names = |slot: usize| rig.attachments().get(slot).map(|point| Self::key(&point.name));
        if names(usize::from(self.slot)) == Some(self.name) {
            return Some(self);
        }
        (0..rig.attachments().len())
            .find(|slot| names(*slot) == Some(self.name))
            .and_then(|slot| u16::try_from(slot).ok())
            .map(|slot| Self { slot, ..self })
    }
}

impl bevy::ecs::entity::MapEntities for CapturedBy {
    fn map_entities<M: bevy::ecs::entity::EntityMapper>(&mut self, mapper: &mut M) {
        self.captor = mapper.get_mapped(self.captor);
    }
}

/// Returns the captive held by `captor`, if any.
///
/// Capture acquisition guarantees at most one captive per captor, so this
/// inverse lookup has no iteration-order ambiguity.
pub fn captive_of(captor: Entity, captives: &Query<(Entity, &CapturedBy)>) -> Option<Entity> {
    captives
        .iter()
        .find(|(_, held)| held.captor == captor)
        .map(|(entity, _)| entity)
}

/// The structural law of capture at one admission cut: a body takes part in
/// at most one capture, as captor or captive.
///
/// Every ingress (a fighter's grab, a module's seize) asks this before it
/// cancels a move, refills a budget or inserts the relation. It starts from
/// the committed holds, at both ends, and reserves both ends of each edge the
/// cut admits, because `Commands` are deferred and the committed query cannot
/// see an edge this cut is about to insert. Entry policy (grounded, defense,
/// reach) stays with each ingress.
#[derive(Clone, Debug, Default)]
pub struct CaptureOccupancy(std::collections::BTreeSet<Entity>);

impl CaptureOccupancy {
    /// The bodies the committed holds already occupy.
    pub fn committed(captives: &Query<(Entity, &CapturedBy)>) -> Self {
        Self(captives.iter().flat_map(|(victim, held)| [victim, held.captor]).collect())
    }

    /// Whether `body` takes part in no capture at this cut.
    pub fn is_free(&self, body: Entity) -> bool {
        !self.0.contains(&body)
    }

    /// Occupy `body` for this cut without a relation (a parried grab).
    pub fn occupy(&mut self, body: Entity) {
        self.0.insert(body);
    }

    /// Admit `captor` holding `victim` when both are free and distinct, and
    /// occupy both. A refusal changes nothing.
    pub fn admit(&mut self, captor: Entity, victim: Entity) -> bool {
        if captor == victim || !self.is_free(captor) || !self.is_free(victim) {
            return false;
        }
        self.0.insert(captor);
        self.0.insert(victim);
        true
    }
}

/// Order competing capture proposals by their declared priority: the
/// captor's [`SimId`](ambition_platformer2d_shared_tangle::sim_id::SimId),
/// then the victim's. A body with no identity sorts last, so it loses a
/// contest rather than winning one by absence. Equal keys keep their order.
///
/// The order is a property of the world, not of message or query order, so
/// the same proposals admit the same edges on every peer and after a rewind.
pub fn sort_by_capture_priority<T>(
    proposals: &mut [T],
    pair: impl Fn(&T) -> (Entity, Entity),
    identities: &Query<&ambition_platformer2d_shared_tangle::sim_id::SimId>,
) {
    let id = |body: Entity| identities.get(body).ok();
    let rank = |a: Option<&ambition_platformer2d_shared_tangle::sim_id::SimId>,
                b: Option<&ambition_platformer2d_shared_tangle::sim_id::SimId>| match (a, b) {
        (Some(a), Some(b)) => a.cmp(b),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    };
    proposals.sort_by(|a, b| {
        let ((a_captor, a_victim), (b_captor, b_victim)) = (pair(a), pair(b));
        rank(id(a_captor), id(b_captor)).then_with(|| rank(id(a_victim), id(b_victim)))
    });
}

/// A grab's active window is asking to catch somebody this tick.
///
/// Written every frame the window is live (see `smash_capture`'s
/// `CAPTURE_ATTEMPT`), so the handler acquires on the first frame an eligible
/// body overlaps and no-ops on the rest.
///
/// the ruleset's authored effect key never reaches the body runtime — a Smash
/// adapter recognises `"smash.capture_attempt"`, hydrates its params, and writes
/// THIS. The generic runtime matches no strings.
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub struct CaptureAttemptRequested {
    /// The body attempting the capture.
    pub captor: Entity,
    /// Centre of the grab reach, captor-body-local.
    pub offset: ae::Vec2,
    /// Half-extents of the grab reach, captor-body-local.
    pub half_extents: ae::Vec2,
    /// Where a caught body will be held, captor-body-local.
    pub hold_offset: ae::Vec2,
    /// The move use whose effect asked for this grab
    /// (`ActorActionMessage::move_instance`). A catch marks that use as
    /// CONNECTED (`MovePlayback::landed_hit`), the way a throw and a strike
    /// mark theirs; `None` (a seize no move authored) marks nothing.
    pub move_instance: Option<u32>,
    /// The one body this attempt may catch, or `None` to search the reach. A
    /// strike that seizes what it struck (`ambition.combat.seize_struck`)
    /// names its victim: the strike already found it, and a box search could
    /// catch a different body. A named body is caught without the reach test
    /// and under every other rule a grab is judged by.
    pub target: Option<Entity>,
}

/// Two bodies grabbed each other on one tick, and neither grab holds.
///
/// ⭐ THE PAIR, not a winner, for the reason [`crate::clank::AttacksClanked`]
/// names its pair: a mutual grab is a tie, and a tie has no first party. Each
/// grab ends, and both bodies rebound (`rebound_from_parried_grabs`), as two
/// attacks that clank do. A rule that gave one of them the hold would treat two
/// mirrored bodies differently (Q49).
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct GrabsParried {
    /// The two bodies, ascending.
    pub owners: (Entity, Entity),
}

/// A carry: the captor takes the captive's weight and keeps hold of it.
///
/// ⛔ IT CARRIES NO VICTIM, like a pummel and unlike a grab, because the
/// relationship already selected one. And it is NOT a throw: a throw ends the
/// hold, this one changes its terms.
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub struct CaptureCarryRequested {
    pub captor: Entity,
    /// Where the captive rides from now on, captor-body-local.
    pub hold_offset: ae::Vec2,
    /// The point of the captor's rig that offset is measured from
    /// ([`CapturedBy::hold_point`]); `None`: from the captor's position.
    pub hold_point: Option<HoldPoint>,
}

/// A pummel impact targeting the captive already selected by the relationship.
///
/// It carries no victim because pummels do not reacquire through collision.
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub struct CapturePummelRequested {
    pub captor: Entity,
    pub damage: i32,
    /// The move use whose effect asked for this pummel
    /// (`ActorActionMessage::move_instance`). A pummel that lands marks that
    /// use CONNECTED, as a catch and a throw mark theirs; `None` (a module's
    /// pummel no move authored) marks nothing.
    pub move_instance: Option<u32>,
}

/// A throw's authored release frame, which damages, launches, and ends capture.
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub struct CaptureThrowRequested {
    pub captor: Entity,
    pub damage: i32,
    /// Base knockback before the victim's damage and weight apply.
    pub knockback: f32,
    /// Growth per point of the victim's accumulated damage.
    pub knockback_growth: f32,
    /// Launch direction, captor-body-local: `+x` = facing, `+y` = gravity-down.
    /// The same contract an authored `HitVolume` states, so a throw feeds the
    /// ordinary scaled-knockback road instead of a second launch engine.
    pub launch_dir: ae::Vec2,
    /// The use of the captor's move that asked for this throw, from
    /// `ActorActionMessage::move_instance`. The throw stales that move, and
    /// only when the captor's playback is still that use. `None` (a throw that
    /// no move authored) stales nothing and records nothing.
    pub move_instance: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::*;

    /// THE INVERSE QUERY IS THE CAPTOR-SIDE ANSWER, AND IT IS THE ONLY ONE.
    ///
    /// The whole argument for not mirroring the relation on the captor is that
    /// this scan is cheap and cannot disagree with itself. If it ever stopped
    /// answering, the pressure to add a `Capturing { victim }` beside
    /// `CapturedBy` would be immediate — and that second authority is the thing
    /// this design exists to avoid.
    #[test]
    fn a_captor_finds_its_captive_and_a_free_body_finds_nobody() {
        let mut app = App::new();
        let captor = app.world_mut().spawn_empty().id();
        let bystander = app.world_mut().spawn_empty().id();
        let captive = app
            .world_mut()
            .spawn(CapturedBy {
                captor,
                hold_offset_local: ae::Vec2::new(16.0, -2.0),
                hold_point: None,
            })
            .id();

        let mut system_state: bevy::ecs::system::SystemState<Query<(Entity, &CapturedBy)>> =
            bevy::ecs::system::SystemState::new(app.world_mut());
        let captives = system_state.get(app.world()).expect("capture params");

        assert_eq!(captive_of(captor, &captives), Some(captive));
        assert_eq!(
            captive_of(bystander, &captives),
            None,
            "a body holding nobody was reported as a captor"
        );
    }

    /// THE CAPTOR HANDLE SURVIVES A REWIND.
    ///
    /// bevy_ggrs destroys and recreates rollback entities, so a stored `Entity`
    /// points at nothing after a restore unless it is remapped. An unremapped
    /// captor is the worst available failure: the captive stays held by a handle
    /// that now names some other body or none, and the release path — which
    /// finds the relation through this field — cannot free it.
    #[test]
    fn the_captor_handle_is_remapped_across_a_restore() {
        let before = Entity::from_raw_u32(7).unwrap();
        let after = Entity::from_raw_u32(107).unwrap();

        /// Stands in for the restore's real mapper: every old handle becomes
        /// the one new handle, which is enough to tell "remapped" from "left
        /// alone" without depending on `EntityIndex`'s arithmetic surface.
        struct ToFixed(Entity);
        impl bevy::ecs::entity::EntityMapper for ToFixed {
            fn get_mapped(&mut self, _entity: Entity) -> Entity {
                self.0
            }
            fn set_mapped(&mut self, _source: Entity, _target: Entity) {}
        }
        use bevy::ecs::entity::MapEntities as _;

        let mut held = CapturedBy {
            captor: before,
            hold_offset_local: ae::Vec2::new(16.0, -2.0),
            hold_point: None,
        };
        held.map_entities(&mut ToFixed(after));
        assert_eq!(
            held.captor, after,
            "the captor handle was not remapped, so a restored capture names the wrong body"
        );
        // The claim is unchanged — remapping touches the ENTITY HANDLE and nothing else — so it is
        // now made against the fields the relation still carries.
        assert_eq!(
            held.hold_offset_local,
            ae::Vec2::new(16.0, -2.0),
            "remapping disturbed where the body is held"
        );
    }
}
