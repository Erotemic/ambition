//! Historical projectile-system tests, retained as a test-only namespace.
//!
//! Production has one `ambition_projectiles::LiveProjectile` occurrence family and one
//! authoritative `ProjectileSpawnRequest` road.

#[cfg(test)]
pub(crate) mod test_support;
#[cfg(test)]
mod tests;
