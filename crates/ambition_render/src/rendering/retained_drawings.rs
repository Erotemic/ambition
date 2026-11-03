//! Retained gizmo buffers with native room or view ownership.

use ambition_platformer2d_shared_tangle::lifecycle::{
    InRoomInstance, LiveRoomInstance, LiveSessionScope, SessionScopedEntity,
};
use ambition_sim_view::PresentedForView;
use bevy::prelude::*;

/// The live room or local view that owns a drawing.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DrawingScope {
    Room(LiveRoomInstance),
    View(Entity),
}

/// Each owner keeps its own entities, handles and reusable vertex buffers.
#[derive(bevy::ecs::system::SystemParam)]
pub struct RetainedDrawings<'w, 's, Owner: Component + Default> {
    commands: Commands<'w, 's>,
    drawings: Query<
        'w,
        's,
        (
            Entity,
            &'static DrawingScope,
            &'static bevy::gizmos::retained::Gizmo,
        ),
        With<Owner>,
    >,
    assets: ResMut<'w, Assets<bevy::gizmos::GizmoAsset>>,
    session: LiveSessionScope<'w, 's>,
    used: Local<'s, Vec<DrawingScope>>,
}

impl<Owner: Component + Default> RetainedDrawings<'_, '_, Owner> {
    /// Draw each scope once, then call `finish` to retire unused drawings.
    pub fn draw(&mut self, scope: DrawingScope, draw: impl FnOnce(&mut bevy::gizmos::GizmoAsset)) {
        self.used.push(scope);
        if let Some((entity, _, gizmo)) = self
            .drawings
            .iter()
            .find(|(_, current, _)| **current == scope)
        {
            if let Some(mut picture) = self.assets.get_mut(&gizmo.handle) {
                picture.clear();
                draw(&mut picture);
                return;
            }
            self.commands.entity(entity).despawn();
        }
        let mut picture = bevy::gizmos::GizmoAsset::new();
        draw(&mut picture);
        let mut root = self.commands.spawn((
            Owner::default(),
            scope,
            bevy::gizmos::retained::Gizmo {
                handle: self.assets.add(picture),
                ..default()
            },
            Name::new(std::any::type_name::<Owner>()),
        ));
        match scope {
            DrawingScope::Room(room) => {
                root.insert(InRoomInstance(room));
            }
            DrawingScope::View(view) => {
                root.insert(PresentedForView(view));
            }
        }
        if let Some(session) = self.session.get() {
            root.insert(SessionScopedEntity(session));
        }
    }

    /// Retire drawings not used in this pass.
    pub fn finish(&mut self) {
        for (entity, scope, _) in &self.drawings {
            if !self.used.contains(scope) {
                self.commands.entity(entity).despawn();
            }
        }
        self.used.clear();
    }
}
