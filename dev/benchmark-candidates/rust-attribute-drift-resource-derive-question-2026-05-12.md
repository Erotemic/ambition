# Benchmark candidate: Rust attribute drift turns a Bevy resource into a const annotation

## Failure

A patch inserted a new constant between an attribute and the item it was meant to annotate:

```rust
#[derive(Resource)]
pub const BLINK_IN_ANIM_TIME: f32 = 0.22;
pub const ROOM_DOOR_CAMERA_SNAP_TIME: f32 = 0.08;

pub struct SandboxRuntime { ... }
```

Rust then correctly reported:

```text
error[E0774]: `derive` may only be applied to `struct`s, `enum`s and `union`s
```

The follow-on errors were misleading but predictable: because `#[derive(Resource)]` no longer applied to `SandboxRuntime`, every system using `Res<SandboxRuntime>` or `ResMut<SandboxRuntime>` failed with `SandboxRuntime is not a Resource`.

## Why this is hard for current LLMs

The first error is small and local, but it causes a large cascade across unrelated systems. A model may chase the repeated `SandboxRuntime is not a Resource` errors instead of noticing the misplaced attribute immediately above the new const.

## Desired behavior

Given a Rust/Bevy compile log with `derive may only be applied to structs` followed by many `T is not a Resource` errors:

1. Inspect the line reported by E0774 first.
2. Check whether an attribute that used to annotate a struct was separated from it by a newly inserted item.
3. Move the new item above the attribute or below the annotated struct.
4. Do not add redundant `impl Resource` or new wrapper types until the misplaced attribute is ruled out.

## Minimal fix

```rust
pub const BLINK_IN_ANIM_TIME: f32 = 0.22;
pub const ROOM_DOOR_CAMERA_SNAP_TIME: f32 = 0.08;

#[derive(Resource)]
pub struct SandboxRuntime { ... }
```
