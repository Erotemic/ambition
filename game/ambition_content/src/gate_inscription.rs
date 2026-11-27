//! Ambition's gate inscription: how an address is spelled on a ring.
//!
//! The engine dials opaque chevron sequences ([`GateAddressRule`] is all it
//! asks of a spelling). Ambition's rule (Jon, 2026-10-24): an address is at
//! most [`MAX_ADDRESS_LEN`] chevrons, each used at most [`MAX_CHEVRON_USES`]
//! times, because a ring shows an address by lighting, for each chevron it
//! uses, a POSITION GRID ([`position_grid`]): one row per use, the bits of
//! that use's place. The lit ring spells the whole address, its order too
//! ([`ring_lit_marks`]).

use ambition_platformer2d::world::rooms::{Chevron, DialRefusal, GateAddressRule};

/// The bits of a place in an address.
pub const GRID_BITS: usize = 4;
/// The longest address: a place in it fits in [`GRID_BITS`] bits.
pub const MAX_ADDRESS_LEN: usize = 1 << GRID_BITS;
/// The most times one chevron is used in an address.
pub const MAX_CHEVRON_USES: usize = 4;

/// The position grid of `chevron` in `address`: row k is the chevron's k-th
/// use, its columns the bits of that use's place in the address, most
/// significant first. Uses past [`MAX_CHEVRON_USES`] are not drawn.
pub fn position_grid(address: &[Chevron], chevron: Chevron) -> [[bool; GRID_BITS]; MAX_CHEVRON_USES] {
    let mut grid = [[false; GRID_BITS]; MAX_CHEVRON_USES];
    let uses = address.iter().enumerate().filter(|(_, used)| **used == chevron).map(|(at, _)| at);
    for (row, at) in uses.take(MAX_CHEVRON_USES).enumerate() {
        for (bit, cell) in grid[row].iter_mut().enumerate() {
            *cell = at >> (GRID_BITS - 1 - bit) & 1 == 1;
        }
    }
    grid
}

/// Whether `address` keeps the limits: at most [`MAX_ADDRESS_LEN`]
/// chevrons, each used at most [`MAX_CHEVRON_USES`] times.
pub fn address_fits(address: &[Chevron]) -> bool {
    address.len() <= MAX_ADDRESS_LEN
        && address.iter().all(|chevron| address.iter().filter(|used| *used == chevron).count() <= MAX_CHEVRON_USES)
}

/// The rule the engine's dial asks.
pub fn address_rule() -> GateAddressRule {
    GateAddressRule(address_fits)
}

/// What a terminal shows for a refused dial: the engine's words, except
/// where Ambition's spelling is the reason.
pub fn refusal_words(refusal: &DialRefusal) -> String {
    match refusal {
        DialRefusal::Malformed => format!(
            "An address is at most {MAX_ADDRESS_LEN} chevrons, each used at most {MAX_CHEVRON_USES} times."
        ),
        other => other.message(),
    }
}

/// What lights on a ring's plaque: its glyph, or one cell of its position
/// grid, a set bit or a used row's clear bit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum GateRingMark {
    Glyph(Chevron),
    Bit { chevron: Chevron, cell: u8 },
    Used { chevron: Chevron, cell: u8 },
}

impl GateRingMark {
    /// The row of the ring's glyph sheet, and the middle of the mark's frame
    /// as a fraction of the row, for a sheet of `chevrons` glyphs.
    pub fn frame(self, chevrons: usize) -> (&'static str, f32) {
        let cells = (MAX_CHEVRON_USES * GRID_BITS) as f32;
        let (row, index, count) = match self {
            Self::Glyph(chevron) => ("lit", f32::from(chevron), chevrons as f32),
            Self::Bit { chevron, cell } => ("bit", f32::from(chevron) * cells + f32::from(cell), chevrons as f32 * cells),
            Self::Used { chevron, cell } => ("used", f32::from(chevron) * cells + f32::from(cell), chevrons as f32 * cells),
        };
        (row, (index + 0.5) / count)
    }
}

