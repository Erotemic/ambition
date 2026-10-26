//! Static authored room-feature spawn helpers.
//!
//! These functions stay family-specific so adding an authored static
//! feature remains "add a RoomSpec Vec + add one loop in spawn.rs".

use bevy::prelude::*;
use ambition_platformer2d_shared_tangle::lifecycle::RoomVisual;
use ambition_combat::components::{CenteredAabb, FeatureId, FeatureName};
use ambition_combat::hazard_runtime::HazardFeature;
use ambition_combat::hazard_runtime::HazardRuntime;
use ambition_entity_catalog::placements::PlacementSchema;
use ambition_platformer2d_shared_tangle::construction::RootScope;
use ambition_platformer2d_shared_tangle::lifecycle::FeatureSimEntity;
use bevy::prelude::Name;

fn damage_volume_from_authored(
    authored: &ambition_platformer2d_world::rooms::Authored<
        ambition_platformer2d_world::rooms::HazardVolumeSpec,
    >,
) -> ambition_combat::DamageVolume {
    let mut damage = ambition_combat::Damage::new(
        authored.payload.damage,
        authored.payload.kind,
        authored.payload.team,
    );
    damage.knockback = ambition_platformer2d_core::Vec2::new(
        authored.payload.knockback[0],
        authored.payload.knockback[1],
    );
    damage.hitstop_seconds = authored.payload.hitstop_seconds;
    ambition_combat::DamageVolume {
        id: authored.id.clone(),
        aabb: authored.aabb,
        damage,
        respawn: authored.payload.respawn,
        path_id: authored.payload.path_id.clone(),
        motion: authored.payload.motion.clone(),
        enabled: authored.payload.enabled,
    }
}

fn interaction_kind_from_spec(
    kind: &ambition_platformer2d_world::rooms::InteractionKindSpec,
) -> ambition_interaction::InteractionKind {
    match kind {
        ambition_platformer2d_world::rooms::InteractionKindSpec::Door { target } => {
            ambition_interaction::InteractionKind::Door {
                target: target.clone(),
            }
        }
        ambition_platformer2d_world::rooms::InteractionKindSpec::Npc {
            character_id,
            dialogue_id,
            patrol_radius,
            patrol_path_id,
            brain_override,
        } => ambition_interaction::InteractionKind::Npc {
            character_id: character_id.clone(),
            dialogue_id: dialogue_id.clone(),
            patrol_radius: *patrol_radius,
            patrol_path_id: patrol_path_id.clone(),
            brain_override: brain_override.clone(),
        },
        ambition_platformer2d_world::rooms::InteractionKindSpec::Chest => {
            ambition_interaction::InteractionKind::Chest
        }
        ambition_platformer2d_world::rooms::InteractionKindSpec::Pickup => {
            ambition_interaction::InteractionKind::Pickup
        }
        ambition_platformer2d_world::rooms::InteractionKindSpec::Breakable => {
            ambition_interaction::InteractionKind::Breakable
        }
        ambition_platformer2d_world::rooms::InteractionKindSpec::Custom(value) => {
            ambition_interaction::InteractionKind::Custom(value.clone())
        }
    }
}

pub(crate) fn interactable_from_authored(
    authored: &ambition_platformer2d_world::rooms::Authored<
        ambition_platformer2d_world::rooms::InteractableSpec,
    >,
) -> ambition_interaction::Interactable {
    ambition_interaction::Interactable {
        id: authored.id.clone(),
        prompt: authored.payload.prompt.clone(),
        aabb: authored.aabb,
        kind: interaction_kind_from_spec(&authored.payload.kind),
        requires_facing: authored.payload.requires_facing,
    }
}

#[cfg(feature = "portal")]
fn portal_color_from_spec(
    color: ambition_platformer2d_world::rooms::PortalChannelColorSpec,
) -> ambition_portal2d::PortalChannelColor {
    match color {
        ambition_platformer2d_world::rooms::PortalChannelColorSpec::Purple => {
            ambition_portal2d::PortalChannelColor::Purple
        }
        ambition_platformer2d_world::rooms::PortalChannelColorSpec::Yellow => {
            ambition_portal2d::PortalChannelColor::Yellow
        }
        ambition_platformer2d_world::rooms::PortalChannelColorSpec::Teal => {
            ambition_portal2d::PortalChannelColor::Teal
        }
        ambition_platformer2d_world::rooms::PortalChannelColorSpec::Red => {
            ambition_portal2d::PortalChannelColor::Red
        }
        ambition_platformer2d_world::rooms::PortalChannelColorSpec::Green => {
            ambition_portal2d::PortalChannelColor::Green
        }
        ambition_platformer2d_world::rooms::PortalChannelColorSpec::Magenta => {
            ambition_portal2d::PortalChannelColor::Magenta
        }
        ambition_platformer2d_world::rooms::PortalChannelColorSpec::Cyan => {
            ambition_portal2d::PortalChannelColor::Cyan
        }
        ambition_platformer2d_world::rooms::PortalChannelColorSpec::Rose => {
            ambition_portal2d::PortalChannelColor::Rose
        }
        ambition_platformer2d_world::rooms::PortalChannelColorSpec::Indexed(n) => {
            ambition_portal2d::PortalChannelColor::Indexed(n)
        }
    }
}

