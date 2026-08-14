# Mobile Dialogue Touch Interaction Failure Report

## Scope

Investigate and repair dialogue interaction on touch devices, with a Pixel 5–class phone as the primary reference device.

The observed symptoms are:

1. Dialogue choices are difficult or unreliable to activate by touching them directly.
2. The right-hand action-button half of the on-screen gamepad disappears during dialogue.
3. The missing buttons would otherwise provide a reliable way to confirm the currently selected dialogue option.
4. Some hidden controls may still respond to touches in their former screen regions.
5. Long choice lists are likely worse than short lists.

This appears to be a combination of several independently real bugs rather than one isolated layout issue.

---

# Executive Summary

There are four principal problems.

## 1. Dialogue rows use a two-step touch interaction

On Android, touching a dialogue row first selects or “arms” it. A second touch is required to activate it.

That is already unusual for a mobile dialogue list, where users generally expect one tap to choose a response.

## 2. The selected row can move between the first and second taps

Long dialogue lists show a window around the selected option. When the first tap selects a new row, the visible window recenters and the list is rebuilt.

The row the user just touched can therefore move to another vertical position before the required second tap. The second tap may hit a neighboring response instead of confirming the intended response.

This is a deterministic moving-target interaction.

## 3. The touch HUD hides the controls that dialogue explicitly allows

The dialogue input prompt marks Jump and Interact as available confirmation actions, but a separate visibility gate hides them whenever gameplay does not own the active input context.

Dialogue owns a non-gameplay context, so the right-side action-button cluster disappears exactly when it is supposed to become a dialogue confirmation interface.

The same context rule also hides the virtual stick, even though directional input can be used for menu navigation.

## 4. Hidden touch controls may still participate in raw hit-testing

Visual visibility and touch hit-testing appear to use different conditions.

The HUD can hide a button because gameplay does not own the context, while raw touch handling still considers that action available according to the dialogue prompt. This can leave invisible interactive regions over the dialogue.

The resulting behavior can include:

* Touching an apparently empty area advances dialogue.
* Touching a visible dialogue choice activates an invisible HUD action instead.
* Some dialogue rows appear intermittently untappable.
* The visual layout and interactive layout disagree.

These problems should be fixed together. Merely showing the buttons again would leave direct row touching broken, and merely changing dialogue rows to single tap would leave invisible HUD interception and layout overlap.

---

# Relevant Code Areas

The current investigation points to these areas.

## Dialogue touch policy

* `crates/ambition_input/src/settings.rs`

  * Android touch confirmation policy.
  * `TapToSelectThenConfirm` or equivalent default.
  * Platform-specific menu interaction settings.

* `crates/ambition_dialog/src/systems.rs`

  * Conversion of dialogue choices into selectable menu entries.
  * Dialogue choices currently appear to be treated through the generic menu confirmation policy.
  * Choices are non-destructive, but still inherit two-step confirmation behavior.

## Visible choice-window calculation

* `crates/ambition_ui_nav/src/list.rs`

  * Visible list range calculation.
  * Selected-item centering behavior.

* `game/ambition_content/src/presentation/dialog.rs`

  * Dialogue option rendering.
  * Maximum visible option count on phone layouts.
  * UI reconstruction after selection changes.
  * Dialogue panel sizing and Z order.

## Touch HUD visibility and input collection

* `crates/ambition_touch_input/src/bevy_plugin.rs`

  * `sync_touch_button_visibility_from_prompt`
  * `sync_touch_stick_visibility_from_context`
  * Context-based HUD visibility.
  * Prompt-based action availability.
  * Raw screen-rectangle hit-testing.
  * Touch HUD global Z index.
  * Dialogue/menu action relabeling.

* `crates/ambition_touch_input/src/layout.rs`

  * Right-side action-cluster geometry.
  * Stick geometry.
  * Margins and screen-relative placement.

## Input context ownership

Look for:

* `SeatInputContexts`
* `gameplay_owned()`
* active or primary input context selection
* `ControlContextKind::Dialogue`
* `ControlContextKind::Menu`
* `touch_action_available`
* `is_menu_confirm_button`
* `is_menu_button`
* `always_available`
* action masking after raw hit-testing

---

# Detailed Failure 1: Two-Tap Dialogue Choices

## Current behavior

