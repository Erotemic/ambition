# Gameplay presentation profiles — full-bleed framing, fixed-aspect viewports, and control-safe layout

> **State:** TRIAGE. This design has not been admitted to `docs/planning/tracks.md`.
> It records a candidate engine subsystem and concrete product profiles so the
> idea can be reviewed without colliding with the live queue. Promotion should
> produce bounded track cards rather than copying this document into the queue.
>
> **Scope:** presentation only. Nothing here changes simulation, collision,
> actor motion, rollback state, or room geometry.

## Problem

Landscape phones are often much wider than the gameplay composition. Virtual
movement and action controls then cover mechanically important parts of the
world, including the controlled actor, nearby platforms, enemies, portals, and
hazards.

A game author should not have to solve this independently for every game or
write device-specific camera code. Ambition should provide a small set of
composable presentation primitives and tested platformer presets:

- render gameplay across the full display while softly keeping important
  subjects away from occupied control regions;
- render gameplay into a fixed-aspect viewport and reserve the surrounding
  screen for controls and HUD;
- use normal camera behavior where neither policy is wanted;
- select different policies by game and by stable presentation environment.

The three motivating profiles are:

- **Ambition flagship:** normal camera on desktop; full-bleed, occlusion-aware
  soft framing on touch-primary mobile.
- **Sanic:** full-bleed soft framing on every platform, with additional
  touch-control avoidance when virtual controls are present.
- **Super Mary O:** fixed 4:3 gameplay viewport on every platform, with the
  surrounding area available to HUD and touch controls.

These must be configurations of one subsystem, not game-name branches in the
engine.

## Design law

Separate four independent policy axes:

1. **Viewport:** where the gameplay camera renders on the physical display.
2. **Framing:** where important subjects should remain inside that viewport.
3. **Screen occupancy:** which regions are reserved or occluded by controls,
   HUD, safe-area insets, and game-specific presentation.
4. **Activation:** which profile is selected for the current game and stable
   presentation environment.

The world-space actor is never constrained by presentation. The camera and UI
adapt around the actor; simulation remains identical across desktop, mobile,
full-bleed, and fixed-aspect modes.

## Core vocabulary

The exact names may change during implementation, but the ownership split
should remain visible.

```rust
pub struct GameplayPresentationProfiles {
    pub default: GameplayPresentationProfile,
    pub touch_primary: Option<GameplayPresentationProfile>,
    pub handheld: Option<GameplayPresentationProfile>,
}

pub struct GameplayPresentationProfile {
    pub viewport: GameplayViewportPolicy,
    pub framing: SubjectFramingPolicy,
    pub surround: SurroundPolicy,
    pub hud: HudLayoutPolicy,
}

pub enum GameplayViewportPolicy {
    FullBleed,
    FixedAspect {
        aspect: AspectRatio,
        fit: FixedAspectFit,
    },
}

pub enum SubjectFramingPolicy {
    Normal,
    SoftSafeRegion(SoftFramingProfile),
    OcclusionAware(SoftFramingProfile),
}

pub enum SurroundPolicy {
    None,
    Solid,
    GameAuthored,
    DecorativeWorldExtension,
}
```

The resolved runtime product is shared by every consumer:

```rust
pub struct ResolvedGameplayPresentation {
    pub display_safe_rect: Rect,
    pub gameplay_rect: Rect,
    pub subject_safe_region: ScreenRegion,
    pub occlusion_regions: Vec<ScreenRegion>,
    pub hud_regions: Vec<NamedScreenRegion>,
    pub touch_regions: Vec<NamedScreenRegion>,
}
```

No camera, HUD, touch, pointer, or transition system should independently
recalculate margins.

## Primitive 1 — fixed-aspect gameplay viewport

A fixed-aspect profile fits a gameplay rectangle inside the device-safe display
rectangle and applies it through Bevy's physical camera viewport primitive.
The frontend/HUD camera remains full-screen.

On a wide display, a 4:3 gameplay rectangle produces side pillarboxes:

```text
┌──────────┬────────────────────────┬──────────┐
│ surround │                        │ surround │
│ / HUD    │      4:3 gameplay      │ / input  │
│ zone     │                        │ zone     │
└──────────┴────────────────────────┴──────────┘
```

On a 4:3 display, gameplay fills the safe display. On a display narrower than
4:3, preserving the aspect may create top and bottom letterboxing.

The surround is not necessarily black. A game may choose plain bars, themed
art, HUD panels, controls, or decorative non-mechanical world extension. The
mechanically authoritative view remains the gameplay rectangle.

The camera observer must receive the gameplay viewport dimensions rather than
the full window dimensions. Screen-to-world and world-to-screen conversion
must use the configured camera viewport.

## Primitive 2 — soft full-bleed framing

Full-bleed mode renders gameplay across the whole safe display. A
screen-relative subject-safe region tells the camera where the controlled actor
and other critical subjects should preferably remain.

```text
┌─────────────────────────────────────────────┐
│ world renders everywhere                   │
│                                             │
│ controls   ┌─────────────────────┐ controls │
│ overlap    │ subject-safe region │ overlap  │
│ zone       │       subject       │ zone     │
│            └─────────────────────┘          │
└─────────────────────────────────────────────┘
```

The camera does not rigidly center the subject. It behaves normally while the
subject remains in the soft region. When the subject's projected presentation
bounds cross an edge, the camera applies only the correction needed to return
them to the region.

A framing profile may also provide:

- velocity look-ahead;
- asymmetric horizontal anchors for high-speed movement;
- correction speed and damping;
- hysteresis so controls appearing or disappearing do not make the camera
  twitch;
- subject presentation padding for held items, attack anticipation, or large
  controlled bodies.

Camera bounds remain authoritative for presentation. If room clamping prevents
full correction, the fallback order is:

1. preserve required camera/room bounds;
2. reduce or reposition contextual controls when permitted;
3. fade controls near the subject without changing hit regions;
4. strengthen subject outline or silhouette;
5. permit overlap as the final fallback.

## Primitive 3 — screen occupancy

Touch controls and important HUD elements publish generic screen occupancy;
they do not own camera policy.

```rust
#[derive(Component)]
pub struct ScreenOccluder {
    pub purpose: ScreenOcclusionPurpose,
    pub padding_px: Vec2,
}
```

Potential producers include:

- virtual movement stick;
- action-button cluster;
- contextual action button;
- boss health presentation;
- dialogue or accessibility overlays;
- game-specific permanent HUD.

A first implementation may reduce these shapes to conservative left, right,
top, and bottom insets. The public contract should permit actual rectangles or
screen regions later so a circular thumbstick does not reserve an entire side
of the display.

The resolved subject-safe region is conceptually:

```text
authored framing region
∩ gameplay viewport
∩ device safe area
− active critical occlusions
```

The input/prompt subsystem may eventually determine which contextual touch
controls are visible, but the presentation subsystem only consumes their
published occupancy.

## Primitive 4 — stable profile selection

Profile selection belongs to the game/provider declaration. The runtime owns
platform and presentation-environment resolution.

Do not switch camera composition merely because the last input event changed
from touch to gamepad. Glyphs may change immediately; the gameplay viewport and
camera framing should normally remain stable for the session or until the
participant explicitly changes the presentation preference.

Useful activation inputs include:

- platform or form factor;
- touch-primary presentation mode;
- explicit participant preference;
- game-provided default.

## Built-in author presets

The engine should expose tested presets rather than requiring every game to
construct low-level policies.

### Adaptive platformer

```rust
profiles::adaptive_platformer()
```

- desktop/default: full bleed with normal framing;
- touch-primary: full bleed with occlusion-aware soft framing;
- intended initial consumer: Ambition flagship.

### High-speed full bleed

```rust
profiles::high_speed_full_bleed()
```

