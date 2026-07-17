//! `ControlPrompt` — the read-model of "what does each on-screen control do
//! right now, and what is it called," for whatever currently owns input.
//!
//! This is the observation boundary the touch overlay (and any future prompt
//! surface) reads instead of reaching into the sim heart. It is rebuilt once
//! per tick in the sim tail by resolving the controlled subject's live
//! authorities (`AbilitySet` + moveset + `ActionSet` + techniques) through the
//! SHARED `derive_action_scheme` — the very same resolver the gameplay persona
//! gate calls to gate/route behavior. Both re-derive from the body's current
//! authorities every tick, so possessing a different body swaps the labels and
//! the prompt can never advertise an action the body won't perform — not even
//! for one frame across a kit swap (no lagged cache sits on this path).
//!
//! Menu / dialogue contexts publish an explicit context with no gameplay
//! entries; the specific command label (Equip / Use) is supplied by the
//! app-side menu provider into [`ControlPrompt::menu_confirm`]. Per-slot glyphs
//! (the physical binding) land with the `ActiveBindings` source in P1/P5; the
//! touch overlay keeps its own glyph subtitle in the meantime, so this model is
//! label-first.

use ambition_actors::actor::BodyAbilities;
use ambition_characters::action_scheme::{derive_action_scheme, ActorTechniques};
use ambition_characters::brain::action_set::ActionSet;
use ambition_combat::moveset::ActorMoveset;
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
/// The scheme is resolved HERE from the subject's live authorities via the shared
/// [`derive_action_scheme`] — the SAME function, on the SAME immediate
/// authorities, that the gameplay persona gate (`gate_worn_player_control`) calls
/// to gate/route behavior. Because both consumers re-derive from the body's
/// current `AbilitySet` / moveset / `ActionSet` / techniques each tick, a button's
/// label and what it fires cannot drift — not even for one frame across a kit
/// swap (there is no one-tick-lagged cache on the critical path; the derived
/// `ActorActionScheme` component is a separate observation cache).
///
/// Follows [`ControlledSubject`] (falling back to the primary player), so the
/// prompt describes the body you are DRIVING — the same relativity rule the
/// camera and input already obey. Menu / dialogue publish an explicit context.
pub fn rebuild_control_prompt(
    mode: Res<State<GameMode>>,
    controlled: Option<Res<ControlledSubject>>,
    primary: Query<Entity, (With<PlayerEntity>, With<PrimaryPlayer>)>,
    authorities: Query<(
        &BodyAbilities,
        Option<&ActorMoveset>,
        Option<&ActionSet>,
        Option<&ActorTechniques>,
    )>,
    mut prompt: ResMut<ControlPrompt>,
) {
    // Menu / dialogue own input: no gameplay scheme. Publish an explicit
    // context + the generic confirm verb so the overlay relabels the
    // select-functional buttons and hides the rest. The specific item verb
    // (Equip / Use) is supplied by the app-side provider (see `menu_confirm`).
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
    let Some((abilities, moveset, action_set, techniques)) =
        subject.and_then(|e| authorities.get(e).ok())
    else {
        // Cold start (no player yet) or a controlled body without authorities.
        set_prompt(&mut prompt, ControlContextKind::Empty, Vec::new(), None);
        return;
    };

    let scheme = derive_action_scheme(
        &abilities.abilities,
        moveset.map(|m| &m.0),
        action_set,
        techniques.map_or(&[], |t| t.0.as_slice()),
    );
    let entries = scheme
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
    use ambition_engine_core::AbilitySet;
    use ambition_entity_catalog::{ClipBinding, MoveSpec, MovesetContract};
    use std::collections::BTreeMap;

    /// A body's LIVE authorities: `jump` + optionally an attack MOVE (id
    /// `attack_move`, so its label comes from the move, not the verb). The prompt
    /// derives its scheme from these — the SAME authorities gameplay gates on —
    /// so the test exercises the real resolver, not a pre-baked scheme component.
    fn authorities(jump: bool, attack_move: Option<&str>) -> (BodyAbilities, ActorMoveset) {
        let mut a = AbilitySet::default();
        a.jump = jump;
        a.dash = false;
        a.dodge = false;
        a.blink = false;
        a.fly = false;
        a.shield = false;
        let mut m = MovesetContract::default();
        if let Some(move_id) = attack_move {
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
        }
        (BodyAbilities::new(a), ActorMoveset(m))
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
                authorities(true, Some("swat")),
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
            authorities(true, Some("swat")),
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
            .spawn((PlayerEntity, PrimaryPlayer, authorities(true, None)))
            .id();
        // A possessable body with a richer scheme (has an attack).
        let other = app
            .world_mut()
            .spawn(authorities(true, Some("cleave")))
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

    /// Gate 4 (GPT-5.6 review): the VISIBLE slot and the EXECUTABLE behavior
    /// cannot disagree for one frame across a kit swap. The real gameplay gate
    /// (`gate_worn_player_control`) and the real prompt (`rebuild_control_prompt`)
    /// both re-derive from the body's IMMEDIATE `ActionSet` each tick via the
    /// shared `derive_action_scheme`, so on the very tick the kit changes, the
    /// button's presence and whether the verb fires flip TOGETHER — there is no
    /// one-tick-lagged cache between them.
    #[test]
    fn a_same_tick_kit_swap_cannot_drift_the_prompt_from_the_gate() {
        use ambition_characters::action_scheme::ResolvedTechniqueEdges;
        use ambition_characters::actor::character_catalog::CharacterCatalog;
        use ambition_characters::actor::control::ActorControlFrame;
        use ambition_characters::actor::WornCharacter;
        use ambition_characters::brain::{ActorControl, MeleeActionSpec, SwipeSpec};

        let mut app = App::new();
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.init_state::<GameMode>();
        app.init_resource::<ControlPrompt>();
        app.insert_resource(CharacterCatalog::empty());
        // Run the REAL gate and the REAL prompt in one tick, gate first (the
        // order gameplay uses); both read the same immediate ActionSet.
        app.add_systems(
            Update,
            (
                ambition_actors::avatar::gate_worn_player_control,
                rebuild_control_prompt,
            )
                .chain(),
        );

        // Kit A: a striker (has melee). Pressing melee.
        let mut kit_a = ActionSet::default();
        kit_a.melee = Some(MeleeActionSpec::Swipe(SwipeSpec::STRIKER_DEFAULT));
        let mut frame = ActorControlFrame::neutral();
        frame.melee_pressed = true;
        let body = app
            .world_mut()
            .spawn((
                PlayerEntity,
                PrimaryPlayer,
                WornCharacter::new("hero"),
                BodyAbilities::new(AbilitySet::sandbox_all()),
                kit_a,
                ResolvedTechniqueEdges::default(),
                ActorControl(frame),
            ))
            .id();
        app.insert_resource(ControlledSubject(Some(body)));
        app.update();

        // Same tick, kit A: the button SHOWS Attack AND the gate KEPT melee.
        let shows_attack = |app: &App| {
            app.world()
                .resource::<ControlPrompt>()
                .label_for(ControlSlot::Attack)
                .is_some()
        };
        let fires_melee = |app: &App| {
            app.world().get::<ActorControl>(body).unwrap().0.melee_pressed
        };
        assert!(shows_attack(&app), "striker kit advertises Attack");
        assert!(fires_melee(&app), "striker kit keeps the melee verb");
        assert_eq!(
            shows_attack(&app),
            fires_melee(&app),
            "kit A: prompt and gate agree"
        );

        // SWAP to a peaceful kit in-place (what apply_worn_character_gameplay does
        // on a kit change), and re-press melee for the new tick.
        {
            let mut set = app.world_mut().get_mut::<ActionSet>(body).unwrap();
            *set = ActionSet::peaceful();
        }
        app.world_mut()
            .get_mut::<ActorControl>(body)
            .unwrap()
            .0
            .melee_pressed = true;
        app.update();

        // The SAME tick as the swap: the button DROPS Attack AND the gate STRIPS
        // melee — they flipped together, no one-frame disagreement.
        assert!(!shows_attack(&app), "peaceful kit hides Attack");
        assert!(!fires_melee(&app), "peaceful kit strips the melee verb");
        assert_eq!(
            shows_attack(&app),
            fires_melee(&app),
            "kit B (same tick as swap): prompt and gate still agree — no drift"
        );
    }
}