Android uses a menu interaction policy equivalent to:

1. Touch a row to select it.
2. Touch the selected row again to confirm it.

This may make sense for a destructive or unusually dangerous menu action, but dialogue responses are ordinary non-destructive actions. A mobile user expects touching a response to choose it.

The current visible instruction is also insufficient. A prompt such as “Confirm: select” does not communicate that a row must be touched twice.

## Why this is especially harmful

A touchscreen does not have the same hover and focus feedback as a mouse or controller.

The user sees a response, touches it, and observes only a selection-state change. This can look like:

* The tap did not work.
* The interface is lagging.
* The row is merely highlighted but cannot be activated.
* Another input method is required.

## Desired behavior

For ordinary dialogue responses:

1. Record the row touched on pointer-down.
2. Track movement while the pointer is held.
3. If movement exceeds the drag threshold, treat the gesture as scrolling or navigation.
4. If movement stays within the threshold and release occurs on the same row, activate that response.
5. Do not require a second independent tap.

This should be tap-on-release rather than immediate activation on pointer-down. Tap-on-release allows drag cancellation and avoids accidental activation when the user intended to scroll.

## Destructive actions

If the generic menu system needs two-step confirmation for destructive actions, preserve that behavior only where explicitly requested.

Dialogue options are currently passed as non-destructive and should not inherit a destructive-style confirmation policy.

---

# Detailed Failure 2: Moving Target After Selection

## Current behavior

Phone layouts show only a limited number of dialogue choices, apparently three in portrait mode.

The visible range is calculated around the selected index. When selection changes, the range recenters around that item.

The dialogue UI is then rebuilt.

## Concrete example

Assume a long list currently displays:

```text
Option 0
Option 1
Option 2
```

The user touches `Option 2`.

The first touch only selects it.

The list recenters around the new selection and becomes:

```text
Option 1
Option 2
Option 3
```

`Option 2` has moved from the bottom row to the middle row.

The user touches the same physical screen location a second time, intending to confirm `Option 2`. That screen location now corresponds to `Option 3`.

The second touch selects the wrong option instead of confirming the original one.

## Consequences

* The center visible row may work more reliably than the top or bottom row.
* The bug becomes much more apparent in long merchant or conversation menus.
* Users may report that the wrong choice is selected.
* Users may repeatedly chase an option as it moves.
* The behavior can look random even though it follows deterministic recentering.

## Required correction

Single-tap activation removes most of this failure, but list stability still needs attention.

During an active pointer gesture:

* Do not recenter the list.
* Do not destroy and recreate the touched row.
* Preserve the touched choice’s identity and screen position until release or cancellation.
* Resolve activation using a stable choice identifier, not the entity created for one rendered frame.

For controller or keyboard navigation, centering the selection may remain appropriate. Touch selection and directional navigation do not need to produce identical intermediate layout behavior.

---

# Detailed Failure 3: Dialogue Confirmation Buttons Disappear

## Intended behavior

The touch prompt logic explicitly treats some gameplay actions as dialogue or menu confirmation actions.

The relevant logic appears to classify Jump and Interact as available while the active control context is:

* `ControlContextKind::Menu`
* `ControlContextKind::Dialogue`

These buttons should remain visible and be relabeled for their current purpose, such as:

* Select
* Confirm
* Advance

## Contradictory visibility rule

The button synchronization system also computes a condition resembling:

```rust
let gameplay = active_context
    .as_deref()
    .is_none_or(|seats| seats.primary().gameplay_owned());
```

It then shows an action only if something equivalent to this is true:

```rust
(gameplay || always_available)
    && touch_action_available(action, prompt)
```

During dialogue:

* The active context is present.
* The primary seat is owned by the dialogue context.
* `gameplay_owned()` is false.
* Jump and Interact are not shell-level `always_available` actions.
* The dialogue prompt says the actions are available.
* The separate gameplay gate nevertheless hides them.

The prompt says “show these as dialogue controls,” while the ownership gate says “hide everything that is not active gameplay.”

## Result

The right-hand button cluster disappears during dialogue despite being logically usable.

This removes the fallback interaction method that could otherwise compensate for unreliable direct row touching.

## Required correction

HUD visibility should be derived from the resolved control context and prompt, not from a broad “gameplay owns input” Boolean.

A context-aware policy should look approximately like this:

