# Persistence, save compatibility, and confirmed side effects

**Checked 2026-08-07.** Ambition already has enough persistence policy to compare
against mature save systems: versioned typed save data, migration, preservation
of future/invalid files, crash-resistant replacement, isolated non-session app
roots, and rollback-aware autosave that refuses to persist speculative state.

## The Ambition capability that already exists

[`ambition_persistence`](../../crates/ambition_persistence/src/lib.rs) separates
persisted data shapes from menu/UI policy. The source contains several contracts
that are easy to miss if we describe it only as "RON save files":

- `AmbitionGameSaveData` has an explicit `CURRENT_SAVE_VERSION` and migration
  chain;
- missing fields default so older records can be extended compatibly;
- a save from a **newer** build is detected as `FromTheFuture` and left
  untouched rather than overwritten by an older executable;
- a file declaring a schema with no migration path (including historical
  `version: 0` development saves) is likewise non-fatal: startup uses fresh
  defaults, preserves the original bytes, and explains how to reset the file;
- an unreadable or unparseable existing file also disables writes for that
  session instead of turning a fallback fresh state into destructive autosave;
- a migrated value is marked `upgraded` so it is actually rewritten in the new
  format instead of being mistaken for the on-disk shadow;
- writes use temp/replace/backup behavior intended to preserve either the old or
  new complete save across failure;
- `PersistenceRoot` is app-owned state, allowing tests/headless tools/multiple
  Apps to use isolated roots instead of sharing ambient per-user files;
- visible-session persistence is omitted from headless/RL composition;
- autosave runs only when `world_state_is_confirmed`, because the sandbox save
  is rollback state and must not commit a speculative history to disk.

See [`save.rs`](../../crates/ambition_persistence/src/save.rs) and
[`save_data.rs`](../../crates/ambition_persistence/src/save_data.rs).

The last point connects persistence to the engine's deterministic host model in
a way ordinary "serialize some fields" tutorials do not.

---

## Unreal `SaveGame` — project-defined durable state with engine I/O support

Unreal's save/load system revolves around project-defined `SaveGame` classes
containing the information a game chooses to preserve across sessions. Unreal
provides sync/async save/load operations around those objects.

