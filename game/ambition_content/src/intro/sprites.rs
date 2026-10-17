//! Intro prop sheets.
//!
//! The props the intro rooms author (the cart, the creator lab's props, the
//! cut-rope arena's props, the gate ring and its portal) are rows of the
//! pack's `data/prop_sheets.ron`, keyed by `Prop.kind`, as every other prop
//! of the game is.

// There is no intro NPC sprite table. Every `NpcSpawn` in `intro.ldtk`
// carries a `character_id`, `convert_npc_spawn` puts it into
// `InteractionKindSpec::Npc.character_id`, and `demand_worn_character_sheets`
// raises that sheet on room entry. A table keyed by display name would decode
// art that no lookup reaches, and would add a second preload road.
//
// Props are rows of the pack's prop sheets: a `Prop` is keyed by
// `Prop.kind`, which the world authors.

/// The kind of the cart of the wake room: a prop sheet, and the `visual` of
/// the platform that is the cart. How it is laid on that platform, and the
/// first step of its ramp (`intro_cart_ramp`), are rows of the pack's
/// `platform_looks.ron`.
pub const INTRO_CART_KIND: &str = "intro_cart";

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
        let rows = ambition_sprite_sheet::prop_sheets::lowered_prop_sheets(crate::pack::shipped()).expect("the pack states its prop sheets");
        let row = &rows[INTRO_CART_KIND];
        let spec = ambition_sprite_sheet::character::try_load_spec_for_target(row.target(INTRO_CART_KIND), &row.tuning())
            .expect("the cart's baked sheet");
        let frame = spec.frame_pixels();
        let looks = ambition_sprite_sheet::prop_sheets::lowered_platform_looks(crate::pack::shipped())
            .expect("the pack states its platform looks");
        let (cart, ramp) = (&looks[INTRO_CART_KIND], &looks["intro_cart_ramp"]);
        assert_eq!(ramp.sheet.as_deref(), Some(INTRO_CART_KIND), "the ramp is a row of the cart's sheet");
        for look in [cart, ramp] {
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
