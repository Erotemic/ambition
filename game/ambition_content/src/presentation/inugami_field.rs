//! The Inugami's field (Jon, 2026-10-20: "a shader may be in order here"):
//! about the risen head the picture ripples and takes its red, through the
//! renderer's [`SpiritAura`] pass. Polish only: its glow and its fire are in
//! its sheet, and with screen shaders off it looks the same but for this.
//!
//! The head has a field only while it is risen: not while it sleeps in his
//! body (its `unseen` row), and not once it is dead.

use bevy::prelude::*;

use ambition_render::rendering::FeatureVisual;
use ambition_render::spirit_field::{SpiritAura, SpiritFieldPlugin};
use ambition_sim_view::{BossFrameIndex, BossRenderIndex, FeatureViewIndex};

/// The field reaches this many of its body's sizes from its middle.
const REACH: f32 = 0.85;
const STRENGTH: f32 = 0.9;
/// Its red (linear).
const TINT: Vec3 = Vec3::new(1.0, 0.22, 0.26);

/// Install the field and the system that gives the risen head its aura. An
/// app that does not draw installs nothing.
pub fn install(app: &mut App) {
    if app.is_plugin_added::<SpiritFieldPlugin>() || app.get_sub_app(bevy::render::RenderApp).is_none() {
        return;
    }
    app.add_plugins(SpiritFieldPlugin).add_systems(Update, reconcile_inugami_fields);
}

/// Make the aura of each boss visual what its boss is now: the risen Inugami
/// has one the size of its body; every other has none. Each frame, both ways.
pub fn reconcile_inugami_fields(
    mut commands: Commands,
    bosses: Res<BossRenderIndex>,
    frames: Res<BossFrameIndex>,
    features: Res<FeatureViewIndex>,
    visuals: Query<(Entity, &FeatureVisual, Option<&SpiritAura>)>,
) {
    for (entity, visual, aura) in &visuals {
        let want = bosses
            .get(&visual.id)
            .filter(|boss| boss.behavior_id == crate::bosses::inugami::INUGAMI_ID && boss.awake)
            .and_then(|_| features.get(&visual.id))
            .filter(|feature| feature.alive)
            .filter(|_| {
                let pinned = frames.get(&visual.id).and_then(|frame| frame.pinned.as_ref());
                pinned.is_none_or(|pin| pin.rows.first().is_none_or(|row| row != "unseen"))
            })
            .map(|feature| SpiritAura { radius: feature.size.max_element() * REACH, strength: STRENGTH, tint: TINT });
        match (want, aura) {
            (Some(want), Some(have)) if *have == want => {}
            (Some(want), _) => {
                commands.entity(entity).try_insert(want);
            }
            (None, Some(_)) => {
                commands.entity(entity).try_remove::<SpiritAura>();
            }
            (None, None) => {}
        }
    }
}
