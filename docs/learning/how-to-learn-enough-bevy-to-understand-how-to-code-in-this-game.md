# How to learn enough Bevy to understand how to code in this game

This is an advanced, project-specific Bevy course for working on Ambition without depending on an LLM.

It assumes:

- you know Python well;
- you understand ordinary Rust syntax, ownership, enums, traits, `Option`, and `Result`;
- you can read and modify a small Rust program;
- you have access to the companion course [`how-to-learn-enough-rust-to-understand-how-to-code-in-this-game.md`](./how-to-learn-enough-rust-to-understand-how-to-code-in-this-game.md);
- you want to understand Bevy as an execution model, not merely memorize component/query syntax.

Ambition currently pins Bevy 0.18.1. Bevy changes quickly, so use the exact version in the workspace's `Cargo.lock` when checking external examples.

The goal is to make these parts of Ambition legible:

- how an `App` is composed;
- how plugins establish contracts;
- how entities, components, resources, messages, and assets differ;
- how system parameters declare authority;
- how schedules and sets produce temporal structure;
- why commands are deferred;
- how fixed simulation differs from frame presentation;
- how a shared title host creates and retires exact gameplay sessions;
- how no-window and recording test applications differ from the shipping app;
- how to debug missing resources, duplicate authority, stale messages, and schedule-order bugs.

This is not a general game-design course and not a tour of every Bevy feature. It teaches the Bevy concepts needed to continue developing this repository deliberately.

## 1. The central translation from Python game code to Bevy

A Python game often grows around long-lived objects:

```python
class Game:
    def __init__(self):
        self.player = Player()
        self.room = Room()
        self.audio = AudioManager()

    def update(self, dt):
        self.player.update(dt, self.room)
        self.audio.update(dt)
```

Bevy decomposes this into data and scheduled transformations:

```text
World
├── entities with components
├── one-per-world resources
├── message queues
├── asset handles and asset storage
└── schedules containing systems
```

The closest translation is:

| Python instinct | Bevy replacement |
|---|---|
| One large game object | An `App` containing a `World` and schedules |
| Object fields | Components or resources |
| Object identity/reference | `Entity` ID |
| `update()` methods | Systems in schedules |
| Constructor wiring | Plugins configuring an `App` |
| Calling another manager | Message or shared typed state |
| Global singleton | App-local resource |
| Scene object destruction | Deferred entity despawn |
| Frame loop | `Update` and related schedules |
| Physics step | Fixed simulation schedule |
| Asset object | `Handle<T>` plus asset storage |
| Mocking the whole engine | Constructing a small real `App` in a test |

The crucial conceptual shift is:

> Bevy does not ask objects to update themselves. It schedules functions that transform selected data.

That means architecture is visible in:

- which component/resource types exist;
- which systems may read or mutate them;
- which plugin installs those systems;
- which schedule and set runs them;
- which lifecycle condition permits them to run.

## 2. Build a standalone Bevy laboratory

Do not begin by experimenting inside Ambition. Build a tiny headless crate where compilation is fast and every entity is understandable.

```bash
cargo new bevy-lab
cd bevy-lab
```

Use:

```toml
[package]
name = "bevy-lab"
version = "0.1.0"
edition = "2024"

[dependencies]
bevy = { version = "0.18.1", default-features = false }
```

A minimal application:

```rust
use bevy::prelude::*;

fn hello() {
    println!("hello from a Bevy system");
}

fn main() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_systems(Startup, hello);
    app.update();
}
```

`MinimalPlugins` gives you the core application, task, time, and schedule machinery without creating a window, renderer, or physical audio device.

Use this laboratory for the standalone examples in this course. Later, compare them with the corresponding Ambition system.

## 3. The complete mental model: `App`, `World`, schedules, and sub-apps

A Bevy `App` is the composition root.

It owns:

- the main ECS `World`;
- schedules that run systems against that world;
- plugin registration state;
- runners and exit behavior;
- optional sub-apps, such as the rendering app.

A `World` stores typed values:

- entities and their components;
- resources;
- message queues;
- internal change ticks;
- schedule data.

A schedule stores systems plus ordering constraints. `app.update()` invokes the configured main schedule, which runs familiar schedules such as startup, update, post-update, and fixed-time work according to Bevy's main-loop configuration.

A visible Bevy app may also contain a render sub-app. Data is extracted from the main world into that render world. This explains errors such as:

```text
Render app did not exist when trying to add extract_resource
```

That error does not mean the main ECS world is absent. It means a render-only plugin was installed into an application intentionally constructed without the render sub-app.

### Standalone inspection

```rust
use bevy::prelude::*;

#[derive(Resource, Default)]
struct Counter(u32);

fn increment(mut counter: ResMut<Counter>) {
    counter.0 += 1;
}

fn main() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<Counter>()
        .add_systems(Update, increment);

    app.update();
    app.update();

    assert_eq!(app.world().resource::<Counter>().0, 2);
}
```

This is a complete Bevy program. There is no window and no special test harness.

## 4. Entities are IDs, not objects

An `Entity` is a generational identifier. It is not a Rust reference and does not contain behavior.

```rust
use bevy::prelude::*;

#[derive(Component, Debug)]
struct NameTag(&'static str);

fn spawn(mut commands: Commands) {
    let entity = commands.spawn(NameTag("player")).id();
    println!("spawn request for {entity:?}");
}
```

The entity becomes meaningful through its components.

Important consequences:

- storing an `Entity` does not keep it alive;
- an entity ID may become invalid after despawn;
- generation bits help prevent a stale ID from referring to a newly reused slot;
- a query must still verify that the expected components exist;
- long-lived architecture should store typed identity and authority, not assume an entity remains valid forever.

In Ambition, exact session and activation IDs complement Bevy's entity generation. The entity generation prevents accidental slot reuse; session identity prevents stale work from one gameplay activation affecting another.

## 5. Components model per-entity facts

A component is a Rust type attached to zero or more entities.

```rust
use bevy::prelude::*;

#[derive(Component, Debug)]
struct Position(Vec2);

#[derive(Component, Debug)]
struct Velocity(Vec2);

#[derive(Component)]
struct Player;
```

Spawn a bundle of components:

```rust
fn spawn_player(mut commands: Commands) {
    commands.spawn((
        Player,
        Position(Vec2::new(2.0, 3.0)),
        Velocity(Vec2::new(4.0, 0.0)),
        Name::new("Player"),
    ));
}
```

