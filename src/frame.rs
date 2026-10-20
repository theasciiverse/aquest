//! The original 320x240 screen, drawn as xquest-sdl's render.c draws it:
//! a 320x217 window on the world above a 23-line status bar. `view.rs`
//! then fits this picture to the terminal.

use crate::assets::{Assets, Sprite, font_glyph};
use crate::game::{FRAME_RATE, Game, ObjType, PU_COUNT, VIEWPORT_H, WORLD_H, WORLD_W};
use crate::starfield::Starfield;

pub const W: i32 = 320;
pub const H: i32 = 240;
pub const HUD_Y: i32 = 217;

pub type Rgb = [u8; 3];

pub struct Frame {
    pub px: Vec<Rgb>,
    pub pal: [Rgb; 256],
    /// How many of the 400 stars to draw. A shrunk picture shows fewer,
    /// or they would crowd it: the terminal has far fewer pixels.
    pub star_limit: usize,
}

/// SmartBombPal from xqvars.pas, 8-bit, for flash levels 1 to 11.
const SBPAL: [Rgb; 12] = [
    [0, 0, 0],
    [0, 0, 0],
    [32, 0, 0],
    [56, 0, 0],
    [80, 16, 0],
    [104, 48, 40],
    [128, 80, 80],
    [152, 112, 120],
    [176, 144, 160],
    [200, 176, 200],
    [224, 208, 200],
    [252, 252, 252],
];

impl Frame {
    pub fn new(a: &Assets) -> Frame {
        Frame { px: vec![[0; 3]; (W * H) as usize], pal: a.palette, star_limit: 400 }
    }

    fn fill(&mut self, y0: i32, y1: i32, c: Rgb) {
        self.px[(y0 * W) as usize..(y1 * W) as usize].fill(c);
    }

    pub fn clear(&mut self) {
        self.px.fill([0; 3]);
    }

    /// A sprite at screen (x, y), clipped to rows y_min..y_max.
    fn blit(&mut self, s: &Sprite, x: i32, y: i32, y_min: i32, y_max: i32) {
        for row in 0..s.h {
            let dy = y + row;
            if dy < y_min || dy >= y_max {
                continue;
            }
            for col in 0..s.w {
                let dx = x + col;
                if !(0..W).contains(&dx) {
                    continue;
                }
                let p = s.at(col, row);
                if p != 0 {
                    self.px[(dy * W + dx) as usize] = self.pal[p as usize];
                }
            }
        }
    }

    pub fn sprite(&mut self, s: &Sprite, x: i32, y: i32) {
        self.blit(s, x, y, 0, H);
    }

    /// A sprite in grey at `k` of its brightness, for an unlit icon.
    pub fn sprite_dim(&mut self, s: &Sprite, x: i32, y: i32, k: f32) {
        for row in 0..s.h {
            for col in 0..s.w {
                let (dx, dy) = (x + col, y + row);
                let p = s.at(col, row);
                if p != 0 && (0..W).contains(&dx) && (0..H).contains(&dy) {
                    let c = self.pal[p as usize];
                    let grey = ((c[0] as f32 * 0.3 + c[1] as f32 * 0.59 + c[2] as f32 * 0.11) * k) as u8;
                    self.px[(dy * W + dx) as usize] = [grey; 3];
                }
            }
        }
    }

    /// Twice as tall, as the title banner is drawn.
    pub fn sprite_2xv(&mut self, s: &Sprite, x: i32, y: i32) {
        for row in 0..s.h {
            for r2 in 0..2 {
                let dy = y + row * 2 + r2;
                if !(0..H).contains(&dy) {
                    continue;
                }
                for col in 0..s.w {
                    let dx = x + col;
                    let p = s.at(col, row);
                    if p != 0 && (0..W).contains(&dx) {
                        self.px[(dy * W + dx) as usize] = self.pal[p as usize];
                    }
                }
            }
        }
    }

    fn world(&mut self, s: &Sprite, wx: i32, wy: i32, g: &Game) {
        self.blit(s, wx - g.cam_x, wy - g.cam_y, 0, VIEWPORT_H);
    }

    /// Stars, on rows below `max_y`.
    pub fn stars(&mut self, sf: &Starfield, max_y: i32) {
        for st in sf.stars.iter().take(self.star_limit) {
            if st.c == 0 || !(0..W).contains(&st.xz) || !(0..max_y).contains(&st.yz) {
                continue;
            }
            self.px[(st.yz * W + st.xz) as usize] = self.pal[st.c as usize];
        }
    }

    fn hline(&mut self, x1: i32, x2: i32, wy: i32, g: &Game, c: Rgb) {
        let sy = wy - g.cam_y;
        if !(0..VIEWPORT_H).contains(&sy) {
            return;
        }
        let (a, b) = ((x1 - g.cam_x).max(0), (x2 - g.cam_x).min(W - 1));
        for x in a..=b {
            self.px[(sy * W + x) as usize] = c;
        }
    }

