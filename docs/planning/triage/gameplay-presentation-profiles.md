# Gameplay presentation profiles — remaining work

**Status:** GP1–GP5 are built: profile resolution, fixed/aspect viewport policy,
surround layout, provider profile declaration, occupancy/control regions, touch
placement, and the player HUD as the first surround-region consumer. Source
files cite this page as the design of record:
`crates/ambition_platformer2d_shared_tangle/src/gameplay_presentation/mod.rs`,
`crates/ambition_platformer2d_host/src/gameplay_presentation.rs` and
`crates/ambition_render/src/gameplay_surround.rs`.

## Finding

`DisplaySafeAreaInsets` is wired end to end except for its producer. It is
defined in `shared_tangle/src/gameplay_presentation/mod.rs`, `init_resource`'d
in `ambition_platformer2d_host/src/gameplay_presentation.rs`, and read by
`resolve_host_gameplay_presentation`, which feeds the whole layout resolve.
Nothing writes a non-zero value, so every layout uses zero insets. On a display
with a notch, a cutout or rounded corners, gameplay is laid out into the unsafe
region. Desktop runs and headless tests cannot show this; closing it needs a
device (same family as the Android font path in
[checks that did not run](../../recipes/checks-that-did-not-run.md)).

Other remaining work, each only when a real case needs it:

- **Overlap fallbacks.** Reposition contextual controls, fade presentation near
  the controlled subject, strengthen silhouette/readability. Build from
  observed overlap cases, not as a speculative framework.
- **Participant-facing layout preference.** Add it only when the settings/UI
  owner is clear.
- **Authored surround art.** `GameAuthored` and `DecorativeWorldExtension` need
  a content path that supplies authored surround art, not only the base fill.
- **Overlays on computed layout.** Quest, debug, map and dialogue surfaces
  consume resolved regions where they compete with gameplay or control space.
  Do not build one responsive-HUD manager.

## Evidence command

```bash
rg -n "DisplaySafeAreaInsets" crates game --type rust
```

A fix shows a production writer (`ResMut<DisplaySafeAreaInsets>` or an
`insert_resource` with a value).

## Owner

None.

## Trigger to promote

A shipped target with a notch, cutout or rounded corners (Android, iOS or a web
build on a phone), or a reported overlap between gameplay and a HUD/control
region.
