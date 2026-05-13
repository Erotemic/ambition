//! Bridge from `FeatureEvents` (the engine-side damage event bundle)
//! into the projectile system's audio / VFX / debris message writers.

use bevy::prelude::*;

use crate::fx::VfxMessage;
use crate::physics::DebrisBurstMessage;

/// Push the audio / VFX / debris cues from a `FeatureEvents` bundle
/// onto the message writers visible to the projectile system. This
/// is the projectile-side counterpart to `app::handle_feature_events`
/// (which uses the `Vec` collectors that `sandbox_update` builds);
/// keeping a small writer-shaped variant local avoids exposing those
/// collectors outside `sandbox_update`'s scope.
pub fn forward_damage_feedback(
    vfx: &mut MessageWriter<VfxMessage>,
    debris: &mut MessageWriter<DebrisBurstMessage>,
    events: &crate::features::FeatureEvents,
) {
    use crate::physics::PhysicsDebrisCue;
    for burst in &events.physics_bursts {
        let cue = match burst.cue {
            crate::features::FeaturePhysicsCue::Breakable => PhysicsDebrisCue::Breakable,
            crate::features::FeaturePhysicsCue::EnemyRagdoll => PhysicsDebrisCue::EnemyRagdoll,
            crate::features::FeaturePhysicsCue::BossRagdoll => PhysicsDebrisCue::BossRagdoll,
        };
        debris.write(DebrisBurstMessage {
            pos: burst.pos,
            cue,
        });
    }
    for &pos in &events.impacts {
        vfx.write(VfxMessage::Impact { pos });
        vfx.write(VfxMessage::Burst {
            pos,
            count: 14,
            speed: 300.0,
            color: [1.0, 0.34, 0.28, 0.88],
            kind: crate::fx::ParticleKind::Shard,
        });
        debris.write(DebrisBurstMessage {
            pos,
            cue: PhysicsDebrisCue::Impact,
        });
    }
    for &pos in &events.bursts {
        vfx.write(VfxMessage::Burst {
            pos,
            count: 16,
            speed: 230.0,
            color: [0.84, 0.95, 1.0, 0.82],
            kind: crate::fx::ParticleKind::Spark,
        });
    }
    // NPC hit / hostility barks are produced for any damage source that
    // reaches an NPC's AABB (see `apply_damage_event`). Without this loop
    // the bubbles fire from melee but not from projectiles — fireballs
    // would damage the NPC without the dialog read, which the player
    // experiences as a missing reaction.
    for bubble in &events.speech_bubbles {
        vfx.write(VfxMessage::SpeechBubble {
            pos: bubble.pos,
            text: bubble.text.clone(),
        });
    }
}
