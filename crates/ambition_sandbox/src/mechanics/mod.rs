//! Self-contained gameplay mechanics, each owning its own components, systems,
//! resources, and plugin registration (the Stage 12 `src/mechanics/` layout the
//! plugin-refactor plan targets). The first resident is [`gravity`], extracted
//! out of `crate::portal` (Stage 6 follow-up).

pub mod gravity;
