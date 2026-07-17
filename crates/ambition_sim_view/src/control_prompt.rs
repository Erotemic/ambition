//! `ControlPrompt` — the read-model of "what does each on-screen control do
//! right now, and what is it called," for whatever currently owns input.
//!
//! This is the observation boundary the touch overlay (and any future prompt
//! surface) reads instead of reaching into the sim heart. It is rebuilt once
//! per tick in the sim tail from the controlled subject's
//! [`ActorActionScheme`] — so possessing a different body swaps the labels,
//! and the prompt can never advertise an action the body's scheme doesn't
//! carry.
//!
//! Menu / dialogue contexts publish an explicit context with no gameplay
//! entries today; their command labels (Equip / Use / Back / Advance) arrive
//! with the menu providers in P4. Per-slot glyphs (the physical binding) land
//! with the `ActiveBindings` source in P1/P5; the touch overlay keeps its own
//! glyph subtitle in the meantime, so this model is label-first.

use ambition_characters::action_scheme::ActorActionScheme;
use ambition_entity_catalog::action_scheme::{ControlSlot, VisualId};
use ambition_platformer_primitives::markers::{ControlledSubject, PlayerEntity, PrimaryPlayer};
use ambition_platformer_primitives::schedule::GameMode;
use bevy::prelude::*;

/// Who currently owns input — the source of the prompt's entries.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ControlContextKind {
    /// A controlled character in gameplay; entries are its action scheme.
    Gameplay,
    /// A menu owns input; entries are the menu's commands (filled in P4).
    Menu,
    /// Dialogue owns input; entries are advance / choose / close (filled in P4).
    Dialogue,
    /// No controllable subject / nothing to prompt.
    #[default]
    Empty,
}

/// One control's current meaning: the slot it sits on, its player-facing
/// label, and an optional visual. Ordered within [`ControlPrompt::entries`] by
/// the scheme's canonical slot order.
#[derive(Clone, Debug, PartialEq)]
pub struct PromptEntry {
    pub slot: ControlSlot,
    pub label: String,
    pub visual: Option<VisualId>,
}

/// The published prompt the on-screen buttons render. A plain-data snapshot
/// (no `Entity` borrows), rebuilt each tick like every other `SimView` fact.
#[derive(Resource, Clone, Debug, Default)]
pub struct ControlPrompt {
    pub context: ControlContextKind,
    pub entries: Vec<PromptEntry>,
    /// In a `Menu` / `Dialogue` context, the label the confirm-functional
    /// controls (touch Jump / Interact fold into menu-select) should show —
    /// "Select" / "Advance" today, and the active menu's item verb (Equip /
    /// Use) once P4b wires the app-side provider. `None` in gameplay.
    pub menu_confirm: Option<String>,
}

impl ControlPrompt {
    /// The label currently on a given slot, if the prompt claims it.
    pub fn label_for(&self, slot: ControlSlot) -> Option<&str> {
        self.entries
            .iter()
            .find(|e| e.slot == slot)
            .map(|e| e.label.as_str())
    }
}

/// Rebuild [`ControlPrompt`] from the controlled subject's action scheme.
///
/// Follows [`ControlledSubject`] (falling back to the primary player), so the
/// prompt describes the body you are DRIVING — the same relativity rule the
/// camera, input, and affordances already obey. Gameplay only for now; menu /
/// dialogue publish an explicit empty context until P4.
pub fn rebuild_control_prompt(
    mode: Res<State<GameMode>>,
    controlled: Option<Res<ControlledSubject>>,
    primary: Query<Entity, (With<PlayerEntity>, With<PrimaryPlayer>)>,
    schemes: Query<&ActorActionScheme>,
    mut prompt: ResMut<ControlPrompt>,
) {
    // Menu / dialogue own input: no gameplay scheme. Publish an explicit
    // context + the generic confirm verb so the overlay relabels the
    // select-functional buttons and hides the rest. The specific item verb
    // (Equip / Use) arrives with the app-side provider in P4b.
    if !mode.get().allows_gameplay() {
        let (context, confirm) = match mode.get() {
            GameMode::Dialogue => (ControlContextKind::Dialogue, "Advance"),
            _ => (ControlContextKind::Menu, "Select"),
        };
        set_prompt(&mut prompt, context, Vec::new(), Some(confirm.to_owned()));
        return;
    }

    let subject = controlled
        .and_then(|s| s.0)
        .or_else(|| primary.single().ok());
    let Some(scheme) = subject.and_then(|e| schemes.get(e).ok()) else {
        // Cold start (no player yet) or a controlled body without a scheme.
        set_prompt(&mut prompt, ControlContextKind::Empty, Vec::new(), None);
        return;
    };

    let entries = scheme
        .0
        .iter()
        .map(|action| PromptEntry {
            slot: action.slot,
            label: action.display(),
            visual: action.visual.clone(),
        })
        .collect();
    set_prompt(&mut prompt, ControlContextKind::Gameplay, entries, None);
}

