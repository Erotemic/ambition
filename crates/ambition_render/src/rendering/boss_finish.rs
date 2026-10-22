//! What a boss's slow-motion finish looks like
//! (`ambition_boss_encounter::finish`): while the world hangs the picture
//! loses its colour but the boss, and cinema bars come in from the top and
//! the bottom of the screen. Both follow how deep in its finish the boss is
//! (`BossRenderView::finish`), so they go as the world comes back to speed.
//!
//! The drain is the void field's pass (`crate::void_field`): a wide aura on
//! the boss takes the colour out, and a ward on it keeps the boss's own. With
//! screen shaders off only the bars and the slow motion are left.

use bevy::prelude::*;

use ambition_platformer2d_shared_tangle::lifecycle::{ActiveSessionScope, SessionSpawnScope, SpawnSessionScopedExt};
use ambition_sim_view::{BossRenderIndex, FeatureViewIndex};

use crate::void_field::{VoidAura, VoidFieldPlugin, VoidWard};

/// The aura reaches past any screen; its middle drains this much.
const DRAIN_RADIUS: f32 = 8000.0;
const DRAIN: f32 = 0.9;
/// The ward keeps this many of the boss's body sizes in colour.
const WARD_REACH: f32 = 1.0;
/// Each bar comes in this far, as a part of the screen's height.
const BAR: f32 = 0.11;

/// The drain on a visual that this module put there (a void thing's own
/// aura is not this module's to take away).
#[derive(Component)]
pub struct FinishDrain;

/// A cinema bar of a finish.
#[derive(Component)]
pub struct FinishBar;

/// Install the finish's look. The drain needs the void field's pass.
pub fn install(app: &mut App) {
    if !app.is_plugin_added::<VoidFieldPlugin>() && app.get_sub_app(bevy::render::RenderApp).is_some() {
        app.add_plugins(VoidFieldPlugin);
    }
    // A finish is a named boss's: asleep where no live room's game has one,
    // after the frame that takes the drain and the bars away.
    app.add_systems(
        Update,
        (drain_around_finishing_bosses, show_finish_bars).in_set(ambition_platformer2d_shared_tangle::capability::CapabilityPresentationSet(ambition_platformer2d_shared_tangle::capability::Capability::BossEncounters)),
    );
}

/// Give each boss visual in its finish the drain about it, and take it away
/// when the finish is over.
pub fn drain_around_finishing_bosses(
    mut commands: Commands,
    // `Option`: a composition with no sim read model has no boss to finish.
    bosses: Option<Res<BossRenderIndex>>,
    features: Option<Res<FeatureViewIndex>>,
    visuals: Query<(Entity, &super::FeatureVisual, Has<FinishDrain>)>,
) {
    for (entity, visual, drained) in &visuals {
        let finish = bosses.as_deref().and_then(|bosses| bosses.get(&visual.id)).map_or(0.0, |boss| boss.finish);
        if finish > 0.0 {
            let size = features
                .as_deref()
                .and_then(|features| features.get(&visual.id))
                .map_or(100.0, |feature| feature.size.max_element());
            commands.entity(entity).try_insert((
                FinishDrain,
                VoidAura { radius: DRAIN_RADIUS, strength: DRAIN * finish },
                VoidWard { radius: size * WARD_REACH, strength: finish },
            ));
        } else if drained {
            commands.entity(entity).try_remove::<(FinishDrain, VoidAura, VoidWard)>();
        }
    }
}

/// The cinema bars, as deep as the deepest finish.
pub fn show_finish_bars(
    mut commands: Commands,
    bars: Query<Entity, With<FinishBar>>,
    // `Option`, as for the drain.
    bosses: Option<Res<BossRenderIndex>>,
    active_session: Option<Res<ActiveSessionScope>>,
) {
    for entity in &bars {
        commands.entity(entity).despawn();
    }
    let finish = bosses.iter().flat_map(|bosses| bosses.iter()).map(|(_, boss)| boss.finish).fold(0.0f32, f32::max);
    if finish <= 0.0 {
        return;
    }
    let Some(scope) = SessionSpawnScope::for_optional_active_session(active_session.as_deref()) else {
        return;
    };
    // They come in fast and go out slowly.
    let k = 1.0 - (1.0 - finish) * (1.0 - finish);
    for top in [true, false] {
        commands.spawn_session_scoped(
            scope,
            (
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    right: Val::Px(0.0),
                    top: if top { Val::Px(0.0) } else { Val::Auto },
                    bottom: if top { Val::Auto } else { Val::Px(0.0) },
                    height: Val::Percent(100.0 * BAR * k),
                    ..default()
                },
                BackgroundColor(Color::BLACK),
                ZIndex(35),
                Name::new("Boss finish bar"),
                FinishBar,
            ),
        );
    }
}
