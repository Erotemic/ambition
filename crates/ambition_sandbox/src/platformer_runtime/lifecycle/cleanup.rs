use bevy::prelude::*;

/// Despawn a scoped entity without encoding the scope in the caller.
///
/// This small helper gives cleanup call sites a runtime-owned verb to grow from.
/// Presentation/physics adapters can still wrap it when they need extra teardown
/// before despawning a room-scoped entity.
pub fn despawn_scoped_entity(commands: &mut Commands, entity: Entity) {
    commands.entity(entity).despawn();
}