- all platforms: full bleed with velocity-aware soft framing;
- touch-primary: the safe region is additionally reduced by control
  occlusions;
- intended initial consumer: Sanic.

### Fixed four-by-three

```rust
profiles::fixed_four_by_three()
```

- all platforms: fixed 4:3 gameplay viewport;
- surround regions are available to HUD and controls;
- intended initial consumer: Super Mary O.

A custom game may use a builder, but common games should select one preset and
move on.

## Ownership and likely code placement

Bevy provides the rendering and coordinate primitives. Ambition owns the
platformer-oriented policy and author experience.

Do not create a new crate for the first implementation.

### `ambition_platformer_primitives`

Own content-free vocabulary and pure layout math:

- presentation policies and presets;
- normalized screen geometry;
- fixed-aspect fitting;
- safe-region and occlusion composition;
- a pure `resolve_gameplay_presentation` function.

The resolver must not depend on windows, rendering, touch input, game content,
or a particular provider.

### `ambition_host`

Own visible-host integration:

- read primary-window dimensions and platform safe-area inputs;
- select the active game-provided profile;
- collect generic screen occupancy;
- invoke the pure resolver;
- publish `ResolvedGameplayPresentation`;
- apply the physical gameplay camera viewport;
- publish actual gameplay viewport dimensions to camera observation.

The host must not know the names Ambition, Sanic, or Mary O.

### Existing camera/sim-view resolver

Consume a screen-framing fact such as:

```rust
pub struct CameraScreenFraming {
    pub gameplay_viewport_px: Vec2,
    pub subject_safe_region: NormalizedScreenRegion,
}
```

Use it to compute the desired presentation camera center. Mobile conditions do
not enter actor simulation or collision code.

### `ambition_render`

Own rendering of:

- surround presentation;
- gameplay-only versus full-display effects;
- presentation diagnostics.

It does not select policy.

### Touch and HUD crates

Publish generic occupancy and consume resolved named regions. They do not
compute camera margins or choose presentation profiles.

## Author-facing endpoint

A provider should be able to declare presentation with one tested preset:

```rust
impl GameProvider for AmbitionProvider {
    fn gameplay_presentation_profiles(&self) -> GameplayPresentationProfiles {
        profiles::adaptive_platformer()
    }
}
```

A custom profile remains possible:

```rust
GameplayPresentationProfiles::builder()
    .default(
        GameplayPresentationProfile::full_bleed()
            .with_soft_framing(SoftFramingProfile::platformer()),
    )
    .touch_primary(
        GameplayPresentationProfile::fixed_aspect(4, 3)
            .with_reserved_surround(),
    )
    .build()
```

The exact provider seam should follow the active provider/content architecture;
this triage document does not authorize a neighboring registration API.

## Interaction with existing camera policies

Viewport layout and world framing are different axes.

Existing camera aspect/zoom policy answers how world units map inside the
resolved gameplay rectangle. The presentation profile answers where that
rectangle is and where a subject should appear within it.

Do not overload one enum to answer both questions.

For fixed 4:3, the first safe behavior should preserve the complete authored
horizontal view rather than silently cropping threats that were composed for a
wider camera. A later game-authored mobile framing preset may deliberately show
additional vertical world or use different world scale, but that is a content
choice and needs encounter review.

## Coordinate and effects audit

Any implementation must inspect assumptions that the gameplay camera begins at
physical `(0, 0)` or occupies the full window.

At minimum audit:

- camera viewport publication;
- camera clamping and visible-world calculations;
- pointer/touch screen-to-world conversion;
- portal continuity and capture cameras;
- debug overlays;
- room-transition fades;
- damage flashes;
- pause dimmers;
- frontend and dialogue presentation.

Each full-screen effect should declare whether it covers:

- gameplay viewport only;
- entire physical display;
- gameplay plus surround but not controls;
- controls as well.

## Implementation slices

### GP1 — pure policies and layout resolver

- add the vocabulary and three presets;
- implement fixed-aspect fitting and safe-region composition as pure code;
- test common display aspects and safe-area insets;
- no runtime camera changes yet.

