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
//! entries; the specific command label (Equip / Use / Play / Continue) comes
//! from the owning surface's published [`UiCue`] (`ambition_input::cues`).
//! Ownership of THIS resource follows the resolved input context: while a
//! frontend context (startup cards, launcher) owns the participant's actions,
//! [`publish_frontend_context_prompt`] writes the prompt and the sim-side
//! rebuild yields; while gameplay owns them, [`rebuild_control_prompt`] is
//! the sole writer. Per-slot glyphs (the physical binding) land with the
//! `ActiveBindings` source in P1/P5; the touch overlay keeps its own glyph
//! subtitle in the meantime, so this model is label-first.

use ambition_characters::action_scheme::{derive_action_scheme, ActorTechniques};
use ambition_characters::brain::action_set::ActionSet;
use ambition_combat::moveset::ActorMoveset;
use ambition_entity_catalog::action_scheme::{ControlSlot, VisualId};
use ambition_input::{ActiveUiCues, SeatInputContexts, UiCue, GAMEPLAY_CONTEXT};
use ambition_platformer2d_core::BodyAbilities;
use ambition_platformer2d_shared_tangle::markers::{
    ControlledSubject, PlayerEntity, PrimaryPlayer,
};
use ambition_platformer2d_shared_tangle::schedule::GameMode;
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

/// Does a gameplay prompt name the BUTTON, or the MOVE currently on it?
///
/// plain. E.g. \"Attack\" \"Special\" \"Jump\" \"Grab\", no context sensitive naming
/// of the move in smash, at least not yet."*
///
/// `ByMove` stays the DEFAULT, so every experience that did not ask keeps
/// exactly the prompt it had. This is a knob, not a policy change — "at least
/// not yet" is a decision that may come back, and the move-naming machinery is
/// worth keeping working while it is switched off.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PromptNaming {
    /// The move on the slot right now — "Spin Dash", "Shoulder Check". Reads
    /// well for a platformer with a handful of signature techniques.
    #[default]
    ByMove,
    /// The button itself. In a fighting game a slot hosts a dozen moves chosen
    /// by stick direction and posture, so naming it after whichever one is
    /// currently resolvable tells the player something that changes as they
    /// walk — and never the thing they need, which is which button to press.
    ByButton,
}

/// The plain, player-facing name of a button.
///
/// this lives in the PRESENTATION layer on purpose, not on `ControlSlot`.
/// A slot is an engine identity — `Burst` is the right internal name for the
/// channel dodge and dash share — and the player-facing word is a different
/// question the engine should not get to answer. It is also why this is not
/// `title_case_id` over the variant name: `"Projectile"` is what the engine
/// calls it and `"Shot"` is what the button says.
///
/// exhaustive on purpose. A new `ControlSlot` variant must choose its word
/// here rather than inherit a wrong one from a catch-all arm.
fn button_label(slot: ControlSlot) -> &'static str {
    match slot {
        ControlSlot::Jump => "Jump",
        ControlSlot::Attack => "Attack",
        ControlSlot::Special => "Special",
        ControlSlot::Projectile => "Shot",
        // the one GENRE-DEPENDENT word here, flagged rather than settled.
        // The slot is `Burst` because dodge and dash are one press; "Dodge" is
        // what a platform fighter's player calls that button and "Dash" is what
        // a platformer's does. `ByButton` has exactly one adopter today (smash),
        // so this is true for every current reader — but the second experience
        // to opt in may need the other word, and at that point the label belongs
        // on the experience rather than here.
        ControlSlot::Burst => "Dodge",
        ControlSlot::Blink => "Blink",
        ControlSlot::Interact => "Interact",
        ControlSlot::Utility => "Utility",
        ControlSlot::Shield => "Shield",
        ControlSlot::Grab => "Grab",
        ControlSlot::Modifier => "Modifier",
        ControlSlot::Taunt => "Taunt",
    }
}

