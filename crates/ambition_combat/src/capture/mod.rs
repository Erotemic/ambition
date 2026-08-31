//! Capture is a persistent relationship between two bodies, separate from hit
//! resolution and [`MovePlayback`](crate::moveset::MovePlayback).
//!
//! Acquisition selects the captive once. Pummels target that relationship
//! without reacquiring through collision, and throws end it at their authored
//! release frame.

pub mod systems;

use bevy::prelude::{Component, Entity, Message, Query};

use ambition_platformer2d_core as ae;

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
    pub hold_offset_local: ae::Vec2,
    /// What capture SUSPENDED and release must give back.
    ///
    /// not assumed to be `1.0`.
    pub prior_gravity_scale: f32,
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
}

/// A pummel impact targeting the captive already selected by the relationship.
///
/// It carries no victim because pummels do not reacquire through collision.
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub struct CapturePummelRequested {
    pub captor: Entity,
    pub damage: i32,
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
                prior_gravity_scale: 1.0,
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
            prior_gravity_scale: 0.0,
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
        assert_eq!(
            held.prior_gravity_scale, 0.0,
            "remapping disturbed the gravity scale release must give back"
        );
    }
}
