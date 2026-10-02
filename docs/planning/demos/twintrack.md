# TwinTrack — Relativity Plaza

## Purpose

TwinTrack is the executable acceptance game for flat-spacetime relativity. Its
first duty is technical: prove that clocks, worldlines, light signals, local
measurements, observer views and rollback compose through normal engine seams.
Its second duty is to show gameplay ideas that could grow into a visually strange
relativity game.

Engine capabilities it stresses:

- multi-view: two participants, two `LocalView`s and two observer-derived
  presentations over one authoritative Minkowski simulation
  ([`../engine/multiplayer-and-multiview.md`](../engine/multiplayer-and-multiview.md));
- per-route seating and input assignment (`RouteSeating`,
  `InputAssignmentPolicy::JoinToClaim`);
- permanent gravity-free 2D movement through the shared flight limb, with
  spatial proper-velocity control and a radial speed cap below `c`;
- Minkowski proper-time accumulation for every marked character;
- analytic null-signal propagation, with stable emitter identity, opaque game
  payloads and destination channels on light packets;
- receiver-local Doppler measurement and authored passbands;
- bounded worldline and arrival histories;
- past-light-cone source selection, aberration and Doppler presentation;
- exact constant-velocity null interception (light tag);
- rollback-safe experiment phase, clocks, signals, cooldowns and dialogue facts;
- a `Camera3d` presentation consumer composited over the 2D game.

Relativity never becomes a second body-motion authority. The controlled body uses
the shared movement kernel. Plaza characters own prescribed content worldlines,
written once before relativity samples their clocks.

## Status

**Parked** (Jon, 2026-08-20: *"I want to put twintrack down for now and get back
to the main games. Twin track is fun, but not the primary target."*). The plaza
is in a finished state and the second seat drives on hardware. Nothing here is
queued work.

Built: `game/ambition_demo_twintrack` (content) and
`game/ambition_demo_twintrack_app` (standalone shell).

Current shape:

- **Two players, permanent split screen.** The traveler (seat zero) is on the
  left. The laboratory twin, Emmy No-Ether, is a character body with
  `DrivingParticipant(PlayerSlot(1))`, on the right. Each pane is an engine
  rectangle: `ambition_sim_view::ViewPlacement` says where a view sits, and
  `ViewSubject` says which body it frames. A seat with no pad reads neutral
  input, so one controller is a complete session.
- **Per-observer optics.** `RelativisticOpticalView2d` and
  `RelativisticTargetingView2d` publish one row per observer, keyed by observer
  entity, in a sorted order. Every TwinTrack consumer names the observer it draws
  for. Do not take the first row.
- **Ordering exhibit (an instrument, not the split screen).** Two lab beacons
  flash together in lab time. Each pane reports which flash's light reached its
  observer first and which flash happened first in its own frame.
- **Light-speed pulse.** The twin fires a three-ray flare every
  `PULSE_PERIOD_SECONDS`. A ray has no stored position: its lab position derives
  from the emission event and `c`, and every observer-frame number is an exact
  `lorentz_boost_event`. A traveler at 0.9c reads `1.000 c`, not `c - v`.
  `TwinTrackLightPulseView` is derived each frame; it is not rollback state.

Standing rules:

- ⛔ **Known limit, not a bug.** Both panes render one instant of the
  simulation's coordinate time. They can disagree about optics (light delay,
  aberration, Doppler), not about simultaneity, which is what the twin paradox
  is. Where that limit should live is Q47 in
  [`../awaiting-maintainer-decision.md`](../awaiting-maintainer-decision.md).
- ⛔ Compose the second view in the session, never at plugin build time.
  `ambition_app` links this crate, so a build-time second view splits every
  route in the game. Guard:
  `camera_names_its_view::the_launcher_has_one_view_and_no_split_layout`.
- Seating is declared per route: `declare_route_seating` with
  `RouteSeating::new(SeatCount::Fixed(..), InputAssignmentPolicy::JoinToClaim)`
  and a `LocalChannelPlan` (keyboard drives the traveler, first pad drives the
  twin). A seat count alone is not enough: the default `UnifiedPrimary` policy
  sends every device to seat zero. The headless suite builds without the `input`
  feature and cannot catch this class; the mechanism is pinned in
  `ambition_input::local_seats`.
- TwinTrack stays permanently split. A view that merges when observers are close
  would hide the phenomenon when the comparison is most instructive. Adaptive
  share/split is Ambition's product requirement, not this demo's.

## Design

**Premise.** Everyone starts with a clock. The participant flies around a plaza
and asks moving characters what their clocks read. Questions and replies travel
as light, so an answer is already old when it arrives.

