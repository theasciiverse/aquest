mod assets;
mod audio;
mod demo;
mod frame;
mod game;
mod input;
mod save;
mod starfield;
#[cfg(test)]
mod tests;
mod view;

use assets::Assets;
use audio::Audio;
use demo::Demo;
use frame::{Bonus, Frame, Rgb};
use game::{BUT_BOMB, BUT_FIRE, BUT_FIRE_HELD, DIFF_NAMES, Game, Outcome, PU_COUNT};
use input::{Cmd, Input};
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{
    self, DisableFocusChange, DisableMouseCapture, EnableFocusChange, EnableMouseCapture, Event, KeyboardEnhancementFlags, MouseButton,
    MouseEventKind, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::supports_keyboard_enhancement;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use save::{HiTable, NAME_MAX, Settings, View};
use starfield::Starfield;
use std::io;
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// The original's tick: 15 ms, about 67 a second.
const TICK: Duration = Duration::from_millis(15);
const SNAPSHOT_EVERY: Duration = Duration::from_millis(500);
/// Seconds idle on the menu before the demo plays (MenuTimeOut).
const MENU_IDLE: Duration = Duration::from_secs(30);
/// Mouse thrust per game pixel moved: about what xquest-sdl's relative
/// mouse gives at its usual window size.
const MOUSE_GAIN: f32 = 3.5;

// The level-end bonus, in ticks, as GiveBonus times it in Ticks (180 a
// second): half a second before and between the counts, a fifteenth of a
// second a step, two and a half seconds at the end.
const BONUS_PAUSE: u32 = 33;
const BONUS_STEP: u32 = 4;
const BONUS_HOLD: u32 = 167;

const HELP: &str = "aquest [options]

AQuest: XQuest, Mark Mackey's 1994 arcade game, in the terminal.
Fly your jolly little ship dingus round the arena, pick up every crystal,
dodge the mines and the Mucoids' many friends, and leave through the gate.

  arrows / WASD    thrust (the ship drifts on: brake to stop)
  Space / Enter    fire (hold it with RapidFire)
  Shift, Z or 5    brake
  B or X           smart bomb
  mouse            move to thrust, left fires, right bombs, middle brakes
  gamepad          stick or d-pad thrust, A fires, B bombs, X brakes
  P pause   Esc quit the game   V blocks or Braille   M sound

  --braille, --blocks   how to draw (V switches while playing)
  --demo           play the built-in demo, then exit
  --play FILE      play back a demo (xquest.dmo format), then exit
  --record FILE    record the next game to FILE
  --verify FILE    replay a demo without drawing it and print the result
  --difficulty N   0 Wimp, 1 Timid, 2 Average, 3 Tricky, 4 Inhuman
  --mute           no sound
  --version        show the version

Best in a terminal that reports key releases (the kitty keyboard
protocol: kitty, foot, WezTerm, Ghostty, Alacritty, iTerm2 and others),
with the mouse, or with a gamepad.
Session log: RUST_LOG=debug aquest 2>aquest.log";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Item {
    Start,
    Difficulty,
    Fame,
    PlayDemo,
    RecordDemo,
    View,
    Sound,
    Quit,
}

const ITEMS: [Item; 8] = [Item::Start, Item::Difficulty, Item::Fame, Item::PlayDemo, Item::RecordDemo, Item::View, Item::Sound, Item::Quit];

#[derive(Clone, Debug, PartialEq)]
enum Screen {
    Menu,
    Play,
    /// The Game Over box over the last picture.
    GameOver { until: Instant },
    /// The level-end window, `t` ticks in (GiveBonus).
    Bonus { t: u32 },
    /// "A New High Score!" flashing, then the name typed in.
    NewScore { rank: usize, typing: bool, until: Instant },
    Fame { diff: usize, until: Instant },
}

struct Playback {
    demo: Demo,
    at: usize,
}

struct App {
    a: Assets,
    frame: Frame,
    sf: Starfield,
    settings: Settings,
    table: HiTable,
    audio: Audio,
    input: Input,
    screen: Screen,
    sel: usize,
    game: Option<Game>,
    playback: Option<Playback>,
    recording: Option<(Demo, Option<PathBuf>)>,
    paused: bool,
    quit_confirm: bool,
    cycle: u32,
    idle_since: Instant,
    name: String,
    /// --demo or --play: leave after the demo.
    one_shot: bool,
    /// --record FILE
    record_to: Option<PathBuf>,
    record_now: bool,
    seed: Option<u32>,
    aspect: f32,
    /// Where the menu items were drawn, for the mouse.
    item_rows: Vec<(u16, usize)>,
    /// Game pixels per cell, for mouse thrust.
    px_per_cell: (f32, f32),
    last_mouse: Option<(u16, u16)>,
    quit: bool,
    /// While the bonus window is up: what it shows, and the points poured
    /// into the score so far.
    bonus_view: Option<Bonus>,
    poured: i64,
}

