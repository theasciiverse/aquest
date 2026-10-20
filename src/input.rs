//! Keyboard, mouse and gamepad, turned into the original's controls.
//!
//! XQuest steers by thrust: every tick a held direction adds 10 to the
//! ship's speed that way, and the ship drifts on when let go. The mouse
//! thrusts by how far it moves, as it did in DOS; a gamepad stick by how
//! far it is pushed.
//!
//! - Thrust: arrows or WASD (or numpad 8 4 6 2), mouse movement, stick
//! - Fire: Space or Enter, left button, pad A. Held, it keeps firing
//!   with RapidFire
//! - Brake: Shift (with the kitty keyboard protocol), Z or numpad 5,
//!   middle button, pad X or a shoulder
//! - Smart bomb: B or X, right button, pad B
//!
//! With the kitty keyboard protocol presses and releases are exact.
//! Without it the terminal only sends key repeats, and a key counts as held
//! while they keep arriving: playable, but a held key stutters until the
//! repeats start, and thrust goes on a moment after it is let go.

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers, ModifierKeyCode};
use std::time::{Duration, Instant};

/// Without key releases, a press counts as held this long. XQuest adds
/// thrust every tick a key is held, so this is short: a tap is a nudge, and
/// a held key keeps going once the terminal's repeats arrive.
const PRE_REPEAT: Duration = Duration::from_millis(120);
const REPEAT_GAP: Duration = Duration::from_millis(100);

const LEFT: usize = 0;
const RIGHT: usize = 1;
const UP: usize = 2;
const DOWN: usize = 3;
const BRAKE: usize = 4;
const FIRE: usize = 5;
const KEYS: usize = 6;

/// Gamepad stick: dead zone and full-deflection thrust (KEYBOARD_STEP).
#[cfg(feature = "gamepad")]
const DEAD: f32 = 0.25;

/// Commands, as edges.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cmd {
    Up,
    Down,
    Left,
    Right,
    Confirm,
    Back,
    Pause,
    Mute,
    View,
    Yes,
    Exit,
    /// Any key at all, for ending demos and waits.
    Any,
    Char(char),
    Backspace,
}

#[derive(Clone, Copy, Default)]
struct Key {
    down: bool,
    latched: bool,
    last: Option<Instant>,
    repeats: u32,
}

#[cfg(feature = "gamepad")]
fn usable_pad(p: &gilrs::Gamepad) -> bool {
    p.mapping_source() == gilrs::MappingSource::SdlMappings || std::env::var("AQUEST_ANY_PAD").is_ok_and(|v| v == "1")
}

pub struct Input {
    pub exact: bool,
    keys: [Key; KEYS],
    pub cmds: Vec<Cmd>,
    fire_pressed: bool,
    bomb_pressed: bool,
    pub mouse_dx: i32,
    pub mouse_dy: i32,
    pub mouse_fire: bool,
    pub mouse_brake: bool,
    #[cfg(feature = "gamepad")]
    gilrs: Option<gilrs::Gilrs>,
    pub pad_name: Option<String>,
    joy: (f32, f32),
    pad_dir: [bool; 4],
    pad_fire: bool,
    pad_brake: bool,
}

impl Input {
    pub fn new(exact: bool) -> Input {
        Input {
            exact,
            keys: [Key::default(); KEYS],
            cmds: Vec::new(),
            fire_pressed: false,
            bomb_pressed: false,
            mouse_dx: 0,
            mouse_dy: 0,
            mouse_fire: false,
            mouse_brake: false,
            #[cfg(feature = "gamepad")]
            gilrs: None,
            pad_name: None,
            joy: (0.0, 0.0),
            pad_dir: [false; 4],
            pad_fire: false,
            pad_brake: false,
        }
    }

