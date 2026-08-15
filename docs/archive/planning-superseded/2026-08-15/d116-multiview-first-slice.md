# D116 — Ambition multiplayer/multi-view first slice (M2 presentation half closed 2026-08-15)

**Role:** EVIDENCE. ⛔ not authority. The live row is the compact rest row in
`docs/planning/queue-72h-2026-08-08.md`; current design lives in
[`../../../planning/engine/multiplayer-and-multiview.md`](../../../planning/engine/multiplayer-and-multiview.md).

⚠ M2 is **half** done: the presentation/projection sub-slice is closed; production
two-view composition/layout, HUD ownership and input routing are deferred.

---

- ⏸ **D116 — Ambition multiplayer/multi-view first slice. RESTING: M2's presentation/projection half is CLOSED, its production two-view composition/layout half is DEFERRED.**

Use [`engine/multiplayer-and-multiview.md`](engine/multiplayer-and-multiview.md)
and [`game/multiplayer.md`](game/multiplayer.md). Do not start with networking.
Start with the architecture that local and remote transport will share.

The first proof should make **view identity/indexing explicit** while preserving
the current one-view case, then prove two local participants/control assignments
can be observed without creating player-body special cases. The target model
keeps transport, control assignment, room residency and presentation layout as
independent axes.

Ambition needs shared screen, fixed split and BG3-like adaptive share/split. A
later slice must allow participants to occupy different rooms. TwinTrack is a
strong independent-observer acceptance customer for the same engine seam.

✔ **M2a — one body-generic presented pose — LANDED 2026-08-14.** Jon's F1
overlay was the live symptom; the cause was that `PresentedPose` followed
`BodyPoseView`, which is rebuilt `With<PlayerVisual>`. Three consumers read that
absence as "no interpolation" instead of "not covered", so a boss's strike drew
on the tick clock, its unauthored-attack stand-in drew nothing, and its slash
visual never followed it. Now `PresentedPose` follows `BodyKinematics` and
`PresentedPose::delta()` is the ONE translation every rigidly-attached row takes
in a frame. `owner_anchor` deleted. Detail in
[`engine/multiplayer-and-multiview.md`](engine/multiplayer-and-multiview.md) §M2a.

✔ **the camera→view LINK landed 2026-08-14 — do not rebuild it.** `PresentsView`
is a component on the camera naming the view it presents, and `camera_follow` no
longer pairs a `Single<…, With<LocalView>>` against a `With<MainCamera>` query by
the coincidence that there is one of each. `LocalView` / `LocalViewId` /
`spawn_local_view` and view-owned camera state are all present, and `PresentedPose`
is body-generic (M2a).

⇒ **M2's real remainder, measured against HEAD 2026-08-14 — the objective is TWO
LOCAL VIEWS OVER ONE ROOM AND ONE SIMULATION, and nothing wider:**

✔ **1 and 2 LANDED 2026-08-14.** `camera_follow` took the FIRST camera's link,
resolved that one view, and wrote its transform, rotation and ortho scale onto
EVERY main camera — so two cameras bound to two views both drew the first one. It
is now a per-camera loop. ⭐ **three sites must agree on "which view is this
camera for"** — `camera_follow`, the viewport applier and `PresentedViewState` —
so the rule is stated ONCE (`ViewsOnHand`) instead of a third time; every
disagreement between copies would have been silent, because each site still
resolves *some* view and draws *something*. The proof is
`each_main_camera_presents_the_view_it_names`, asserted on VALUES derived from
room geometry (not "the two differ", which passes for a pair that differ and are
both wrong), and falsified from inside the test by a second run that swaps ONLY
the two links and requires the cameras to swap with them.

⛔ **and my own briefing of that job was wrong, in the way this ledger keeps
teaching.** I wrote that `CameraViewport` "already exists with NO consumer".
It has several — `resolve_camera_observation` reads it for orthographic scale,
visible extent and clamp half-extents, and four host oracles assert on it. What
it lacked was a PLACEMENT consumer, and it could not have had one: **it carried a
size and no origin**, so it could never position a second view. It now carries
`origin_px`, and `apply_gameplay_camera_viewport` — which had the same defect in
the viewport axis, writing one global `gameplay_rect` to every main camera — hands
each camera its own view's rectangle. ⚠ "no consumer" was a memory, not a
measurement.

