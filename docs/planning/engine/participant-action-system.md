# Participant/action system — remaining work

> **Verified against `cecd01ca` (2026-08-13).** Per-seat identity/context/device
> state, binding recipes/overrides, persisted rebinding, vendor-aware glyphs,
> inventory/debug/pause context claims, `SeatMenuFrames`, and the semantic action
> registry/module-contribution seam are implemented. Historical PA1–PA4 execution
> and decisions are archived under
> [`../../archive/planning-superseded/2026-08-13/`](../../archive/planning-superseded/2026-08-13/).

## Remaining architecture

- ▢ **Remove the seat-0 control split.** `ControlFrame`/`ControlFrameLatch` still
  carry a primary-seat special path while secondary seats use slot/seat state.
  Converge on one participant-keyed input channel without changing simulation
  semantics merely for naming symmetry.
  ⚠ **RE-READ THIS ONE BEFORE REMOVING ANYTHING (2026-08-26).** The split is
  still there and it is now DEFENDED rather than merely present: `input_systems.rs`
  carries six separate paragraphs on why the primary seat is the keyboard's, why
  `gamepad_only()` for every non-primary seat encoded the wrong thing, and why
  the primary seat *"has no equivalent hazard because GGRS overwrites"*. ⇒ the
  first question is no longer *how* to converge but whether this is still a
  SPLIT or has become a stated DESIGN — and this row's own last sentence
  (*"not merely for naming symmetry"*) is the test to apply.

- ✔ **Per-seat pause ownership — SHIPPED, verified at HEAD 2026-08-26.** This
  doc's header predates it. `PauseMenu` carries the owner
  (*"the seat currently driving the pause menu, if one owns it"*), reads
  `SeatMenuFrames`, and implements exactly the rule stated here: while OPEN it
  reads only the opening seat's frames, and while CLOSED every seat is a
  candidate with the first in slot order winning, *"which makes a simultaneous
  press deterministic rather than dependent on iteration luck"*. Guarded by
  `the_seat_that_paused_is_the_seat_that_drives_the_menu`
  (`pause_menu.rs:788`). ⚠ the global `MenuControlFrame` remains the fallback on
  purpose — a standalone demo composes the shell without the participant
  pipeline, and there the one global frame IS the only seat.

- ◐ **Dialogue through participant contexts only.** Finish the ruling that
  dialogue is per-seat by default rather than globally suspending the world.
  Experiences that intentionally stop world time should request that policy
  explicitly.

  ✔ **THE EXPLICIT-POLICY HALF SHIPPED**: `DialogueStopsTheWorld` is a resource,
  it defaults to `false`, and `GameMode::stops_the_world` consults it. Body
  simulation is not suspended either — `ambition_conversation::rules`'s own
  header says so, which is what lets contact and damage END a live conversation.

  ⛔⛔ **AND THE OTHER HALF IS BLOCKED BY ONE MODE ANSWERING TWO QUESTIONS, ONLY
  ONE OF WHICH LEARNED THE POLICY. Measured 2026-08-26:**

```text
GameMode::stops_the_world(self, policy)   Dialogue => policy.0     ← policy-aware
GameMode::allows_gameplay(self)           matches!(self, Playing)  ← unconditional
```

  ⇒ under the DEFAULT policy the world keeps running during a conversation and
  **every seat's gameplay input is refused anyway**, because the input gate never
  asks. So *"dialogue is per-seat by default"* is not merely unbuilt — it is
  currently INEXPRESSIBLE: `GameMode` is one global `State`, and the gate that
  would have to become per-seat is a bare `matches!` on it.
  ⇒ **the real remaining work is naming what a per-seat dialogue mode IS** (a
  second seat playing while the first talks needs the gate keyed by seat, not a
  policy flag on a global mode), not threading `DialogueStopsTheWorld` into a
  second function. ⚠ and the present behaviour is coherent even if it is not the
  ruling: the world runs, you cannot act, and being hit cuts the conversation
  short — do not "fix" the gate without deciding the mode.

