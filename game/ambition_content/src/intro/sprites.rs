//! Intro prop sheets.
//!
//! [`intro_prop_sprite_rows`] lists the props the intro rooms author, keyed by
//! `Prop.kind`. `IntroPlugin` registers each row as a prop sheet, and the engine
//! loads them with the rest of the art. A row whose sheet does not load draws
//! as a placeholder rectangle.

use ambition_asset_manager::AssetId;

use ambition_sprite_sheet::character::{try_load_spec_for_target, CharacterSheetSpec, SheetTuning};

/// Resolve a content-owned sheet spec by manifest target, with intro
/// tuning. Panics in tests via the registry checks; at runtime a
/// missing manifest falls back to the colored-rectangle contract by
/// the caller skipping the row.
fn intro_sheet(target: &str, tuning: &SheetTuning) -> Option<CharacterSheetSpec> {
    try_load_spec_for_target(target, tuning)
}

// There is no intro NPC sprite table. Every `NpcSpawn` in `intro.ldtk`
// carries a `character_id`, `convert_npc_spawn` puts it into
// `InteractionKindSpec::Npc.character_id`, and `demand_worn_character_sheets`
// raises that sheet on room entry. A table keyed by display name would decode
// art that no lookup reaches, and would add a second preload road.
//
// Props keep their table below: a `Prop` is keyed by `Prop.kind`, which the
// world authors.

/// The kind of the cart of the wake room: a prop sheet, and the `visual` of
/// the platform that is the cart.
pub const INTRO_CART_KIND: &str = "intro_cart";

/// How the cart is drawn on its platform.
///
/// The cart is not a prop that stands on a platform: its bed IS the platform
/// (`MovingPlatform` with `visual: intro_cart` and `speed: 0`), and the sheet
/// is laid on it. The numbers are frame pixels of the sheet, and the art
/// module states them (`FRAME_SIZE`, `BED_SPAN_PX`, `BED_TOP_PX` in
/// `tools/ambition_sprite2d_renderer/.../targets/props/intro_cart.py`): the
/// span of the bed along the frame, and the row of its surface. The row
/// `front` is the near rail of the cart, drawn in front of each body, so a
/// body on the bed is in the cart.
pub fn intro_cart_platform_look() -> ambition_render::rendering::moving_platforms::PlatformLook {
    ambition_render::rendering::moving_platforms::PlatformLook {
        sheet: None,
        row: "idle".into(),
        frame_px: INTRO_CART_FRAME_PX,
        span_px: [108.0, 636.0],
        top_px: 126.0,
        faces_right: true,
        front_row: Some("front".into()),
    }
}

/// The frame of the sheet of the cart (`FRAME_SIZE` of the art module).
const INTRO_CART_FRAME_PX: [f32; 2] = [960.0, 312.0];

/// The `visual` of the platform that is the first step of the ramp of the
/// cart: it is drawn as the row `ramp` of the sheet of the cart.
///
/// The ramp is the back end of the cart, let down. Its collision is a row of
/// one-way steps (each 7 long and 4 under the one before), and all but the
/// first have no picture (`visual: none`). The numbers are those of the art
/// module (`RAMP_STEP_SPAN_PX`, `RAMP_STEP_TOP_PX`): the first step in frame
/// pixels. A cart with its ramp up has no such platform, and the row `gate`
/// of the sheet is the picture of its shut back end (not drawn yet: the cart
/// does not roll yet).
pub const INTRO_CART_RAMP_KIND: &str = "intro_cart_ramp";

/// How the ramp of the cart is drawn on its first step. See [`INTRO_CART_RAMP_KIND`].
pub fn intro_cart_ramp_look() -> ambition_render::rendering::moving_platforms::PlatformLook {
    ambition_render::rendering::moving_platforms::PlatformLook {
        sheet: Some(INTRO_CART_KIND.into()),
        row: "ramp".into(),
        frame_px: INTRO_CART_FRAME_PX,
        span_px: [636.0, 657.0],
        top_px: 138.0,
        faces_right: true,
        front_row: None,
    }
}

/// Prop tuning: props render at their authored AABB size.
const PROP_TUNING: SheetTuning = SheetTuning::new(1.00, 2);

