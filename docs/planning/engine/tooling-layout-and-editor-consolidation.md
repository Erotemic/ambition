# Tooling layout, discovery, and Ambition Editor consolidation

**State:** COMPLETE. T0-T5 are implemented and verified.
The user selected all packets on 2026-11-02. The shared queue was not reorganized.
**Source snapshot:** `19c66394` (2026-11-02 archive).
**Scope:** Repository development and authoring tools, their paths, entry points, and discovery. No game or engine behavior change is intended.

**Receipts:** discovery and editor shell: `3e3bb72a27`, `61a8189823`;
package-owned inspection: `e0fac3518b`, `6820d91348`;
measurements: `0ee9e6ab2d`, `3318d174bf`; maintenance: `8c9d8fd92f`;
packaging: `c169ff5a37`; checks: `4309a73549`; final layout and cleanup: `1e898136d4`.
The final acceptance lane passed 290 tests, with four artifact-dependent skips.
All 53 absence contracts passed. Editor HTTP/JSON operations, CLI discovery,
LDtk metadata inspection, and Cargo target inventory checks passed. Asset bytes
and map Gitlinks match the baseline. The five existing broken documentation
links are outside this change.

Root scripts now have one operational category or a documented stable launcher
role. Owner tests, intent retrieval, CLI smoke checks, and a path-consumer audit
cover the moves. Renderer submodule paths must stay fixed.

## Layout decision

Use `tools/` for substantial author-time applications and packages. Keep all existing Git submodules in `tools/` at their existing paths. Grow the current Asset Inspector into one human-facing **Ambition Editor** under `tools/ambition_editor/`. Keep other specialized tool implementations independent. Use `scripts/` for small repository operations, grouped by purpose, with a few stable root entry points.

The editor must also serve LLM development. The same functions must be discoverable and callable without clicking through the editor. Do not build a second authoring authority, a universal tool runtime, or a new gameplay mutation service as part of this migration.

The implementation uses separate commits for tool ownership, measurements,
maintenance, packaging, checks, and final driver cleanup. Future moves must
check current consumers before a file is moved.

## Current ownership

The baseline had 257 direct files under scripts/. The implemented layout keeps
13 root launchers. The [script guide](../../../scripts/README.md) records their
roles and the operational categories.

- Ambition Editor owns asset inspection, previews, lineage, and authored review
  notes under tools/ambition_editor. The supported CLI remains
  scripts/asset_inspector.py. Generated pages remain under .agent/asset_inspector.
- The ECS browser owns its builder and tests under tools/ecs_architecture_browser.
- The moveset tool owns its server, reports, diagnostic images, timing proposals,
  and comparison-data fetch commands. Rust capture and simulation binaries remain
  in game/ambition_app_tools.
- The LDtk tool owns semantic world operations and tileset retarget proposals.
- Renderer submodule paths stay fixed. One sprite-renderer guide now names the
  new parent quality-check path. No renderer code or authored art changed.
- Owner command descriptions, README status, package entry points, script path
  risks and consumers, and explicit and implicit Cargo binaries appear in
  .agent/index/tool_index.json. The catalog is not execution authority.
- Completed schema migrations are removed. Audio experiments keep their source
  and review artifacts under scripts/experimental. Asset attribution and
  historical patch bytes stay unchanged.

## Rules for the target architecture

1. **Keep every submodule in `tools/`.** Do not rename, nest, vendor, or relocate the three renderer submodules. Honor the existing submodule policy and preserve their Gitlink identities. Other submodules under `dev/` and `game/` are also outside this reorganization.
2. **One primary editor.** Promote the Asset Inspector rather than adding a competing dashboard. Its first section remains asset inspection and review. Other specialist tools need not be absorbed into its process or dependency environment.
3. **One owner per operation.** The editor invokes the LDtk, moveset, renderer, capture, and content-publication owners. It does not reimplement their formats, simulations, geometry, publication rules, or provenance.
4. **Agent access is a first-class contract.** Every supported workflow has a CLI or structured invocation, discoverable purpose, primary command, input/output contract, mutation policy, and useful failure output. The UI calls or links to these operations. It must not be the only route.
5. **Do not turn discovery into authority.** A generated catalog describes real commands and schemas. It cannot declare a tool usable when its implementation or required capability is missing. Retain source revision and generation metadata where available.
6. **Keep iteration permissive.** Do not make normal inspection and previews depend on full-library hash checks, full regeneration, release publication, or Cargo builds when usable existing artifacts are available. Report staleness and missing provenance without hiding usable products. Use deep validation at deliberate publication boundaries.
7. **Keep the packages cohesive.** A substantial program with its own tests, dependencies, and domain model belongs in `tools/`. A short repo orchestration, check, measurement, or maintenance command belongs in `scripts/`. Frequency of use alone does not set the directory.
8. **Avoid long-lived duplicate entry points.** Preserve a small set of intentionally supported launchers. Change internal callers atomically. Remove redundant aliases when their consumers are updated rather than adding hundreds of forwarding shims.
9. **Protect concurrent work.** Do not mass-edit the shared planning queue. Do not overwrite generated or authored review state. Do not touch other agents' working trees or submodule branches.

