//! Publish the damage volume of each live breakable.

use super::*;
use ambition_combat::components::*;

/// Publish damageable breakable volumes for intact hit-reactive breakables.
///
/// Breakable pogo-orbs remain damageable even though their actual damage is
/// resolved by the dedicated `HitSource::Pogo` path. Regular OnHit/Either
/// breakables participate in the default damageable => pogoable rule; pure
/// stand-to-crumble platforms opt into world rebound geometry through
/// `PogoTargetContributor` instead of pretending to be body-damage targets.
pub fn refresh_breakable_damageable_volumes(
    mut breakables: Query<(&CenteredAabb, &BreakableFeature, &mut DamageableVolumes)>,
) {
    for (aabb, feature, mut damageable) in &mut breakables {
        if feature.broken() {
            damageable.clear();
            continue;
        }
        if feature.breakable.trigger.allows_hit() || feature.breakable.pogo_refresh {
            damageable.set_single(aabb.aabb());
        } else {
            damageable.clear();
        }
    }
}
