//! Thin adapter from shell-route lifecycle to contributor-neutral load presentation.

use std::collections::BTreeMap;

use ambition_game_shell::{
    AmbitionGameShellSet, ShellCommand, ShellEvent, ShellHoldId, ShellRouteCatalog,
    ShellRouteHolds, ShellRouteId, ShellRouter,
};
use bevy::prelude::{
    App, IntoScheduleConfigs, MessageReader, MessageWriter, Plugin, Res, ResMut, Resource, Update,
};

use crate::{
    LoadExperienceSpec, LoadForegroundPhase, LoadForegroundState, LoadPresentationCommand,
    LoadPresentationEvent, LoadPresentationOwnerId, LoadPresentationSet,
};

const LOAD_PRESENTATION_HOLD: &str = "ambition.load-presentation.ready-hold";

/// Shell-route-specific policy selection. The generic presentation receives
/// the selected [`LoadExperienceSpec`] in its Begin command and never reads a
/// `ShellRouteId` itself.
#[derive(Resource)]
pub struct ShellLoadPresentationCatalog {
    pub default: LoadExperienceSpec,
    pub by_route: BTreeMap<ShellRouteId, LoadExperienceSpec>,
}

impl Default for ShellLoadPresentationCatalog {
    fn default() -> Self {
        Self {
            default: LoadExperienceSpec::basic("ambition.load.basic"),
            by_route: BTreeMap::new(),
        }
    }
}

impl ShellLoadPresentationCatalog {
    pub fn for_route(&self, route: &ShellRouteId) -> &LoadExperienceSpec {
        self.by_route.get(route).unwrap_or(&self.default)
    }
}

#[derive(Clone, Debug)]
struct ActiveShellLoadPresentation {
    owner: LoadPresentationOwnerId,
    route_id: ShellRouteId,
}

#[derive(Resource, Default)]
struct ShellLoadPresentationState {
    active: Option<ActiveShellLoadPresentation>,
}

#[derive(Default)]
pub struct AmbitionLoadShellPresentationPlugin;

impl Plugin for AmbitionLoadShellPresentationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ShellLoadPresentationCatalog>()
            .init_resource::<ShellLoadPresentationState>()
            .add_systems(
                Update,
                observe_shell_waits
                    .after(AmbitionGameShellSet::Commands)
                    .before(LoadPresentationSet::Observe),
            )
            .add_systems(
                Update,
                (process_shell_presentation_events, sync_shell_hold)
                    .chain()
                    .after(LoadPresentationSet::Actions)
                    .before(AmbitionGameShellSet::Pending)
                    .before(LoadPresentationSet::Finalize),
            )
            .add_systems(
                Update,
                finalize_activated_route
                    .after(AmbitionGameShellSet::Pending)
                    .before(LoadPresentationSet::Finalize),
            );
    }
}

fn shell_owner(
    route_id: &ShellRouteId,
    load_id: &ambition_load::LoadId,
) -> LoadPresentationOwnerId {
    LoadPresentationOwnerId::new(format!("shell:{}:{}", route_id.as_str(), load_id.as_str()))
}

fn observe_shell_waits(
    mut events: MessageReader<ShellEvent>,
    catalog: Res<ShellLoadPresentationCatalog>,
    mut state: ResMut<ShellLoadPresentationState>,
    mut holds: ResMut<ShellRouteHolds>,
    mut presentation: MessageWriter<LoadPresentationCommand>,
) {
    for event in events.read() {
        let ShellEvent::WaitingForLoad { route_id, barrier } = event else {
            continue;
        };
        if let Some(previous) = state.active.take() {
            holds.release(
                &previous.route_id,
                &ShellHoldId::new(LOAD_PRESENTATION_HOLD),
            );
            presentation.write(LoadPresentationCommand::Cancel {
                owner: previous.owner,
            });
        }
        let owner = shell_owner(route_id, &barrier.load_id);
        holds.release(route_id, &ShellHoldId::new(LOAD_PRESENTATION_HOLD));
        presentation.write(LoadPresentationCommand::Begin {
            owner: owner.clone(),
            barrier: barrier.clone(),
            spec: catalog.for_route(route_id).clone(),
        });
        state.active = Some(ActiveShellLoadPresentation {
            owner,
            route_id: route_id.clone(),
        });
    }
}

