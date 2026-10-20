//! The original game data, compiled into the binary: xquest.gfx (every
//! sprite), xquest.fnt (status bar font), xquest2.fnt (the comix menu font)
//! and startpic.raw (the title banner, converted from startpic.pbm by
//! xquest-sdl's tools/convert_pbm.py). A port of xquest-sdl's assets.c.

pub const SHIP_FRAMES: usize = 24;
pub const ENEMY_KINDS: usize = 19;
pub const MISSILE_KINDS: usize = 6;
pub const POWERUP_KINDS: usize = 7;
pub const GATE_FRAMES: usize = 6;

const GFX: &[u8] = include_bytes!("../data/xquest.gfx");
const FNT: &[u8] = include_bytes!("../data/xquest.fnt");
const FNT2: &[u8] = include_bytes!("../data/xquest2.fnt");
const STARTPIC: &[u8] = include_bytes!("../data/startpic.raw");

/// Animation frames per enemy kind (numframes+1 in xqenter.pas).
pub const ENEMY_FRAME_COUNTS: [usize; ENEMY_KINDS] = [6, 6, 4, 4, 4, 4, 1, 4, 6, 4, 4, 4, 4, 4, 4, 4, 4, 4, 6];

#[derive(Clone, Default)]
pub struct Sprite {
    pub w: i32,
    pub h: i32,
    /// Palette indices, row-major; 0 is transparent.
    pub px: Vec<u8>,
    /// One row per entry, bit 31 the leftmost column, for pixel-accurate
    /// collision as in the original.
    pub mask: Vec<u32>,
}

impl Sprite {
    pub fn at(&self, x: i32, y: i32) -> u8 {
        self.px[(y * self.w + x) as usize]
    }
}

pub struct Assets {
    pub palette: [[u8; 3]; 256],
    pub ship: Vec<Sprite>,
    pub player_missile: Sprite,
    pub crystal: Sprite,
    pub mine: Sprite,
    pub smart_bomb: Sprite,
    pub enemy_mine: Sprite,
    pub enemy: Vec<Vec<Sprite>>,
    pub enemy_missile: Vec<Sprite>,
    pub ship_icon: Sprite,
    pub smart_bomb_hud: Sprite,
    pub crystal_hud: Sprite,
    pub powerup_hud: Vec<Sprite>,
    pub gate_left: Sprite,
    pub gate_right: Sprite,
    /// tl, tr, br, bl
    pub corner: Vec<Sprite>,
    pub enemy_gate_left: Vec<Sprite>,
    pub enemy_gate_right: Vec<Sprite>,
    pub font: Vec<Sprite>,
    pub startpic: Sprite,
    /// xquest2.fnt by ASCII code; glyphs are 0 clear, 1 stroke, 2 shadow.
    /// Loaded but not drawn yet: too fine to read in a shrunk picture.
    #[allow(dead_code)]
    pub comix: Vec<Option<Sprite>>,
}

/// ASCII to glyph number in `font` (1-based, 0 unmapped), from xquest.pas.
pub fn font_glyph(c: u8) -> usize {
    match c {
        b' ' => 39,
        b'+' => 37,
        b'0'..=b'9' => (c - b'0') as usize + 1,
        b':' => 38,
        b'?' => 40,
        b'A'..=b'Z' => (c - b'A') as usize + 11,
        _ => 0,
    }
}

