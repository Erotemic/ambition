//! Crushing C1 (crushing-and-obstruction-response.md): each tick's
//! obstruction facts, one per conflict a body's step reported, in identity
//! order, for the responses (C3 movers, C4 bodies) and diagnostics to read.

use bevy::prelude::*;

use ambition_platformer2d_actor::body::BodyFrameOutput;
use ambition_platformer2d_core::collision_semantics::Obstruction;
use ambition_platformer2d_shared_tangle::sim_id::SimId;

/// One body's obstruction this tick.
#[derive(Clone, Debug)]
pub struct BodyObstruction {
    pub body: Entity,
    pub obstruction: Obstruction,
}

/// This tick's obstructions, rebuilt every tick after bodies step: ordered by
/// the bodies' `SimId` (bodies without one last), then in the order each
/// body's step reported them. Derived, not state: a rewind rebuilds it.
#[derive(Resource, Clone, Debug, Default)]
pub struct TickObstructions(pub Vec<BodyObstruction>);

impl TickObstructions {
    pub fn of(&self, body: Entity) -> impl Iterator<Item = &Obstruction> {
        self.0.iter().filter(move |row| row.body == body).map(|row| &row.obstruction)
    }
}

pub(crate) fn register_rollback_state(registrar: &mut impl ambition_platformer2d_core::snapshot::RollbackRegistrar) {
    registrar.declare_rollback_derived_resource::<TickObstructions>(
        "ambition_platformer2d_runtime", "derived.tick_obstructions",
        "Cleared and rebuilt every tick from the bodies' step output (crushing C1).");
}

/// Collect the conflicts every body's step reported into this tick's list.
pub(crate) fn publish_tick_obstructions(
    bodies: Query<(Entity, &BodyFrameOutput, Option<&SimId>)>,
    mut published: ResMut<TickObstructions>,
) {
    published.0.clear();
    let mut rows: Vec<(Option<&SimId>, Entity, &BodyFrameOutput)> = bodies.iter()
        .filter(|(_, output, _)| output.reset.is_none() && !output.events.constraint_conflicts.is_empty())
        .map(|(entity, output, id)| (id, entity, output))
        .collect();
    // Identity order: by SimId, those without one after, by entity (only a
    // body no snapshot carries has none).
    rows.sort_by(|(a, ea, _), (b, eb, _)| match (a, b) {
        (Some(a), Some(b)) => a.cmp(b),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => ea.cmp(eb),
    });
    for (_, body, output) in rows {
        published.0.extend(output.events.constraint_conflicts.iter()
            .map(|conflict| BodyObstruction { body, obstruction: conflict.obstruction() }));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d_core::collision_semantics::{Axis, AxisConstraintConflict, ContactSource};

    fn conflict(min_motion: f32) -> AxisConstraintConflict {
        let block = |name: &str| ContactSource::Block { kind: ambition_platformer2d_core::BlockKind::Solid,
            id: ambition_platformer2d_core::GeoId::placement(ambition_platformer2d_core::PlacementId::new(name), 0) };
        AxisConstraintConflict { axis: Axis::Y, min_center: 10.0, max_center: 6.0, min_source: block("floor"),
            max_source: block("press"), min_motion, max_motion: 0.0 }
    }

    /// The list is in identity order whatever order the bodies spawned in,
    /// and a body with no conflict publishes nothing.
    #[test]
    fn obstructions_are_published_in_identity_order() {
        let mut app = App::new();
        app.init_resource::<TickObstructions>();
        app.add_systems(Update, publish_tick_obstructions);
        let with = |motion: f32| {
            let mut output = BodyFrameOutput::default();
            output.events.constraint_conflicts.push(conflict(motion));
            output
        };
        let late = app.world_mut().spawn((with(4.0), SimId::placement("b"))).id();
        let early = app.world_mut().spawn((with(0.0), SimId::placement("a"))).id();
        app.world_mut().spawn((BodyFrameOutput::default(), SimId::placement("c")));
        app.update();
        let rows = &app.world().resource::<TickObstructions>().0;
        assert_eq!(rows.iter().map(|row| row.body).collect::<Vec<_>>(), vec![early, late]);
        assert!(rows[0].obstruction.driver.is_none(), "a still pinch has no driver");
        assert!(rows[1].obstruction.driver.is_some(), "the closing claimant drives");
        app.update();
        assert_eq!(app.world().resource::<TickObstructions>().0.len(), 2, "rebuilt, not accumulated");
    }
}