Governing documents: [agent-native authoring](../../concepts/agent-native-authoring.md), [authoring and tools](authoring-and-tools.md), [inspection and workbench](inspection-diagnostics-and-workbench.md), [asset library, editor, and delivery](asset-library-editor-and-delivery.md), and [tool authoring policy](../../tools/tool-authoring-policy.md).

## Implemented layout

```text
tools/
  ambition_editor/               # The current Asset Inspector, promoted
    ...                         # Keep inspector index, web UI, tests and reviews
  ambition_moveset_inspector/    # Independent moveset tool and server
  ambition_ldtk_tools/          # Semantic LDtk editing package
  ecs_architecture_browser/     # Browser plus its owned builder, if moved
  ambition_background_renderer/
  optimization_report/
  vanity_card_prep/
  experimental/
  ambition_sprite2d_renderer/   # Git submodule: path fixed
  ambition_music_renderer/      # Git submodule: path fixed
  ambition_sfx_renderer/        # Git submodule: path fixed
  ...                           # Other existing tools stay by default

scripts/
  regen/                        # Existing repo-level generation drivers
  checks/                       # Repository checks and policy guards
  measure/                      # Profiling, measurement and reports
  maintenance/                  # Occasional cleanup and repair
  package/                      # Build/package/distribution orchestration
  setup/                        # Existing environment setup
  experimental/                 # Existing one-off experiments
  lib/                          # Shared script support
  tests/                        # Remain here initially
  agent_query.py                # Stable agent discovery entry point
  generate_agent_index.py       # Stable index generator
  run_tests.py                  # Stable main test entry point
  required_checks.py            # Stable main checks entry point
  asset_inspector.py            # Stable editor CLI entry point, if retained
  ...                           # Other justified root commands

game/ambition_app_tools/        # Existing Cargo binaries stay here
```

This is a direction, not a command to create empty folders. Decide which scripts to move by auditing their consumers first. Do not move Rust binaries to `tools/` just for directory symmetry; changing their Cargo ownership could undo the existing dev-build isolation.

### Editor responsibilities and non-responsibilities

- **Phase-one editor:** retain existing asset list, JSON inspection commands, preview pages, local `serve` workflow, generation status, current/old products, lineage, and review notes. Rename/promote the product without discarding accepted workflows.
- **Initial integration:** link to the moveset inspector, LDtk tooling, capture output, character authoring tools, generated music/SFX review, and ECS browser. The first version may use deep links, subprocess launch commands, and read-only tool descriptors. One common UI does **not** require one Python environment or server process.
- **Later integration, only where valuable:** cross-links from a selected character or asset to its generator, move, use sites, capture, room, and historical appearance; unified display of tool outputs and diagnostics. Favor navigation and presentation integration over merging internal implementations.
- **Excluded:** an alternative content catalog, a generic mutable registry of Bevy state, a second simulation harness, replacing the LDtk editor, copying renderer code out of submodules, or recreating source-of-truth metadata inside the editor.

Ambition Editor stores authored notes under `tools/ambition_editor/reviews/notes/`. A physical package move must preserve all notes, stable note IDs, provenance, internal links, and any scripts that look them up. Its generated `.agent/asset_inspector` outputs can remain under their current path until there is a demonstrated need to change that path. Do not require a generated-output rename merely to rename the tool.

## Operational ownership

| Candidate destination | Current examples | Ownership test |
| --- | --- | --- |
| `scripts/checks/` | `check_absence_contracts.py`, `check_rollback_kind_spelled_once.py`, `check_doc_links.py`, `check_regeneration_coverage.py` | Is it a standalone repo-wide guard rather than a test internal to one tool? |
| `scripts/measure/` | `measure_kernel_module_graph.py`, `compile_cost.py`, `profile_desktop.sh`, `architecture_census.py` | Does it produce measurements or reports for the repository? |
| `scripts/maintenance/` | `git_debloat.py`, `prune_profile_bundles.py`, `sweep_target.py` | Is it an infrequent cleanup or repair action with explicit safety checks? |
| `scripts/package/` | `package_single_exe.py`, `package_outlander.py`, `web_bundle.py` | Does it assemble or deliver an application artifact? |
| `scripts/regen/` | Repository producer coordination, including `generate_library_content.py` | Does it orchestrate producers without owning their generation algorithms? |
| Existing tool packages | `ecs_browser.py`, `moveset_report.py`, `moveset_visualizations.py`, `render_take_diagnostic.py` | Does it implement a tool-specific operation better owned and tested beside that tool? |

