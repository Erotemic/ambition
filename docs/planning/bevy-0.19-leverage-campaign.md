# Bevy 0.19.1 leverage campaign

## Status

Ambition has completed the Bevy 0.19.1 migration.

This campaign is a short follow-up intended to spend roughly one or two focused development days converting newly available Bevy 0.19.1 capabilities into concrete improvements.

The campaign should favor three kinds of work:

1. replace Ambition-specific mechanisms with stronger upstream mechanisms;
2. improve runtime observability without adding a second interpretation of gameplay;
3. remove work from ordinary frames when a feature is disabled.

This is not another engine migration and should not become a broad rewrite.

---

# Goals

By the end of the campaign:

* FPS is visible everywhere in the visible application whenever the global FPS setting is enabled;
* the FPS setting defaults to enabled and remains independent of the visual-quality profile;
* F1 exposes useful numeric runtime diagnostics in addition to world-space debug geometry;
* F1 world labels use Bevy's text gizmos instead of spawning `Text2d` entities;
* existing runtime measurements begin feeding Bevy's `DiagnosticsStore` rather than being useful only as log output;
* menu viewport-height text sizing uses `FontSize::Vh` directly;
* entity diagnostics correctly account for Bevy 0.19 resource entities;
* Ambition has an explicit render-device-loss policy;
* Bevy's `SettingsPlugin` has been evaluated against Ambition's actual persistence requirements, with migration performed only if the fit is clearly better.

The campaign should leave behind fewer custom mechanisms than it starts with.

---

# A. Make diagnostics host-global

This is the highest-priority work.

The current diagnostics architecture is partly session-scoped even though several of the facts being displayed are process/display facts.

That distinction should become explicit:

```text
host diagnostics
    FPS
    frame time
    process/system information
    render information
    asset residency
    camera/render-target census
    presentation workload
        |
        +---- available from launcher/title/loading/gameplay/pause

session diagnostics
    body state
    hitboxes
    contacts
    room geometry
    rollback/session state
    portal state
        |
        +---- available only when the corresponding authority exists
```

The host-level diagnostic UI must not disappear because no gameplay session currently exists.

---

# A1. Replace the custom FPS UI with Bevy's FPS overlay

Current implementation:

`game/ambition_app/src/dev/fps_overlay.rs`

This currently:

* installs `FrameTimeDiagnosticsPlugin`;
* spawns and owns its own UI `Text`;
* calculates its own recent min/mean/max;
* owns `FpsOverlayState`;
* mirrors `UserSettings::video.show_fps` into that resource;
* manually positions the UI;
* manually controls visibility.

Bevy 0.19.1 already has:

`bevy::dev_tools::fps_overlay::FpsOverlayPlugin`

and:

`FpsOverlayConfig`.

Use the upstream implementation unless testing identifies a product requirement it cannot satisfy.

## Required semantics

The authoritative setting remains:

```text
UserSettings::video.show_fps
```

It is a global presentation preference.

It must:

* default to `true`;
* remain persisted;
* apply on desktop, web and Android;
* apply independently of `VisualQualityProfile`;
* work with no gameplay session;
* remain in effect while switching between Ambition, Smash and other shell-hosted games.

Do not make `High` or `Ultra` quality imply FPS visibility. `show_fps` is the explicit authority.

Sync:

```text
UserSettings::video.show_fps
        ↓
FpsOverlayConfig.enabled
```

directly.

Delete `FpsOverlayState` if it becomes only a mirror of that boolean.

## Visibility contract

When enabled, FPS must be visible in at least:

* initial loading;
* launcher/title;
* game selection;
* Ambition gameplay;
* Smash gameplay;
* normal pause menu;
* kaleidoscope pause menu;
* dialogue/cutscene UI;
* room-transition/loading presentation;
* any state where no session world exists.

The current host already owns a persistent `FrontHudCamera` marked `IsDefaultUiCamera`.

Treat that camera as the host-global UI presentation seam.

Do not bind FPS lifetime to gameplay entities, a room, `GameMode`, or `session_world_exists`.

## Layering

The current custom FPS node has no explicit `GlobalZIndex`.