The tuple implements `Bundle`. A bundle is only a convenient group of components; it is not a hidden object hierarchy.

Use components for facts that vary per entity:

- body position and velocity;
- health;
- character identity;
- session ownership;
- animation state;
- visual markers;
- cooldowns;
- interaction capability.

Avoid a single component that contains an entire mutable game object merely to recreate object-oriented design inside ECS.

### Standalone exercise

Create three entities:

- one `Player` with `Position` and `Velocity`;
- one enemy with `Position` and `Health`;
- one decoration with only `Position`.

Write down which queries select each entity before running the program.

## 6. Resources model one-per-world authority

A resource is one value of a type in a `World`.

```rust
use bevy::prelude::*;

#[derive(Resource, Debug)]
struct Gravity(f32);

fn main() {
    let mut app = App::new();
    app.insert_resource(Gravity(24.0));
    assert_eq!(app.world().resource::<Gravity>().0, 24.0);
}
```

Resources are appropriate for:

- active route or active gameplay session;
- settings;
- catalogs and registries;
- load coordinators;
- fixed simulation tick;
- global input frame;
- asset collections;
- app-local backend selection.

A resource is not a process global. Two `App` instances have separate worlds and separate values.

```rust
use bevy::prelude::*;

#[derive(Resource, Debug, PartialEq)]
struct SelectedProvider(&'static str);

fn main() {
    let mut a = App::new();
    let mut b = App::new();

    a.insert_resource(SelectedProvider("sanic"));
    b.insert_resource(SelectedProvider("mary-o"));

    assert_eq!(a.world().resource::<SelectedProvider>().0, "sanic");
    assert_eq!(b.world().resource::<SelectedProvider>().0, "mary-o");
}
```

This App-local property is essential for tests and provider composition.

## 7. Systems are ordinary functions with declared world access

A system is usually an ordinary Rust function whose parameters implement `SystemParam`.

```rust
use bevy::prelude::*;

#[derive(Component)]
struct Position(Vec2);

#[derive(Component)]
struct Velocity(Vec2);

fn integrate(mut bodies: Query<(&mut Position, &Velocity)>) {
    for (mut position, velocity) in &mut bodies {
        position.0 += velocity.0;
    }
}
```

The signature is both data access and scheduling metadata:

- `Query<&Position>` means shared component access;
- `Query<&mut Position>` means exclusive component access;
- `Res<Settings>` means shared resource access;
- `ResMut<Settings>` means exclusive resource access;
- `Commands` means deferred structural mutations;
- `MessageReader<T>` reads one message stream;
- `Local<T>` gives one system persistent private state.

Bevy can run compatible systems in parallel because the access is typed.

A system should be read as:

> During this schedule, this function has exactly these capabilities over the world.

## 8. Queries select archetypes by type shape

A query describes required component access and optional filters.

```rust
use bevy::prelude::*;

#[derive(Component)]
struct Position(Vec2);

#[derive(Component)]
struct Player;

#[derive(Component)]
struct Sleeping;

fn print_awake_players(
    players: Query<&Position, (With<Player>, Without<Sleeping>)>,
) {
    for position in &players {
        println!("{:?}", position.0);
    }
}
```

Common filters:

- `With<T>` — require `T` without borrowing it;
- `Without<T>` — reject entities with `T`;
- `Added<T>` — `T` was added since this system last observed the world;
- `Changed<T>` — `T` changed since this system last ran;
- `Or<(A, B)>` — match either filter;
- tuples combine filters with logical AND.

### Cardinality is a design decision

Use:

- iteration when zero or many are valid;
- `single()` when exactly one is required;
- `get(entity)` when identity is already known;
- `get_many()` when a small known set is required.

```rust
fn one_player(players: Query<Entity, With<Player>>) {
    match players.single() {
        Ok(player) => println!("player is {player:?}"),
        Err(error) => println!("not exactly one player: {error:?}"),
    }
}
```

Do not replace an exactly-one invariant with iteration merely to avoid handling the error. Conversely, do not call `single().unwrap()` in a system that legitimately runs during startup before the player exists.

## 9. Query conflicts are architecture feedback

This system is invalid because its mutable and shared queries may overlap:

```rust,compile_fail
fn invalid(
    mut movers: Query<&mut Position>,
    readers: Query<&Position>,
) {}
```

Bevy cannot prove that they access disjoint entities.

Make them disjoint with filters when that is the real architecture:

```rust
#[derive(Component)]
struct Dynamic;

#[derive(Component)]
struct Static;

fn valid(
    mut movers: Query<&mut Position, With<Dynamic>>,
    readers: Query<&Position, (With<Static>, Without<Dynamic>)>,
) {
    for mut position in &mut movers {
        position.0.x += 1.0;
    }
    for position in &readers {
        println!("static: {:?}", position.0);
    }
}
```

Use `ParamSet` only when the same logical system must perform conflicting accesses sequentially:

```rust
fn sequential(mut params: ParamSet<(
    Query<&mut Position, With<Dynamic>>,
    Query<&Position>,
)>) {
    for mut position in &mut params.p0() {
        position.0.x += 1.0;
    }

    let count = params.p1().iter().count();
    println!("{count} positioned entities");
}
```

A `ParamSet` is an explicit promise that accesses will not be held concurrently. It should not be the first response to a confusing ownership design.

## 10. `Commands` are deferred structural changes

`Commands` queues changes instead of immediately mutating archetype storage.

```rust
use bevy::prelude::*;

#[derive(Component)]
struct Spawned;

fn request_spawn(mut commands: Commands) {
    commands.spawn(Spawned);
}

fn count_spawned(query: Query<(), With<Spawned>>) {
    println!("visible now: {}", query.iter().count());
}
```

Without ordering and a deferred-command synchronization point, `count_spawned` may not see the entity requested by `request_spawn` during the same schedule pass.

This affects:

- spawn visibility;
- despawn visibility;
- inserted/removed components;
- entity hierarchy changes;
- tests that expect cleanup after one exact frame.

### A deterministic demonstration

```rust
use bevy::prelude::*;

#[derive(Component)]
struct Marker;

#[derive(Resource, Default)]
struct Observed(Vec<usize>);

fn spawn(mut commands: Commands) {
    commands.spawn(Marker);
}

fn observe(query: Query<(), With<Marker>>, mut observed: ResMut<Observed>) {
    observed.0.push(query.iter().count());
}

fn main() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<Observed>()
        .add_systems(Update, (spawn, observe).chain());

    app.update();
    app.update();

    println!("{:?}", app.world().resource::<Observed>().0);
}
```

