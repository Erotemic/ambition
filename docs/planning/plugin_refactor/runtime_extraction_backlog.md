# Runtime extraction backlog

**Status:** empty.

The proto-runtime remainder has been extracted. Modules under the sandbox's historical `platformer_runtime` path are facades/adapters rather than not-yet-moved runtime code.

Current runtime homes:

- `ambition_platformer_runtime::world_query`
- `ambition_platformer_runtime::body`
- `ambition_platformer_runtime::gravity`
- `ambition_platformer_runtime::orientation`
- `ambition_platformer_runtime::math`
- `ambition_platformer_runtime::transit`
- `ambition_platformer_runtime::projectile`

When future runtime candidates appear, add entries here only if a sandbox-local module is intentionally waiting on a specific dependency inversion. Otherwise use `22_monolith_breaker_survey.md` for broader breakup planning.
