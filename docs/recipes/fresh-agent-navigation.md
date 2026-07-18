# Fresh-agent repository navigation

Use this procedure before broad source exploration. The repository ships a
commit-matched generated navigation bundle under `.agent/` whenever
`./archive_agent_source.sh` creates an agent archive.

The generated bundle is a localization aid, not architectural authority. It
helps answer where to look; current source and current planning/ADR documents
still decide what is true.

## Progressive drill-down

### Level 0: orient

Read:

1. `AGENTS.md`
2. `.agent/README.md`
3. `docs/planning/README.md`

`.agent/README.md` records the archived commit, available indexes, generated
inventory counts, and the shortest query commands.

### Level 1: ask for a task packet

Start with the user's own wording:

```bash
python scripts/agent_query.py "room transition loading"
python scripts/agent_query.py "ground contact landing SFX"
```

The default task query returns a compact ranked packet of likely documents,
source files, symbols, ECS registrations, and tests. It is intended to replace
repository-wide exploratory reading, not targeted source inspection.

### Level 2: narrow by evidence type

```bash
python scripts/agent_query.py symbol GroundContactTransition
python scripts/agent_query.py docs "transactional construction"
python scripts/agent_query.py ecs "room transition" --crate ambition_app
python scripts/agent_query.py tests "ground contact"
python scripts/agent_query.py crate ambition_runtime
python scripts/agent_query.py path game/ambition_app/src/app/world_flow/room_transition_loading.rs
```

Use `crate` when a task clearly belongs to one package. The generated crate
packet combines its files, symbols, tests, module map, and ECS inventory paths.

### Level 3: inspect the generated shards

The main generated drill-down surfaces are:

- `.agent/index/catalog.json` — small repository overview and index directory.
- `.agent/index/crates/index.json` — package list and per-package counts.
- `.agent/index/crates/<crate>.json` — one package's files, symbols, tests,
  module map, and ECS inventory references.
- `.agent/ecs_inventory/project.md` — project-wide Bevy/ECS summary.
- `.agent/ecs_inventory/crates/<crate>.md` — readable per-package ECS inventory.
- `.agent/index/symbol_index.json` and `.agent/index/test_map.json` — complete
  flat indexes when a machine consumer needs the full corpus.

Prefer a crate shard over loading a whole flat index into context.

### Level 4: read source and focused authority

After localization:

1. inspect the defining source and its callers;
2. read the crate's `MODULES.md`;
3. read one focused active plan, ADR, concept, system doc, or recipe;
4. search `dev/journals` and `dev/benchmark-candidates` for prior failure modes.

Do not infer ownership from a facade import alone. Confirm where a type is
defined and which layer is allowed to mutate it.

## When generated data and source disagree

Generated navigation describes the commit recorded in `.agent/manifest.yaml`.
Treat disagreement as one of these cases:

- the archive was generated from another commit;
- the live working tree has uncommitted changes;
- the generator missed a Rust or Bevy pattern;
- a hand-maintained document is stale.

In every case, source wins for current implementation fact. Active planning and
ADRs win for intended direction. Generated indexes never justify preserving a
legacy path that current architecture is replacing.

## Query strategy

Use three passes rather than one enormous query:

1. **Symptom or requested outcome** — the user's words.
2. **Likely subsystem** — loading, collision, snapshot, input, animation, etc.
3. **Failure class or invariant** — duplicate authority, stale derived state,
   scheduling, transactionality, identity, reconstruction, or presentation.

Then inspect only the highest-ranked packet and direct neighbors.

## Refreshing the bundle

For a live checkout:

```bash
python scripts/generate_agent_index.py
python scripts/ecs_inventory.py --workspace --out-dir .agent/ecs_inventory
python scripts/agent_query.py build-catalog
```

`./archive_agent_source.sh` runs these steps in the staged committed checkout and
packages the results automatically.
