//! Measure which parts of gameplay opaque UI covers hide.

use ambition_input::InputOwner;
use ambition_platformer2d_shared_tangle::gameplay_presentation::PresentationCover;
use bevy::prelude::*;
use crate::ui_surface::ParticipantSurface;

#[derive(Default)]
struct CoverageScratch {
    remaining: Vec<Rect>,
    next: Vec<Rect>,
}

#[derive(bevy::ecs::system::SystemParam)]
pub struct PresentationCoverage<'w, 's> {
    surface: ParticipantSurface<'w, 's>,
    covers: Query<'w, 's, (&'static ComputedNode, &'static UiGlobalTransform,
        Option<&'static Visibility>, Option<&'static InheritedVisibility>), With<PresentationCover>>,
    scratch: Local<'s, CoverageScratch>,
}

impl PresentationCoverage<'_, '_> {
    /// A covered viewport does not hide gameplay in another viewport.
    pub fn hides_all_gameplay(&mut self) -> bool {
        let Some(target) = self.surface.gameplay_rect(Some(InputOwner::AllParticipants)) else { return false; };
        let display_origin = self.surface.display_rect().min;
        let scratch = &mut *self.scratch;
        scratch.remaining.clear();
        scratch.remaining.push(target);
        for (node, transform, visibility, inherited) in &self.covers {
            if visibility.is_some_and(|visibility| *visibility == Visibility::Hidden)
                || inherited.is_some_and(|visibility| !visibility.get()) {
                continue;
            }
            let scale = node.inverse_scale_factor();
            let size = node.size() * scale;
            if !size.is_finite() || size.cmple(Vec2::ZERO).any() { continue; }
            let covered = Rect::from_center_size(transform.translation * scale + display_origin, size);
            scratch.next.clear();
            for piece in scratch.remaining.drain(..) {
                let min = piece.min.max(covered.min);
                let max = piece.max.min(covered.max);
                if !max.cmpgt(min).all() {
                    scratch.next.push(piece);
                    continue;
                }
                for (a, b) in [
                    (piece.min, Vec2::new(piece.max.x, min.y)),
                    (Vec2::new(piece.min.x, max.y), piece.max),
                    (Vec2::new(piece.min.x, min.y), Vec2::new(min.x, max.y)),
                    (Vec2::new(max.x, min.y), Vec2::new(piece.max.x, max.y)),
                ] {
                    if b.cmpgt(a).all() { scratch.next.push(Rect::from_corners(a, b)); }
                }
            }
            std::mem::swap(&mut scratch.remaining, &mut scratch.next);
            if scratch.remaining.is_empty() { return true; }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opaque_regions_cover_gameplay_only_when_their_union_is_whole() {
        #[derive(Resource, Default)]
        struct Covered(bool);
        let mut app = App::new();
        app.init_resource::<Covered>();
        app.add_systems(Update, |mut coverage: PresentationCoverage, mut covered: ResMut<Covered>| {
            covered.0 = coverage.hides_all_gameplay();
        });
        let cover = |world: &mut World, x, width| world.spawn((
            PresentationCover,
            ComputedNode { size: Vec2::new(width, 720.0), ..default() },
            UiGlobalTransform::from_xy(x, 360.0),
        )).id();
        app.update();
        assert!(!app.world().resource::<Covered>().0);
        cover(app.world_mut(), 320.0, 640.0);
        app.update();
        assert!(!app.world().resource::<Covered>().0);
        let right = cover(app.world_mut(), 960.0, 640.0);
        app.update();
        assert!(app.world().resource::<Covered>().0);
        app.world_mut().get_mut::<ComputedNode>(right).unwrap().size.x = 600.0;
        app.update();
        assert!(!app.world().resource::<Covered>().0, "a gap is visible gameplay");
        app.world_mut().get_mut::<ComputedNode>(right).unwrap().size.x = 640.0;
        app.world_mut().entity_mut(right).insert(Visibility::Hidden);
        app.update();
        assert!(!app.world().resource::<Covered>().0);
    }
}
