---
status: current
last_verified: 2026-07-18
related_docs:
  - docs/concepts/engine-mental-model.md
  - .agent/README.md
---

# Fresh-agent navigation

The generated `.agent/` tree is a commit-matched localization index. It is not a
second source tree and should normally be queried, not read wholesale.

## Ten-minute protocol

From the repository root:

```bash
sed -n '1,220p' README.md
sed -n '1,260p' AGENTS.md
sed -n '1,220p' .agent/README.md
python scripts/agent_query.py overview
python scripts/agent_query.py "<the user's exact task words>"
```

Then read:

1. [`../concepts/engine-mental-model.md`](../concepts/engine-mental-model.md);
2. [`../planning/vision.md`](../planning/vision.md) and the relevant track;
3. one likely crate packet and its `MODULES.md`;
4. one focused concept/system/recipe;
5. the current source owner and narrowest tests.

Do not begin by recursively reading `docs/`, `.agent/`, or a large crate.

## Query progression

```bash
# Broad task packet: docs, files, symbols, ECS, tests.
python scripts/agent_query.py "portal transition arrival"

# Current package role and generated module packet.
python scripts/agent_query.py crate ambition_portal

# Durable or current prose mentioning an invariant.
python scripts/agent_query.py docs "transactional room load"

# Bevy ownership: resources, messages, systems, registrations, spawn evidence.
python scripts/agent_query.py ecs "RoomScope"

# Tests before implementation search.
python scripts/agent_query.py tests "portal gravity"
```

If a query is noisy, use the most distinctive domain noun plus the observable
behavior. Prefer `"action scheme prompt"` over `"input"`, and
`"loading transaction commit"` over `"room"`.

## Evidence order

- Current user request: task intent.
- Active planning/ADRs: intended direction.
- Concepts: durable vocabulary and invariants.
- Source/manifests/tests: implementation fact.
- `.agent`: localization evidence generated from that source.
- `dev/journals` and benchmark candidates: failure history and traps.
- Git history/archive: why something changed, not present authority.

Confirm generated hits in source before editing. A symbol in an index may still
be an adapter, compatibility export, test-only item, or presentation consumer.

## Before changing code

```bash
rg -n "<subsystem>|<symptom>" dev/journals dev/benchmark-candidates
python scripts/agent_query.py tests "<invariant>"
git log --oneline --all -- <likely paths>
```

Ask:

- Which layer owns authority?
- Is this named provider content or reusable capability?
- Is there already a player/enemy, human/brain, runtime/headless, or
  simulation/presentation fork to remove?
- What lifecycle scope owns the state?
- What is the narrowest property that proves the change?

## After changing structure or docs

```bash
python scripts/check_doc_links.py
python scripts/generate_agent_index.py
./run_tests.sh --list
```

Run the focused tests returned by `agent_query.py`, then the broadest affordable
headless suite. Generated `.agent` data should match the commit packaged for the
next agent archive.
