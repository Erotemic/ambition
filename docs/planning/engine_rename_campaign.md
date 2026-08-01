Rename the platformer-focused crate family so the repository more clearly communicates Ambition’s current center of gravity and its longer-term direction.

The main focus remains building an exceptional 2D platformer engine.
That vertical push should stay central: movement, combat, worlds, portals,
  actors, runtime composition, and presentation should become increasingly
  polished and coherent as a dedicated `platformer2d` stack.

At the same time, Ambition should leave clean seams for external contributors to
  extend the engine and for other styles of games to become possible in the
  future.
The goal is not to prematurely generalize every platformer subsystem.
It is to avoid claiming generic names for crates that are currently
  platformer-specific, while preserving genuinely general services such as
  content compilation, input, causal inspection, loading, assets, audio, and
  time.

This supports the longer-term ambition of becoming a real Unity or Godot
  competitor without weakening the immediate product focus.
Ambition can grow outward from a deep, high-quality 2D platformer engine rather
  than attempting to become shallowly universal from the beginning.

Proposed moves:

```text
ambition_engine_core
    → ambition_platformer2d_core

ambition_platformer_primitives
    → ambition_platformer2d_shared_tangle

ambition_world
    → ambition_platformer2d_world

ambition_ldtk_map
    → ambition_platformer2d_ldtk

ambition_portal
    → ambition_portal2d

ambition_portal_presentation
    → ambition_portal2d_presentation

ambition_runtime
    → ambition_platformer2d_runtime

ambition_host
    → ambition_platformer2d_host

ambition_platformer_provider
    → ambition_platformer2d_provider

ambition
    → ambition_platformer2d

ambition_actors
    → ambition_platformer2d_actor_monolith
```

`ambition_platformer2d_shared_tangle` deliberately acknowledges that the current shared
  layer is a catch-all with architectural smell.
It likely needs decomposition eventually, but that work is lower priority than
  the active architecture campaigns.
Defer it until the surrounding boundaries are more mature or a concrete problem
  makes decomposition the right solution.

`ambition_platformer2d_actor_monolith` is intentionally blunt.
The crate is already a major decomposition target, and the name should
  constantly remind agents that it is not the final architecture and should not
  become the default home for new reusable behavior.

Keep this rename thrust mostly mechanical:

* rename directories, packages, dependency keys, and Rust crate paths;
* update workspace manifests, scripts, fixtures, guards, and documentation;
* regenerate lockfiles;
* do not add compatibility aliases or shim crates;
* avoid unrelated refactors while performing the rename, only perform unavoidable ones.

---

# Report — executed 2026-08-01

All eleven renames are in, in two commits: the rename itself, and a follow-up
for two classes of damage the compiler could not see.

```text
ambition_engine_core            → ambition_platformer2d_core
ambition_platformer_primitives  → ambition_platformer2d_shared_tangle
ambition_world                  → ambition_platformer2d_world
ambition_ldtk_map               → ambition_platformer2d_ldtk
ambition_portal                 → ambition_portal2d
ambition_portal_presentation    → ambition_portal2d_presentation
ambition_runtime                → ambition_platformer2d_runtime
ambition_host                   → ambition_platformer2d_host
ambition_platformer_provider    → ambition_platformer2d_provider
ambition_actors                 → ambition_platformer2d_actor_monolith
ambition                        → ambition_platformer2d
```

No compatibility aliases, no shim crates, no unrelated refactors. Directories,
package names, dependency keys, feature-forward strings, Rust paths, workspace
manifests, `.gitignore` asset rules, CI, guard scripts, fixtures, the four
out-of-workspace consumers and their lockfiles, and documentation all moved
together. 1,469 files.

## Verified

* `cargo check --workspace --all-targets` — clean.
* 131 `scripts/tests`; 156 `tools/ambition_ldtk_tools` tests; 34
  `ambition_workspace_policy` tests.
* `check_doc_links`, `check_absence_contracts`,
  `check_engine_systems_are_engine_installed`, `check_roadmap_evidence`,
  `check_agent_kb` — all green.
* All four out-of-workspace consumers re-locked and building:
  `examples/capability_demo`, `examples/portal_tutorial`,
  `fixtures/minimal_game`, `fixtures/external_consumer`.
* Full `scripts/run_tests.py` — see the commit that follows this report.

## What was NOT mechanical

The ten suffixed names were. `\bambition_portal\b` cannot eat
`ambition_portal_presentation`, so a word-boundary substitution is exact.

**Bare `ambition` was not, and a blanket `sed` would have caused silent damage.**
It is also a prefix of the repository directory, the GitHub URL, the runtime
asset namespace (`ambition/worlds/sandbox.ldtk`), the content namespace
(`ambition:character/goblin`), the XDG save directory (`$XDG_DATA_HOME/ambition`),
the audio provider id, log tags (`[ambition] audio:`) and an LDtk layer called
`Ambition`. Substituting all 7,051 occurrences would have moved every player's
save file and broken asset resolution and content identity, and none of it would
have failed to compile. Only crate-referencing contexts were rewritten; the other
339 bare occurrences were read individually and deliberately left.

