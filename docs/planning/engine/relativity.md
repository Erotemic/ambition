# Relativity capability

> **Status (2026-08-12): SR-8 keeps the default-on 3D 2+1D minimap and finishes the open-plaza camera contract: TwinTrack recenters its teaching exhibits around the laboratory, authors no perimeter collision, and uses an unclamped follow-camera zone plus a deep zero-gravity blast margin so leaving the authored rectangle does not pin or reset the controlled body. The change remains game-authored and adds no relativity runtime cost to other experiences. Local Rust compile and visible-feel validation remain.**

Ambition treats special relativity as the first exact spacetime model, not as a
bag of visual effects. The reusable boundary is:

```text
relativistic consumers
  clocks, local mechanisms, signals, observers, presentation
                         ↑
2D spacetime-provider and null-signal adapter
                         ↑
Minkowski now; analytic/sampled/evolved GR later
```

## Ownership

- `ambition_relativity` owns dimension-independent Minkowski interval,
  proper-time, rapidity, velocity-composition, event-boost, and photon-frequency
  mathematics. It has no Bevy dependency.
- `ambition_relativity2d` reads canonical 2D body kinematics, samples a
  session-owned spacetime provider, writes the existing `ProperTimeScale`,
  accumulates f64 proper time, propagates analytic null signals, measures local
  receiver frequency, records bounded arrival/worldline telemetry, solves
  light-delayed compact-source events, and publishes observer-specific presentation
  read models.
- A provider owns the selected spacetime model and coordinate-time epoch.
  TwinTrack selects Minkowski with an authored invariant speed.
- Relativity observes movement authorities. It does not become a pose or
  velocity writer; TwinTrack's prescribed target worldline remains game-owned content.

## Current SR systems

### Clocks and intervals

Marked entities receive a spacetime-derived proper-time rate. Null and
spacelike worldlines are classified without generating NaNs. Engine mechanisms
that already consume `ProperTimeScale` require no relativity-specific branch.

### Events and observer coordinates

`MinkowskiEvent` and exact Lorentz boosts transform event-coordinate
differences between inertial frames. These are coordinate transforms, not
claims about optical appearance. TwinTrack uses this surface to report a real
arrival event relative to the traveler's instantaneous inertial frame.

### Analytic null signals and messages

A light signal is canonical state defined by an emission event, normalized
chart direction, chart frequency, packet identity, stable emitter tag, opaque
game payload, and invariant speed. The signal layer does not interpret the
payload: TwinTrack uses it for clock requests/replies, Doppler notes, and light
tag, while another game can carry different finite-speed information. Its
position is evaluated analytically rather than integrated as an ordinary
projectile. Swept signal/receiver intersections prevent tunneling and are sorted
by coordinate-arrival fraction before effects are applied.

### Local emission and reception

An emitter declares a source-local proper frequency. The SR kernel converts it
to chart frequency from the emitter four-velocity. A receiver then measures
that photon frequency against its own local four-velocity. Passbands are
content policy; the measurement is reusable engine work.

Reflect mode is a coherent retroreflector: the incoming frequency measured in
the receiver frame becomes the outgoing source-local frequency for the returned
chart direction. The packet retains identity and payload. More general
specular/tetrad policies remain additive work.

### Coordinate and proper timers

Session coordinate time advances once per simulation tick. `ProperTimeCooldown2d`
advances through the marked entity's proper-time rate. Coordinate-time resets
increment an epoch so derived histories cannot mix separate experiments.

### Telemetry

Worldline telemetry is opt-in, bounded, and derived. It is keyed by simulation
tick, truncates abandoned rollback futures, and clears when coordinate epochs
change. Signal-arrival history is bounded canonical state because game rules
may depend on which packet reached which receiver and when. Receiver passbands
and deterministic signal-pool slots are canonical too, while transient emission
and arrival message buffers are cleared on rollback before resimulation.

### Observer past-light-cone views

`RelativisticObserver2d` selects one canonical body as an observer without
changing the simulation chart. `OpticalSource2d` marks compact emitters whose
bounded worldlines may be intersected with the observer event's past light
cone. The solver brackets the null intersection in recorded samples and uses a
fixed bisection count, making the derived answer deterministic.