### Gameplay context

Show gameplay actions enabled by the current control scheme.

### Dialogue context

Show:

* One or more visible confirmation or advance buttons.
* Back or cancel if supported.
* Menu or pause if intentionally allowed.
* The navigation stick if directional dialogue navigation is supported.

Hide unrelated gameplay-only actions.

### Menu context

Show:

* Confirm.
* Back.
* Directional navigation.
* Menu or pause according to policy.

### Empty or shell context

Show only explicitly shell-level controls.

The same resolved action model should drive:

* Visibility.
* Labeling.
* Enabled state.
* Raw hit-testing.
* Input event generation.

---

# Detailed Failure 4: Invisible Interactive HUD Regions

## Suspected current split

Visual visibility appears to use both:

* Active-context gameplay ownership.
* Prompt action availability.

Raw touch processing appears to:

1. Hit-test fixed button rectangles.
2. Generate candidate actions.
3. Mask actions using prompt availability.

It may not apply the same gameplay-ownership visibility condition.

## Dialogue outcome

During dialogue:

* The visual synchronization system hides Jump and Interact.
* The prompt still marks them available as confirm actions.
* Raw touch processing can continue to accept their screen rectangles.

This creates invisible buttons.

## Why this is severe

The touch HUD has a very high global Z index and is intended to win picking over other UI.

If an invisible button overlaps a dialogue choice, the user can touch the visible choice and trigger the invisible button instead.

Even without direct overlap, invisible hotspots produce confusing behavior:

* Empty regions advance text.
* The bottom-right corner unexpectedly confirms choices.
* Dialogue choices under those areas seem dead.
* Visual debugging gives no indication of the interactive geometry.

## Required invariant

A hidden or disabled control must not participate in hit-testing.

The control state should have one authoritative representation, for example:

```rust
struct ResolvedTouchControl {
    action: Action,
    visible: bool,
    enabled: bool,
    label: String,
    bounds: Rect,
}
```

Rendering and hit-testing should consume the same resolved control set.

Do not independently reconstruct action availability in several systems.

At minimum, raw hit-testing must check the actual resolved visibility and enabled state before emitting an action.

---

# Detailed Failure 5: Stick Hidden During Dialogue

The stick visibility system also appears to use `gameplay_owned()` as its primary gate.

This hides the virtual stick whenever dialogue owns the context.

That conflicts with an input architecture in which joystick or directional input is used to navigate menus and dialogue choices.

## Desired behavior

If touch-stick navigation is supported in dialogue:

* Keep the stick visible.
* Route its vertical axis through the same repeat, dead-zone, and navigation logic as controller input.
* Suppress character movement while dialogue owns the input context.

The important distinction is:

* Hide or disable gameplay movement.
* Do not necessarily hide the physical directional input control.

The stick can change semantic role when the context changes.

If product design intentionally does not want a visible stick in dialogue, then provide explicit up/down controls or ensure direct row touching is fully reliable. The current state provides neither a reliable direct interaction nor the directional fallback.

---

# Detailed Failure 6: Touch HUD Can Overlap Dialogue Choices

## Current geometry

The touch HUD is assigned a global Z index much higher than the dialogue UI.

The right-side action cluster occupies a substantial bottom-right rectangle.

The dialogue panel uses most of the screen width and may expand vertically without a strict maximum height.

On a narrow portrait screen, lower dialogue choices can enter the same region as the action cluster.

## Result

Where geometry overlaps:

* The touch HUD wins interaction.
* The dialogue choice beneath it cannot receive the touch.
* If the HUD button is hidden but still hit-testable, the overlap is visually undetectable.
* Long dialogue bodies and long choice lists increase the likelihood.

## Required correction

Choose one consistent layout strategy.

### Strategy A: Reserve HUD-safe areas

The dialogue layout knows the touch-control exclusion zones and does not place interactive choices underneath them.

### Strategy B: Relayout controls for dialogue

When dialogue opens:

* Move confirm and back buttons beside or below the dialogue panel.
* Reduce their footprint.
* Hide gameplay-only controls.
* Reserve a stable dialogue-control strip.

### Strategy C: Let dialogue own touch UI

Suppress the gameplay HUD entirely and render dialogue-specific touch controls within the dialogue UI hierarchy.

