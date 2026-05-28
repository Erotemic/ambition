//! Sandbox-side swim adapter.
//!
//! All gameplay-meaningful water response (drowning without ability,
//! Mario-style jump→swim conversion, passive buoyancy / drag / fall
//! cap) now lives in `crate::engine_core::movement` so a single tick
//! sees consistent water state. This module is intentionally a thin
//! shim: it just owns the test fixtures we use to pin water behavior
//! end-to-end.
//!
//! Source-agnostic: the engine queries `World::water_at(player_aabb)`
//! once per simulation tick. Authoring layer (LDtk IntGrid `Water` or
//! entity `WaterVolume`) chooses which regions exist; the runtime
//! never branches on which authoring source produced a region.

#[cfg(test)]
mod tests {
    use crate::engine_core as ae;

    fn pool_world(kind: ae::WaterKind, spawn: ae::Vec2) -> ae::World {
        let mut world = ae::World::new(
            "swim_test",
            ae::Vec2::new(2000.0, 2000.0),
            spawn,
            Vec::new(),
        );
        world.water_regions.push(ae::WaterRegion::new(
            ae::Aabb::new(ae::Vec2::new(500.0, 500.0), ae::Vec2::new(400.0, 400.0)),
            kind,
            ae::WaterVolumeSpec::default(),
        ));
        world
    }

    fn scratch_with(abilities: ae::AbilitySet, spawn: ae::Vec2) -> ae::PlayerClusterScratch {
        ae::PlayerClusterScratch::from_player(&ae::Player::new_with_abilities(spawn, abilities))
    }

    /// Without the `swim` ability, contacting water triggers the same
    /// reset/respawn the existing hazard path uses.
    #[test]
    fn no_swim_ability_water_contact_triggers_reset() {
        let mut world = pool_world(ae::WaterKind::Clear, ae::Vec2::new(50.0, 50.0));
        // Force the hazard reset path to win over OOB (player parked
        // safely inside world bounds).
        world.size = ae::Vec2::new(2000.0, 2000.0);
        let mut abilities = ae::AbilitySet::sandbox_all();
        abilities.swim = false;
        let mut scratch = scratch_with(abilities, world.spawn);
        scratch.kinematics.pos = ae::Vec2::new(500.0, 500.0);
        let events = ae::update_player_simulation_scratch(
            &world,
            &mut scratch,
            ae::InputState::default(),
            0.016,
        );
        assert!(events.reset, "expected reset on water without swim");
        assert!(events.hazard, "expected hazard flag (drowned)");
        assert_eq!(scratch.kinematics.pos, world.spawn);
    }

    /// With swim, jump_pressed becomes a single upward stroke. The
    /// engine never delivers a normal jump from the same press.
    #[test]
    fn swim_ability_jump_press_becomes_upward_impulse() {
        let world = pool_world(ae::WaterKind::Clear, ae::Vec2::new(500.0, 100.0));
        let mut abilities = ae::AbilitySet::sandbox_all();
        abilities.swim = true;
        let mut scratch = scratch_with(abilities, world.spawn);
        scratch.kinematics.pos = ae::Vec2::new(500.0, 500.0);
        scratch.kinematics.vel = ae::Vec2::new(0.0, 600.0);
        let input = ae::InputState {
            jump_pressed: true,
            jump_held: true,
            control_dt: 0.016,
            ..ae::InputState::default()
        };
        // Control phase: would normally fill the jump buffer.
        ae::update_player_control_scratch(&world, &mut scratch, input, 0.016);
        // Simulation phase: the buffered jump must be consumed as a
        // swim stroke, not a normal jump.
        ae::update_player_simulation_scratch(&world, &mut scratch, input, 0.016);
        assert!(
            scratch.kinematics.vel.y < 0.0,
            "expected upward (negative) vel.y after swim stroke; got {}",
            scratch.kinematics.vel.y
        );
        // Buffer must be cleared so the same press can't fire again.
        assert_eq!(scratch.action_buffer.jump, 0.0);
    }

    /// Without a fresh press, water still applies passive buoyancy
    /// (drag) and clamps fall speed.
    #[test]
    fn swim_ability_passive_buoyancy_clamps_fall() {
        let world = pool_world(ae::WaterKind::Clear, ae::Vec2::new(500.0, 100.0));
        let mut abilities = ae::AbilitySet::sandbox_all();
        abilities.swim = true;
        let mut scratch = scratch_with(abilities, world.spawn);
        scratch.kinematics.pos = ae::Vec2::new(500.0, 500.0);
        scratch.kinematics.vel = ae::Vec2::new(40.0, 1500.0);
        let input = ae::InputState {
            control_dt: 0.016,
            ..ae::InputState::default()
        };
        ae::update_player_simulation_scratch(&world, &mut scratch, input, 0.016);
        let spec = ae::WaterVolumeSpec::default();
        assert!(
            scratch.kinematics.vel.x.abs() < 40.0,
            "expected horizontal drag in water"
        );
        assert!(
            scratch.kinematics.vel.y <= spec.max_fall_speed + 1.0,
            "fall speed must clamp; got {}",
            scratch.kinematics.vel.y
        );
    }

    /// Out-of-water frames must not register a water contact.
    #[test]
    fn out_of_water_leaves_water_contact_none() {
        let world = pool_world(ae::WaterKind::Clear, ae::Vec2::new(50.0, 100.0));
        let mut abilities = ae::AbilitySet::sandbox_all();
        abilities.swim = true;
        let mut scratch = scratch_with(abilities, world.spawn);
        scratch.kinematics.pos = ae::Vec2::new(50.0, 50.0); // outside the pool
        let input = ae::InputState {
            control_dt: 0.016,
            ..ae::InputState::default()
        };
        ae::update_player_simulation_scratch(&world, &mut scratch, input, 0.016);
        assert!(scratch.env_contact.water.is_none());
    }
}
