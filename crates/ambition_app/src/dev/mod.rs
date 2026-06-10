//! App-level dev presentation: the F1 debug overlay and the F3 FPS
//! counter. These are pure presentation/host systems with no lib
//! consumer, moved up from `ambition_sandbox::dev` (Stage 20 devtools
//! split). The lib keeps the dev STATE (`DeveloperTools` + editable
//! profiles, read by persistence/presentation), the gameplay `trace`
//! recorder (written by sim code), and `profiling` (read by audio).
pub mod debug_overlay;
pub mod fps_overlay;
