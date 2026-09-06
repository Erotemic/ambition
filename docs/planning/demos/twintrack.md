# TwinTrack — Relativity Plaza

> **⏸ PARKED 2026-08-20.** Jon: *"I want to put twintrack down for now and get
> back to the main games. Twin track is fun, but not the primary target."* The
> second seat is confirmed driving on hardware and the plaza is in a finished
> state; nothing below is queued work. ⛔ **and the exhibit has a KNOWN LIMIT**
> that a future session must not rediscover as a bug: both panes render one
> instant of the simulation's coordinate time, so they can disagree about optics
> (light delay, aberration, Doppler) and NOT about simultaneity, which is what
> the twin paradox is.
> ⚠ **THIS PAGE IS NOW THE ONLY RECORD OF THAT LIMIT.** It was also filed as
> question 28 in [`../awaiting-maintainer-decision.md`](../awaiting-maintainer-decision.md),
> and that entry is GONE — not answered and archived, just absent, and the text
> appears nowhere else in `docs/`. That file states it "intentionally does not
> retain answered decision transcripts", so a PARKED question living there was
> always one cleanup away from deletion. ⇒ The pointer is removed rather than
> repaired, because there is nothing to point at; the limit survives only
> because it was written out here too. (Checked 2026-09-03.) ⇒ Re-filed the same
> day as question 47 in [`../awaiting-maintainer-decision.md`](../awaiting-maintainer-decision.md),
> with the convention that an answer row names the number it closes.
>
> **Status (2026-08-20): TwinTrack is a TWO-PLAYER game with a real split
> screen.** The laboratory twin is Emmy No-Ether, driven by seat one; the plaza
> opens split, one gameplay view per participant. See SR-11 below for what that
> cost the engine and what it still owes. Earlier: SR-8's open-follow camera and
> centered plaza (2026-08-12) are the spatial baseline it sits on.

TwinTrack is Ambition's executable acceptance game for flat-spacetime
relativity. Its first duty remains technical: prove that clocks, worldlines,
light signals, local measurements, observer views, and rollback compose through
normal engine seams. Its second duty is now equally explicit: show recognizable
gameplay ideas that could grow into a visually strange relativity game.

✔ **Dual-observer split screen landed 2026-08-20** (SR-11): two controlled
participants share one authoritative Minkowski simulation and each viewport is
framed from that participant's own body. What is NOT yet per-view is the
relativistic OPTICAL presentation — `RelativisticOpticalView2d` is a
single-observer resource — so the two panes differ in framing and in every
instrument reading, and not yet in aberration and Doppler.

> **Landed 2026-08-20 — SR-9 ordering exhibit.** An instrument drawing two
> demo-owned panes: the laboratory twin at rest and the controlled traveler. Two
> beacons at rest in the laboratory, symmetric about the lab twin, flash together
> in laboratory coordinate time; each pane reports **which flash's light reached
> its observer first** and **which flash happened first in its observer's own
> frame** (an exact `lorentz_boost_event`), plus that observer's own length
> contraction of the beacon axis. The lab pane answers `SIMULTANEOUS` by
> construction; the traveler pane answers with an order whose sign is the sign of
> its velocity. Headless tests in `twintrack_it` assert the two panes disagree and
> that reversing the traveler reverses its answer.
>
> ⚠ **it is an INSTRUMENT, not the split screen** — a diagram over the top of the
> gameplay panes, selected from the in-world view console like the optical view
> and the 2+1D minimap. The real split landed separately (below); this stays what
> it always was and only swapped sides, so its left pane is now the traveler's,
> matching the participant order underneath it.
>
> ⚠ **this note used to say `RelativisticOpticalView2d` was single-observer, and
> that was WRONG twice over.** `publish_optical_view` publishes one image per
> observer and had for a while; `publish_targeting_view` was the one still doing
> `observers.single()` — which did not merely fail to produce a second aim, it
> BLANKED the only one there was — and that landed 2026-08-20. Both resources now
> hold one row per observer, keyed by observer entity, published in the same
> sorted order so a per-pane consumer can pair them, with a `Deref` to the first
> row that keeps every one-observer reading byte-identical.
>
> ✔ **ADOPTED the same day.** The laboratory twin carries her own
> `RelativisticObserver2d`, so the plaza publishes TWO optical images and TWO
> sets of intercepts, and every TwinTrack consumer — the teacher HUD, the
> observatory's stars, aberration beacons and proxies, the light-tag guides, and
> the integration suite — names the observer it is drawing for instead of taking
> the first row. ⛔ the two had to land together: `"laboratory"` sorts before
> `"traveler"`, so making her an observer without naming the observers first
> would have silently redrawn the traveler's instruments from her eyes.
>
> Guarded by `both_observers_publish_their_own_sky_and_the_two_disagree`, whose
> falsifier is the interesting half: two rows carrying the SAME numbers would
> mean one image had been computed and copied, which is what the `Deref` did.
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