fn main() -> io::Result<()> {
    env_logger::init();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let has = |f: &str| args.iter().any(|a| a == f);
    let value = |f: &str| args.iter().position(|a| a == f).and_then(|i| args.get(i + 1)).filter(|v| !v.starts_with("--"));
    if has("--help") || has("-h") {
        println!("{HELP}");
        return Ok(());
    }
    if let Some(p) = value("--verify") {
        // the summary xquest-sdl's tests/test_replay.c prints, to compare
        let Some(d) = Demo::load(p.as_ref()) else {
            eprintln!("aquest: cannot read the demo {p}");
            std::process::exit(1);
        };
        let (frames, levels, crystals, score, deaths, over) = demo::replay(&Assets::load(), &d);
        println!("replayed {frames}/{} frames, {levels} levels, {crystals} crystals, {score} points, {deaths} deaths{}", d.frames.len(), if over { ", game over" } else { "" });
        return Ok(());
    }
    if has("--version") {
        println!("aquest {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    let mut settings = Settings::load();
    if has("--braille") {
        settings.view = View::Braille;
    }
    if has("--blocks") {
        settings.view = View::Blocks;
    }
    if let Some(d) = value("--difficulty").and_then(|v| v.parse::<usize>().ok()) {
        settings.difficulty = d.min(4);
    }
    let playback = if has("--demo") {
        Demo::parse(demo::BUILT_IN)
    } else if let Some(p) = value("--play") {
        match Demo::load(p.as_ref()) {
            Some(d) => Some(d),
            None => {
                eprintln!("aquest: cannot read the demo {p}");
                std::process::exit(1);
            }
        }
    } else {
        None
    };

    // open audio before the alternate screen, so any ALSA chatter lands above it
    let audio = Audio::new(has("--mute"), settings.sound);
    let a = Assets::load();
    let frame = Frame::new(&a);
    let mut terminal = ratatui::init();
    let kitty = supports_keyboard_enhancement().unwrap_or(false);
    if kitty {
        let flags = KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
            | KeyboardEnhancementFlags::REPORT_EVENT_TYPES
            | KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES;
        execute!(io::stdout(), PushKeyboardEnhancementFlags(flags))?;
    }
    let _ = execute!(io::stdout(), EnableFocusChange, EnableMouseCapture);
    let mut input = Input::new(kitty);
    input.open_pad();
    log::info!("start kitty_keyboard={kitty} pad={:?} audio={} settings={settings:?}", input.pad_name, audio.available());

    let mut app = App {
        a,
        frame,
        sf: Starfield::new(0),
        table: HiTable::load(),
        audio,
        input,
        screen: Screen::Menu,
        sel: 0,
        game: None,
        playback: None,
        recording: None,
        paused: false,
        quit_confirm: false,
        cycle: 0,
        idle_since: Instant::now(),
        name: String::new(),
        one_shot: playback.is_some(),
        record_to: value("--record").map(PathBuf::from),
        record_now: has("--record"),
        seed: value("--seed").and_then(|v| v.parse().ok()),
        aspect: view::cell_aspect(),
        item_rows: Vec::new(),
        px_per_cell: (1.0, 1.0),
        last_mouse: None,
        quit: false,
        bonus_view: None,
        poured: 0,
        settings,
    };
    if let Some(d) = playback {
        app.start_playback(d);
    } else if app.record_now {
        app.start_game(true);
    }
    let result = run(&mut terminal, &mut app);

    let _ = execute!(io::stdout(), DisableMouseCapture, DisableFocusChange);
    if kitty {
        let _ = execute!(io::stdout(), PopKeyboardEnhancementFlags);
    }
    ratatui::restore();
    app.settings.save();
    result
}

fn run(terminal: &mut ratatui::DefaultTerminal, app: &mut App) -> io::Result<()> {
    let mut next = Instant::now();
    let mut snapshot = Instant::now();
    while !app.quit {
        loop {
            let wait = next.saturating_duration_since(Instant::now());
            if !event::poll(wait)? {
                break;
            }
            match event::read()? {
                Event::Key(k) => {
                    log::debug!("key {:?} {:?} {:?}", k.code, k.kind, k.modifiers);
                    app.input.on_key(k, Instant::now());
                    app.idle_since = Instant::now();
                }
                Event::Mouse(m) => {
                    app.on_mouse(m.kind, m.column, m.row);
                }
                Event::FocusLost => {
                    log::debug!("focus lost");
                    app.input.release_all();
                }
                Event::Resize(w, h) => {
                    app.aspect = view::cell_aspect();
                    log::info!("resize {w}x{h} aspect {:.3}", app.aspect);
                }
                _ => {}
            }
        }
        app.input.poll_pad();
        let now = Instant::now();
        // catch up to two ticks if the terminal was slow, never more
        let mut ticks = 0;
        while now >= next && ticks < 2 {
            app.tick(now);
            next += TICK;
            ticks += 1;
        }
        if now >= next {
            next = now + TICK;
        }
        if now.duration_since(snapshot) >= SNAPSHOT_EVERY {
            snapshot = now;
            app.snapshot();
        }
        terminal.draw(|f| {
            let area = f.area();
            app.draw(f.buffer_mut(), area);
        })?;
    }
    Ok(())
}

fn vga_rgb(r: u8, g: u8, b: u8) -> Color {
    Color::Rgb(assets::vga8(r), assets::vga8(g), assets::vga8(b))
}

fn rgb(c: Rgb) -> Color {
    Color::Rgb(c[0], c[1], c[2])
}

/// Text at (x, y), cut at the buffer's edge.
fn put(buf: &mut Buffer, x: u16, y: u16, s: &str, style: Style) {
    let area = buf.area;
    if y < area.y || y >= area.bottom() {
        return;
    }
    let mut cx = x;
    for ch in s.chars() {
        if cx >= area.right() {
            break;
        }
        if cx >= area.x {
            buf[(cx, y)].set_char(ch).set_style(style);
        }
        cx += 1;
    }
}

fn put_centred(buf: &mut Buffer, r: Rect, y: u16, s: &str, style: Style) {
    let w = s.chars().count() as u16;
    put(buf, r.x + r.width.saturating_sub(w) / 2, y, s, style);
}

impl App {
    fn now_seed(&self) -> u32 {
        self.seed.unwrap_or_else(|| std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.subsec_nanos() ^ d.as_secs() as u32).unwrap_or(1))
    }

    fn start_game(&mut self, record: bool) {
        let seed = self.now_seed();
        let diff = self.settings.difficulty;
        log::info!("start game diff={} seed={seed} record={record}", DIFF_NAMES[diff]);
        self.game = Some(Game::new(&self.a, seed, diff));
        self.recording = record.then(|| (Demo::start(seed, diff), self.record_to.clone().or_else(save::demo_path)));
        self.playback = None;
        self.sf = Starfield::new(0);
        self.paused = false;
        self.quit_confirm = false;
        self.input.clear();
        self.screen = Screen::Play;
    }

    fn start_playback(&mut self, d: Demo) {
        let diff = d.difficulty().unwrap_or(self.settings.difficulty);
        log::info!("play demo seed={} diff={} frames={}", d.seed, DIFF_NAMES[diff], d.frames.len());
        self.game = Some(Game::new(&self.a, d.seed, diff));
        self.playback = Some(Playback { demo: d, at: 0 });
        self.recording = None;
        self.sf = Starfield::new(0);
        self.paused = false;
        self.quit_confirm = false;
        self.input.clear();
        self.screen = Screen::Play;
    }

    /// The demo the menu plays: the last one recorded, or the built-in one.
    fn menu_demo(&self) -> Option<Demo> {
        save::demo_path().and_then(|p| Demo::load(&p)).or_else(|| Demo::parse(demo::BUILT_IN))
    }

    fn to_menu(&mut self) {
        log::info!("menu");
        if self.one_shot {
            self.quit = true;
        }
        self.game = None;
        self.playback = None;
        self.screen = Screen::Menu;
        self.idle_since = Instant::now();
        self.sf = Starfield::new(0);
    }

    fn end_recording(&mut self) {
        if let Some((d, path)) = self.recording.take() {
            if let Some(p) = path {
                match d.save(&p) {
                    Ok(()) => log::info!("recorded {} frames to {}", d.frames.len(), p.display()),
                    Err(e) => log::warn!("could not write the demo to {}: {e}", p.display()),
                }
            }
        }
        self.record_now = false;
    }

    fn end_game(&mut self, score: i64) {
        self.end_recording();
        if let Some(g) = &mut self.game {
            g.score = score;
        }
        log::info!("game over score={score}");
        self.screen = Screen::GameOver { until: Instant::now() + Duration::from_secs(2) };
        self.input.clear();
    }

    fn click(&mut self) {
        self.audio.play(game::SND_MENUCLICK);
    }

    fn color_step(&mut self) {
        self.cycle = (self.cycle + 1) % 39;
    }

    /// The menu highlight: grey to blue and back (ColorCyclePal).
    fn highlight(&self) -> Color {
        const CC: [[u8; 3]; 13] =
            [[45, 45, 45], [40, 40, 48], [35, 35, 51], [30, 30, 54], [25, 25, 57], [20, 20, 60], [15, 15, 63], [20, 20, 60], [25, 25, 57], [30, 30, 54], [35, 35, 51], [40, 40, 48], [45, 45, 45]];
        let c = CC[(self.cycle / 3) as usize % 13];
        vga_rgb(c[0], c[1], c[2])
    }

    fn on_mouse(&mut self, kind: MouseEventKind, col: u16, row: u16) {
        match kind {
            MouseEventKind::Moved | MouseEventKind::Drag(_) => {
                if let Some((lc, lr)) = self.last_mouse
                    && self.screen == Screen::Play
                {
                    let dx = (col as f32 - lc as f32) * self.px_per_cell.0 * MOUSE_GAIN;
                    let dy = (row as f32 - lr as f32) * self.px_per_cell.1 * MOUSE_GAIN;
                    self.input.mouse_dx += dx.round() as i32;
                    self.input.mouse_dy += dy.round() as i32;
                }
                self.last_mouse = Some((col, row));
                if self.screen == Screen::Menu {
                    self.idle_since = Instant::now();
                    if let Some(&(_, i)) = self.item_rows.iter().find(|(r, _)| *r == row)
                        && i != self.sel
                    {
                        self.sel = i;
                        self.click();
                    }
                }
            }
            MouseEventKind::Down(b) => {
                log::debug!("mouse down {b:?} at {col},{row}");
                self.idle_since = Instant::now();
                let n = match b {
                    MouseButton::Left => 0,
                    MouseButton::Right => 1,
                    MouseButton::Middle => 2,
                };
                if self.screen == Screen::Play {
                    self.input.mouse_down(n);
                } else if self.screen == Screen::Menu {
                    if n == 0 && self.item_rows.iter().any(|(r, i)| *r == row && *i == self.sel) {
                        self.choose(true);
                    }
                } else {
                    self.input.cmds.push(Cmd::Any);
                }
            }
            MouseEventKind::Up(b) => {
                let n = match b {
                    MouseButton::Left => 0,
                    MouseButton::Right => 1,
                    MouseButton::Middle => 2,
                };
                self.input.mouse_up(n);
            }
            MouseEventKind::ScrollUp if self.screen == Screen::Menu => self.input.cmds.push(Cmd::Up),
            MouseEventKind::ScrollDown if self.screen == Screen::Menu => self.input.cmds.push(Cmd::Down),
            _ => {}
        }
    }

    /// Enter (or a click) on the menu. `forward` is the direction a
    /// setting turns.
    fn choose(&mut self, forward: bool) {
        match ITEMS[self.sel] {
            Item::Start => self.start_game(false),
            Item::Difficulty => {
                self.settings.difficulty = (self.settings.difficulty + if forward { 1 } else { 4 }) % 5;
                self.settings.save();
                self.click();
            }
            Item::Fame => {
                self.screen = Screen::Fame { diff: self.settings.difficulty, until: Instant::now() + Duration::from_secs(120) };
            }
            Item::PlayDemo => {
                if let Some(d) = self.menu_demo() {
                    self.start_playback(d);
                }
            }
            Item::RecordDemo => self.start_game(true),
            Item::View => {
                self.toggle_view();
                self.click();
            }
            Item::Sound => {
                self.toggle_sound();
                self.click();
            }
            Item::Quit => self.quit = true,
        }
    }

    fn toggle_view(&mut self) {
        self.settings.view = if self.settings.view == View::Blocks { View::Braille } else { View::Blocks };
        log::info!("view {:?}", self.settings.view);
        self.settings.save();
    }

    fn toggle_sound(&mut self) {
        self.settings.sound = !self.settings.sound;
        self.audio.on = self.settings.sound;
        log::info!("sound {}", self.settings.sound);
        self.settings.save();
    }

    fn tick(&mut self, now: Instant) {
        self.color_step();
        let cmds = std::mem::take(&mut self.input.cmds);
        if cmds.contains(&Cmd::Exit) {
            log::info!("ctrl+c");
            self.end_recording();
            self.quit = true;
            return;
        }
        // the switches work everywhere but in the name entry
        let typing = matches!(self.screen, Screen::NewScore { typing: true, .. });
        if !typing {
            for c in &cmds {
                match c {
                    Cmd::View => self.toggle_view(),
                    Cmd::Mute => self.toggle_sound(),
                    _ => {}
                }
            }
        }
        match self.screen.clone() {
            Screen::Menu => self.tick_menu(&cmds, now),
            Screen::Play => self.tick_play(&cmds, now),
            Screen::Bonus { t } => self.tick_bonus(&cmds, t),
            Screen::GameOver { until } => {
                if now >= until || cmds.contains(&Cmd::Any) {
                    self.after_game_over(now);
                }
            }
            Screen::NewScore { rank, typing, until } => self.tick_new_score(&cmds, rank, typing, until, now),
            Screen::Fame { diff, until } => {
                self.sf.step();
                let mut diff = diff;
                for c in &cmds {
                    match c {
                        Cmd::Left => diff = (diff + 4) % 5,
                        Cmd::Right => diff = (diff + 1) % 5,
                        Cmd::Any => {}
                        _ => {}
                    }
                }
                let other = cmds.iter().any(|c| matches!(c, Cmd::Confirm | Cmd::Back | Cmd::Char(_)));
                if now >= until || other {
                    self.to_menu();
                } else {
                    self.screen = Screen::Fame { diff, until };
                }
            }
        }
    }

    fn tick_menu(&mut self, cmds: &[Cmd], now: Instant) {
        self.sf.step();
        for c in cmds {
            match c {
                Cmd::Up | Cmd::Char('w' | 'W') => {
                    self.sel = (self.sel + ITEMS.len() - 1) % ITEMS.len();
                    self.click();
                }
                Cmd::Down | Cmd::Char('s' | 'S') => {
                    self.sel = (self.sel + 1) % ITEMS.len();
                    self.click();
                }
                Cmd::Left | Cmd::Right if matches!(ITEMS[self.sel], Item::Difficulty | Item::View | Item::Sound) => {
                    self.choose(*c == Cmd::Right);
                }
                Cmd::Confirm => {
                    self.choose(true);
                    return;
                }
                Cmd::Back => {
                    self.quit = true;
                    return;
                }
                _ => {}
            }
        }
        if now.duration_since(self.idle_since) >= MENU_IDLE {
            log::info!("menu idle: attract demo");
            if let Some(d) = self.menu_demo() {
                self.start_playback(d);
            }
        }
    }

    fn tick_play(&mut self, cmds: &[Cmd], now: Instant) {
        let demo = self.playback.is_some();
        if self.quit_confirm {
            // Y also arrives as "any key", so look for it first
            if cmds.contains(&Cmd::Yes) {
                log::info!("quit confirmed");
                self.quit_confirm = false;
                // as in the original: the game ends with no score
                self.end_game(0);
                return;
            }
            if cmds.contains(&Cmd::Any) {
                self.quit_confirm = false;
            }
        }
        for c in cmds {
            if self.quit_confirm {
                continue;
            }
            match c {
                Cmd::Back if demo => {
                    self.to_menu();
                    return;
                }
                Cmd::Back => {
                    log::info!("quit asked");
                    self.quit_confirm = true;
                }
                Cmd::Pause if !demo => {
                    self.paused = !self.paused;
                    log::info!("paused {}", self.paused);
                }
                _ => {}
            }
        }
        if self.paused || self.quit_confirm {
            // the controls pile up while paused; drop them
            self.input.controls(now);
            return;
        }
        let mut c = self.input.controls(now);
        if let Some(p) = &mut self.playback {
            // any key or button ends a demo (the original takes the buttons)
            if (c.fire || c.bomb || cmds.contains(&Cmd::Any)) && !self.one_shot {
                log::info!("demo stopped by a key");
                self.to_menu();
                return;
            }
            let Some(&(dx, dy, but)) = p.demo.frames.get(p.at) else {
                log::info!("demo ended");
                self.to_menu();
                return;
            };
            c = game::Controls { fire: but & BUT_FIRE != 0, held: but & BUT_FIRE_HELD != 0, bomb: but & BUT_BOMB != 0, demo: Some((dx as i32, dy as i32)), ..game::Controls::default() };
        }
        let Some(g) = &mut self.game else { return };
        let (out, but, took) = g.step(&self.a, c);
        if took {
            if let Some(p) = &mut self.playback {
                p.at += 1;
            }
            if let Some((d, _)) = &mut self.recording {
                d.push(g.rec_delx, g.rec_dely, but);
            }
        }
        for s in std::mem::take(&mut g.sounds) {
            self.audio.play(s);
        }
        self.sf.step();
        match out {
            Outcome::Playing => {}
            Outcome::LevelDone => {
                let (taken, bonus) = g.level_bonus();
                log::info!("level {} done in {taken} s, bonus {bonus}", g.level);
                self.audio.play(game::SND_PHEW);
                self.screen = Screen::Bonus { t: 0 };
            }
            Outcome::LifeLost => log::info!("life lost, {} left, level {}", g.lives, g.level),
            Outcome::GameOver | Outcome::Won => {
                let score = g.score;
                log::info!("{out:?} at level {} score {score}", g.level);
                if demo {
                    self.to_menu();
                } else {
                    self.end_game(score);
                }
            }
        }
    }

    /// The bonus window at tick `t`: what it shows and the points poured
    /// so far; None once it is over.
    fn bonus_at(&self, t: u32) -> Option<(Bonus, i64)> {
        let g = self.game.as_ref()?;
        let (taken, bonus) = g.level_bonus();
        let n = (bonus / 500) as u32;
        let up_end = BONUS_PAUSE + n * BONUS_STEP;
        let pause_end = up_end + if n > 0 { BONUS_PAUSE } else { 0 };
        let down_end = pause_end + n * BONUS_STEP;
        if t >= down_end + BONUS_HOLD {
            return None;
        }
        let (shown_taken, shown, poured) = if t < BONUS_PAUSE {
            (taken, 0, 0)
        } else if t < up_end {
            let i = (t - BONUS_PAUSE) / BONUS_STEP + 1;
            (taken + i as i32, i as i64 * 500, 0)
        } else if t < pause_end {
            (taken + n as i32, bonus, 0)
        } else if t < down_end {
            let j = ((t - pause_end) / BONUS_STEP + 1) as i64;
            (taken + n as i32, bonus - j * 500, j * 500)
        } else {
            (taken + n as i32, 0, bonus)
        };
        Some((Bonus { level: g.level, taken: shown_taken, par: g.level_rec().time, bonus: shown }, poured))
    }

    fn tick_bonus(&mut self, cmds: &[Cmd], t: u32) {
        self.sf.step();
        let Some(g) = &self.game else { return };
        let n = (g.level_bonus().1 / 500) as u32;
        let up_end = BONUS_PAUSE + n * BONUS_STEP;
        let pause_end = up_end + if n > 0 { BONUS_PAUSE } else { 0 };
        let down_end = pause_end + n * BONUS_STEP;
        // the countdown sound on every third step up and every step down
        if t >= BONUS_PAUSE && t < up_end && (t - BONUS_PAUSE) % BONUS_STEP == 0 && ((t - BONUS_PAUSE) / BONUS_STEP + 1) % 3 == 1
            || t >= pause_end && t < down_end && (t - pause_end) % BONUS_STEP == 0
        {
            self.audio.play(game::SND_COUNTDOWN);
        }
        // a key hurries it to the end, a second one closes it
        let key = cmds.iter().any(|c| matches!(c, Cmd::Any | Cmd::Confirm));
        let mut t = t + 1;
        if key {
            if t < down_end {
                if n > 0 {
                    self.audio.play(game::SND_COUNTDOWN);
                }
                t = down_end;
            } else {
                t = down_end + BONUS_HOLD;
            }
        }
        if let Some((b, poured)) = self.bonus_at(t) {
            self.bonus_view = Some(b);
            self.poured = poured;
            self.screen = Screen::Bonus { t };
            return;
        }
        self.bonus_view = None;
        self.poured = 0;
        let Some(g) = &mut self.game else { return };
        let out = g.finish_level(&self.a);
        for s in std::mem::take(&mut g.sounds) {
            self.audio.play(s);
        }
        log::info!("on to level {}, score {}, lives {}", g.level, g.score, g.lives);
        if out == Outcome::Won {
            let score = g.score;
            if self.playback.is_some() {
                self.to_menu();
            } else {
                self.end_game(score);
            }
        } else {
            self.screen = Screen::Play;
            self.input.clear();
        }
    }

    fn after_game_over(&mut self, now: Instant) {
        let Some(g) = &self.game else {
            self.to_menu();
            return;
        };
        let (diff, score, level) = (g.diff_level, g.score, g.level);
        self.sf = Starfield::new(0);
        if let Some(rank) = self.table.rank(diff, score) {
            log::info!("new score ranks {rank}");
            self.table.insert(diff, rank, score, level, "");
            self.name.clear();
            self.screen = Screen::NewScore { rank, typing: false, until: now + Duration::from_millis(1500) };
        } else {
            self.screen = Screen::Fame { diff, until: now + Duration::from_secs(5) };
        }
        self.input.clear();
    }

    fn tick_new_score(&mut self, cmds: &[Cmd], rank: usize, typing: bool, until: Instant, now: Instant) {
        self.sf.step();
        if !typing {
            if now >= until || cmds.contains(&Cmd::Any) {
                self.screen = Screen::NewScore { rank, typing: true, until };
            }
            return;
        }
        let diff = self.game.as_ref().map_or(self.settings.difficulty, |g| g.diff_level);
        let mut done = false;
        for c in cmds {
            match c {
                Cmd::Char(ch) if (' '..='~').contains(ch) && self.name.len() < NAME_MAX => self.name.push(*ch),
                Cmd::Backspace => {
                    self.name.pop();
                }
                Cmd::Confirm => done = true,
                Cmd::Back => {
                    self.name.clear();
                    done = true;
                }
                _ => {}
            }
        }
        if done {
            let name = if self.name.is_empty() { "Anonymous".to_string() } else { self.name.clone() };
            log::info!("name entered {name:?}");
            self.table.table[diff][rank].name = name;
            self.table.save();
            self.screen = Screen::Fame { diff, until: now + Duration::from_secs(4) };
        }
    }

    fn snapshot(&self) {
        match (&self.screen, &self.game) {
            (Screen::Play, Some(g)) => log::debug!(
                "snapshot play demo={} level={} score={} lives={} bombs={} crystals_left={} ship=({},{}) vel=({},{}) enemies={} paused={} pu={:?}",
                self.playback.is_some(),
                g.level,
                g.score,
                g.lives,
                g.bombs,
                g.num_crystals,
                g.ship.x,
                g.ship.y,
                g.ship.delx,
                g.ship.dely,
                g.enemies.len(),
                self.paused,
                g.powerup_timer
            ),
            (s, _) => log::debug!("snapshot {s:?} sel={}", self.sel),
        }
    }

    // ------------------------------------------------------------- drawing

    fn draw(&mut self, buf: &mut Buffer, area: Rect) {
        let black = Style::default().bg(Color::Rgb(0, 0, 0)).fg(Color::Rgb(180, 180, 180));
        buf.set_style(area, black);
        if area.width < 24 || area.height < 8 {
            put_centred(buf, area, area.y + area.height / 2, "make the window bigger", black);
            return;
        }
        match self.screen.clone() {
            Screen::Menu => self.draw_menu(buf, area),
            Screen::Play => {
                self.draw_play(buf, area);
            }
            Screen::Bonus { .. } => {
                let field = self.draw_play(buf, area);
                if let (Some(b), false) = (&self.bonus_view, self.full_size(area)) {
                    let l = b.lines();
                    self.draw_box(buf, field, &[&l[0], "", &l[1], &l[2], &l[3]]);
                }
            }
            Screen::GameOver { .. } => {
                let field = self.draw_play(buf, area);
                let score = self.game.as_ref().map_or(0, |g| g.score);
                self.draw_box(buf, field, &["Game Over", "", &format!("Score  {score}")]);
            }
            Screen::NewScore { rank, typing, .. } => {
                let diff = self.game.as_ref().map_or(self.settings.difficulty, |g| g.diff_level);
                let title = if rank == 0 { "A New High Score!" } else { "A Top Ten Score!" };
                self.draw_fame(buf, area, diff, title, Some((rank, typing)));
            }
            Screen::Fame { diff, .. } => {
                let title = format!("Hall of Fame  {}", DIFF_NAMES[diff]);
                self.draw_fame(buf, area, diff, &title, None);
            }
        }
    }

    /// The starfield over the whole 320x240 screen; returns where it went.
    /// Stars for a picture `tw` x `th` dots: the original's density, a little
    /// more so the sky is not empty.
    fn set_stars(&mut self, tw: u16, th: u16) {
        let share = tw as f32 * th as f32 / (frame::W * frame::H) as f32 * 3.0;
        self.frame.star_limit = (400.0 * share.clamp(0.12, 1.0)) as usize;
    }

    fn draw_backdrop(&mut self, buf: &mut Buffer, area: Rect, banner: bool) -> Rect {
        let p = view::place(area, frame::W, frame::H, self.settings.view, self.aspect);
        self.set_stars(p.tw, p.th);
        self.frame.clear();
        self.frame.stars(&self.sf, frame::H);
        if banner {
            let s = &self.a.startpic;
            self.frame.sprite_2xv(s, 0, 10);
        }
        view::draw(buf, &self.frame, (0, 0, frame::W, frame::H), &p, self.settings.view);
        p.cells
    }

    fn draw_menu(&mut self, buf: &mut Buffer, area: Rect) {
        // the bottom line keeps the keys
        let pic = Rect { height: area.height - 1, ..area };
        let r = self.draw_backdrop(buf, pic, true);
        let normal = Style::default().fg(vga_rgb(45, 45, 45)).bg(Color::Rgb(0, 0, 0));
        let hl = Style::default().fg(self.highlight()).bg(Color::Rgb(0, 0, 0)).add_modifier(Modifier::BOLD);
        // the items sit where the original put them, below the banner
        let below = (100 * r.height as u32).div_ceil(240) as u16;
        let rows_left = r.height.saturating_sub(below);
        let gap = if rows_left as usize >= ITEMS.len() * 2 { 2 } else { 1 };
        let top = r.y + below + rows_left.saturating_sub(ITEMS.len() as u16 * gap) / 2;
        self.item_rows.clear();
        for (i, item) in ITEMS.iter().enumerate() {
            let label = match item {
                Item::Start => "Start Game".to_string(),
                Item::Difficulty => format!("Difficulty  < {} >", DIFF_NAMES[self.settings.difficulty]),
                Item::Fame => "Hall of Fame".into(),
                Item::PlayDemo => "Play Demo".into(),
                Item::RecordDemo => "Record Demo".into(),
                Item::View => format!("Graphics  < {} >", if self.settings.view == View::Blocks { "Blocks" } else { "Braille" }),
                Item::Sound => format!("Sound  < {} >", if !self.audio.available() { "none" } else if self.settings.sound { "On" } else { "Off" }),
                Item::Quit => "Quit".into(),
            };
            let y = top + i as u16 * gap;
            let label = if i == self.sel { format!("▸ {label} ◂") } else { label };
            put_centred(buf, r, y, &label, if i == self.sel { hl } else { normal });
            self.item_rows.push((y, i));
        }
        let keys = "arrows choose  Enter go  V graphics  M sound  Esc quit";
        put_centred(buf, area, area.bottom() - 1, keys, Style::default().fg(Color::Rgb(90, 90, 90)));
    }

    /// Whether the picture fits at full size, with the original status bar.
    fn full_size(&self, area: Rect) -> bool {
        view::place(area, frame::W, frame::H, self.settings.view, self.aspect).tw as i32 >= frame::W
    }

    /// The game; returns the field's cells.
    fn draw_play(&mut self, buf: &mut Buffer, area: Rect) -> Rect {
        let v = self.settings.view;
        let whole = view::place(area, frame::W, frame::H, v, self.aspect);
        self.set_stars(whole.tw, whole.th);
        let full = whole.tw as i32 >= frame::W;
        let Some(g) = &self.game else { return area };
        self.frame.game(&self.a, g, &self.sf, full);
        if let (Some(b), true) = (&self.bonus_view, full) {
            self.frame.bonus_window(&self.a, b);
        }
        // the original status bar once the picture is full size
        let (field, hud_row) = if whole.tw as i32 >= frame::W {
            view::draw(buf, &self.frame, (0, 0, frame::W, frame::H), &whole, v);
            (whole.cells, None)
        } else {
            let p = view::place(Rect { height: area.height - 1, ..area }, frame::W, frame::HUD_Y, v, self.aspect);
            view::draw(buf, &self.frame, (0, 0, frame::W, frame::HUD_Y), &p, v);
            (p.cells, Some(p.cells.bottom()))
        };
        self.px_per_cell = (frame::W as f32 / field.width as f32, frame::HUD_Y as f32 / field.height as f32);
        if let Some(y) = hud_row {
            self.draw_hud(buf, field, y);
        }
        let label = Style::default().fg(Color::Rgb(255, 255, 255)).bg(Color::Rgb(0, 0, 0)).add_modifier(Modifier::BOLD);
        if !full && let Some(t) = self.game.as_ref().and_then(frame::countdown) {
            // the countdown to par, top right as in the original
            let s = format!(" {t} ");
            put(buf, field.right().saturating_sub(s.len() as u16 + 1), field.y, &s, label.fg(Color::Rgb(200, 200, 200)));
        }
        let mid = field.y + field.height / 2;
        if self.quit_confirm {
            put_centred(buf, field, mid, " QUIT?  Y quits, any other key plays on ", label);
        } else if self.paused {
            put_centred(buf, field, mid, " PAUSE ", label);
        }
        if self.playback.is_some() {
            let s = if self.one_shot { " DEMO " } else { " DEMO  any key ends it " };
            put_centred(buf, field, field.y, s, label.fg(self.highlight()));
        }
        field
    }

    /// The status bar as text: score, power-ups, ships, bombs, crystals.
    fn draw_hud(&self, buf: &mut Buffer, field: Rect, y: u16) {
        let Some(g) = &self.game else { return };
        let a = &self.a;
        let bg = Color::Rgb(8, 8, 8);
        buf.set_style(Rect::new(field.x, y, field.width, 1), Style::default().bg(bg));
        let st = |c: Rgb| Style::default().fg(rgb(c)).bg(bg);
        let white = Style::default().fg(Color::Rgb(230, 230, 230)).bg(bg).add_modifier(Modifier::BOLD);
        let dim = Style::default().fg(Color::Rgb(130, 130, 130)).bg(bg);

        let left = format!(" {:>8}", g.score + self.poured);
        put(buf, field.x, y, &left, white);
        let rec = g.level_rec();
        let right = [
            ("▲", (g.lives - 1).max(0).to_string(), frame::main_colour(a, &a.ship_icon)),
            ("✱", g.bombs.to_string(), frame::main_colour(a, &a.smart_bomb_hud)),
            ("◆", format!("{}/{}", rec.numcryst - g.num_crystals, rec.numcryst), frame::main_colour(a, &a.crystal_hud)),
        ];
        let lvl = format!("L{} ", g.level);
        let rlen: usize = right.iter().map(|(_, t, _)| t.chars().count() + 3).sum::<usize>() + lvl.len();
        let mut x = field.right().saturating_sub(rlen as u16);
        for (icon, text, c) in &right {
            put(buf, x, y, icon, st(*c));
            put(buf, x + 1, y, text, white);
            x += text.chars().count() as u16 + 3;
        }
        put(buf, x, y, &lvl, dim);

        // power-ups between, full names if they fit
        const NAMES: [&str; PU_COUNT] = ["Shield", "Aimed", "Rapid", "Multi", "Ass", "Heavy", "Bounce"];
        let on: Vec<usize> = (0..PU_COUNT).filter(|&p| g.powerup_timer[p] > 0).collect();
        let x0 = field.x + left.len() as u16 + 2;
        let room = field.right().saturating_sub(rlen as u16 + 2).saturating_sub(x0) as usize;
        let full: usize = on.iter().map(|&p| NAMES[p].len() + 1).sum();
        let mut x = x0;
        for &p in &on {
            let name = if full <= room { NAMES[p] } else { &NAMES[p][..2] };
            if frame::powerup_shown(g.powerup_timer[p]) {
                put(buf, x, y, name, st(frame::main_colour(a, &a.powerup_hud[p])).add_modifier(Modifier::BOLD));
            }
            x += name.len() as u16 + 1;
        }
    }

    fn draw_box(&self, buf: &mut Buffer, over: Rect, lines: &[&str]) {
        let w = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0) as u16 + 6;
        let h = lines.len() as u16 + 2;
        let x = over.x + over.width.saturating_sub(w) / 2;
        let y = over.y + over.height.saturating_sub(h) / 2;
        let border = Style::default().fg(Color::Rgb(255, 255, 255)).bg(Color::Rgb(8, 8, 8));
        let inner = Style::default().fg(vga_rgb(45, 45, 45)).bg(Color::Rgb(8, 8, 8)).add_modifier(Modifier::BOLD);
        for row in 0..h {
            let s: String = if row == 0 {
                format!("┏{}┓", "━".repeat(w as usize - 2))
            } else if row == h - 1 {
                format!("┗{}┛", "━".repeat(w as usize - 2))
            } else {
                format!("┃{}┃", " ".repeat(w as usize - 2))
            };
            put(buf, x, y + row, &s, border);
        }
        for (i, l) in lines.iter().enumerate() {
            put_centred(buf, Rect::new(x, y, w, h), y + 1 + i as u16, l, inner);
        }
    }

    fn draw_fame(&mut self, buf: &mut Buffer, area: Rect, diff: usize, title: &str, entry: Option<(usize, bool)>) {
        let r = self.draw_backdrop(buf, area, false);
        let bg = Color::Rgb(0, 0, 0);
        let normal = Style::default().fg(vga_rgb(45, 45, 45)).bg(bg);
        let hl = Style::default().fg(self.highlight()).bg(bg).add_modifier(Modifier::BOLD);
        let lines = 13u16;
        let gap = if r.height >= lines * 2 { 2 } else { 1 };
        let top = r.y + r.height.saturating_sub((lines - 1) * gap + 1) / 2;
        let width = 40u16.min(r.width);
        let x = r.x + r.width.saturating_sub(width) / 2;
        put_centred(buf, r, top, title, hl);
        let row = |buf: &mut Buffer, y: u16, name: &str, score: &str, lv: &str, s: Style| {
            put(buf, x, y, name, s);
            put(buf, x + width.saturating_sub(5 + score.len() as u16), y, score, s);
            put(buf, x + width.saturating_sub(lv.len() as u16), y, lv, s);
        };
        row(buf, top + 2 * gap, "Name", "Score", "Lv", normal);
        let blink = (self.cycle / 10) % 2 == 0;
        for (k, e) in self.table.table[diff].iter().enumerate() {
            let y = top + (3 + k as u16) * gap;
            let mine = entry.is_some_and(|(r, _)| r == k);
            let name = match entry {
                Some((r, true)) if r == k => format!("{}{}", self.name, if blink { "_" } else { " " }),
                Some((r, false)) if r == k => String::new(),
                _ => e.name.clone(),
            };
            row(buf, y, &name, &e.score.to_string(), &e.level.to_string(), if mine { hl } else { normal });
        }
        let hint = match entry {
            Some((_, true)) => "type your name, Enter when done",
            Some(_) => "",
            None => "left and right: other difficulties",
        };
        put_centred(buf, area, area.bottom() - 1, hint, Style::default().fg(Color::Rgb(90, 90, 90)));
    }
}