/// Write only when the prompt actually changed, so `Changed<ControlPrompt>`
/// stays honest for the presentation systems that filter on it.
fn set_prompt(
    prompt: &mut ControlPrompt,
    context: ControlContextKind,
    entries: Vec<PromptEntry>,
    menu_confirm: Option<String>,
) {
    if prompt.context != context || prompt.entries != entries || prompt.menu_confirm != menu_confirm
    {
        prompt.context = context;
        prompt.entries = entries;
        prompt.menu_confirm = menu_confirm;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_characters::action_scheme::derive_action_scheme;
    use ambition_engine_core::AbilitySet;
    use ambition_entity_catalog::{ClipBinding, MoveSpec, MovesetContract};
    use std::collections::BTreeMap;

    /// A body with `jump` and, optionally, an attack move whose id is
    /// `attack_move` (so its label comes from the MOVE, not the verb).
    fn scheme_component(jump: bool, attack_move: Option<&str>) -> ActorActionScheme {
        let mut a = AbilitySet::default();
        a.jump = jump;
        a.dash = false;
        a.dodge = false;
        a.blink = false;
        a.fly = false;
        a.shield = false;
        let moveset = attack_move.map(|move_id| {
            let mut m = MovesetContract::default();
            m.verbs = BTreeMap::from([("attack".to_string(), move_id.to_string())]);
            m.moves = vec![MoveSpec {
                id: move_id.to_string(),
                clip: ClipBinding {
                    clip: move_id.to_string(),
                    fallbacks: vec![],
                },
                duration_s: 0.3,
                windows: vec![],
                events: vec![],
                gates: Default::default(),
                start_impulse: None,
                smash_charge_mult: 1.0,
            }];
            m
        });
        ActorActionScheme(derive_action_scheme(&a, moveset.as_ref(), &[]))
    }

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.init_state::<GameMode>();
        app.init_resource::<ControlPrompt>();
        app.insert_resource(ControlledSubject(None));
        app.add_systems(Update, rebuild_control_prompt);
        app
    }

    #[test]
    fn publishes_controlled_subjects_scheme_labels() {
        let mut app = app();
        let body = app
            .world_mut()
            .spawn((
                PlayerEntity,
                PrimaryPlayer,
                scheme_component(true, Some("swat")),
            ))
            .id();
        app.world_mut().resource_mut::<ControlledSubject>().0 = Some(body);
        app.update();

        let prompt = app.world().resource::<ControlPrompt>();
        assert_eq!(prompt.context, ControlContextKind::Gameplay);
        assert_eq!(prompt.label_for(ControlSlot::Jump), Some("Jump"));
        // The attack label comes from the bound move id (title-cased).
        assert_eq!(prompt.label_for(ControlSlot::Attack), Some("Swat"));
        assert_eq!(prompt.label_for(ControlSlot::Special), None);
    }

    #[test]
    fn menu_context_publishes_a_confirm_verb_not_the_scheme() {
        let mut app = app();
        app.world_mut().spawn((
            PlayerEntity,
            PrimaryPlayer,
            scheme_component(true, Some("swat")),
        ));
        // Enter a paused (menu) mode and let the transition apply.
        app.world_mut()
            .resource_mut::<NextState<GameMode>>()
            .set(GameMode::Paused);
        app.update();

        let prompt = app.world().resource::<ControlPrompt>();
        assert_eq!(prompt.context, ControlContextKind::Menu);
        assert_eq!(prompt.menu_confirm.as_deref(), Some("Select"));
        // The gameplay scheme is NOT published while a menu owns input.
        assert!(prompt.entries.is_empty());
        assert_eq!(prompt.label_for(ControlSlot::Jump), None);
    }

    #[test]
    fn prompt_follows_the_controlled_subject_on_possession() {
        let mut app = app();
        let home = app
            .world_mut()
            .spawn((PlayerEntity, PrimaryPlayer, scheme_component(true, None)))
            .id();
        // A possessable body with a richer scheme (has an attack).
        let other = app
            .world_mut()
            .spawn(scheme_component(true, Some("cleave")))
            .id();

        app.world_mut().resource_mut::<ControlledSubject>().0 = Some(home);
        app.update();
        assert_eq!(
            app.world()
                .resource::<ControlPrompt>()
                .label_for(ControlSlot::Attack),
            None,
            "home avatar has no attack"
        );

        // Possess the other body — the prompt must swap to ITS scheme.
        app.world_mut().resource_mut::<ControlledSubject>().0 = Some(other);
        app.update();
        assert_eq!(
            app.world()
                .resource::<ControlPrompt>()
                .label_for(ControlSlot::Attack),
            Some("Cleave"),
            "possessed body's attack now labels the slot"
        );
    }
}