**Player-facing language:** *your clock* (proper time), *laboratory time* (the
Minkowski coordinate chart), *what reaches you now* or *light-delayed image* (the
source event whose light arrives now), *when this message left* (emission
event), and speed as a percentage of light speed. Do not use "retarded event" in
game text.

**Presentation contract.** Every clock-bearing character shows a clock driven by
its proper time. Questions, replies, notes and tag shots carry short labels on
the light packet. The opening asks the participant to synchronize at the lab,
drift away and exceed 50% of light speed before the stations start.

**Stations:**

1. **Clock census.** Courier, Drifter and Spinner orbit at about 35%, 55% and
   75% of light speed. A request goes out as light; the reply carries the
   sender's clock when the reply left. The dialogue shows sender clock at
   emission, participant clock at reception, lab time at reception and travel
   time.
2. **Doppler dance.** The transmitter emits G2 (98 Hz). DJ Blue Shift accepts G3
   (196 Hz). A 0.6c approach doubles the frequency exactly. The visual pitch meter
   is the measurement authority; a quantized procedural tone gives audible
   feedback without simulation audio state.
3. **Light tag.** Photon Fox follows a near-inertial arc. The participant aims at
   the future intercept. Colors: red is the light-delayed image, yellow is
   coordinate-now, green is the intercept, cyan is local aim. Three hits, with
   less assistance each round.
4. **Reunion.** Compare clocks with the lab twin at one shared event.

**Movement.** `FreeFlight`, not `RunJump`: the stick changes spatial proper
velocity, inertia continues, drag brakes, coordinate speed stays below `c`, and
diagonal input cannot exceed the radial cap. No jump or flight-toggle action is
advertised (`fly` versus `fly_toggle`). Interact sends messages, changes the
view, or completes reunion.

**Views.** The laboratory map shows authoritative coordinate positions. *What
reaches you now* shows a star field and compact emitters with exact aberration,
Doppler and light-delayed source images, plus a 24-beacon reference ring; it is
exact for point sources, not for extended sprites. The 2+1D spacetime exhibit is
a `Camera3d` minimap (toggle with `M` or Special): worldlines as tubes (helices
for orbits), one bead per own-clock second, recorded light paths, the observer's
past light cone, and lab and observer simultaneity planes. After reunion,
left/right scrubs the replay. The room has no perimeter walls and the camera is
unclamped.

## Open acceptance

None is queued while the demo is parked. Recorded follow-ups:

- **Player-fired pulse.** It needs the emission event (coordinate time, position,
  direction) persisted on `TwinTrackExperiment`, which is a snapshot schema
  change.
- **Which pane shows which observer's aberrated sky.** The two gameplay panes
  still draw the same world-space sprites. This is a presentation design choice,
  not a missing seam.

Standing acceptance contract:

- the provider runs standalone and inside `ambition_app`;
- the action scheme has Interact and no Jump or Fly Toggle;
- free flight moves diagonally below the terminal speed and `c`;
- a forced 0.9c worldline matches the analytic proper-time ratio;
- seven named clocks are published (participant, laboratory, five plaza
  characters);
- requests and replies carry stable character identity and clock values as
  light-signal payloads;
- the opening requires synchronization, separation and at least 50% of light
  speed before the census, and all three reports return before the course
  advances;
- the DJ passband accepts the exact 0.6c octave shift;
- Photon Fox's visible direction and intercept direction differ; three hits
  advance through reduced assistance;
- a scripted course reaches reunion with the lab clock ahead;
- switching views changes presentation only;
- a completed experiment scrubs its replay cursor over retained history;
- the 3D exhibit renders helical worldlines, beads, a past light cone and
  distinct simultaneity planes;
- two bodies move at once through participant-aware input, each pane follows its
  observer, and rollback and headless state do not depend on the view count;
- leaving the session removes its spacetime provider and derived views.

## Deliberate limits

TwinTrack is special relativity in flat spacetime. It does not vary the invariant
speed spatially, evolve a gravitational metric, ray-trace extended geometry, or
reconstruct a 3D optical world. Those are separate future capabilities (the 3D
Slower Light game, later GR research). The 3D exhibit visualizes 2+1D data; it is
not a 3D gameplay runtime.

## How to run

```bash
./run_game.sh twintrack                      # windowed, two-seat split screen
./run_game.sh twintrack --headless -- --ticks 600
cargo test -p ambition_demo_twintrack -p ambition_demo_twintrack_app
cargo run -p ambition_demo_twintrack_app --features capture \
    --bin capture_twintrack -- OUT.png [WIDTHxHEIGHT] [--warmup N] [--run N] [--no-ui] [--split]
```
