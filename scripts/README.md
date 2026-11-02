# Repository commands

Use `python3 scripts/agent_query.py tools "<task>"` to find supported commands.
The command catalog reads owner descriptions from the current source tree.
Generate its path-consumer inventory with
`python3 scripts/generate_agent_index.py --tools-only`.

## Operational owners

| Directory | Responsibility |
| --- | --- |
| [checks](checks/README.md) | Repository checks, policy findings, and check selection helpers. |
| [measure](measure/README.md) | Source analysis, profiling, measurements, and reports. |
| [maintenance](maintenance/README.md) | Archives, pruning, cleanup, and repository repair. |
| [package](package/README.md) | Application build, package, and delivery coordination. |
| [regen](regen/README.md) | Generated-content coordination and publication drivers. |
| setup | Host, tool environment, submodule, and target-volume setup. |
| experimental | Unpromoted experiments and their review artifacts. |
| lib | Shared script support. These modules are not duplicate CLI owners. |
| tests | Tests for repository operations. Tool-specific tests live with their tools. |
| baselines | Authored inputs to the existing checks. |
| web | Frontend support used by repository commands. |

## Stable root entry points

| Command | Reason for the root path |
| --- | --- |
| agent_query.py | Main agent discovery interface. |
| generate_agent_index.py | Main navigation-index generator. |
| ecs_inventory.py | Source ECS inventory generator used by navigation. The browser consumes this index. |
| run_tests.py | Main test runner. |
| required_checks.py | Main check selector for a source change. |
| last_test_run.py | Read the main runner's recorded result. |
| asset_inspector.py | Supported Ambition Editor CLI launcher. |
| content_status.py | Fast content-readiness query without a renderer or build. |
| chargen_studio.sh | Supported launcher for the independent character-authoring UI. |
| agent_worktree.sh | Worktree coordinator interface. |
| mirror_assets_for_worktree.py | Prepare generated assets for a worktree. |
| goal_guard.py | Goal-state CLI used by agents and hooks. |
| goal_guard_hook.sh | Installed hook entry point for the goal-state CLI. |

The categories contain the implementation. Do not add a root forwarding script
for each moved command. Update its callers and owner command description.

Use the LDtk tool's semantic schema operations for new field changes.
Completed one-shot schema migrations are retained in Git history.