- ▢ **Unify semantic menu activation.** Controller submit, virtual-touch submit,
  and pointer release should produce one semantic activation seam. Pointer
  press/release-with-drag-cancel is already shared; backend-specific select
  consumption remains.
  ⭐ **AND "REMAINS" NOW HAS A NUMBER, measured 2026-08-26: the shared seam has
  exactly ONE adopter.** `ambition_ui_nav::resolve_selectable_row_interaction`
  (press/release with drag cancel, `ROW_TAP_SLOP_PX`) is used only by
  `ambition_dialog` (two call sites). `ambition_game_shell` reads the raw
  `MenuControlFrame::select` flag and routes it itself at three separate places —
  `input.rs` maps it to `confirm`, `startup_acknowledge` AND `loading_continue`,
  and `pause_menu` consumes it directly. ⇒ **the seam is not missing, it is
  UNADOPTED**, which makes this a migration with a countable finish line rather
  than a design question. ⛔ mind the three shell meanings: one flag currently
  answers three different questions there, so moving it needs those to stay
  distinguishable.

- ◐ **Move directional repeat/focus/wrap behind `ambition_ui_nav`.** Preserve the
  existing navigation semantics while removing backend-specific duplication.
  ⭐ **MEASURED 2026-08-26, and the vague half is now a NAMED DIFFERENCE.** The
  crate exists and carries all three concepts: `MenuFocusState`,
  `move_next_wrapping` / `move_previous_wrapping`. `ambition_dialog` adopts
  `MenuFocusState` (runtime + systems). ⛔ `ambition_game_shell`'s pause menu does
  NOT — it keeps its own cursor and CLAMPS it
  (`menu.cursor = (menu.cursor + 1).min(rows.len() - 1)`, `pause_menu.rs:438`).
  ⇒ **two menus in one game disagree about what happens at the end of a list**:
  the dialogue picker is on the shared vocabulary and the pause menu stops dead.
  ✔ **DONE 2026-08-26 — the pause menu is on `ListCursor` now**, and the wrap
  rule needed no decision: the crate's ONLY movement verbs wrap, the dialogue
  picker beside it already wrapped, and `ListCursor`'s own doc names
  *"pause-menu"* FIRST among the callers whose rules it exists to own. ⭐ it also
  fixed a second thing for free — `apply_directional` answers whether the
  selection CHANGED, so a press at either end no longer plays a move cue for a
  cursor that did not move.

  ⛔⛔ **AND THE ADOPTION EXPOSED A HOLE THAT MATTERS MORE THAN THE CHANGE. The
  ten tests in `pause_menu::tests` never run, and do not pass.** `mod pause_menu`
  is behind `feature = "basic_presentation"`, which is NOT default, so
  `cargo test -p ambition_game_shell` compiles neither the module nor its tests —
  the suite reports 45 and every one of them comes from `src/tests.rs`. Under
  `--features basic_presentation` the module builds and **every pause test panics
  the same way**: *"Parameter `::messages` failed validation: Message not
  initialized"*, including tests nothing touched
  (`confirming_the_mute_row_toggles_mute`). ⚠ that isolated build is
  known-awkward — this crate's own `Cargo.toml` says `ui_api` drags in `winit`
  there — so the honest statement is that these tests have NO configuration in
  which they are known to run. ⇒ **third feature-gate hole found in one day**
  (`ambition_demo_smash --lib` was red for days; `ambition_conversation`'s dialog
  road runs 25 tests by default and 35 with `--features ui`). A per-crate `cargo
  test` is evidence about a FEATURE SET, not about a crate.
  ⚠ REPEAT is deliberately elsewhere and should probably stay: it lives in
  `MenuInputState::step` in `ambition_input`, driven by the user's
  `menu_repeat_initial_delay` / `menu_repeat_interval` settings — a repeat is an
  INPUT cadence, not a list-navigation rule.

  ⛔⛔ **AND A FOURTH, IN THE CRATE AT THE CENTRE OF THIS DOC: `ambition_input`
  runs 56 tests by default and 125 with `--all-features`.** Sixty-nine tests of
  the participant / binding / menu machinery every item on this page is about,
  and the plain per-crate command sees none of them. ⚠ they PASS — unlike the
  shell's — so this one is a VISIBILITY hole rather than hidden red; but any item
  here that gets worked and "verified" with `cargo test -p ambition_input` has
  verified less than half of it.

  ✔ **AND THE REPAIR WAS ONE LINE, so 26 TESTS CAME BACK.** The fixture
  installed `ShellPauseMenuPlugin` alone and registered only `ShellCommand` — a
  composition that cannot exist, because `drive_shell_pause_menu` WRITES
  `ShellAbandonRequested` and the SHELL plugin owns that channel
  (`plugin.rs:48`). Registering it there took the crate from **45 tests to 72**
  under `--features basic_presentation`: the ten pause tests plus sixteen more
  from `basic_presentation` that had never run either. ⇒ the wrap guard lives
  where the code does now
  (`the_pause_cursor_wraps_at_both_ends_like_every_other_list`), and poisoning
  the clamp back reddens it.

