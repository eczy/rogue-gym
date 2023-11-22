//! Symbol representation of tiles for neural network
use crate::tile::Tile;
use rect_iter::Get2D;
use thiserror::Error;

/// Symbol
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Symbol(u8);

impl Symbol {
    pub fn to_byte(self) -> u8 {
        self.0
    }
    pub fn decrement(self) -> Self {
        Symbol(self.0 - 1)
    }
    pub fn from_tile(t: Tile) -> Option<Symbol> {
        let sym = |u| Some(Symbol(u));
        match t.to_byte() {
            b' ' => sym(0),
            b'@' => sym(1),
            b'#' => sym(2),
            b'.' => sym(3),
            b'-' | b'|' => sym(4),
            b'%' => sym(5),
            b'+' => sym(6),
            b'^' => sym(7),
            b'!' => sym(8),
            b'?' => sym(9),
            b']' => sym(10),
            b')' => sym(11),
            b'/' => sym(12),
            b'*' => sym(13),
            b':' => sym(14),
            b'=' => sym(15),
            b',' => sym(16),
            x if b'A' <= x && x <= b'Z' => sym(x - b'A' + 17),
            _ => None,
        }
    }
}

pub fn tile_to_sym(t: u8) -> Option<u8> {
    Symbol::from_tile(Tile::from(t)).map(|s| s.0)
}

#[derive(Clone, Copy, Debug, Error)]
#[error("Invalid tile: {}, while max is {}", _0, _1)]
pub struct InvalidTileError(Tile, u8);

pub fn construct_symbol_map<'c>(
    map: &impl Get2D<Item = u8>,
    h: usize,
    w: usize,
    symbol_max: u8,
    mut res: impl 'c + FnMut([usize; 3]) -> &'c mut f32,
) -> Result<(), InvalidTileError> {
    for i in 0..usize::from(symbol_max) {
        for y in 0..h {
            for x in 0..w {
                let t = *map.get_xy(x, y);
                let sym = tile_to_sym(t).ok_or_else(|| InvalidTileError(t.into(), symbol_max))?;
                if sym >= symbol_max {
                    return Err(InvalidTileError(t.into(), symbol_max));
                }
                *res([i, y, x]) = if usize::from(sym) == i { 1.0 } else { 0.0 };
            }
        }
    }
    Ok(())
}
