# TwinTrack — Relativity Plaza

> **Status (2026-08-06): SR-5 Relativity Festival polish implemented.** TwinTrack
> now teaches its systems through a guided opening, visible character clocks,
> labeled light-message packets, continuous Doppler-note feedback, three rounds
> of progressively less-assisted light tag, and a scrub-able post-experiment
> spacetime replay. The default HUD remains small; numerical diagnostics stay in
> the teaching view.

TwinTrack is Ambition's executable acceptance game for flat-spacetime
relativity. Its first duty remains technical: prove that clocks, worldlines,
light signals, local measurements, observer views, and rollback compose through
normal engine seams. Its second duty is now equally explicit: show recognizable
gameplay ideas that could grow into a visually strange relativity game.

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

### 1. Clock census

Courier, Drifter, and Spinner follow different 2D worldlines. Interacting near a
character sends a light-speed request. The character replies with the value of
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
optical reconstruction for extended sprites or tile geometry.

### 2+1D spacetime sculpture

The optional teaching view represents two spatial dimensions plus time:

- one axis for x position;
- one axis for y position;
- one upward time axis;
- colored character worldline trails;
- yellow light-signal paths;
- blue past-light-cone edges;
- explicit labels and a legend;
- an isometric projection that can later become an orbitable 3D teaching view.

The sculpture explains or replays the same simulation. It is not required for
steering and does not become a second game world. After reunion it uses the
festival's extended worldline history and arrival records: the horizontal input
scrubs a normalized replay cursor over the completed experiment.

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
- leaving the session removes its spacetime provider and derived views.

## Deliberate limits

TwinTrack remains special relativity in flat spacetime. It does not vary the
invariant speed spatially, evolve a gravitational metric, ray-trace arbitrary
extended geometry, or reconstruct a complete 3D optical world. Those are
separate future capabilities. The plaza is the grounded testbed from which the
3D Slower Light game and later GR research can grow.
