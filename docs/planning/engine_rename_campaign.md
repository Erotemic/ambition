Rename the platformer-focused crate family so the repository more clearly communicates Ambition’s current center of gravity and its longer-term direction.

The main focus remains building an exceptional 2D platformer engine.
That vertical push should stay central: movement, combat, worlds, portals,
  actors, runtime composition, and presentation should become increasingly
  polished and coherent as a dedicated `platformer2d` stack.

At the same time, Ambition should leave clean seams for external contributors to
  extend the engine and for other styles of games to become possible in the
  future.
The goal is not to prematurely generalize every platformer subsystem.
It is to avoid claiming generic names for crates that are currently
  platformer-specific, while preserving genuinely general services such as
  content compilation, input, causal inspection, loading, assets, audio, and
  time.

This supports the longer-term ambition of becoming a real Unity or Godot
  competitor without weakening the immediate product focus.
Ambition can grow outward from a deep, high-quality 2D platformer engine rather
  than attempting to become shallowly universal from the beginning.

Proposed moves:

```text
ambition_engine_core
    → ambition_platformer2d_core

ambition_platformer_primitives
    → ambition_platformer2d_shared_tangle

ambition_world
    → ambition_platformer2d_world

ambition_ldtk_map
    → ambition_platformer2d_ldtk

ambition_portal
    → ambition_portal2d

ambition_portal_presentation
    → ambition_portal2d_presentation

ambition_runtime
    → ambition_platformer2d_runtime

ambition_host
    → ambition_platformer2d_host

ambition_platformer_provider
    → ambition_platformer2d_provider

ambition
    → ambition_platformer2d

ambition_actors
    → ambition_platformer2d_actor_monolith
```

`ambition_platformer2d_shared_tangle` deliberately acknowledges that the current shared
  layer is a catch-all with architectural smell.
It likely needs decomposition eventually, but that work is lower priority than
  the active architecture campaigns.
Defer it until the surrounding boundaries are more mature or a concrete problem
  makes decomposition the right solution.

`ambition_platformer2d_actor_monolith` is intentionally blunt.
The crate is already a major decomposition target, and the name should
  constantly remind agents that it is not the final architecture and should not
  become the default home for new reusable behavior.

Keep this rename thrust mostly mechanical:

* rename directories, packages, dependency keys, and Rust crate paths;
* update workspace manifests, scripts, fixtures, guards, and documentation;
* regenerate lockfiles;
* do not add compatibility aliases or shim crates;
* avoid unrelated refactors while performing the rename, only perform unavoidable ones.