This may be the cleanest long-term architecture because dialogue controls then share layout, Z ordering, and accessibility behavior with the dialogue itself.

Regardless of strategy, invisible controls must never occupy an active exclusion zone.

---

# Test Deficiency

A test reportedly expects menu confirmation buttons to remain visible, but it does not install the production `SeatInputContexts` resource.

Because the visibility code treats a missing context resource as equivalent to gameplay being allowed, the test passes in a state that does not represent the assembled application.

In production:

* `SeatInputContexts` exists.
* Dialogue owns the seat.
* `gameplay_owned()` is false.
* The buttons disappear.

## Required test repair

Unit and integration fixtures must include the same resources that production uses.

At minimum, add tests with:

```text
SeatInputContexts present
Primary seat owned by Dialogue context
Resolved prompt kind is Dialogue
Jump and/or Interact available as confirm actions
```

Expected result:

* The relevant action buttons are visible.
* They are labeled for dialogue.
* Their screen bounds are hit-testable.
* Hidden gameplay-only actions are not hit-testable.

Also test Menu, Gameplay, and Empty contexts explicitly rather than relying on missing-resource defaults.

---

# Proposed Implementation Plan

## Phase 1: Reproduce and lock down current failures

Add failing tests before changing behavior.

### Test A: Dialogue confirm buttons remain visible

Create an app fixture with:

* Touch controls enabled.
* `SeatInputContexts` installed.
* Primary seat owned by dialogue.
* Dialogue prompt active.
* Jump and Interact mapped as confirmation actions.

Assert that the appropriate button entities have inherited visibility.

### Test B: Hidden buttons do not hit-test

Hide a touch action using the resolved context.

Touch the center of its previous rectangle.

Assert that no action is emitted.

### Test C: Direct touch activates a response once

Create three dialogue choices.

Perform pointer-down and pointer-up within one choice without crossing the drag threshold.

Assert that the choice activates after one gesture.

### Test D: Drag does not activate

Touch a choice, move beyond the drag threshold, and release.

Assert that no dialogue choice activates.

### Test E: Long-list edge choice does not move before release

Create more choices than the visible limit.

Touch the first or last visible row.

Assert that:

* The same choice remains the active touch target until release.
* The intended choice activates.
* A neighboring row does not become the target due to recentering.

### Test F: Touch HUD and dialogue do not overlap interactively

Use a Pixel 5–class portrait viewport.

Assert that each visible dialogue choice’s interactive rectangle is not obstructed by a higher-priority touch-control rectangle.

Alternatively, assert that dialogue-owned touch controls are laid out in explicitly reserved regions.

---

## Phase 2: Unify touch-control resolution

Introduce or identify a single system that resolves the complete control state from:

* Active input context.
* Control prompt.
* Current touch-control layout profile.
* Platform settings.
* Action mappings.
* Dialogue/menu/gameplay state.

Its output should determine:

* Which controls exist.
* Which controls are visible.
* Which controls are enabled.
* Their labels.
* Their bounds.
* Their semantic actions.

Both rendering and raw hit-testing should read this output.

Remove or reduce duplicate policy logic such as:

* Prompt says available.
* Context visibility system says hidden.
* Raw hit-testing says active.
* Labeling system says dialogue confirm.

Those decisions must not be made independently.

---

## Phase 3: Repair dialogue-row touch semantics

Implement pointer ownership for dialogue choices.

A gesture should retain:

* Pointer or touch ID.
* Choice identity.
* Initial screen position.
* Current displacement.
* Whether drag cancellation occurred.
* Whether the gesture is still over the original target.

On release:

* Activate the original choice only if the gesture remained a tap.
* Do not activate based merely on whichever rebuilt entity now occupies that coordinate.

Avoid recentering or rebuilding the visible choice window while a choice gesture is active.

Use a stable choice key or index that survives UI reconstruction.

---

## Phase 4: Repair dialogue touch-control presentation

During dialogue:

* Keep at least one visible confirm or advance button.
* Keep Back visible if dialogue supports cancellation.
* Keep directional navigation visible if the stick is intended to navigate choices.
* Hide only actions that have no dialogue meaning.
* Relabel retained controls for their dialogue role.

Ensure the visible prompt and visible controls agree.

For example:

```text
Stick: choose response
A / Interact: select
B / Jump: advance or back
Menu: pause
```

