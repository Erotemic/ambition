//! **Seating a registered character as a body.** (C4 slice 1)
//!
//! [`MatchParticipantRoster`](super::MatchParticipantRoster) has been able to say
//! WHO is in a match since §7.8, and all it could do with that was project art
//! demand. Nothing turned a participant into a body, so the closest thing to a
//! versus mode was a test that hand-assembled two fighters — which is why C4 read
//! "no versus mode in a running game; the fight is proven only in a test".
//!
//! This is the missing verb. It is deliberately NOT a new construction path:
//! seating builds the same [`ActorClusterSeed`] the room stager and the
//! programmatic spawn seam build, so a seated fighter is an ordinary actor body
//! and every system that already works on one keeps working. What seating adds is
//! the join from a `CharacterDefinition` — registered, prepared, art-demanded — to
//! a body wearing it.
//!
//! ## Why not `SpawnActorRequest`
//!
//! That seam exists and is the right one for an ENEMY: it resolves an archetype
//! out of the roster fragment by name. A match participant is a CHARACTER, and a
//! character registered only through `register_character` has no roster archetype
//! to resolve — which is exactly the population C3 exists for. Routing seating
//! through the enemy spawner would mean every fighter needs a second declaration
//! in a roster fragment, which is the duplication the character seam removed.
//!
//! ## What it does not do yet
//!
//! Only CPU participants are seated. A `Human` binding needs a slot-to-body
//! assignment (`Brain::Player(slot)`), which is the couch-versus slice and comes
//! after this one — Jon's order is CPU vs CPU, then player vs CPU, then local
//! couch, and only then netcode.

use bevy::prelude::*;

use ambition_engine_core::Vec2;

use super::{MatchParticipant, MatchParticipantRoster, PreparedCharacterRegistry};

/// Half the horizontal gap between two seated fighters, in world pixels.
///
/// Wide enough that neither starts inside the other's authored silhouette —
/// seating two bodies overlapping would resolve on the first tick and read as a
/// physics bug rather than a seating one.
const SEAT_SPREAD_PX: f32 = 96.0;

/// The body box a seated fighter gets before its character projects its own.
///
/// A placeholder ON PURPOSE, and a small one: `SpritePosedBody` replaces it from
/// the character's authored sheet within a tick, and
/// `project_prepared_character_definitions` puts the authored silhouette on top.
/// Making this generous would hide a character whose art never resolved behind a
/// plausible-looking rectangle.
const SEAT_BODY_PX: Vec2 = Vec2::new(30.0, 48.0);

/// Seat one registered character as a body at `at`, facing `facing`.
///
/// Returns `None` when the character is not in the prepared registry — seating an
/// unregistered id would produce a body wearing a character nothing can describe,
/// and the load ledger already reports unknown tokens, so this stays quiet rather
/// than becoming a second reporter of the same fact.
///
/// Facing is not decoration: a move's authored offsets are mirrored through it, so
/// a fighter looking the wrong way swings into empty space. Seating faces each
/// participant toward the centre of the stage for that reason.
#[allow(clippy::too_many_arguments)]
pub fn seat_character(
    commands: &mut Commands,
    registry: &PreparedCharacterRegistry,
    catalog: &ambition_characters::actor::character_catalog::CharacterCatalog,
    roster: &crate::features::CharacterRoster,
    character_id: &str,
    at: Vec2,
    facing: f32,
    faction: crate::combat::components::ActorFaction,
    brain: ambition_entity_catalog::placements::CharacterBrain,
) -> Option<Entity> {
    let prepared = registry.get(character_id)?;
    let aabb = ambition_engine_core::Aabb::new(at, SEAT_BODY_PX / 2.0);
    // `new_in`, not the test-only `new`: production construction never has a
    // hidden catalog fallback, and a seated fighter resolves its sprite identity
    // from the SAME App-local catalog every other spawn path uses.
    let mut seed = crate::features::ecs::actor_clusters::ActorClusterSeed::new_in(
        catalog,
        roster,
        character_id.to_string(),
        prepared.display_name.clone(),
        aabb,
        brain,
        &[],
    );
    seed.health = ambition_characters::actor::BodyHealth::new(
        ambition_characters::actor::Health::new(prepared.vitals.max_health.max(1)),
    );
    seed.kin.facing = facing;
    let (identity, disposition, combat, intent, cooldowns) =
        crate::features::ecs::enemy_component_snapshot(&seed);
    Some(
        commands
            .spawn((
                (
                    ambition_platformer_primitives::lifecycle::FeatureSimEntity,
                    crate::features::FeatureId::new(character_id),
                    ambition_engine_core::CenteredAabb::from_center_size(at, SEAT_BODY_PX),
                    seed.into_components(),
                    crate::features::MotionModel::default(),
                ),
                (identity, disposition, combat, intent, cooldowns, faction),
                // The body WEARS the character. Everything that makes it that
                // fighter rather than a generic actor follows from this one
                // component: the moveset and silhouette arrive via
                // `project_prepared_character_definitions`, and the presentation
                // source via `publish_body_presentation_sources`. Seating does not
                // insert any of them by hand — that hand projection is exactly
                // what made the old fixture prove less than it looked like.
                ambition_characters::actor::WornCharacter::new(character_id),
                ambition_characters::brain::ActorControl::default(),
                ambition_characters::actor::attack_gesture::AttackGestureState::default(),
                ambition_characters::actor::attack_gesture::AttackGestureTuning::default(),
                ambition_characters::actor::attack_gesture::ResolvedAttackGesture::default(),
            ))
            .id(),
    )
}

