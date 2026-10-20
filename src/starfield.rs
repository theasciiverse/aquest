//! The 3-D starfield behind the menus and the game, as xquest-sdl's
//! starfield.c ports starunit.pas: 400 stars flying at the viewer, closer
//! ones brighter on the grey ramp of the palette.

use crate::game::Rng;

const STARS: usize = 400;
const SPEED: i32 = 128;

#[derive(Clone, Copy, Default)]
pub struct Star {
    x: i32,
    y: i32,
    z: i32,
    /// Where it was last drawn, on the 320x240 screen.
    pub xz: i32,
    pub yz: i32,
    /// Palette index, 0 until placed.
    pub c: u8,
}

pub struct Starfield {
    pub stars: Vec<Star>,
    rng: Rng,
}

impl Starfield {
    pub fn new(seed: u32) -> Starfield {
        let mut rng = Rng(if seed != 0 { seed } else { 0x9E37_79B9 });
        let stars = (0..STARS)
            .map(|_| Star { x: -5000 + rng.range(10000), y: -5000 + rng.range(10000), z: rng.range(12000) + 256, xz: 1, yz: 1, c: 0 })
            .collect();
        Starfield { stars, rng }
    }

    pub fn step(&mut self) {
        for st in &mut self.stars {
            if st.xz < 0 || st.xz >= 320 || st.yz < 0 || st.yz >= 240 || {
                st.z -= SPEED;
                st.z < 257
            } {
                let r = &mut self.rng;
                *st = Star { x: -8191 + r.range(16383), y: -8191 + r.range(16383), z: r.range(1256) + 14500, xz: 1, yz: 1, c: 0 };
                continue;
            }
            let hi_z = st.z >> 8;
            st.xz = st.x / hi_z + 160;
            st.yz = st.y / hi_z + 120;
            st.c = (31 - (st.z >> 9)).max(1) as u8;
        }
    }
}
