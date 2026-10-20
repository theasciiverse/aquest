//! Files kept between runs, in `$XDG_DATA_HOME/aquest` (`%APPDATA%\aquest`
//! on Windows, `~/Library/Application Support/aquest` on macOS):
//!
//! - `xquest.scr`, the Hall of Fame in the original binary format (as
//!   xquest-sdl's hiscore.c reads and writes it), so a table can be copied
//!   between AQuest, xquest-sdl and the 1994 game;
//! - `xquest.dmo`, the last demo recorded, played on the title screen
//!   instead of the built-in one;
//! - `aquest.cfg`, AQuest's own settings as `key value` lines.

use std::path::PathBuf;

pub const NUM_DIFF: usize = 5;
pub const NUM_SCORES: usize = 10;
pub const NAME_MAX: usize = 20;
const ENTRY: usize = 27;

pub fn dir() -> Option<PathBuf> {
    let env = |k: &str| std::env::var_os(k).filter(|v| !v.is_empty()).map(PathBuf::from);
    if let Some(d) = env("AQUEST_DATA_DIR") {
        return Some(d);
    }
    let base = if cfg!(windows) {
        env("APPDATA")
    } else if cfg!(target_os = "macos") {
        env("HOME").map(|h| h.join("Library/Application Support"))
    } else {
        env("XDG_DATA_HOME").or_else(|| env("HOME").map(|h| h.join(".local/share")))
    };
    base.map(|b| b.join("aquest"))
}

fn file(name: &str) -> Option<PathBuf> {
    let d = dir()?;
    std::fs::create_dir_all(&d).ok()?;
    Some(d.join(name))
}

pub fn demo_path() -> Option<PathBuf> {
    file("xquest.dmo")
}

#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub score: i64,
    pub level: i32,
    pub name: String,
}

#[derive(Clone)]
pub struct HiTable {
    pub table: Vec<Vec<Entry>>,
}

impl HiTable {
    fn defaults() -> HiTable {
        HiTable { table: vec![vec![Entry { score: 0, level: 1, name: "XQuest".into() }; NUM_SCORES]; NUM_DIFF] }
    }

    pub fn parse(b: &[u8]) -> Option<HiTable> {
        if b.len() < NUM_DIFF * NUM_SCORES * ENTRY {
            return None;
        }
        let mut t = HiTable::defaults();
        for (d, row) in t.table.iter_mut().enumerate() {
            for (i, e) in row.iter_mut().enumerate() {
                let p = &b[(d * NUM_SCORES + i) * ENTRY..][..ENTRY];
                let len = (p[6] as usize).min(NAME_MAX);
                *e = Entry {
                    score: i32::from_le_bytes([p[0], p[1], p[2], p[3]]) as i64,
                    level: u16::from_le_bytes([p[4], p[5]]) as i32,
                    name: p[7..7 + len].iter().map(|&c| c as char).collect(),
                };
            }
        }
        Some(t)
    }

    pub fn bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        for row in &self.table {
            for e in row {
                let mut p = [0u8; ENTRY];
                p[0..4].copy_from_slice(&(e.score as i32).to_le_bytes());
                p[4..6].copy_from_slice(&(e.level as u16).to_le_bytes());
                let name: Vec<u8> = e.name.chars().filter(|c| c.is_ascii()).map(|c| c as u8).take(NAME_MAX).collect();
                p[6] = name.len() as u8;
                p[7..7 + name.len()].copy_from_slice(&name);
                out.extend_from_slice(&p);
            }
        }
        out
    }

    pub fn load() -> HiTable {
        file("xquest.scr").and_then(|p| std::fs::read(p).ok()).and_then(|b| HiTable::parse(&b)).unwrap_or_else(HiTable::defaults)
    }

    pub fn save(&self) {
        let Some(p) = file("xquest.scr") else { return };
        if let Err(e) = std::fs::write(&p, self.bytes()) {
            log::warn!("could not save the Hall of Fame to {}: {e}", p.display());
        }
    }

    /// The place `score` earns at `diff`, 0 the top, or None.
    pub fn rank(&self, diff: usize, score: i64) -> Option<usize> {
        if score <= 0 {
            return None;
        }
        self.table[diff].iter().position(|e| score > e.score)
    }

    pub fn insert(&mut self, diff: usize, rank: usize, score: i64, level: i32, name: &str) {
        let row = &mut self.table[diff];
        row.insert(rank, Entry { score, level, name: name.into() });
        row.truncate(NUM_SCORES);
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum View {
    Blocks,
    Braille,
    Mixed,
}

impl View {
    pub fn name(self) -> &'static str {
        match self {
            View::Blocks => "Blocks",
            View::Braille => "Braille",
            View::Mixed => "Mixed",
        }
    }

    /// The next one, for the V key and the menu.
    pub fn next(self) -> View {
        match self {
            View::Blocks => View::Braille,
            View::Braille => View::Mixed,
            View::Mixed => View::Blocks,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Settings {
    pub difficulty: usize,
    pub view: View,
    pub sound: bool,
}

impl Settings {
    pub fn load() -> Settings {
        let mut s = Settings { difficulty: 2, view: View::Mixed, sound: true };
        let Some(text) = file("aquest.cfg").and_then(|p| std::fs::read_to_string(p).ok()) else { return s };
        for line in text.lines() {
            let mut it = line.split_whitespace();
            match (it.next(), it.next()) {
                (Some("difficulty"), Some(v)) => s.difficulty = v.parse::<usize>().unwrap_or(2).min(4),
                (Some("view"), Some(v)) => {
                    s.view = match v {
                        "braille" => View::Braille,
                        "blocks" => View::Blocks,
                        _ => View::Mixed,
                    }
                }
                (Some("sound"), Some(v)) => s.sound = v != "off",
                _ => {}
            }
        }
        s
    }

    pub fn save(&self) {
        let Some(p) = file("aquest.cfg") else { return };
        let view = self.view.name().to_lowercase();
        let text = format!("difficulty {}\nview {view}\nsound {}\n", self.difficulty, if self.sound { "on" } else { "off" });
        if let Err(e) = std::fs::write(&p, text) {
            log::warn!("could not save settings to {}: {e}", p.display());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hall_of_fame_round_trips_in_the_original_format() {
        let mut t = HiTable::defaults();
        t.insert(2, 0, 36000, 7, "David");
        let b = t.bytes();
        assert_eq!(b.len(), 5 * 10 * 27);
        let u = HiTable::parse(&b).unwrap();
        assert_eq!(u.table[2][0], Entry { score: 36000, level: 7, name: "David".into() });
        assert_eq!(u.table[2][1].name, "XQuest");
        assert_eq!(u.rank(2, 100), Some(1));
        assert_eq!(u.rank(2, 0), None);
    }
}