`.chain()` establishes order and inserts the necessary deferred-command application between systems where Bevy supports it. Use it only when same-pass visibility is the contract.

## 11. Messages represent transient requests and observations

A message is typed transient data sent through the ECS world.

```rust
use bevy::prelude::*;

#[derive(Message, Debug)]
struct Damage {
    target: Entity,
    amount: u32,
}

#[derive(Component, Debug)]
struct Health(u32);

fn apply_damage(
    mut messages: MessageReader<Damage>,
    mut health: Query<&mut Health>,
) {
    for damage in messages.read() {
        let Ok(mut target_health) = health.get_mut(damage.target) else {
            continue;
        };
        target_health.0 = target_health.0.saturating_sub(damage.amount);
    }
}
```

Register the message:

```rust
app.add_message::<Damage>();
```

Write from a system:

```rust
fn attack(
    target: Single<Entity, With<Enemy>>,
    mut damage: MessageWriter<Damage>,
) {
    damage.write(Damage {
        target: *target,
        amount: 10,
    });
}
```

Write from a test or host:

```rust
app.world_mut().write_message(Damage {
    target,
    amount: 10,
});
```

Messages are appropriate for:

- commands crossing subsystem ownership;
- one-frame facts such as damage or interaction;
- route requests;
- lifecycle notifications;
- audio/presentation requests;
- load progress and cancellation.

Do not use a message when persistent state is required. Do not use one untyped catch-all message bus for unrelated meanings.

## 12. Messages do not automatically provide authority

A message queue answers “what was requested,” not “who was allowed to request it.”

This is unsafe in a multi-session host:

```rust
#[derive(Message)]
struct PlaySound(&'static str);
```

A stale session could emit the same message after a new session activates.

Add exact ownership:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SessionId(u64);

#[derive(Message)]
struct OwnedPlaySound {
    session: SessionId,
    cue: &'static str,
}
```

The resolver must compare the request's session with the active session before producing playback.

This is a general principle:

> Bevy transports typed data. Your domain model must still encode authority and staleness.

## 13. Plugins are composition contracts

A plugin configures an `App`:

```rust
use bevy::prelude::*;

#[derive(Resource, Default)]
struct Score(u32);

#[derive(Message)]
struct AddScore(u32);

fn apply_score(
    mut messages: MessageReader<AddScore>,
    mut score: ResMut<Score>,
) {
    for AddScore(amount) in messages.read() {
        score.0 += amount;
    }
}

struct ScorePlugin;

impl Plugin for ScorePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Score>()
            .add_message::<AddScore>()
            .add_systems(Update, apply_score);
    }
}
```

A plugin should usually establish all contracts required by its systems:

- resources;
- messages;
- assets and loaders;
- schedules and sets;
- systems;
- subordinate plugins.

A missing-resource panic often means this composition contract is incomplete.

### Test the plugin independently

```rust
#[test]
fn score_plugin_is_self_contained() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, ScorePlugin));

    app.world_mut().write_message(AddScore(7));
    app.update();

    assert_eq!(app.world().resource::<Score>().0, 7);
}
```

This is more valuable than testing that the source file contains `init_resource::<Score>()`.

## 14. Plugin groups are profiles, not inheritance

A `PluginGroup` selects and configures a set of plugins:

```rust
use bevy::app::{PluginGroup, PluginGroupBuilder};
use bevy::prelude::*;

struct GamePlugins;

impl PluginGroup for GamePlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(ScorePlugin)
            .add(MovementPlugin)
    }
}
```

Useful profiles include:

- minimal headless simulation;
- visible presentation;
- shipping desktop app;
- web app;
- recording audio tests;
- provider standalone host;
- shared title host.

Do not assume every plugin belongs in every profile. A no-window app should not install render extraction or physical audio plugins merely because the production app does.

## 15. Schedules are temporal domains

Systems run in schedules. Common schedules include:

- `Startup` — once before ordinary updates;
- `PreUpdate` — early per-frame work;
- `Update` — general per-frame logic;
- `PostUpdate` — late frame logic and common transform propagation;
- `FixedUpdate` — fixed-time work;
- custom schedules defined by the application.

Adding a system:

```rust
app.add_systems(Update, update_ui);
app.add_systems(FixedUpdate, simulate_body);
```

A schedule is not merely a convenient list. It identifies a time domain and consistency contract.

In Ambition:

- raw input and shell navigation are frame-driven;
- deterministic gameplay advances through the configured simulation schedule;
- presentation reads simulation results later;
- cleanup happens in explicit lifecycle order;
- loading readiness and route authorization occur outside fixed simulation.

## 16. System sets express partial ordering

A `SystemSet` gives a stable phase name:

```rust
use bevy::prelude::*;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum GameSet {
    Input,
    Simulation,
    Presentation,
}

fn main() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .configure_sets(
            Update,
            (
                GameSet::Input,
                GameSet::Simulation.after(GameSet::Input),
                GameSet::Presentation.after(GameSet::Simulation),
            ),
        )
        .add_systems(Update, read_input.in_set(GameSet::Input))
        .add_systems(Update, simulate.in_set(GameSet::Simulation))
        .add_systems(Update, present.in_set(GameSet::Presentation));
}
```

Sets are preferable to a giant chain because they express only required ordering and preserve parallelism within phases.

Use exact `.before(function)` or `.after(function)` relationships sparingly. Function names are more fragile architectural anchors than phase names.

## 17. `.chain()` is stronger than ordinary ordering

This:

```rust
app.add_systems(Update, (a, b, c).chain());
```

means strict sequential order. It may also create deferred-command synchronization boundaries.

Use it when each step genuinely depends on the previous step's writes or commands.

Do not chain a large subsystem simply because tests become deterministic. Over-chaining:

- removes scheduler parallelism;
- hides which dependencies are real;
- makes unrelated work wait;
- encourages one enormous temporal pipeline.

A better process:

1. identify the specific data dependency;
2. assign systems to coherent sets;
3. order the sets;
4. chain only the small transaction that requires exact sequencing.

## 18. Run conditions gate coherent system graphs

A run condition decides whether a system or set should run:

```rust
use bevy::prelude::*;

#[derive(Resource, Default)]
struct Paused(bool);

fn not_paused(paused: Res<Paused>) -> bool {
    !paused.0
}

