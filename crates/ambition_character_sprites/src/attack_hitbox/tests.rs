use super::*;

/// Ambition's shipped catalog, parsed straight from the content file.
///
/// The same fixture shape the actor crate uses for its own catalog/sprite
/// conformance tests (`character_roster.rs` there, and four more `include_str!`
/// sites beside it): a test wants REAL authored render sizes, and reading them
/// out of the checked-in data is cheaper and more honest than a dependency on
/// the content crate for a `#[cfg(test)]` helper.
fn catalog() -> CharacterCatalog {
    CharacterCatalog::from_data(
        ambition_characters::actor::character_catalog::parse_catalog(include_str!(
            "../../../../game/ambition_content/assets/data/character_catalog.ron"
        )),
    )
}

fn collision() -> ae::Vec2 {
    ae::Vec2::new(30.0, 48.0)
}

/// Screen-down gravity (`(0,1)`) — the upright reference frame.
fn down() -> ae::Vec2 {
    ae::Vec2::new(0.0, 1.0)
}

fn player_box(facing: f32) -> ae::Aabb {
    player_attack_hitbox_world(
        &Default::default(),
        &catalog(),
        "attack_side",
        ae::Vec2::new(0.0, 0.0),
        collision(),
        facing,
        down(),
    )
    .expect("player_robot_v3/attack_side has an authored manifest hitbox")
    .bounds()
}

/// This pins that covariance: the hitbox offset under gravity `g` is the screen-down offset rotated
/// into `g`'s frame.
#[test]
fn attack_hitbox_covaries_with_gravity_like_the_slash_vfx() {
    let body = ae::Vec2::new(100.0, 100.0);
    let center = |g: ae::Vec2| {
        let b = player_attack_hitbox_world(
            &Default::default(),
            &catalog(),
            "attack_side",
            body,
            collision(),
            1.0,
            g,
        )
            .expect("attack_side authored")
            .bounds();
        (b.min + b.max) * 0.5
    };
    let down_off = center(down()) - body;
    for g in [
        ae::Vec2::new(0.0, -1.0), // screen-up
        ae::Vec2::new(1.0, 0.0),  // screen-right
        ae::Vec2::new(-1.0, 0.0), // screen-left
    ] {
        let off = center(g) - body;
        let expected = ae::AccelerationFrame::new(g).to_world(down_off);
        assert!(
            (off - expected).length() < 1.0,
            "gravity {g:?}: hitbox offset {off:?} should be the down offset \
             {down_off:?} rotated into the gravity frame ({expected:?}) — \
             the box must track gravity like the slash VFX",
        );
    }
}

#[test]
fn player_attack_side_reaches_forward_starts_in_body_and_is_tall() {
    let body_right = collision().x * 0.5; // +15
    let aabb = player_box(1.0);
    // Reaches well forward, PAST the body, to surround the slash effect.
    assert!(
        aabb.max.x > body_right + collision().x,
        "hitbox should reach well forward of the body (max.x {} > {})",
        aabb.max.x,
        body_right + collision().x
    );
    // Starts a bit INSIDE the body (back edge left of the body's right edge),
    // not disjoint in front — the authored hull begins within the player.
    assert!(
        aabb.min.x < body_right,
        "hitbox should start inside the body (min.x {} < {})",
        aabb.min.x,
        body_right
    );
    // At least as tall as the player body.
    let height = aabb.max.y - aabb.min.y;
    assert!(
        height >= collision().y,
        "hitbox should be at least body-height ({height} >= {})",
        collision().y
    );
}

#[test]
fn player_attack_side_mirrors_with_facing() {
    let body_left = -collision().x * 0.5; // -15
    let aabb = player_box(-1.0);
    // Left-facing reaches well forward to the LEFT, past the body.
    assert!(
        aabb.min.x < body_left - collision().x,
        "left-facing hitbox should reach forward on the LEFT (min.x {} < {})",
        aabb.min.x,
        body_left - collision().x
    );
}

