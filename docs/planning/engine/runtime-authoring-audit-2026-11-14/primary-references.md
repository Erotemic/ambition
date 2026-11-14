# Primary references

These references were consulted for the audit on 2026-11-14. They support facts about the candidate languages, Cargo, and Godot. They do not establish that a candidate is integrated into Ambition, that it is deterministic under Ambition's rules, or that it meets a performance target. Repository source supplies all claims about the supplied engine snapshot.

## Cargo

- [Cargo build scripts](https://doc.rust-lang.org/cargo/reference/build-scripts.html): `rerun-if-changed` input tracking and `rustc-env` outputs. This supports the interpretation of the app build-label script. The audit did not time the cost of its broad watch set.

## Rhai

- [Rhai Book](https://rhai.rs/book/): embedding and language overview. Rhai is the selected first adapter candidate, subject to the existing engine's acceptance requirements.
- [Feature flags](https://rhai.rs/book/start/features.html): configuration including `sync`, `f32_float`, and `no_time`. The current documentation also describes optional Grain bytecode support; the audit does not characterize Rhai as exclusively AST interpretation.
- [WebAssembly builds](https://rhai.rs/book/start/builds/wasm.html): documented Wasm/browser use and the need for an appropriate module resolver. This is integration evidence for a prototype choice, not proof that Ambition's browser package already loads scripts.
- [Scopes](https://rhai.rs/book/engine/scope.html): scope lifetime and reuse. The recommended engine adapter uses fresh mutable invocation context and host-owned persistent records. It must also exclude hidden mutable state in captured values and imported modules.
- [Calling script functions](https://rhai.rs/book/engine/call-fn.html): function invocation, AST top-level evaluation, and invocation options. The engine must define its initialization and import-generation rules explicitly.
- [Operation limits](https://rhai.rs/book/safety/max-operations.html): interpreter operation budgeting. A Rust function call counts as one operation, so expensive host queries need their own cardinality/work limits.
- [Function metadata](https://rhai.rs/book/engine/metadata/index.html): available engine metadata facilities. This does not replace the missing structural descriptions for Ambition's domain port values.
- [Custom types](https://rhai.rs/book/rust/custom-types.html): Rust-bound values and registration. Opaque exact engine references can be exposed through a binding; their identity and lifetime rules must still come from the domain owner.

## Luau and Lua

- [Luau types](https://luau.org/types/): gradual typing and strict checking. This is a specific advantage for early author feedback; no comparative LLM success rate was measured.
- [Luau repository and embedding guidance](https://github.com/luau-lang/luau/blob/master/README.md): CLI tools, analyzer, C++ implementation, and embedding through source-to-bytecode compilation and loading. Internal preparation can satisfy the requested author workflow without an external native build per edit.
- [Luau C API](https://luau.org/api/): runtime and embedding API reference.
- [Luau sandbox guidance](https://luau.org/sandbox/): embedding controls and limits. Ambition's host-owned effects and records remain necessary independently of VM isolation.
- [Luau performance notes](https://luau.org/performance): interpreter and optional optimization design. No values from these notes are used as a benchmark for Ambition.
- [mlua README](https://github.com/mlua-rs/mlua): the common Rust binding documents WebAssembly support through `wasm32-unknown-emscripten`. Ambition's reviewed browser target is `wasm32-unknown-unknown`, so that integration requires a concrete target spike. This is not a claim that every possible Luau integration with the target is impossible.
- [Lua 5.4 reference manual](https://www.lua.org/manual/5.4/manual.html): language types, integer/float distinction, state, and embedding. Lua was considered as a baseline, not proposed as a second language to support alongside the first candidate.

## Godot

- [Godot command-line tutorial](https://docs.godotengine.org/en/stable/tutorials/editor/command_line_tutorial.html): project execution, scripts, headless operation, parse checks, and export. These capabilities mean that merely providing a CLI is not a competitive distinction. The proposed comparison is a complete game-authoring task with the same inputs and a stated model/tool budget.
- [Godot frequently asked questions](https://docs.godotengine.org/en/stable/about/faq.html): supported scripting choices and platform considerations. The audit does not score editor feature parity or claim an observed performance/quality win over Godot.

## Repository evidence owners

The relevant existing plans include `extension-model.md`, `extension-domain-contracts.md`, `extension-state-and-execution.md`, `content-generation-and-reload.md`, `actor-effects-and-modifier-composition.md`, `capability-and-runtime-composition.md`, `public-sdk-1.0.md`, `authoring-and-tools.md`, and `project-build-and-distribution.md` in the parent directory.

`extension-iteration-evidence.md` contains historical iteration receipts. Their measurements are useful background, but are not current benchmark results from this audit. `queue.md` remains the implementation ledger. This dated review adds findings and an execution proposal without replacing those owners.