/// The marks `address` lights: each chevron it uses once, and each cell of
/// that chevron's used rows ([`position_grid`]), so the lit ring spells the
/// whole address, its order too.
pub fn ring_lit_marks(address: &[Chevron]) -> Vec<GateRingMark> {
    let mut chevrons = address.to_vec();
    chevrons.sort_unstable();
    chevrons.dedup();
    let mut marks = Vec::new();
    for chevron in chevrons {
        marks.push(GateRingMark::Glyph(chevron));
        let uses = address.iter().filter(|used| **used == chevron).count().min(MAX_CHEVRON_USES);
        for (row, bits) in position_grid(address, chevron).iter().enumerate().take(uses) {
            for (column, set) in bits.iter().enumerate() {
                let cell = (row * GRID_BITS + column) as u8;
                marks.push(if *set { GateRingMark::Bit { chevron, cell } } else { GateRingMark::Used { chevron, cell } });
            }
        }
    }
    marks
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spelled(word: &str) -> Vec<Chevron> {
        crate::gate_network::lowered_gate_network(crate::pack::shipped()).spell(word).expect("the word spells")
    }

    /// A chevron's grid: row k is its k-th use, the columns its place in
    /// bits, most significant first; the grids of a lit ring spell the
    /// address back.
    #[test]
    fn the_position_grids_spell_the_address() {
        let tangent = spelled("TANGENT");
        let nu = tangent[2];
        let grid = position_grid(&tangent, nu);
        assert_eq!(grid[0], [false, false, true, false], "first N at place 2");
        assert_eq!(grid[1], [false, true, false, true], "second N at place 5");
        assert_eq!(grid[2], [false; 4]);
        let mut back = vec![None; tangent.len()];
        for chevron in 0..15u8 {
            for row in position_grid(&tangent, chevron) {
                if row.iter().any(|bit| *bit) || (chevron == tangent[0] && back[0].is_none()) {
                    let at = row.iter().fold(0, |n, bit| n * 2 + usize::from(*bit));
                    if at < back.len() && back[at].is_none() && tangent[at] == chevron {
                        back[at] = Some(chevron);
                    }
                }
            }
        }
        assert_eq!(back.into_iter().map(Option::unwrap).collect::<Vec<_>>(), tangent);
        assert!(address_fits(&[3; 4]) && !address_fits(&[3; 5]) && !address_fits(&[0; 17]));
    }

    /// Every ring Ambition ships keeps the limits its grids can show (the
    /// engine's catalog check does not know the spelling).
    #[test]
    fn every_shipped_address_keeps_the_inscription() {
        let catalog = crate::gate_network::lowered_gate_network(crate::pack::shipped());
        for ring in &catalog.rings {
            let address = catalog.address_of(ring).expect("every ring has an address");
            assert!(address_fits(&address), "`{}`'s address {address:?} does not fit the inscription", ring.zone);
        }
    }

    /// An address lights each chevron's glyph once and a row of its grid
    /// per use: the place's bits bright, the rest of the row dim.
    #[test]
    fn a_lit_address_spells_its_order_in_the_grids() {
        // Chevron 3 at places 0 and 5 (0b0101), chevron 1 at place 1 (0b0001).
        let marks = ring_lit_marks(&[3, 1, 9, 9, 9, 3]);
        let of = |chevron| marks.iter().filter(|mark| match mark {
            GateRingMark::Glyph(c) => *c == chevron,
            GateRingMark::Bit { chevron: c, .. } | GateRingMark::Used { chevron: c, .. } => *c == chevron,
        }).copied().collect::<Vec<_>>();
        let three = of(3);
        assert_eq!(three.len(), 1 + 2 * 4, "{three:?}");
        assert!(three.contains(&GateRingMark::Glyph(3)));
        for cell in [0, 1, 2, 3, 4, 6] {
            assert!(three.contains(&GateRingMark::Used { chevron: 3, cell }), "{cell}");
        }
        for cell in [5, 7] {
            assert!(three.contains(&GateRingMark::Bit { chevron: 3, cell }), "{cell}");
        }
        assert!(of(1).contains(&GateRingMark::Bit { chevron: 1, cell: 3 }));
        assert_eq!(of(9).len(), 1 + 3 * 4);
        assert!(of(0).is_empty());
    }

    /// A refusal for the spelling is worded with Ambition's limits.
    #[test]
    fn a_malformed_address_is_worded_by_the_game() {
        assert!(refusal_words(&DialRefusal::Malformed).contains("at most 16 chevrons"));
        assert_eq!(refusal_words(&DialRefusal::Unknown), DialRefusal::Unknown.message());
    }
}
