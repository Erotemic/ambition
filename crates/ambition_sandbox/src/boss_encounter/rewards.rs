// Boss reward chest sync is handled by `sync_boss_reward_chests_ecs` in
// `crate::features::ecs`. This file is intentionally empty; retained as a
// module boundary so `pub use rewards::*` imports in the parent compile cleanly
// while callers are updated to the ECS system.