/// One control's current meaning: the slot it sits on, its player-facing
/// label, and an optional visual. Ordered within [`ControlPrompt::entries`] by
/// the scheme's canonical slot order.
#[derive(Clone, Debug, PartialEq)]
pub struct PromptEntry {
    pub slot: ControlSlot,
    pub label: String,
    pub visual: Option<VisualId>,
    /// The physical control this slot is bound to for the local primary seat —
    /// "Z", "A", "Cross" — or `None` when nothing bound it (or when no
    /// projection is installed, as in a headless sim).
    ///
    /// read from `SeatBindings`, never written by hand. The verb and the
    /// key are two different facts with two different owners: the verb is what
    /// this slot DOES (the action scheme's answer) and the binding is which
    /// control presses it (the input map's). A prompt that hardcoded the second
    /// would go stale on the first rebind, and the player would be told to press
    /// a key that does nothing.
    pub binding: Option<String>,
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

/// The frontend half of the prompt: while a non-gameplay input context owns
/// the participant's actions (startup cards, the launcher), publish the
/// owning surface's cue as a Menu-context prompt so the touch overlay (and
/// any prompt surface) shows a labeled confirm control with no session and
/// no gameplay actor.
///
/// Runs on the frame clock between cue publication (`InputSet::PublishCues`)
/// and the consumers; [`rebuild_control_prompt`] yields on exactly the frames
/// this system writes, so the resource has one writer per frame by
/// construction. Absent resources (headless sims without a host input stack)
/// make it a no-op.
pub fn publish_frontend_context_prompt(
    active_context: Option<Res<SeatInputContexts>>,
    cues: Option<Res<ActiveUiCues>>,
    mut prompt: ResMut<ControlPrompt>,
) {
    // The on-screen prompt describes ONE seat — the local primary, the one
    // whose body the camera follows. Other seats resolve their own contexts and
    // route their own input; they do not compete to write this HUD.
    let Some(owner) = active_context
        .as_deref()
        .and_then(|seats| seats.primary().owner())
    else {
        return;
    };
    if owner == GAMEPLAY_CONTEXT {
        return;
    }
    // A resolved non-gameplay owner IS the proof that a surface owns input, so
    // this exit carries a fallback verb and never resolves `Empty`.
    let (context, confirm) = surface_prompt(
        ControlContextKind::Menu,
        cues.as_deref().and_then(|cues| cues.for_context(owner)),
        Some("Select"),
    );
    set_prompt(&mut prompt, context, Vec::new(), confirm);
}

/// The set [`rebuild_control_prompt`] runs in — the prompt view is rebuilt.
///
/// Anything contributing a cue for this frame's prompt must land before it, and
/// three call sites say so: two inside this crate and one in the app's menu.
///
/// ONE member, nested inside the `FeatureViewSync` phase, which holds every
/// other view rebuild. A contributor needs to beat THIS rebuild, not every view.
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ControlPromptRebuilt;

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
    active_context: Option<Res<SeatInputContexts>>,
    // The physical bindings, projected from the live `InputMap`. Optional
    // because a headless sim installs no input stack; absent, entries carry no
    // glyph rather than a stale one.
    bindings: Option<Res<ambition_input::SeatBindings>>,
    // Which pad the seat is holding, so a binding is SPELLED in that pad's
    // vocabulary: "Cross" on a DualSense where an Xbox pad says "A". Absent, the
    // labels take the documented Xbox-style default rather than guessing.
    devices: Option<Res<ambition_input::SeatActiveDevices>>,
    controlled: Option<Res<ControlledSubject>>,
    primary: Query<Entity, (With<PlayerEntity>, With<PrimaryPlayer>)>,
    authorities: Query<(
        Ref<BodyAbilities>,
        Option<Ref<ActorMoveset>>,
        Option<Ref<ActionSet>>,
        Option<Ref<ActorTechniques>>,
    )>,
    cues: Option<Res<ActiveUiCues>>,
    // Whether this experience wants the BUTTON named or the MOVE on it. Absent
    // (the ordinary case) is `ByMove` — the behaviour every experience had
    // before the knob existed.
    naming: Option<Res<PromptNaming>>,
    mut prompt: ResMut<ControlPrompt>,
    // (last subject, authority-presence bits, resource-presence bits) from the
    // previous rebuild. `None` = never rebuilt, so the first frame always
    // derives. The RESOURCE presence bits are part of the key because
    // `is_changed()` on an `Option<Res<T>>` can only speak while the resource
    // is `Some`: a removal contributes nothing to `inputs_changed`, so without
    // the bits a quiet-frame removal of `SeatInputContexts` / `ActiveUiCues` /
    // `ControlledSubject` / `SeatBindings` / `SeatActiveDevices` would be
    // skipped and the prompt would keep describing a context that no longer
    // exists.
    mut last: Local<Option<(Option<Entity>, [bool; 3], [bool; 6])>>,
) {
    // A frontend context (startup cards, launcher) owns the participant's
    // actions: its provider (`publish_frontend_context_prompt`) writes the
    // prompt, and the sim-side rebuild yields — one writer per frame, decided
    // by the SAME resolved context that routes the input itself. (Absent
    // resource = headless sim without a host input stack; proceed as the
    // sole writer.)
    if active_context
        .as_deref()
        .and_then(|seats| seats.primary().owner())
        .is_some_and(|owner| owner != GAMEPLAY_CONTEXT)
    {
        // While someone else writes the prompt, OUR cache key describes a
        // prompt that no longer exists. Drop it, so the frame that hands
        // ownership back (including by REMOVING the context resource, which no
        // change detection reports) always re-derives.
        *last = None;
        return;
    }

    // Change-detection gate: the derive below reads only these resources and
    // the subject's authority components, and Bevy change detection fires the
    // SAME frame as the mutation — so skipping quiet frames cannot lag a kit
    // swap even one tick (the doc contract above). This was ~1.4% of frame
    // CPU re-deriving an identical scheme.
    let inputs_changed = mode.is_changed()
        || active_context.as_ref().is_some_and(|r| r.is_changed())
        || controlled.as_ref().is_some_and(|r| r.is_changed())
        || cues.as_ref().is_some_and(|r| r.is_changed())
        // A REBIND MUST INVALIDATE THE PROMPT. Without this line the entries
        // keep the glyph they were built with and the player is told to press
        // the old key — which is the precise staleness the binding projection
        // exists to make impossible, reintroduced one layer up by a cache.
        || bindings.as_ref().is_some_and(|r| r.is_changed())
        // AND PICKING UP A DIFFERENT PAD MUST TOO. The binding did not move, so `SeatBindings`
        // is quiet — only the SPELLING changed, and a cache keyed on the binding alone would
        // keep telling a DualSense player to press A.
        || devices.as_ref().is_some_and(|r| r.is_changed())
        // AND FLIPPING THE NAMING MUST TOO. It changes every label without
        // touching a binding, an authority or a subject, so a cache keyed on
        // those alone would keep publishing the old vocabulary forever.
        || naming.as_ref().is_some_and(|r| r.is_changed());
    // Presence is tracked separately from change: an `Option<Res<T>>` that went
    // `Some -> None` reports no change at all (see `last`'s doc).
    let resources = [
        active_context.is_some(),
        controlled.is_some(),
        cues.is_some(),
        bindings.is_some(),
        devices.is_some(),
        naming.is_some(),
    ];

    // Menu / dialogue own input: no gameplay scheme. Publish an explicit context
    // + a confirm verb so the overlay relabels the select-functional buttons and
    // hides the rest. The SPECIFIC verb ("Equip" / "Use") comes from the owning
    // surface's published cue; absent that (no menu open, or a non-item focus),
    // fall back to the generic context verb.
    if !mode.get().allows_gameplay() {
        if matches!(*last, Some((_, _, seen)) if seen == resources) && !inputs_changed {
            return;
        }
        *last = Some((None, [false; 3], resources));
        let (kind, fallback) = match mode.get() {
            GameMode::Dialogue => (ControlContextKind::Dialogue, "Advance"),
            _ => (ControlContextKind::Menu, "Select"),
        };
        // The mode itself is the proof here: gameplay input cannot route, so
        // something else owns the screen even if it published no cue.
        let (context, confirm) = surface_prompt(
            kind,
            cues.as_deref().and_then(ActiveUiCues::top),
            Some(fallback),
        );
        set_prompt(&mut prompt, context, Vec::new(), confirm);
        return;
    }

    let subject = controlled
        .as_deref()
        .and_then(|s| s.0)
        .or_else(|| primary.single().ok());
    let Some((abilities, moveset, action_set, techniques)) =
        subject.and_then(|e| authorities.get(e).ok())
    else {
        // Cold start (no player yet) or a controlled body without authorities —
        // and NO independent proof that anything owns the screen, because
        // gameplay is allowed to route and no other context resolved. So this
        // exit passes no fallback: a published cue is the only thing that can
        // make it a Menu, and with none it stays `Empty`.
        let (context, confirm) = surface_prompt(
            ControlContextKind::Menu,
            cues.as_deref().and_then(ActiveUiCues::top),
            None,
        );
        set_prompt(&mut prompt, context, Vec::new(), confirm);
        *last = Some((subject, [false; 3], resources));
        return;
    };

    let presence = [
        moveset.is_some(),
        action_set.is_some(),
        techniques.is_some(),
    ];
    let authorities_changed = abilities.is_changed()
        || moveset.as_ref().is_some_and(|r| r.is_changed())
        || action_set.as_ref().is_some_and(|r| r.is_changed())
        || techniques.as_ref().is_some_and(|r| r.is_changed());
    if *last == Some((subject, presence, resources)) && !inputs_changed && !authorities_changed {
        return;
    }
    *last = Some((subject, presence, resources));

    let scheme = derive_action_scheme(
        &abilities.abilities,
        moveset.as_deref().map(|m| &m.0),
        action_set.as_deref(),
        techniques.as_deref().map_or(&[], |t| t.0.as_slice()),
    );
    let entries = scheme
        .iter()
        .map(|action| PromptEntry {
            slot: action.slot,
            label: match naming.as_deref().copied().unwrap_or_default() {
                PromptNaming::ByButton => button_label(action.slot).to_owned(),
                PromptNaming::ByMove => action.display(),
            },
            visual: action.visual.clone(),
            binding: bindings.as_deref().and_then(|seats| {
                seats.label_for_slot(
                    ambition_input::ParticipantId::PRIMARY.slot(),
                    action.slot,
                    devices.as_deref(),
                )
            }),
        })
        .collect();
    set_prompt(&mut prompt, ControlContextKind::Gameplay, entries, None);
}

