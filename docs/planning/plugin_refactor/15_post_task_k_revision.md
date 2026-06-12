# Post-Task-K revision — compact historical summary

**Status:** landed/superseded. The runtime-substance pass described here became the Stage 16 platformer-runtime extraction and later Stage 20 bisection.

## Lesson

The first `ambition_platformer_runtime` extraction was intentionally a seed crate. That was better than forcing adapters around an unstable center, but it meant the next work had to add real runtime substance before more adapter crates could pay off.

## What this revision changed

It delayed broad adapter-crate extraction until the runtime owned stable vocabulary:

- body / kinematic components;
- world-query and raycast helpers;
- gravity core;
- orientation / roll behavior;
- portal vector math and generic transit pieces.

Those pieces now live in foundation crates or their facades. The remaining active question is no longer “how do we finish Task K?” but “which `ambition_sandbox` modules are still worth moving up/down?”, tracked in `22_monolith_breaker_survey.md`.