Do not move a check into `regen` solely because regeneration calls it. A producer implementation belongs with its owning tool; a repository-level generation coordinator may remain in `scripts/regen`. Review root-level one-off migrations under `tools/` separately: the top level of `tools/` should not become another dumping ground.

## Implementation packets

### T0. Produce a command and path-consumer inventory

**Precondition:** none. **Status:** implemented. See [command discovery](../../tools/command-discovery.md).

1. Enumerate actual supported commands from the root scripts, tool READMEs, package entry points, `game/ambition_app_tools/Cargo.toml`, and available command help. Separate supported, internal, experimental, archived, and unknown.
2. For proposed moves, identify references in Python imports, shell scripts, tests, Cargo config, CI workflows, documentation, and generated indexes. Detect `__file__`-relative repository-root computations, `sys.path` assumptions, working-directory dependencies, and references to authored data under old tool paths.
3. Record the owning package, canonical invocation, input/output side effects, build requirement, and whether a binary can be reused. Build a small machine-readable report that a reviewer can inspect. Do not require a complex persistent inventory database.

**Done when:** an agent can identify all first-class screenshot, moveset, LDtk, asset-inspection, capture, and diagnostic workflows and their canonical entry points without broad repository search. The output identifies risky path consumers before the first move.

### T1. Improve agent tool discovery before moving tools

**Depends on:** T0. **Status:** implemented. Intent retrieval and missing-source tests cover the command catalog.

Extend `generate_agent_index.py` and/or its focused helpers to enumerate a curated set of supported commands across root scripts, tools, and Cargo binaries. Preserve `scripts/agent_query.py` as the agent entry point. Search by developer intent, not only package directory name. Each result should provide a usable example, help command, owner, build/reuse requirement, relevant output, and link to authoritative documentation. Prefer reading commands from tool-local descriptors or existing definitions. Do not maintain an exhaustive hand-written second command registry. Keep generation fast and the catalog non-authoritative.

**Done when:** representative queries such as screenshot, deterministic capture, tune a move, edit an LDtk room, inspect published assets, and inspect ECS systems route to supported commands. Add focused tests for those retrieval cases and for stale or absent tool records. No GUI is required for this acceptance.

### T2. Promote the existing Asset Inspector to Ambition Editor

**Depends on:** TOOL-DISCOVERY (T0-T1). **Status:** implemented. The editor Tools page uses the owner command catalog.

Start with the existing inspector implementation. Retain the asset browsing and review workflows as the first editor area. Introduce a coherent name and navigation shell. Preserve stable review data, JSON CLI operations, selected library behavior, cached previews, and current `serve` semantics. Integrate other tools first by links or command invocation, not by moving their core code. The existing `scripts/asset_inspector.py` can remain the small supported CLI launcher if it is useful; retire the `art_inspector.py` alias when all call sites are migrated. Inspect compatibility needs before choosing the final rename operation.

**Done when:** users can launch one editor and navigate from assets to at least one other existing inspection capability; agents can obtain the same asset facts and invoke the integrated tool without browser automation. No asset, provenance, or published-selection authority is duplicated.

### T3. Move package-owned tool implementations, one owner at a time

**Depends on:** T0 and the relevant owners' tests. **Status:** implemented.
The ECS browser and three moveset commands now live with their tool owners.
Their tests and command descriptions moved with them. Owner verification
covers the report, SVG, browser, and retarget behavior. Tests that need published
art run when that artifact exists.

Evaluate `tools/ecs_architecture_browser/ecs_browser.py` for relocation beside `tools/ecs_architecture_browser`. Evaluate the three named moveset scripts against the moveset inspector and Rust take-rendering binaries. Move a script only if the destination can own its tests, entry point, and documentation. Update all direct consumers as part of the same move. Avoid creating a second backend in the editor.

**Done when:** each migrated tool passes its focused smoke path and its command is found through T1. A script left at the root has a documented ownership reason, not simply an incomplete migration.

### T4. Split root scripts into small operational batches