- ▢ **Finish context migration.** Inventory/specialized menus beyond the current
  cue ownership, a `VEHICLE` context, and loading/retry input remain outside the
  normal participant-context path. For loading/retry, fix the schedule ownership
  seam rather than introducing a cycle.
  ⭐ **BOTH HALVES CONFIRMED AT HEAD 2026-08-26, and one of them needs a question
  answered before it needs code.** `ControlContextKind` has exactly four
  variants — `Gameplay`, `Menu`, `Dialogue`, `Empty` — so there is no `VEHICLE`,
  and mounts DO ship, which means a rider on a shark reads the same prompt as a
  fighter on foot. ⚠ **but a context earns its place by CHANGING THE PROMPT**:
  before adding a variant, measure whether a mounted body's verbs actually
  differ from an unmounted one's. If the same buttons do the same things, the
  variant buys a label and costs a seam.
  ⛔ the loading/retry half is confirmed and is not a question: `game_shell`'s
  `input.rs` maps the raw `MenuControlFrame::select` straight onto
  `startup_acknowledge` and `loading_continue`, which is the same unadopted-seam
  finding as the activation item above — one flag answering three questions.

- ✔ **Pad-specific calibration filtering with shared bindings — SHIPPED,
  verified at HEAD 2026-08-26.** Bindings remain machine-wide by decision, and
  the filters follow the pad: `filters_for_seat` resolves
  `ControlFilters::for_pad(&settings.controls, devices.gamepad_style_for(slot))`,
  and BOTH roads take it — the menu decode and the production gameplay seat loop.
  The site says why in one sentence: *"a deadzone is a fact about the stick in
  somebody's hands"*, after player two's drifty 360 pad ran on whatever suited
  player one's DualSense.
  ⚠ **one second answer survives and is worth knowing about**:
  `read_gameplay_control_frame` (no `_with_settings`) builds
  `ControlSettings::default()` and has two callers — `ambition_sim_view::facts`
  (the blink-aim preview) and the debug overlay. Harmless today because the field
  it reads is a BUTTON and buttons are not deadzoned; it becomes a real
  disagreement the day either caller reads an axis.

- ▢ **Make provider-defined semantic actions fully usable end to end.** The code
  now has `SemanticActionId`, `ActionRegistry`, `InstalledActions`, and
  `ModuleDraft::actions` (including a tested external `grapple` registration),
  but the physical input map/cue/touch path still bottoms out in the finite
  built-in platformer action enum. A provider action should be registerable,
  bindable, presentable, and consumable without editing core action vocabulary.

  ⭐ **THE GATE, WITH ITS SIZE, measured 2026-08-26.** The registry stores
  DESCRIPTIONS — `SemanticActionDef { id, capability, kind, contexts, doc }` —
  and the test that proves a capability *"registers its own action without
  touching the engine enum"* asserts exactly that and nothing about binding it.
  The physical road is leafwing keyed by the closed enum:

```text
Platformer2dInputActionMonolith variants                 35
`Platformer2dInputActionMonolith::` references          288 across 21 files
`InputMap<Platformer2dInputActionMonolith>`   the only thing a binding can name
`ActionState<Platformer2dInputActionMonolith>` the only thing a reader can poll
```

  ⇒ **a registered `grapple` is describable, and unbindable and unreadable**, so
  the four verbs this item asks for split two-and-two. ⛔ this is NOT an `S`: the
  gate is the KEY TYPE, so closing it means a second `InputMap` over a dynamic
  key or making the key generic, and 288 references decide how far that reaches.
  ⇒ price it as a carve, and start by asking whether the cue/touch/presentation
  path can key on `SemanticActionId` while the leafwing map stays closed —
  presentable and consumable may be reachable well before bindable is.

  ⛔⛔ **AND THAT QUESTION HAS AN ANSWER, MEASURED 2026-08-26: THEY CANNOT, AND
  IT IS NOT ONE ENUM BUT THREE.** `SemanticActionId` appears NOWHERE in
  `ambition_sim_view` or `ambition_touch_input` — each road carries its own
  closed vocabulary:

```text
binding       Platformer2dInputActionMonolith   35 variants  (leafwing key)
presentation  ControlSlot                        8 variants  (`PromptEntry.slot`,
                                                  `action_scheme.rs:20`)
touch         TouchActionButton                 20 variants  (`layout.rs`, mapped
                                                  BACK onto the leafwing enum)
```

  ⇒ a provider action has to appear in THREE closed enums to be bindable,
  presentable and pressable, and the second map above only opens the first. ⚠ so
  *"presentable before bindable"* is the wrong order — presentation is the
  road with its OWN enum AND a hand-written mapping table, and it is the
  expensive one.

  ⭐⭐ **AND THERE IS A CANDIDATE THAT NEEDS NO ERASURE, checked against the
  vendored trait 2026-08-26. A SECOND MAP, NOT A WIDER ENUM.** `InputMap` is
  already generic — `InputMap<A: Actionlike>` — so nothing stops a composition
  installing a second one beside the engine's, keyed by a type a provider can
  mint. The bound is satisfiable by a plain newtype:

```text
Actionlike: Debug + Eq + Hash + Send + Sync + Clone
          + Reflect + Typed + TypePath + FromReflect + 'static
          + fn input_control_kind(&self) -> InputControlKind
                          (leafwing-input-manager 0.20.0, lib.rs:101)
```

  A `String`-backed id derives every one of those. The only non-derivable member
  is `input_control_kind`, which takes `&self` — so the key must CARRY its kind
  rather than look it up:

```text
struct ProviderAction { id: SemanticActionId, kind: ActionControlKind }
```

  ⭐ and `SemanticActionDef` already holds exactly that `kind`, so the registry
  mints the key and the registry's own uniqueness rule keeps `Hash`/`Eq`
  consistent — one kind per id, so two keys can never disagree about the same
  action.

  ✔ **AND IT IS COMPILED RATHER THAN ARGUED**:
  `a_registry_minted_key_satisfies_leafwing_without_erasure` (`semantic.rs`)
  registers `grapple`, mints a key from the registration, binds it in an
  `InputMap` and reads the binding back. Poisoned by binding a different key: it
  reports *"a provider-minted key bound nothing"*.

  ⛔ **AND RUNNING IT FOUND A COST THE REASONING MISSED: THE KIND HAS TO BE
  MIRRORED.** Neither enum can be a field of a hashed, reflected key — the
  registry's `ActionControlKind` has no `Hash` and no `Reflect`, and leafwing's
  `InputControlKind` has no `Eq` and no `Hash`. ⇒ a real implementation carries a
  small three-variant mirror beside the registry rather than widening either
  upstream type, and that mirror is the honest price of this route.

  ⇒ **that is `InputMap`/`ActionState` reached with NO `Any`, NO `TypeId`, NO
  service locator, and NO edit to the 35-variant enum**, which is the combination
  every previous attempt failed. ⛔ it is still a carve — two maps means two
  reader paths and a decision about which wins a conflict — but it is a candidate
  where this item had none, and it is worth checking against D168's
  `StateMachineCfg`, which is the same closed-enum shape with an extra
  obligation: that one is WIRE-ENCODED, so a second keyspace there also needs a
  codec story.

- ▢ **Author input schemas/assets where it improves tooling.** Do this through
  the same registry/binding model rather than adding another settings authority.
  ⛔ **GATED BY THE ITEM ABOVE, not by effort — 2026-08-26.** *"The same
  registry/binding model"* is exactly the half that does not exist: the registry
  describes, and only the closed enum binds. An authoring surface written against
  the registry today could describe actions nothing can bind, and one written
  against the enum would be the *"another settings authority"* this sentence
  refuses. ⇒ nothing to do here until the key type moves; and *"where it improves
  tooling"* means it wants a tooling customer as well.

## Exit

A local participant is represented by participant/seat/channel facts rather than
by a privileged primary-player identity, and a provider can contribute a new
semantic action all the way from authoring through physical binding and UI cue to
consumption without a core-engine edit.

⚠ **NEITHER CLAUSE IS MET AT HEAD, and the two are held by different things
(2026-08-26).** The first is held by the seat-0 split — which is now DEFENDED in
six paragraphs of `input_systems.rs` rather than merely present, so the honest
next step is deciding whether it is a split or a design, not removing it. The
second is held by ONE key type: a provider action is describable and neither
bindable nor readable, because `InputMap` and `ActionState` are keyed by a
35-variant enum with 288 references across 21 files. ⇒ **this exit is two
questions, not one milestone**, and five of the nine items above are already
closed or partly closed without touching either.