/// Populate one authored hazard onto a root the construction executor
/// allocated (the placement plan-row shape).
pub(crate) fn spawn_hazard_into(
    scope: &mut RootScope,
    authored: &ambition_platformer2d_world::rooms::Authored<
        ambition_platformer2d_world::rooms::HazardVolumeSpec,
    >,
    paths: &[(String, ambition_platformer2d_core::KinematicPath)],
) {
    let hazard = HazardRuntime::new_with_paths(
        authored.id.clone(),
        authored.name.clone(),
        authored.aabb,
        damage_volume_from_authored(authored),
        paths,
    );
    scope.insert_room_in_session((
            Name::new(format!("Feature hazard: {}", authored.name)),
            FeatureSimEntity,
            RoomVisual,
            FeatureId::new(authored.id.clone()),
            FeatureName::new(authored.name.clone()),
            CenteredAabb::from_center_size(hazard.pos, hazard.size),
            HazardFeature::new(hazard),
    ));
}

pub(crate) fn lower_hazard_placement(
    record: &ambition_platformer2d_world::placements::PlacementRecord,
    ctx: &mut ambition_platformer2d_construction::placements::LoweringCtx<'_, '_, '_>,
) {
    let PlacementSchema::Hazard(spec) = &record.schema else {
        return;
    };
    let authored = ambition_platformer2d_world::rooms::Authored {
        id: record.id.as_str().to_string(),
        name: record.name.clone(),
        aabb: record.aabb,
        payload: ambition_platformer2d_world::rooms::HazardVolumeSpec {
            damage: spec.damage,
            knockback: spec.knockback,
            kind: spec.kind,
            team: spec.team,
            hitstop_seconds: spec.hitstop_seconds,
            respawn: spec.respawn,
            path_id: spec.path_id.clone(),
            motion: None,
            enabled: true,
        },
    };
    spawn_hazard_into(&mut ctx.scope.reborrow(), &authored, ctx.paths);
}

/// Whether a placement counts against the measurement actor cap.
///
/// ⛔⛔ EXHAUSTIVE ON PURPOSE. The check used to sit at the top of
/// `lower_interactable_placement`, before the kind was known, so a cap of 16 in
/// a room with doors and chests omitted whichever placements came after the
/// sixteenth — cast or furniture. The hall contains nothing but `NpcSpawn`,
/// which is the only reason its curve measured what it claimed to.
///
/// A new interaction kind is a compile error here rather than a silent change to
/// what a scaling experiment is varying.
pub(crate) fn counts_against_the_actor_cap(
    kind: &ambition_platformer2d_world::rooms::InteractionKindSpec,
) -> bool {
    use ambition_platformer2d_world::rooms::InteractionKindSpec as Kind;
    match kind {
        Kind::Npc { .. } => true,
        Kind::Door { .. }
        | Kind::Chest
        | Kind::Pickup
        | Kind::Breakable
        | Kind::Custom(_) => false,
    }
}

pub(crate) fn lower_interactable_placement(
    record: &ambition_platformer2d_world::placements::PlacementRecord,
    ctx: &mut ambition_platformer2d_construction::placements::LoweringCtx<'_, '_, '_>,
) {
    let PlacementSchema::Interactable(spec) = &record.schema else {
        return;
    };
    // The scaling-curve population cap is no longer applied here: a refused
    // NPC used to be refused by THIS function, after the plan had frozen its
    // row and allocated its root. `RoomFeatureConstructionPlan::prepare` spends
    // the quota before `plan_room`, so a refused NPC never reaches a lowering.
    // `counts_against_the_actor_cap` below still says which kinds count.
    let authored = ambition_platformer2d_world::rooms::Authored {
        id: record.id.as_str().to_string(),
        name: record.name.clone(),
        aabb: record.aabb,
        payload: spec.clone(),
    };
    let fate = ctx.facts.npc_fate(&authored.id);
    ambition_platformer2d_actor_spawn::spawn_interactable_into(
        &mut ctx.scope.reborrow(),
        &ctx.context.characters,
        &ctx.context.sheets,
        &ctx.context.prepared,
        // ⭐ THE CONVERSION HAPPENS HERE NOW, where it always belonged: this file
        // both defines `interactable_from_authored` and calls the primitive, so
        // the primitive was reaching up for a function sitting beside its own
        // caller. It takes the runtime component and the authored name; the
        // authored SPEC does not cross the layer any more.
        &interactable_from_authored(&authored),
        &authored.name,
        ctx.paths,
        &ctx.context.forced_brains,
        fate,
    );
}

