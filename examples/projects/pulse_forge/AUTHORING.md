# Author a mechanic, then choose its implementation

Pulse Forge has two authored mechanics: a charge-and-release emitter and a
group patrol. Start with the emitter's [Rhai source](modules/pulse_forge.rhai).
Its [portable Rust source](portable/src/lib.rs) implements the same contract.
Neither implementation owns physics, cost admission, or damage.

Use the [standalone commands](STANDALONE.md) to create an independent game
directory, inspect it, and start it with a prebuilt runtime. That guide covers
all five build and delivery steps. The [example reference](README.md) gives
the commands within the SDK checkout.

1. **Declare and bind the mechanic.** `declare()` names the provider, module,
   entries, phases, trigger selectors, state schemas, required ports, and work
   limits. The character row in `content/characters.ron` binds the
   `pulse_forge` technique to Projectile. The world uses that character row.
   The module key and binding stay the same through promotion.
2. **Keep state in records.** The body's charge and each emitter's cadence
   have separate records. Field tags identify saved values across a rename
   or field reorder. Use `this.state` in Rhai and `record!` plus `load/store`
   in Rust. Mutable script globals and native or WASM statics cannot hold
   gameplay state. `this.dt` is the scope's gameplay time in seconds;
   `this.tick` is the logical simulation tick.
3. **Request domain effects.** `this.trigger` contains the domain's input.
   `this.requests.push(request("spend_mana", #{ amount: ... }))` asks the
   resource owner to pay before the spawn request applies. Insufficient mana
   refuses the full invocation's effects. A script fault commits no writes
   or requests. Rust uses the corresponding typed invocation ports. The
   [generated reference](../../../docs/sdk/script-ports.md) gives fields,
   numeric kinds, coordinates, defaults, and bounds. Inspection lists the
   ports and phases installed in the actual runtime.
4. **Edit and reload.** Change `BOLT_SPEED` in the selected Rhai file. The
   next admitted generation changes later pulses without a Cargo command.
   Add a script with `declare()` to add a module. Remove an unselected module
   file to remove it. A removed explicitly selected file refuses the candidate.
   Related script and content edits publish together at the mechanical-edit
   boundary. A syntax error or invalid content keeps the last good generation;
   fix the reported file to retry. Some edits reconstruct the session and
   reset its rollback history. Arbitrary live state migration is not supported.
5. **Check the contract before promotion.** Compile `portable` with the
   module builder, then run `extension_inspect --project ... --compare ...`.
   Semantic comparison excludes source and binary identity. A mismatch names
   the declaration field. Keep state tags, entry order, ports, selectors, and
   limits consistent. `project-wasm.ron` selects the resulting file for the
   emitter and keeps the patrol in Rhai. The same stock runtime runs both.
6. **Link for release when useful.** `project-linked.ron` selects `Linked`.
   Build the example host, which registers the same portable module factory.
   This step compiles and links an executable. It is a separate build from
   the module-only WASM loop. Use `CARGO_INCREMENTAL=0` for optimized builds.
   The host reports the running executable's build digest on desktop Linux.
7. **Verify behavior and cost.** Contract equality alone does not prove parity
   or speed. Run the checks below. The parity fixture compares ordered
   requests, records, outcomes, body identities, and populated rollback. The
   negative case must detect a changed speed. The profiling recipe measures
   the same inputs on all three backends. Choose from those measurements.

```bash
cargo test -p pulse_forge_host --test promotion -- --test-threads=1
cargo test -p pulse_forge_host --test project_state -- --test-threads=1
python3 examples/projects/pulse_forge/profile.py
python3 examples/projects/pulse_forge/measure_edits.py
```

Run these commands from the SDK checkout after the module build in the
standalone guide or example reference. The edit measurement temporarily changes
the example's speed constant and restores it. It measures a restart through
the first changed projectile, not drawn-frame readiness. The detailed
[performance report](PERFORMANCE.md) states the profile, counters, and limits.

The [durable counter recipe](recipes/progress.rhai) uses an authored body's
stable identity for save/load. Transient charge, emitter, and patrol state
stay out of the save. Live and saved schemas refuse a changed kind on an
existing tag, attachment, or save policy. Use a new schema key for new state.
Unknown unloaded module rows stay in the save. The current home avatar has no
durable construction identity and does not use this body save road.

The portable API does not expose raw ECS state, files, clocks, networking, or
new engine services. WASM imports are refused; each invocation gets a fresh
instance. Domain observations have their own bounded scope and stable order.
A missing installed port needs a runtime that provides its adapter, or a
module declaration that uses an available capability. An unknown script alias
needs a described binding in the engine. A duplicate module needs an explicit
implementation selection or removal of the competing file. A missing WASM
target needs `rustup target add wasm32-unknown-unknown` on the build machine.

The [desktop package check](STANDALONE.md) verifies bytes, identity, and file
reads outside the SDK checkout. It does not prove pixel rendering. Browser and
Android project loaders and native build identity remain unavailable in this
workflow. The shared asset library is packaged whole; asset reduction remains
open.
