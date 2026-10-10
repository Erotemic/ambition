//! The architecture of the two-state look, as textures.
//!
//! [`super::architecture`] says what each piece of the architecture looks
//! like. This module draws each piece one time, in its two states, into a
//! page of an atlas (a plate), and gives the room one quad for each piece
//! that shows the plate.
//!
//! The work for each frame is then two texture reads for each pixel and the
//! front of the room, which decides the state a point shows
//! (`room_plate.wgsl`). The art itself costs nothing after the room comes.
//!
//! A texel is one world px. The art has lines one px wide, so the plate is
//! read with a filter that keeps a texel sharp and blends only at its edge:
//! the art does not shimmer when the camera moves by a part of a pixel.

use bevy::{
    asset::RenderAssetUsages,
    image::ImageSampler,
    prelude::*,
    reflect::TypePath,
    render::render_resource::{AsBindGroup, Extent3d, TextureDimension, TextureFormat},
    shader::ShaderRef,
    sprite_render::{AlphaMode2d, Material2d},
    tasks::{ComputeTaskPool, TaskPool},
};

use super::architecture::{colour_at, Piece, State};

/// Texels of nothing around each cell of a page, so the filter does not read
/// the next cell.
const GUTTER: u32 = 2;
/// The largest page. It is the unit of the engine's upload budget for a
/// frame of visible gameplay (16 MiB: `render_asset_budget.rs`), so a room's
/// pages go to the GPU one for each frame and no frame pays for more. Each
/// device takes a texture of this size.
pub(super) const PAGE_MAX: u32 = 2048;
/// About how many texels one task draws.
const BAND_TEXELS: u32 = 48_000;

/// One piece of the architecture, drawn from its plate.
#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub struct RoomPlateMaterial {
    /// The piece: `min.x, min.y, size.x, size.y`.
    #[uniform(0)]
    pub piece: Vec4,
    /// The part of the world this quad shows: `min.x, min.y, size.x, size.y`.
    /// It is the piece and its pad, or one part of that for a large piece.
    #[uniform(1)]
    pub window: Vec4,
    /// `x` = the role of the piece (`ROLE_*` of `room_look.rs`). `y, z` =
    /// the size of the part of the window that the plate holds, from the
    /// upper left corner of the window. The rest of the window has no art.
    #[uniform(2)]
    pub kind: Vec4,
    /// A point on the room's front (`x, y`) and its normal (`z, w`), which
    /// points into the corrupted side.
    #[uniform(3)]
    pub front: Vec4,
    /// Where the window is on the plate, in texels: the clean state
    /// (`x, y`) and the corrupted state (`z, w`).
    #[uniform(4)]
    pub cells: Vec4,
    #[texture(5)]
    #[sampler(6)]
    pub plate: Handle<Image>,
}

impl Material2d for RoomPlateMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://ambition_content/presentation/shaders/room_plate.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

/// A part of the world that one quad shows.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Window {
    /// Index of the piece in the list the windows were made from.
    pub piece: usize,
    pub min: Vec2,
    /// The part of the window that a cell of a plate holds, from `min`, in
    /// texels, which are world px.
    pub size: UVec2,
    /// The size of the quad. It is `size`, or more where the piece goes on
    /// below its art (what falls from a platform is drawn with no plate).
    pub extent: Vec2,
}

/// A window with its place on a page.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Placed {
    pub window: Window,
    pub page: usize,
    /// The upper left texel of the clean state. The corrupted state is below
    /// it, a gutter away.
    pub clean: UVec2,
}

impl Placed {
    pub fn corrupt(&self) -> UVec2 {
        self.clean + UVec2::new(0, self.window.size.y + GUTTER)
    }
}