fn main() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<Paused>()
        .add_systems(Update, simulate.run_if(not_paused));
}
```

Useful lifecycle gates:

- an active gameplay session exists;
- a frontend route is active;
- a loading transaction is visible;
- a resource has been published;
- an asset is ready;
- a game mode permits input;
- a window or device exists.

Prefer one clear gate on a coherent graph over making every required parameter `Option<Res<T>>` and silently returning.

## 19. `Option<Res<T>>` has several different meanings

This parameter:

```rust
fn system(value: Option<Res<Something>>) {}
```

may mean:

1. the plugin is intentionally optional in this app profile;
2. the resource is created later in a lifecycle;
3. the device/platform capability may not exist;
4. the system is hiding a broken plugin contract.

The syntax does not tell you which meaning is intended.

Good example: keyboard input may be absent in a headless host.

Questionable example: a gameplay system requires `RoomGeometry`, but the plugin forgot to initialize it. Making the parameter optional converts an architecture bug into a silent no-op.

When reviewing `Option<Res<T>>`, ask:

- Should this system exist in an app without `T`?
- Is absence a stable supported composition or a transient frame?
- What run condition expresses that lifecycle more clearly?
- Who owns initialization?

## 20. Change detection is observation, not mutation history

Bevy tracks component and resource change ticks.

```rust
fn changed_positions(query: Query<&Position, Changed<Position>>) {
    for position in &query {
        println!("changed to {:?}", position.0);
    }
}
```

Resource checks:

```rust
fn watch_settings(settings: Res<Settings>) {
    if settings.is_changed() {
        println!("settings changed");
    }
}
```

Important subtleties:

- obtaining mutable access may mark a value changed even if the semantic value remains equal;
- change detection is relative to the observing system's last run;
- a system that does not run for several frames observes accumulated change;
- change ticks are not an authoritative gameplay event log;
- snapshot/rollback logic must not rely on incidental Bevy change flags as complete history.

Use messages for meaningful one-shot domain events. Use change detection to avoid unnecessary synchronization or presentation work.

## 21. `Local<T>` is private persistent system state

`Local<T>` belongs to one system instance:

```rust
fn every_third_call(mut calls: Local<u32>) {
    *calls += 1;
    if *calls % 3 == 0 {
        println!("third call");
    }
}
```

Use it for:

- edge-detection latches;
- cached query/system-local bookkeeping;
- presentation throttling;
- small state that no other system should access.

Do not use `Local<T>` for shared domain state. Tests and other systems cannot inspect it directly, and lifecycle ownership may become obscure.

Ambition's analog shell input latch is a good example: each presentation consumer needs private hysteresis state, not a process-global controller singleton.

## 22. Fixed time and frame time are different clocks

A visible application renders at variable frame intervals. Deterministic movement usually needs a stable simulation step.

Bevy's fixed-time machinery accumulates frame time and runs fixed work zero, one, or multiple times as required.

Consequences:

- the first frame may not run a fixed tick;
- one `app.update()` does not guarantee one simulation step;
- a long frame may run several fixed steps;
- presentation can run when no simulation tick occurred;
- tests should measure simulation ticks, not assume exact frame counts.

A manually stepped test may use:

```rust
use std::time::Duration;
use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;

fn main() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .insert_resource(TimeUpdateStrategy::ManualDuration(
            Duration::from_secs_f64(1.0 / 60.0),
        ));

    app.update();
}
```

Read [`../concepts/input-and-game-modes.md`](../concepts/input-and-game-modes.md) before changing Ambition's simulation scheduling.

## 23. Separate input sampling from deterministic control

Raw input is device- and frame-oriented:

- key presses;
- controller buttons;
- analog axes;
- touch events.

Simulation should usually consume a normalized frame or command:

```rust
#[derive(Resource, Default, Clone, Copy)]
struct ControlFrame {
    horizontal: f32,
    jump_pressed: bool,
}
```

The pipeline is:

```text
device input
→ action interpretation
→ edge/hysteresis processing
→ normalized control frame
→ fixed simulation
```

This keeps physical devices and frame timing outside deterministic movement.

### Standalone input injection

```rust
use bevy::input::ButtonInput;
use bevy::prelude::*;

fn read_confirm(keys: Res<ButtonInput<KeyCode>>) {
    if keys.just_pressed(KeyCode::Enter) {
        println!("confirm");
    }
}

fn main() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<ButtonInput<KeyCode>>()
        .add_systems(Update, read_confirm);

    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Enter);
    app.update();

    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .reset(KeyCode::Enter);
}
```

`clear()` and `reset()` are not equivalent. A test simulating repeated taps must release/reset the held state.

## 24. States are useful, but not every lifecycle should use Bevy `State`

Bevy has state-machine facilities for coarse application states. They can be useful for:

- loading versus playing;
- menu versus game;
- editor modes;
- one active high-level mode.

But a complex shared host may need richer identities:

- exact route identity;
- activation ID;
- provider ID;
- load transaction;
- one-shot authorization;
- session scope;
- delayed stale-work rejection.

A single enum state may not encode all of that authority.

Ambition uses explicit shell resources, messages, and exact IDs for its provider lifecycle. Do not replace a rich transaction model with a simplistic `State<GameState>` merely because Bevy provides it.

Use Bevy state when the domain really is one closed mutually exclusive mode. Use typed resources/messages/entities when identity and concurrent lifecycle facts matter.

## 25. Entity ownership and cleanup should be explicit

Temporary entities need an owner:

- gameplay session;
- loading transaction;
- frontend route;
- cutscene;
- encounter;
- application lifetime.

A minimal session-scoped model:

```rust
use bevy::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SessionId(u64);

#[derive(Component)]
struct SessionOwned(SessionId);