**Depends on:** T0; T1 recommended. **Status:** implemented.
Repository operations now use checks, measure, maintenance, package, and regen.
The [script guide](../../../scripts/README.md) records all 13 stable root roles.

Start with low-risk leaf scripts and one category at a time. Suggested order: a small group of measurement scripts, then isolated maintenance utilities, then packaging drivers, then checks with many references. Treat the order as a hypothesis to verify against T0. Keep `scripts/regen/`, `scripts/setup/`, `scripts/lib/`, and `scripts/tests/` intact during the first batches. Keep a short list of discoverable root commands, especially `agent_query.py`, `generate_agent_index.py`, `run_tests.py`, and `required_checks.py`.

For each moved script, update repo-root path resolution, imports, test expectations, CI, shell calls, docs, and agent metadata in the same change. Run focused tests after each batch. Do not generate hundreds of root compatibility wrappers. If a command is a durable external entry point, use one intentional wrapper; otherwise update its callers and remove the old path.

**Done when:** the moved category is clearly organized, all known supported entry points work from the repository root and from another working directory where applicable, and agent discovery uses the new canonical path.

### T5. Remove obsolete and duplicate launch paths

**Depends on:** T2-T4 as applicable. **Status:** implemented.
The redundant inspector alias and three completed schema migrations are removed.
Audio experiments retain their source and review files under scripts/experimental.
Historical producer references inside asset files stay unchanged to preserve
asset bytes and content hashes.

Review unowned one-shot migrations, duplicate aliases, stale documentation, experimental scripts, and wrappers. Check active references before deletion. Keep unique development capabilities even when use is infrequent. Do not delete a tool only because a usage count is zero. Remove an alias when the canonical path is established and its consumers have moved.

**Done when:** each remaining root script has a clear operational purpose or stable launcher role. Obsolete paths are not represented as supported commands by agent discovery.

## Minimal acceptance and regression checks

For every implementation packet, use the smallest relevant tests. A file move should not trigger a full game recompilation or a whole-library regeneration.

- Compare Gitlinks and `.gitmodules` with the baseline. All authoring renderer submodule paths stay fixed and their recorded commits change only through the normal submodule process.
- Confirm representative `agent_query.py` searches point to the canonical tool and runnable help/invocations.
- Exercise the Asset Inspector's `status --json`, a focused `inspect`/`list --json`, its offline build/serve entry point where feasible, and its review-note path. Do not modify real review notes in a smoke test.
- Exercise moveset inspector help/launch discovery, an LDtk read-only operation, and capture-tool discovery. Reuse built binaries when present. Do not require capturing a rendered scene for a pure script relocation.
- Run only the affected script/tool test targets plus relevant path/document policy checks. Use an assembled app build only if a changed Rust boundary requires one.
- Verify moved commands work independently of the current shell directory and do not assume old `__file__` depth. Inspect error reporting and exit codes, not just a success screenshot.
- Do not change physics, content publication rules, game state, or asset bytes as part of directory moves.

## Decisions and non-goals

The implementation keeps the proposed defaults: Ambition Editor is the product
name; the internal asset_index package remains; command descriptions cover
supported workflows; the script guide names stable root paths; tool packages own
their operations and tests; and editor integration uses navigation and commands.
The design questions below remain criteria for future changes:

1. Is `ambition_editor` the right final product/package name, with the current Asset Inspector promoted into it, or should the existing Python package name remain internal for now? Default recommendation: new top-level tool name, minimal internal renaming.
2. Which tool workflows deserve first-class command descriptors, and which are internal diagnostics? Default: curate supported workflows first; do not catalog every experimental script as public.
3. Which root script paths are intentionally stable interfaces? Default: only the primary agent/test/index launchers and a small number of justified compatibility entry points.
4. Which scripts actually belong to specialized tool packages? Default: move only after confirming semantic ownership and tests.
5. How much editor integration is useful now? Default: navigable shell and links; defer unified editing, cross-process state, and consolidated servers until a specific workflow needs them.
6. What work can be separated into small, nonconflicting patches while other agents edit the repository? Default: discovery first, then editor shell, then one script family at a time.

**Out of scope:** new game mechanics, engine backend changes, a universal authoring data format, moving Rust instrument binaries across Cargo crates without a build rationale, replacing LDtk, relocating submodules, merging all Python environments, large profiling campaigns, and editing the shared planning queue in this overlay.

**Success criterion:** an LLM agent can ask how to perform a supported platformer authoring or inspection task and receive one reliable executable route. A human can use Ambition Editor as the central browsing and review hub. The actual work stays with the domain tool that owns it, and common iteration does not gain new rebuild or validation requirements.
