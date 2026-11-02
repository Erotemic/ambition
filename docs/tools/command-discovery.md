# Find development commands

Run `python3 scripts/agent_query.py tools "<task>"` from any directory.
Run the reported commands from the repository root. For example:

```bash
python3 scripts/agent_query.py tools screenshot
python3 scripts/agent_query.py tools "tune a move"
python3 scripts/agent_query.py tools "edit an LDtk room"
python3 scripts/agent_query.py tools "inspect published assets"
python3 scripts/agent_query.py tools "inspect ECS systems"
```

Command descriptions live in `commands.json` beside each owner. The root
script descriptions live in `scripts/commands.json`. Each description records
the command, help, required source files, owner, documentation, inputs, outputs,
mutation policy, and build requirements. Add supported workflows here. Do not
list each internal script as a supported command.

Queries read these files directly. Missing source remains visible as missing.
Source presence does not prove that dependencies, content, or graphics support
are available. Existing binaries can be reused. Their presence does not prove
that they match current source.

Generate the review inventory without building Rust or generating assets:

```bash
python3 scripts/generate_agent_index.py --tools-only
```

The output is `.agent/index/tool_index.json`. It includes source revision,
commands, root script candidates, Cargo binaries, and consumers of proposed
move paths. Unclassified candidates have status `unknown`. Path risk fields
show relative root and import assumptions. Review each consumer before a move.
The inventory is a navigation aid. Source and owner documentation remain the
authority.

The ECS browser owns its builder and tests under
`tools/ecs_architecture_browser/`. The moveset inspector owns the take report,
diagnostic SVG renderer, and visualization coordinator under
`tools/ambition_moveset_inspector/`. Their commands remain independent of the
editor server and do not build Rust by default.
