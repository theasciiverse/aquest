//! The 320x240 picture in the terminal. Two ways to draw it:
//!
//! - Blocks: half-block cells (`▀`), two pixels a cell, each its own
//!   colour. The original's look.
//! - Braille: every cell Braille, at the full 2x4 detail, in two colours
//!   (see `mixed_cell`), so lettering, shading and colours show.
//! - Mixed (the default): Braille's detail with colour. Sprite edges are
//!   dots in the cell's brightest colour, sprite insides half blocks or
//!   two-colour dots, so the smaller the picture, the more of it is
//!   Braille; from about 200 columns it is all blocks, which resolve the
//!   sprites by then and look more solid.
//!
//! The picture keeps the original's square pixels: its size comes from the
//! terminal's cell shape. When it is shrunk, each terminal pixel shows the
//! brightest of the pixels it covers, so one-pixel stars and shots never
//! vanish.

use crate::frame::{Frame, Rgb, W};
use crate::save::View;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;

/// Height over width of a terminal cell.
pub fn cell_aspect() -> f32 {
    match ratatui::crossterm::terminal::window_size() {
        Ok(w) if w.width > 0 && w.height > 0 && w.columns > 0 && w.rows > 0 => {
            let a = (w.height as f32 / w.rows as f32) / (w.width as f32 / w.columns as f32);
            if (1.0..4.0).contains(&a) { a } else { 2.0 }
        }
        _ => 2.0,
    }
}

/// Dots a cell holds across and down.
pub fn dots(v: View) -> (u16, u16) {
    match v {
        View::Blocks => (1, 2),
        View::Braille | View::Mixed => (2, 4),
    }
}

/// Where a `sw` x `sh` source goes in `area`: the biggest rectangle of its
/// shape, centred, and its size in dots.
pub struct Placed {
    pub cells: Rect,
    pub tw: u16,
    pub th: u16,
}

pub fn place(area: Rect, sw: i32, sh: i32, v: View, aspect: f32) -> Placed {
    let (cx, cy) = dots(v);
    // a dot is aspect/2 times as tall as it is wide, in either view
    let dot_h = aspect / 2.0;
    let max_w = (area.width * cx) as f32;
    let max_h = (area.height * cy) as f32;
    let ratio = sw as f32 / sh as f32;
    let tw = max_w.min(ratio * max_h * dot_h);
    let th = (tw / (ratio * dot_h)).min(max_h);
    let cols = ((tw / cx as f32).round() as u16).clamp(1, area.width);
    let rows = ((th / cy as f32).round() as u16).clamp(1, area.height);
    let cells = Rect::new(area.x + (area.width - cols) / 2, area.y + (area.height - rows) / 2, cols, rows);
    Placed { cells, tw: cols * cx, th: rows * cy }
}

fn lum(c: Rgb) -> u32 {
    c[0] as u32 * 2 + c[1] as u32 * 5 + c[2] as u32
}

/// Squared distance between two colours.
fn dist(a: Rgb, b: Rgb) -> u32 {
    let d = |i: usize| (a[i] as i32 - b[i] as i32).pow(2) as u32;
    d(0) + d(1) + d(2)
}

fn brightest(cs: &[Rgb]) -> Rgb {
    cs.iter().copied().max_by_key(|&c| lum(c)).unwrap_or([0; 3])
}

/// Nearly black: no channel above this. Drawn as black, which is space.
fn dark(c: Rgb) -> bool {
    c.iter().all(|&v| v < 48)
}

/// From this scale (half-block pixels per original pixel) Mixed is all
/// blocks: half blocks already resolve the sprites there, and solid colour
/// beats the thinner dots. 0.6 is about 200 columns.
const BLOCKS_FROM: f32 = 0.6;

/// Two colours this close are one colour as far as a cell can show.
const SAME: u32 = 3 * 40 * 40;

/// How one Mixed cell is drawn.
enum Cell {
    Solid(Rgb),
    Half(Rgb, Rgb),
    /// Dot bits, dot colour, background colour.
    Dots(u32, Rgb, Rgb),
}

/// A patch as a half block: the brightest of each half, as Blocks does it.
fn half_cell(p: &[[Rgb; 4]; 2]) -> Cell {
    let top = brightest(&[p[0][0], p[0][1], p[1][0], p[1][1]]);
    let bot = brightest(&[p[0][2], p[0][3], p[1][2], p[1][3]]);
    if top == bot { Cell::Solid(top) } else { Cell::Half(top, bot) }
}