Source: [Saving and Loading Your Game](https://dev.epicgames.com/documentation/unreal-engine/saving-and-loading-your-game-in-unreal-engine?lang=en-US)
(Epic, official).

### Comparison

Unreal is the right usability bar for a public platformer SDK: games should own
their durable schema while the engine supplies slots, paths, async I/O and common
lifecycle hooks.

Ambition currently has game-shaped sandbox data in `ambition_persistence`, so
one design pressure is clear: separate **engine persistence mechanics** from
**Ambition-the-game's save schema** before presenting persistence as a reusable
SDK feature.

The source already contains reusable mechanics worth preserving in that split:
compatibility verdicts, non-destructive fallback, app-local roots, safe commit
and confirmed-history gating.

---

## Unity serialization — broad object/data serialization is not a save policy

Unity's serialization system transforms data structures and object state into a
form Unity can store/reconstruct. `JsonUtility` exposes the same structured
serializer for JSON data.

Sources:

- [Script serialization](https://docs.unity3d.com/6000.0/Documentation/Manual/script-serialization.html)
  (Unity, official).
- [`JsonUtility`](https://docs.unity3d.com/6000.0/Documentation/ScriptReference/JsonUtility.html)
  (Unity, official).

### Comparison

This is an important distinction for Ambition: **serialization is mechanism;
compatibility and commit policy are engine contracts**. A serializer cannot by
itself answer whether an older build may overwrite a future save, whether a
migration needs to be persisted, whether a predicted tick may autosave, or
whether a corrupt-but-existing file should be preserved.

Ambition should keep those decisions explicit rather than treating whatever
Serde can deserialize as "safe to write back".

---

## Godot saving guidance — flexible project-level persistence

Godot's saving-games documentation presents project-level approaches using
`FileAccess`, JSON or binary variants, with game code deciding which objects and
properties to persist.

Sources:

- [Saving games](https://docs.godotengine.org/en/stable/tutorials/io/saving_games.html)
  (Godot, official).
- [`FileAccess`](https://docs.godotengine.org/en/stable/classes/class_fileaccess.html)
  (Godot, official).

### Comparison

Godot is a useful simplicity bar: persistence should remain understandable as
ordinary game data plus file I/O. Ambition's additional value should be a small
set of hard-earned correctness policies, not a reflection-heavy universal
object snapshot format.

That fits the rest of the engine: persist semantic IDs/facts needed to rebuild
state rather than treating the live ECS allocation graph as durable identity.

---

## File replacement semantics — durability is below the serializer

Ambition's `write_save` comments explicitly account for platform-specific rename
behavior and preserve a backup if replacement cannot be performed directly.
Windows documents separate move/replace operations and replacement options.

Source: [Moving and Replacing Files](https://learn.microsoft.com/en-us/windows/win32/fileio/moving-and-replacing-files)
(Microsoft, official).

### Comparison

This is mundane but engine-worthy. A platformer engine claiming save support
should test crash/failure behavior on supported hosts rather than assuming
"write temp then rename" means the same thing everywhere.

The implementation's intended invariant is stronger and easy to state:
**failure may lose the newest progress, but must not leave neither the old nor
new complete save.**

---

## What Ambition already distinguishes

| Concern | Common baseline | Ambition today |
|---|---|---|
| durable schema | project-defined save object/data | typed versioned `AmbitionGameSaveData` |
| old saves | serializer defaults/project migration | explicit migration chain + rewrite-needed state |
| future saves | often project convention | `FromTheFuture` => usable fallback but file becomes non-writable |
| unsupported old schema | often migration crash/error | fresh in-memory fallback, original preserved, actionable reset diagnostic |
| corrupt/unreadable file | fallback or error | fresh in-memory fallback while preserving existing bytes |
| write safety | normal file write / async slot API | temp replacement with backup/restore fallback |
| process/app isolation | global user-data path | `PersistenceRoot` resource, isolated roots for non-session Apps |
| rollback interaction | usually networking/project-specific | autosave gated on confirmed world state |
| durable identity | object references/IDs vary by engine | semantic save IDs intended to survive ECS reconstruction |

## Design work the comparison exposes now

### 1. Split engine persistence policy from Ambition game schema

Move reusable mechanics behind a small SDK surface that lets a consumer define
its own versioned save payload and slots without depending on Ambition's quests,
encounters, inventory or dialog vocabulary.

### 2. Define a migration registry and compatibility policy

The current v1 -> v2 -> v3 chain is good evidence. Turn it into an explicit
contract before many versions accumulate: ordered migrations, supported oldest
version, failure diagnostics, future-version preservation and tests using golden
fixtures from released schemas.

### 3. Make confirmed-side-effect policy reusable

Disk save is one external side effect; achievements, cloud sync, analytics,
haptics and some audio/VFX also cannot be "un-emitted" by rollback. Define a
shared confirmed-effect publication seam rather than teaching every subsystem
independently about speculative history.

### 4. Add durable write tests per supported host/filesystem profile

Test repeated replacement, failure injection, interrupted writes and recovery.
The contract matters more than the exact temp/backup algorithm and may need a
platform-specific implementation underneath one interface.

### 5. Tie save restoration to stable construction identity

The reconstruction system already owns `SimId` and `SpawnOrigin`. Persistence
should resolve save records against semantic construction/content identity and
report orphaned/changed references explicitly rather than silently keying on
runtime entities or positional array indices.

### 6. Define cloud/multi-slot concurrency before adding it

Named campaign slots, cloud sync and multiple running instances introduce
conflict/revision semantics that the one-slot sandbox does not need. Preserve
the current principle — never overwrite information a build cannot understand —
when that work arrives.

## What this comparison changed

Persistence should enter the related-work map as an implemented correctness
subsystem. Ambition does not need a more magical serializer; it needs to retain
and generalize the policies already present in source:

> version explicitly, migrate deliberately, preserve unknown data, commit
> durably, isolate non-session Apps, and never persist speculative rollback
> history.