The exact mapping should follow the game’s established input scheme rather than introducing an unrelated mobile-only convention.

---

## Phase 5: Establish mobile layout safety

Test at least these viewports:

* Pixel 5 portrait: approximately 393 by 851 CSS pixels.
* Pixel 5 landscape: approximately 851 by 393 CSS pixels.
* A smaller narrow phone.
* A tablet-like viewport.
* Desktop touch emulation if supported.

Test with:

* Short dialogue text.
* Long wrapped dialogue text.
* One choice.
* Three choices.
* Four or more choices.
* The longest existing merchant choice list.
* Large text or accessibility scaling if supported.
* Display cutouts and safe-area insets if the platform exposes them.

No visible dialogue response should be covered by an active HUD rectangle.

---

# Acceptance Criteria

The work is complete when all of the following hold.

## Direct touch

* A normal tap activates a non-destructive dialogue response in one gesture.
* A drag does not accidentally activate a response.
* The touched row does not move out from under the finger before release.
* Long lists do not cause neighboring responses to be selected by the release.

## Gamepad fallback

* The dialogue context displays visible confirmation controls.
* The right-side action-button cluster does not disappear merely because gameplay does not own the input context.
* Directional touch navigation remains available if it is part of the intended dialogue interaction.
* Labels reflect dialogue semantics.

## Hit-testing

* Hidden controls do not receive touches.
* Disabled controls do not emit actions.
* Rendering and hit-testing use the same resolved control state.
* There are no invisible hotspots.

## Layout

* Touch controls do not obstruct dialogue options.
* Dialogue options do not extend beneath active higher-Z controls without an intentional exclusion or ownership policy.
* Pixel 5 portrait and landscape both remain operable.

## Tests

* Tests include production-like `SeatInputContexts`.
* Tests exercise Dialogue, Menu, Gameplay, and Empty contexts.
* Tests cover long-list recentering.
* Tests cover tap-versus-drag behavior.
* Tests cover hidden-control hit-testing.
* At least one assembled-app or integration-level test verifies that the unit-level visibility result survives plugin composition.

---

# Important Non-Solutions

The following changes alone are insufficient.

## Only making the buttons visible

This leaves:

* Two-tap row activation.
* Moving choices.
* Possible HUD overlap.
* Potential mismatch between visible and hit-testable state.

## Only changing dialogue rows to single tap

This leaves:

* Disappearing confirmation buttons.
* Invisible HUD hotspots.
* Hidden stick navigation.
* Layout overlap.

## Only increasing row height

The existing rows appear reasonably large. Target size is not the primary problem.

## Only lowering the HUD Z index

This could make visible HUD buttons stop working when they overlap dialogue and would not resolve invisible hit-testing or contradictory context policy.

## Only deleting startup or fallback input behavior

Do not remove controller-style confirmation just to make direct touch appear fixed. Both interaction paths should work.

---

# Recommended Architectural Principle

The touch UI should be context-semantic, not gameplay-semantic.

A physical touch control is not inherently a “jump button” or a “movement stick.” Its meaning can change with the active input context:

* In gameplay, the stick moves the character.
* In dialogue, the stick navigates responses.
* In gameplay, Interact performs an action.
* In dialogue, the same button confirms a choice.
* In menus, it selects the highlighted item.

The resolved context should control both meaning and presentation.

The current bug appears to come from partially adopting that model in prompt and action routing while retaining an older rule that hides touch controls whenever gameplay does not own input.

Complete the context-semantic model instead of adding another dialogue-specific exception around the existing contradictions.

---

# Suggested Agent Deliverables

The implementing agent should return:

1. A concise root-cause summary confirming or correcting the static analysis above.
2. A list of every modified system and the invariant it now enforces.
3. Tests demonstrating each previously broken interaction.
4. Pixel 5 portrait and landscape screenshots or recorded interaction traces if the repository supports visual tests.
5. An explicit statement of whether the stick remains visible during dialogue and why.
6. An explicit statement of how HUD/dialogue overlap is prevented.
7. Confirmation that hidden controls are removed from hit-testing.
8. Confirmation that long dialogue lists do not recenter during an active touch gesture.
9. Full touched-crate test results.
10. Any remaining mobile dialogue limitations, without describing partial mitigation as complete resolution.