Four traps, each of which produced a real failure:

1. **A `\b`-anchored rule cannot see `\bambition::` in a regex literal.** The
   character before the crate name is the `b` of the escape, so the substitution
   skips exactly the files that parse crate paths — and the verification grep has
   the identical blind spot, so the sweep reports itself clean. Five files,
   including the SDK-docs guard, whose failure message was "these modules are a
   compatibility PROMISE and the SDK never mentions them".
2. **`startswith("ambition")` must not be rewritten.** It is a prefix test over
   every crate in the workspace. Three guard scripts mix that meaning and a
   reference to the facade in the same file.
3. **`^ambition = ` is a manifest key in TOML and a local variable in Python.**
   It renamed five `ambition = find_layer(level, "Ambition")` bindings in the
   LDtk tools. Reported as `NameError`, not as anything about a rename.
4. **`git mv` does not follow symlinks.** `game/ambition_content/assets/sprites`
   pointed into `crates/ambition_actors/assets/sprites` and was left dangling.
   `cargo check` was clean and the Rust suite was clean, because the compiler
   never reads a symlink target. One link in the repository, caught by an LDtk
   tools assertion that two canonical asset paths resolve equal.

The shape they share: **a rename is verified by whatever READS the name**, and
the compiler does not read symlink targets, string-keyed layer names, regex
literals, or another language's identifier conventions in the same tree.

## ⚠ One consequence that is not cosmetic

**The rollback schema fingerprint moved.** `schema_dump()` writes
`std::any::type_name::<T>()`, which contains the crate path, so 94 of the 352
lines in `rollback_schema_baseline.txt` changed. The baseline is regenerated in
lockstep and the guard passes.

But it means **a crate name is currently a wire-format fact**: two peers with
byte-identical snapshot encodings compute different fingerprints and refuse to
agree, purely because a crate was repackaged. That is the same category of
organisational label the v5 fingerprint bump already rejected — `registry.rs:33`
says it stopped hashing the registration OWNER because doing so "made 'which
module registered this' a wire-format fact". The crate path inside `type_name`
is the same thing one level up, and it is still hashed. Recorded as queue row
S30.

## Recommended follow-ups

Ordered by value, with the evidence each rests on. None of them are in the rename
commits.

### 1. ⭐ The general kernel is trapped inside a platformer-named crate

This is the highest-value item, and the rename is what made it legible.

The brief says to preserve "genuinely general services such as content
compilation, input, causal inspection, loading, assets, audio, and time." Five of
those seven have **no dependency edge into the platformer stack** and their
unqualified names are earned: `ambition_content_pack`, `ambition_causal`,
`ambition_load`, `ambition_asset_manager`, `ambition_audio`.

**`ambition_input` and `ambition_time` are not general — both depend on
`ambition_platformer2d_core`.** The edges are tiny, and that is the point:

```text
ambition_input   →  ControlFrame                  (4 lines)
ambition_time    →  snapshot::{…}                 (2 lines)
ambition_characters →  ControlFrame, Vec2, AbilityGrant
ambition_dialog  →  Vec2
ambition_interaction → Aabb
ambition_persistence → InputFrameMode
```

None of `ControlFrame`, `InputFrameMode`, `snapshot::{…}`, `Vec2` or `Aabb` is
platformer-specific. They are the simulation kernel: an input frame, a rollback
snapshot vocabulary, and 2D math. **19 unqualified crates depend on the
platformer stack, and a large share of them do it only for those.** 28 crates
depend on `ambition_platformer2d_core` in total.

So the crate now named `ambition_platformer2d_core` is two things wearing one
name — a general kernel and platformer vocabulary — and the rename has made every
general service in the workspace declare a platformer dependency it does not
have.

**Recommendation:** carve the kernel out (`ambition_sim_kernel`, or whatever it
should be called) holding the snapshot traits, `ControlFrame`/`InputFrameMode`
and the 2D math primitives, leaving `ambition_platformer2d_core` for the
platformer vocabulary. The payoff is exactly the stated long-term goal: a second
stack (top-down, puzzle, whatever) reuses input, time, causal, loading, assets
and audio without linking a platformer, and the `orphan rule` keeps deciding
placement rather than taste. Measure it first — the import census above is one
command and says how much of the 28 is kernel-only.

### 2. ⭐ A guard that no source names a retired crate — ✔ BUILT 2026-08-01

