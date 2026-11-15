# Scripting language bake-off: Rhai and Luau

**State:** measured 2026-11-14; provisional choice **Rhai** for the first
adapter (audit P3), with the condition that would change it below. Asked by
Jon on 2026-11-14: "We need to explore Luau vs Rhai, I think its time for a
scripting language." The plan it serves is the
[runtime authoring audit](runtime-authoring-audit-2026-11-14/README.md)
(P2, P3) and queue item 0b.

## What was compared

The sentry module's stateful `turret` entry
(`game/ambition_content_modules/src/sentry.rs`) written in both languages
(`game/ambition_content_modules/rhai/sentry.rhai`, `fixtures/scripting_bakeoff/modules/sentry.luau`), each behind
the extension host's backend contract: invocation bytes in, checked output
bytes out. The SDK decodes and encodes the invocation for a script exactly as
for a compiled guest (`ambition_extension_sdk::abi::invoke_with`, added for
this), so a fault stages nothing. The binding is shared: the script sees one
neutral `inv` value (`tick`, `dt`, `trigger`, `state`, `requests`); records
convert generically by their schema's field names and kinds, ports by one
hand-written converter each (P2 would generate those).

Reproduce:

    cd fixtures/scripting_bakeoff
    cargo run --release   # its one bin, src/bin/bakeoff.rs (the fixture is outside the workspace)
    cargo build --release --lib --target wasm32-unknown-unknown --no-default-features --features rhai   # or luau

Versions: Rhai 1.27.0 (`sync`, `f32_float`, `serde`); mlua 0.11.6 with Luau
0.709 (`luau0-src` 0.18.3); `luau-analyze` from the Luau release of
2026-11-13.

## Results

| | Rhai | Luau |
|---|---|---|
| Outputs equal to the native sentry, byte for byte (2000 ticks, each carrying its own state) | **2000 of 2000**, after exact float comparisons are registered (below); 1999 before | 1968 of 2000: Luau's number is f64, so the bolt's direction differs by one ulp on 32 ticks. Deterministic between peers, not equal to the native f32 module |
| Cost per invocation (native 0.6 µs; the shared JSON binding alone 2.4 µs) | 10.2 µs: about 5 µs conversion and call (empty entry 5.2 µs), 5 µs the turret's work | 15.2 µs: 14.7 µs is conversion through mlua's serde (empty entry), **0.5 µs** the turret's work. A direct table binding would remove most of it |
| A syntax error | refused at load, line and column | refused at load, line |
| A misspelt variable | **refused at load** (`set_strict_variables`) | not seen until that line runs (here it did not run: "ran without complaint"); **refused by `luau-analyze`** |
| A misspelt field read, a misspelt field write, text where a number goes | at run time, with line (writes: by the host's record check) | at run time, with line; **all three refused by `luau-analyze --!strict`** with the typed `Inv` |
| An endless loop | stopped by `set_max_operations` | stopped by the interrupt callback |
| State a script keeps that the host does not hold (a replay of one tick gives another answer) | **impossible**: a function cannot see a mutable global (refused at load) | possible: a chunk-level `local` outlives the invocation. Running the chunk afresh per call removes it (+3.9 µs); the analyzer does not flag it |
| `wasm32-unknown-unknown` (the browser build) | **builds** (with getrandom's `wasm_js`, as the app has); not yet run in a browser | **does not build**: Luau is C++ and the target has no C++ standard library; mlua documents the browser only through emscripten, and Luau's errors need C++ exceptions or `longjmp`, which this target lacks |
| Embedding | pure Rust; `Engine` and `AST` are `Send + Sync` | a C++ build (about a minute); `Lua` is `Send`, not `Sync` (a mutex per backend) |

### Two findings a production adapter must keep

- **Rhai's float comparisons are tolerant by default**: within `f32::EPSILON`,
  absolute when either side is zero (`1.1e-8 > 0.0` is false, `0.1 + 0.2 ==
  0.3` is true). A cooldown of 1.1e-8 fired a tick early. The `unchecked`
  feature compares exactly but also drops the operation budget, so the
  adapter turns off the fast operators and registers exact comparisons
  (`rhai_backend::exact_float_comparisons`). Cost: included in the numbers
  above.
- **Luau's mutable chunk locals are hidden state.** A Luau adapter must run
  the chunk per invocation or refuse chunk-level mutable locals statically.

## Decision (provisional)

**Rhai for the first adapter.** It meets every requirement today: the browser
target builds, the arithmetic is the engine's (`f32`, exact comparisons), a
script cannot keep hidden state, and it embeds as pure Rust. Its weaknesses
are author feedback (most mistakes are run-time errors) and speed (about ten
times Luau's VM on the turret's work). The feedback gap is narrowed by the
audit's P2: port descriptions that the host checks at the boundary (a
misspelt field write is already refused there, naming the field) and that
generate the documentation an author reads.

**What would change it to Luau:** a Luau build that runs in the browser
inside the engine's `wasm32-unknown-unknown` app (a wasi-sdk C++ sysroot and
an exception strategy), or a decision to move the web build to emscripten.
Luau's analyzer is the strongest single property seen here for an LLM author:
with a typed interface it caught all four mistakes before anything ran. If
the web target is solved, Luau is the better author language, and its VM is
faster; its binding should then build tables directly, not through serde.

## Next

1. Rhai adapter in the extension host (audit P3). **Landed** as
   `crates/ambition_extension_rhai` (`RhaiModules`, a `ModuleBackend`): a
   script supplies entries of a module the game declares, falling back to the
   native code for an entry it does not define; exact comparisons, the
   operation budget, strict variables. The neutral binding moved into the SDK
   (`ambition_extension_sdk::script`: `ScriptValue`, and `ScriptPort`
   converters owned by each port's crate). The fixture's `engine` row runs
   it: 2000 of 2000 equal to native, 12.5 µs per invocation.
2. Port descriptions (P2) for the sentry's ports, generating the converters
   this fixture writes by hand, and the author-facing reference. **Landed:**
   `ambition_extension_sdk::structure` (a `Field` codec gives a value's
   shape, wire encoding and script form; `port_record!` declares a record
   once; `script_port!` makes a port script-reachable). The module entity
   tick, projectile spawn and body sound ports use it, with their published
   bytes unchanged; `request(alias, value)` checks a request at the call
   (section 3b of the fixture: a misspelt field, a wrong kind, a misspelt
   port and a missing field each name the line); the reference is generated
   ([script ports](../../sdk/script-ports.md)) and has a JSON form. The
   fixture's own Rhai row keeps the script as it was measured
   (`fixtures/scripting_bakeoff/modules/sentry.rhai`).
3. The sentry as a real loaded script in the running game, its rollback
   witness and its invocation-fault witness. **Landed:**
   `game/ambition_content_modules/rhai/sentry.rhai` (the fixture compares
   this file), loaded by `AMBITION_EXTENSION_MODULES` and reloaded on edit;
   a script's code identity is `rhai-f32-exact/1` and the digest of its
   text, so an edit is a new content generation and each fault names it.
   Witness: `game/ambition_app/tests/a_script_module_runs_without_a_build.rs`.
   P3's project entry (a manifest, no game Rust) remains.
4. Story sequences (SS1, SS2) as the first content customer of script-callable
   commands. **Landed** as authored commands and cutscene beats
   ([story sequences](story-sequences.md)); a script calling the same
   command catalog is not yet wired.