/// The windows of `pieces`: the art of each piece with its pad, on whole
/// texels. A piece that is larger than a page holds is cut into parts.
pub(super) fn windows_of(pieces: &[Piece], page: u32) -> Vec<Window> {
    // A cell holds the two states of a window, one above the other.
    let max = UVec2::new(page - 2 * GUTTER, (page - 3 * GUTTER) / 2);
    let mut windows = Vec::new();
    for (index, piece) in pieces.iter().enumerate() {
        let pad = Vec2::splat(piece.pad());
        let min = (piece.min - pad).floor();
        let size = ((piece.min + piece.art_size() + pad).ceil() - min).as_uvec2();
        // How far the piece goes on below its art.
        let below = (piece.size.y - piece.art_size().y).max(0.0);
        let mut y = 0;
        while y < size.y {
            let h = (size.y - y).min(max.y);
            let mut x = 0;
            while x < size.x {
                let w = (size.x - x).min(max.x);
                // The last row of parts takes what is below the art.
                let last_row = y + h == size.y;
                windows.push(Window {
                    piece: index,
                    min: min + Vec2::new(x as f32, y as f32),
                    size: UVec2::new(w, h),
                    extent: Vec2::new(w as f32, h as f32 + if last_row { below } else { 0.0 }),
                });
                x += w;
            }
            y += h;
        }
    }
    windows
}

/// Put each window on a page: shelves of cells, the tallest first. Gives the
/// places, and the size of each page.
pub(super) fn pack(windows: &[Window], page: u32) -> (Vec<Placed>, Vec<UVec2>) {
    let cell = |window: &Window| UVec2::new(window.size.x + 2 * GUTTER, 2 * window.size.y + 3 * GUTTER);
    let mut order: Vec<usize> = (0..windows.len()).collect();
    order.sort_by_key(|&index| (std::cmp::Reverse(cell(&windows[index]).y), index));
    let mut placed = vec![None; windows.len()];
    let mut pages: Vec<UVec2> = Vec::new();
    // The shelf that is filled now: where its next cell goes, and its height.
    let (mut at, mut shelf) = (UVec2::ZERO, 0);
    for index in order {
        let size = cell(&windows[index]);
        if pages.is_empty() || at.x + size.x > page {
            // The next shelf, or the next page when this one is full.
            at = UVec2::new(0, at.y + shelf);
            shelf = 0;
            if pages.is_empty() || at.y + size.y > page {
                pages.push(UVec2::ZERO);
                at = UVec2::ZERO;
            }
        }
        placed[index] = Some(Placed {
            window: windows[index],
            page: pages.len() - 1,
            clean: at + UVec2::splat(GUTTER),
        });
        shelf = shelf.max(size.y);
        at.x += size.x;
        let used = pages.last_mut().expect("a page was pushed for the first cell");
        *used = used.max(UVec2::new(at.x, at.y + shelf));
    }
    (placed.into_iter().map(|placed| placed.expect("each window was placed")).collect(), pages)
}

/// Rows of one state of one window: the work of one task.
#[derive(Clone, Copy)]
struct Band {
    placed: usize,
    state: State,
    rows: (u32, u32),
}

/// A display value in `[0, 1]` as the byte an sRGB texture stores for it. The
/// art is written in display values with a gamma of 2.2, as the shaders of
/// the look write them.
fn stored_byte(display: f32, alpha: f32) -> u8 {
    // Premultiplied in linear light: the filter blends linear values, and a
    // texel of nothing must add nothing.
    let linear = display.clamp(0.0, 1.0).powf(2.2) * alpha;
    let encoded = if linear <= 0.003_130_8 { linear * 12.92 } else { 1.055 * linear.powf(1.0 / 2.4) - 0.055 };
    (encoded * 255.0 + 0.5) as u8
}

/// [`stored_byte`] of each opaque display value, in steps of 1/4095.
fn opaque_bytes() -> Vec<u8> {
    (0..4096).map(|step| stored_byte(step as f32 / 4095.0, 1.0)).collect()
}

