# Reusable script behavior

Rhai entry scripts can import captured libraries. A library can define shared
functions, parameterized behaviors, and factories for typed state declarations.
A new use can choose parameters and compose those behaviors in script code.
It needs no new Rust adapter.

Use [structured state](script-state.md) for reusable nested record types and
bounded text. Library factories can compose these declarations.

## Files and imports

A file or directory listed in `AMBITION_EXTENSION_MODULES` supplies entry
scripts. The environment list uses the platform path separator.
`ExtensionModuleFiles` supplies the same list for a composed host.
Put library files in a subdirectory:

```text
scripts/
  workshop.rhai
  arena.rhai
  lib/
    cadence.rhai
    crafting.rhai
    turret.rhai
```

Both entry scripts can use:

```rhai
import "lib/cadence.rhai" as cadence;
```

IDs are relative to the entry script's directory, including imports inside
libraries. Use slash separators. Absolute paths, parent segments, empty
segments, and dynamic import expressions are refused. Imports must resolve to
constant strings during preparation. A library can import another library.
Cycles are refused with the import chain. Library files do not become
independent entry modules merely because another script imports them.

## Compose behavior and instance state

A library can supply both a state declaration and an algorithm:

```rhai
fn schema(key) {
    #{ key: key, fields: [#{ tag: 1, name: "elapsed", kind: "f32" }] }
}
fn advance(state, dt, period) {
    state.elapsed += dt;
    let ready = state.elapsed >= period;
    if ready { state.elapsed -= period; }
    #{ state: state, ready: ready }
}
```

The entry's `declare()` can call `cadence::schema("left_clock")` and
`cadence::schema("right_clock")`. Declare those keys in the entry's `writes`
list. Each instance has its own record. The entry can then call:

```rhai
let left = cadence::advance(this.state.left_clock, this.dt, 0.2);
let right = cadence::advance(this.state.right_clock, this.dt, 0.5);
this.state.left_clock = left.state;
this.state.right_clock = right.state;
```

Each schema has its own record on the invocation body or session. Give
independent instances distinct schema keys. Modules from one provider must
also use distinct keys for independent state. A factory can select field
initial values and the existing transient, checkpoint, or durable save policy.
Use ordinary parameters for behavior variation. Persistent records support
scalars, vectors, options, bounded sequences and text, body and item locators,
and nested records. Transient records can also retain live object references. A library can return reusable record-kind declarations.
See [structured state](script-state.md) for composition and save rules.

A behavior library can return updated state and native requests, or accept and
return the invocation context. The entry still declares its trigger, reads,
writes, requests, ordering, and request budget. Its hook functions must exist
in the entry script. Library functions can supply the shared implementation.

Native owners decide whether actions apply. Route
[action results](action-results.md) back to the behavior that submitted them.
A timer firing is not proof that a trade applied or a shot was created.

## State and work ownership

Only code is retained across invocations. Each import creates a module
environment within its invocation. Transient variables and captured closures
cannot retain gameplay state between calls. Persistent state belongs in the
host's typed records, which preserve scope, rollback, and save policy.

Library initialization, nested imports, and function calls use the caller's
operation budget. They cannot reset that budget. Default preparation limits
are 64 libraries, 1 MiB of total source, and 32 import levels. Runtime imports
are bounded at 128 per invocation. `LibraryLimits` exposes these limits to
host composition; the operation limit remains a separate preparation input.

The loaded-code ABI is `rhai-f32-exact/5`. Code identity includes entry text
and every reachable library's ID and text, in stable order. Script-visible source names are `entry` and the bundle-relative library IDs.
Host diagnostic paths are excluded from script input and code identity. A helper edit therefore changes each dependent module's code
identity.

## Edit without rebuilding

The file loader captures library source before admission. It reads shared
files once per reload candidate. An edit reloads all dependent entries in
one candidate through the normal mechanical-edit protocol. The timeline
owner controls publication. A foreign or unhealthy timeline can refuse it.

Syntax errors, missing imports, or incompatible state changes keep the last
valid program. Missing dependencies remain watched, so adding or repairing
the file can retry without another edit to the entry. A deleted library does
not erase the already captured program. A removed entry file removes its
modules through the normal module lifetime path.

## Hosts with captured source

With the facade's `script_modules` feature, use
`ambition_platformer2d::extension::{PreparedScript, LibrarySource, LibraryLimits}`.
`PreparedScript::with_libraries` accepts entry text and a library reader. The
reader runs during preparation only. It can read packaged or in-memory text.
`add_prepared_script` installs a captured graph. `bind_file` can instead bind
its own declaration or a linked module selected by stem. No runtime file
access is required. Native and browser hosts use the same compiler and state
contracts; their source transport can differ.

## Evidence and cost

The loaded [library fixture](../../game/ambition_app/tests/scripted_libraries.rs)
composes crafting and firing behaviors through one timing library. Two modules
have independent parameters and records on both a controlled body and an NPC.
The fixture checks native item and wallet changes, actual projectile creation
replies, and GGRS replay. File edits cover syntax errors, missing dependencies,
incompatible schema changes, repair, deletion, and updates to both consumers.

One cached development run on 2026-11-18 measured 123–256 microseconds per
backend call and 153–313 microseconds per whole host invocation. These are
per-entry averages from 32–160 calls under a GGRS sync test. The records include
nested reports and bounded outcome logs. These times include ABI conversion and
library initialization. Whole-host time also includes state
read, validation, and commit. Native reducers, physics, rendering, source
compilation, and asset startup are excluded. This is a local fixture result,
not a release or device budget.

The owner builds for WebAssembly. The assembled app also passes the Android
check with its declared Android features and API 31 NDK wrapper. These checks
prove compilation; they do not prove browser or device playback.

Libraries and structured state support source reuse, behavior factories, nested
records and bounded text. [Actor creation](actor-creation.md) now supplies native character compositions.
Interactive props, cross-object event routing, and the remaining domain lifecycle
contracts stay open in the [capability census](../planning/engine/script-first-capability-census.md).

## Invocation value limits

`LibraryLimits` also bounds values in each prepared Rhai environment. Rhai
counts nested entries and text within a reachable value. The defaults permit
16,384 array or map entries and 65,536 text bytes. A multi-object factory can
therefore build a bounded request containing many nested binding records.
Native domain bounds still decide whether the request can apply.

Hosts can set `max_value_collection_entries` and `max_value_text_bytes` through
`PreparedScript::with_libraries`. The operation, import and value limits are
part of loaded code identity. Equal source with different execution budgets
has a different identity. Rebuild prepared scripts after a limit change.