> **Landed 2026-08-20 — SR-11 TWO PARTICIPANTS, TWO REAL VIEWS.** The laboratory
> twin is Emmy No-Ether: a constructed character body wearing
> `DrivingParticipant(PlayerSlot(1))`, so the plaza's at-rest reference frame is
> a person a second controller steers instead of a prop with a clock. The screen
> is split by construction — one gameplay `LocalView` per participant, the
> traveler on the left and the twin on the right — and the panes are engine
> rectangles resolved from engine per-view framing, not composited diagrams.
>
> The three engine seams the acceptance list called for landed with it:
>
> - **`ambition_sim_view::ViewPlacement`** — where a view SITS, as a fraction of
>   the gameplay rectangle. `publish_camera_viewport` carves its resolved rect by
>   it, and `apply_gameplay_camera_viewport` already handed each camera the
>   rectangle of the view it presents. Absent means the whole rectangle, so every
>   single-view composition is unchanged.
> - **`ambition_sim_view::ViewSubject`** — the body a view FRAMES.
>   `resolve_camera_observation` resolved the followed body, the framing focus and
>   the reference-frame down axis ONCE above the per-view loop; those three are
>   per view now. A view naming no subject keeps the session's controlled body,
>   which is what that default always meant. `ControlledSubject` is untouched.
> - **`spawn_main_camera` declines to spawn a rig it cannot honestly bind**,
>   instead of spawning one and leaving the link off for every consumer to refuse.
>   A composition owning N views owns N rigs; the shared plugin's room visuals,
>   sprite chain and front HUD camera are unchanged.
>
> ⛔ **the second view is composed by the SESSION, never at plugin build time.**
> `ambition_app` links this crate beside the launcher, Mary-O and Smash, so a
> build-time second view splits the screen of every route in the game. The
> symptom is nowhere near the cause: with two views the shared camera-spawn site
> correctly refuses to bind, the demo's own rigs appear instead, and `bevy_egui`
> — which attaches its primary context to the first camera it sees — takes down
> 95 `app_it` tests with a message about schedules. A view APPEARING is the
> ordinary couch event of somebody joining. Guarded by
> `camera_names_its_view::the_launcher_has_one_view_and_no_split_layout`.
>
> ⛔ **two seats is TWO statements.** `DeclaredInputSeats(2)` gets seat one an
> `InputParticipant`; it does not get it a DEVICE, because the default
> `InputAssignmentPolicy` is `UnifiedPrimary` — every local source drives the
> primary participant, which is right for solo play and hands the only pad to the
> seat that already has the keyboard. Jon measured exactly that: *"I have a
> keyboard and controller hooked up to twin track, but they both control patent
> clerk, neither controls emmy."* TwinTrack claims `JoinToClaim` while its session
> is live and restores the default when it ends, which is the same route-scoped
> claim Smash makes. ⚠ **the headless suite cannot catch this class at all**: the
> integration tests build without the `input` feature, so they have no
> participants, no devices and no assignment pass — they write `SlotControls`
> directly. The declaration is asserted where a test can read it; the mechanism it
> buys is pinned in `ambition_input::local_seats`.
>
> ⚠ **one controller is a complete session.** A seat with no pad reads neutral
> input, so the twin stands still in the laboratory and her pane keeps framing
> her — an unattended observer is still an observer (Jon, 2026-08-20).
>
> ▢ **adaptive share/split is deliberately absent.** TwinTrack is permanently
> split for the reason recorded below; the policy that WRITES a `ViewPlacement`
> from subject separation with hysteresis is Ambition's product requirement and
> has no customer yet.

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

✔ **All of it landed on 2026-08-20** — see SR-11. *"Each view may choose
laboratory or observer-local presentation independently"* is real: both
`RelativisticOpticalView2d` and `RelativisticTargetingView2d` publish one row per
observer, the plaza has two observers, and every consumer names whose eyes it is
drawing from.

▢ **what is left is a PRESENTATION choice nobody has made**: the two gameplay
panes still draw the same world-space sprites, because which pane shows which
observer's aberrated sky is a design question about what the exhibit should look
like, not a missing seam.

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