3. **The remaining `PresentedViewState` consumers name WHICH view.** ✔ census
   RE-VERIFIED against HEAD: `foreground.rs:89`, `label_layout.rs:438`,
   `nameplates.rs:209` are real; `actors/mod.rs:337` feeds only the
   `[sprite-size]` eprintln and affects no pixel; `dev/debug_overlay.rs:100` is
   dev. Each of those three draws ONE set of world-space entities — one
   `Transform` per world label, nameplate or parallax layer — so naming the view is
   not the blocker; per-view CORRECTNESS needs either those entities duplicated per
   view, or a policy that picks one.

   ✔ **DECIDED AND LANDED 2026-08-14 — PER-VIEW PRESENTATION PROJECTIONS.** One
   authoritative source, N projections of it; **no view is privileged**, and the
   world/simulation entity stays SINGULAR. `PresentedForView(Entity)` is the key,
   and `ViewsOnHand::drawn_for` answers *"which view is this drawn entity for"*
   using the SAME rule `presented_by` uses (one shared private `resolve`, but
   deliberately separate `error_once!` sites — one shared site would silence
   whichever ambiguity happened second). Both live systems iterate VIEWS now.
   ⇒ duplicated: nameplate plates and drawn label copies. ⇒ **NOT** duplicated: the
   sim body, the `NameplateIndex` row, `DoorNameplateSource` on the room visual,
   the room's authored label list.

   ⛔⛔ **AND MY CENSUS WAS WRONG BY ONE — `rendering/foreground.rs` WAS DEAD
   CODE.** No `mod foreground;` declaration existed anywhere in the tree, so it had
   never been compiled; it named `ForegroundParallaxSprite`,
   `foreground_parallax_factor` and a `GameAssets::foregrounds` field that **exist
   nowhere**, so it could not have compiled if hooked up. ⚠ **M2a counted it as a
   live `CameraViewState` reader and migrated it — an edit that was free because
   nothing built it.** Deleted; the two sites recording "five readers" now record
   four and say why. ⇒ the real per-view population is **TWO** systems,
   `layout_world_labels` and `sync_actor_nameplates`. ⭐ **a census that counts
   files instead of asking the compiler will count a file that does not exist.**

   ✔ **the `Vec2::ZERO` fallback is DELETED, not repaired** — iterating views
   leaves no branch that can invent a focus, so zero views DECLINES. ⛔
   `LocalViewId` is explicitly documented as NOT a `RenderLayers` bit; nothing in
   the semantic API touches renderer isolation. ⚠ **consequence to be honest
   about: two views' entity sets currently draw into BOTH cameras.** Transforms are
   per-view correct; VISUAL isolation is the render side's remaining job, and that
   is what "two correct pictures" still owes.

   ⚠ **adjacent, same class, deliberately out of scope:**
   `parallax.rs::sync_parallax_layers` reads the `MainCamera` transform via
   `.single()`, so with two cameras it **silently does nothing** — and
   `PARALLAX_BACKGROUND_LAYER`'s own comment already documents this bug class. And
   `MainCameraEntity` has exactly ONE real reader (`kaleidoscope_app/scrim.rs`, UI
   scrim retargeting); its replacement shape is now obvious — "the main camera"
   becomes "the camera that presents view V" — but a `UiTargetCamera` consumer
   needs the DISPLAY answer, not the view answer, so it is genuinely its own job.

4. **D118 C5 folds in here** — camera policy read off the view index is N-view
   work, not camera-frame work.
5. ⚠ **two shapes found while doing 1 and 2, both left alone deliberately.**
   With several cameras, label layout and nameplates fall back to a WORLD-ORIGIN
   focus (`Vec2::ZERO`) rather than declining to draw — silent-wrong where the
   rest of this seam is loud-wrong, and the two should agree. And
   `MainCameraEntity` is a **seventh process-global "the main camera" resource**
   that split-screen will have to answer for.