/// `(Prop.kind, asset filename, sheet spec, pack target)` rows for intro
/// props (keyed by `Prop.kind` so LDtk renames don't re-point sprites).
/// Includes the cut-rope arena props until a dedicated non-intro prop
/// catalog exists.
pub fn intro_prop_sprite_rows() -> Vec<(
    &'static str,
    &'static str,
    CharacterSheetSpec,
    Option<&'static str>,
)> {
    let t = &PROP_TUNING;
    let mut rows: Vec<(&str, &str, Option<CharacterSheetSpec>, Option<&str>)> = vec![
        (
            "intro_cart",
            "intro_cart_spritesheet.png",
            intro_sheet("intro_cart", t),
            Some("intro_cart"),
        ),
        // Creator lab props — separate records inside the shared
        // creator_lab_props sheet.
        (
            "lab_genesis_vat",
            "creator_lab_props_spritesheet.png",
            intro_sheet("genesis_vat", t),
            None,
        ),
        (
            "lab_neural_console",
            "creator_lab_props_spritesheet.png",
            intro_sheet("neural_console", t),
            None,
        ),
        (
            "lab_power_core",
            "creator_lab_props_spritesheet.png",
            intro_sheet("power_core", t),
            None,
        ),
        (
            "lab_repair_cradle",
            "creator_lab_props_spritesheet.png",
            intro_sheet("repair_cradle", t),
            None,
        ),
        (
            "lab_resonance_coil",
            "creator_lab_props_spritesheet.png",
            intro_sheet("resonance_coil", t),
            None,
        ),
        // Cut-rope boss props.
        (
            "cut_rope_rope",
            "cut_rope_rope_spritesheet.png",
            intro_sheet("cut_rope_rope", t),
            None,
        ),
        (
            "cut_rope_anvil",
            "cut_rope_anvil_spritesheet.png",
            intro_sheet("cut_rope_anvil", t),
            None,
        ),
        (
            "cut_rope_piano",
            "cut_rope_piano_spritesheet.png",
            intro_sheet("cut_rope_piano", t),
            None,
        ),
        // The engine ships its own effect sheets (`ambition_sprite_sheet::fx::fx_sheets`);
        // a story's prop table does not declare them. Interdimensional gate ring and
        // portal surface.
        (
            "gate_ring",
            "interdimensional_gate_ring_spritesheet.png",
            intro_sheet("interdimensional_gate_ring", t),
            None,
        ),
        (
            "gate_portal",
            "interdimensional_gate_portal_spritesheet.png",
            intro_sheet("interdimensional_gate_portal", t),
            None,
        ),
    ];
    rows.drain(..)
        .filter_map(|(kind, file, spec, pack)| spec.map(|s| (kind, file, s, pack)))
        .collect()
}

/// Stable [`AssetId`] for an intro prop sprite. Namespace
/// `sprite.character.intro_prop_<lower_snake_kind>` — props share the
/// `sprite.character.*` namespace with NPCs because they ride the same
/// `CharacterSpriteAsset` runtime type, but the `intro_prop_` prefix
/// keeps the two cleanly separable.
pub fn intro_prop_asset_id(prop_kind: &str) -> AssetId {
    AssetId::new(format!(
        "sprite.character.intro_prop_{}",
        prop_kind.replace(['-', ' '], "_"),
    ))
}

use ambition_asset_manager::{
    AssetEntry, AssetKind, AssetManifest, MissingAssetPolicy, PreloadGroup,
};

/// via `catalog.try_path_for_load(...)` like every other loader.
///
/// IDs are `sprite.character.intro_<name_snake>` for NPCs and
/// `sprite.character.intro_prop_<kind_snake>` for props. Both use
/// `SilentPlaceholder` because missing intro art falls back to colored
/// rectangles per the existing contract.
/// It takes no cast: `Prop` rows resolve from their own table. A parameter
/// nobody reads would claim that this still knows about characters.
pub fn extend_with_intro_sprite_entries(manifest: &mut AssetManifest, sprite_folder: &str) {
    for (kind, filename, _spec, _pack) in intro_prop_sprite_rows() {
        let id = intro_prop_asset_id(kind);
        let logical_path = format!("{sprite_folder}/{filename}");
        manifest.insert(
            AssetEntry::new(id, AssetKind::Image, logical_path)
                .with_missing_policy(MissingAssetPolicy::SilentPlaceholder)
                .with_preload_group(PreloadGroup::SandboxCore),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The look of the cart names the bed in pixels of the frame of its sheet,
    /// and the art module makes that frame (`intro_cart.py`: `FRAME_SIZE`,
    /// `BED_SPAN_PX`, `BED_TOP_PX`). A sheet with another frame, or with no
    /// `front` row, puts the cart in the wrong place with no error: this reads
    /// the published sheet.
    #[test]
    fn the_look_of_the_cart_is_in_the_frame_of_its_published_sheet() {
        let spec = intro_sheet(INTRO_CART_KIND, &PROP_TUNING).expect("the cart's baked sheet");
        let frame = spec.frame_pixels();
        let (cart, ramp) = (intro_cart_platform_look(), intro_cart_ramp_look());
        for look in [&cart, &ramp] {
            assert_eq!([frame.x, frame.y], look.frame_px, "the frame the look was measured in");
            assert!(spec.clip_slot([look.row.as_str()]).is_some(), "the sheet has the row `{}`", look.row);
            assert!(0.0 <= look.span_px[0] && look.span_px[0] < look.span_px[1] && look.span_px[1] <= frame.x);
            assert!(0.0 < look.top_px && look.top_px < frame.y);
        }
        let front = cart.front_row.as_deref().expect("the cart has a near rail");
        assert!(spec.clip_slot([front]).is_some(), "the sheet has the row `{front}`");
        assert_ne!(spec.clip_slot([cart.row.as_str()]), spec.clip_slot([front]));
        assert_ne!(spec.clip_slot([cart.row.as_str()]), spec.clip_slot([ramp.row.as_str()]));
        // The ramp starts where the bed ends, one step under it.
        assert_eq!(ramp.span_px[0], cart.span_px[1]);
        assert!(ramp.top_px > cart.top_px);
    }
}
