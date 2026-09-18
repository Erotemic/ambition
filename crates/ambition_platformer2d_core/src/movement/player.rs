use crate::Vec2;

/// Default standing movement collider width in world pixels.
///
/// Keep this authoritative for gameplay; presentation code may render a
/// larger placeholder sprite around this body while art is still temporary.
pub const DEFAULT_PLAYER_BODY_WIDTH: f32 = 30.0;
/// Default standing movement collider height in world pixels.
pub const DEFAULT_PLAYER_BODY_HEIGHT: f32 = 48.0;

/// Default standing movement collider size.
///
/// ⚠ **THE PUBLIC SDK EXPORTS THIS AS `default_body_size`**
/// (`ambition_platformer2d::sim`), because the value is not player-specific —
/// every materialized body falls back to it. The name here is historical. ⇒ A
/// reader who greps the SDK name will not find this function, and a reader here
/// cannot tell the two are one fact, which is why the rename is written at BOTH
/// ends rather than only at the facade.
pub fn default_player_body_size() -> Vec2 {
    Vec2::new(DEFAULT_PLAYER_BODY_WIDTH, DEFAULT_PLAYER_BODY_HEIGHT)
}