fn sync_shell_hold(
    state: Res<ShellLoadPresentationState>,
    foreground: Res<LoadForegroundState>,
    mut holds: ResMut<ShellRouteHolds>,
) {
    let Some(shell_active) = state.active.as_ref() else {
        return;
    };
    let should_hold = foreground
        .active
        .as_ref()
        .filter(|active| active.owner == shell_active.owner)
        .is_some_and(|active| {
            active.should_hold_ready() || active.phase == LoadForegroundPhase::Failed
        });
    let hold_id = ShellHoldId::new(LOAD_PRESENTATION_HOLD);
    if should_hold {
        holds.hold(shell_active.route_id.clone(), hold_id);
    } else {
        holds.release(&shell_active.route_id, &hold_id);
    }
}

fn process_shell_presentation_events(
    mut events: MessageReader<LoadPresentationEvent>,
    mut state: ResMut<ShellLoadPresentationState>,
    mut router: ResMut<ShellRouter>,
    routes: Res<ShellRouteCatalog>,
    mut holds: ResMut<ShellRouteHolds>,
    mut shell: MessageWriter<ShellCommand>,
    mut presentation: MessageWriter<LoadPresentationCommand>,
) {
    for event in events.read() {
        let owner = match event {
            LoadPresentationEvent::ContinueRequested { owner }
            | LoadPresentationEvent::RetryRequested { owner, .. }
            | LoadPresentationEvent::CancelRequested { owner }
            | LoadPresentationEvent::QuitRequested { owner } => owner,
        };
        let Some(active) = state.active.clone() else {
            continue;
        };
        if &active.owner != owner {
            continue;
        }
        match event {
            LoadPresentationEvent::ContinueRequested { .. } => {
                holds.release(&active.route_id, &ShellHoldId::new(LOAD_PRESENTATION_HOLD));
            }
            LoadPresentationEvent::RetryRequested { .. } => {
                if routes
                    .get(&active.route_id)
                    .is_some_and(|route| route.preparation.is_some())
                {
                    shell.write(ShellCommand::ReplaceWith(active.route_id.clone()));
                }
            }
            LoadPresentationEvent::CancelRequested { .. } => {
                let had_active_route = router.active.is_some();
                if let Some(pending) = router.cancel_pending() {
                    holds.release(&pending.route_id, &ShellHoldId::new(LOAD_PRESENTATION_HOLD));
                }
                clear_shell_presentation(&mut state, &mut presentation);
                if !had_active_route {
                    shell.write(ShellCommand::QuitToHome);
                }
            }
            LoadPresentationEvent::QuitRequested { .. } => {
                if let Some(pending) = router.cancel_pending() {
                    holds.release(&pending.route_id, &ShellHoldId::new(LOAD_PRESENTATION_HOLD));
                }
                clear_shell_presentation(&mut state, &mut presentation);
                shell.write(ShellCommand::QuitToHome);
            }
        }
    }
}

fn finalize_activated_route(
    mut events: MessageReader<ShellEvent>,
    mut state: ResMut<ShellLoadPresentationState>,
    mut holds: ResMut<ShellRouteHolds>,
    mut presentation: MessageWriter<LoadPresentationCommand>,
) {
    for event in events.read() {
        let ShellEvent::RouteActivated(activated) = event else {
            continue;
        };
        let Some(active) = state.active.as_ref() else {
            continue;
        };
        if active.route_id != activated.route_id {
            continue;
        }
        holds.release(&active.route_id, &ShellHoldId::new(LOAD_PRESENTATION_HOLD));
        clear_shell_presentation(&mut state, &mut presentation);
    }
}

fn clear_shell_presentation(
    state: &mut ShellLoadPresentationState,
    presentation: &mut MessageWriter<LoadPresentationCommand>,
) {
    let Some(active) = state.active.take() else {
        return;
    };
    presentation.write(LoadPresentationCommand::Finish {
        owner: active.owner,
    });
}
