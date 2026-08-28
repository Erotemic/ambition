# Participant/action system — remaining work

> **Verified against `cecd01ca` (2026-08-13).** Per-seat identity/context/device
> state, binding recipes/overrides, persisted rebinding, vendor-aware glyphs,
> inventory/debug/pause context claims, `SeatMenuFrames`, and the semantic action
> registry/module-contribution seam are implemented. Historical PA1–PA4 execution
> and decisions are archived under
> [`../../archive/planning-superseded/2026-08-13/`](../../archive/planning-superseded/2026-08-13/).

## Remaining architecture

- ✔ **Remove the seat-0 control split — THE SPLIT THIS ROW NAMES IS GONE.
  Verified by reading HEAD, 2026-08-28.** The row asked whether this was still a
  split or had become a stated design, and its own test was *"not merely for
  naming symmetry"*. Applied:

  ⭐ **DELIVERY IS ONE ROAD.** `drive_slot_frame(world, slot, frame)` is the
  channel every seat uses, and `input_drive.rs` says in its own words that this
  *"was TWO functions with the same shape"* and that *"seat zero's latch has
  since become row zero of the same table every other seat uses"*.
  `drive_control_frame` survives as the NAME for the primary seat and is
  documented as *"a convenience over this, not a second road with different
  rules"*. Measured the same day by a two-seat test driving slots 0 and 1
  through the identical call.

  ⭐ **AND THE LOOPS ARE PER SEAT.** `input_systems.rs` iterates
  `0..SlotControls::MAX_SLOTS` and resolves each seat's own gravity, gestures and
  interact buffer — with a comment recording that the interact buffer *"was
  `slot_gestures.primary_mut` too"*, so *"a second player standing at a door
  pressed a button that was buffered for nobody"*. That was the split, and it was
  fixed.

  ⛔ **WHAT STILL NAMES `PlayerSlot::PRIMARY` IS A FALLBACK, NOT A CHANNEL.**
  Every surviving site is `body_driving_seat(slot).or_else(|| PRIMARY.then(…))`
  or `driving_slot(body).unwrap_or(PRIMARY)` — the answer for a body no
  participant drives, which `acting.rs` states outright: it *"preserves the
  behaviour every existing single-player fixture depends on"* and is *"NOT a
  claim that a body with no participant may consume the primary seat's input
  during play"*.

  ⛔ **AND THE DEVICE POLICY IS THE STATED DESIGN THIS ROW WARNED ABOUT.** The
  keyboard belonging to the primary seat is six defended paragraphs in
  `input_systems.rs`, not an accident of the old channel. Converging it would be
  naming symmetry, which this row forbids.

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

