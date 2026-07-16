# How to learn enough Rust to understand how to code in this game

This is an advanced, project-specific Rust course for working on Ambition without depending on an LLM.

It assumes:

- you already know Python well;
- you have seen basic Rust syntax but want a systematic, advanced refresher;
- you can run `cargo check` and read a compiler error;
- you want to understand this repository well enough to make deliberate changes, not merely copy patterns until they compile.

The goal is not to make you a general Rust expert. The goal is to make the important parts of Ambition legible:

- a large Cargo workspace;
- Bevy ECS and plugin composition;
- ownership and exact lifecycle scope;
- fixed-step simulation;
- the simulation/presentation boundary;
- provider-authored games inside a shared title host;
- load transactions and prepared sessions;
- Rust, RON, and LDtk as different authoring layers;
- tests that construct small Bevy apps and inspect real state.

This course combines self-contained language examples with reading and modifying the actual code. Rust becomes much easier when syntax, ownership, and patterns are first understood in isolation and then attached to a concrete architectural purpose.

## 1. The central translation from Python to Rust

Python lets you begin with objects and discover their contracts at runtime. Rust asks you to decide more of the contract before the program runs.

The most useful mental translation is:

| Python instinct | Rust/Ambition replacement |
|---|---|
| Pass an object around and trust convention | Pass a value or borrow with an explicit type |
| Mutate shared objects from many places | Give one system or owner explicit mutable authority |
| Use `None` and discover mistakes later | Use `Option<T>` and handle both cases |
| Raise an exception | Return `Result<T, E>` or reject invalid composition at startup |
| Use inheritance or duck typing | Use enums, traits, components, and composition |
| Store heterogeneous state on one object | Split state into ECS components and resources |
| Call methods in an implicit order | Register systems into explicit schedules and sets |
| Keep global registries | Keep App-local resources and plugin-owned registration |
| Use tests to find type mistakes | Let the compiler reject type, ownership, and lifetime mistakes |

Rust is not mainly about memory management in this repository. It is about making authority and dataflow visible.

When Rust says that two systems cannot mutably borrow the same resource, the useful question is not “how do I satisfy the borrow checker?” It is:

> Why do two independent pieces of code believe they own the same state?

When a Bevy system fails because a resource does not exist, the useful question is:

> Which plugin promised this resource, and under what application composition is that promise valid?

Treat compiler errors as architecture questions first and syntax questions second.

## 2. A self-contained Rust syntax and coding-patterns primer

The rest of this course uses Ambition as its laboratory. This section is different: every example is self-contained and can be studied in a scratch crate without opening the game repository.

Create one with:

```bash
cargo new rust-pattern-lab
cd rust-pattern-lab
```

Replace `src/main.rs` with any example below and run:

```bash
cargo run
cargo test
```

The examples are intentionally small enough to type by hand. Typing them is more useful than copying them because Rust's punctuation carries meaning.

### Bindings, mutability, type inference, and shadowing

A binding is immutable unless declared `mut`:

```rust
fn main() {
    let room_name = "intro_lab";
    let mut visit_count = 0;

    visit_count += 1;
    println!("{room_name} has been visited {visit_count} time(s)");
}
```

Rust usually infers types, but an annotation can clarify intent or resolve ambiguity:

```rust
fn main() {
    let lives: u8 = 3;
    let gravity: f32 = 24.0;
    let room_ids: Vec<String> = Vec::new();

    println!("{lives} {gravity} {}", room_ids.len());
}
```

Shadowing creates a new binding with the same name. It can also change type:

```rust
fn main() {
    let input = " 42 ";
    let input = input.trim();
    let input: i32 = input.parse().expect("integer input");

    assert_eq!(input, 42);
}
```

Use shadowing for staged interpretation of one concept. Use `mut` when one value changes over time.

### Expressions, statements, blocks, and semicolons

Rust is expression-oriented. A block evaluates to its final expression when that expression has no semicolon:

```rust
fn clamp_energy(raw: i32) -> i32 {
    if raw < 0 {
        0
    } else if raw > 100 {
        100
    } else {
        raw
    }
}

fn main() {
    let bonus = {
        let base = 10;
        base * 2
    };

    assert_eq!(bonus, 20);
    assert_eq!(clamp_energy(150), 100);
}
```

Adding a semicolon changes an expression into a statement whose value is `()`:

```rust,compile_fail
fn broken() -> i32 {
    5; // evaluates to (), not i32
}
```

This detail explains many return-type errors.

### Scalar types, tuples, arrays, vectors, strings, and slices

Common scalar types:

```rust
fn main() {
    let tick: u64 = 120;
    let health: i32 = -5;
    let speed: f32 = 8.5;
    let grounded: bool = true;
    let grade: char = 'A';

    println!("{tick} {health} {speed} {grounded} {grade}");
}
```

Tuples have fixed length and may contain different types:

```rust
fn main() {
    let spawn = (12.0_f32, 8.0_f32, "intro_lab");
    let (x, y, room) = spawn;
    println!("spawn {room} at ({x}, {y})");
}
```

Arrays have fixed length and one element type:

```rust
fn main() {
    let cardinal = ["north", "east", "south", "west"];
    assert_eq!(cardinal[1], "east");
}
```

`Vec<T>` is a growable owned sequence:

```rust
fn main() {
    let mut rooms = vec!["intro", "hall"];
    rooms.push("roof");

    for room in &rooms {
        println!("{room}");
    }

    assert_eq!(rooms.len(), 3);
}
```

`String` owns mutable UTF-8 text. `&str` borrows a string slice:

```rust
fn announce(label: &str) {
    println!("launching {label}");
}

fn main() {
    let owned = String::from("Sanic");
    announce(&owned);
    announce("Mary-O");
}
```

A slice borrows a contiguous region without owning it:

```rust
fn average(values: &[f32]) -> f32 {
    values.iter().sum::<f32>() / values.len() as f32
}

fn main() {
    let samples = vec![2.0, 4.0, 6.0, 8.0];
    assert_eq!(average(&samples), 5.0);
    assert_eq!(average(&samples[1..3]), 5.0);
}
```

### Functions, methods, associated functions, and constructors

A free function is namespaced by its module:

```rust
fn squared(value: f32) -> f32 {
    value * value
}
```

Methods take `self`, `&self`, or `&mut self`:

```rust
#[derive(Debug)]
struct Energy {
    current: u32,
    maximum: u32,
}

impl Energy {
    fn new(maximum: u32) -> Self {
        Self {
            current: maximum,
            maximum,
        }
    }

    fn fraction(&self) -> f32 {
        self.current as f32 / self.maximum as f32
    }

    fn spend(&mut self, amount: u32) -> bool {
        if amount > self.current {
            return false;
        }
        self.current -= amount;
        true
    }
}

fn main() {
    let mut energy = Energy::new(10);
    assert!(energy.spend(3));
    assert_eq!(energy.current, 7);
    assert_eq!(energy.fraction(), 0.7);
}
```

`Energy::new` is an associated function, not a special language constructor. Rust has no required constructor name. `new` is convention.

### Struct forms and update syntax

Named-field structs:

```rust
#[derive(Debug, Clone, PartialEq)]
struct Body {
    x: f32,
    y: f32,
    health: u32,
}

fn main() {
    let original = Body {
        x: 1.0,
        y: 2.0,
        health: 100,
    };

    let moved = Body {
        x: 8.0,
        ..original.clone()
    };

    assert_eq!(moved.y, 2.0);
    assert_eq!(original.health, 100);
}
```

Tuple structs are useful for newtypes:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct SessionId(u64);

fn retire_session(id: SessionId) {
    println!("retiring session {}", id.0);
}

fn main() {
    retire_session(SessionId(7));
}
```

Unit structs carry type identity without fields:

```rust
struct SanicProvider;
struct MaryOProvider;
```

These are often used as zero-sized marker types in generic APIs.

### Enums carry data and define valid states

An enum variant may contain different data:

```rust
#[derive(Debug, PartialEq)]
enum Route {
    Launcher,
    Loading { destination: String },
    Gameplay { session: u64 },
    Exiting,
}

fn describe(route: &Route) -> String {
    match route {
        Route::Launcher => "launcher".to_owned(),
        Route::Loading { destination } => format!("loading {destination}"),
        Route::Gameplay { session } => format!("gameplay session {session}"),
        Route::Exiting => "exiting".to_owned(),
    }
}

