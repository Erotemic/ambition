# Agent-native authoring and tools

**State:** open.
**Doctrine:** [agent-native authoring](../../concepts/agent-native-authoring.md).
**Execution:** [the queue](../queue.md); technique admission/bounds packets A11/A12
are in [the frontier](actor-monolith-work-frontier.md). The
[authored technique protocol](authored-technique-admission.md) owns their exact
support declaration, visitor, graph limits, clock semantics and activation gate.

## Goal

An author, including an LLM agent, can discover the engine's supported vocabulary,
inspect current content, change intent-level source, prepare/validate the result,
run a useful test, obtain a focused review artifact and explicitly publish/package
it. No visual editor operation or knowledge of monolith migration history is a
prerequisite. Optional visual tools consume the same authored semantics.

The authoring system is a control plane. Preparation and publication feed the
simulation; the author's model calls, filesystem operations and nondeterministic
planning are not part of a deterministic simulation tick.

## Current implementation: preserve these surfaces

The repository already has LDtk semantic inspection/editing/validation, world
queries and review bundles; procedural sprite authoring and published metadata;
MusicIR/SFXIR source and render diagnostics; provider-defined preparation and
public app builders. Use those surfaces rather than build another uniform CLI or
serialization format. See the root README and `docs/tools/index.md` for tool and
submodule ownership.

`TechniqueFlow` is implemented: `game/ambition_demo_smash/src/moveset.rs`
assigns a flow to `read_and_seize`, and `crates/ambition_combat/src/moveset/mod.rs`
interprets it. Installed technique support (`TechniqueSupport`), the exhaustive
effect-reference walk and checked acyclic flows are landed; the
[authored technique protocol](authored-technique-admission.md) owns their rules.

## A1 - capability discovery

Domain owners expose list/describe/schema/support/diagnostic projections of their
actual installed or prepared vocabulary. Required categories include character
and action definitions, technique support, world entities/fields, sprite products,
music/SFX constructs and provider extensions.

A discovery record distinguishes known, installed, supported for this profile,
prepared and ready. It includes owner, schema/version, source/provenance and
relevant prerequisites. Do not advertise a schema as executable behavior. Do not
build a generic runtime service locator to implement a read-only catalog.

## A2 - semantic inspection before mutation

Inspection answers what exists, which source owns it, what it resolves to, which
capability executes it and which other definitions reference it. Runtime inspection
names the session/content revision/frame and separates authoritative values,
derived views and presentation projections. A value with no selected owner is
reported as unresolved, not a default from some other profile.

Reuse domain preparation/reference enumeration for both forward and reverse
queries. A parallel hand-maintained dependency graph would become another source
of truth. Stale generated indexes must identify their source revision, tool
version and generation status; regeneration failure is visible to the consumer.

## A3 - intent-level mutation and reviewable plans

Direct edits are appropriate for simple source formats. Fragile formats such as
LDtk use semantic operations for relationships, IDs and placements. A plan states
base revision/content identity, affected definitions, proposed operations, expected
references and validation/acceptance before applying changes.

Reject an apply against a changed base rather than overwriting concurrent edits.
Repeat application should be explicitly idempotent or return an already-applied
result. Multi-file edits need a staged/recoverable application protocol; do not
promise filesystem-wide atomicity because each individual file can be renamed
atomically. Publication of an accepted content revision is a separate operation
from editing source files.

The review artifact contains the semantic diff and validation result, not a
second manually curated copy of all source. Include source edits in the normal
version-control workflow; generated products carry provenance to those sources.

## A4 - preparation diagnostics and admission

Preparation rejects unknown/uninstalled references, conflicting definitions,
invalid parameter domains, impossible structural relations and known unsupported
nondefault fields. A diagnostic should identify:

```text
code and severity
provider / definition / field or node
source location or structured source path
selected profile and content revision
expected semantic kind / available support
actual value and repair guidance
```

This is an output contract, not a mandate to replace every domain's internal error
type. Compose existing diagnostics at the preparation boundary. Source spans are
used where available; compiled Rust-authored content may need owner/definition/
field provenance rather than an invented file offset.

