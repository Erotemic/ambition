//! **CAPTURE — one body holding another, as a relationship rather than a hit.**
//!
//! ```text
//! a HIT       spatial overlap → damage → knockback → over, inside one move
//! a CAPTURE   spatial acquisition → RELATIONSHIP → later moves target it → release
//! ```
//!
//! The distinction is not stylistic. It decides four things at once:
//!
//! * a shield stops a hit and does NOT stop a grab, so a grab cannot resolve as
//!   a blocked strike;
//! * a pummel affects an already-SELECTED counterpart, not everything
//!   overlapping a box;
//! * a pummel must not END the thing that lets it happen;
//! * a throw ends it, at an authored frame rather than at a button press.
//!
//! ⛔ so this is not `HitVolume { damage: 0, grab: true }`, and it is not
//! [`MovePlayback`](crate::moveset::MovePlayback) either. Those answer different
//! questions and both answers are needed at once:
//!
//! ```text
//! MovePlayback   WHICH authored technique this body is executing right now
//! CapturedBy     WHO currently holds this body
//! ```
//!
//! A pummel gets a fresh `MovePlayback` while `CapturedBy` is untouched. That
//! separation IS the architecture; everything else here follows from it.

pub mod systems;

use bevy::prelude::{Component, Entity, Message, Query};

use ambition_platformer2d_core as ae;

/// **Who is holding this body.** The one authority on a capture relationship.
///
/// ⛔ **there is deliberately no `Capturing { victim }` on the captor.** Two
/// mutable authorities for one relationship is two things to keep in agreement,
/// and nothing in the engine would notice when they stopped agreeing — the
/// failure would surface as a body held by somebody who does not think they are
/// holding anybody. To ask *"who am I holding?"*, use [`captive_of`]: a scan
/// over a handful of fighters is not a performance problem, and a wrong answer
/// would be.
///
/// ⚠ if a measured customer ever needs a captor-side projection, it should be a
/// DERIVED cache rebuilt from this, never a second thing to write.
///
/// # Why live `Entity`, not `SimId`
///
/// This is short-lived in-match state, not persistent-world identity: a capture
/// begins and ends inside one exchange and no save file needs to remember one.
/// So it holds a live handle and implements
/// [`MapEntities`](bevy::ecs::entity::MapEntities) for rollback, the same shape
/// `RidingOn` uses. A `SimId`-keyed capture would buy durability nothing wants
/// and pay a lookup every tick for it.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct CapturedBy {
    /// The body holding this one.
    pub captor: Entity,
    /// Where this body is held, in the CAPTOR's body-local frame: `+x` = the
    /// captor's committed facing, `+y` = gravity-down. Resolved against the
    /// captor's live facing and motion frame every tick, so a capture survives
    /// the captor turning around and survives arbitrary gravity.
    pub hold_offset_local: ae::Vec2,
    /// **What capture SUSPENDED and release must give back.**
    ///
    /// ⚠ not assumed to be `1.0`. Flying bodies and special movement modes
    /// already exist, and a release that wrote a constant would quietly convert
    /// a floating character into a falling one — a bug that only ever appears
    /// for the characters least likely to be tested.
    pub prior_gravity_scale: f32,
    // ⛔⛔ **`pummels_landed`, `held_for` and the escape clock LEFT THIS STRUCT
    // on 2026-08-19** — see `ambition_characters::smash_capture::SmashHoldState`.
    //
    // They were fine here while capture was being proven and they are not
    // convincing final owners, which the 2026-08-19 GPT review put plainly: a
    // radically different game may want "actor A constrains actor B" with no
    // concept of pummels, mash escape, or a four-second grab timeout. What is
    // left is the RELATION — who holds whom, where, and what physical state
    // release must give back — and every field of it is answerable without
    // knowing what genre is being played.
    //
    // ⚠ the split is not cosmetic: it is why a capture in another game does not
    // pay to rewind a pummel counter it has no rule for.
}

impl bevy::ecs::entity::MapEntities for CapturedBy {
    fn map_entities<M: bevy::ecs::entity::EntityMapper>(&mut self, mapper: &mut M) {
        self.captor = mapper.get_mapped(self.captor);
    }
}

/// **Who is `captor` holding, if anyone?** The inverse of [`CapturedBy`].
///
/// The deliberate answer to not mirroring the relation on the captor. Linear in
/// captives — of which there are at most one per captor and a handful per stage.
///
/// ⚠ **deterministic by construction**: at most one body may name a given
/// captor, so there is no iteration-order question to get wrong. The runtime
/// that establishes a capture is what upholds that, by refusing to acquire for a
/// captor that already holds somebody.
pub fn captive_of(captor: Entity, captives: &Query<(Entity, &CapturedBy)>) -> Option<Entity> {
    captives
        .iter()
        .find(|(_, held)| held.captor == captor)
        .map(|(entity, _)| entity)
}

/// **A grab's active window is asking to catch somebody this tick.**
///
/// Written every frame the window is live (see `smash_capture`'s
/// `CAPTURE_ATTEMPT`), so the handler acquires on the first frame an eligible
/// body overlaps and no-ops on the rest.
///
/// ⚠ the ruleset's authored effect key never reaches the body runtime — a Smash
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

/// **A pummel's impact frame, landing on whoever this body already holds.**
///
/// ⭐ it names no victim, and that is the point: the target was selected when
/// the capture was established, so a pummel does not reacquire anybody through
/// collision. This is the first authored technique in the codebase that targets
/// a semantic RELATIONSHIP rather than a volume.
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub struct CapturePummelRequested {
    pub captor: Entity,
    pub damage: i32,
}

/// **A throw's authored release frame.**
///
/// Damage, launch, and the END of the relationship, at one instant chosen by the
/// timeline rather than by the press that started the move.
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

    /// **THE INVERSE QUERY IS THE CAPTOR-SIDE ANSWER, AND IT IS THE ONLY ONE.**
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
        let captives = system_state.get(app.world());

        assert_eq!(captive_of(captor, &captives), Some(captive));
        assert_eq!(
            captive_of(bystander, &captives),
            None,
            "a body holding nobody was reported as a captor"
        );
    }

    /// **THE CAPTOR HANDLE SURVIVES A REWIND.**
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
        // ⚠ the "own state" this guards used to be `pummels_landed`, which left
        // for the fighter capability on 2026-08-19. The claim is unchanged —
        // remapping touches the ENTITY HANDLE and nothing else — so it is now
        // made against the fields the relation still carries.
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
