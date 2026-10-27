//! The border, the gate and the enemy gates as line characters, wherever
//! the picture is Braille (David, 2026-10-27: "pipe graphics for the border
//! and where the enemies come out, instead of braille, to make them a bit
//! crisper"). Shrunk, Mark's five-line pipe straddles cells and comes out
//! as ragged dots; one double line a cell is the same wall, sharp at any
//! size. Where the picture is half blocks it keeps his pixels.

use crate::assets::Assets;
use crate::frame::{Rgb, W};
use crate::game::{Game, VIEWPORT_H, WORLD_H, WORLD_W};
use crate::save::View;
use crate::view::{Placed, cell_at};
use ratatui::buffer::Buffer;
use ratatui::style::Color;

/// The middle of the pipe, its brightest line, from each edge of the world.
const MID: i32 = 7;

struct Pen<'a> {
    buf: &'a mut Buffer,
    p: &'a Placed,
    v: View,
    g: &'a Game,
}

impl Pen<'_> {
    /// The cell of world pixel (wx, wy).
    fn cell(&self, wx: i32, wy: i32) -> (i32, i32) {
        cell_at(self.p, self.v, W, VIEWPORT_H, wx - self.g.cam_x, wy - self.g.cam_y)
    }

    fn put(&mut self, x: i32, y: i32, ch: char, c: Rgb) {
        let r = self.p.cells;
        if (0..r.width as i32).contains(&x) && (0..r.height as i32).contains(&y) {
            let cell = &mut self.buf[(r.x + x as u16, r.y + y as u16)];
            cell.set_char(ch).set_fg(Color::Rgb(c[0], c[1], c[2])).set_bg(Color::Rgb(0, 0, 0));
        }
    }

    /// Cells x1..=x2 of row y, clipped to the field.
    fn row(&mut self, x1: i32, x2: i32, y: i32, ch: char, c: Rgb) {
        for x in x1.max(0)..=x2.min(self.p.cells.width as i32 - 1) {
            self.put(x, y, ch, c);
        }
    }

    fn col(&mut self, x: i32, y1: i32, y2: i32, ch: char, c: Rgb) {
        for y in y1.max(0)..=y2.min(self.p.cells.height as i32 - 1) {
            self.put(x, y, ch, c);
        }
    }

    /// An enemy gate on the wall at cell column `wall`: a box `out` cells
    /// into the field (negative on the right), lit while an enemy comes in.
    fn enemy_gate(&mut self, a: &Assets, wall: i32, out: i32, top: i32, bot: i32, entering: i32) {
        let (wall_c, lit) = (a.palette[25], a.palette[0x45]);
        let c = if entering > 0 { lit } else { wall_c };
        let e = wall + out;
        let (x1, x2) = (wall.min(e) + 1, wall.max(e) - 1);
        let left = out > 0;
        self.put(wall, top, if left { '╠' } else { '╣' }, c);
        self.put(wall, bot, if left { '╠' } else { '╣' }, c);
        self.row(x1, x2, top, '═', c);
        self.row(x1, x2, bot, '═', c);
        self.put(e, top, if left { '╗' } else { '╔' }, c);
        self.put(e, bot, if left { '╝' } else { '╚' }, c);
        self.col(e, top + 1, bot - 1, '║', c);
        self.col(wall, top + 1, bot - 1, '║', c);
        // inside: the original's screen of moving dots while an enemy comes
        const SHADE: [char; 4] = ['░', '▒', '▓', '▒'];
        for y in top + 1..bot {
            for x in x1..=x2 {
                if entering > 0 {
                    let k = ((entering >> 2) + x + y).rem_euclid(4) as usize;
                    self.put(x, y, SHADE[k], a.palette[0x71]);
                } else {
                    self.put(x, y, ' ', [0; 3]);
                }
            }
        }
    }
}

/// Draw the border, gate and enemy gates over the field `p`.
pub fn draw(buf: &mut Buffer, p: &Placed, v: View, a: &Assets, g: &Game) {
    let mut pen = Pen { buf, p, v, g };
    let wall = a.palette[25];
    let (l, t) = pen.cell(MID, MID);
    let (r, b) = pen.cell(WORLD_W - 1 - MID, WORLD_H - 1 - MID);

    // the gate the ship leaves by: posts on the top wall, a red bar while shut
    let post = |wx: i32| wx + a.gate_left.w / 2;
    let (gl, _) = pen.cell(post(g.gate_left_x), MID);
    let (gr, _) = pen.cell(post(g.gate_right_x) - 1, MID);
    let gr = gr.max(gl + 1);
    pen.row(l + 1, gl - 1, t, '═', wall);
    pen.row(gr + 1, r - 1, t, '═', wall);
    let gate = a.palette[0x25];
    pen.put(gl, t, '╡', gate);
    pen.put(gr, t, '╞', gate);
    if !g.gate_open {
        pen.row(gl + 1, gr - 1, t, '━', a.palette[54]);
    }

    pen.row(l + 1, r - 1, b, '═', wall);
    pen.col(l, t + 1, b - 1, '║', wall);
    pen.col(r, t + 1, b - 1, '║', wall);
    pen.put(l, t, '╔', wall);
    pen.put(r, t, '╗', wall);
    pen.put(l, b, '╚', wall);
    pen.put(r, b, '╝', wall);

    // the enemy gates, at least three rows tall and two cells deep, so the
    // box and its inside always show
    let (_, top) = pen.cell(MID, WORLD_H / 2 - 7);
    let (_, bot) = pen.cell(MID, WORLD_H / 2 + 6);
    let bot = bot.max(top + 2);
    let (e, _) = pen.cell(15, 0);
    let (l0, _) = pen.cell(MID, 0);
    let out = (e - l0).max(2);
    pen.enemy_gate(a, l, out, top, bot, g.enemy_entering_left);
    pen.enemy_gate(a, r, -out, top, bot, g.enemy_entering_right);
}
