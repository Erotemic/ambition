//! Live-room interaction reach and body gestures.

use ambition_platformer2d_core::{AabbExt, CenteredAabb, snapshot::{SnapshotState, Reader, put_bytes}};
use bevy::prelude::*;

/// What one body's Interact press reaches. The one rule for the selection
/// (`select_interactions`, which the prompt, the door and the mount follow)
/// and for each executor's revalidation at its own cut
/// (`interact_ecs_actors_and_switches`, `open_ecs_chests`), so the prompt
/// names what the press does. Before, the prompt asked a copy with no live-room
/// check, no door rule and no facing gate (review 2026-10-08).
#[derive(Clone, Debug)]
pub struct InteractReach {
    pos: ambition_platformer2d_core::Vec2,
    facing: f32,
    /// The run axis of the body's own frame: forward for the facing gate.
    side: ambition_platformer2d_core::Vec2,
    /// The body's collision box: what a chest or a switch must touch.
    reach: ambition_platformer2d_core::Aabb,
    /// The box a body talks from ([`ambition_interaction::talk_reach`]).
    talk: ambition_platformer2d_core::Aabb,
    room: Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>,
    /// How far the body's front is from its root along `side`: its material
    /// projected on that axis, not the world-axis envelope, which a turned
    /// body would overstate.
    front_extent: f32,
    /// The admitted parts of the body when an opening cuts it: what a chest
    /// or a switch must touch then, each with the map back to the root's
    /// chart. `None`: no opening cuts it, and `reach` is its material.
    material: Option<Result<std::sync::Arc<[ambition_platformer2d_core::frame::thread::OccupiedPart]>,
        ambition_platformer2d_core::frame::thread::Refusal>>,
}

impl InteractReach {
    /// The reach of a body with these kinematics, last step and frame, in
    /// the live room `room`.
    pub fn of(
        kin: &ambition_platformer2d_core::BodyKinematics,
        step: Option<&ambition_platformer2d_core::SweepSample>,
        frame: Option<&ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame>,
        room: Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>,
        admission: Option<ambition_platformer2d_core::frame::thread::MotionAdmission<'_>>,
    ) -> Self {
        let reach = kin.aabb();
        let side = frame.map_or(ambition_platformer2d_core::Vec2::X, |frame| frame.basis().side);
        let material = admission.filter(|admission| !admission.links.is_empty())
            .and_then(|admission| match kin.occupied(admission) {
                Ok(None) => None,
                Ok(Some(parts)) => Some(Ok(parts.into())),
                Err(refusal) => Some(Err(refusal)),
            });
        Self {
            pos: kin.pos,
            facing: kin.facing,
            side,
            reach,
            // A body talks from a little farther than it touches.
            talk: ambition_interaction::talk_reach(
                reach,
                step.map_or(ambition_platformer2d_core::Vec2::ZERO, |step| step.down),
            ),
            room,
            front_extent: kin.projected_half_extent(side.as_dvec2()).map_or(reach.half_size().x, |extent| extent as f32),
            material,
        }
    }

    /// The body's collision box.
    pub fn collision_box(&self) -> ambition_platformer2d_core::Aabb {
        self.reach
    }

    /// Where the body stands.
    pub fn pos(&self) -> ambition_platformer2d_core::Vec2 {
        self.pos
    }

    /// The facing gate (Q63): a target that requires facing is reached only
    /// by a body that faces it.
    fn passes_the_facing_gate(
        &self,
        target: ambition_platformer2d_core::Vec2,
        requires_facing: bool,
    ) -> bool {
        !requires_facing || ambition_interaction::faces(self.pos, self.facing, self.side, target)
    }

    /// The box the body talks from.
    pub fn talk_box(&self) -> ambition_platformer2d_core::Aabb {
        self.talk
    }

    /// Can the press talk to a peaceful body at `aabb` in live room `room`?
    /// The door rule is not part of it: see [`Self::a_door_keeps_the_press`].
    pub fn can_talk_to(
        &self,
        room: impl FnOnce() -> Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>,
        aabb: &CenteredAabb,
        requires_facing: bool,
    ) -> bool {
        self.talk_geometry_refusal(aabb, requires_facing).is_none() && room() == self.room
    }