/// Where a participant sits and which way it looks.
///
/// Symmetric about `centre`, alternating sides, facing inward. Two participants
/// is the case that matters now and the general rule degrades sensibly: four
/// fighters get two per side, and everyone still looks at the middle.
fn seat_for(index: usize, centre: Vec2) -> (Vec2, f32) {
    let side = if index % 2 == 0 { -1.0 } else { 1.0 };
    let rank = (index / 2) as f32;
    let x = centre.x + side * (SEAT_SPREAD_PX + rank * SEAT_SPREAD_PX * 0.5);
    // Facing points back toward the centre: a left-hand seat looks right.
    (Vec2::new(x, centre.y), -side)
}

/// Marker: this roster has already been seated for this session.
///
/// Seating is a one-shot per match. Without the latch the system would re-seat
/// every tick the roster resource exists, which is a fresh pair of fighters per
/// frame — the kind of runaway that looks like a spawn bug three systems away.
#[derive(Resource, Debug, Default)]
pub struct MatchSeated(pub bool);

/// Seat every CPU participant in [`MatchParticipantRoster`], once.
///
/// Runs on the sim schedule so a seated body exists on a tick boundary like every
/// other constructed entity, rather than mid-frame where half the pipeline has
/// already run.
pub fn seat_match_participants(
    mut commands: Commands,
    roster: Option<Res<MatchParticipantRoster>>,
    registry: Option<Res<PreparedCharacterRegistry>>,
    // REQUIRED, not optional: `engine.character-authority-is-app-local` forbids
    // making the character authority optional, and it is right to. A composition
    // with no catalog must be NAMED by the capability audit, not silently seat
    // fighters that resolve their sprite identity against nothing.
    catalog: Res<ambition_characters::actor::character_catalog::CharacterCatalog>,
    archetypes: Res<crate::features::CharacterRoster>,
    geometry: Option<
        ambition_platformer_primitives::lifecycle::SessionWorldRef<
            ambition_engine_core::RoomGeometry,
        >,
    >,
    mut seated: ResMut<MatchSeated>,
) {
    if seated.0 {
        return;
    }
    let (Some(roster), Some(registry), Some(geometry)) = (roster, registry, geometry) else {
        return;
    };
    if roster.participants.is_empty() {
        return;
    }
    // The stage centre is the room's authored spawn: the one point a room
    // guarantees is standable, which is the only guarantee seating needs.
    let centre = geometry.0.spawn;
    let mut any = false;
    for (index, participant) in roster.participants.iter().enumerate() {
        let MatchParticipant {
            character,
            controller,
            ..
        } = participant;
        // CPU only for this slice. A human seat needs a slot-to-body assignment,
        // which is the next one.
        let Some(profile) = controller.brain_profile() else {
            continue;
        };
        let (at, facing) = seat_for(index, centre);
        // Alternating factions so the two sides can actually hit each other:
        // `effective_faction` refuses a strike between same-faction bodies, so a
        // roster seated all-Enemy would stand and stare.
        let faction = if index % 2 == 0 {
            crate::combat::components::ActorFaction::Player
        } else {
            crate::combat::components::ActorFaction::Enemy
        };
        if seat_character(
            &mut commands,
            &registry,
            &catalog,
            &archetypes,
            character,
            at,
            facing,
            faction,
            ambition_entity_catalog::placements::CharacterBrain::Custom(profile.to_string()),
        )
        .is_some()
        {
            any = true;
        }
    }
    if any {
        seated.0 = true;
    }
}

#[cfg(test)]
mod tests;
