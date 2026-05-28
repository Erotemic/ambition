use crate::engine_core::Vec2;

/// Default standing movement collider width in world pixels.
///
/// Keep this authoritative for gameplay; presentation code may render a
/// larger placeholder sprite around this body while art is still temporary.
pub const DEFAULT_PLAYER_BODY_WIDTH: f32 = 30.0;
/// Default standing movement collider height in world pixels.
pub const DEFAULT_PLAYER_BODY_HEIGHT: f32 = 48.0;

/// Default standing movement collider size.
pub fn default_player_body_size() -> Vec2 {
    Vec2::new(DEFAULT_PLAYER_BODY_WIDTH, DEFAULT_PLAYER_BODY_HEIGHT)
}