Bevy's FPS overlay deliberately renders at a very high `GlobalZIndex`.

Use that upstream layering rather than rebuilding another ordering convention.

Add an acceptance test involving an opaque fullscreen menu/loading UI so the test proves the FPS surface stays above it, rather than merely proving that an FPS entity exists.

## Frame-time graph

The upstream FPS overlay can also display a frame-time graph.

Recommended policy:

* ordinary `show_fps=true`: compact FPS/frame-time text;
* F1 debug mode: enable the graph.

That keeps the default display small while making frame pacing immediately inspectable during tuning.

If the graph is too intrusive in practice, keep it F1-only.

---

# A2. Make F1 a host-level diagnostics toggle

Current F1 handling eventually reaches:

`game/ambition_app/src/app/dev_runtime.rs::handle_debug_hotkeys`

but its installation in `app/plugins.rs` currently places it in a chain gated by:

```text
session_world_exists
```

That means the semantic F1 action cannot update the main debug state from launcher/title/loading states.

Separate the actions.

Host-global debug actions such as:

```text
ToggleDebugOverlay
ToggleInspector
ToggleWorldInspector
```

should be consumed by host presentation regardless of session existence.

Operations that genuinely require a session, such as LDtk reload or room-specific operations, remain session-gated.

The desired shape is:

```text
DeveloperAction
        |
        +---- host debug state             always
        |
        +---- session-specific operations  only when session exists
```

F1 should therefore work before entering a game and continue to refer to the same debug mode after entering or leaving one.

---

# A3. Use `DiagnosticsOverlayPlugin` for the F1 numeric panel

Add Bevy's new:

```text
DiagnosticsOverlayPlugin
```

as the screen-space numeric diagnostics surface.

When F1 is enabled, display an Ambition diagnostics window.

The initial useful set should include existing Bevy facts rather than building equivalent measurements:

### Frame

* FPS;
* frame time;
* frame count where useful.

### Host/process

Evaluate:

* `SystemInformationDiagnosticsPlugin`;
* CPU usage;
* memory usage.

These are useful for distinguishing:

```text
the game got slower
```

from:

```text
the host is under CPU/memory pressure
```

without opening an external profiler.

### ECS

Do not blindly display Bevy's raw entity count as "game entities".

Bevy 0.19 resources occupy singleton entities.

Publish two clearly named values if both are useful:

```text
ecs/scene_entities
ecs/resource_entities
```

or:

```text
ecs/entities_total
ecs/entities_non_resource
```

The semantic names must make the distinction clear.

---

# A4. Turn Ambition's existing censuses into diagnostics

Ambition already measures considerably more than the FPS overlay exposes.

Relevant code includes:

```text
crates/ambition_render/src/runtime_census.rs
ambition_dev_tools runtime-census machinery
game/ambition_app/src/dev/rollback_observatory.rs
```

The render census already knows about:

* cameras;
* camera roles;
* repeated world rendering;
* offscreen targets;
* portal capture rigs;
* active capture budget;
* resident images;
* cumulative decoded image work;
* retained HUD image hits/loads;
* Bevy render-pass CPU timings;
* Bevy render-pass GPU timings when supported;
* pipeline statistics when supported.

Today much of this exists primarily as periodic text output.

Start publishing the high-value numeric facts into Bevy diagnostics.

A useful first vocabulary would be approximately:

```text
ambition/ecs/entities_non_resource

ambition/render/cameras
ambition/render/world_draws
ambition/render/offscreen_targets

ambition/assets/images_resident
ambition/assets/decoded_megapixels
ambition/assets/decoded_bytes

ambition/portal/capture_rigs
ambition/portal/active_captures

ambition/rollback/current_frame
ambition/rollback/confirmed_frame
ambition/rollback/resimulated_frames
```

Only publish a metric where the owning subsystem already knows the fact.

Do not move calculation into the overlay.

The intended architecture is:

```text
runtime authority / measurement
            ↓
       DiagnosticPath
            ↓
      DiagnosticsStore
        ↙          ↘
 F1 overlay         logging/reporting
```