/// The VGA palette, 6-bit values, from xquest-sdl's assets.c.
#[rustfmt::skip]
const PAL_VGA: [[u8; 3]; 256] = [
    [0,0,0],[2,2,2],[4,4,4],[6,6,6],[8,8,8],[10,10,10],[12,12,12],[14,14,14],
    [16,16,16],[18,18,18],[20,20,20],[22,22,22],[24,24,24],[26,26,26],[28,28,28],[30,30,30],
    [32,32,32],[34,34,34],[36,36,36],[38,38,38],[40,40,40],[42,42,42],[44,44,44],[46,46,46],
    [48,48,48],[50,50,50],[52,52,52],[54,54,54],[56,56,56],[58,58,58],[60,60,60],[63,63,63],
    [0,0,6],[0,0,14],[0,0,22],[0,0,30],[0,0,38],[0,0,46],[0,0,54],[0,0,63],
    [6,6,63],[14,14,63],[22,22,63],[30,30,63],[38,38,63],[46,46,63],[54,54,63],[6,0,0],
    [14,0,0],[22,0,0],[30,0,0],[38,0,0],[46,0,0],[54,0,0],[63,0,0],[63,6,6],
    [63,14,14],[63,22,22],[63,30,30],[63,38,38],[63,46,46],[63,54,54],[0,6,0],[0,14,0],
    [0,22,0],[0,30,0],[0,38,0],[0,46,0],[0,54,0],[0,63,0],[6,63,6],[14,63,14],
    [22,63,22],[30,63,30],[38,63,38],[46,63,46],[54,63,54],[6,0,6],[14,0,14],[22,0,22],
    [30,0,30],[38,0,38],[46,0,46],[54,0,54],[63,0,63],[63,6,63],[63,14,63],[63,22,63],
    [63,30,63],[63,38,63],[63,46,63],[63,54,63],[6,6,0],[14,14,0],[22,22,0],[30,30,0],
    [38,38,0],[46,46,0],[54,54,0],[63,63,0],[63,63,6],[63,63,14],[63,63,22],[63,63,30],
    [63,63,38],[63,63,46],[63,63,54],[0,6,6],[0,14,14],[0,22,22],[0,30,30],[0,38,38],
    [0,46,46],[0,54,54],[0,63,63],[6,63,63],[14,63,63],[22,63,63],[30,63,63],[38,63,63],
    [46,63,63],[54,63,63],[6,6,2],[14,14,6],[22,22,10],[30,30,14],[38,38,18],[46,46,22],
    [54,54,26],[63,63,30],[63,63,34],[63,63,38],[63,63,42],[63,63,46],[63,63,50],[63,63,54],
    [63,63,58],[2,6,6],[6,14,14],[10,22,22],[14,30,30],[18,38,38],[22,46,46],[26,54,54],
    [30,63,63],[34,63,63],[38,63,63],[42,63,63],[46,63,63],[50,63,63],[54,63,63],[58,63,63],
    [6,2,6],[14,6,14],[22,10,22],[30,14,30],[38,18,38],[46,22,46],[54,26,54],[63,30,63],
    [63,34,63],[63,38,63],[63,42,63],[63,46,63],[63,50,63],[63,54,63],[63,58,63],[57,6,0],
    [51,12,0],[45,18,0],[38,25,0],[32,31,0],[26,37,0],[19,44,0],[13,50,0],[7,56,0],
    [0,63,0],[0,57,6],[0,51,12],[0,45,18],[0,39,24],[0,32,31],[0,26,37],[0,20,43],
    [0,14,49],[0,7,56],[0,0,63],[45,45,45],[23,23,23],[45,0,0],[23,0,0],[0,0,0],
    [0,0,0],[35,1,1],[46,22,22],[10,0,0],[34,15,15],[63,2,2],[21,0,0],[63,14,14],
    [63,30,30],[63,10,10],[57,2,2],[21,7,7],[50,3,3],[55,26,26],[58,15,15],[4,0,0],
    [35,11,11],[63,18,18],[25,0,0],[63,6,6],[14,0,0],[63,22,22],[53,16,16],[35,7,7],
    [22,10,10],[63,32,32],[48,15,15],[57,10,10],[14,4,4],[50,9,9],[43,17,17],[27,4,4],
    [43,11,11],[43,1,1],[32,6,6],[63,4,4],[63,28,28],[47,7,7],[63,20,20],[63,12,12],
    [28,6,6],[17,0,0],[63,35,35],[63,16,16],[29,0,0],[42,6,6],[63,24,24],[8,0,0],
    [57,23,23],[29,12,12],[20,3,3],[63,8,8],[40,15,15],[58,18,18],[29,2,2],[12,0,0],
    [63,26,26],[58,4,4],[32,10,10],[43,20,20],[6,0,0],[30,10,10],[47,19,19],[0,0,0],
];

pub fn vga8(v: u8) -> u8 {
    ((v as u32 * 255 + 31) / 63) as u8
}

