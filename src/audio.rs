//! The original sound effects from xquest.snd (25 samples, 8-bit unsigned,
//! 11025 Hz mono), mixed on 8 voices as xquest-sdl's audio.c does it: a
//! new sound takes a free voice, or else the one furthest through its
//! sample. Without an audio device, or with `--mute`, every call does
//! nothing.

use rodio::{MixerDeviceSink, Source};
use std::num::NonZero;
use std::sync::{Arc, Mutex};
use std::time::Duration;

const SND: &[u8] = include_bytes!("../data/xquest.snd");
const RATE: u32 = 11_025;
const SOUNDS: usize = 25;
const VOICES: usize = 8;
/// The mix's master gain, over 128. xquest-sdl uses 350; AQuest plays 22%
/// quieter (David, 2026-10-20: every effect a bit loud), which also leaves
/// more headroom before loud moments clip.
const GAIN: i32 = 273;

struct Sample {
    data: Vec<u8>,
    vol: i32,
}

fn samples() -> Vec<Sample> {
    let mut out = Vec::new();
    let mut at = 0;
    for i in 1..=SOUNDS {
        let Some(len) = SND.get(at..at + 2).map(|b| u16::from_le_bytes([b[0], b[1]]) as usize) else { break };
        let Some(data) = SND.get(at + 2..at + 2 + len) else { break };
        at += 2 + len;
        // xquest-sdl turns the glass shatter down
        let vol = if i == crate::game::SND_GETCRYSTAL as usize { 45 } else { 128 };
        out.push(Sample { data: data.to_vec(), vol });
    }
    out
}

#[derive(Clone, Copy, Default)]
struct Voice {
    sound: usize,
    pos: usize,
    len: usize,
    vol: i32,
}

struct Mixer {
    bank: Arc<Vec<Sample>>,
    shared: Arc<Mutex<Vec<u8>>>,
    voices: [Voice; VOICES],
    poll: u32,
}

impl Mixer {
    fn start(&mut self, snd: u8) {
        let i = snd as usize;
        if i == 0 || i > self.bank.len() {
            return;
        }
        let mut best = 0;
        let mut best_pos = 0;
        for (c, v) in self.voices.iter().enumerate() {
            if v.pos >= v.len {
                best = c;
                break;
            }
            if v.pos > best_pos {
                best_pos = v.pos;
                best = c;
            }
        }
        let s = &self.bank[i - 1];
        self.voices[best] = Voice { sound: i - 1, pos: 0, len: s.data.len(), vol: s.vol };
    }
}

impl Iterator for Mixer {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        if self.poll == 0 {
            self.poll = 64;
            let started: Vec<u8> = self.shared.lock().map(|mut q| std::mem::take(&mut *q)).unwrap_or_default();
            for s in started {
                self.start(s);
            }
        }
        self.poll -= 1;
        let mut mix: i32 = 0;
        for v in &mut self.voices {
            if v.pos < v.len {
                mix += (self.bank[v.sound].data[v.pos] as i32 - 128) * v.vol;
                v.pos += 1;
            }
        }
        let mix = (mix * GAIN / 128).clamp(-32768, 32767);
        Some(mix as f32 / 32768.0)
    }
}

impl Source for Mixer {
    fn current_span_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> rodio::ChannelCount {
        NonZero::new(1).unwrap()
    }
    fn sample_rate(&self) -> rodio::SampleRate {
        NonZero::new(RATE).unwrap()
    }
    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

pub struct Audio {
    // the sink must outlive the mixer playing on it
    sink: Option<MixerDeviceSink>,
    queue: Arc<Mutex<Vec<u8>>>,
    pub on: bool,
}

impl Audio {
    pub fn new(off: bool, on: bool) -> Audio {
        let queue = Arc::new(Mutex::new(Vec::new()));
        let sink = if off {
            None
        } else {
            match rodio::DeviceSinkBuilder::open_default_sink() {
                Ok(mut s) => {
                    s.log_on_drop(false);
                    let m = Mixer { bank: Arc::new(samples()), shared: queue.clone(), voices: [Voice::default(); VOICES], poll: 0 };
                    s.mixer().add(m);
                    Some(s)
                }
                Err(e) => {
                    log::warn!("no audio: {e}");
                    None
                }
            }
        };
        Audio { sink, queue, on }
    }

    pub fn available(&self) -> bool {
        self.sink.is_some()
    }

    /// Sound `snd`, 1 to 25 as numbered in xqvars.pas.
    pub fn play(&mut self, snd: u8) {
        if !self.on || self.sink.is_none() {
            return;
        }
        if let Ok(mut q) = self.queue.lock() {
            q.push(snd);
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn all_25_sounds_load() {
        let s = super::samples();
        assert_eq!(s.len(), 25);
        assert!(s.iter().all(|x| !x.data.is_empty()));
    }
}
