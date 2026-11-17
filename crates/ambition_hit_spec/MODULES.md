# Hit policy

`src/lib.rs` owns the shared contact values. Content, projectile ports, and body
reactions use these values. This crate has no ECS or simulation dependency.

See `../../docs/systems/hit-properties.md` for the runtime rules.