/// Is the player working a SURFACE rather than driving a body — and what does
/// its confirm control say?
///
/// `Empty` is what the touch overlay reads to hide the move stick and the confirm buttons, and a
/// hidden node takes no drags, so such a surface drew perfectly and could not be touched. Every
/// exit calls THIS now; the answer cannot depend on which branch arrived at it.
///
/// `fallback` is the caller's INDEPENDENT proof that a surface owns input — a
/// non-gameplay `GameMode`, or a resolved non-gameplay context. An exit holding
/// no such proof passes `None` and must earn its context from a cue alone. With
/// neither, the honest answer is `Empty` and no verb: nothing has claimed the
/// screen, which is the cold start `Empty` exists for.
///
/// a cue is the right evidence because the menu lane is UNGATED —
/// `populate_seat_menu_frames` folds every participant's `MenuStick` into the
/// per-seat frames whatever owns the context, and arbitration happens at the
/// CONSUMER. So a published cue means some surface is reading those frames, and
/// the stick genuinely steers it.
fn surface_prompt(
    kind: ControlContextKind,
    cue: Option<&UiCue>,
    fallback: Option<&str>,
) -> (ControlContextKind, Option<String>) {
    match cue
        .map(|cue| cue.submit_label.clone())
        .or_else(|| fallback.map(str::to_owned))
    {
        Some(confirm) => (kind, Some(confirm)),
        None => (ControlContextKind::Empty, None),
    }
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
    use ambition_entity_catalog::{ClipBinding, MoveSpec, MovesetContract};
    use ambition_input::UiCue;
    use ambition_platformer2d_core::AbilitySet;
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
        // a move table is WHAT the attack is; the ability is WHETHER this body may attack at
        // all. So a fixture that hands a body an attack MOVE has to say the body may attack,
        // or the scheme resolves no Attack slot and every label here reads `None`.
        a.attack = attack_move.is_some();
        let mut m = MovesetContract::default();
        if let Some(move_id) = attack_move {
            m.verbs = BTreeMap::from([("attack".to_string(), move_id.to_string())]);
            m.moves = vec![MoveSpec {
                display_name: None,
                landing_lag_s: None,
                autocancel_after_s: None,
                sprite_spin_hz: None,
                equips: None,
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
                smash_charge: None,
                charge_gesture: ambition_entity_catalog::ChargeGesture::default(),
                repeat: None,
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
            .spawn((PlayerEntity, PrimaryPlayer, authorities(true, Some("swat"))))
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

    /// The prompt shows the key, and the key is the one the router reads.
    ///
    /// The verb and the binding are two facts with two owners — what the slot
    /// DOES (the action scheme) and which control presses it (the input map).
    /// This asserts the second is read rather than written, and that a REBIND
    /// moves it: a prompt cached past a remap tells the player to press a key
    /// that does nothing, which is exactly the staleness the projection exists
    /// to prevent, reintroduced one layer up by a cache.
    #[test]
    fn the_prompt_carries_the_binding_and_a_rebind_moves_it() {
        use ambition_input::{KeyboardPreset, SeatBindings};
        use leafwing_input_manager::prelude::InputMap;

        let mut app = app();
        app.init_resource::<SeatBindings>();
        app.add_systems(
            Update,
            ambition_input::publish_seat_bindings.before(rebuild_control_prompt),
        );
        // Drive the REAL projection the way the host does — spawn the participant holding the
        // map — rather than hand-writing a `SeatBindings`.
        let arrows = KeyboardPreset::arrows_zxc();
        app.world_mut().spawn((
            ambition_input::InputParticipant::primary(),
            arrows.input_map(),
        ));
        let body = app
            .world_mut()
            .spawn((PlayerEntity, PrimaryPlayer, authorities(true, Some("swat"))))
            .id();
        app.world_mut().resource_mut::<ControlledSubject>().0 = Some(body);
        app.update();

        let binding_for = |app: &App, slot: ControlSlot| -> Option<String> {
            app.world()
                .resource::<ControlPrompt>()
                .entries
                .iter()
                .find(|entry| entry.slot == slot)
                .and_then(|entry| entry.binding.clone())
        };
        assert_eq!(
            binding_for(&app, ControlSlot::Jump).as_deref(),
            Some("Z"),
            "Arrows+ZXC puts Jump on Z, and the prompt says so without a table of its own"
        );
        assert_eq!(
            app.world()
                .resource::<ControlPrompt>()
                .label_for(ControlSlot::Jump),
            Some("Jump"),
            "the VERB is unchanged — the binding is a second field, not a replacement"
        );

        // Rebind, the way a remap screen would.
        {
            let world = app.world_mut();
            let mut maps =
                world.query::<&mut InputMap<ambition_input::Platformer2dInputActionMonolith>>();
            for mut map in maps.iter_mut(world) {
                map.clear_action(&ambition_input::Platformer2dInputActionMonolith::Jump);
                map.insert(
                    ambition_input::Platformer2dInputActionMonolith::Jump,
                    KeyCode::F13,
                );
            }
        }
        app.update();
        assert_eq!(
            binding_for(&app, ControlSlot::Jump).as_deref(),
            Some("F13"),
            "the prompt followed the rebind — a cache that skipped this would tell the player \
             to press a key that does nothing"
        );
    }

    /// The prompt spells the button the way the seat's own pad does.
    ///
    /// and picking up a different pad has to reach it. The BINDING does not move when a player
    /// swaps a DualSense for an Xbox pad, so the projection stays quiet and a cache keyed on it
    /// alone keeps saying "Cross" to somebody holding a pad with an A on it.
    #[test]
    fn the_prompt_spells_a_button_in_the_seats_own_vocabulary() {
        use ambition_input::{
            ActiveDevice, GamepadStyle, SeatActiveDevices, SeatBindings,
        };

        let mut app = app();
        app.init_resource::<SeatBindings>();
        app.init_resource::<SeatActiveDevices>();
        app.add_systems(
            Update,
            ambition_input::publish_seat_bindings.before(rebuild_control_prompt),
        );
        // A gamepad-only seat, so the FIRST binding for Jump — the one a prompt
        // prints — is a pad button rather than a key.
        app.world_mut().spawn((
            ambition_input::InputParticipant::primary(),
            ambition_input::KeyboardPreset::of(ambition_input::KeyboardPreset::by_index(0).id)
                .map_for(ambition_input::BindingSources::GamepadOnly),
        ));
        let body = app
            .world_mut()
            .spawn((PlayerEntity, PrimaryPlayer, authorities(true, Some("swat"))))
            .id();
        app.world_mut().resource_mut::<ControlledSubject>().0 = Some(body);

        let binding_for = |app: &App, slot: ControlSlot| -> Option<String> {
            app.world()
                .resource::<ControlPrompt>()
                .entries
                .iter()
                .find(|entry| entry.slot == slot)
                .and_then(|entry| entry.binding.clone())
        };

        app.world_mut()
            .resource_mut::<SeatActiveDevices>()
            .mark_primary(ActiveDevice::Gamepad(GamepadStyle::XboxLike));
        app.update();
        assert_eq!(
            binding_for(&app, ControlSlot::Jump).as_deref(),
            Some("A"),
            "an Xbox pad jumps with A"
        );

        app.world_mut()
            .resource_mut::<SeatActiveDevices>()
            .mark_primary(ActiveDevice::Gamepad(GamepadStyle::PlayStation));
        app.update();
        assert_eq!(
            binding_for(&app, ControlSlot::Jump).as_deref(),
            Some("Cross"),
            "the same binding, spelled by the pad now in the player's hands"
        );
    }

    #[test]
    fn menu_context_falls_back_to_the_generic_verb_with_no_provider() {
        let mut app = app();
        app.world_mut()
            .spawn((PlayerEntity, PrimaryPlayer, authorities(true, Some("swat"))));
        // Enter a paused (menu) mode and let the transition apply.
        app.world_mut()
            .resource_mut::<NextState<GameMode>>()
            .set(GameMode::Paused);
        app.update();

        let prompt = app.world().resource::<ControlPrompt>();
        assert_eq!(prompt.context, ControlContextKind::Menu);
        // No cue published -> the generic fallback verb.
        assert_eq!(prompt.menu_confirm.as_deref(), Some("Select"));
        // The gameplay scheme is NOT published while a menu owns input.
        assert!(prompt.entries.is_empty());
        assert_eq!(prompt.label_for(ControlSlot::Jump), None);
    }

    /// Gate 6: the SPECIFIC menu verb comes from a published
    /// cue, not a hardcoded string. When the app menu provider publishes the
    /// focused item's verb as a `UiCue`, the prompt shows it ("Equip") instead
    /// of the generic "Select". (The full app-menu-model -> provider path is
    /// exercised end-to-end in the ambition_app menu tests; this pins the
    /// sim_view read half.)
    #[test]
    fn a_published_menu_cue_overrides_the_generic_verb() {
        use ambition_input::InputContextId;
        let mut app = app();
        app.init_resource::<ActiveUiCues>();
        app.world_mut()
            .resource_mut::<ActiveUiCues>()
            .declare(UiCue {
                context: InputContextId("app.inventory"),
                priority: 150,
                submit_label: "Equip".to_owned(),
            });
        app.world_mut()
            .resource_mut::<NextState<GameMode>>()
            .set(GameMode::Paused);
        app.update();

        let prompt = app.world().resource::<ControlPrompt>();
        assert_eq!(prompt.context, ControlContextKind::Menu);
        assert_eq!(
            prompt.menu_confirm.as_deref(),
            Some("Equip"),
            "the focused item's real verb overrides the generic Select"
        );
    }

    /// A surface that published a cue owns input even with no body to drive.
    ///
    /// The no-subject exit answered `Empty` unconditionally, twenty lines below
    /// an exit that already folded the cue in — so a cue published by a surface
    /// running under gameplay's own `GameMode`, with nothing seated yet, was
    /// dropped on the floor. `Empty` is what the touch overlay reads to hide the
    /// move stick AND the confirm buttons, and a hidden node takes no drags, so
    /// the menu was dead rather than merely unlabelled.
    #[test]
    fn a_published_cue_gives_a_bodiless_surface_a_usable_menu_prompt() {
        use ambition_input::InputContextId;
        let mut app = app();
        app.init_resource::<ActiveUiCues>();
        app.world_mut()
            .resource_mut::<ActiveUiCues>()
            .declare(UiCue {
                context: InputContextId("test.surface"),
                priority: 130,
                submit_label: "Choose".to_owned(),
            });
        // `GameMode` stays `Playing` and no body exists: the no-subject exit.
        app.update();

        let prompt = app.world().resource::<ControlPrompt>();
        assert_eq!(
            prompt.context,
            ControlContextKind::Menu,
            "a cue is a surface saying it owns input; `Empty` hides the stick \
             the surface is steered with"
        );
        assert_eq!(
            prompt.menu_confirm.as_deref(),
            Some("Choose"),
            "and the confirm control wears that surface's own verb"
        );
    }

    /// The poison: no body AND no cue is still `Empty`.
    ///
    /// This is the case the original comment defends and the reason `Empty`
    /// exists at all — a genuine cold start, where nothing has claimed the
    /// screen and a control nobody can use must not be drawn. Widening the exit
    /// above to answer `Menu` unconditionally would delete the state and put a
    /// dead stick back on screen at boot.
    #[test]
    fn a_cold_start_with_no_cue_at_all_is_still_empty() {
        let mut app = app();
        // The resource EXISTS and is empty — the honest cold start, not the
        // absent-resource case a headless sim produces.
        app.init_resource::<ActiveUiCues>();
        app.update();

        let prompt = app.world().resource::<ControlPrompt>();
        assert_eq!(
            prompt.context,
            ControlContextKind::Empty,
            "no body and nothing claiming the screen: there is nothing to prompt"
        );
        assert_eq!(prompt.menu_confirm, None, "and no verb to invent");
    }

    /// While a frontend context owns input (the launcher, with no session and
    /// no gameplay actor), the frontend provider writes the prompt from the
    /// owning surface's cue and the sim-side rebuild yields — the touch
    /// overlay gets a labeled confirm control at the title screen.
    #[test]
    fn a_frontend_context_owns_the_prompt_with_its_own_cue() {
        use ambition_input::participant::{context_priority, ContextClaim};
        use ambition_input::{
            resolve_active_input_context, InputParticipant, ParticipantContexts, SeatInputContexts,
            LAUNCHER_CONTEXT,
        };

        let mut app = app();
        app.init_resource::<SeatInputContexts>();
        app.init_resource::<ActiveUiCues>();
        // Run the REAL pair the host schedules: resolver, frontend provider,
        // then the sim rebuild — proving the yield, not just the write.
        app.add_systems(
            Update,
            (
                resolve_active_input_context,
                publish_frontend_context_prompt,
            )
                .chain()
                .before(rebuild_control_prompt),
        );
        let mut contexts = ParticipantContexts::default();
        contexts.declare(ContextClaim::capturing(
            LAUNCHER_CONTEXT,
            context_priority::LAUNCHER,
        ));
        app.world_mut()
            .spawn((InputParticipant::primary(), contexts));
        app.world_mut()
            .resource_mut::<ActiveUiCues>()
            .declare(UiCue {
                context: LAUNCHER_CONTEXT,
                priority: context_priority::LAUNCHER,
                submit_label: "Play".to_owned(),
            });
        app.update();

        let prompt = app.world().resource::<ControlPrompt>();
        assert_eq!(
            prompt.context,
            ControlContextKind::Menu,
            "the launcher presents as a menu context to prompt surfaces"
        );
        assert_eq!(
            prompt.menu_confirm.as_deref(),
            Some("Play"),
            "the confirm control wears the launcher's verb"
        );
        assert!(prompt.entries.is_empty());
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

    /// Gate 4: the VISIBLE slot and the EXECUTABLE behavior
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
        use ambition_characters::brain::{MeleeActionSpec, SwipeSpec};
        use ambition_characters::control::ActorControl;

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
                ambition_platformer2d_actor_monolith::avatar::gate_worn_player_control,
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
            app.world()
                .get::<ActorControl>(body)
                .unwrap()
                .0
                .melee_pressed
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

    /// A removed cue resource must refresh the verb on an otherwise-quiet
    /// frame. `is_changed()` on an `Option<Res<T>>` says nothing at all about
    /// a `Some -> None` transition, so before the presence bits joined the
    /// cache key this frame was skipped and the prompt kept the dead cue's
    /// verb.
    #[test]
    fn a_removed_cue_resource_refreshes_the_menu_verb() {
        use ambition_input::InputContextId;
        let mut app = app();
        app.init_resource::<ActiveUiCues>();
        app.world_mut()
            .resource_mut::<ActiveUiCues>()
            .declare(UiCue {
                context: InputContextId("app.inventory"),
                priority: 150,
                submit_label: "Equip".to_owned(),
            });
        app.world_mut()
            .resource_mut::<NextState<GameMode>>()
            .set(GameMode::Paused);
        app.update();
        // A settle frame, so the mode transition's own change signal is spent.
        app.update();
        assert_eq!(
            app.world()
                .resource::<ControlPrompt>()
                .menu_confirm
                .as_deref(),
            Some("Equip")
        );

        // The ONLY event: the cue resource disappears. No mode change, no
        // subject, no authority edit.
        app.world_mut().remove_resource::<ActiveUiCues>();
        app.update();
        assert_eq!(
            app.world()
                .resource::<ControlPrompt>()
                .menu_confirm
                .as_deref(),
            Some("Select"),
            "the dead cue's verb is not served from the cache"
        );
    }

    /// The inverse transition: a cue resource INSERTED on a quiet menu frame
    /// takes effect immediately.
    #[test]
    fn an_inserted_cue_resource_updates_the_menu_verb() {
        use ambition_input::InputContextId;
        let mut app = app();
        app.world_mut()
            .resource_mut::<NextState<GameMode>>()
            .set(GameMode::Paused);
        app.update();
        app.update();
        assert_eq!(
            app.world()
                .resource::<ControlPrompt>()
                .menu_confirm
                .as_deref(),
            Some("Select"),
            "no cue resource -> the generic verb"
        );

        app.init_resource::<ActiveUiCues>();
        app.world_mut()
            .resource_mut::<ActiveUiCues>()
            .declare(UiCue {
                context: InputContextId("app.inventory"),
                priority: 150,
                submit_label: "Equip".to_owned(),
            });
        app.update();
        assert_eq!(
            app.world()
                .resource::<ControlPrompt>()
                .menu_confirm
                .as_deref(),
            Some("Equip"),
            "the fresh cue is picked up the frame it appears"
        );
    }

    /// Removing `SeatInputContexts` hands the prompt back to the sim. While
    /// a frontend context owns the prompt the rebuild yields; when the resource
    /// is REMOVED (host teardown — a transition no change detection reports),
    /// the next frame must re-derive the gameplay scheme rather than serve the
    /// pre-yield cache key.
    #[test]
    fn a_removed_input_context_hands_the_prompt_back_to_the_sim() {
        use ambition_input::participant::{context_priority, ContextClaim};
        use ambition_input::{
            resolve_active_input_context, InputParticipant, ParticipantContexts, SeatInputContexts,
            LAUNCHER_CONTEXT,
        };
        use bevy::ecs::system::RunSystemOnce;

        let mut app = app();
        let body = app
            .world_mut()
            .spawn((PlayerEntity, PrimaryPlayer, authorities(true, Some("swat"))))
            .id();
        app.world_mut().resource_mut::<ControlledSubject>().0 = Some(body);
        app.update();
        assert_eq!(
            app.world().resource::<ControlPrompt>().context,
            ControlContextKind::Gameplay,
            "baseline: the sim owns the prompt"
        );

        // A launcher context takes ownership; the frontend provider (simulated
        // by a direct write) puts its own prompt up, and the sim-side rebuild
        // yields on these frames.
        app.init_resource::<SeatInputContexts>();
        let mut contexts = ParticipantContexts::default();
        contexts.declare(ContextClaim::capturing(
            LAUNCHER_CONTEXT,
            context_priority::LAUNCHER,
        ));
        app.world_mut()
            .spawn((InputParticipant::primary(), contexts));
        app.world_mut()
            .run_system_once(resolve_active_input_context)
            .expect("the resolver runs");
        {
            let mut prompt = app.world_mut().resource_mut::<ControlPrompt>();
            prompt.context = ControlContextKind::Menu;
            prompt.entries = Vec::new();
            prompt.menu_confirm = Some("Play".to_owned());
        }
        app.update();
        assert_eq!(
            app.world()
                .resource::<ControlPrompt>()
                .menu_confirm
                .as_deref(),
            Some("Play"),
            "the frontend's prompt survives while it owns input"
        );

        // The ONLY event: the context resource disappears. Subject and
        // authorities untouched since the baseline frame.
        app.world_mut().remove_resource::<SeatInputContexts>();
        app.update();
        let prompt = app.world().resource::<ControlPrompt>();
        assert_eq!(
            prompt.context,
            ControlContextKind::Gameplay,
            "the sim re-derives the frame ownership returns"
        );
        assert_eq!(prompt.label_for(ControlSlot::Attack), Some("Swat"));
    }
}