    /// Why the press cannot talk to a body at `aabb` in live room `room`: the
    /// first clause of [`Self::can_talk_to`] it fails, or `None`.
    pub fn refuses_talk(
        &self,
        room: Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>,
        aabb: &CenteredAabb,
        requires_facing: bool,
    ) -> Option<&'static str> {
        // OW1 cut 7b: two live rooms can hold a body at one position.
        if room != self.room {
            return Some("in another live room");
        }
        self.talk_geometry_refusal(aabb, requires_facing)
    }

    fn talk_geometry_refusal(&self, aabb: &CenteredAabb, requires_facing: bool) -> Option<&'static str> {
        if !aabb.aabb().strict_intersects(self.talk) {
            Some("out of talking reach")
        } else if !self.passes_the_facing_gate(aabb.center, requires_facing) {
            Some("not faced")
        } else {
            None
        }
    }

    /// Does the press reach a chest or a switch at `aabb` in live room `room`?
    pub fn touches(
        &self,
        room: Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>,
        aabb: &CenteredAabb,
        requires_facing: bool,
    ) -> bool {
        self.refuses_touch(room, aabb, requires_facing).is_none()
    }

    /// The press's preference for a chest or switch at `aabb` it touches
    /// ([`Self::preference`] of the target as the body meets it), or `None`
    /// when it does not touch it.
    pub fn touch_preference(
        &self,
        room: Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>,
        aabb: &CenteredAabb,
        requires_facing: bool,
    ) -> Option<f32> {
        self.touched(room, aabb, requires_facing).ok().map(|center| self.preference(center))
    }

    /// Where the body meets a target at `aabb`, in the root's chart: its
    /// centre, carried back through the first admitted part that touches it
    /// (the root's own part first), so facing and preference compare body
    /// and target in one chart.
    fn touched_at(&self, aabb: &CenteredAabb) -> Result<ambition_platformer2d_core::Vec2, &'static str> {
        match &self.material {
            None if aabb.aabb().strict_intersects(self.reach) => Ok(aabb.center),
            None => Err("not touched"),
            Some(Err(_)) => Err("the body's material was refused"),
            Some(Ok(parts)) => {
                let part = parts.iter().find(|part| part.strictly_overlaps(aabb.aabb())).ok_or("not touched")?;
                part.to_root.point(aabb.center).map_err(|_| "the body's material was refused")
            }
        }
    }

    fn touched(
        &self,
        room: Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>,
        aabb: &CenteredAabb,
        requires_facing: bool,
    ) -> Result<ambition_platformer2d_core::Vec2, &'static str> {
        if room != self.room {
            return Err("in another live room");
        }
        let center = self.touched_at(aabb)?;
        if !self.passes_the_facing_gate(center, requires_facing) {
            return Err("not faced");
        }
        Ok(center)
    }

    /// Why the press does not reach a chest or switch at `aabb`: the first
    /// clause of [`Self::touches`] it fails, or `None`.
    pub fn refuses_touch(
        &self,
        room: Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>,
        aabb: &CenteredAabb,
        requires_facing: bool,
    ) -> Option<&'static str> {
        self.touched(room, aabb, requires_facing).err()
    }

    /// How far a target at `center` is from the point the body reaches with:
    /// the front of its box, the way it faces. When several things are in
    /// reach (a person standing at a terminal), the press is for the one with
    /// the least, so the player chooses by where they stand and which way they
    /// face, and the prompt names the same one.
    pub fn preference(&self, center: ambition_platformer2d_core::Vec2) -> f32 {
        center.distance(self.front())
    }

    /// The point the body reaches with: the front of its material, the way
    /// it faces.
    pub fn front(&self) -> ambition_platformer2d_core::Vec2 {
        self.pos + self.side * (self.facing.signum() * self.front_extent)
    }

    /// One press can be for a door or for a body to talk to: the nearer of
    /// the two takes it, and a tie goes to the door. `door` is the distance
    /// to the door the body stands in, `talker` the distance to the body.
    pub fn a_door_keeps_the_press(door: Option<f32>, talker: f32) -> bool {
        door.is_some_and(|door| door <= talker)
    }
}

#[cfg(test)]
mod touch_through_openings_tests;

/// What a press does to the target it is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum InteractKind {
    Talk,
    Activate,
    Open,
}

/// One target that a press reaches: what the press does to it, its distance
/// from the point the body reaches with ([`InteractReach::preference`]), and
/// a stable identity that breaks a tie.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InteractCandidate<Id> {
    pub kind: InteractKind,
    pub preference: f32,
    pub id: Id,
}

/// The targets one press reaches, in the order the press tries them. The one
/// rule of the press and of its prompt, so the prompt names the target the
/// press is for.
///
/// - The nearest target of each kind goes first, and equal distances go to
///   the lesser `id`, so the order does not depend on query order.
/// - Bodies to talk to come before switches, and switches before chests (the
///   chest system takes only a press that nothing else took).
/// - When the nearest switch is nearer than every body to talk to, the
///   bodies drop out: the switch takes the press. A tie goes to the body.
///
/// The executor of each kind revalidates its target and can refuse it (a
/// conversation with nothing to say); the press then goes to the next target.
pub fn interact_order<Id: Ord>(candidates: impl IntoIterator<Item = InteractCandidate<Id>>) -> Vec<InteractCandidate<Id>> {
    let mut order: Vec<_> = candidates.into_iter().collect();
    order.sort_by(|a, b| {
        a.kind.cmp(&b.kind).then(a.preference.total_cmp(&b.preference)).then_with(|| a.id.cmp(&b.id))
    });
    let nearest = |kind| order.iter().find(|candidate| candidate.kind == kind).map(|candidate| candidate.preference);
    if let (Some(talk), Some(switch)) = (nearest(InteractKind::Talk), nearest(InteractKind::Activate)) {
        if switch < talk {
            order.retain(|candidate| candidate.kind != InteractKind::Talk);
        }
    }
    order
}