There should be one measurement and multiple consumers.

This also creates a path to eventually simplify the periodic `[census]` printer around common diagnostic values.

Do not try to convert every census field during this campaign. Select the values that answer common performance/debugging questions.

---

# A5. Surface Bevy render-pass diagnostics

`PresentationCensusPlugin` already conditionally installs:

```text
bevy::render::diagnostic::RenderDiagnosticsPlugin
```

and already knows how to enumerate:

```text
render/.../elapsed_cpu
render/.../elapsed_gpu
```

when those measurements exist.

Investigate showing a small useful subset in the F1 diagnostic panel.

Do not create a giant dynamically changing wall of every render diagnostic.

Start with facts that answer:

```text
Is the frame CPU-bound or GPU-bound?
Which major render pass is expensive?
Are portal/offscreen cameras causing repeated rendering?
```

GPU measurement availability must remain explicit.

No GPU timestamp support means "measurement unavailable", not zero GPU cost.

---

# B. Replace F1 label entities with text gizmos

Bevy 0.19.1 added world-space debug text through `Gizmos::text` / `text_2d` using a built-in stroke font.

This fits Ambition's F1 diagnostics directly.

Current implementation:

```text
game/ambition_app/src/dev/debug_overlay.rs
game/ambition_app/src/dev/debug_overlay/prims.rs
```

currently:

1. queues `DebugLabel`s into `DebugOverlayLabels`;
2. despawns the previous frame's `Text2d` labels;
3. spawns new `Text2d` entities;
4. relies on normal Bevy text/font presentation.

Replace this with text gizmos.

## Benefits

This should remove:

* per-frame debug-label entity spawning;
* per-frame debug-label despawning;
* `DebugOverlayLabel`;
* product font resolution from F1 world labels;
* font atlas/glyph asset dependencies from these labels;
* label lifetime bookkeeping.

The text-gizmo font is deliberately ASCII-only.

That is appropriate here.

Keep user-facing text on the real typography stack.

## Implementation options

Prefer direct immediate-mode rendering if the call structure stays readable:

```text
draw box
draw label
```

If changing every debug helper to accept text-gizmo state creates excessive churn, keeping the one-frame `DebugOverlayLabels` scratch buffer is acceptable:

```text
geometry systems
    ↓
DebugOverlayLabels scratch
    ↓
one system emits gizmo text
```

The important removal is the retained/spawned `Text2d` entity layer.

## Legibility

Text gizmo size is expressed in screen-space pixels.

That is likely better for debug labels than the current world-space text whose readability changes with camera zoom.

Test:

* normal room;
* boss zoom;
* Smash camera;
* split/local view if supported;
* portal-debug geometry.

Labels should remain associated with their boxes and remain readable.

---

# C. Delete `MenuTextHeightFraction`  ✔ DONE 2026-08-31

Landed. `MenuNode::Text`/`DynamicText`'s `size` is spawned as `FontSize::Vh`
directly, and the unit is now documented on the FIELD — the place the original
five-pixel bug came from having no documented unit at all. Deleted:
`MenuTextHeightFraction`, `MENU_REFERENCE_VIEWPORT_HEIGHT`,
`resolve_menu_text_size`, `install_bevy_ui_menu_text_scaling`, its
`MenuTextScalingInstalled` marker, the `before(UiSystems::Content)` constraint
that existed only for it, and the three unit tests of the conversion arithmetic.

⭐ THE ENGINE RESOLVES AGAINST THE UI RENDER TARGET, NOT THE PRIMARY WINDOW, and
for Ambition those are the same thing: menu UI resolves to the order-9
`FrontHudCamera` (`IsDefaultUiCamera`), which carries no viewport override.
Split-screen viewports are set on the GAMEPLAY cameras only. Verified by reading
`host_presentation_scaffold` and `gameplay_presentation`.

⛔ ONE BEHAVIOUR DID CHANGE, and it only ever affected tests: with no window the
old system fell back to a 1080 reference height, so a windowless composition drew
legible text. `Vh` against a 0x0 target is 0px, which Bevy warns about and skips.
`VisibleRenderMode::NoWindow` composes no window AND no render app, so nothing
was being presented there anyway — measured, every UI target in that harness
reads 0x0. The launcher legibility test now states its own reference viewport
and resolves through `FontSize::eval`, which is the call Bevy's text pipeline
makes.