struct Reader<'a> {
    b: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn u8(&mut self) -> Option<u8> {
        let v = *self.b.get(self.at)?;
        self.at += 1;
        Some(v)
    }
    fn u16(&mut self) -> Option<u16> {
        Some(self.u8()? as u16 | (self.u8()? as u16) << 8)
    }
    fn bytes(&mut self, n: usize) -> Option<&[u8]> {
        let s = self.b.get(self.at..self.at + n)?;
        self.at += n;
        Some(s)
    }
    fn done(&self) -> bool {
        self.at >= self.b.len()
    }
}

fn with_mask(w: i32, h: i32, px: Vec<u8>) -> Sprite {
    let mask = (0..h)
        .map(|r| {
            let mut row = 0u32;
            for c in 0..w.min(32) {
                if px[(r * w + c) as usize] != 0 {
                    row |= 1 << (31 - c);
                }
            }
            row
        })
        .collect();
    Sprite { w, h, px, mask }
}

/// One sprite: u16 w, u16 h, then rows padded to a multiple of 4 bytes.
fn sprite(r: &mut Reader) -> Option<Sprite> {
    let w = r.u16()? as usize;
    let h = r.u16()? as usize;
    let stride = (w + 3) & !3;
    let mut px = Vec::with_capacity(w * h);
    for _ in 0..h {
        let row = r.bytes(stride)?;
        px.extend_from_slice(&row[..w]);
    }
    Some(with_mask(w as i32, h as i32, px))
}

fn sprites(r: &mut Reader, n: usize) -> Option<Vec<Sprite>> {
    (0..n).map(|_| sprite(r)).collect()
}

impl Assets {
    pub fn load() -> Assets {
        Self::parse().expect("the compiled-in game data is damaged")
    }

    fn parse() -> Option<Assets> {
        let mut palette = [[0u8; 3]; 256];
        for (p, v) in palette.iter_mut().zip(PAL_VGA.iter()) {
            *p = [vga8(v[0]), vga8(v[1]), vga8(v[2])];
        }

        let r = &mut Reader { b: GFX, at: 0 };
        let ship = sprites(r, SHIP_FRAMES)?;
        let player_missile = sprite(r)?;
        let crystal = sprite(r)?;
        let mine = sprite(r)?;
        let smart_bomb = sprite(r)?;
        let enemy_mine = sprite(r)?;
        let enemy = ENEMY_FRAME_COUNTS.iter().map(|&n| sprites(r, n)).collect::<Option<Vec<_>>>()?;
        let enemy_missile = sprites(r, MISSILE_KINDS)?;
        let ship_icon = sprite(r)?;
        let smart_bomb_hud = sprite(r)?;
        let crystal_hud = sprite(r)?;
        let powerup_hud = sprites(r, POWERUP_KINDS)?;
        let gate_left = sprite(r)?;
        let gate_right = sprite(r)?;
        let corner = sprites(r, 4)?;
        let enemy_gate_left = sprites(r, GATE_FRAMES)?;
        let enemy_gate_right = sprites(r, GATE_FRAMES)?;
        let _attractor = sprite(r)?;
        let _digits = sprites(r, 10)?;

        let font = sprites(&mut Reader { b: FNT, at: 0 }, 40)?;

        // [code u8][w u16][h u16][w*h bytes], unpadded
        let mut comix: Vec<Option<Sprite>> = vec![None; 128];
        let r = &mut Reader { b: FNT2, at: 0 };
        while !r.done() {
            let code = r.u8()? as usize;
            let w = r.u16()? as i32;
            let h = r.u16()? as i32;
            let px = r.bytes((w * h) as usize)?.to_vec();
            if code < 128 {
                comix[code] = Some(Sprite { w, h, px, mask: Vec::new() });
            }
        }

        let r = &mut Reader { b: STARTPIC, at: 0 };
        let (w, h) = (r.u16()? as i32, r.u16()? as i32);
        let startpic = Sprite { w, h, px: r.bytes((w * h) as usize)?.to_vec(), mask: Vec::new() };

        Some(Assets {
            palette,
            ship,
            player_missile,
            crystal,
            mine,
            smart_bomb,
            enemy_mine,
            enemy,
            enemy_missile,
            ship_icon,
            smart_bomb_hud,
            crystal_hud,
            powerup_hud,
            gate_left,
            gate_right,
            corner,
            enemy_gate_left,
            enemy_gate_right,
            font,
            startpic,
            comix,
        })
    }
}
