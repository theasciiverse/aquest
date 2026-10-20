//! The 320x240 picture in the terminal. Two ways to draw it:
//!
//! - Blocks: half-block cells (`▀`), two pixels a cell, each its own
//!   colour. The original's look.
//! - Braille: 2x4 dots a cell in one colour, four times the detail.
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
        View::Braille => (2, 4),
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
        View::Braille => {
            // dot bits by (column, row), as Unicode lays them out
            const BIT: [[u32; 4]; 2] = [[0x01, 0x02, 0x04, 0x40], [0x08, 0x10, 0x20, 0x80]];
            for cy in 0..r.height {
                for cx in 0..r.width {
                    let mut bits = 0;
                    let mut best = [0u8; 3];
                    let mut bl = 0;
                    for (dx, col) in BIT.iter().enumerate() {
                        for (dy, &bit) in col.iter().enumerate() {
                            let c = s.at(cx as usize * 2 + dx, cy as usize * 4 + dy);
                            let l = lum(c);
                            if l > 160 {
                                bits |= bit;
                            }
                            if l > bl {
                                bl = l;
                                best = c;
                            }
                        }
                    }
                    let ch = char::from_u32(0x2800 + bits).unwrap_or(' ');
                    buf[(r.x + cx, r.y + cy)].set_char(ch).set_fg(rgb(best)).set_bg(black);
                }
            }
        }
    }
}