The acceptance test (`menu_text_is_sized_as_a_percentage_of_the_live_viewport`)
compares TWO viewport heights and runs no system of Ambition's own. Poisoning the
spawn back to `Px` fails it; poisoning it to the wrong axis (`Vw`) fails it on
the value, because the role filter asks for "not `Px`" rather than "is `Vh`".

---

## Original plan

This is the cleanest non-diagnostic 0.19 cleanup.

Current code owns a unit that Bevy now owns:

```text
MenuTextHeightFraction
```

and a system converts that viewport-height fraction into `TextFont` pixel values.

Bevy 0.19.1 provides:

```text
FontSize::Vh
```

directly.

Replace menu-authored viewport-height text sizes with `FontSize::Vh`.

Delete:

* `MenuTextHeightFraction`;
* the per-frame window-height conversion system;
* scheduling required only for that conversion;
* resize handling that exists only to recalculate font pixels;
* tests whose sole purpose is the custom conversion.

Keep tests that verify actual authored sizing semantics.

Acceptance should compare at multiple window sizes and prove the menu scales as before.

---

# D. Harden resources-as-components semantics  ✔ DONE 2026-08-31 (the census; the campaign's broader sweep stands)

The `runtime_census` defect is fixed. The four populations moved behind one
`EcsPopulation` system param so the `Without<IsResource>` cannot be forgotten by
the next reader who wants an entity count, and `resources=` is reported BESIDE
`live=` rather than folded into it. The regression the section asks for exists
and has both arms — adding a resource must move `resources` and not `live`, and
spawning an entity must do the opposite — and is poison-verified.

The sweep found no second production defect: the four other `Query<()>` and the
one `Query<Entity>` in the tree are all `get(known_entity)` existence checks,
where a resource entity can never be the argument. The one `entities().len()`
use in a test is a before/after delta, which a constant offset cancels.

⛔ NO GREP PROHIBITION WAS ADDED, per this section's own last line: the five
legitimate uses would each need a waiver, and the discriminator that actually
matters is not the query TYPE but whether it is ITERATED. `EcsPopulation` is the
semantic pin instead.

---

## Original plan

Bevy 0.19 makes resources components stored on singleton resource entities.

This has already produced one port regression and there is another remaining measurement issue:

`runtime_census` uses a broad:

```text
Query<()>
```

to mean "live game entities".

That now includes resource entities.

Fix it using the Bevy 0.19 resource marker, e.g. the appropriate:

```text
Without<IsResource>
```

filter.

Then sweep broad population queries whose meaning is:

```text
all gameplay / scene entities
```

Pay particular attention to:

```text
Query<()>
Query<Entity>
EntityRef / EntityMut iteration
world.iter_entities*
raw world entity counts
```

Do not change targeted queries that happen to include resources intentionally.

Add at least one regression proving that inserting another Resource does not change the reported scene-entity count.

A large grep-based prohibition is probably too coarse. Pin semantic consumers instead.

---

# E. Add explicit render recovery

Bevy 0.19.1 exposes typed render failures through:

```text
RenderErrorHandler
RenderErrorPolicy
ErrorType
```

Ambition should install an explicit policy rather than inherit whatever Bevy's default happens to be.

This belongs in the visible host/platform composition.

Start conservatively.

Recommended initial policy:

```text
DeviceLost
    → log structured diagnostic
    → attempt renderer recovery

OutOfMemory
    → log prominently
    → stop rendering or exit through a deliberate product policy

Validation
    → preserve Bevy's sensible handling unless Ambition has stronger evidence

Internal
    → fail loudly
```

Do not endlessly recover from repeated device-loss/OOM loops.

If practical, keep a small host-side recovery counter so repeated failures within one run escalate rather than producing repeated flashing.

This is presentation/platform state and must not enter rollback simulation.

A pure policy test should pin the response chosen for each Bevy error category.