The SR kernel then transforms the arriving photon direction into the observer's
local inertial frame and reports exact flat-spacetime aberration and Doppler
measurements. A documented `D^3` point-source beaming proxy is presentation
policy, not a claim of full radiative transfer. The resulting
`RelativisticOpticalView2d` is derived state rebuilt after rewind.


### Causal targeting

`RelativisticTarget2d` opts one compact body into an observer-relative targeting
read model. For each active observer, the adapter solves the earliest future
constant-velocity intersection between a newly emitted null ray and the target
worldline. It publishes four deliberately separate facts:

- the target's light-delayed apparent direction from the optical view;
- the direction to its current coordinate position;
- the exact chart direction to the future intercept event;
- that firing direction transformed into the observer's local inertial frame.

The pure solver evaluates `|r + vt| = ct`, accepts only finite timelike target
velocities, and validates the returned root against the null distance. Controller
aim is stored in the observer-local frame and transformed back to the chart only
when an emission request is authored. Targeting is a derived view; it never
steers an actor, moves a target, or mutates a signal. This is the same separation
a future GR implementation needs, although curved providers will replace the
closed-form Minkowski intercept with null-geodesic boundary-value solving.

### Proper-velocity free flight

The pure crate exposes algebraic conversions between coordinate velocity and
spatial proper velocity (`w = gamma v`). The shared axis-swept flight limb can
optionally integrate acceleration and drag in spatial proper velocity and then
convert back to coordinate velocity. This guarantees subluminal output for
finite inputs while retaining an authored coordinate-speed terminal.

The guarantee is a postcondition of the LIMB, not a property of that one
integration mode. `FlightTuning::coordinate_speed_cap` is the single bound every
control policy's output passes through — the authored terminal, held strictly
below `c` whenever an invariant speed is authored. A direct-velocity command (a
boss pattern driving an exact per-tick velocity, which bypasses the
proper-velocity integration entirely) is therefore subluminal for the same reason
the accelerated path is, and authoring a terminal at or above `c` yields a slower
body rather than a broken invariant.

This is not a relativity-owned pose writer. It is an optional tuning mode of the
same movement authority used by ordinary flying bodies. Radial terminal-speed
enforcement prevents diagonal input from reaching `sqrt(2)` times the authored
cap. `AbilitySet` separately represents flight capability and permission to
expose a flight-toggle action, allowing permanent 2D flight with no phantom Jump
or Toggle button.

## Cost contract

Games that do not enable the facade's `relativity` feature do not link either
crate. Linking the crates does no work. Installing `Relativity2dPlugin` adds only
spacetime-presence checks until a session owns `ActiveSpacetime2d`.

Costs are proportional only to opted-in data:

- marked clocks: one model sample and small clock calculation per tick;
- active signals: analytic position plus swept tests against registered
  receivers;
- tracked worldlines: one bounded sample per marked track per tick;
- optical sources: one bounded-history light-cone solve per marked source and active observer;
- causal targets: one closed-form quadratic null-intercept plus one local-frame
  transform per marked target and active observer;
- presentation: observer views rebuild only when a live spacetime exists.

There is no universal shader, global history buffer, or relativity scan over
ordinary bodies. Proper-velocity flight runs only for a body whose normal
movement tuning opts into an invariant speed. TwinTrack alone installs the
relativity plugin, marks clocks/sources, and activates the full-screen optical
presentation and synthetic star field.

### Re-measured 2026-09-03 — the contract holds NOW, and did not when it was written