/// One cell's 2x4 patch (`p[x][y]`) in at most two colours.
///
/// The background is the colour covering most of the cell (space's black,
/// a window's grey), since a background floods the whole cell, gaps
/// included. The dots are the colour that stands out most from it, by how
/// far it is and how much of it there is (a crystal's cyan, a letter's
/// highlight). Every other sample joins whichever of the two it is nearer:
/// a letter's dark shadow melts into the grey window, a crystal's blue
/// edge joins its cyan outline. So a star is a white dot on black and a
/// sprite's edge is its outline in dots, at the full 2x4 detail.
///
/// With `blocks` (Mixed), a cell in one colour, or two split top and
/// bottom, is a half block, which shows it more solidly. Small pictures are
/// mostly edges, so mostly Braille; bigger ones have more insides, so more
/// blocks.
fn mixed_cell(p: &[[Rgb; 4]; 2], blocks: bool) -> Cell {
    // dot bits by (column, row), as Unicode lays them out
    const BIT: [[u32; 4]; 2] = [[0x01, 0x02, 0x04, 0x40], [0x08, 0x10, 0x20, 0x80]];
    const TOP: u32 = 0x1B;
    let at = |i: usize| {
        let c = p[i / 4][i % 4];
        if dark(c) { [0; 3] } else { c }
    };
    let bit = |i: usize| BIT[i / 4][i % 4];

    // the lit colours of the patch: (first, brightest, how many), close
    // colours counted as one; space (black) is counted apart
    let n_dark = (0..8).filter(|&i| at(i) == [0; 3]).count() as u32;
    if n_dark == 8 {
        return Cell::Solid([0; 3]);
    }
    let mut groups: Vec<(Rgb, Rgb, u32)> = Vec::new();
    for i in 0..8 {
        let c = at(i);
        if c == [0; 3] {
            continue;
        }
        match groups.iter_mut().find(|g| dist(g.0, c) < SAME) {
            Some(g) => {
                g.2 += 1;
                if lum(c) > lum(g.1) {
                    g.1 = c;
                }
            }
            None => groups.push((c, c, 1)),
        }
    }
    // over space (two or more dark samples) the background is black and
    // every lit sample a dot, so outlines stay exact and thin parts thin;
    // inside a sprite, the most common colour, the darker on a tie
    let over_space = n_dark >= 2;
    if n_dark > 0 && !over_space {
        groups.push(([0; 3], [0; 3], n_dark));
    }
    let space = ([0u8; 3], [0u8; 3], n_dark);
    let bg = if over_space { space } else { *groups.iter().max_by_key(|g| (g.2, std::cmp::Reverse(lum(g.1)))).unwrap() };
    let Some(fg) = groups.iter().filter(|g| g.0 != bg.0).max_by_key(|g| g.2 as u64 * dist(g.1, bg.1) as u64).copied() else {
        return if blocks { half_cell(p) } else { Cell::Solid(bg.1) };
    };
    let (a, b) = (fg.1, bg.1);
    let mut bits = 0;
    for i in 0..8 {
        let c = at(i);
        let lit = if over_space { c != [0; 3] } else { dist(c, bg.0) >= SAME && dist(c, a) <= dist(c, b) };
        if lit {
            bits |= bit(i);
        }
    }
    if bits == 0 {
        return if blocks { half_cell(p) } else { Cell::Solid(b) };
    }
    if blocks && !over_space && (bits == TOP || bits == 0xFF & !TOP) {
        return half_cell(p);
    }
    Cell::Dots(bits, a, b)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CYAN: Rgb = [0, 252, 252];
    const BLUE: Rgb = [0, 0, 252];
    const K: Rgb = [0, 0, 0];

    #[test]
    fn mixed_cells_follow_coverage() {
        // empty: space
        assert!(matches!(mixed_cell(&[[K; 4]; 2], true), Cell::Solid(c) if c == K));
        // a sprite's edge: dots on black, in the brightest colour
        let edge = [[CYAN, BLUE, K, K], [K, K, K, K]];
        assert!(matches!(mixed_cell(&edge, true), Cell::Dots(0x03, c, b) if c == CYAN && b == K));
        // inside, one colour: a block
        assert!(matches!(mixed_cell(&[[BLUE; 4]; 2], true), Cell::Solid(c) if c == BLUE));
        // inside, two colours in a diagonal: dots of the brighter over the other
        let diag = [[CYAN, BLUE, CYAN, BLUE], [BLUE, CYAN, BLUE, CYAN]];
        assert!(matches!(mixed_cell(&diag, true), Cell::Dots(_, c, b) if c == CYAN && b == BLUE));
        // inside, split top and bottom: a half block
        let half = [[CYAN, CYAN, BLUE, BLUE], [CYAN, CYAN, BLUE, BLUE]];
        assert!(matches!(mixed_cell(&half, true), Cell::Half(t, b) if t == CYAN && b == BLUE));
        // the Braille view keeps it in dots, brighter over darker
        assert!(matches!(mixed_cell(&half, false), Cell::Dots(0x1B, c, b) if c == CYAN && b == BLUE));
        assert!(matches!(mixed_cell(&[[BLUE; 4]; 2], false), Cell::Solid(c) if c == BLUE));
    }
}

