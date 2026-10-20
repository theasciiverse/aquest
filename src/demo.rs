//! Demos in the original xquest.dmo format, as xquest-sdl's demo.c reads
//! and writes them: a u32 random seed, a game mode byte, two 37-byte player
//! records, then 5 bytes a tick (i16 delx, i16 dely, u8 buttons). Each tick
//! stores the ship's velocity after braking and before the speed clamp, so
//! a recording does not depend on the input device.
//!
//! Demos recorded by xquest-sdl play here and the other way round, since
//! both use the same random number generator. The 1994 game's own demo does
//! not: Turbo Pascal's generator was different.

use crate::assets::Assets;
use crate::game::{BUT_BOMB, BUT_FIRE, BUT_FIRE_HELD, Controls, Game, Outcome};
use std::path::Path;

const HEADER: usize = 79;
const PLAYER_INFO: usize = 74;

/// The demo shipped inside AQuest: a run through six levels recorded in
/// xquest-sdl (its tests/data/reference.dmo). It plays on the title screen.
pub const BUILT_IN: &[u8] = include_bytes!("../data/reference.dmo");

#[derive(Clone)]
pub struct Demo {
    pub seed: u32,
    pub game_mode: u8,
    pub player_info: [u8; PLAYER_INFO],
    /// (delx, dely, buttons) per tick.
    pub frames: Vec<(i16, i16, u8)>,
}

impl Demo {
    /// A new recording at difficulty `diff`. The player records are the
    /// original's defaults (xquest-sdl's config_defaults) with the
    /// difficulty filled in, so the file reads back in xquest-sdl.
    pub fn start(seed: u32, diff: usize) -> Demo {
        let mut player_info = [0u8; PLAYER_INFO];
        // input device, h and v sensitivity, joystick and mouse buttons,
        // difficulty, then eleven DOS key scancodes
        const KEYS: [[u16; 11]; 2] = [[200, 208, 203, 205, 71, 73, 79, 81, 76, 57, 28], [72, 80, 75, 77, 71, 73, 79, 81, 76, 28, 57]];
        for (p, keys) in KEYS.iter().enumerate() {
            let q = &mut player_info[p * 37..(p + 1) * 37];
            q[0] = 0;
            // xquest-sdl only sets player 1's difficulty; player 2 stays Average
            let d = if p == 0 { diff as u16 } else { 2 };
            let ints: [u16; 7] = [64, 64, 16, 32, 1, 2, d];
            for (i, v) in ints.iter().chain(keys.iter()).enumerate() {
                q[1 + i * 2..3 + i * 2].copy_from_slice(&v.to_le_bytes());
            }
        }
        Demo { seed, game_mode: 0, player_info, frames: Vec::new() }
    }

    pub fn parse(b: &[u8]) -> Option<Demo> {
        if b.len() < HEADER {
            return None;
        }
        let seed = u32::from_le_bytes(b[0..4].try_into().ok()?);
        let mut player_info = [0u8; PLAYER_INFO];
        player_info.copy_from_slice(&b[5..HEADER]);
        let frames: Vec<_> = b[HEADER..]
            .chunks_exact(5)
            .map(|f| (i16::from_le_bytes([f[0], f[1]]), i16::from_le_bytes([f[2], f[3]]), f[4]))
            .collect();
        if frames.is_empty() {
            return None;
        }
        Some(Demo { seed, game_mode: b[4], player_info, frames })
    }

    pub fn load(path: &Path) -> Option<Demo> {
        Demo::parse(&std::fs::read(path).ok()?)
    }

    pub fn bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(HEADER + self.frames.len() * 5);
        out.extend_from_slice(&self.seed.to_le_bytes());
        out.push(self.game_mode);
        out.extend_from_slice(&self.player_info);
        for &(x, y, b) in &self.frames {
            out.extend_from_slice(&x.to_le_bytes());
            out.extend_from_slice(&y.to_le_bytes());
            out.push(b);
        }
        out
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        std::fs::write(path, self.bytes())
    }

    /// Clamped, not wrapped: Pascal integers are 16 bits.
    pub fn push(&mut self, delx: i32, dely: i32, but: u8) {
        self.frames.push((delx.clamp(-32768, 32767) as i16, dely.clamp(-32768, 32767) as i16, but));
    }

    /// The difficulty it was recorded at (player 1), 0 to 4, or None.
    pub fn difficulty(&self) -> Option<usize> {
        let d = i16::from_le_bytes([self.player_info[13], self.player_info[14]]);
        (0..5).contains(&d).then_some(d as usize)
    }
}

/// What a run of `d` comes to: (frames used, levels, crystals, score,
/// deaths, finished in game over).
pub fn replay(a: &Assets, d: &Demo) -> (usize, i32, i32, i64, i32, bool) {
    let diff = d.difficulty().unwrap_or(2);
    let mut g = Game::new(a, d.seed, diff);
    let (mut ptr, mut levels, mut crystals, mut deaths) = (0, 0, 0, 0);
    while ptr < d.frames.len() {
        let (dx, dy, but) = d.frames[ptr];
        let c = Controls { fire: but & BUT_FIRE != 0, held: but & BUT_FIRE_HELD != 0, bomb: but & BUT_BOMB != 0, demo: Some((dx as i32, dy as i32)), ..Controls::default() };
        let before = g.num_crystals;
        let (out, _, took) = g.step(a, c);
        if took {
            ptr += 1;
        }
        crystals += (before - g.num_crystals).max(0);
        match out {
            Outcome::LevelDone => {
                levels += 1;
                if g.finish_level(a) == Outcome::Won {
                    break;
                }
            }
            Outcome::LifeLost => deaths += 1,
            Outcome::GameOver => return (ptr, levels, crystals, g.score, deaths + 1, true),
            Outcome::Won | Outcome::Playing => {}
        }
        g.sounds.clear();
    }
    (ptr, levels, crystals, g.score, deaths, false)
}