### GP2 — fixed-aspect runtime vertical slice

- select a provider-owned profile;
- publish one resolved layout;
- apply `Camera.viewport` to the main gameplay camera;
- keep frontend/HUD camera full-screen;
- make camera observation consume gameplay viewport dimensions;
- prove `fixed_four_by_three()` for Super Mary O.

### GP3 — soft subject framing

- project controlled-subject presentation bounds into screen space;
- implement soft-region correction with damping and hysteresis;
- preserve existing room/camera clamps;
- prove `high_speed_full_bleed()` for Sanic on desktop.

### GP4 — occupancy-aware mobile framing

- add generic screen-occluder publication;
- make touch controls publish their actual occupied regions;
- compose those regions into the safe region;
- prove `adaptive_platformer()` gives normal desktop behavior and mobile-only
  soft framing for Ambition flagship.

### GP5 — surrounds and author polish

- expose named surround/HUD/control regions;
- add Mary O surround presentation;
- define control fading and unavoidable-overlap behavior;
- add participant-facing layout preference only if product testing warrants
  it.

These slices may be reordered after source inspection, but each should remain
independently demonstrable and green.

## Acceptance oracles

Test at least 4:3, 16:9, 16:10, 19.5:9, and 20:9 displays, plus asymmetric
safe-area insets.

Required structural and behavioral evidence:

1. Fixed-aspect gameplay preserves the requested aspect inside the device-safe
   rectangle.
2. Full-bleed mode uses the full safe display.
3. The main camera observer reports gameplay viewport dimensions, not blindly
   the window dimensions.
4. Screen-to-world conversion is correct at all four gameplay-viewport
   corners.
5. Touch controls remain outside the Mary O gameplay viewport when using
   reserved surround regions.
6. Ambition desktop retains normal framing.
7. Ambition touch-primary mode keeps the controlled subject out of control
   regions when camera bounds permit.
8. Sanic uses soft high-speed framing on desktop and mobile.
9. Profile selection never changes simulation results.
10. Room transitions do not reset or briefly apply the wrong viewport.
11. Full-screen menus and startup presentation remain usable across the safe
    display.
12. No engine branch selects behavior by game name.

## Non-goals

- no new public standalone crate in the first pass;
- no custom render-target composition when Bevy camera viewports suffice;
- no invisible world walls protecting the actor from controls;
- no mobile-only simulation or collision behavior;
- no automatic redesign of authored encounters;
- no arbitrary UI layout framework;
- no coupling to one touch-control implementation;
- no dynamic camera-mode flicker based on the most recent device event.

## Questions before promotion to `tracks.md`

1. Which provider/prepared-content seam should own the profile without
   creating a neighboring registration authority?
2. Does the current camera resolver already expose a stable normalized anchor
   that soft framing can extend, or is one additional screen-framing input
   required?
3. Which existing screen effects are gameplay-only versus full-display?
4. What safe-area information is available on each supported platform, and
   what is the fallback when the host cannot provide it?
5. Should fixed 4:3 preserve authored horizontal extent by default, or should
   Mary O declare an explicit world-space 4:3 framing preset immediately?
6. Which subject bounds should the first soft-framing slice protect: body
   bounds, presentation bounds, or a small actor-authored visibility envelope?
7. Is `ScreenOccluder` sufficient, or should occupancy be published as a
   host-owned message/read model to avoid presentation-entity coupling?
8. What exact tests can boot Ambition, Sanic, and Mary O cheaply enough to pin
   their profiles in the normal local gate?
9. After all three profiles work, does extraction into a dedicated
   `ambition_gameplay_presentation` crate remove a real dependency edge? If
   not, keep the subsystem in existing owners.

## Promotion rule

This document remains in triage until source inspection resolves the questions
above and an execution owner selects bounded GP cards. Promotion should add
only those open cards to `tracks.md`; this document remains the design of
record and should not be duplicated there.
