//! The authored attack-volume SEAM (E2): combat asks "what convex hit
//! polygon did the artist author for this body's `animation` clip?" without
//! naming the sprite-metadata pipeline.
//!
//! The sprite layer (`character_sprites`) installs its resolver at
//! runtime-assembly time (`install_authored_attack_volumes` — the
//! `install_enemy_roster`/`install_world_manifest` seam contract: process
//! global, first install wins, set before any strike resolves). Uninstalled
//! (headless combat tests, minimal fixtures) every lookup is `None`, which is
//! exactly the "no authored row" fallback the strike paths already handle —
//! the synthetic spec volume.

use std::sync::OnceLock;

use ambition_engine_core as ae;

/// Resolver signature: `(sprite_character_id, animation clip, body pos,
/// collision size, facing, gravity_dir) -> authored volume`.
/// `sprite_character_id = None` means the controllable player manifest root.
pub type AuthoredAttackVolumeFn =
    fn(Option<&str>, &str, ae::Vec2, ae::Vec2, f32, ae::Vec2) -> Option<ae::CombatVolume>;

static RESOLVER: OnceLock<AuthoredAttackVolumeFn> = OnceLock::new();

/// Install the sprite layer's resolver. First install wins; later calls are
/// ignored (the standard install-seam contract).
pub fn install_authored_attack_volumes(resolver: AuthoredAttackVolumeFn) {
    let _ = RESOLVER.set(resolver);
}

/// Resolve the authored attack volume for a body's clip, if a resolver is
/// installed AND the clip has an authored row. `None` ⇒ the caller's
/// synthetic/spec fallback.
pub fn authored_attack_volume(
    sprite_character_id: Option<&str>,
    animation: &str,
    body_pos: ae::Vec2,
    collision: ae::Vec2,
    facing: f32,
    gravity_dir: ae::Vec2,
) -> Option<ae::CombatVolume> {
    RESOLVER.get().and_then(|resolve| {
        resolve(
            sprite_character_id,
            animation,
            body_pos,
            collision,
            facing,
            gravity_dir,
        )
    })
}