/// The operation one body's Interact press is selected for. Talking,
/// activating and opening are the three [`InteractKind`]s of what the press
/// reaches; boarding a mount, using the door the body stands in and getting
/// off what it rides are the contextual operations that take the press when
/// nothing in reach does. The precedence between them is stated where the
/// selection is made, not read from this order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum InteractOperation {
    Talk,
    Activate,
    Open,
    /// Board the target mount.
    Ride,
    /// Use the door the body stands in. No target body.
    Use,
    /// Get off what the body rides. No target body.
    Dismount,
}

impl From<InteractKind> for InteractOperation {
    fn from(kind: InteractKind) -> Self {
        match kind {
            InteractKind::Talk => Self::Talk,
            InteractKind::Activate => Self::Activate,
            InteractKind::Open => Self::Open,
        }
    }
}

/// What one body's press is selected for at the selection cut: the
/// operation and, when it has one, the target body or object.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InteractSelected {
    pub operation: InteractOperation,
    pub target: Option<Entity>,
}

/// Each driven body's selected interaction, rebuilt every tick by the
/// gameplay composition from the domains' own availability facts (a talker
/// with no conversation to have is not offered). The prompt draws its label
/// from it, and a contextual consumer that yields to an ordinary interaction
/// (a portal's colour toggle, a mount's boarding) asks it. A body with no
/// entry has nothing its press is for.
///
/// It is a read model, not a reservation: the operation's owner revalidates
/// its target when it acts.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct InteractSelection(pub std::collections::HashMap<Entity, InteractSelected>);

impl InteractSelection {
    /// What `body`'s press is selected for, if anything.
    pub fn of(&self, body: Entity) -> Option<InteractSelected> {
        self.0.get(&body).copied()
    }

}

/// The set that rebuilds [`InteractSelection`]. Its readers run after it.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct InteractSelectionSet;

/// Play the interact gesture on the body that ACTED.
pub fn pose_interact(
    anims: &mut Query<&mut ambition_characters::actor::BodyAnimFacts>,
    body: Entity,
    hold_secs: f32,
) {
    if let Ok(mut anim) = anims.get_mut(body) {
        anim.interact_anim_timer = hold_secs;
    }
}


/// Recoverable, occurrence-owned evidence of native activation arbitration.
#[derive(Component, Clone, Debug, Default, PartialEq, Eq)]
pub struct ActivationState { pub revision: u64, pub actor: Option<ambition_extension_sdk::ObjectRef> }
ambition_extension_sdk::port_record! { ActivationState { revision: u64, actor: Option<ambition_extension_sdk::ObjectRef> } }
impl SnapshotState for ActivationState {
    fn encode(&self, out: &mut Vec<u8>) {
        let mut bytes = Vec::new(); ambition_extension_sdk::structure::put(self, &mut bytes);
        put_bytes(out, &bytes);
    }
    fn decode(reader: &mut Reader<'_>) -> Option<Self> {
        ambition_extension_sdk::wire::decode_all(reader.bytes()?, ambition_extension_sdk::structure::read).ok()
    }
}
impl ActivationState {
    pub fn accept(&mut self, object: ambition_extension_sdk::ObjectRef, actor: Option<ambition_extension_sdk::ObjectRef>,
        room: ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance) -> Option<ActivationCommitted> {
        self.revision = self.revision.checked_add(1)?;
        self.actor = actor;
        Some(ActivationCommitted {object, actor, revision: self.revision, room})
    }
}

/// Capture provenance at application. The source need not survive notification.
#[derive(Message, Clone, Debug)]
pub struct ActivationCommitted {
    pub object: ambition_extension_sdk::ObjectRef,
    pub actor: Option<ambition_extension_sdk::ObjectRef>,
    pub revision: u64,
    pub room: ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance,
}

pub fn register_rollback_state(registrar: &mut impl ambition_platformer2d_core::snapshot::RollbackRegistrar) {
    registrar.rollback_component_canonical::<ActivationState>(env!("CARGO_PKG_NAME"), "interaction.activation");
    registrar.clear_message_on_rollback::<ActivationCommitted>(env!("CARGO_PKG_NAME"), "message.activation_committed");
}