fn draw_band(pieces: &[Piece], placed: &Placed, band: Band, opaque: &[u8]) -> Vec<u8> {
    let window = placed.window;
    let piece = &pieces[window.piece];
    let mut out = Vec::with_capacity(((band.rows.1 - band.rows.0) * window.size.x * 4) as usize);
    for y in band.rows.0..band.rows.1 {
        for x in 0..window.size.x {
            let p = window.min + Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
            let colour = colour_at(piece, band.state, p);
            if colour.w <= 0.0 {
                out.extend_from_slice(&[0; 4]);
            } else if colour.w >= 1.0 {
                let byte = |value: f32| opaque[(value.clamp(0.0, 1.0) * 4095.0 + 0.5) as usize];
                out.extend_from_slice(&[byte(colour.x), byte(colour.y), byte(colour.z), 255]);
            } else {
                let byte = |value: f32| stored_byte(value, colour.w);
                out.extend_from_slice(&[byte(colour.x), byte(colour.y), byte(colour.z), (colour.w * 255.0 + 0.5) as u8]);
            }
        }
    }
    out
}

/// Draw each placed window, in its two states, into the pages. Each page is
/// RGBA, sRGB, premultiplied. The work is spread over the compute pool.
pub(super) fn draw_pages(pieces: &[Piece], placed: &[Placed], pages: &[UVec2]) -> Vec<Vec<u8>> {
    let mut bands = Vec::new();
    for (index, cell) in placed.iter().enumerate() {
        let rows = (BAND_TEXELS / cell.window.size.x.max(1)).max(1);
        for state in [State::Clean, State::Corrupt] {
            let mut from = 0;
            while from < cell.window.size.y {
                let to = (from + rows).min(cell.window.size.y);
                bands.push(Band { placed: index, state, rows: (from, to) });
                from = to;
            }
        }
    }
    let opaque = opaque_bytes();
    let opaque = opaque.as_slice();
    let pool = ComputeTaskPool::get_or_init(TaskPool::default);
    let drawn: Vec<(Band, Vec<u8>)> = pool.scope(|scope| {
        for band in bands {
            scope.spawn(async move { (band, draw_band(pieces, &placed[band.placed], band, opaque)) });
        }
    });
    let mut data: Vec<Vec<u8>> = pages.iter().map(|size| vec![0; (size.x * size.y * 4) as usize]).collect();
    for (band, texels) in drawn {
        let cell = &placed[band.placed];
        let origin = match band.state {
            State::Clean => cell.clean,
            State::Corrupt => cell.corrupt(),
        };
        let width = cell.window.size.x as usize * 4;
        let page_width = pages[cell.page].x as usize;
        for (row, line) in (band.rows.0..band.rows.1).zip(texels.chunks_exact(width)) {
            let start = ((origin.y + row) as usize * page_width + origin.x as usize) * 4;
            data[cell.page][start..start + width].copy_from_slice(line);
        }
    }
    data
}