⇒ **what M2 still owes: the pictures.** Two cameras now receive two correct
transforms and projections, and nothing yet proves they produce two correct
PICTURES. That is the acceptance gap, not a plumbing one.

✔ **M2's BOUNDED COMPOSITION PROOF LANDED 2026-08-15 — and it found the two real
defects, at the earliest point in the chain.** ⭐ **both writers of `PresentsView`
were still guessing** — `spawn_main_camera` and `host_presentation_scaffold` each
did `views.iter().next()` and bound the rig to whichever view yielded first. That
is why a wrong link downstream still produced a complete picture: everything below
resolved *some* view faithfully. Both now go through `ViewsOnHand`, taking the
only view in a one-view composition and **REFUSING LOUDLY** — leaving the link off
— when several exist.

⇒ census of every "take the first view" shape, with verdicts: the two writers were
the only defects. `PresentedViewState::get` (already refuses on several cameras),
`ViewsOnHand::survey` (**this IS the rule** — it takes the first only after proving
there is no second), `the_only_view` (a fixture helper whose name is the
disclaimer, and which panics rather than picking), and `label_layout`'s
`ordered.first()` after sorting by `LocalViewId` (a stated rule, not archetype
order) are all correct as they stand.

⇒ the proof is `each_camera_renders_into_the_rectangle_of_the_view_it_names`
(`ambition_platformer2d_host`), running the **real** plugin, window and shipped
schedule, with both views spawned through the production `spawn_local_view` — **a
second view is a second CALL, not a second code path.** It falsifies itself by
running the whole composition twice and swapping ONLY the two links, so an applier
keyed on iteration or spawn order passes run 1 and fails run 2; plus a
precondition that both viewports are `None` after a full-bleed update, so the
distinct rectangles cannot be setup residue.

⛔⛔ **TWO HONEST BYPASSES, AND THEY ARE THE REMAINING FINDING — M2 stops here
either way.** (1) **No production seam spawns a second camera**: both camera-spawn
sites spawn exactly one `MainCamera`. (2) **`publish_camera_viewport` writes ONE
rectangle to every view BY CONSTRUCTION**, because it projects the single
`ResolvedGameplayPresentation` — a fact about the physical screen. ⇒ **distinct
rectangles cannot come out of the shipped schedule today**, and that is a
composition/layout gap owned by a future split host, not a bug in this seam. Both
are named in the test's own doc block.

⚠ **three process-globals a split host will have to answer for**, all recorded and
none touched: `parallax.rs::sync_parallax_layers` reads the `MainCamera` transform
via `.single()` and **silently stops the backdrop in BOTH views** with two
cameras; `MainCameraEntity` is written unconditionally by both camera-spawn sites
and would be last-writer-wins; and portal camera continuity is still one
process-global.

⏸ **THIS ROW RESTS (2026-08-15) — and M2 is NOT complete. Classify it in two
parts, because saying "M2 done" overstates it:**

- ✔ **CLOSED — the presentation/projection sub-slice.** An assembled-host fixture
  proves per-view association and viewport application, and both `PresentsView`
  writers that took `views.iter().next()` are fixed. Verified: app gate clean,
  `app_it` 361, host 120, Smash 18.
- ▢ *deferred* — **production two-view composition and layout.** Production still
  spawns **one** camera and publishes **one** screen rectangle to every view:
  `publish_camera_viewport` projects the single `ResolvedGameplayPresentation`, a
  fact about the physical screen, so distinct gameplay rectangles cannot come out
  of the shipped schedule **by construction**. ⚠ **and M2's own plan additionally
  names HUD ownership and input routing**, neither of which this slice touched.

⛔ **do not expand into networking**, and do not open an M3 on presentation. The
deferred half is gated on a real product need for a second view.

⛔ **`ControlledSubject` per-`PlayerSlot` is a PARTICIPANT PROJECTION, not part of
this slice.** View identity must not become synonymous with control authority: a
spectator view controls nothing, and a participant may be observed by two views.
It is 49 sites across 9 crates and the maintainer has not said whether to take it
whole — ⭐ ASK before starting it. Stop M2 once two simultaneous local views of
one simulation are structurally real; do not widen into networking or transport.

