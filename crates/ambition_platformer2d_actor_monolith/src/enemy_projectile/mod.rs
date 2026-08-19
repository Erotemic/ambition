//! Historical projectile-system tests, retained as a test-only namespace.
//!
//! Production has one `ambition_projectiles::LiveProjectile` occurrence family and
//! one authoritative `ProjectileSpawnRequest` road. These helpers keep the old
//! collision/routing regression suite readable without reintroducing a production
//! enemy-projectile category.

pub(crate) mod test_support;
mod tests;