The first sentence above ("games that do not enable the facade's `relativity`
feature do not link either crate") is TRUE at HEAD, checked two-sided across
every shipped app rather than only the one it is about — `cargo tree` per app,
counting `ambition_relativity v`:

| App | links relativity | tree |
|---|---|---|
| `ambition_app` | **0** | 2,686 |
| `ambition_demo_mary_o_app` | 0 | 1,841 |
| `ambition_demo_sanic_app` | 0 | 1,841 |
| `ambition_demo_smash_app` | 0 | 1,854 |
| `ambition_demo_pocket` | 0 | 1,838 |
| `ambition_demo_twintrack_app` | **2** | 1,849 |

Exactly one app links it and it is the one this doc names. The five zeros are
meaningful because the sixth is not zero: a grep that finds nothing everywhere
is indistinguishable from a broken grep.

⛔ **AND IT WAS FALSE FOR THE FIRST THREE WEEKS THIS PAGE ASSERTED IT.** The
status header above is dated 2026-08-12. `relativity` was listed in the facade's
`all_capabilities` until **2026-09-01**, and `game/ambition_content` takes
`all_capabilities` — so the shipped game linked both crates for weeks *without
naming the feature*, which is precisely what the contract forbids. The facade's
own comment now records the removal and the reason:

> *"nothing in the shipped game asks for spacetime, and listing it here put
> `ambition_relativity` + `ambition_relativity2d` into every build that took the
> default features."*

⇒ The lesson is about the shape of the claim, not the crates. **A cost contract
phrased as "games that do not opt in do not pay" is not enforced by the feature
flag it names** — it is enforced by every aggregate feature that might contain
it, and `all_capabilities` is exactly such an aggregate. The contract became
true when someone audited the aggregate, three weeks after the page promised it.
⚠ Nothing here is guarded: no policy row asserts that `all_capabilities` excludes
`relativity`, so the same edit re-introduces the same silent regression. That is
the cheapest available follow-up on this page.

*Method note.* Enumerating the apps with `ls game/ | grep -E '_app$'` returned
NOTHING — `ls` is emitting colour escapes here, so the reset sequence sits after
the final `p` and the anchor cannot match. An empty list would have read as "no
demo apps to check". `find … -name Cargo.toml` is immune.

## GR growth path

A future GR provider can preserve the consumer-facing concepts while adding
metric samples, tetrads, derivatives, and geodesic integration through additive
traits. Likely providers are:

1. analytic stationary metrics;
2. sampled numerical-relativity output;
3. reduced-order or AI surrogate fields;
4. a live Rust field solver.

Minkowski remains the exact flat limit and regression oracle. Proper clocks,
emission/reception events, photon wavevectors/frequencies, local observer
measurements, and worldline diagnostics remain useful around curved or evolved
metrics. `ambition_relativity2d` means a two-coordinate game adapter, not a
commitment to physical 2+1-dimensional gravity.

## Explicitly absent from this slice

- curved metrics and timelike/null geodesic solvers;
- dynamic spacetime or backreaction;
- general relativistic collision response;
- light-delayed rendering of arbitrary extended/tile geometry;
- Doppler/aberration full-screen shaders or full radiative transfer;
- a separate relativity movement authority or global speed clamp;
- accelerated-target or curved-spacetime intercept solvers;
- the future 3D Slower Light game.

## SR-5 festival polish boundary

SR-5 adds no new relativistic law. It improves the game-facing consumers of the
existing exact systems:

- proper-time clocks drive world-space hands/readouts;
- packet payloads drive presentation labels without changing signal authority;
- continuous Doppler preview is derived from body state and the Minkowski
  measurement helper rather than stored as canonical state;
- multi-round assistance changes only presentation declarations; every hit still
  uses the same exact null-intercept and swept receiver path;
- TwinTrack raises the opt-in worldline/arrival history capacities for its own
  replay, so other games retain zero runtime and allocation cost unless they
  install and configure the relativity capability.


## SR-6 classroom spectacle boundary

SR-6 still adds no new relativistic law. It makes already-authoritative SR facts
visually interpretable:

- orbiting plaza characters expose their authored paths, radial arms, speed as a
  percentage of `c`, and much larger proper-time clock faces;
- the 3D teaching scene maps the two gameplay coordinates into X/Z and `ct` into
  vertical Y, so circular motion becomes a helix and null propagation forms a
  literal light cone;
- one-second beads are derived from each track's existing `proper_time` samples,
  making differential aging visible as bead density rather than another number;
- the laboratory-now and observer-now planes are derived from the same selected
  event and observer coordinate velocity. The observer plane is the flat-space
  instantaneous simultaneity slice, not a second simulation clock;
- the 3D camera, meshes, orbit guides, and all new labels are visible-feature
  presentation. Headless games and games that do not select TwinTrack pay no
  runtime cost.

The exhibit is intentionally not the deferred 3D Slower Light game: it renders a
3D graph of 2+1D data while the game world, collisions, optics, and controls remain
2D.

The SR-6 classroom spectacle slice remains presentation-only: a perspective 3D
2+1D worldline exhibit consumes existing derived histories and signal views. It
does not add a 3D gameplay world, a second simulation authority, or curved
spacetime behavior. The same slice adds an exact aberration reference ring in
TwinTrack's optical presentation from uniformly spaced laboratory-frame point
sources.