- ◐ **Unify semantic menu activation.** Controller submit, virtual-touch submit,
  and pointer release should produce one semantic activation seam.
  ⛔ **THE 2026-08-26 MEASUREMENT BELOW WAS PART STALE AND PART MIS-SCOPED; RE-MEASURED
  2026-08-28 against HEAD.** It read "the shared seam has exactly ONE adopter
  (`ambition_dialog`), and `ambition_game_shell`'s pause menu consumes raw
  `MenuControlFrame::select` itself". Both halves of that are wrong today:
  `shell_pause_menu_pointer` consumes `MenuActionActivated<PauseEntry>`
  (`pause_menu.rs:478`), and `ambition_menu`'s pointer bridge
  (`publish_bevy_ui_menu_actions`) already runs on `ambition_ui_nav::PressArm` — the
  SAME tap-geometry primitive `resolve_selectable_row_interaction` uses. The three
  `input.rs` edges (`confirm`, `startup_acknowledge`, `loading_continue`) are three
  different questions asked in three different app phases, not one seam fanned out.
  ⭐ **What was actually divergent was the POLICY on top of that shared gesture, and
  the divergence had a user-visible cost.** `MenuTapMode` has three arms and ships
  defaulting to `SingleTapWithDestructiveGuard`, whose own doc names its reason: *"a
  stray touch on Quit"*. Only `ambition_ui_nav`'s index-addressed helper consulted it.
  Every menu drawn by the pointer bridge — the pause menu, with `Abandon`,
  `QuitToTitle` and `QuitToDesktop` on it — activated on the first release. The
  setting's default guarded nothing on the exact row it was written for.
  ✔ **DONE 2026-08-28 — one policy, two call shapes.** `MenuTapMode::resolve_press`
  is now generic over an opaque row identity (it only ever asked whether two presses
  landed on the same row; the `usize` was never an ordinate), so the entity/action-
  addressed bridge calls the same function the index-addressed helper does. A menu
  declares its risky rows once with `MenuDestructiveActions<Action>` — destructiveness
  belongs to the action, not to the drawn rect, so this cost one registration in
  `ShellPauseMenuPlugin` rather than a flag at 21 `MenuPage::control` call sites — and
  a menu that registers nothing stays single-tap throughout.
  ▢ **STILL OPEN: the non-pointer half.** Controller submit and virtual-touch submit
  reach activation without passing through any tap policy, so the destructive guard
  is a POINTER guard today. Whether it should also apply to a gamepad A-press is a
  feel question, not a plumbing one — a controller cannot stray-tap the way a thumb
  can, so the honest default may well be "pointer only". ⛔ mind the three shell
  meanings: `confirm` / `startup_acknowledge` / `loading_continue` answer three
  different questions, so anything that merges them needs those distinguishable.

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
  ✔ **MEASURED 2026-08-28, AND THE ANSWER IS NO VARIANT.** The verbs DO differ —
  the row's own test is met — but a `VEHICLE` context is the wrong instrument for
  it, and the measurement says why. `Menu` and `Dialogue` exist because a SURFACE
  owns the participant's input; a rider is still driving a body through gameplay.
  The prompt is not built from the context at all: `rebuild_control_prompt` derives
  it from the subject's live authorities through the same `derive_action_scheme`
  the gameplay routing gate calls, precisely so a button's label and what it fires
  cannot drift. A context variant would have added a second, parallel answer to a
  question one authority already owns.
  ⛔ **The actual defect was a PROMPT LIE, and it was four buttons wide.**
  `body_step` zeroes the stick and clears every `MovementAction`
  (`Jump`/`Burst`/`Blink`/`FlyToggle`/`FastFall`) the moment `PoseOwnedExternally`
  is on the body, and the movement kernel refuses the buffered burst on top of
  that. None of that touches `AbilitySet`, so the derive kept advertising Jump,
  Burst, Blink and Fly to a rider whose presses were already being thrown away —
  beside Attack, which really does work from the saddle, looking identical.
  ✔ **FIXED by masking the authority, not by adding a context:**
  `AbilitySet::while_pose_is_held` (exhaustive, so a new ability must be
  classified rather than defaulting into "still available"), applied in the prompt
  derive and NOT in the routing gate — a press made a moment before the constraint
  took the body is input memory the player is entitled to, so the refusal stays
  ⛔ FORBIDDEN, NOT ERASED where it is. Boarding adds a component and dismounting
  removes one, so the fact also had to join the rebuild's presence key.
  ⭐ **One thing the mask deliberately does NOT clear: `move_horizontal`.**
  `steer_mount_from_rider` copies exactly `locomotion`, `velocity_target` and
  `facing` across the saddle, so a rider's lean is the one intent that still
  reaches the world. The same function states the boundary: *"the jump edge is the
  mount's own to decide."* ⚠ the first draft of the mask cleared the stick too,
  which would have been a new false statement in the other direction.
  ▢ the loading/retry half stands, but its 2026-08-26 framing does not: it called
  `game_shell`'s `input.rs` mapping of `MenuControlFrame::select` onto
  `startup_acknowledge` and `loading_continue` *"the same unadopted-seam finding as
  the activation item above — one flag answering three questions"*. Re-measured
  2026-08-28 with that item: those ARE three different questions, asked in three
  different app phases, and `shell_action_edges` already publishes them as three
  named fields. The open work here is the SCHEDULE ownership seam the row names in
  its first paragraph — loading/retry input arriving outside the normal participant
  context path — not a flag that needs splitting.

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
  ⛔⛔ **THAT PRICE WAS NOT REAL — measured 2026-08-28 by building it.** The
  sentence quietly treated two very different types as one obstacle.
  `InputControlKind` is leafwing's and genuinely cannot be a field; but
  `ActionControlKind` is OURS, three lines above in the same file, and it is a
  fieldless enum — `Hash` and `Reflect` derive for free. The mirror only existed
  because the check was written inside a test function, where reaching for a local
  type is easier than editing the one four hundred lines up. ⇒ no mirror ships.
  ✔ **AND THE KEY IS PRODUCTION NOW, not a shape argued for inside a test.**
  `ProviderAction { id: String, kind: ActionControlKind }` lives beside the
  registry with its `Actionlike` impl, and `ActionRegistry::key` is the ONLY way to
  get one. That last part is not tidiness: the kind is part of the hash, so two
  hand-built keys for one action could disagree about its shape and miss each other
  in the map. Minting through the registry extends its one-kind-per-id rule to the
  bindings, and an unregistered id mints `None` rather than a binding to a slot
  nobody polls. The test that used to hand-build the key now asks the registry for
  it, and its two local types are deleted.
  ▢ **WHAT REMAINS IS THE ROUTING**: nothing installs an `InputMap<ProviderAction>`
  in production, so a provider action is registerable, bindable, and still neither
  presentable nor consumable.
  ✔ **AND THE CARVE WARNING SHRANK WHEN MEASURED, 2026-08-28.** *"Two maps means
  two reader paths and a rule for which wins a conflict"* assumed the maps live
  somewhere separate. They do not: `InputMap` and `ActionState` are COMPONENTS on
  the `InputParticipant` entity — `input_systems.rs` spawns both seats' worth in
  one tuple each (lines ~136 and ~383) and every reader is already a query over
  that entity. A second map is a second component in the same tuple, read by the
  same pass. ⇒ there is no precedence rule to invent: a physical key bound in both
  maps fires both actions, which is a BINDING mistake for a rebind UI to catch, not
  an architectural ambiguity.
  ⛔⛔ **THE ACTUAL BLOCKER IS UPSTREAM OF THE MAP, AND THE TWO REMAINING ITEMS ARE
  ORDERED BACKWARDS BECAUSE OF IT.** Nothing can say *"pulse is on G"*.
  `SemanticActionDef` describes an action and carries no binding; `BindingRecipe`
  binds, and it is preset-based over the closed enum. So an `InputMap<ProviderAction>`
  has nothing to put in it. The last item — *"Author input schemas/assets"* — is
  marked **GATED BY** this one; measured, the dependency runs the other way for
  this half: a place to AUTHOR a provider binding is what unblocks the routing, not
  the reverse.
  ⭐ **THERE IS A REAL CUSTOMER, not a hypothetical one.** `examples/capability_demo`
  is the capability-integration sentinel, it registers `PULSE_ACTION`, and its own
  module doc names the workaround: *"The action is declared, but input currently
  reaches the capability by writing `PulseRequested` until semantic actions own
  device bindings."* Closing this deletes that sentence. ⚠ `PulseRequested` itself
  STAYS — its doc is right that a scripted sequence or an AI writes it the same way.
  What is missing is the router that writes it from a press.

  ✔✔ **THE ROAD IS OPEN AS OF 2026-08-28, and it is checked end to end.**
  `a_registered_action_bound_to_a_key_comes_back_as_a_seat_press` registers an
  action the engine has never heard of, binds it to a key, presses the key and gets
  a `SemanticActionPressed` back — no `Any`, no `TypeId`, no variant added to the
  35-variant enum. Three pieces, all in `ambition_input`:

```text
ProviderBindings                     the composition's map. SEPARATE from the
                                     registry: a capability DESCRIBES its action,
                                     the game it is installed into decides the key
install_provider_bindings_on_seats   PreUpdate, before leafwing resolves. A SYNC,
                                     not a spawn edit — a capability installed
                                     after the seats exist still reaches them
publish_provider_action_edges        `InputSet::Route`. `just_pressed`, sorted by
                                     seat id so two seats pressing on one frame
                                     publish in the same order every run
```

  ⛔⛔ **AND A SECOND `InputManagerPlugin` IS NOT HOW YOU ADD A SECOND KEYSPACE.**
  `InputManagerPlugin::<A>::build` guards only `CentralInputStorePlugin`; it adds
  `clear_central_input_store` and `filter_captured_input` UNCONDITIONALLY, so a
  second action type registers both TWICE — and `clear_central_input_store` DRAINS
  the store. The app's own `no_system_is_registered_twice_in_one_schedule` caught it
  and stated the class in its own words: a doubled system that drains or decays is a
  rate bug that reads as bad tuning. The host registers the three GENERIC-half
  systems instead (`tick_action_state::<A>`, `update_action_state::<A>`,
  `release_on_input_map_removed::<A>`) in the sets the plugin puts them in.
  ⚠ this is an upstream limitation, not a design choice: leafwing 0.20 has no
  "additional action type" entry point.
  ⛔ **AND THE TEST EARNED ITS KEEP ON THE FIRST RUN: BOTH SYSTEMS IN `PreUpdate`
  PUBLISHED ON NO FRAME.** `InputSet::*` is configured in `Update`, so an `in_set`
  in `PreUpdate` orders nothing at all — the edge ran before leafwing had resolved
  anything, silently, every frame. The split is now stated where it lives: the map
  must land before `InputManagerSystem::Update`, and the edge is a routed semantic
  like every other one.
  ▢ **WHAT IS STILL THE COMPOSITION'S**: which seat drives which body. The demo
  refuses to know that on purpose (it carries `PulseBody`, not an actor-domain
  type), so the last hop — `SemanticActionPressed` → `PulseRequested { body }` —
  belongs to whoever mounts both, and that is the correct place for it.
  ◐ **AND PRESENTABLE IS A DIFFERENT KIND OF PROBLEM THAN THE PLAN RECORDS —
  re-measured 2026-08-28.** The *"a provider action has to appear in THREE closed
  enums"* table counts `ControlSlot` and `TouchActionButton` as arbitrary limits
  alongside the device enum. They are not the same kind of thing. The device enum
  is a closed vocabulary with no reason to be closed; the other two are
  DESCRIPTIONS OF HARDWARE — `ControlSlot` is the buttons a controller has (11 of
  them at HEAD, not the 8 the table says), and `TouchActionButton` is the buttons
  that fit on a phone screen. And the touch mapping runs the direction the table
  implies it does not: `touch_button_slot` goes BUTTON → slot → `prompt.label_for`,
  so the overlay draws its fixed set and asks the prompt what each one is called.
  ⇒ **a provider action becomes presentable by being ASSIGNED a slot, not by
  widening one.** Adding a 21st on-screen button is a layout decision about finite
  screen space; giving a provider action a face button is a decision about a finite
  controller. Neither is plumbing, and neither is expensive in the way the note
  meant — *"presentation is the expensive road"* was measuring a hand-written
  mapping table that turns out to point the other way.
  ⚠ **and keyboard already sidesteps it entirely**: `ProviderBindings` binds
  `pulse` to a key with no slot in sight, which is why the road above works today.
  What has no answer yet is a provider action on a PAD or a PHONE, and that answer
  is a design call about which finite button it takes.

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