    pub fn on_key(&mut self, k: KeyEvent, now: Instant) {
        let press = k.kind != KeyEventKind::Release;
        let fresh = k.kind == KeyEventKind::Press;
        if fresh && k.modifiers.contains(KeyModifiers::CONTROL) && matches!(k.code, KeyCode::Char('c' | 'C')) {
            self.cmds.push(Cmd::Exit);
            return;
        }
        let which = match k.code {
            KeyCode::Left | KeyCode::Char('a' | 'A' | '4') => Some(LEFT),
            KeyCode::Right | KeyCode::Char('d' | 'D' | '6') => Some(RIGHT),
            KeyCode::Up | KeyCode::Char('w' | 'W' | '8') => Some(UP),
            KeyCode::Down | KeyCode::Char('s' | 'S' | '2') => Some(DOWN),
            KeyCode::Char('z' | 'Z' | '5') => Some(BRAKE),
            KeyCode::Modifier(ModifierKeyCode::LeftShift | ModifierKeyCode::RightShift) => Some(BRAKE),
            KeyCode::Char(' ') | KeyCode::Enter => Some(FIRE),
            _ => None,
        };
        if let Some(d) = which {
            let key = &mut self.keys[d];
            if self.exact {
                if press && !key.down {
                    key.latched = true;
                }
                key.down = press;
            } else {
                if held_by_repeat(key, now) {
                    key.repeats += 1;
                } else {
                    key.repeats = 0;
                    key.latched = true;
                }
                key.last = Some(now);
            }
            // a press or a repeat fires, as SDL's key repeat does in xquest-sdl
            if d == FIRE && press {
                self.fire_pressed = true;
            }
        }
        if !press {
            return;
        }
        match k.code {
            KeyCode::Up => self.cmds.push(Cmd::Up),
            KeyCode::Down => self.cmds.push(Cmd::Down),
            KeyCode::Left => self.cmds.push(Cmd::Left),
            KeyCode::Right => self.cmds.push(Cmd::Right),
            KeyCode::Backspace => self.cmds.push(Cmd::Backspace),
            _ => {}
        }
        if !fresh {
            return;
        }
        if !matches!(k.code, KeyCode::Modifier(_)) {
            self.cmds.push(Cmd::Any);
        }
        if let KeyCode::Char(c) = k.code {
            self.cmds.push(Cmd::Char(c));
        }
        match k.code {
            KeyCode::Enter => self.cmds.push(Cmd::Confirm),
            KeyCode::Esc => self.cmds.push(Cmd::Back),
            KeyCode::Char('p' | 'P') => self.cmds.push(Cmd::Pause),
            KeyCode::Char('m' | 'M') => self.cmds.push(Cmd::Mute),
            KeyCode::Char('v' | 'V') | KeyCode::F(2) => self.cmds.push(Cmd::View),
            KeyCode::Char('y' | 'Y') => self.cmds.push(Cmd::Yes),
            KeyCode::Char('b' | 'B' | 'x' | 'X') => self.bomb_pressed = true,
            _ => {}
        }
    }

    pub fn mouse_down(&mut self, button: u8) {
        match button {
            0 => {
                self.fire_pressed = true;
                self.mouse_fire = true;
            }
            1 => self.bomb_pressed = true,
            _ => self.mouse_brake = true,
        }
    }

    pub fn mouse_up(&mut self, button: u8) {
        match button {
            0 => self.mouse_fire = false,
            1 => {}
            _ => self.mouse_brake = false,
        }
    }

    /// The window lost focus: releases will not arrive, so let go.
    pub fn release_all(&mut self) {
        for k in &mut self.keys {
            k.down = false;
            k.last = None;
        }
        self.mouse_fire = false;
        self.mouse_brake = false;
    }

    /// Forget presses made before now (when a game starts).
    pub fn clear(&mut self) {
        self.release_all();
        for k in &mut self.keys {
            k.latched = false;
        }
        self.fire_pressed = false;
        self.bomb_pressed = false;
        self.mouse_dx = 0;
        self.mouse_dy = 0;
        self.cmds.clear();
    }

    pub fn open_pad(&mut self) {
        #[cfg(feature = "gamepad")]
        match gilrs::Gilrs::new() {
            Ok(g) => {
                let all: Vec<_> = g.gamepads().map(|(_, p)| (p.name().to_string(), usable_pad(&p))).collect();
                log::info!("gamepads (name, used): {all:?}");
                self.pad_name = all.into_iter().find(|(_, used)| *used).map(|(n, _)| n);
                self.gilrs = Some(g);
            }
            Err(e) => log::warn!("gamepad support unavailable: {e}"),
        }
    }