fn main() {
    let route = Route::Loading {
        destination: "sanic".into(),
    };
    assert_eq!(describe(&route), "loading sanic");
}
```

Prefer one enum over several booleans that can express contradictory states.

### Pattern matching and destructuring

Patterns appear in `match`, `if let`, `let`, function parameters, and loops:

```rust
#[derive(Debug)]
enum Command {
    Move { dx: f32, dy: f32 },
    Damage(u32),
    Quit,
}

fn apply(command: Command) {
    match command {
        Command::Move { dx, dy } if dx == 0.0 && dy == 0.0 => {
            println!("ignored zero movement");
        }
        Command::Move { dx, dy } => println!("move by {dx}, {dy}"),
        Command::Damage(amount @ 1..=9) => println!("light damage: {amount}"),
        Command::Damage(amount) => println!("heavy damage: {amount}"),
        Command::Quit => println!("quit"),
    }
}

fn main() {
    apply(Command::Move { dx: 2.0, dy: 0.0 });
    apply(Command::Damage(5));
}
```

`let ... else` is ideal for rejecting the wrong shape early:

```rust
fn destination(command: &Command) -> Option<(f32, f32)> {
    let Command::Move { dx, dy } = command else {
        return None;
    };
    Some((*dx, *dy))
}
```

### Control flow: `if`, `loop`, `while`, and `for`

`if` is an expression:

```rust
let label = if health == 0 { "dead" } else { "alive" };
```

A `loop` may return a value through `break`:

```rust
fn first_power_of_two_at_least(target: u32) -> u32 {
    let mut value = 1;
    loop {
        if value >= target {
            break value;
        }
        value *= 2;
    }
}
```

Use `while` for state-driven repetition and `for` for iteration:

```rust
fn main() {
    let mut countdown = 3;
    while countdown > 0 {
        countdown -= 1;
    }

    for index in 0..4 {
        println!("index {index}");
    }
}
```

Labels disambiguate nested loops:

```rust
fn find(grid: &[Vec<i32>], target: i32) -> Option<(usize, usize)> {
    'rows: for (y, row) in grid.iter().enumerate() {
        for (x, value) in row.iter().enumerate() {
            if *value == target {
                return Some((x, y));
            }
            if *value < 0 {
                continue 'rows;
            }
        }
    }
    None
}
```

### Ownership: moves, copies, clones, and borrows

Types such as integers commonly implement `Copy`:

```rust
fn main() {
    let a = 3_u32;
    let b = a;
    println!("{a} {b}");
}
```

`String` is moved by assignment:

```rust,compile_fail
fn main() {
    let a = String::from("room");
    let b = a;
    println!("{a} {b}"); // a was moved
}
```

Borrow when the callee only needs temporary access:

```rust
fn count_bytes(text: &str) -> usize {
    text.len()
}

fn main() {
    let room = String::from("intro_lab");
    assert_eq!(count_bytes(&room), 9);
    println!("{room}"); // still owned here
}
```

Use a mutable borrow for temporary exclusive mutation:

```rust
fn append_suffix(text: &mut String) {
    text.push_str("_night");
}

fn main() {
    let mut room = String::from("roof");
    append_suffix(&mut room);
    assert_eq!(room, "roof_night");
}
```

Clone only when two independent owned values are semantically required:

```rust
fn main() {
    let original = String::from("provider-a");
    let snapshot = original.clone();
    assert_eq!(original, snapshot);
}
```

### Borrow scopes and non-lexical lifetimes

A mutable borrow ends after its final use, not necessarily at the closing brace:

```rust
fn main() {
    let mut values = vec![1, 2, 3];

    let first = &mut values[0];
    *first += 10;

    values.push(4); // legal: first is no longer used
    assert_eq!(values, vec![11, 2, 3, 4]);
}
```

When borrow errors arise, first shorten the lifetime of references by moving work into a smaller block or by extracting owned values.

### `Option<T>`: explicit absence

A lookup may return no result:

```rust
fn find_room<'a>(rooms: &'a [String], wanted: &str) -> Option<&'a str> {
    rooms
        .iter()
        .find(|room| room.as_str() == wanted)
        .map(String::as_str)
}

fn main() {
    let rooms = vec!["intro".to_owned(), "hall".to_owned()];

    assert_eq!(find_room(&rooms, "hall"), Some("hall"));
    assert_eq!(find_room(&rooms, "roof"), None);
}
```

Common transformations:

```rust
fn main() {
    let maybe_name = Some("Sanic");

    let length = maybe_name.map(str::len);
    let loud = maybe_name.map(|name| name.to_uppercase());
    let fallback = None::<&str>.unwrap_or("Mary-O");

    assert_eq!(length, Some(5));
    assert_eq!(loud.as_deref(), Some("SANIC"));
    assert_eq!(fallback, "Mary-O");
}
```

Use `?` to propagate absence:

```rust
fn first_character(text: Option<&str>) -> Option<char> {
    text?.chars().next()
}
```

### `Result<T, E>`: explicit failure

Define a focused error type:

```rust
#[derive(Debug, PartialEq)]
enum ParseSpeedError {
    Empty,
    NotANumber,
    Negative,
}

fn parse_speed(text: &str) -> Result<f32, ParseSpeedError> {
    let text = text.trim();
    if text.is_empty() {
        return Err(ParseSpeedError::Empty);
    }

    let value: f32 = text.parse().map_err(|_| ParseSpeedError::NotANumber)?;
    if value < 0.0 {
        return Err(ParseSpeedError::Negative);
    }
    Ok(value)
}

fn main() {
    assert_eq!(parse_speed("12.5"), Ok(12.5));
    assert_eq!(parse_speed("-1"), Err(ParseSpeedError::Negative));
}
```

`?` returns early with the error after applying any required conversion.

### Collections: `Vec`, `HashMap`, `BTreeMap`, and sets

Use `Vec<T>` for ordered sequences and stacks:

```rust
fn main() {
    let mut stack = vec!["launcher"];
    stack.push("gameplay");
    assert_eq!(stack.pop(), Some("gameplay"));
}
```

Use `HashMap` for fast key lookup when iteration order is irrelevant:

```rust
use std::collections::HashMap;

fn main() {
    let mut health = HashMap::new();
    health.insert("player", 100_u32);
    health.entry("boss").or_insert(500);
    assert_eq!(health.get("player"), Some(&100));
}
```

Use `BTreeMap` when deterministic sorted iteration matters:

```rust
use std::collections::BTreeMap;

fn main() {
    let mut providers = BTreeMap::new();
    providers.insert("sanic", 2);
    providers.insert("ambition", 1);

    let names: Vec<_> = providers.keys().copied().collect();
    assert_eq!(names, vec!["ambition", "sanic"]);
}
```

Use sets to represent membership, not key/value association.

### Iterators and closures

Closures may borrow, mutate, or consume captured values:

```rust
fn main() {
    let threshold = 10;
    let values = [4, 12, 3, 20];

    let large: Vec<_> = values
        .into_iter()
        .filter(|value| *value >= threshold)
        .map(|value| value * 2)
        .collect();

    assert_eq!(large, vec![24, 40]);
}
```

Useful adapters:

```rust
fn main() {
    let values = [3, 8, 2];

    assert!(values.iter().any(|value| *value > 5));
    assert!(values.iter().all(|value| *value > 0));
    assert_eq!(values.iter().find(|value| **value == 8), Some(&8));
    assert_eq!(values.iter().copied().sum::<i32>(), 13);
    assert_eq!(values.iter().copied().max(), Some(8));
}
```

When types become confusing, insert an intermediate binding with an annotation.

### Traits and generic functions

A trait defines a capability:

```rust
trait HasHealth {
    fn health(&self) -> u32;

    fn is_alive(&self) -> bool {
        self.health() > 0
    }
}

struct Player {
    health: u32,
}

impl HasHealth for Player {
    fn health(&self) -> u32 {
        self.health
    }
}

fn report<T: HasHealth>(value: &T) -> &'static str {
    if value.is_alive() { "alive" } else { "dead" }
}

fn main() {
    assert_eq!(report(&Player { health: 10 }), "alive");
}
```

The equivalent `where` form is clearer with several bounds:

```rust
fn duplicate<T>(value: T) -> (T, T)
where
    T: Clone,
{
    (value.clone(), value)
}
```

Trait objects provide runtime polymorphism:

```rust
trait Rule {
    fn apply(&self, value: i32) -> i32;
}

struct Double;

impl Rule for Double {
    fn apply(&self, value: i32) -> i32 {
        value * 2
    }
}

fn run(rule: &dyn Rule, value: i32) -> i32 {
    rule.apply(value)
}
```

Prefer enums when the set of variants is closed and known. Prefer traits when independent crates should add implementations.

### Derives and manually implemented traits

Derives generate common implementations:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
struct Tick(u64);
```