/// One page as a texture the renderer keeps and the main world does not.
pub(super) fn page_image(size: UVec2, data: Vec<u8>) -> Image {
    let mut image = Image::new(
        Extent3d { width: size.x, height: size.y, depth_or_array_layers: 1 },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.sampler = ImageSampler::linear();
    image
}

#[cfg(test)]
mod tests {
    use super::super::architecture::Part;
    use super::*;

    fn pieces() -> Vec<Piece> {
        vec![
            Piece { part: Part::Surface, min: Vec2::new(0.0, 600.0), size: Vec2::new(900.0, 64.0) },
            Piece { part: Part::Surface, min: Vec2::new(320.0, 480.0), size: Vec2::new(256.0, 32.0) },
            Piece { part: Part::Underside, min: Vec2::new(320.0, 512.0), size: Vec2::new(256.0, 230.0) },
            Piece { part: Part::Door { aspect: 0.52 }, min: Vec2::new(700.0, 440.0), size: Vec2::new(96.0, 160.0) },
        ]
    }

    /// The texels a cell of `size` at `at` takes, with the gutter after it.
    /// Two such rects that do not overlap are a gutter apart.
    fn with_gutter(at: UVec2, size: UVec2) -> (UVec2, UVec2) {
        (at, at + size + UVec2::splat(GUTTER))
    }

    fn overlap(a: (UVec2, UVec2), b: (UVec2, UVec2)) -> bool {
        a.0.x < b.1.x && b.0.x < a.1.x && a.0.y < b.1.y && b.0.y < a.1.y
    }

    /// No two cells of a page are less than a gutter apart, each cell is on its
    /// page, and the windows of a piece are the piece and its pad with no
    /// gap. A page small enough that pieces are cut and pages are many.
    #[test]
    fn cells_do_not_touch_and_windows_cover_their_piece() {
        let pieces = pieces();
        for page in [PAGE_MAX, 256] {
            let windows = windows_of(&pieces, page);
            let (placed, pages) = pack(&windows, page);
            assert_eq!(placed.len(), windows.len());
            if page == 256 {
                assert!(pages.len() > 1 && windows.len() > pieces.len(), "the small page did not cut a piece");
            }
            let mut cells = Vec::new();
            for cell in &placed {
                for at in [cell.clean, cell.corrupt()] {
                    let rect = with_gutter(at, cell.window.size);
                    let size = pages[cell.page];
                    assert!(rect.0.x >= GUTTER && rect.0.y >= GUTTER, "{rect:?} has no gutter before it");
                    assert!(rect.1.x <= size.x && rect.1.y <= size.y && size.x <= page && size.y <= page, "{rect:?} on {size}");
                    cells.push((cell.page, rect));
                }
            }
            for (i, a) in cells.iter().enumerate() {
                for b in &cells[i + 1..] {
                    assert!(a.0 != b.0 || !overlap(a.1, b.1), "two cells overlap: {a:?} {b:?}");
                }
            }
            for (index, piece) in pieces.iter().enumerate() {
                let area: u32 = windows.iter().filter(|w| w.piece == index).map(|w| w.size.x * w.size.y).sum();
                let whole = piece.art_size() + Vec2::splat(piece.pad() * 2.0);
                assert_eq!(area, (whole.x * whole.y) as u32, "piece {index}");
                // The quads of a piece cover the whole piece, art or not.
                let covered: f32 = windows.iter().filter(|w| w.piece == index).map(|w| w.extent.x * w.extent.y).sum();
                let quad = piece.size + Vec2::splat(piece.pad() * 2.0);
                assert_eq!(covered, quad.x * quad.y, "piece {index}");
            }
        }
    }

    /// A page holds what the architecture says: the texel of a world point is
    /// the colour of that point, in each state, and the gutter is empty.
    #[test]
    fn a_page_holds_the_colour_of_each_point_of_each_state() {
        let pieces = pieces();
        let windows = windows_of(&pieces, 256);
        let (placed, pages) = pack(&windows, 256);
        let data = draw_pages(&pieces, &placed, &pages);
        let texel = |page: usize, at: UVec2| {
            let start = ((at.y * pages[page].x + at.x) * 4) as usize;
            <[u8; 4]>::try_from(&data[page][start..start + 4]).unwrap()
        };
        let mut drawn = 0;
        for cell in &placed {
            let piece = &pieces[cell.window.piece];
            for (state, origin) in [(State::Clean, cell.clean), (State::Corrupt, cell.corrupt())] {
                // The corners and the middle of the cell.
                let last = cell.window.size - UVec2::ONE;
                for at in [UVec2::ZERO, last, UVec2::new(last.x, 0), UVec2::new(0, last.y), last / 2] {
                    let p = cell.window.min + at.as_vec2() + Vec2::splat(0.5);
                    let colour = colour_at(piece, state, p);
                    let want = [
                        stored_byte(colour.x, colour.w),
                        stored_byte(colour.y, colour.w),
                        stored_byte(colour.z, colour.w),
                        (colour.w * 255.0 + 0.5) as u8,
                    ];
                    let got = texel(cell.page, origin + at);
                    // The opaque table is in steps of 1/4095: one byte off at most.
                    for (got, want) in got.iter().zip(want) {
                        assert!(got.abs_diff(want) <= 1, "{:?} {state:?} at {at}: {got} for {want}", piece.part);
                    }
                    drawn += usize::from(colour.w > 0.0);
                }
                assert_eq!(texel(cell.page, origin - UVec2::ONE), [0; 4], "the gutter of a cell is not empty");
            }
        }
        assert!(drawn > placed.len(), "the samples saw almost no art: {drawn}");
    }
}