    fn vline(&mut self, y1: i32, y2: i32, wx: i32, g: &Game, c: Rgb) {
        let sx = wx - g.cam_x;
        if !(0..W).contains(&sx) {
            return;
        }
        let (a, b) = ((y1 - g.cam_y).max(0), (y2 - g.cam_y).min(VIEWPORT_H - 1));
        for y in a..=b {
            self.px[(y * W + sx) as usize] = c;
        }
    }

    /// The whole game screen. The countdown is drawn only `full` size: shrunk,
    /// its five-pixel digits are a smudge, and the terminal shows it as text.
    pub fn game(&mut self, a: &Assets, g: &Game, sf: &Starfield, full: bool) {
        self.fill(0, HUD_Y, self.pal[0]);
        self.fill(HUD_Y, H, self.pal[1]);
        self.stars(sf, VIEWPORT_H);

        // the border, a five-line pipe
        const BCOLOR: [usize; 5] = [10, 15, 25, 15, 10];
        for (i, &bc) in BCOLOR.iter().enumerate() {
            let i = i as i32 + 1;
            let c = self.pal[bc];
            self.hline(10, g.gate_left_x, 10 - i, g, c);
            self.hline(g.gate_right_x + a.gate_right.w, WORLD_W - 10, 10 - i, g, c);
            self.hline(10, WORLD_W - 10, WORLD_H - 5 - i, g, c);
            self.vline(10, WORLD_H - 9, 4 + i, g, c);
            self.vline(10, WORLD_H - 9, WORLD_W - 11 + i, g, c);
        }
        self.world(&a.gate_left, g.gate_left_x, 0, g);
        self.world(&a.gate_right, g.gate_right_x, 0, g);
        if !g.gate_open {
            let c = self.pal[54];
            self.hline(g.gate_left_x + a.gate_left.w, g.gate_right_x - 1, 7, g, c);
        }
        self.world(&a.corner[0], 0, 0, g);
        self.world(&a.corner[1], WORLD_W - 10, 0, g);
        self.world(&a.corner[2], WORLD_W - 10, WORLD_H - 10, g);
        self.world(&a.corner[3], 0, WORLD_H - 10, g);

        let lf = if g.enemy_entering_left > 0 { ((g.enemy_entering_left >> 3) % 5 + 1) as usize } else { 0 };
        let rf = if g.enemy_entering_right > 0 { ((g.enemy_entering_right >> 3) % 5 + 1) as usize } else { 0 };
        self.world(&a.enemy_gate_left[lf], 0, WORLD_H / 2 - 10, g);
        self.world(&a.enemy_gate_right[rf], WORLD_W - 20, WORLD_H / 2 - 10, g);

        for o in g.objects.iter().filter(|o| o.active) {
            let s = match o.kind {
                ObjType::Crystal => &a.crystal,
                ObjType::Mine => &a.mine,
                ObjType::Smart => &a.smart_bomb,
                ObjType::EnemyMine => &a.enemy_mine,
            };
            self.world(s, o.x, o.y, g);
        }
        for m in g.missiles.iter().filter(|m| m.active) {
            self.world(&a.player_missile, m.x, m.y, g);
        }
        for m in g.emissiles.iter().filter(|m| m.active) {
            self.world(&a.enemy_missile[m.firetype as usize - 1], m.x, m.y, g);
        }
        for e in g.enemies.iter().filter(|e| e.active) {
            self.world(&a.enemy[e.kind][(e.frame >> 8) as usize], e.x, e.y, g);
        }
        if let Some(t) = countdown(g).filter(|_| full) {
            // TimerX=280, kept clear of the right-hand border; the colons
            // are gaps, only digits are drawn
            let wx = (g.cam_x + 280).min(WORLD_W - 46);
            for (i, c) in format!("{t:>7}").bytes().enumerate() {
                if c.is_ascii_digit() {
                    self.world(&a.digit[(c - b'0') as usize], wx + i as i32 * 5, g.cam_y + 10, g);
                }
            }
        }
        if g.ship_explode_timer == 0 {
            self.world(&a.ship[g.ship.dir], g.ship.x, g.ship.y, g);
        }

        if (1..=11).contains(&g.smart_bomb_flash) {
            let f = SBPAL[g.smart_bomb_flash as usize];
            for p in &mut self.px[..(HUD_Y * W) as usize] {
                for k in 0..3 {
                    p[k] = p[k].saturating_add(f[k]);
                }
            }
        }
        self.hud(a, g);
    }

    /// The status bar in the original's font.
    fn hud(&mut self, a: &Assets, g: &Game) {
        self.text(a, 10, HUD_Y + 5, &format!("{:8}", g.score));
        self.text(a, 185, HUD_Y + 5, &format!("  {}", g.lives - 1));
        self.sprite(&a.ship_icon, 185, HUD_Y + 6);
        self.text(a, 230, HUD_Y + 5, &format!("  {}", g.bombs));
        self.sprite(&a.smart_bomb_hud, 230, HUD_Y + 6);
        self.text(a, 275, HUD_Y + 5, &format!("  {}", g.level_rec().numcryst - g.num_crystals));
        self.sprite(&a.crystal_hud, 275, HUD_Y + 6);
        for pu in 0..PU_COUNT {
            if powerup_shown(g.powerup_timer[pu]) {
                self.sprite(&a.powerup_hud[pu], 80 + pu as i32 * 16, HUD_Y + 4);
            }
        }
    }

