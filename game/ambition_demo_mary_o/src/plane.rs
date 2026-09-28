//! Flying Mary-O enemy archetypes.
//! Aerial/float behavior and art are character-authored.

/// Brain key carried by the level placement alongside `character_id`.
pub const PAPER_PLANE_BRAIN_KEY: &str = "mary_o_snakes_on_a_paper_plane";
/// See [`PAPER_PLANE_BRAIN_KEY`].
pub const CARTESIAN_PLANE_BRAIN_KEY: &str = "mary_o_snakes_on_a_cartesian_plane";

/// Character id authored by the level placement.
pub const PAPER_PLANE_CHARACTER_ID: &str = "npc_snakes_on_a_paper_plane";
/// See [`PAPER_PLANE_CHARACTER_ID`].
pub const CARTESIAN_PLANE_CHARACTER_ID: &str = "npc_snakes_on_a_cartesian_plane";

/// Sprite-sheet target; deliberately distinct from the catalog character id.
pub const PAPER_PLANE_SHEET_TARGET: &str = "snakes_on_a_paper_plane";
/// See [`PAPER_PLANE_SHEET_TARGET`].
pub const CARTESIAN_PLANE_SHEET_TARGET: &str = "snakes_on_a_cartesian_plane";

/// Display name used by the enemy-render lookup.
pub const PAPER_PLANE_DISPLAY_NAME: &str = "Snakes on a Paper Plane";
/// See [`PAPER_PLANE_DISPLAY_NAME`].
pub const CARTESIAN_PLANE_DISPLAY_NAME: &str = "Snakes on a Cartesian Plane";