fn retire_session(
    mut commands: Commands,
    owned: Query<(Entity, &SessionOwned)>,
    retiring: Res<RetiringSession>,
) {
    for (entity, owner) in &owned {
        if owner.0 == retiring.0 {
            commands.entity(entity).despawn();
        }
    }
}
```

Ambition centralizes this pattern in lifecycle helpers so providers do not hand-roll cleanup.

Ownership markers are valuable because they make cleanup data-driven. A route transition does not need to remember every HUD, camera, particle, or room visual entity by name.

## 26. Hierarchy and transforms are presentation structures

A Bevy hierarchy expresses parent/child relationships. Transforms are often propagated through that hierarchy.

```rust
fn spawn_ship(mut commands: Commands) {
    commands
        .spawn((
            Name::new("Ship"),
            Transform::default(),
            GlobalTransform::default(),
        ))
        .with_children(|ship| {
            ship.spawn((
                Name::new("Turret"),
                Transform::from_xyz(1.0, 0.0, 0.0),
                GlobalTransform::default(),
            ));
        });
}
```

Use hierarchy for presentation and ownership where parent-relative transforms or recursive cleanup make sense.

Do not make simulation truth depend on a sprite hierarchy. A body position should not be discovered by reading a child visual's transform when a canonical simulation component exists.

Understand the difference:

- `Transform` — local relative transform;
- `GlobalTransform` — propagated world-space transform;
- canonical simulation position — game-state fact that may use another type entirely.

## 27. Assets are handles plus asynchronous storage

Loading an asset usually returns a handle immediately:

```rust
fn load(mut commands: Commands, asset_server: Res<AssetServer>) {
    let texture: Handle<Image> = asset_server.load("sprites/player.png");
    commands.spawn((Name::new("Texture owner"), texture));
}
```

The handle is not proof that bytes were found or decoded.

Questions to distinguish:

1. Was the path resolved by the intended `AssetSource`?
2. Was a loader registered for the extension/type?
3. Has the asset finished loading?
4. Is the handle strong enough to keep it resident?
5. Is a rendering/device backend installed?
6. Does this app profile intentionally avoid presentation assets?

Ambition uses multiple asset roots and provider-owned content. Read:

- [`../concepts/asset-management.md`](../concepts/asset-management.md)
- [`../systems/asset-manager.md`](../systems/asset-manager.md)
- [`../concepts/generated-assets-audio.md`](../concepts/generated-assets-audio.md)

When you see `Path not found`, trace the asset source before copying files into another crate.

## 28. Asset sources and loaders are part of architecture

Bevy lets applications register named asset sources such as:

```text
assets://...
game://...
embedded://...
```

A provider-owned source can use one root, while shared generated assets use another. A custom reader may implement a deliberate fallback without duplicating files.

An `AssetLoader` turns bytes into typed assets. It should own parsing and dependencies, not gameplay behavior.

A useful division:

- asset source: where bytes come from;
- loader: how bytes become a typed asset;
- catalog: which authored identity selects which asset;
- preparation: whether required assets are ready;
- presentation: what entities use handles;
- simulation: should usually not depend on GPU objects.

## 29. Rendering is a second world and pipeline

Visible Bevy applications commonly have:

- main world: simulation, app state, presentation intent;
- render world: extracted data prepared for GPU work.

Render plugins register extraction, preparation, queuing, and draw systems.

This matters when constructing tests. A no-window app may still include some rendering types but intentionally omit the `RenderApp`. Installing render-only extraction plugins then produces warnings or errors.

Test profiles should choose deliberately:

- pure headless ECS: no render app;
- no-window presentation: maybe asset and transform systems, but no GPU backend;
- software/recording backend where supported;
- full visible GPU app for manual acceptance.

Do not assume “no window” automatically means “no renderer,” “no GPU,” or “no audio device.” Configure each boundary explicitly.

## 30. Audio has the same device-boundary problem

An automated test may need to exercise:

- provider-relative cue resolution;
- session authority;
- stale-request rejection;
- accepted/rejected playback counters;
- load readiness.

It should not necessarily open speakers.

Use an output abstraction:

```text
resolved audio command
├── real device sink
└── recording sink
```

The recording sink preserves the real resolver and ownership logic while recording what would have played.

Muting after opening a real backend is weaker:

- device initialization still occurs;
- platform errors remain possible;
- accidental sound may escape before mute applies;
- tests are not portable to machines without an audio device.

The same principle applies to windows and GPUs: test the contract below the physical device unless literal device behavior is the subject of the test.

## 31. `SystemParam` bundles organize coherent authority

A custom `SystemParam` groups related resources and queries:

```rust
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

#[derive(Resource)]
struct Settings;

#[derive(Component)]
struct Player;

#[derive(SystemParam)]
struct PlayerAccess<'w, 's> {
    settings: Res<'w, Settings>,
    players: Query<'w, 's, Entity, With<Player>>,
}

