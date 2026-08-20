# TwinTrack — Relativity Plaza

> **Status (2026-08-12): SR-8 open-follow composition candidate implemented.**
> TwinTrack keeps the default-on 3D spacetime minimap, recenters the clock race
> around the laboratory, removes collision walls, and authors an effectively
> unbounded unclamped follow-camera region so free flight is never pinned to the
> old room rectangle. Local compile and classroom-feel validation remain.

TwinTrack is Ambition's executable acceptance game for flat-spacetime
relativity. Its first duty remains technical: prove that clocks, worldlines,
light signals, local measurements, observer views, and rollback compose through
normal engine seams. Its second duty is now equally explicit: show recognizable
gameplay ideas that could grow into a visually strange relativity game.

The next major gameplay direction is **dual-observer split screen**: two controlled
participants share one authoritative Minkowski simulation while each viewport is
resolved from that participant's own reference frame. That work is deliberately
after SR-8; the open-follow camera and centered plaza land first so the single-
observer experience has a clean spatial baseline.

> **Landed 2026-08-20 — SR-9 ordering exhibit, PERMANENTLY SPLIT.** A
> `SplitObservers` view mode draws two demo-owned panes: the laboratory twin at
> rest and the controlled traveler. Two beacons at rest in the laboratory,
> symmetric about the lab twin, flash together in laboratory coordinate time;
> each pane reports **which flash's light reached its observer first** and
> **which flash happened first in its observer's own frame** (an exact
> `lorentz_boost_event`), plus that observer's own length contraction of the
> beacon axis. The lab pane answers `SIMULTANEOUS` by construction; the traveler
> pane answers with an order whose sign is the sign of its velocity. Headless
> tests in `twintrack_it` assert the two panes disagree and that reversing the
> traveler reverses its answer.
>
> ⚠ **this is deliberately not the acceptance target below.** There is still one
> controlled body and one `LocalView`; the panes are demo-owned composited
> cameras in the shape of the 2+1D minimap, not engine per-view rectangles. What
> the acceptance list needs from the engine is a **split layout policy** (today
> `publish_camera_viewport` writes the display rect to every view by
> construction), a **second `LocalView` plus explicit `PresentsView` binding on
> both cameras** (the shared rule refuses to bind at all once several views
> exist, so a naive second view unbinds the gameplay camera), a **per-view view
> SUBJECT** (framing resolves one subject above the per-view loop), and
> **per-view parallax**.
>
> ⚠ **`RelativisticOpticalView2d` is single-observer.** `publish_optical_view`
> does `observers.single()`, so a second `RelativisticObserver2d` would blank the
> optical view entirely rather than produce a second one. The ordering exhibit
> is computed demo-side from `WorldlineHistoryView2d`-shaped facts and the public
> SR kernel instead; a genuine per-view aberration/Doppler presentation is
> blocked on that resource becoming view-indexed.
>
> **Adaptive vs permanently split, for THIS demo:** permanently split. §11's
> adaptive-with-hysteresis ruling is Ambition's product layout policy; a view
> that merges when the two observers are close would hide the phenomenon exactly
> when the comparison is most instructive, and the lab observer never moves so
> "close" is a statement about the traveler alone. The world-space entities are
> still duplicated per pane, which is the part of §11 that is architectural.

