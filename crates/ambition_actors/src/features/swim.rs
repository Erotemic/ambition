//! Sandbox-side swim adapter.
//!
//! All gameplay-meaningful water response (drowning without ability,
//! Mario-style jump→swim conversion, passive buoyancy / drag / fall
//! cap) now lives in `ambition_engine_core::movement` so a single tick
//! sees consistent water state. This module is intentionally a thin
//! shim: it just owns the test fixtures we use to pin water behavior
//! end-to-end.
//!
//! Source-agnostic: the engine queries `World::water_at(player_aabb)`
//! once per simulation tick. Authoring layer (LDtk IntGrid `Water` or
//! entity `WaterVolume`) chooses which regions exist; the runtime
//! never branches on which authoring source produced a region.

#[cfg(test)]
mod tests;