#[test]
fn player_attack_side_is_an_authored_convex_blade() {
    // The robot's attack_side authors a poly (blade arc), so the player
    // slash resolves a Convex volume — not a box.
    let vol = player_attack_hitbox_world(
        &Default::default(),
        &catalog(),
        "attack_side",
        ae::Vec2::ZERO,
        collision(),
        1.0,
        down(),
    )
    .expect("attack_side authored");
    assert!(
        matches!(vol, ae::CombatVolume::Convex { .. }),
        "expected a Convex blade, got {vol:?}"
    );
}

#[test]
fn actor_attack_hitbox_resolves_an_authored_enemy_blade() {
    // The robot enemy (character_id "robot") authors an `attack_side` hitbox
    // in its sheet, so the actor-neutral path resolves a real box instead of
    // the hardcoded fallback — the unification payoff: an enemy swings the
    // authored blade you see in `debug-hitboxes`, not magic numbers.
    let aabb = actor_attack_hitbox_world(
        &Default::default(),
        &catalog(),
        "robot",
        "attack_side",
        ae::Vec2::new(0.0, 0.0),
        collision(),
        1.0,
        down(),
    );
    assert!(
        aabb.is_some(),
        "robot/attack_side should resolve an authored manifest hitbox"
    );
}

#[test]
fn actor_attack_hitbox_is_none_for_unknown_character() {
    assert!(actor_attack_hitbox_world(
        &Default::default(),
        &catalog(),
        "definitely_not_a_character",
        "attack_side",
        ae::Vec2::ZERO,
        collision(),
        1.0,
        down(),
    )
    .is_none());
}

/// The seam-facing resolver resolves the REAL authored player blade for
/// `attack_side` (the assertion the combat-side moveset test delegates
/// here — combat tests the seam with a fixture; the DATA lives with the
/// sprites).
#[test]
fn seam_resolver_resolves_the_authored_player_blade() {
    let volume = authored_attack_volume_resolver(
        &Default::default(),
        &catalog(),
        None,
        "attack_side",
        ae::Vec2::new(30.0, 48.0),
        None,
    );
    assert!(
        matches!(volume, Some(ae::CombatVolume::Convex { .. })),
        "the player manifest authors a convex attack_side blade, got {volume:?}"
    );
}

/// A LEFT-DRAWN fighter's forward swings land in front of her.
///
/// Pointed Polygon's art is drawn facing left, so her authored polys sit at
/// `x < feet_x` — and every consumer that mirrored by `facing` alone put them
/// behind her at both facings. `air_back` is the control: it is authored on the
/// other side of the feet on purpose, so a fix that simply negated everything
/// would show up here as a back-air that hits forward.
///
/// ⚠ Her sheet is GENERATED and gitignored, so this SKIPS on a checkout without
/// the art rather than passing vacuously — the skip is loud on purpose.
#[test]
fn a_left_drawn_fighters_forward_swings_land_in_front_of_her() {
    let catalog = catalog();
    if catalog.get("pointed_polygon").is_none()
        || actor_attack_hitbox_local(
            &Default::default(),
            &catalog,
            "pointed_polygon",
            "jab",
            collision(),
            None,
        )
        .is_none()
    {
        eprintln!(
            "SKIPPED: pointed_polygon's generated sheet is not on disk; \
             regenerate the sprites to exercise this"
        );
        return;
    }
    let reach = |animation: &str| {
        let bounds = actor_attack_hitbox_local(
            &Default::default(),
            &catalog,
            "pointed_polygon",
            animation,
            collision(),
            None,
        )
        .unwrap_or_else(|| panic!("pointed_polygon/{animation} authors a hitbox"))
        .bounds();
        // Body-LOCAL, so `+x` is already "toward the swing's forward" whatever
        // way she happens to be facing — no facing to get wrong.
        (bounds.min.x + bounds.max.x) * 0.5
    };
    for forward in ["jab", "attack_side", "smash_forward", "dash_attack", "air_forward"] {
        assert!(
            reach(forward) > 0.0,
            "{forward} must land in FRONT of her, resolved centre x = {}",
            reach(forward)
        );
    }
    assert!(
        reach("air_back") < 0.0,
        "air_back is authored behind her and must stay there, resolved centre x = {}",
        reach("air_back")
    );
}