#[cfg(feature = "portal")]
pub(crate) fn lower_portal_placement(
    record: &ambition_platformer2d_world::placements::PlacementRecord,
    ctx: &mut ambition_platformer2d_construction::placements::LoweringCtx<'_, '_, '_>,
) {
    let PlacementSchema::Portal(schema) = &record.schema else {
        return;
    };
    // Reconstruct the runtime-facing spec: the face center is the record's
    // authored footprint center (the converter set `pos = box center`), and the
    // Tier-0 mirror carries the axis-aligned normal / link / half-length.
    let center = ambition_platformer2d_core::Vec2::new(
        (record.aabb.min.x + record.aabb.max.x) * 0.5,
        (record.aabb.min.y + record.aabb.max.y) * 0.5,
    );
    let spec = ambition_platformer2d_world::rooms::PortalSpec {
        id: record.id.as_str().to_string(),
        name: record.name.clone(),
        color: schema.color,
        pos: center,
        normal: ambition_platformer2d_core::Vec2::new(schema.normal[0], schema.normal[1]),
        link: schema.link.clone(),
        half_length: schema.half_length,
    };
    spawn_portal_into(&mut ctx.scope.reborrow(), &spec);
}

#[cfg(feature = "portal")]
pub(crate) fn spawn_portal_into(
    scope: &mut RootScope,
    spec: &ambition_platformer2d_world::rooms::PortalSpec,
) {
    let half_length = spec.half_length.unwrap_or(ambition_portal2d::PORTAL_OPENING_HALF);
    scope.insert_room_in_session((
            Name::new(format!("Portal ({}): {}", spec.color.name(), spec.name)),
            ambition_portal2d::PlacedPortal::fixed(
                portal_color_from_spec(spec.color).channel(),
                spec.pos,
                spec.normal,
                half_length,
            ),
    ));
    if let Some(link) = &spec.link {
        scope.insert(ambition_portal2d::PortalLink(ambition_portal2d::link_hash(
            link,
        )));
    }
}

#[cfg(test)]
mod actor_cap_selects_actors_tests {
    //! The measurement cap must remove CAST and leave furniture alone.
    //!
    //! ⛔⛔ **THE DEFECT THIS PINS SELECTED THE RIGHT THING BY LUCK.**
    //! `admit_actor()` was called from the top of `lower_interactable_placement`,
    //! before the placement's kind was known, so an "actor cap" of n omitted
    //! whichever placements came after the nth — doors and chests included. It
    //! measured what it claimed to only because `hall_of_characters` contains
    //! nothing but `NpcSpawn`; any room with furniture would have lost furniture.

    use super::counts_against_the_actor_cap;
    use ambition_platformer2d_world::rooms::InteractionKindSpec as Kind;

    fn npc() -> Kind {
        Kind::Npc {
            character_id: Some("goblin".into()),
            dialogue_id: None,
            patrol_radius: 0.0,
            patrol_path_id: None,
            brain_override: None,
        }
    }

    /// A cap of one drops the SECOND NPC and nothing else; uncapped, the same
    /// room loses nobody. The quota is the lowering's own (`ActorAdmission`),
    /// so this no longer races a process-global and can run beside its siblings.
    #[test]
    fn a_cap_of_one_omits_the_second_npc_and_keeps_every_prop() {
        use ambition_characters::actor::{ActorAdmission, AuthoredPopulationCap};

        let room = [
            ("Door", Kind::Door { target: None }),
            ("NPC A", npc()),
            ("NPC B", npc()),
            ("Chest", Kind::Chest),
        ];
        let survivors_under = |admission: &ActorAdmission| -> Vec<&str> {
            room.iter()
                .filter(|(_, kind)| !counts_against_the_actor_cap(kind) || admission.admit_actor())
                .map(|(name, _)| *name)
                .collect()
        };

        let capped = ActorAdmission::new(AuthoredPopulationCap::capped_at(1));
        assert_eq!(
            survivors_under(&capped),
            vec!["Door", "NPC A", "Chest"],
            "a cap of 1 must drop the SECOND NPC and nothing else"
        );

        // Premise guard: uncapped, the same room loses nobody.
        let uncapped = ActorAdmission::default();
        assert_eq!(
            survivors_under(&uncapped),
            vec!["Door", "NPC A", "NPC B", "Chest"]
        );
    }

    #[test]
    fn only_npcs_count_against_it() {
        assert!(counts_against_the_actor_cap(&npc()));
        for kind in [
            Kind::Door { target: None },
            Kind::Chest,
            Kind::Pickup,
            Kind::Breakable,
            Kind::Custom("switch".into()),
        ] {
            assert!(
                !counts_against_the_actor_cap(&kind),
                "{kind:?} is furniture; an actor cap must not remove it"
            );
        }
    }
}

#[cfg(all(test, feature = "portal"))]
mod authored_channel_range_tests {
    /// The authoring parser and the portal crate agree where the link
    /// channels start, so no authored channel name is an end of a link pair.
    #[test]
    fn authoring_stops_where_the_link_channels_start() {
        use ambition_platformer2d_world::rooms::PortalChannelColorSpec as Spec;
        assert_eq!(Spec::FIRST_LINK_CHANNEL, ambition_portal2d::LINK_CHANNEL_FIRST);
        assert_eq!(Spec::from_name("c127"), Some(Spec::Indexed(127)));
        assert_eq!(Spec::from_name("c128"), None);
        assert_eq!(Spec::from_name("c255"), None);
        assert_eq!(ambition_portal2d::PortalChannelColor::from_name("c129"), None);
    }
}