    /// Drain gamepad events.
    pub fn poll_pad(&mut self) {
        #[cfg(feature = "gamepad")]
        {
            use gilrs::{Axis, Button, EventType};
            let Some(g) = &mut self.gilrs else { return };
            let mut events = Vec::new();
            while let Some(ev) = g.next_event() {
                events.push(ev);
            }
            for ev in events {
                let g = self.gilrs.as_ref().unwrap();
                let pad = g.gamepad(ev.id);
                if !usable_pad(&pad) {
                    continue;
                }
                match ev.event {
                    EventType::Connected => {
                        self.pad_name = Some(pad.name().to_string());
                        log::info!("gamepad connected: {:?}", self.pad_name);
                    }
                    EventType::Disconnected => {
                        log::info!("gamepad disconnected");
                        self.pad_name = None;
                    }
                    EventType::ButtonPressed(b, _) => {
                        log::debug!("pad press {b:?}");
                        self.cmds.push(Cmd::Any);
                        match b {
                            Button::South => {
                                self.fire_pressed = true;
                                self.cmds.push(Cmd::Confirm);
                            }
                            Button::East => {
                                self.bomb_pressed = true;
                                self.cmds.push(Cmd::Back);
                            }
                            Button::Start => self.cmds.push(Cmd::Pause),
                            Button::DPadUp => self.cmds.push(Cmd::Up),
                            Button::DPadDown => self.cmds.push(Cmd::Down),
                            Button::DPadLeft => self.cmds.push(Cmd::Left),
                            Button::DPadRight => self.cmds.push(Cmd::Right),
                            _ => {}
                        }
                    }
                    EventType::ButtonReleased(b, _) => log::debug!("pad release {b:?}"),
                    _ => {}
                }
                let on = |b: Button| pad.is_pressed(b);
                let axis = |a: Axis| pad.axis_data(a).map_or(0.0, |d| d.value());
                let (sx, sy) = (axis(Axis::LeftStickX), axis(Axis::LeftStickY));
                let was = self.joy;
                self.joy = (sx, -sy);
                // the stick steps through menus too
                if sy > 0.5 && -was.1 <= 0.5 {
                    self.cmds.push(Cmd::Up);
                }
                if sy < -0.5 && -was.1 >= -0.5 {
                    self.cmds.push(Cmd::Down);
                }
                self.pad_dir = [on(Button::DPadLeft), on(Button::DPadRight), on(Button::DPadUp), on(Button::DPadDown)];
                self.pad_fire = on(Button::South);
                self.pad_brake = on(Button::West) || on(Button::LeftTrigger) || on(Button::RightTrigger);
            }
        }
    }

    /// This tick's controls. Clears the presses.
    pub fn controls(&mut self, now: Instant) -> crate::game::Controls {
        let mut held = [false; KEYS];
        for (d, h) in held.iter_mut().enumerate() {
            let k = &mut self.keys[d];
            let down = if self.exact { k.down } else { held_by_repeat(k, now) };
            *h = down || k.latched;
            k.latched = false;
        }
        let step = crate::game::KEYBOARD_STEP;
        let mut dx = 0;
        let mut dy = 0;
        if held[LEFT] || self.pad_dir[0] {
            dx -= step;
        }
        if held[RIGHT] || self.pad_dir[1] {
            dx += step;
        }
        if held[UP] || self.pad_dir[2] {
            dy -= step;
        }
        if held[DOWN] || self.pad_dir[3] {
            dy += step;
        }
        dx += self.mouse_dx + stick(self.joy.0);
        dy += self.mouse_dy + stick(self.joy.1);
        self.mouse_dx = 0;
        self.mouse_dy = 0;
        let c = crate::game::Controls {
            dx,
            dy,
            brake: held[BRAKE] || self.mouse_brake || self.pad_brake,
            fire: self.fire_pressed,
            held: held[FIRE] || self.mouse_fire || self.pad_fire,
            bomb: self.bomb_pressed,
            demo: None,
        };
        self.fire_pressed = false;
        self.bomb_pressed = false;
        c
    }
}

#[allow(unused_variables)]
fn stick(v: f32) -> i32 {
    #[cfg(feature = "gamepad")]
    if v.abs() > DEAD {
        return ((v.abs() - DEAD) / (1.0 - DEAD) * crate::game::KEYBOARD_STEP as f32 * v.signum()).round() as i32;
    }
    0
}

fn held_by_repeat(k: &Key, now: Instant) -> bool {
    let Some(last) = k.last else { return false };
    let window = if k.repeats == 0 { PRE_REPEAT } else { REPEAT_GAP };
    now.duration_since(last) <= window
}