fn system(access: PlayerAccess) {
    println!("{} player(s)", access.players.iter().count());
}
```

Good bundles:

- represent one coherent transaction or authority;
- have a narrow domain name;
- expose a small method such as `build`, `prepare`, or `publish`;
- reduce signatures without hiding unrelated dependencies;
- avoid duplicate mutable borrows through nested bundles.

Bad bundles are god objects that grant every system access to most of the world.

When a bundle fails validation, expand it mentally and find the missing or conflicting parameter.

## 32. Observers and hooks are useful, but scheduling remains important

Bevy supports reactive mechanisms such as component hooks and observers. They can reduce explicit polling for lifecycle events.

Use them when the behavior is naturally tied to:

- component insertion/removal;
- entity lifecycle;
- a typed trigger;
- a small local reaction.

Do not hide large gameplay transactions across many observers. Explicit schedules and messages are often easier to trace for:

- load authorization;
- provider activation;
- exact session retirement;
- fixed simulation phases;
- rollback-sensitive state.

A useful test is whether you can draw the temporal order without reading ten disconnected callbacks.

## 33. Ambiguity and conflicting-access errors

Two major Bevy failure classes are different.

### Runtime parameter validation

```text
Parameter failed validation: Resource does not exist
```

Likely causes:

- plugin failed to initialize a required resource;
- system runs before lifecycle publication;
- app profile omitted a required plugin;
- a resource was retired but the system still runs;
- a custom `SystemParam` contains the missing dependency.

### System ambiguity or access conflict

Two systems may access data incompatibly without ordering, or one system may declare overlapping mutable access.

Resolve by:

- narrowing queries;
- splitting authoritative data appropriately;
- adding a real ordering edge;
- using sets;
- using `ParamSet` when sequential access is truly required.

Do not silence diagnostics before understanding whether they reveal a real race or merely a deliberately disjoint app profile.

## 34. Debug a missing-resource panic systematically

Suppose a test says:

```text
Resource does not exist
```

Use this method:

1. Run the exact test with `RUST_BACKTRACE=full` and `--nocapture`.
2. Enable enough debug information to identify the failing system.
3. Find every `Res<T>` and custom `SystemParam` in that system.
4. Find which plugin initializes each resource.
5. Verify that the test composes that plugin.
6. Verify that the system is gated until lifecycle-created resources exist.
7. Decide whether absence is legitimate or a broken contract.
8. Fix initialization or gating; do not blindly wrap the parameter in `Option`.

Useful searches:

```bash
rg -n "struct SceneEntities|Res<SceneEntities>|init_resource::<SceneEntities>" crates game
rg -n "impl Plugin for|add_plugins" path/to/subsystem
rg -n "run_if|in_set|before|after" path/to/subsystem
```

## 35. Small Bevy tests are real applications

A good unit/integration test creates the smallest real app that owns the behavior:

```rust
#[test]
fn movement_advances_position() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_systems(Update, integrate);

    let entity = app
        .world_mut()
        .spawn((Position(Vec2::ZERO), Velocity(Vec2::X)))
        .id();

    app.update();

    assert_eq!(
        app.world().entity(entity).get::<Position>().unwrap().0,
        Vec2::X,
    );
}
```

This is preferable to mocking a scheduler or testing that a function name appears in a plugin source file.

Test levels:

1. pure function;
2. minimal ECS app;
3. subsystem plugin app;
4. provider/standalone host;
5. shared shipping host;
6. manual device acceptance.

Choose the lowest level that can observe the contract.

## 36. Wait for observable conditions, not magic frame counts

Bevy commands, asset loading, messages, fixed time, and lifecycle staging make exact-frame assumptions brittle.

A helper can wait within a strict budget:

```rust
fn step_until(
    app: &mut App,
    budget: usize,
    mut condition: impl FnMut(&mut App) -> bool,
) {
    for _ in 0..budget {
        app.update();
        if condition(app) {
            return;
        }
    }
    panic!("condition did not become true within {budget} updates");
}
```

The condition must be the real contract:

- route became active;
- player exists;
- load barrier is ready;
- exact session retired;
- no session-owned entities remain.

Do not use a huge loop to hide an ordering defect. The budget should be small and explained by the lifecycle.

## 37. Test cleanup by exact ownership

A useful lifecycle test:

```text
launch session A
→ session-owned player/HUD/camera exist
return home
→ no A-owned entities remain
launch session B
→ no entity reports A's ownership
```

Do not merely count total entities. Inspect exact markers and authority resources.

This catches:

- process-resident gameplay UI;
- stale cameras;
- unretired audio owners;
- old room geometry;
- delayed commands targeting a new session;
- provider state leaking across relaunch.

But keep these tests behavioral. Avoid turning every marker into a permanent source-text inventory.

## 38. The Ambition simulation/presentation seam

Read [`../concepts/sim-presentation-seam.md`](../concepts/sim-presentation-seam.md).

Simulation owns facts such as:

- position and velocity;
- body mode;
- health;
- active room;
- encounters;
- deterministic inputs;
- collision results.

Presentation owns consequences such as:

- sprites;
- animation frames;
- HUD entities;
- cameras;
- particles;
- audio playback;
- window/UI state.

Bevy makes it easy to put all of these in one world. That convenience does not make them the same authority.

A common pattern:

```text
fixed simulation updates canonical components
→ simulation emits typed result/message
→ frame presentation observes result
→ presentation creates or updates visual/audio entities
```

Headless tests should be able to run simulation without GPU or speakers.

## 39. The Ambition session-world model

Read:

- [`../../crates/ambition_game_shell/src/session.rs`](../../crates/ambition_game_shell/src/session.rs)
- [`../../crates/ambition_runtime/src/session_world.rs`](../../crates/ambition_runtime/src/session_world.rs)
- [`../../crates/ambition/src/session_world.rs`](../../crates/ambition/src/session_world.rs)
- [`../../crates/ambition_platformer_primitives/src/lifecycle/`](../../crates/ambition_platformer_primitives/src/lifecycle/)

The shared host cannot use one process-global gameplay world because it launches several providers and may relaunch the same provider.

The model is:

```text
route request
→ exact activation ID
→ exact session scope
→ canonical world entity
→ session-owned gameplay/presentation entities
→ exact retirement
→ frontend zero state
```

Important Bevy mechanisms:

- resources for current authority;
- entities for live world/session roots;
- components for scope ownership;
- messages for lifecycle transitions;
- commands for deferred spawn/despawn;
- run conditions for frontend/gameplay graphs;
- queries for exact cleanup and tests.

The title route should not merely ignore gameplay state. It should structurally have no active gameplay-world owner.

## 40. Providers are plugins plus lifecycle participation

Read:

- [`../../crates/ambition_platformer_provider/src/lib.rs`](../../crates/ambition_platformer_provider/src/lib.rs)
- [`../../game/ambition_content/src/provider.rs`](../../game/ambition_content/src/provider.rs)
- provider files in `game/ambition_demo_sanic`, `game/ambition_demo_mary_o`, and `game/ambition_demo_pocket`.

A provider contributes:

- experience registration;
- authored catalog fragments;
- a session-world source registered through the shared preparation/activation lifecycle;
- gameplay and presentation plugins;
- session-scoped entity construction;
- teardown participation.

The host chooses which provider plugins to link. It should not contain a growing match statement that knows each provider's internals.

Bevy plugin composition is what makes this possible: each provider modifies the same `App` through typed registrations while remaining App-local.

## 41. Loading is a transaction, not a screen

Read:

- [`../../crates/ambition_load/src/`](../../crates/ambition_load/src/)
- [`../../crates/ambition_load_presentation/src/`](../../crates/ambition_load_presentation/src/)
- [`../../crates/ambition_game_shell/src/preparation.rs`](../../crates/ambition_game_shell/src/preparation.rs)

The lifecycle is:

```text
route requested
→ fresh load ID
→ provider declares required/streamable/speculative work
→ work reports progress/failure
→ immutable prepared session is published
→ one-shot authorization is consumed
→ gameplay session activates
```

Bevy roles:

- resources store transaction models;
- messages carry work updates and actions;
- systems reconcile readiness;
- frontend entities present progress;
- exact IDs reject stale completion;
- session construction moves prepared data into live state.

A loading screen that merely waits a timer is presentation. A load transaction is authoritative lifecycle state.

## 42. The movement kernel demonstrates Bevy's proper boundary

Read:

- [`../concepts/movement-collision.md`](../concepts/movement-collision.md)
- [`../adr/0024-frame-aware-unified-movement-kernel.md`](../adr/0024-frame-aware-unified-movement-kernel.md)
- [`../../crates/ambition_engine_core/src/movement/`](../../crates/ambition_engine_core/src/movement/)

Bevy should organize bodies and schedule stepping, but the core movement algorithm can remain ordinary typed Rust.

A useful shape:

```text
Bevy query selects bodies and context
→ system builds focused movement inputs
→ pure/shared movement kernel steps one body
→ system writes canonical body result
→ presentation later observes result
```

Do not put every algorithm inside ECS access code. Pure Rust functions are easier to test, reason about, and reuse.

Bevy is the orchestration layer; it need not absorb all domain logic.

## 43. Learn Bevy by following registration

Do not read a large crate from top to bottom.

For any feature:

1. Find the component/resource/message type.
2. Find the plugin that registers it.
3. Find every system that reads or mutates it.
4. Find its schedule and set.
5. Find run conditions.
6. Find spawn/despawn ownership.
7. Find the smallest test app.
8. Find which app profiles include the plugin.

Useful searches:

```bash
rg -n "impl Plugin for" crates game
rg -n "init_resource|insert_resource|add_message" crates game
rg -n "add_systems|configure_sets|in_set|run_if" crates game
rg -n "Query<|Res<|ResMut<|MessageReader<|MessageWriter<" path/to/crate
rg -n "spawn_session_scoped|SessionScopedEntity" crates game
```

This method follows Bevy's actual dependency graph.

## 44. Standalone capstone: a tiny session-owned host

This example combines plugins, resources, messages, components, commands, queries, and exact cleanup without depending on Ambition.

```rust
use bevy::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SessionId(u64);

