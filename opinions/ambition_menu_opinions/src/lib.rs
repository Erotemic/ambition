//! Optional pages for the basic pause menu.

#[cfg(feature = "rebind")]
mod rebind;
#[cfg(feature = "rebind")]
pub use rebind::RebindPausePage;

#[cfg(feature = "video")]
mod video;
#[cfg(feature = "video")]
pub use video::{VideoMenuOption, VideoPausePage};

#[cfg(all(test, any(feature = "video", feature = "rebind")))]
mod tests;