    fn rect(&mut self, x1: i32, y1: i32, x2: i32, y2: i32, c: usize) {
        for y in y1.max(0)..=y2.min(H - 1) {
            for x in x1.max(0)..=x2.min(W - 1) {
                self.px[(y * W + x) as usize] = self.pal[c];
            }
        }
    }

    /// XText with its box (DrawBox).
    fn boxed(&mut self, a: &Assets, x: i32, y: i32, s: &str) {
        let n = s.len() as i32;
        self.text(a, x, y, s);
        self.rect(x - 2, y - 1, x + n * 8 + 2, y - 1, 10);
        self.rect(x - 2, y, x - 2, y + 14, 10);
        self.rect(x - 2, y + 14, x + n * 8 + 2, y + 14, 23);
        self.rect(x + n * 8 + 2, y, x + n * 8 + 2, y + 14, 23);
    }

    /// GiveBonus's window: XWindow(50,40,270,160) with a bevel of 2.
    pub fn bonus_window(&mut self, a: &Assets, b: &Bonus) {
        let (x1, y1, x2, y2, bs) = (50, 40, 270, 160, 2);
        self.rect(x1 + bs, y1 + bs, x2 - bs, y2 - bs, 14);
        self.rect(x1 + bs, y1, x2 - bs, y1 + bs, 23);
        self.rect(x1 + bs, y2 - bs, x2 - bs, y2, 10);
        self.rect(x1, y1 + bs, x1 + bs, y2 - bs, 20);
        self.rect(x2 - bs, y1 + bs, x2, y2 - bs, 12);
        let lines = b.lines();
        self.boxed(a, x1 + 42, y1 + 15, &lines[0]);
        for (i, l) in lines[1..].iter().enumerate() {
            self.boxed(a, x1 + 20, y1 + 40 + 25 * i as i32, l);
        }
    }

    /// xquest.fnt text, 8 pixels a character; index 14 is its background.
    fn text(&mut self, a: &Assets, mut x: i32, y: i32, s: &str) {
        for c in s.bytes() {
            let gi = font_glyph(c);
            if gi > 0 {
                let f = &a.font[gi - 1];
                for row in 0..f.h {
                    for col in 0..f.w {
                        let p = f.at(col, row);
                        let (dx, dy) = (x + col, y + row);
                        if p != 14 && (0..W).contains(&dx) && (0..H).contains(&dy) {
                            self.px[(dy * W + dx) as usize] = self.pal[p as usize];
                        }
                    }
                }
            }
            x += 8;
        }
    }
}

/// What the level-end window shows at this moment.
pub struct Bonus {
    pub level: i32,
    pub taken: i32,
    pub par: i32,
    pub bonus: i64,
}

impl Bonus {
    /// The four lines of GiveBonus, the figures where it puts them.
    pub fn lines(&self) -> [String; 4] {
        [
            format!("LEVEL {} COMPLETED", self.level),
            format!("TIME TAKEN: {:3} SECONDS", self.taken),
            format!("PAR       : {:3} SECONDS", self.par),
            format!("BONUS     :  {:<7}   ", self.bonus),
        ]
    }
}

/// The countdown to par as ShowCountDown writes it, `[m:]ss:hh`, or None
/// once past par.
pub fn countdown(g: &Game) -> Option<String> {
    let t = g.level_rec().time as i64 * FRAME_RATE - g.time_on_level;
    if t < 0 {
        return None;
    }
    let bits = (t % FRAME_RATE) * 100 / FRAME_RATE;
    let secs = t / FRAME_RATE;
    let (mins, secs) = (secs / 60, secs % 60);
    Some(if mins > 0 {
        format!("{mins}:{secs:02}:{bits:02}")
    } else if secs > 0 {
        format!("{secs}:{bits:02}")
    } else {
        format!("{bits:02}")
    })
}

/// A power-up's icon blinks in its last three seconds.
pub fn powerup_shown(t: i32) -> bool {
    t > 0 && (t >= 198 || (t % 22) >= 11)
}

/// The colour a sprite is mostly drawn in, for text stand-ins.
pub fn main_colour(a: &Assets, s: &Sprite) -> Rgb {
    let mut count = [0u32; 256];
    for &p in &s.px {
        if p != 0 {
            count[p as usize] += 1;
        }
    }
    // the brightest of the commonest few, so outlines don't win
    let mut idx: Vec<usize> = (1..256).filter(|&i| count[i] > 0).collect();
    idx.sort_by_key(|&i| std::cmp::Reverse(count[i]));
    idx.truncate(3);
    let lum = |c: Rgb| c[0] as u32 * 2 + c[1] as u32 * 5 + c[2] as u32;
    idx.into_iter().map(|i| a.palette[i]).max_by_key(|&c| lum(c)).unwrap_or([200, 200, 200])
}
