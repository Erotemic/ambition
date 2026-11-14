# First implementation task

This prompt is a bounded starting task. It does not authorize an automatic migration of every game mechanic.

```text
Work in the assigned Ambition checkout. Do not create or switch worktrees.
Read AGENTS.md and docs/reviewer-guide.md. The audit baseline is 34f03976b639
from 2026-11-14. Recheck all findings against the actual working HEAD before
editing. The audit is under docs/planning/engine/runtime-authoring-audit-2026-11-14.

Product goal: a prebuilt host runs game content and source scripts without
native compilation for ordinary edits. Use the current content compiler,
prepared generations, extension host, domain ports, and rollback authority.
Keep the engine Bevy-native. Keep the existing Rust-Wasm path as an optional
compiled-module route. Do not create another ECS, physics path, or save system.

First complete P1 from the execution plan:

1. Inspect run_game.sh's existing --print-plan and final Cargo invocation.
   Add an explicit reuse-only launch mode through that owner. It must report
   and run the selected existing binary/profile with the intended environment
   and arguments. If the binary is absent, explain the separate build/install
   action. Do not silently build it.

2. Inspect game/ambition_app/build.rs. Its build label currently watches all
   crates/game source trees, Git index/refs, and uses a current timestamp in
   a rustc-env value. Separate stable engine build identity from active
   content/module identity and optional runtime workspace status. Remove
   stamp-only invalidation outside actual build inputs. Preserve deliberate
   release IDs and real static-content embedding inputs.

3. Fix scripts/regen/build_extension_modules.sh so --watch for a selected
   package observes that package and its actual local source/build inputs.
   The current fixed Ambition list misses Mary-O and Smash module sources.
   Keep last-good behavior on a failed module build.

4. Demonstrate a runtime data edit and one selected Rust-module edit against
   an existing host. Distinguish no host build from the explicit module build.
   Report active native/loaded backend and generation. Use existing tooling.

Use the narrowest behavioral verification for each change. Do not add
permanent source-text tests, tests-of-tests, a feature Cartesian product,
or a new general policy framework. Before native builds, inspect the target
storage as AGENTS.md requires and coordinate the shared target.

Then prepare the first Rhai adapter task, without broad mechanic migration:
use the existing stateful sentry, generate selected port binding metadata
from the domain-owned values, and implement ModuleBackend with host-owned
records and fresh mutable invocation context. Rhai is the proposed first
candidate, not a measured runtime winner. Keep source/import/runtime/math
identity explicit. Do not let script closures or VM globals retain hidden
authoritative state.

Important source limits for later packets:
- actor_think is an AI controller hook; add separate attached-body behavior.
- the current state schema lacks first-class stable references/records/enums.
- only SessionDurable is an implemented extension save road in the baseline.
- submitted requests are not applied results; records commit before domain
  lowering, so pickup success must come from custody's result or observation.
- Primeval coconut carrying is coordinated and must retain native custody,
  stable ordering, cooperation, and ordinary control gates.
- shared collision-safe body transitions already exist. Wrap them; do not
  create a script-specific resize/teleport implementation.
- main Ambition already uses App-local Arc pack selection. The per-revision
  leak finding concerns EmbeddedPack publication used by SDK/demo reload.

Preserve settled improvements: disk content, Wasm execution/reload, extension
rollback state, one movement kernel, native portal geometry, source-owned
capabilities, Outlander's native SDK proof, and the existing authoring tools.
Do not split actor_monolith merely because of its name or size.

Update the existing owner plans with actual results and remaining work.
Record build, load, and observed behavior separately. A static inspection or
an unchanged binary hash alone does not establish an end-to-end latency claim.
```