Important derives:

- `Debug` for diagnostics;
- `Clone` for explicit duplication;
- `Copy` for small values with trivial duplication semantics;
- `PartialEq`/`Eq` for comparisons;
- `Hash` for hash collections;
- `Ord` for sorted collections;
- `Default` for conventional construction.

Do not derive `Copy` merely to silence move errors. It changes the semantic meaning of assignment.

### Modules, visibility, and imports

A small module tree:

```rust
mod movement {
    pub struct Velocity(pub f32);

    pub fn integrate(position: f32, velocity: Velocity, dt: f32) -> f32 {
        position + velocity.0 * dt
    }

    fn internal_helper() {}
}

fn main() {
    use movement::{integrate, Velocity};
    assert_eq!(integrate(2.0, Velocity(3.0), 0.5), 3.5);
}
```

Visibility forms you should recognize:

```rust
pub               // visible outside the module/crate as appropriate
pub(crate)        // visible anywhere in this crate
pub(super)        // visible to the parent module
pub(in crate::x)  // visible within a specific ancestor path
```

Keep implementation details private by default. Visibility is part of architecture.

### Closures as behavior parameters

A generic callback is statically dispatched:

```rust
fn retry<T, E, F>(attempts: usize, mut operation: F) -> Result<T, E>
where
    F: FnMut() -> Result<T, E>,
{
    let mut remaining = attempts;
    loop {
        match operation() {
            Ok(value) => return Ok(value),
            Err(error) if remaining > 1 => remaining -= 1,
            Err(error) => return Err(error),
        }
    }
}
```

Know the closure traits:

- `Fn` may be called repeatedly without mutating captured state;
- `FnMut` may mutate captured state;
- `FnOnce` may consume captured values and be called once.

### The builder pattern

Builders make optional configuration readable:

```rust
#[derive(Debug, Default)]
struct LaunchPlan {
    label: String,
    retries: u32,
    visible_loading: bool,
}

impl LaunchPlan {
    fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            ..Self::default()
        }
    }

    fn retries(mut self, retries: u32) -> Self {
        self.retries = retries;
        self
    }

    fn visible_loading(mut self, enabled: bool) -> Self {
        self.visible_loading = enabled;
        self
    }
}

fn main() {
    let plan = LaunchPlan::new("Sanic")
        .retries(2)
        .visible_loading(true);

    assert_eq!(plan.retries, 2);
}
```

Use a builder when many optional fields exist. Use a normal constructor when all fields are required and obvious.

### The newtype pattern

