# Parallax overlay: stale-base module graph clobber

Error class: stale/cross-patch overlay clobbered module declarations. Occurrence count: 1.

The parallax overlay was generated from a clean source snapshot that did not include the prior `content_validation` module declaration in `lib.rs`, but it was applied to a worktree where `app.rs` and `app/resources.rs` still referenced `crate::content_validation`. Because overlay ZIPs replace whole files, the parallax `lib.rs` removed the module declaration and left the worktree in an inconsistent cross-patch state.

Lesson: when generating an overlay from a fresh archive while the user may have accepted previous patches, avoid replacing broad module-root files unless necessary. For `lib.rs` and other module graph files, merge new module declarations into the user's current shape or explicitly preserve recently added modules.

Tell: a new feature patch fails with `unresolved import crate::<module>` for a module that was introduced in a previous accepted patch, while the current patch did not intend to touch that feature.
