//! Poison fixture: the frame-reconstruction shapes the mechanics guard must catch.

fn knockback_frame(gravity: &GravityCtx, pos: Vec2) -> Vec2 {
    gravity.dir_at(pos)
}

fn gesture_frame(field: Option<Res<GravityField>>) -> Vec2 {
    gravity_dir_or_default(field.as_deref())
}

fn global_field(gravity: Res<GravityField>) -> Vec2 {
    gravity.dir
}