Newtypes prevent semantic mixups:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ActivationId(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SessionId(u64);

fn retire(_session: SessionId) {}

fn main() {
    let activation = ActivationId(5);
    let session = SessionId(5);
    retire(session);

    // retire(activation); // compile-time type mismatch
    println!("{activation:?}");
}
```

Use newtypes whenever two primitive values have different authority or units.

### State machines with enums

A compact state machine:

```rust
#[derive(Debug, PartialEq)]
enum LoadState {
    Idle,
    Loading { completed: u32, total: u32 },
    Ready,
    Failed(String),
}

impl LoadState {
    fn advance(&mut self) {
        match self {
            LoadState::Loading { completed, total } => {
                *completed += 1;
                if *completed >= *total {
                    *self = LoadState::Ready;
                }
            }
            LoadState::Idle | LoadState::Ready | LoadState::Failed(_) => {}
        }
    }
}

fn main() {
    let mut state = LoadState::Loading {
        completed: 0,
        total: 2,
    };
    state.advance();
    state.advance();
    assert_eq!(state, LoadState::Ready);
}
```

This makes illegal combinations such as `failed == true && ready == true` impossible.

### Command enums separate requests from execution

```rust
#[derive(Debug)]
enum WorldCommand {
    Spawn { kind: String },
    Despawn { id: u64 },
    ChangeRoom { room: String },
}

#[derive(Default)]
struct World {
    room: String,
    entities: Vec<(u64, String)>,
    next_id: u64,
}

impl World {
    fn apply(&mut self, command: WorldCommand) {
        match command {
            WorldCommand::Spawn { kind } => {
                let id = self.next_id;
                self.next_id += 1;
                self.entities.push((id, kind));
            }
            WorldCommand::Despawn { id } => {
                self.entities.retain(|(entity_id, _)| *entity_id != id);
            }
            WorldCommand::ChangeRoom { room } => self.room = room,
        }
    }
}
```

This pattern appears in event-driven engines because producers can describe intent without owning the execution machinery.

### Interior mutability: know it, but do not default to it

`Cell`, `RefCell`, `Mutex`, and `RwLock` permit mutation through shared ownership under different runtime rules.

```rust
use std::cell::RefCell;

fn main() {
    let log = RefCell::new(Vec::new());
    log.borrow_mut().push("event");
    assert_eq!(log.borrow().len(), 1);
}
```

`RefCell` enforces borrow rules at runtime and panics on violation. `Mutex` coordinates threads and may block. In Bevy gameplay code, ECS resources/components and system scheduling are usually better than hiding shared mutation behind these types.

### Smart pointers and recursive ownership

`Box<T>` gives one owner a heap allocation:

```rust
#[derive(Debug)]
enum Expression {
    Number(i32),
    Add(Box<Expression>, Box<Expression>),
}
```

`Rc<T>` and `Arc<T>` provide shared ownership. `Arc<T>` is thread-safe. Neither provides mutation by itself.

Use shared ownership for genuinely shared immutable data. Do not use it to avoid deciding which subsystem owns mutable state.

### RAII and cleanup through `Drop`

Rust runs destructors automatically when values leave scope:

```rust
struct Trace(&'static str);

impl Drop for Trace {
    fn drop(&mut self) {
        println!("leaving {}", self.0);
    }
}

fn main() {
    let _trace = Trace("main");
    println!("inside");
}
```

RAII is excellent for files, locks, temporary directories, and native resources. ECS entity cleanup is usually expressed through commands and ownership markers instead of relying on a Rust value's `Drop` implementation.

### Tests, modules, and assertion styles

A self-contained test module:

```rust
fn damage(health: u32, amount: u32) -> u32 {
    health.saturating_sub(amount)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn damage_never_underflows() {
        assert_eq!(damage(10, 3), 7);
        assert_eq!(damage(2, 10), 0);
    }
}
```

Useful assertions:

```rust
assert!(condition);
assert_eq!(actual, expected);
assert_ne!(left, right);
assert!(matches!(value, Some(_)));
```

Prefer assertions that explain the real behavior. Avoid tests coupled only to incidental formatting or private function names.

### A complete standalone example: exact-session command rejection

This example combines newtypes, enums, ownership, `Option`, `Result`, and collections. It models the same class of stale-authority problem that appears in a multi-game host, but it has no Bevy or Ambition dependency.

```rust
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct SessionId(u64);

#[derive(Debug, Clone, PartialEq, Eq)]
struct Session {
    id: SessionId,
    provider: String,
    score: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum SessionCommand {
    AddScore {
        session: SessionId,
        amount: u32,
    },
    Retire {
        session: SessionId,
    },
}

#[derive(Debug, PartialEq, Eq)]
enum CommandError {
    NoActiveSession,
    StaleSession {
        requested: SessionId,
        active: SessionId,
    },
}

#[derive(Default)]
struct Host {
    active: Option<Session>,
    retired_scores: BTreeMap<SessionId, u32>,
    next_id: u64,
}

impl Host {
    fn launch(&mut self, provider: impl Into<String>) -> SessionId {
        let id = SessionId(self.next_id);
        self.next_id += 1;
        self.active = Some(Session {
            id,
            provider: provider.into(),
            score: 0,
        });
        id
    }

    fn apply(&mut self, command: SessionCommand) -> Result<(), CommandError> {
        let active = self
            .active
            .as_mut()
            .ok_or(CommandError::NoActiveSession)?;

        let requested = match &command {
            SessionCommand::AddScore { session, .. } => *session,
            SessionCommand::Retire { session } => *session,
        };

        if requested != active.id {
            return Err(CommandError::StaleSession {
                requested,
                active: active.id,
            });
        }

        match command {
            SessionCommand::AddScore { amount, .. } => {
                active.score = active.score.saturating_add(amount);
            }
            SessionCommand::Retire { .. } => {
                let retired = self.active.take().expect("checked active session");
                self.retired_scores.insert(retired.id, retired.score);
            }
        }
        Ok(())
    }
}

fn main() {
    let mut host = Host::default();
    let session_a = host.launch("Sanic");
    host.apply(SessionCommand::AddScore {
        session: session_a,
        amount: 5,
    })
    .unwrap();

    let session_b = host.launch("Sanic");

    let stale = host.apply(SessionCommand::AddScore {
        session: session_a,
        amount: 100,
    });
    assert!(matches!(
        stale,
        Err(CommandError::StaleSession {
            requested,
            active
        }) if requested == session_a && active == session_b
    ));

    host.apply(SessionCommand::AddScore {
        session: session_b,
        amount: 7,
    })
    .unwrap();
    assert_eq!(host.active.as_ref().map(|session| session.score), Some(7));
}
```

Questions to answer:

- Why is `SessionId` a newtype instead of a bare `u64`?
- Why does `apply` borrow `&mut self`?
- Why is the stale check performed before matching the command's behavior?
- Why does retirement use `Option::take`?
- What bug would appear if the code checked only the provider name?

### Standalone practice sequence

Work through these without opening Ambition:

1. Add a `Pause` command to the session example using an enum state rather than a boolean pair.
2. Change `Host::launch` to retire an existing session explicitly and return both IDs.
3. Add a `RoomId` newtype and make score commands valid only in the active room.
4. Replace one `expect` with a structured error and explain whether the extra complexity helps.
5. Write table-driven tests for stale, missing, current, and retired sessions.
6. Add a generic `Registry<K, V>` backed by `BTreeMap`, with `insert_unique` returning a custom duplicate-key error.
7. Implement a builder for `Session` and decide which fields should remain mandatory constructor arguments.
8. Write a small iterator pipeline that reports every session whose score exceeds a threshold.

When these exercises feel ordinary, the syntax in the rest of the course should no longer be the main obstacle.

## 3. Learn the repository before learning more syntax

Ambition is a Cargo workspace. The root [`Cargo.toml`](../../Cargo.toml) lists many crates, but they are not peers. They form layers.

Read these first:

1. [`../systems/architecture.md`](../systems/architecture.md)
2. [`../concepts/bevy-native-data-driven-ecs.md`](../concepts/bevy-native-data-driven-ecs.md)
3. [`../concepts/sim-presentation-seam.md`](../concepts/sim-presentation-seam.md)
4. [`../concepts/rust-module-boundaries.md`](../concepts/rust-module-boundaries.md)
5. [`../adr/0002-engine-must-be-bevy-native.md`](../adr/0002-engine-must-be-bevy-native.md)
6. [`../adr/0012-sim-presentation-split-and-events-refactor.md`](../adr/0012-sim-presentation-split-and-events-refactor.md)
7. [`../adr/0017-rust-behavior-ron-content-ldtk-space.md`](../adr/0017-rust-behavior-ron-content-ldtk-space.md)

The broad dependency direction is:

```text
foundation vocabulary
    ↓
simulation machinery
    ↓
presentation and named content
    ↓
provider applications and shared host
```

Useful landmarks:

- `crates/ambition_engine_core` — reusable movement and geometry logic;
- `crates/ambition_platformer_primitives` — lifecycle, schedule, and shared platformer vocabulary;
- `crates/ambition_actors` — the largest body of gameplay machinery;
- `crates/ambition_runtime` — canonical runtime/session state;
- `crates/ambition_render` — presentation systems;
- `crates/ambition_game_shell` — title routes, startup, launchers, and session bridging;
- `crates/ambition_load` — load transactions and authorization;
- `crates/ambition` — a public facade and shared provider authoring surface;
- `game/ambition_content` — Ambition-specific named content and provider implementation;
- `game/ambition_app` — the shipping host and binaries;
- `game/ambition_demo_*` — smaller providers and standalone hosts.

### Exercise: prove the dependency direction to yourself

Run:

```bash
cargo metadata --format-version 1 > /tmp/ambition-metadata.json
cargo tree -p ambition_app --depth 2
cargo tree -p ambition_engine_core --depth 2
```

Then answer without guessing:

- Why can `ambition_app` depend on `ambition_content`?
- Why should `ambition_engine_core` not depend on it?
- Where would a reusable collision shape belong?
- Where would a named boss roster belong?
- Where would window creation belong?

If you cannot answer those questions, do not begin a cross-crate refactor yet.

## 4. Cargo is part of the language

In Python, packaging can feel separate from programming. In Rust, crate boundaries, features, profiles, and dependency direction are part of program design.

You need to be comfortable with these commands:

```bash
cargo check -p ambition_game_shell
cargo test -p ambition_game_shell
cargo test -p ambition_game_shell one_specific_test
cargo test -p ambition_demo_sanic_app --features visible
cargo tree -p ambition_app
cargo tree -p ambition_app -i ambition_audio
cargo metadata --format-version 1
cargo rustc -p ambition_app -- --print cfg
```

Use `cargo check` while changing types and system signatures. It is usually the fastest way to let the compiler explain what you broke.

Use targeted tests while changing behavior. Do not begin with `cargo test --workspace` unless you are doing final integration verification.

Features matter. Some crates deliberately separate:

- headless versus visible presentation;
- native versus web audio;
- test/recording versus physical audio output;
- optional development tools.

When code appears to be missing, inspect the nearest `Cargo.toml` and search for `#[cfg(...)]` before assuming the symbol does not exist.

### Exercise: trace one feature

Choose the `visible` feature in either demo application. Find:

1. where it is declared in `Cargo.toml`;
2. which dependency features it enables;
3. which `#[cfg(feature = "visible")]` modules it exposes;
4. which tests require it.

This is the Rust equivalent of tracing an optional Python dependency through installation extras, imports, and runtime code paths—but the compiler enforces it.

## 5. Ownership is dataflow, not ceremony

The three most important Rust operations are:

```rust
let owned: T = value;
let shared: &T = &owned;
let exclusive: &mut T = &mut owned;
```

An owned value may be moved. Many shared borrows may coexist. One mutable borrow excludes all other borrows for its duration.

In ordinary Rust, this controls memory safety. In Ambition, it also expresses who may change game state.

Consider a Bevy system parameter:

```rust
fn system(
    rooms: Res<RoomSet>,
    mut active_room: ResMut<ActiveRoomMetadata>,
) {
    // many systems may read rooms;
    // this system has exclusive access to active_room for this schedule step.
}
```

`Res<T>` is a shared borrow from the ECS world. `ResMut<T>` is an exclusive borrow. A `Query<&T>` is shared component access. A `Query<&mut T>` is exclusive component access.

The compiler and Bevy scheduler use those signatures to decide whether systems can run safely in parallel.

### Moves

A common surprise for Python programmers:

```rust
let a = String::from("room");
let b = a;
// a is no longer usable
```

The string allocation was moved into `b`. Use `.clone()` only when you really want a second owned value.

In this repository, excessive cloning often indicates one of three things:

- immutable authored data is being copied into live mutable state;
- a snapshot is being made deliberately;
- ownership is unclear and cloning is being used to avoid deciding.

Do not mechanically remove clones, but ask what each clone means.

### Borrowing fields versus whole structures

Rust is much easier when structures reflect independent authority. If two systems need to mutate unrelated fields of one giant resource, the resource may be too broad. ECS components and focused resources often exist to make those authorities separable.

### Exercise: follow an owned value

Read [`../../crates/ambition_game_shell/src/session.rs`](../../crates/ambition_game_shell/src/session.rs), especially:

- `ActiveGameplaySession`;
- `GameplaySessionInstance`;
- `spawn_world_for`;
- `retire_if_activation`.

Answer:

- Which values are cloned because they are immutable identity facts?
- Which value is moved when the session retires?
- Why does `spawn_world_for` take `&mut self`?
- Why does it return `Option<Entity>` instead of panicking?
- How does the exact activation ID prevent an old session from mutating a new one?

This one file teaches more useful ownership than many generic Rust tutorials.

## 6. Enums and pattern matching are the control plane

Python often represents state with strings, nullable fields, or loosely related booleans. Rust code is clearer when the valid states are an enum.

Example shape:

```rust
enum GameplaySessionEvent {
    Activated { activation: ActiveShellExperience, scope: SessionScopeId },
    Retiring { activation: ActiveShellExperience, scope: SessionScopeId },
}
```

A `match` must handle every variant:

```rust
match event {
    GameplaySessionEvent::Activated { activation, scope } => { /* ... */ }
    GameplaySessionEvent::Retiring { activation, scope } => { /* ... */ }
}
```

This is more than syntax. It means adding a new lifecycle state creates compiler errors in every place that must decide what that state means.

Use enums for:

- lifecycle phases;
- movement policies;
- body modes;
- load states;
- authority owners;
- error categories;
- commands and messages.

Avoid replacing a meaningful enum with strings simply because strings feel more flexible.

### `if let` and `let ... else`

Ambition frequently uses early rejection:

```rust
let ShellEvent::PreparationRequested(transaction) = event else {
    continue;
};
```

Read this as:

> Continue only for the one event shape this system owns.

This is usually clearer than deeply nested `match` blocks.

### Newtypes

Types such as `ShellActivationId`, `SessionScopeId`, and `ShellExperienceId` may wrap simple integers or strings. They prevent accidental interchange of values that have different meanings.

Python would rely on naming discipline. Rust can make mixing them a type error.

### Exercise: identify impossible states

Read the IDs and lifecycle types exported by [`../../crates/ambition_game_shell/src/lib.rs`](../../crates/ambition_game_shell/src/lib.rs).

For each newtype or enum, write down one bug that would be possible if the code used plain strings or integers everywhere.

## 7. Traits are capabilities, not classes

A Rust trait says that a type provides a capability.

```rust
trait Plugin {
    fn build(&self, app: &mut App);
}
```

A type does not inherit data from `Plugin`. It promises that it knows how to modify an `App` during composition.

Important trait patterns in this repository:

- `Plugin` and `PluginGroup` — compose systems, resources, messages, and other plugins;
- `Bundle` — a collection of components that can be spawned together;
- `Resource`, `Component`, `Message`, `SystemSet` — marker traits usually supplied through derives;
- `Default` — construct a conventional starting value;
- `From`/`Into` — explicit conversion;
- custom extension traits such as `...AppExt` — add domain-specific methods to `App`.

### Derive macros

This syntax:

```rust
#[derive(Resource, Default)]
struct ActiveGameplaySession(...);
```

invokes procedural macros that generate trait implementations. It is not a comment and not runtime reflection.

When compiler errors point into generated code, look first at:

- whether every field satisfies the derived trait bounds;
- whether the required crate feature is enabled;
- whether the type has the correct visibility;
- whether a generic parameter needs `Send + Sync + 'static` for Bevy.

### Generic bounds

Bevy stores most runtime values in a long-lived world and may schedule systems across threads. This is why generic provider resources often require:

```rust
M: Send + Sync + 'static
```

Interpret these bounds as:

- `Send`: the value can move between threads;
- `Sync`: shared references can be used from multiple threads;
- `'static`: it does not borrow temporary stack data.

`'static` does not necessarily mean “lives forever.” It often means “owns everything it contains.”

### Exercise: read one extension trait

Find `GameplaySessionAppExt`, `ShellExperienceAppExt`, or another `AppExt` trait. Trace:

1. the trait definition;
2. its implementation for `App`;
3. one caller;
4. which lower-level Bevy registrations it hides.

Then decide whether the extension trait reduces concepts or merely shortens syntax.

## 8. `Option` and `Result` encode absence and failure

Python code often lets absence travel as `None` until something crashes. Rust requires explicit handling.

```rust
fn active_world_entity(&self) -> Option<Entity>
```

This means the caller must accept that no world exists at the title screen or during a narrow activation window.

Common operators:

```rust
let value = option?;                 // return None early
let value = option.unwrap_or(default);
let value = option.expect("invariant explanation");
let Some(value) = option else { return; };
```

Use `expect` only when the invariant truly belongs to the caller or composition. The message should explain the violated architectural assumption.

Use `Result<T, E>` for operations where failure should carry information:

- file and asset loading;
- parsing;
- save data;
- content validation;
- external tool execution.

Bevy systems sometimes represent recoverable absence with `Option<Res<T>>` or `Option<Single<...>>`. This is appropriate when the system can legitimately run in an application composition that lacks that data.

It is not appropriate when a required plugin contract is accidentally missing. In that case, initialize the resource in the owning plugin or gate the system on the proper lifecycle condition.

### Exercise: classify absence

For five `Option<Res<T>>` occurrences in the repository, decide whether each means:

- optional device or platform capability;
- optional application composition;
- transient lifecycle absence;
- technical debt hiding a missing plugin contract.

The syntax is the same. The architecture is not.

## 9. Iterators replace many Python loops

Rust iterators are lazy, typed pipelines. They are close to Python generator expressions, but the compiler specializes them aggressively.

```rust
let available = catalog
    .entries
    .iter()
    .filter(|entry| entry.available)
    .count();
```

You should understand:

- `.iter()` gives shared references;
- `.iter_mut()` gives mutable references;
- `.into_iter()` consumes the collection;
- `.map`, `.filter`, `.find`, `.any`, `.all`, `.collect`;
- iterator ownership often determines whether later code may still use the collection.

Do not force every loop into an iterator chain. Game logic with stateful early exits is often clearer as a `for` loop.

### Exercise: translate both ways

Choose one iterator chain in the shell or load code. Rewrite it on paper as an explicit `for` loop. Then explain which form makes ownership and early exit clearer.

## 10. Lifetimes: learn to read them before writing them

You do not need advanced lifetime wizardry to work on most of Ambition. You do need to read signatures involving borrowed system parameters.

```rust
pub struct PlatformerSessionBuilder<'w, 's> {
    commands: Commands<'w, 's>,
    // ...
}
```

The lifetimes describe borrows Bevy supplies from the ECS world and system state. You usually do not choose them manually; the `SystemParam` derive and function signature do.

Useful rules:

1. A returned reference cannot outlive the value it points into.
2. Storing references inside long-lived ECS resources is usually wrong; store owned IDs, handles, or data instead.
3. If a resource must live in the `World`, prefer owned values with `'static` contents.
4. If lifetime annotations become complicated in gameplay code, reconsider whether the API should return an owned value or entity ID.

### Smart pointers

Know these distinctions:

- `Box<T>` — one owned heap allocation;
- `Arc<T>` — shared ownership across threads;
- `Mutex<T>`/`RwLock<T>` — synchronized interior mutation;
- `Handle<T>` — Bevy asset identity and ownership;
- `Entity` — ECS identity, not a Rust reference.

Avoid reaching for `Arc<Mutex<T>>` as a Python-style shared object. Inside Bevy, resources, components, messages, and schedules usually provide the right ownership model.

## 11. Bevy ECS: the minimum complete mental model

A Bevy `App` owns one or more ECS worlds and schedules. Plugins configure the app. Systems run later.

### Components

A component is data attached to an entity:

```rust
#[derive(Component)]
struct Velocity(Vec2);
```

An entity is an ID. It becomes meaningful through its components.

Use components for data that belongs to individual runtime objects:

- position and velocity;
- actor identity;
- health;
- session ownership markers;
- presentation markers;
- behavior state.

### Resources

A resource is one value per ECS world:

```rust
#[derive(Resource, Default)]
struct ActiveGameplaySession(...);
```

Use resources for App-local global authority or registries:

- active session;
- catalogs;
- settings;
- load coordinator;
- fixed simulation tick;
- asset collections.

A resource is not a process global. Separate `App` instances have separate resources, which is essential for tests and provider composition.

### Systems

A system is usually an ordinary function whose parameters implement `SystemParam`:

```rust
fn apply_velocity(
    time: Res<Time>,
    mut bodies: Query<(&Velocity, &mut Transform)>,
) {
    for (velocity, mut transform) in &mut bodies {
        transform.translation += velocity.0.extend(0.0) * time.delta_secs();
    }
}
```

Bevy derives data access from the signature.

A runtime panic saying “Parameter failed validation: Resource does not exist” means a required system parameter was unavailable when the system ran. The usual fixes are:

- the owning plugin should `init_resource` or `insert_resource`;
- the system should run only after the lifecycle phase that creates it;
- the parameter is legitimately optional and should be `Option<Res<T>>`;
- the application composition omitted a required plugin.

Do not blindly wrap every missing resource in `Option`. That converts composition bugs into silent no-ops.

### Queries

A query selects entities by component shape:

```rust
Query<(&mut BodyKinematics, &CharacterIdentity), With<PrimaryPlayer>>
```

Important filters:

- `With<T>` — entity must have `T`;
- `Without<T>` — entity must not have `T`;
- `Changed<T>` — component changed since the system last observed it;
- tuples combine filters.

Access methods communicate expected cardinality:

- `iter()` — zero or more;
- `single()` — exactly one, returning an error otherwise;
- `get(entity)` — one known entity;
- `get_mut(entity)` — one known entity with mutable access.

Choose cardinality deliberately. A player query that silently iterates over two primary players may hide an ownership bug. A system that can legitimately run before the player exists should not call `single().unwrap()`.

### Commands are deferred

`Commands` does not mutate the world immediately. It queues structural changes that Bevy applies at a synchronization point.

This explains many one-frame effects:

- an entity spawned in one system may not be queryable by another unordered system in the same schedule;
- despawn requests may remain visible until commands are applied;
- tests may need another `app.update()` when verifying deferred lifecycle cleanup.

If same-frame visibility matters, use explicit ordering and understand where deferred commands are applied. Do not add arbitrary update loops until a test happens to pass.

### Exercise: build a tiny ECS model

In a scratch test module, create an `App` with:

- a `Counter` resource;
- a `Tagged` component;
- one startup system that spawns three tagged entities;
- one update system that counts them;
- a test that calls `app.update()` and inspects the resource.

Then add session ownership to those entities using the lifecycle helpers in `ambition_platformer_primitives`. Retire the session and prove the entities disappear.

This exercise covers most mechanics used by larger Ambition integration tests.

## 12. Plugins are the unit of composition

A plugin should own the registrations needed for one coherent subsystem:

```rust
impl Plugin for ExamplePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ExampleState>()
            .add_message::<ExampleCommand>()
            .add_systems(Update, handle_commands);
    }
}
```

A good plugin makes its required resources and systems appear together. A bad composition requires a host to remember ten unrelated initialization calls in the right order.

Read these examples:

- [`../../crates/ambition_game_shell/src/lib.rs`](../../crates/ambition_game_shell/src/lib.rs)
- [`../../game/ambition_content/src/provider.rs`](../../game/ambition_content/src/provider.rs)
- the plugin entrypoint in [`../../crates/ambition_render/src/lib.rs`](../../crates/ambition_render/src/lib.rs)

Questions to ask when editing a plugin:

- Does it initialize every resource required by its systems?
- Is the resource App-local?
- Does it install systems into the correct schedule?
- Does it depend on a higher-level host unnecessarily?
- Is optional presentation behind a feature or separate plugin?
- Can a minimal test `App` compose it without hidden initialization?

### Plugin groups

A `PluginGroup` is a conventional bundle of plugins. It is useful for host profiles such as minimal shell, visible presentation, or default engine composition.

Do not confuse a plugin group with inheritance. It is just ordered composition.

## 13. Schedules, sets, and ordering

Ambition relies heavily on explicit schedule structure.

You need to understand:

```rust
app.add_systems(Update, system);
app.add_systems(FixedUpdate, simulation_system);
app.configure_sets(Update, MySet.after(OtherSet));
app.add_systems(Update, (a, b, c).chain());
app.add_systems(Update, system.run_if(condition));
```

### `Update` versus fixed simulation

Presentation, routing, input sampling, and asynchronous readiness usually belong to frame-driven schedules.

Deterministic gameplay simulation belongs to the configured fixed simulation schedule. Read:

- [`../systems/two-clock-simulation.md`](../systems/two-clock-simulation.md)
- [`../concepts/input-and-game-modes.md`](../concepts/input-and-game-modes.md)
- [`../adr/0010-time-domains-and-regime-policies.md`](../adr/0010-time-domains-and-regime-policies.md)

Do not use wall-clock `Time` directly inside deterministic simulation unless the architecture explicitly calls for that time domain.

### Sets express partial order

A system set names a phase. Systems can be ordered relative to the phase instead of every other function.

This is easier to maintain than a long chain of exact function-to-function dependencies.

Use `.chain()` when every system genuinely requires the previous system's commands or messages to be visible in that order. Do not chain large groups merely to make behavior deterministic; unnecessary chaining removes parallelism and hides the real dependency.

### Run conditions

A run condition is an architectural gate:

- only while a gameplay session exists;
- only while a menu is open;
- only when an asset is ready;
- only in a particular game mode.

Prefer one clear lifecycle gate on a coherent system graph over dozens of individual `Option<Res<T>>` parameters that silently skip work.

### Exercise: draw a schedule

Choose one plugin you are changing. Write a small diagram showing:

```text
input sampling
→ shell/load commands
→ session activation
→ fixed simulation
→ presentation synchronization
→ cleanup
```

For every arrow, find the actual set ordering or message boundary in code. If an arrow exists only in your assumptions, you have found a likely bug.

## 14. Messages and commands make temporal coupling explicit

Bevy messages allow one system to request work without directly calling another system.

```rust
#[derive(Message)]
enum ShellCommand {
    GoTo(ShellRouteId),
    QuitToHome,
}
```

A writer emits commands. A reader consumes them during a schedule.

This is useful when:

- the sender should not own the receiver;
- ordering is defined by schedule phases;
- multiple systems may produce requests;
- the request carries exact identity or authority.

Do not use messages as an untyped global event bus. Messages should have narrow semantics and clear ownership.

In Ambition, exact identities matter. A stale request from session A must not affect session B just because both sessions belong to the same provider.

### Exercise: trace one message end to end

Trace `QuitToHome` from:

1. input or provider presentation;
2. message writer;
3. shell command processing;
4. route retirement;
5. gameplay session retirement;
6. session-scoped entity cleanup;
7. frontend authority restoration.

Write down every schedule boundary. This is a practical lesson in event-driven Rust and Bevy architecture.

## 15. `SystemParam` bundles are dependency injection with borrow checking

Large systems often need many resources and queries. Bevy supports custom parameter bundles:

```rust
#[derive(SystemParam)]
struct PlatformerSessionBuilder<'w, 's> {
    commands: Commands<'w, 's>,
    // focused resources and queries
}
```

Read the provider authoring surface in [`../../crates/ambition_platformer_provider/src/lifecycle.rs`](../../crates/ambition_platformer_provider/src/lifecycle.rs).

A good `SystemParam` bundle:

- groups one coherent authority;
- has a narrow name;
- exposes a small operation such as `prepare` or `build`;
- avoids borrowing the same resource twice through nested bundles;
- prevents system signatures from becoming unreadable.

A bad bundle is a disguised god object that gives every system access to everything.

When Bevy reports conflicting system parameters, inspect the expanded bundle mentally:

- two `ResMut<T>` values conflict;
- `Res<T>` conflicts with `ResMut<T>` in the same system;
- two queries may overlap mutably unless separated by filters or a `ParamSet`.

Use `ParamSet` only when the accesses must be sequential and cannot be represented as disjoint queries. It is not a general escape hatch.

## 16. The canonical session-world model

The shared title host can launch Ambition, Sanic, Mary-O, and other providers. Therefore gameplay state cannot be a process-global singleton that survives route changes.

The key types are in:

- [`../../crates/ambition_game_shell/src/session.rs`](../../crates/ambition_game_shell/src/session.rs)
- [`../../crates/ambition_runtime/src/session_world.rs`](../../crates/ambition_runtime/src/session_world.rs)
- [`../../crates/ambition/src/session_world.rs`](../../crates/ambition/src/session_world.rs)

The model is:

```text
shell route activation
→ exact activation identity
→ exact gameplay session scope
→ provider publishes one canonical world entity
→ session-scoped entities refer to that scope
→ route retirement retires the exact session
```

The important Rust lesson is that entity IDs and newtyped identities replace long-lived references. Systems look up the current world through exact authority rather than storing borrowed pointers.

The important architecture lesson is that `None` at the title screen is meaningful. There is no gameplay world there.

### Compatibility projections

Some legacy systems may still consume resident Bevy resources projected from the canonical session world. When working in this area, determine whether you are editing:

- canonical state;
- a temporary derived projection;
- presentation state;
- immutable authored data.

Do not accidentally create two mutable sources of truth.

### Exercise: follow room identity

Start with the active room in `PlatformerSessionWorld`. Trace how it reaches:

- collision geometry;
- map or room presentation;
- music selection;
- reset or portal logic.

Mark every clone and every resource projection. This is the best way to understand where the current architecture is clean and where migration remains.

## 17. Providers, preparation, and one-shot authorization

A provider is a game that can be linked into the shared host. Read:

- [`../../crates/ambition_platformer_provider/src/lifecycle.rs`](../../crates/ambition_platformer_provider/src/lifecycle.rs)
- [`../../game/ambition_content/src/provider.rs`](../../game/ambition_content/src/provider.rs)
- the equivalent provider files in `game/ambition_demo_sanic`, `game/ambition_demo_mary_o`, and `game/ambition_demo_pocket`.

The lifecycle is intentionally explicit:

```text
route requested
→ fresh load transaction
→ provider validates and prepares data
→ immutable prepared session is published
→ required barrier becomes ready
→ one-shot authorization is consumed
→ exact gameplay session activates
```

This section combines several advanced Rust ideas:

- one App-local prepared-session store keyed by exact load transactions;
- newtyped IDs preventing accidental cross-use;
- `BTreeMap` for deterministic App-local registries;
- messages for lifecycle transitions;
- owned prepared data moved exactly once into live session state;
- `Option` and `Result` for stale or rejected work;
- `SystemParam` bundles for coherent preparation and construction authority.

### Why one shared prepared-session store is safe

Every provider prepares the same concrete `PlatformerSessionWorld` type, but the
store is keyed by the shell's exact `ProviderLoadTransaction`. Publication and
activation both validate that transaction and consume its one-shot
`PreparedSessionIdentity`. A provider therefore cannot take another provider's
world merely because the concrete Rust type matches.

This is stronger and simpler than the older per-provider marker-resource pattern:
the authorization identity, not a parallel family of resource types, separates
prepared sessions.

### Exercise: add a non-shipping provider on paper

Without writing code, list everything a tiny fifth provider would need:

- registration identity;
- route;
- character and audio fragments;
- a session-world source system passed to `PlatformerExperienceAuthoring::install`;
- standalone host composition;
- teardown behavior.

Then compare your list with Pocket. The differences reveal which concepts the common provider API has successfully centralized.

## 18. Simulation and presentation must remain different kinds of state

Read [`../concepts/sim-presentation-seam.md`](../concepts/sim-presentation-seam.md).

Simulation state answers questions such as:

- where is the body?
- what mode is it in?
- how much health remains?
- which room is active?
- what interaction occurred?

Presentation state answers:

- which sprite entity displays that body?
- which animation frame is visible?
- which sound should play?
- where is the camera?
- which HUD root exists?

The simulation should remain useful in headless tests and deterministic replays. Presentation may observe simulation and create visual/audio consequences, but it should not become the authoritative source of gameplay facts.

Rust helps through crate direction and separate component/resource types. Bevy helps through different schedules and messages.

### Exercise: classify ten types

Choose ten types from a feature you care about. Label each:

- authored data;
- simulation state;
- simulation request/message;
- presentation state;
- presentation request/message;
- host/platform state.

If one type belongs to several categories, inspect whether it is an intentional boundary object or accidental coupling.

## 19. The movement kernel is policy plus shared integration

Read:

- [`../concepts/movement-collision.md`](../concepts/movement-collision.md)
- [`../adr/0024-frame-aware-unified-movement-kernel.md`](../adr/0024-frame-aware-unified-movement-kernel.md)
- the movement module under [`../../crates/ambition_engine_core/src/movement/`](../../crates/ambition_engine_core/src/movement/)

The important Rust design is not any one equation. It is that body kinds converge on a shared typed entrypoint and select explicit movement policies.

Look for:

- enums selecting solver policy;
- immutable context values passed into a step;
- mutable body state passed with exclusive authority;
- helper functions kept private so production callers cannot bypass the kernel;
- tests that compare observable movement rather than source spelling.

### Exercise: trace one frame

Pick Sanic, Mary-O, or the main player. Trace:

```text
raw input
→ action state
→ ControlFrame
→ body mode / ability policy
→ movement model
→ movement kernel
→ collision result
→ updated body state
→ presentation sync
```

Do not stop at function names. Record the type passed across each boundary.

## 20. Rust, RON, and LDtk have different jobs

Read [`../adr/0017-rust-behavior-ron-content-ldtk-space.md`](../adr/0017-rust-behavior-ron-content-ldtk-space.md).

A practical rule:

- Rust owns behavior, invariants, and algorithms;
- RON owns structured authored tuning and catalogs;
- LDtk owns spatial layout and level-authored entities;
- generated asset tables connect authored IDs to files and runtime handles.

When changing a feature, first ask what kind of fact it is.

Examples:

- “Dash acceleration follows this algorithm” — Rust.
- “This character's dash acceleration is 17.5” — likely RON/catalog data.
- “This portal is at this coordinate in this room” — LDtk.
- “This sprite ID resolves to this generated sheet” — asset/catalog data.

Do not add a Rust enum variant for every named content object. Do not move behavioral invariants into unvalidated data merely to avoid compiling.

### Exercise: add one hypothetical enemy

Write down which parts would require:

- no code;
- RON/catalog changes;
- LDtk placement;
- new Rust behavior;
- new presentation logic.

Use [`../recipes/adding-a-character.md`](../recipes/adding-a-character.md) and the character catalog docs as a model.

## 21. Assets are handles and asynchronous readiness

A Bevy `Handle<T>` is not the asset itself. It identifies an asset managed by `AssetServer` and asset storage.

Important consequences:

- loading may be asynchronous;
- a valid handle does not guarantee the asset is ready;
- visible and headless applications may install different asset plugins;
- relative paths resolve against registered asset sources;
- providers may own different source roots;
- tests should distinguish resolution, readiness, and physical device output.

Read:

- [`../concepts/asset-management.md`](../concepts/asset-management.md)
- [`../systems/asset-manager.md`](../systems/asset-manager.md)
- [`../concepts/generated-assets-audio.md`](../concepts/generated-assets-audio.md)

When Bevy logs “Path not found,” determine:

1. which `AssetSource` handled the URI;
2. what root it uses;
3. whether the path is authored or generated;
4. whether the provider or shared engine owns the file;
5. whether the error appears in a no-window test that should not load presentation assets at all.

Do not fix asset ownership by copying files into every crate unless duplication is actually the intended architecture.

## 22. Tests are small programs

A Bevy test often constructs an application explicitly:

```rust
let mut app = App::new();
app.add_plugins(...);
app.update();
```

This is not a mock framework. It is a small real ECS application.

Useful test levels:

1. pure Rust unit test for an algorithm;
2. minimal Bevy `App` for a resource/system contract;
3. provider or standalone-host integration test;
4. shipping-host lifecycle test;
5. manual visible/audio-device acceptance.

Choose the lowest level that can observe the behavior.

### Common test timing mistake

Because commands are deferred and fixed time has an accumulator, “call `app.update()` once” is not a universal activation protocol.

Prefer helpers that wait for an observable condition within a small budget:

```rust
for _ in 0..16 {
    app.update();
    if player_exists(&mut app) {
        break;
    }
}
```

But do not use large arbitrary loops to hide a lifecycle bug. The condition should correspond to the contract being tested.

### Input edge mistake

Bevy's `ButtonInput::clear()` clears transient edge sets but may leave the button held. Tests simulating repeated taps should release or reset the button between frames.

### Device boundaries

No-window automated tests should not play sound or require a physical GPU/display. Use recording or inert backends while still exercising the resolver and ownership logic.

### Exercise: repair a test without weakening it

Find a test that waits for a player or route. Explain:

- what event or state transition is asynchronous/deferred;
- why the old exact-frame assumption was brittle;
- which observable condition is the real contract;
- what maximum budget is reasonable and why.

## 23. Reading compiler errors productively

### Moved value: `E0382`

You used an owned value after transferring ownership.

Questions:

- Should the callee borrow instead?
- Should the caller stop using it?
- Is a clone semantically correct?
- Should a small copyable ID derive `Copy`?

### Conflicting borrows: `E0499`, `E0502`

Two pieces of code want incompatible access.

Questions:

- Can the borrow scopes be shortened?
- Can data be split into independent fields/resources?
- Are two queries actually disjoint?
- Is a `ParamSet` justified?

### Trait bound not satisfied: `E0277`

Read the full chain. In Bevy code, common missing bounds are:

- `Component`/`Resource` derive absent;
- `Send + Sync + 'static` absent;
- a plugin feature disabled;
- a bundle field is not a component;
- an iterator item type differs from what `collect` expects.

### Mismatched types: `E0308`

Do not immediately call `.into()` until it compiles. Compare the semantic types. A typed ID and a source string may intentionally be different.

### Bevy runtime system-parameter validation

A panic that a resource does not exist is not a borrow-checker error. Enable enough debug information to identify the system, then inspect plugin ownership and run conditions.

Useful commands:

```bash
RUST_BACKTRACE=1 cargo test -p CRATE TEST_NAME -- --nocapture
RUST_BACKTRACE=full cargo test -p CRATE TEST_NAME -- --exact --nocapture
rustc --explain E0277
```

Use `rg` to find the type definition, plugin initialization, and every required `Res<T>` parameter:

```bash
rg -n "struct SceneEntities|init_resource::<SceneEntities>|Res<SceneEntities>" crates game
```

## 24. A disciplined code-reading method

Without an LLM, do not read a large crate from top to bottom. Follow types and registration.

For any feature:

1. Find the public type or message.
2. Find the plugin that registers it.
3. Find the systems that read or mutate it.
4. Find the schedule/set ordering.
5. Find the content or asset source.
6. Find the smallest behavioral test.
7. Run that test before changing anything.
8. Make one conceptual change.
9. Run `cargo check` and the focused test again.

Useful searches:

```bash
rg -n "TypeName" crates game
rg -n "add_systems|configure_sets|in_set|before|after" path/to/crate
rg -n "MessageReader<|MessageWriter<" crates game
rg -n "ResMut<RoomSet>|Query<.*BodyKinematics" crates game
rg -n "impl Plugin for" crates game
rg -n "#\[cfg" path/to/crate
```

Use rust-analyzer's “go to definition,” “find references,” “call hierarchy,” and inline type display. For this repository, “find references” is often more useful than reading prose documentation.

## 25. A twelve-module learning path

The modules below are ordered so that each unlocks a real class of Ambition work.

### Module 1: workspace navigation

Read the architecture documents and use `cargo tree`, `cargo metadata`, and `rg`.

Deliverable: draw the crate layers and place five features correctly.

### Module 2: ownership and identity

Read session IDs, active-session state, and session-scoped entity helpers.

Deliverable: explain stale activation rejection without using the phrase “the borrow checker handles it.”

### Module 3: ECS fundamentals

Build the tiny `App` exercise with a resource, components, systems, and cleanup.

Deliverable: one passing minimal-app test.

### Module 4: schedules and fixed time

Trace one input frame into one fixed simulation tick.

Deliverable: a schedule diagram with actual sets and messages.

### Module 5: queries and cardinality

Study player, actor, and room queries.

Deliverable: identify three places where zero, one, and many entities are expected.

### Module 6: plugins and composition

Read a foundation plugin, a provider plugin, and the shipping host.

Deliverable: list what each layer owns and what it must not initialize.

### Module 7: provider lifecycle

Trace registration, preparation, authorization, activation, and retirement.

Deliverable: explain how same-provider relaunch avoids observing stale state.

### Module 8: simulation/presentation seam

Trace one movement or combat result into sprite, HUD, and audio consequences.

Deliverable: classify every involved type by authority.

### Module 9: data authoring

Follow one character or room from RON/LDtk through catalogs into runtime state.

Deliverable: propose a content-only change and a behavior change, with different file lists.

### Module 10: debugging and tests

Reproduce one known failure in a focused test and repair the real contract.

Deliverable: a regression test that observes behavior rather than source wording.

### Module 11: a contained feature

Implement a small feature that touches one simulation component, one system, and one presentation consequence.

Good examples:

- a new diagnostic readout;
- a small authored tuning parameter;
- a new provider-local room behavior;
- an explicit load progress item;
- a session-scoped visual marker.

Avoid starting with movement-kernel redesign, rollback, or a large crate split.

### Module 12: independent maintenance

Take one issue from report to commit:

- reproduce;
- locate ownership;
- read the relevant ADR/concept page;
- make the smallest coherent change;
- run focused tests;
- run broader tests only after the focused surface is green;
- write a commit message that records the motivating problem.

## 26. What to memorize and what to look up

Memorize:

- move versus shared borrow versus mutable borrow;
- `Option` and `Result` control flow;
- struct, enum, trait, and `match`;
- component versus resource versus message;
- query cardinality;
- commands are deferred;
- plugins configure Apps;
- `Update` versus fixed simulation;
- session scope and exact identity;
- simulation versus presentation authority;
- Cargo crate direction.

Look up:

- exact Bevy API names;
- advanced lifetime syntax;
- asset-loader trait details;
- platform feature combinations;
- rarely used iterator adapters;
- proc-macro expansion details;
- wgpu/window/audio backend configuration.

The skill is not remembering every API. It is knowing which layer owns the answer and how to find the definition.

## 27. A practical pre-edit checklist

Before editing:

- What is the authoritative state?
- Which crate should own the change?
- Which plugin installs the relevant systems/resources?
- Which schedule runs them?
- Is this simulation, presentation, content, or host policy?
- Does the code run in headless mode?
- Does it belong to a gameplay session or the frontend?
- What focused test already exercises the path?

While editing:

- Keep mutable authority narrow.
- Prefer typed IDs over strings at runtime boundaries.
- Preserve exact activation/session identities.
- Avoid process globals.
- Do not make required resources optional merely to silence a panic.
- Do not clone live mutable state into a second authority.
- Do not bypass the movement kernel.
- Do not make presentation authoritative over simulation.

After editing:

```bash
cargo check -p THE_TOUCHED_CRATE
cargo test -p THE_TOUCHED_CRATE THE_FOCUSED_TEST
```

Then run the relevant integration application or suite. Use the repository's root `run_tests.sh` only when you are ready for broad verification.

## 28. Recommended reference shelf

Project documents:

- [`../systems/architecture.md`](../systems/architecture.md)
- [`../concepts/bevy-native-data-driven-ecs.md`](../concepts/bevy-native-data-driven-ecs.md)
- [`../concepts/sim-presentation-seam.md`](../concepts/sim-presentation-seam.md)
- [`../concepts/input-and-game-modes.md`](../concepts/input-and-game-modes.md)
- [`../concepts/movement-collision.md`](../concepts/movement-collision.md)
- [`../concepts/asset-management.md`](../concepts/asset-management.md)
- [`../concepts/testing-and-validation.md`](../concepts/testing-and-validation.md)
- [`../concepts/rust-module-boundaries.md`](../concepts/rust-module-boundaries.md)
- [`../systems/two-clock-simulation.md`](../systems/two-clock-simulation.md)
- [`../recipes/profiling.md`](../recipes/profiling.md)
- [`../recipes/ldtk-authoring.md`](../recipes/ldtk-authoring.md)
- [`../recipes/adding-a-character.md`](../recipes/adding-a-character.md)

Source entrypoints:

- [`../../crates/ambition_game_shell/src/lib.rs`](../../crates/ambition_game_shell/src/lib.rs)
- [`../../crates/ambition_game_shell/src/session.rs`](../../crates/ambition_game_shell/src/session.rs)
- [`../../crates/ambition_platformer_provider/src/lifecycle.rs`](../../crates/ambition_platformer_provider/src/lifecycle.rs)
- [`../../crates/ambition_runtime/src/session_world.rs`](../../crates/ambition_runtime/src/session_world.rs)
- [`../../crates/ambition/src/session_world.rs`](../../crates/ambition/src/session_world.rs)
- [`../../crates/ambition_platformer_primitives/src/`](../../crates/ambition_platformer_primitives/src/)
- [`../../crates/ambition_engine_core/src/movement/`](../../crates/ambition_engine_core/src/movement/)
- [`../../crates/ambition_actors/src/schedule/`](../../crates/ambition_actors/src/schedule/)
- [`../../crates/ambition_actors/src/session/`](../../crates/ambition_actors/src/session/)
- [`../../crates/ambition_render/src/`](../../crates/ambition_render/src/)
- [`../../game/ambition_content/src/provider.rs`](../../game/ambition_content/src/provider.rs)
- [`../../game/ambition_app/src/app/`](../../game/ambition_app/src/app/)

External references worth keeping locally bookmarked:

- The Rust Book, especially ownership, enums, traits, iterators, and smart pointers.
- Rust by Example for syntax lookup.
- The Rust Reference when behavior is subtle.
- `rustc --explain` for compiler error codes.
- Bevy's ECS, schedules, assets, and testing examples for the version pinned by this workspace.
- The docs.rs pages for the exact dependency versions in `Cargo.lock`.

Prefer version-matched documentation. Bevy changes quickly, and examples from another release may compile poorly or describe a different scheduling API.

## 29. The graduation standard

You know enough Rust to work independently on Ambition when you can do all of the following without an LLM:

- locate the owning crate and plugin for a feature;
- explain a borrow or move error in terms of authority;
- add a component, resource, message, and system deliberately;
- place a system in the correct schedule and set;
- write a query with the correct cardinality and filters;
- understand deferred commands and fixed-step timing in tests;
- trace one provider from route request through teardown;
- distinguish authored data, live simulation state, and presentation state;
- read a `SystemParam` bundle and detect conflicting access;
- debug a missing-resource panic by finding the missing plugin contract;
- use Cargo features and targeted tests without rebuilding the entire workspace unnecessarily;
- make a contained change and explain why stale sessions, headless hosts, and other providers remain correct.

At that point, unfamiliar Rust syntax is a lookup problem rather than a blocker. That is the level this course is designed to reach.