Two of the four traps above (`\bambition::` in regex literals, and the guard
scripts' `"ambition"` string literals) are the same defect: **a crate name written
as data, where no compiler reads it.** They were found by test failures whose
messages were about SDK documentation and Python `NameError`.

A ~30-line check closes the whole class: scan tracked text for `ambition_[a-z_]+`
tokens, compare against `cargo metadata` package names, and fail on any token
that names no member. It would have caught both mechanically, in one second, and
it keeps paying every time a crate moves. Cheap enough to write before the next
rename, and the next rename is coming (see §1).

✔ **Built as `scripts/check_retired_crate_names.py`**, and the design changed
under measurement. "Any `ambition_*` token that is not a workspace member" was
tried first and is unusable — **128 distinct false positives** on this tree
(Python packages, shell functions, JSON keys, LDtk layer names, asset
manifests). What works is an explicit retired-name table, searched by plain
substring with a TRAILING boundary only, historical prose exempted by the words
it already contains. Probed against a real injected regression, eight self-tests
including both escape cases verbatim, and a live-tree ratchet in `scripts/tests`.

⚠ **and it found drift on its first run**: `ambition_engine`, retired long before
this session, was still named in seventeen live places. Most were legitimate
history. Three were not — a `verification_command` that cannot run, and two
content/spec comments pointing into `crates/ambition_engine/`, one of which also
claimed its spec equivalence was "pinned by" a test that does not exist in this
tree. A comment asserting a guarantee nothing provides is worse than no comment,
and nothing but this guard was ever going to find it.

### 3. Decide whether a crate name may be a wire-format fact (queue S30)

Stated above. Two options, and the second is the one consistent with what v5
already decided:

* **(a)** accept it — regenerate the baseline on every rename and treat repackaging
  as a compatibility break forever;
* **(b)** hash the type's module path BELOW the crate, so the fingerprint stops
  caring where a type is packaged. ⚠ this makes two same-named types in different
  crates collide, so it relies on `RollbackRegistry`'s existing duplicate-`name`
  rejection to stay the thing that catches it.

Only (b) makes the next rename free. Neither is urgent; both are cheap now and
expensive after a shipped multiplayer build.

### 4. Assert every symlink in the repository resolves

One line in the suite. The sprites symlink dangled through a clean
`cargo check --workspace --all-targets`, a clean Rust test run, and every guard
script — it took an LDtk tools assertion about canonical paths to surface it, and
that assertion exists by luck rather than design. There is exactly one such link
today, which is precisely why nothing is watching it.

### 5. `MODULES.md` drift, and no check enforcing it

Regenerating during the rename revealed that **19 crates have stale module maps**
and **3 have none at all** (`ambition_causal`, `ambition_content_cli`,
`ambition_content_pack`). Those changes were deliberately reverted from the
rename commits as unrelated. `scripts/modules_md.py` has a check mode and
**`scripts/run_tests.py` never calls it**, so the D-B navigability standard is
maintained by whoever remembers. Regenerate once, then add the check.

### 6. `run_game.sh` validated LDtk worlds at a path that does not exist — ✔ FIXED 2026-08-01

Pre-existing — confirmed against `HEAD` before the rename, which faithfully
carried the stale path forward:

```bash
local worlds_dir="$repo_root/crates/…_actor_monolith/assets/ambition/worlds"   # does not exist
```

The worlds live in `game/ambition_content/assets/worlds/` (the content split).
⚠ the failure was not loud: `sandbox`/`intro` were passed unconditionally, but
the Hall and cut-the-rope worlds sat behind `[[ -f … ]]` guards and were
therefore **silently dropped** — while the comment directly above said every
secondary world must be passed "so the validator resolves cross-file LoadingZone
targets (the hub door into the Hall, etc.)." Cross-file door validation had been
quietly degraded to nothing, which is the exact silent-skip shape this repo keeps
finding.

✔ **Fixed (Jon, 2026-08-01).** `run_ldtk_validation` now DISCOVERS every `.ldtk`
under the content worlds directory rather than naming four files, so a newly
authored world cannot be dropped by omission; it prints the entry world and the
secondaries it found, so a short list is visible instead of silent; and it fails
loudly — exit 2, through `fail` — when the directory or the entry world is
missing, instead of degrading to a single-world check. Both failure branches were
probed by moving the real files aside, and the happy path now validates all four
worlds with no cross-file errors, which is the first time that check has actually
run.

### 7. The two blunt names, and when to retire them

Both are doing their job as labels. Concrete state, so the decision is not
re-derived later:

* **`_shared_tangle`** is ~13k lines, and two of its modules are most of it:
  `construction/` (4,906) and `gameplay_presentation/` (3,486), then `lifecycle/`
  (1,259) and `projectile/` (1,063). Those top two look like standalone crates
  rather than a decomposition project. Deferred by the brief — noted so the first
  concrete problem that touches construction or presentation can take the seam
  instead of growing the tangle.
* **`_actor_monolith`** is the active campaign. The name should survive until the
  crate no longer accepts new reusable behaviour, not until it merely shrinks.

### 8. `ambition` as a runtime identifier no longer names any crate

The asset namespace (`ambition/worlds/…`), the content namespace
(`ambition:character/goblin`), the audio provider id `"ambition"`, the XDG save
directory and the `"ambition"` experience id were all deliberately left alone —
they are stable identity, and changing them moves players' save files. Worth one
explicit decision recorded somewhere: they mean **the product Ambition**, not the
facade crate. Right now that is true by accident of what was safe to change, and
the next person to rename something will have to re-derive it.