An actual forced device-loss test is optional if the backend makes it impractical.

---

# F. Evaluate `SettingsPlugin`, but do not force a migration

Bevy 0.19.1 now has first-party persistent app settings.

Ambition already has a mature settings implementation.

The correct campaign task is therefore a compatibility spike, not an assumption that upstream must replace it.

Compare Bevy `SettingsPlugin` with the current:

```text
crates/ambition_persistence/src/settings/
```

against these requirements:

* existing RON compatibility;
* schema evolution/defaulting;
* `clamp_all()` normalization;
* test-specific `PersistenceRoot`;
* atomic/crash-resistant save behavior;
* deferred/debounced writes;
* synchronous save where shutdown requires it;
* desktop paths;
* Android paths;
* web persistence;
* preservation of existing user settings.

Web support is particularly interesting because Ambition's current persistence code explicitly leaves browser persistence for later, while Bevy's settings framework was designed to abstract persistent application settings across platforms.

## Decision gate

Only migrate if the Bevy mechanism can preserve Ambition's important semantics while deleting a meaningful amount of code.

If doing so requires replacing a clean tested persistence abstraction with a pile of adapters, stop after the spike and record why.

A successful spike can become its own follow-up campaign.

Do not let settings migration block the diagnostics work.

---

# G. Small post-processing audit

Bevy 0.19 now provides built-in vignette and lens-distortion effects.

Ambition currently has one combined fullscreen shader covering:

* CRT;
* film grain;
* vignette;
* robot death;
* underwater;
* deep dream.

Do not split that single combined pass into several Bevy passes just to use newer APIs.

That could increase GPU work.

Instead answer one question:

> Can any Ambition custom code disappear without adding another fullscreen pass?

If the answer is no, leave the combined effect alone.

One improvement is still worth implementing if it is straightforward:

when every screen effect is disabled, do not keep a no-op fullscreen post-process active.

Prefer component presence to mean that the camera actually needs the effect.

Measure or inspect the resulting render path before claiming a performance improvement.

---

# H. Optional 0.19.1 extras if the core campaign finishes early

These are candidates, not required work.

## H1. Render debug overlay

Bevy dev tools can visualize renderer data such as depth, normals, motion vectors and deferred buffers.

This may be useful for:

* kaleidoscope/3D UI rendering;
* custom material investigation;
* capture/render bugs.

It is less valuable for normal 2D gameplay than Ambition's semantic combat/world overlays.

If added, put it behind a separate developer action rather than overloading normal F1 immediately.

## H2. `Rem` UI scaling

Bevy's:

```text
FontSize::Rem
RemSize
```

provide a clean global text-scale authority.

This becomes worth implementing when Ambition adds a user-facing UI/accessibility scale setting or needs a dedicated Steam Deck readability profile.

Do not mass-convert existing UI sizes during this campaign without that product requirement.

## H3. Observer run conditions

Observers can now have `run_if` conditions.

When touching an observer that currently begins by manually checking:

```text
if debug disabled { return; }
```

or another clean resource condition, consider moving that gate onto the observer.

Do not sweep the entire repository merely to use the API.

## H4. Contiguous query access

Bevy 0.19 can expose dense table component data through contiguous query access for better vectorization.

Ambition should not launch a speculative CPU optimization sweep.

Only try this on a measured hot loop where:

* the query is dense;
* profiling identifies it as meaningful;
* semantics around change detection are understood.

The standing rule remains measurement first.

## H5. Asset saving / transform gizmos

These may eventually help authoring tools.

They are not part of this campaign unless an existing Ambition tool already needs the exact capability.

Do not grow an in-engine level editor simply because Bevy gained editor primitives.

---

# Work order

## Checkpoint 1 — global observability

Complete together:

* upstream FPS overlay;
* global `show_fps` behavior;
* global F1 hotkey handling;
* initial `DiagnosticsOverlay`;
* FPS/frame-time/system diagnostics.

At this checkpoint a developer should be able to start the game and see useful performance information before entering a gameplay session.

## Checkpoint 2 — Ambition diagnostics

Complete:

* selected runtime-census values published as diagnostics;
* resource-aware entity count;
* render diagnostics exposed where supported;
* optional rollback numeric diagnostics;
* F1 panel consuming these values.

At this checkpoint the screen overlay and command-line profiling tools should increasingly consume the same measurements.

## Checkpoint 3 — F1 visual cleanup

Complete:

* debug labels converted to text gizmos;
* `Text2d` label lifecycle removed;
* font dependency removed from world-space F1 labels.

## Checkpoint 4 — small Bevy-native deletions

Complete:

* ✔ `MenuTextHeightFraction` → `FontSize::Vh` (2026-08-31);
* ▢ no-op screen-effects pass avoided if practical;
* ▢ render recovery installed.

## Checkpoint 5 — persistence decision

Perform the `SettingsPlugin` spike.

Either:

* adopt it because the migration is clearly smaller/stronger;

or:

* document the mismatched requirements and retain Ambition persistence.

Both are valid outcomes.

---

# Validation matrix

The diagnostics work is incomplete until it is tested through actual application routes.

At minimum exercise:

| State             | FPS with `show_fps=true` | F1 numeric panel | F1 world gizmos                                                  |
| ----------------- | ------------------------ | ---------------- | ---------------------------------------------------------------- |
| Startup/loading   | visible                  | available        | no session facts                                                 |
| Launcher/title    | visible                  | available        | no session facts                                                 |
| Ambition gameplay | visible                  | available        | available                                                        |
| Smash gameplay    | visible                  | available        | available where supported                                        |
| Grid pause menu   | visible above menu       | available        | underlying world may remain visible according to existing policy |
| Kaleidoscope menu | visible above cube UI    | available        | existing policy                                                  |
| Dialogue/cutscene | visible                  | available        | existing policy                                                  |
| Room transition   | visible                  | available        | only facts whose authority exists                                |
| No active session | visible                  | available        | absent                                                           |

Also test:

```text
show_fps default = true
show_fps false -> hidden everywhere
F6 toggles show_fps
F6 persistence survives restart
F1 works without a session
F1 does not mutate gameplay simulation
additional Resource insertion does not inflate scene entity census
text-gizmo labels require no font asset
```

Where web and Android toolchains are available, compile/test those paths too.

---

# Architectural constraints

Do not:

* make diagnostics another source of gameplay truth;
* rederive hit geometry, contacts, room state or rollback facts in UI code;
* make FPS visibility depend on session lifetime;
* make FPS visibility depend on visual quality;
* use text gizmos for product/UI/dialogue text;
* expose unsupported GPU measurements as zero;
* put diagnostic/presentation state into rollback snapshots;
* add heavyweight per-frame measurement merely because the F1 panel can display it;
* replace the combined screen-effects shader with multiple passes without evidence;
* use Bevy delayed commands for rollback-sensitive gameplay timing;
* turn the settings spike into a forced persistence rewrite.

The intended direction is:

```text
real runtime facts / measurements
              ↓
       shared diagnostic paths
              ↓
         DiagnosticsStore
          ↙          ↘
    F1 display      logs/tools
```

and separately:

```text
semantic debug geometry
              ↓
          Bevy Gizmos
       shapes + ASCII labels
```

The campaign succeeds when diagnostics become easier to see and there is less Ambition-specific presentation machinery maintaining them.

---

# Expected cleanup targets

Successful completion should make some or all of these removable:

```text
game/ambition_app/src/dev/fps_overlay.rs
    custom FpsOverlayState
    custom FPS text entity
    custom text refresh formatting
    custom visibility system
    custom positioning system

game/ambition_app/src/dev/debug_overlay.rs
    DebugOverlayLabel entities
    per-frame Text2d label despawn/spawn

game/ambition_app/src/dev/debug_overlay/prims.rs
    font-dependent debug-label presentation

ambition_menu                                   ✔ REMOVED 2026-08-31
    MenuTextHeightFraction
    viewport-height -> pixel conversion system
```

The exact module files do not have to disappear if thin integration code remains.

What should disappear are the custom mechanisms Bevy 0.19.1 now supplies directly.