> **Landed 2026-08-20 — SR-10 LIGHT-SPEED PULSE, in the split panes.** The lab
> twin fires a three-ray flare from the beacon midpoint every
> `PULSE_PERIOD_SECONDS`: one ray down the axis toward Alpha, one toward Omega,
> one square across it. Each pane prints the speed **its own observer measures**
> for the pulse, the direction that observer measures it travelling in, and the
> Doppler factor and colour it measures — so a viewer compares two numbers
> rather than interpreting an animation.
>
> ⛔ **it is not a fast projectile, and that distinction is the exhibit.** A ray
> has no stored position and nothing integrates it: its laboratory position is
> `emission_position + c * (t - emission_time) * direction`, derived from the
> **emission event** and the invariant speed. Every observer-frame number is an
> exact `lorentz_boost_event` of that null displacement, which is why a traveler
> flying at 0.9c alongside the toward-Omega ray reads `1.000 c` for it instead
> of the `c - v` a velocity that something integrated would have produced. The
> unit tests assert exactly that, with a falsifier that fails if the measured
> speed ever lands near `c - v`.
>
> What the two panes disagree about at 0.9c: the crosswise ray travels at 90° in
> the lab pane and about 154° in the traveler's (aberration, `sin θ' = 1/γ`);
> the toward-Alpha ray is blueshifted ×4.36 and the toward-Omega ray redshifted
> ×0.23 for the traveler and ×1.00 for the lab (and the two axial factors are
> exact reciprocals, ×4.36 · ×0.23 = 1). The pane also plots each front on the
> observer's own map at one instant of **that observer's** time, solved on the
> ray's null worldline rather than length-contracted, and reports the one light
> cone arrival — the toward-Omega ray reaching the Omega beacon — which both
> observers agree happened and time differently on their own clocks.
>
> ⚠ **derived, not canonical, and no schema moved.** `TwinTrackLightPulseView`
> is recomputed every frame from `SpacetimeCoordinateTime2d` and canonical
> `BodyKinematics`, in the same shape as `TwinTrackDualObserverView`: no
> entities, no accumulator, nothing for rollback to rewind.
>
> ▢ **not player-fired.** A pulse the participant triggers needs its emission
> event to persist — an emission coordinate time, an emission position, and a
> direction on `TwinTrackExperiment`, whose encoding is a snapshot schema. The
> timer flare gives the same physics with none of that; the fired version is
> owed whenever the schema is free to move.


## Dual-observer / split-screen acceptance

TwinTrack is the strongest acceptance customer for the engine's multi-view
architecture because two local participants can observe **the same authoritative
Minkowski simulation through different reference frames**.

The target is not two copies of the plaza. It is:

```text
one simulation + two participants/control assignments + two LocalView-like
observer contexts + two independently derived presentations
```

Each view may choose laboratory or observer-local presentation independently. A
view's aberration, Doppler, light-delay source selection, simultaneity plane, HUD
and spacetime overlay derive from that view's observer/reference frame. None of
those presentation choices changes authoritative world state.

This should consume
[`../engine/multiplayer-and-multiview.md`](../engine/multiplayer-and-multiview.md),
not create a TwinTrack-only camera manager. The first proof can be fixed 50/50
split; adaptive grouping is primarily an Ambition product requirement and should
come from the same engine view-index model.

Acceptance:

- two controlled bodies can move simultaneously through participant-aware input;
- each half of the screen follows its assigned observer;
- each observer-local visual transformation is different when their states differ;
- one view may remain laboratory-frame while the other is observer-local;
- the default-on 3D spacetime exhibit is view-owned/presented without duplicating
  simulation state; and
- rollback/headless state remains independent of how many local views render it.

## Player-facing premise

Everyone starts with a clock. The controlled participant flies around a plaza
and asks moving characters what their own clocks read. Questions and replies
travel as light, so the answer is already old when it arrives. The participant
then Doppler-shifts a fixed onboard note into DJ Blue Shift's preferred band,
plays light tag with Photon Fox by leading the visible image, and returns to the
laboratory twin to compare clocks.

Player-facing language uses:

- **your clock** for proper time;
- **laboratory time** for the authored Minkowski coordinate chart;
- **what reaches you now** or **light-delayed image** for the source event whose
  light arrives at the observer now;
- **when this message left** for the signal emission event;
- **speed as a percentage of light speed** instead of requiring beta/gamma.

The standard technical term *retarded event* remains valid in research code and
advanced notes, but is not used in game text because it is a needless classroom
distraction.

## Engine capabilities exercised

- permanent gravity-free 2D movement through the shared flight limb;
- spatial proper-velocity control with an authored invariant-speed limit;
- radial rather than per-axis terminal-speed enforcement;
- Minkowski proper-time accumulation for every marked character;
- analytic null-signal propagation at the invariant speed;
- stable emitter identity, opaque game payloads, and destination channels
  carried by light packets;
- receiver-local Doppler measurement and authored passbands;
- bounded worldline and arrival histories;
- past-light-cone source selection, aberration, and Doppler presentation;
- exact constant-velocity null interception for light tag;
- rollback-safe experiment phase, clocks, signals, cooldowns, and dialogue facts.

Relativity never becomes a second body-motion authority. The controlled body
uses Ambition's ordinary shared movement kernel. Plaza characters own
prescribed content worldlines, written once before relativity samples their
clocks.

## Festival presentation contract

The plaza communicates relativity through characters and world objects before
asking the participant to read instruments:

- every clock-bearing character has a visible numerical clock and hand driven by
  that character's proper time;
- questions, replies, notes, and tag shots carry short labels on the moving light
  packet itself;
- the opening asks the participant to synchronize at the lab, drift away, and
  then exceed 50% of light speed before the station sequence begins;
- DJ Blue Shift publishes a continuous visual pitch meter. The authored note is
  G2 at 98 Hz; a 0.6c approach shifts it by exactly one octave into G3 at 196 Hz;
- light tag has three hits. Round one shows visible/current/intercept/aim facts,
  round two hides coordinate-now, and round three hides the intercept marker;
- after reunion, the space+time view becomes a replay. Left/right scrubs the
  recorded worldlines and completed signal paths; Interact returns to the map.

The pitch meter is continuous and exact. Each transmitted sample also plays a
provider-owned procedural tone quantized to a nearby teaching note; a successful
G3 lock resolves as a short octave flourish. The visual meter remains the precise
measurement authority, while the audible sample supplies immediate classroom
feedback without adding audio state to the simulation.

## Plaza stations

### 1. Clock race / census

Courier, Drifter, and Spinner follow visibly marked circular worldlines at
approximately 35%, 55%, and 75% of light speed. Their orbit paths, radial arms,
large clock faces, and speed labels stay visible in the laboratory map so the
motion reads immediately. Interacting near a character sends a light-speed request. The character replies with the value of
their proper-time clock **when the reply left**. The received dialogue shows:

- the sender's clock at emission;
- the participant's clock at reception;
- laboratory time at reception;
- signal travel time.

This makes delayed dialogue a mechanic rather than an instant UI query.

### 2. Doppler dance

The onboard transmitter emits G2 at 98 Hz in its own rest frame. DJ Blue Shift
accepts G3 around 196 Hz. At a 0.6c approach the longitudinal SR Doppler factor
is exactly two, so the correct movement produces an octave rather than an
arbitrary numeric passband. The participant changes relative
velocity and sends the note. Rejected notes report the measured frequency and
say whether it was too low or too high; the DJ dances only when the locally
measured frequency falls inside the authored passband.

### 3. Light tag

Photon Fox follows a broad, near-inertial arc whose curvature is negligible
during the encounter. In optical mode the visible image is
where the arriving light left Fox, not Fox's coordinate position now. The
participant sends a light pulse toward the future intercept event. The engine
computes the exact observer-local emission direction. During this teaching
station, red marks the light-delayed image, yellow marks coordinate-now, green
marks the future intercept, and cyan marks the participant's local aim.

### 4. Reunion

The participant returns to the laboratory twin and compares clocks at one
shared event. Existing entity-local animation/timing systems consume the same
proper-time scale as the displayed clock.

## Movement and controls

TwinTrack authors `FreeFlight`, not `RunJump`:

- the movement stick changes spatial proper velocity in 2D;
- inertia continues after release and authored drag provides braking;
- coordinate speed approaches the terminal below `c`;
- diagonal input cannot exceed the radial speed cap;
- no jump or flight-toggle action is advertised;
- Interact sends messages, changes the view display, or completes reunion.

The reusable distinction is `fly` versus `fly_toggle`: a body can have permanent
flight without exposing a meaningless on-screen toggle button.

## Three views

### Laboratory map

Shows authoritative coordinate positions. It is the readable navigation and
social-play view. Characters, labels, active light pulses, dialogue bubbles,
the laboratory twin, and the view console are visible in world space.

### What reaches you now

A full-screen observer presentation shows a synthetic star field and compact
character emitters transformed for the controlled observer:

- photon directions undergo exact flat-spacetime aberration;
- frequencies undergo exact local Doppler measurement;
- source images come from light-delayed worldline events;
- brightness uses a documented point-source beaming proxy.

It is exact for ideal point sources and compact proxies, not a claim of complete
optical reconstruction for extended sprites or tile geometry. The classroom
optical view also overlays a 24-beacon reference ring: the beacons are uniformly
spaced in the laboratory frame, then transformed into the controlled observer's
local frame. Their visible bunching toward the direction of travel and their
Doppler color shift make aberration legible even before a student knows the
formula.

### Default-on spacetime minimap

The perspective spacetime exhibit is composited over the live laboratory/optical
view in a physical camera viewport, following the same higher-order Camera3d
composition pattern proven by the Lunex kaleidoscope menu. It is visible by
default and can be hidden/shown with `M` or the Special input. The minimap never
replaces or pauses the 2D simulation. TwinTrack's authored room contains no solid
perimeter wall blocks. Its camera policy is explicitly unclamped across an
effectively unbounded teaching region, so the ordinary smooth follow camera keeps
the controlled body centered even after leaving the authored room rectangle. The
laboratory sits at the room center and the 35%/55%/75%-of-light clock racers are
clustered within the opening view instead of being spread against room edges.

### Perspective 2+1D spacetime exhibit

The minimap teaching view is an actual Bevy `Camera3d` scene rather than a
2D isometric drawing. It renders the same derived history as a perspective
space+time sculpture:

- X and Z are the plaza's two spatial coordinates;
- vertical Y is `ct`, so light forms the expected 45-degree cone in graph units;
- each character's worldline is a thick colored 3D tube; circular motion becomes
  a helix, making the value of having two gameplay dimensions immediately visible;
- bright gold beads mark one second of that character's **own clock**. At the same
  laboratory-time height, faster worldlines accumulate fewer beads;
- yellow beams are actual recorded light-signal paths;
- a blue wire cone is the controlled observer's past light cone;
- a translucent white plane is the laboratory "now" slice;
- a translucent cyan plane is the controlled observer's instantaneous
  simultaneity slice and tilts as their velocity changes;
- the perspective camera slowly orbits the sculpture so depth is unmistakable.

This is a 3D visualization of 2+1D SR data, not a 3D gameplay runtime. The plaza
remains a 2D game and the exhibit is a read-only presentation consumer. After
reunion, left/right scrubs the selected laboratory time while the 3D graph
rebuilds around that event.

## Acceptance

- the provider runs standalone and inside `ambition_app`;
- the controlled body's action scheme contains Interact but no Jump or Fly
  Toggle;
- shared free flight moves diagonally and remains below both the authored
  terminal speed and invariant speed;
- a forced 0.9c worldline matches the analytic SR proper-time ratio;
- seven named clocks are published: participant, laboratory, and five plaza
  characters;
- clock requests and replies carry stable character identity and clock values
  as light-signal payloads;
- the guided opening requires synchronization, spatial separation, and at least
  50% of light speed before the census;
- all three clock reports return before the course advances;
- continuous Doppler feedback identifies the G2/G3 notes and the DJ passband
  accepts the exact 0.6c octave shift;
- Photon Fox's visible direction and future light-intercept direction differ;
- three light-tag hits advance through progressively reduced assistance;
- one complete scripted course reaches reunion with the laboratory clock ahead
  of the participant clock;
- switching views changes presentation only;
- a completed experiment can scrub its canonical replay cursor over the retained
  derived worldline and signal history;
- the perspective 3D spacetime exhibit renders helical worldlines, one-own-second
  beads, a wire past light cone, and distinct lab/observer simultaneity planes;
- leaving the session removes its spacetime provider and derived views.

## Deliberate limits

TwinTrack remains special relativity in flat spacetime. It does not vary the
invariant speed spatially, evolve a gravitational metric, ray-trace arbitrary
extended geometry, or reconstruct a complete 3D optical world. Those are
separate future capabilities. The plaza is the grounded testbed from which the
3D Slower Light game and later GR research can grow. The 3D teaching camera does
not remove that boundary: it visualizes 2+1D data, while Slower Light still needs
a true 3D game world and observer renderer.
