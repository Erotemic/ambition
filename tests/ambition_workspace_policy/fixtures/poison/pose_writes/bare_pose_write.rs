//! Poison fixture: the bare pose-write shapes the authority guard must catch.

fn bare_teleport(kin: &mut BodyKinematics, target: Vec2) {
    kin.pos = target;
}

fn bare_carry(clusters: &mut BodyClustersMut<'_>, delta: Vec2) {
    clusters.kinematics.pos += delta;
    clusters.kinematics.pos -= delta;
}

fn bare_axis_write(kin: &mut BodyKinematics, floor: f32) {
    kin.pos.y = floor;
}
