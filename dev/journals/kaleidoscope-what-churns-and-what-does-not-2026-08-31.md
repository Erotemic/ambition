# The kaleidoscope: what churns, what does not, and what is still unmeasured

**2026-08-31.** Written because the reasoning behind a menu-performance pass is
expensive to rediscover and cheap to record. Landed work is in `88efa268a`
("One page changed, one face rebuilt"); this file is the *map and the open
question*, not a changelog.

Read this before optimizing the cube. Several obvious-looking suspects here are
already **falsified** — chasing them again is the waste this file exists to
prevent.

## 0. Where the thing lives

| Piece | Path |
|---|---|
| Renderer (Bevy-side, generic over the host's `PageId`/`Action`) | `game/ambition_menu_kaleidoscope/src/{lib,page}.rs` |
| Host: publishes pages, owns cursor/scroll/drill | `game/ambition_app/src/menu/kaleidoscope_app.rs` + `kaleidoscope_app/{cache,pointer,scroll,scrim,dev_toggles}.rs` |
| Backend-neutral page vocabulary | `crates/ambition_menu/src/lib.rs` |
| Flat `bevy_ui` alternative backend | `game/ambition_app/src/menu/grid_backend.rs` |

There are **three** gates between "a frame happens" and "the cube does work", and
they are not the same gate. Do not collapse them:

1. `kaleidoscope_render_needed` — a `run_if` on the whole `KaleidoscopeRender`
   set. Off when the menu is shut AND the fold has decayed.
2. `republish_kaleidoscope_pages`' `RebuildKey` — a **value** comparison deciding
   whether to bump `ActiveMenuPages::version` at all.
3. `rebuild_cube_faces` — now a **per-face** comparison against `RenderedFace`.

⛔ Gate 2 must never use Bevy change ticks. Host systems take `ResMut` on
`OwnedItems` / `UserSettings` while merely navigating, so `is_changed()` is true
on frames where no rendered value moved. That mistake is the historical
rebuild-every-frame FPS cliff (`cache.rs:129-136`, and `lessons_learned.md:31`).

## 1. Settled: we are NOT behind the standalone demo

Compared against `/home/joncrall/code/bevy_lunex_oot_kaleidoscope_menu_demo`
(its `PROFILE-IDLE.md`, commit `7c5062b`, and current `src/app/`). Every
optimization that demo landed is already here, and several of its own listed
"next things to profile" are done: one camera rather than a second PBR HUD
camera, `Tonemapping::None`, `unlit` materials throughout, OIT/FXAA never
inserted, in-place focus/detail-text updates, and a `run_if` on the render set
that the demo has no equivalent of.

⇒ **Do not port from that demo.** Its two remaining differences were partial
rebuild (now done here, and generalized to N pages) and culling to 3 of 4 faces
(rejected: the back face is behind the camera and frustum-culled anyway, and
culling has to be special-cased mid-rotation — the demo needed exactly that
carve-out for its capture mode).

## 2. Measured: what a face actually costs

Probe run against `rebuild_cube_faces` in a headless app, 2026-08-31. Entity
counts with the resources-as-entities baseline subtracted (⚠ Bevy 0.19 counts
resources as entities — a raw `Query<Entity>` reads ~33 here before any face
exists, so absolute counts from a naive census lie):

| what | entities |
|---|---|
| a face with no controls (root, background, panels, title, frame) | **11** |
| **each control** (its plane + 8 selection corners + label + detail) | **+11** |
| a face with 24 controls | **275** |

`ITEM_COUNT` is 24 (`crates/ambition_items/src/lib.rs:18`), so the Items face is
almost exactly that 275-entity measurement. `SYSTEM_VISIBLE_ROWS` is 6, so the
System face is far smaller. A built cube is therefore roughly **600–700
entities**, and they stay resident while the menu is closed — `gate_kaleidoscope_menu`
hides the ring and deactivates the camera but despawns nothing.

## 3. FALSIFIED — do not spend time here again

- ⛔ **"Lunex re-lays-out the resident faces every frame while the menu is shut."**
  It does not. Nearly every `bevy_lunex` 0.7 system is `Changed<...>`-filtered
  (`lib.rs:227,309,573,786,867,886,915,945,977,991,1126`). Idle faces cost a tick
  comparison, not layout. The resident population is not, by itself, the problem.
- ⛔ **"`RebuildKey` deep-clones `UserSettings` every frame."** It clones it, but
  `UserSettings` is four sub-structs of plain scalars/enums with **no heap
  allocation anywhere** — that clone is a memcpy.
- ⛔ **`Msaa::Off` as a win.** Already A/B'd; it recovers nothing, and the cube's
  `Camera3d` must match the host `Camera2d`'s sample count or it drops all
  geometry (`lib.rs:440-450`).

## 4. STILL OPEN — the best remaining suspect, unmeasured

Everything below is verified **in source**, at the line. None of it has a
measured magnitude yet. ⚠ A coherent source reading is not a measurement.

**Per-frame allocation churn while the menu is open, all to answer "did anything
change?" — and the answer is almost always no.** `cache_system_menu` runs every
visible frame on *every* face (it says so, `cache.rs:20-22`) and:

- `dev_snapshot()` builds `DevSnapshot { values: Vec<(DevToggleId, bool, String)> }`
  — **one `String` allocated per toggle**, e.g. `"ON".to_string()`
  (`settings_menu/src/system/mod.rs:434-446`). There are **22** `DevToggleId`
  variants (`ALL` is `[Self; 22]`, counted 2026-09-02 — this file said 21).
  ✔ **FIXED 2026-09-02.** The field is `Cow<'static, str>` now, so all 22 borrow:
  every one of them originated in a `&'static str` (`"ON"`/`"OFF"`, or a
  `label()` constant) and was being copied onto the heap to be dropped a few
  systems later. Only `SystemMenuModel::build` materialises an owned `String`,
  and only while the System face is actually shown.
- `radio_snapshot()` builds `RadioSnapshot { stations: Vec<(usize, String)> }` —
  one `String` per station.
- `republish_kaleidoscope_pages` then does `cache.radio.clone()` and
  `cache.dev.clone()` into a fresh `RebuildKey` — **allocating all of it a second
  time** — compares it, and usually drops it.

⇒ order of **40+ String allocations plus several Vecs per open frame**, purely as
a change-detection mechanism. The structural fix is to compare without
materializing: hash the snapshot inputs, or compare field-wise against the stored
key instead of building a new one. ⚠ Whatever replaces it must stay a **value**
comparison (see §0's gate 2).

**And separately:** `kaleidoscope_sync_focus_visuals` calls `focus_for_action`
per control per frame; for System-family actions that allocates a fresh
`Vec<SystemRow>` **per control, per frame** (`system_rows_with_quality_prompt`,
`menu/model.rs:515`) — while `CachedSystemMenu.rows` already holds that exact
list, built once per frame from identical inputs. The fix is `focus_for_action`
taking the rows rather than the model; it touches six call sites including
`grid_backend.rs`, which is why it was left out of `88efa268a` rather than
folded in. ✔ **DONE 2026-09-02**, as described: `focus_for_action` takes
`rows: &[SystemRow]` and the three parameters it only used to rebuild that list
are gone. `grid_backend::focus_key_for_cursor` builds it ONCE outside its
per-node loop.

⚠ **AND THE COST WAS SMALLER THAN THIS PARAGRAPH IMPLIES — read it before
quoting it.** `SystemRow` is a small id enum, so each rebuild was one heap
allocation plus a copy of N enum values, not a walk of the settings IR. Real, and
worth removing from a per-control loop, but allocation traffic rather than
compute.

⛔⛔ **THE SETTINGS-IR WALK IS IN `pointer.rs`, AND THIS FILE MISSED IT.** The
hover observer (`Pointer<Move>`, which fires on every mouse move across a
control) and the release observer both call, when the hovered action is a System
action:

```rust
focus_for_action(action, active_page,
    &SystemMenuModel::build(&settings, &snapshot.radio_snapshot(), &snapshot.dev_snapshot()),
    ..)
```

That is the FULL settings IR — a `String` per label, description and value —
plus both snapshots, rebuilt PER HOVER EVENT, to resolve one row index.
`CachedSystemMenu` already holds the model and the rows, built once that frame.
This is a bigger cost than the per-control `Vec` above and it is not in §4's
list.

⚠ It is NOT a straight swap to the cache. `cache.rows` is populated only while
`pages.active == Some(MenuPage::System)`, but a System action stays reachable off
that face — `focus_without_system_model` returns `Some` only for
Equip/Use/ChangePage, so a hovered System control falls through to the model
build whatever face is up. Substituting an empty `cache.rows` there would
silently resolve every such hover to `MenuFocus::System(0)`. The safe shape
guards on the active page and leaves the off-face path exactly as it is.

**Unverified, inherited from a survey and NOT re-read at the line:**
`project_scrollbar_tracks` doing per-frame `world_to_viewport` with unguarded
`ResMut<ScrollbarDragState>` writes, and `publish_menu_confirm_prompt` running
every sim tick with no run condition. Confirm before acting.

## 5. How to measure it, when someone does

⭐ **Prefer a count to a timing.** The dev box is an HD 630 at ~45–60 ms/frame;
a menu change is far under wall-clock noise there. An allocation count or an
entity count survives what a timing cannot.

- The §2 numbers came from a throwaway test against `rebuild_cube_faces` in a
  headless app — cheap to redo, and the pattern is in
  `game/ambition_menu_kaleidoscope/src/per_face_rebuild_tests.rs`.
- For a real frame cost, `scripts/profile_desktop.sh` with the menu actually
  open for part of the run; the bundle lands in
  `dev/ambition_dev_measurements/profiles/`. ⚠ `docs/recipes/profiling.md`
  warns against profiling a menu *instead of* a match — this is the case where
  the menu is the subject, so say so in the bundle's scenario field.
- ⚠ Take the reading from a clean tree. A dirty working tree makes a bundle
  unattributable to a commit.

## 6. Traps this area has already sprung

- The renderer's rebuild oracle and the host's republish oracle are **two
  different gates**; a change to one that ignores the other either renders
  nothing (a republish with no `version` bump) or rebuilds everything.
- `fake_rebuild_cube_faces` in `lunex_kaleidoscope_app_tests.rs` hand-mirrors the
  renderer's gate. It now deliberately despawns *more* than the real renderer
  does; that asymmetry is safe for those tests but means it is not evidence about
  the renderer's narrowing.
- `sync_control_focus_visuals` owns a control's RGB; `fade_kaleidoscope_materials`
  owns its alpha and `alpha_mode`. The handoff between them is
  `Changed<MenuVisualState>` — the focus sync writes the material in place, so
  there is no `Changed<MeshMaterial3d>` to wake the fade. Break that arm and a
  solid control stays `Blend` instead of `Opaque` while the fold sits settled,
  which is the z-fight the "Fix 3" rule exists to prevent.