#[derive(Resource, Default)]
struct ActiveSession(Option<SessionId>);

#[derive(Resource, Default)]
struct NextSessionId(u64);

#[derive(Component, Debug)]
struct SessionOwned(SessionId);

#[derive(Component, Debug)]
struct ProviderName(&'static str);

#[derive(Component, Debug)]
struct Player;

#[derive(Message, Debug)]
enum HostCommand {
    Launch(&'static str),
    ReturnHome,
}

fn apply_host_commands(
    mut commands: Commands,
    mut messages: MessageReader<HostCommand>,
    mut active: ResMut<ActiveSession>,
    mut next_id: ResMut<NextSessionId>,
    owned: Query<(Entity, &SessionOwned)>,
) {
    for message in messages.read() {
        match *message {
            HostCommand::Launch(provider) => {
                // Retire every entity owned by the previous exact session.
                if let Some(old_session) = active.0 {
                    for (entity, owner) in &owned {
                        if owner.0 == old_session {
                            commands.entity(entity).despawn();
                        }
                    }
                }

                let session = SessionId(next_id.0);
                next_id.0 += 1;
                active.0 = Some(session);

                commands.spawn((
                    SessionOwned(session),
                    ProviderName(provider),
                    Player,
                    Name::new(format!("{provider} player")),
                ));
            }
            HostCommand::ReturnHome => {
                let Some(old_session) = active.0.take() else {
                    continue;
                };
                for (entity, owner) in &owned {
                    if owner.0 == old_session {
                        commands.entity(entity).despawn();
                    }
                }
            }
        }
    }
}

struct HostPlugin;

impl Plugin for HostPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ActiveSession>()
            .init_resource::<NextSessionId>()
            .add_message::<HostCommand>()
            .add_systems(Update, apply_host_commands);
    }
}