fn rgb(c: Rgb) -> Color {
    Color::Rgb(c[0], c[1], c[2])
}

/// The brightest source pixel under each dot of the target.
struct Sampler<'a> {
    f: &'a Frame,
    xs: Vec<(i32, i32)>,
    ys: Vec<(i32, i32)>,
}

impl<'a> Sampler<'a> {
    fn new(f: &'a Frame, src: (i32, i32, i32, i32), tw: u16, th: u16) -> Sampler<'a> {
        let (sx, sy, sw, sh) = src;
        let span = |n: u16, off: i32, len: i32| {
            (0..n as i32)
                .map(|t| {
                    let a = off + t * len / n as i32;
                    let b = (off + (t + 1) * len / n as i32).max(a + 1);
                    (a, b)
                })
                .collect()
        };
        Sampler { f, xs: span(tw, sx, sw), ys: span(th, sy, sh) }
    }

    fn at(&self, tx: usize, ty: usize) -> Rgb {
        let (x0, x1) = self.xs[tx];
        let (y0, y1) = self.ys[ty];
        let mut best = [0u8; 3];
        let mut bl = 0;
        for y in y0..y1 {
            for x in x0..x1 {
                let c = self.f.px[(y * W + x) as usize];
                let l = lum(c);
                if l > bl {
                    bl = l;
                    best = c;
                }
            }
        }
        best
    }
}

/// Each cell of `r` from its 2x4 patch, drawn the way `how` decides.
fn draw_cells(buf: &mut Buffer, s: &Sampler, r: Rect, how: impl Fn(&[[Rgb; 4]; 2]) -> Cell) {
    let black = Color::Rgb(0, 0, 0);
    for cy in 0..r.height {
        for cx in 0..r.width {
            let mut patch = [[[0u8; 3]; 4]; 2];
            for (x, col) in patch.iter_mut().enumerate() {
                for (y, c) in col.iter_mut().enumerate() {
                    *c = s.at(cx as usize * 2 + x, cy as usize * 4 + y);
                }
            }
            let cell = &mut buf[(r.x + cx, r.y + cy)];
            match how(&patch) {
                Cell::Solid(c) => {
                    cell.set_char(' ').set_fg(black).set_bg(rgb(c));
                }
                Cell::Half(t, b) => {
                    cell.set_char('▀').set_fg(rgb(t)).set_bg(rgb(b));
                }
                Cell::Dots(bits, fg, bg) => {
                    cell.set_char(char::from_u32(0x2800 + bits).unwrap_or(' ')).set_fg(rgb(fg)).set_bg(rgb(bg));
                }
            }
        }
    }
}

/// Draw `src` (x, y, w, h of the frame) into `p`.
pub fn draw(buf: &mut Buffer, f: &Frame, src: (i32, i32, i32, i32), p: &Placed, v: View) {
    let s = Sampler::new(f, src, p.tw, p.th);
    let r = p.cells;
    let black = Color::Rgb(0, 0, 0);
    match v {
        View::Blocks => {
            for cy in 0..r.height {
                for cx in 0..r.width {
                    let top = s.at(cx as usize, cy as usize * 2);
                    let bot = s.at(cx as usize, cy as usize * 2 + 1);
                    let cell = &mut buf[(r.x + cx, r.y + cy)];
                    if top == bot {
                        cell.set_char(' ').set_fg(black).set_bg(rgb(top));
                    } else {
                        cell.set_char('▀').set_fg(rgb(top)).set_bg(rgb(bot));
                    }
                }
            }
        }
        View::Mixed => {
            let all_blocks = p.tw as f32 / 2.0 / src.2 as f32 >= BLOCKS_FROM;
            draw_cells(buf, &s, r, |patch| if all_blocks { half_cell(patch) } else { mixed_cell(patch, true) });
        }
        View::Braille => draw_cells(buf, &s, r, |patch| mixed_cell(patch, false)),
    }
}
