//! Self-contained gameplay mechanics, each owning its own components, systems,
//! resources, and plugin registration (the Stage 12 `src/mechanics/` layout the
//! plugin-refactor plan targets). Residents: [`gravity`] (extracted out of
//! `crate::portal`, Stage 6 follow-up) and [`combat`] (the generic combat kit
//! extracted out of `content/features`, Stage 20 / A2).

pub mod combat;
pub mod gravity;