fn main() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, HostPlugin));

    app.world_mut().write_message(HostCommand::Launch("sanic"));
    app.update();

    let first = app.world().resource::<ActiveSession>().0.unwrap();

    app.world_mut().write_message(HostCommand::ReturnHome);
    app.update();
    assert_eq!(app.world().resource::<ActiveSession>().0, None);

    let mut players = app
        .world_mut()
        .query_filtered::<&SessionOwned, With<Player>>();
    assert_eq!(players.iter(app.world()).count(), 0);

    app.world_mut().write_message(HostCommand::Launch("sanic"));
    app.update();

    let second = app.world().resource::<ActiveSession>().0.unwrap();
    assert_ne!(first, second);
}
```

Study questions:

- Why is `ActiveSession` a resource but `SessionOwned` a component?
- Why does the command system need `Commands` rather than direct world mutation?
- Why is the same provider allowed to receive a new `SessionId`?
- What happens if Launch and ReturnHome messages appear in one frame?
- Which exact ordering policy should define that behavior?
- How would you reject a stale provider message carrying `first` after `second` launches?
- How would you separate frontend-owned entities from session-owned entities?

Extend it:

1. Add a frontend launcher entity that survives gameplay retirement.
2. Add `OwnedScoreMessage { session, amount }` and reject stale sessions.
3. Add a loading phase with a fresh `LoadId`.
4. Add a `SystemSet` for route commands, activation, and cleanup.
5. Add tests for launch, home, relaunch, and cross-session rejection.

## 45. A twelve-module Bevy learning path

### Module 1: App and World

Build the minimal counter app.

Deliverable: explain the difference between modifying the app during composition and running a system during update.

### Module 2: entities and components

Build player, enemy, and decoration archetypes.

Deliverable: predict and verify five query results.

### Module 3: resources and App-local state

Create two apps with independent resources.

Deliverable: prove no process-global selection leaks between them.

### Module 4: systems and queries

Implement movement and damage systems.

Deliverable: explain every borrow in the system signatures.

### Module 5: deferred commands

Spawn and despawn entities across ordered systems.

Deliverable: draw where deferred changes become visible.

### Module 6: messages

Implement typed damage and route commands.

Deliverable: distinguish persistent state from transient requests.

### Module 7: plugins

Package one subsystem as a self-contained plugin.

Deliverable: a minimal plugin test with no hidden setup.

### Module 8: schedules and sets

Create input, simulation, presentation, and cleanup phases.

Deliverable: a partial-order diagram and an explanation of every ordering edge.

### Module 9: fixed time and input

Sample keyboard input into a normalized control resource and consume it from fixed simulation.

Deliverable: a test that measures simulation ticks instead of frame count.

### Module 10: ownership and cleanup

Complete the tiny session-owned host.

Deliverable: launch, retire, relaunch, and stale-message tests.

### Module 11: assets and presentation profiles

Create a visible profile and a headless profile.

Deliverable: explain which plugins belong in each and why the headless profile does not initialize devices.

### Module 12: trace Ambition

Trace one provider from registration through preparation, activation, fixed simulation, presentation, Quit-to-Home, and exact cleanup.

Deliverable: a diagram naming the actual resources, messages, sets, and entity ownership markers.

## 46. Common Bevy mistakes in this repository class

### Treating `Entity` as a permanent reference

An entity may be despawned. Always verify existence and expected components.

### Making required resources optional to silence a panic

Fix plugin composition or lifecycle gating first.

### Assuming one update means one fixed tick

Measure the simulation tick or wait for an observable state.

### Spawning gameplay UI without session ownership

It will survive return-to-title unless cleanup happens accidentally.

### Using process globals for provider registries

Separate tests/apps then influence each other.

### Letting presentation become authoritative

Sprites and transforms should not determine canonical gameplay facts.

### Opening devices in automated tests

Use recording/inert backends for window, audio, and GPU boundaries.

### Overusing `.chain()`

It serializes work and conceals which dependency matters.

### Using messages without authority identity

Stale sessions can affect current state.

### Copying live state into several resources

You create multiple mutable sources of truth.

### Testing source spelling instead of behavior

Prefer a small real `App` that demonstrates the contract.

## 47. What to memorize and what to look up

Memorize:

- entity versus component versus resource;
- systems declare access through parameters;
- queries select component shapes;
- commands are deferred;
- plugins configure apps;
- schedules are temporal domains;
- sets express partial ordering;
- messages are transient and do not imply authority;
- fixed time differs from frame time;
- `Handle<T>` is not asset readiness;
- headless/no-window/no-device are separate choices;
- exact ownership markers make cleanup reliable.

Look up:

- exact filter syntax;
- less-common `SystemParam` types;
- observer APIs;
- render extraction stages;
- asset-loader traits;
- platform backend configuration;
- feature names;
- exact Bevy 0.18.1 method names.

The skill is not memorizing all of Bevy. It is knowing where execution, authority, and lifecycle are represented.

## 48. Practical pre-edit checklist

Before editing a Bevy subsystem:

- Which plugin owns it?
- Which app profiles include that plugin?
- Which world contains the data?
- Is the data a component, resource, message, or asset?
- Which systems read and mutate it?
- Which schedule and set run them?
- Which run condition gates the lifecycle?
- Are structural changes deferred?
- Which entity/session/load/frontend owner cleans it up?
- Can the focused behavior run without a physical device?

While editing:

- keep system access narrow;
- use exact typed identity for stale-work boundaries;
- preserve simulation/presentation separation;
- initialize required resources in the owning plugin;
- prefer coherent sets over long exact chains;
- do not turn lifecycle bugs into silent optional parameters;
- do not create process globals;
- do not bypass canonical movement or session authority.

After editing:

```bash
cargo check -p THE_TOUCHED_CRATE
cargo test -p THE_TOUCHED_CRATE THE_FOCUSED_TEST
```

Then run the smallest provider or shared-host integration test that observes the cross-crate behavior.

## 49. Recommended project reading

Concepts and systems:

- [`../concepts/engine-mental-model.md`](../concepts/engine-mental-model.md)
- [`../concepts/bevy-native-data-driven-ecs.md`](../concepts/bevy-native-data-driven-ecs.md)
- [`../concepts/sim-presentation-seam.md`](../concepts/sim-presentation-seam.md)
- [`../concepts/input-and-game-modes.md`](../concepts/input-and-game-modes.md)
- [`../concepts/asset-management.md`](../concepts/asset-management.md)
- [`../concepts/testing-and-validation.md`](../concepts/testing-and-validation.md)
- [`../concepts/input-and-game-modes.md`](../concepts/input-and-game-modes.md)
- [`../adr/0002-engine-must-be-bevy-native.md`](../adr/0002-engine-must-be-bevy-native.md)
- [`../adr/0012-sim-presentation-split-and-events-refactor.md`](../adr/0012-sim-presentation-split-and-events-refactor.md)
- [`../adr/0019-pluginized-platformer-runtime.md`](../adr/0019-pluginized-platformer-runtime.md)
- [`../adr/0024-frame-aware-unified-movement-kernel.md`](../adr/0024-frame-aware-unified-movement-kernel.md)

Source entrypoints:

- [`../../crates/ambition_game_shell/src/lib.rs`](../../crates/ambition_game_shell/src/lib.rs)
- [`../../crates/ambition_game_shell/src/session.rs`](../../crates/ambition_game_shell/src/session.rs)
- [`../../crates/ambition_game_shell/src/router.rs`](../../crates/ambition_game_shell/src/router.rs)
- [`../../crates/ambition_load/src/coordinator.rs`](../../crates/ambition_load/src/coordinator.rs)
- [`../../crates/ambition_platformer_provider/src/lib.rs`](../../crates/ambition_platformer_provider/src/lib.rs)
- [`../../crates/ambition/src/session_world.rs`](../../crates/ambition/src/session_world.rs)
- [`../../crates/ambition_runtime/src/session_world.rs`](../../crates/ambition_runtime/src/session_world.rs)
- [`../../crates/ambition_platformer_primitives/src/lifecycle/`](../../crates/ambition_platformer_primitives/src/lifecycle/)
- [`../../crates/ambition_platformer_primitives/src/schedule.rs`](../../crates/ambition_platformer_primitives/src/schedule.rs)
- [`../../crates/ambition_actors/src/schedule/`](../../crates/ambition_actors/src/schedule/)
- [`../../crates/ambition_render/src/lib.rs`](../../crates/ambition_render/src/lib.rs)
- [`../../game/ambition_content/src/provider.rs`](../../game/ambition_content/src/provider.rs)
- [`../../game/ambition_app/src/app/`](../../game/ambition_app/src/app/)

External references to keep locally bookmarked:

- the official Bevy migration guides for the pinned release;
- docs.rs for Bevy 0.18.1;
- Bevy's official examples at the matching version;
- the Rust course beside this document;
- `Cargo.lock` when examples disagree about API versions.

## 50. Graduation standard

You know enough Bevy to work independently on Ambition when you can do all of the following without an LLM:

- construct a minimal `App` for a subsystem;
- identify the plugin contract behind a resource or system;
- choose correctly among component, resource, message, asset, and local state;
- write queries with deliberate cardinality and disjoint access;
- explain when commands become visible;
- place systems in the correct schedule and set;
- distinguish frame time from fixed simulation time;
- inject input correctly in tests;
- trace a message through ownership validation and consumption;
- build separate visible and headless/no-device app profiles;
- debug a missing-resource panic through composition and lifecycle;
- trace a provider from registration to exact teardown;
- prove that a session-owned entity disappears without relying on total entity counts;
- keep simulation canonical while presentation remains derived;
- make a contained change and select the smallest meaningful Bevy test.

At that point, unfamiliar Bevy APIs are lookup problems. The engine's execution model is no longer mysterious, and the repository can be maintained without depending on generated code or conversational memory.
