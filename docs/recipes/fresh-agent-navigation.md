---
status: current
last_verified: 2026-11-10
related_docs:
  - docs/concepts/engine-mental-model.md
  - .agent/README.md
---

# Fresh-agent navigation

The generated `.agent/` tree helps locate current source owners. Query it as a
navigation index; treat the user's task, present code and authoritative docs as
the sources of requirements and behavior.

## Start from the assigned checkout

```bash
cat AGENTS.md
python3 scripts/agent_query.py overview
python3 scripts/agent_query.py "<task words>"
```

Read the relevant [engine mental model](../concepts/engine-mental-model.md),
system or recipe guide, and the code that owns the requested behavior. Use
active planning for unfinished work. Old journals and reviews explain earlier
decisions but need not describe the current implementation.

For a narrower search:

```bash
python3 scripts/agent_query.py crate ambition_input
python3 scripts/agent_query.py ecs "RoomScope"
python3 scripts/agent_query.py docs "transactional room load"
python3 scripts/agent_query.py tests "portal gravity"
```

Once you know the module, `rg` or `git log -- <paths>` may be faster. Verify
whether a search hit is the owner, adapter, test fixture or consumer.

Work in the checkout assigned by the user. The repository's worktree policy
applies even when other agents share the directory. Identify the responsible
subsystem and implement the task without obligatory broad research or test runs.

When packaging a new agent source archive or after substantial structural
changes, regenerate the `.agent` index:

```bash
python3 scripts/generate_agent_index.py
```

Neither index generation nor broad verification is a default prerequisite for
every documentation edit or small feature change.