Delete an authored field that nothing plans to read. Use an unsupported-field
diagnostic only for a field the engine intends to support. The Q63 ruling
(2026-10-03) wants a facing gate, per-chest persistence and physical-pickup
persistence, and defers the breakable debris cue; each comes back as a field
and its consumer in one change. Per-chest and pickup persistence are
occurrence-ledger rows (`Spent`, `Consumed`), not authored fields; a chest
authored open (`ChestSpec::opened`, Q105) lowers into the same `Opened`
marker. The facing gate is `InteractableSpec::requires_facing` with its
check in the interact road (queue row
[AUTHORED-INTERACTABLE-STATE](../queue.md#authored-interactable-state--facing-gates-per-chest-and-per-pickup-persistence)).
The maintainer's Interact constraint governs behavioral changes.

### Technique admission and flow bounds

The [authored technique protocol](authored-technique-admission.md) owns
installed support, the effect-reference traversal and flow bounds. Handler
systems stay ordinary typed Bevy systems; the catalog never becomes a dynamic
gameplay dispatcher. Do not add arithmetic, arbitrary queries, variables or a
blackboard to move-scoped flow. Arbitrary game algorithms belong to the
procedural tier of the [extension model](extension-model.md).

## A5 - cross-domain content preflight

A meaningful content unit spans definitions and assets. Prepare a character's
body/actions/equipment, its sprite/portrait, writing, sounds and referenced
techniques together; prepare room geometry, paths, placements, encounters and
bindings together. Report all relevant missing inputs with the selected profile.

Validation does not require every asset to be resident on a GPU. Distinguish
semantic resolution, generated-product availability, device materialization and
activation readiness. Runtime simulation binds one accepted content revision;
late visual materialization cannot change its mechanical values.

Prepared metadata fingerprints are not executable-function fingerprints. An
acceptance receipt identifies the engine/build and provider implementation as
well as authored content. Same content metadata on different binaries does not
establish the same simulation behavior.

## A6 - concise review artifacts

Produce the smallest artifact needed for the decision: semantic room summary and
render; character sheet/hitbox strip; music/SFX preview with diagnostic report;
prepared-content/provenance diff; or a deterministic input/replay fixture. Human
judgment remains explicit for visual/audio quality. A passing automated check
does not certify subjective polish.

Artifacts identify source revision, provider, tool version, selected content,
settings and relevant runtime/frame context. Missing generated data is reported
as unavailable, not substituted under the requested artifact's name.

### Content diffs need domain-aware comparison (Q62, Q78)

Rulings 2026-10-04 ([`../maintainer-decisions.md`](../maintainer-decisions.md)):
for a structured editor format such as LDtk, a large textual diff is not a
large semantic change. Do not ask the maintainer to keep or discard a content
or renderer delta by line count, recency or diff size. First compare it with a
domain-aware tool, separate authored changes from generated or serializer
churn, then decide. Moving the maps into the `game/ambition_map_assets`
submodule keeps churn out of the main history; it does not make a content diff
understandable.

**The comparison tool (queue row LDTK-SEMANTIC-DIFF).** `diff semantic`,
`diff range` and `diff normalize` in `tools/ambition_ldtk_tools`
(`edit/semantic_diff.py`, `edit/ldtk_canonical.py`; usage in the tool README,
section "Semantic diff"). Each side is a file or `REV:PATH`; `diff range` takes
a commit range of one repository, such as the map-assets submodule. The report
gives a verdict (`identical`, `noise_only`, `changed`, `ambiguous`), a
per-level summary (geometry, entities, fields, tiles, noise count), the
authored changes, the ambiguities and a noise section by category. Objects
match by identifier and content, never by uid; an iid is a match key only when
the content-matched iids did not change; entities that no stable key pairs are
an ambiguity, not a guess. Changed `autoLayerTiles` are a content change
(`auto_tiles`), because `bevy_ecs_ldtk` can draw them; a tile whose only change
is its `t`/`d` cache is noise.

Measured with the tool: `576a8fd` (96,567 raw lines) is two moved
`OneWayPlatform` entities and 14 one-way cells; `c6df2b7` (about 7,900 raw
lines) is five new `DebugLabel` entities, one entity field def and one level
field def; `056079f` placed one dog in `sandbox.ldtk` and also regenerated the
Hall (exhibits moved, the exit door `name` overwritten).

**What is known about the recurring rewrite (2026-10-04).** Two fixes did not
stop it: `54d99e7fb` (reuse tileset and rule uids; write only when changed)
and `2e69e81b9` (`scripts/check_ldtk_uid_leak.py`, a leaked-uid warning in
`ldtk/io.py`, no fallback that reformats a whole file). Later map-assets
commits still rewrite much more than they change: `576a8fd` rewrote all of
`sanic_darkness.ldtk` (96,147 tile lines, 346 changed iids) for a small move;
`c6df2b7` rewrote 3,722 tile lines and about 68 uids in `sanic_speedway`;
`056079f` placed one dog in `sandbox.ldtk` (32 lines) and also rewrote 1,268
lines of `hall_of_characters.ldtk`.
Not the cause: an LDtk version change (all worlds are `jsonVersion` 1.5.3,
`appBuildId` 473703). Suspects to measure, not yet confirmed:

- `allocate_iid` (`area_authoring.py`) derives iids from `nextUid`, so one
  extra allocation shifts every later iid and uid;
- `ldtk/io.py` `write_project` always runs `normalize_project_for_editor`
  (re-syncs definition uids, `__worldX/Y`, `realEditorValues`);
- `scripts/regen/sprites.sh` rewrites worlds in place on each sprite
  regeneration (`visual_manifest apply-manifest --in-place
  --prune-unused-tilesets`, `asset editor-art --in-place`), so sprite-sheet size
  changes reach the maps;
- the whole-world generators (`author_*_ldtk.py`,
  `generate_hall_of_characters.py`, `gen_symmetry_room.py`).

Find the writer of each rewrite before you fix one: run each writer twice on
an unchanged input and diff the second run.

**Q62 evidence (2026-10-04).** The 4,741-line `mary_o.ldtk` delta is the diff
`48f8e26 → cb7062a` in `game/ambition_map_assets` ("Start git epoch 1",
2026-09-06), and it is already in history. `semantic_diff` reports one change:
the `HazardBlock` editor visual (`tileRect` 108×70 → 32×16). The rest is
auto-layer output: 1,944 tiles re-added identical except the rule uid `d`, 738
new derived tiles (64 in `mary_o_1_2`, 674 in `mary_o_1_3`, which gained its
`CollisionArt` layer instance), and 58 lines of renumbered rule uids. No
entity, IntGrid, geometry or authored field changed. This matches
`asset editor-art --in-place` before `54d99e7fb`. So there is nothing to
discard; whether the 738 tiles draw at runtime is the one visible question, and
the improved tool must answer it. `48f8e26` is on no branch (local reflog and
the `Erotemic/ambition-history` store only).

**Divergent submodule history (Q78 workflow).** Do not take the newest
commit, and do not run `git submodule update` and accept the result. In order:
list the commits unique to each line (`git rev-list A --not B`, and
`--all --not --remotes`); say what source change each represents; separate
source from regenerated artifacts (by path and by the domain tool); find the
line that is the semantic superset; keep unique work (merge, never rebase);
push it; repin the parent to the pushed commit. Q62 and Q78 share this tool
and this workflow: a renderer change moves sprite sizes, and
`scripts/regen/sprites.sh` then rewrites the maps.

State on 2026-10-04: `tools/ambition_sprite2d_renderer` has no divergence.
The parent pins `7bd8024` (= `origin/main`); the checkout is `dff162f`, an
ancestor of the pin, 8 commits behind (all `.py` source); 0 commits exist on
no remote; 0 dirty files. The parent shows `M` only because the checkout lags.
The old divergence (the unpushed `2b4d59f` against the pin `0828fae`) was
merged in `58be2df` on 2026-09-07. Four pre-epoch commits remain only in the
reflog; three are identical in the epoch root and one changed a ledger.

## A7 - one complete agent-authored acceptance slice

Use the existing moving-platform/LDtk slice to exercise world placement, path
references, preparation, dynamic geometry, rollback and review output. Add one
external-provider action/object slice to prove the same support/validation and
physical-input route outside named game content. These are customers of the
current engine, not invitations to build a second authoring framework.

For each slice, an agent must inspect, plan, edit, prepare, test, review and
publish through supported operations. Include a deliberately invalid reference
and unavailable capability that fail before play, then repair and rerun. Measure
successful completion and corrective loops; do not infer quality from command
count or the amount of generated prose.

## A8 - optional human visual frontends

A visual frontend is justified by a concrete editing need. It reads/writes the
same semantic source and invokes the same validation/publication pipeline. It
must not own a hidden scene/state authority that agents cannot inspect or test.
Do not make a GUI a gate for a headless content change.

## A9 - semantic dependency and reference graph

Support forward/reverse reference queries, structured unresolved references,
rename/delete planning and later rule-dependency inspection. Indexes derive from
prepared/domain references and carry a revision. A rename plan must report all
known affected sites and any opaque/unindexed domains; it must not claim complete
safety while ignoring a nested technique payload or external authored backend.

Across backends, apply with explicit expected revisions and a recoverable staged
protocol. Validate the complete new content revision before publication. An
incremental cache is an optimization of canonical preparation, not an alternate
acceptance authority; invalidation follows dependency identity and tool versions.

## Trust, publication and runtime limits

Validated authored data/programs have a bounded vocabulary and declared budgets.
Trusted Rust plugins/providers have application privileges; registration does
not sandbox them. Generated source receives ordinary review/build/tests, and
external tool execution is subject to the host's real trust boundary.

Preparation rejection, verified hidden-candidate publication and recovery from
unsafe native code are three different guarantees; only the first two are
promised (see [construction](construction-and-reconstitution.md)). Runtime model-backed characters
are remote intent participants with admission/deadline policy, not nondeterministic
callbacks inside simulation.

## Tool health and reproducibility

The ECS-inventory generator already has a focused executable health test:
`scripts/tests/test_ecs_inventory_can_actually_run.py`. Preserve its declared
parser-version compatibility and proof that output was actually produced.
Untracked `.agent` inventories are navigation aids, not authoritative current
counts. Check revision/freshness before using them in plans. The loading-zone
name ratchet remains a focused authored-presentation check, not a reason for
another repository-wide content scanner.

## Acceptance

One external and one flagship slice complete discovery, inspection, change,
preparation, behavioral verification, human-review artifact and explicit
publication/package without manual editor work or internal-module knowledge.
Invalid semantics fail at admission; diagnostics point to the real source;
repeating a request cannot accidentally duplicate content; stale plans refuse;
missing prerequisites remain visible. Current tool tests and Rust/GPU/platform
receipts are reported separately rather than combined into an unsupported pass.


## Revision admission is an author-facing guarantee

Invalid edits cannot partly overwrite active prepared definitions or increment
the active generation. Validate a candidate against frozen installed support,
including expanded/nested reference sites, and keep active playback pinned to
its original revision. Initially publish mechanical changes only at an explicit
session/reconstruction boundary. The detailed protocol includes a real provider
fixture and stable source-path diagnostics; it does not require a universal
execution registry, new scripting language or arbitrary ECS transaction system.

## Independent iteration commands

Expose prepare/validate, describe candidate differences, request activation and
inspect activation status through the current tools. I2/I3 and I7 in the
[iteration packets](fast-iteration-implementation.md) provide those operations
for data and procedural modules. Report source digest, active generation,
required ports, chosen check lane and the first observed result. A watcher and
an agent command call the same admission road. No content-edit command silently
runs a full workspace suite or restarts a compiler for the host.

Offline scripts/models may produce artifacts. Live simulation scripts use the
registered state and deterministic service contract; the two modes are not
interchangeable. Preserve the existing LDtk/sprite/audio tool boundaries.

## Repeatable scenario and truthful activation result

[Generation/reload](content-generation-and-reload.md) defines the operation and
status fields. Pin the developer's scenario seed, input trace and accepted
checkpoint before reconstruction. Show rebuilt/reused sections, attempted and
active generation, refusal owner and whether the old scene was unchanged,
recovered or stopped. Do not report 'loaded' from a build or file-read result.

A stale background preparation result cannot overwrite a later edit. An identical
mechanical generation does not clear history. Unsupported state migration refuses
or asks for an explicit scenario restart; it never silently overwrites a durable
save or starts a new game. Existing tools and GUI frontends call the same operation.
