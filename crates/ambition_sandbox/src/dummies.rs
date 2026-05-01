//! Legacy training-target adapter removed.
//!
//! Sandbags are authored as `RoomObjectKind::EnemySpawn` objects with
//! `EnemyBrain::Custom("sandbag_infinite")` or `EnemyBrain::Custom("sandbag_finite")`
//! and run through `features::EnemyRuntime` so health bars, slash damage,
//! debug volumes, and room placement share the same code path as enemies.
