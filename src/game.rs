//! The game itself: a line-by-line port of xquest-sdl's game.c, entities.c
//! and collision.c, which follow Mark Mackey's xquest.pas. Every number,
//! table and random draw is kept in the same order, so a demo recorded in
//! xquest-sdl replays here frame for frame (see `tests/replay.rs`).
//!
//! World coordinates are the original's: a 392x320 playfield seen through a
//! 320x217 window that scrolls after the ship. Positions are fixed point,
//! 64 to the pixel. The game ticks at the original's ~67 per second.

use crate::assets::{Assets, ENEMY_KINDS, Sprite};
use std::f64::consts::PI;

pub const WORLD_W: i32 = 392;
pub const WORLD_H: i32 = 320;
pub const VIEWPORT_W: i32 = 320;
pub const VIEWPORT_H: i32 = 217;

const MAX_VISIBLE_X: i32 = 72;
const MAX_VISIBLE_Y: i32 = 103;
const SCREEN_H_BORDER: i32 = 140;
const SCREEN_V_BORDER: i32 = 108;

const SHIP_MIN_X: i32 = 10;
const SHIP_MAX_X: i32 = 381;
const SHIP_MIN_Y: i32 = 10;
const SHIP_MAX_Y: i32 = 309;
const SHIP_START_X: i32 = 196;
const SHIP_START_Y: i32 = 160;

const MAX_SHIP_SPEED: i32 = 640;
/// Ticks a second (Framerate in xqvars.pas).
pub const FRAME_RATE: i64 = 67;
/// Thrust a held direction key gives each tick.
pub const KEYBOARD_STEP: i32 = 10;

const MINE_X_MIN: i32 = 11;
const MINE_X_MAX: i32 = 371;
const MINE_Y_MIN: i32 = 14;
const MINE_Y_MAX: i32 = 299;
const OBJ_W: i32 = 11;
const OBJ_H: i32 = 11;

const MAX_OBJECTS: usize = 65;
pub const MAX_LEVELS: usize = 50;
const START_LIVES: i32 = 3;
const START_BOMBS: i32 = 3;

pub const PU_SHIELD: usize = 0;
pub const PU_AIMED: usize = 1;
pub const PU_RAPID: usize = 2;
pub const PU_MULTI: usize = 3;
pub const PU_ASS: usize = 4;
pub const PU_HEAVY: usize = 5;
pub const PU_BOUNCE: usize = 6;
pub const PU_COUNT: usize = 7;

const PU_TIMEMIN_SHIELD: i32 = 670;
const PU_TIMRAN_SHIELD: i32 = 1005;
const PU_TIMEMIN_AIMED: i32 = 2010;
const PU_TIMRAN_AIMED: i32 = 4020;
const PU_TIMEMIN_LONG: i32 = 4020;
const PU_TIMRAN_LONG: i32 = 6030;
const PU_TIMEMIN_BOUNCE: i32 = 2010;
const PU_TIMRAN_BOUNCE: i32 = 4020;

pub const SHIP_EXPLODE_TICKS: i32 = 40;

const MAX_ENEMIES: usize = 40;
const MAX_ENEMY_MISSILES: usize = 70;
const ENEMY_ENTER_TICKS: i32 = 80;
const ENEMY_X_MIN: i32 = 10;
const ENEMY_X_MAX: i32 = 382;
const ENEMY_Y_MIN: i32 = 10;
const ENEMY_Y_MAX: i32 = 310;
const ENEMY_START_Y: i32 = 155;
const MAX_MISSILES: usize = 50;
const MISSILE_LIFE: i32 = 300;

pub const DIFF_NAMES: [&str; 5] = ["Wimp", "Timid", "Average", "Tricky", "Inhuman"];

// Sound numbers, as in xqvars.pas (1-based).
pub const SND_PHEW: u8 = 3;
pub const SND_FIRE4: u8 = 4;
pub const SND_FIRE: u8 = 5;
pub const SND_BOING: u8 = 6;
pub const SND_WOOHOO: u8 = 8;
pub const SND_GETCRYSTAL: u8 = 11;
pub const SND_EXPLOSN: u8 = 12;
pub const SND_COUNTDOWN: u8 = 17;
pub const SND_GATESOUND: u8 = 18;
pub const SND_ENEMYENT: u8 = 22;
pub const SND_MENUCLICK: u8 = 23;
pub const SND_REPULSE: u8 = 25;

// Demo button bits.
pub const BUT_FIRE: u8 = 1;
pub const BUT_BOMB: u8 = 2;
pub const BUT_FIRE_HELD: u8 = 4;

/// One level, as in xqvars.pas. `bonus_level` is kept as the table has it;
/// the original compiled bonus levels out ({$UNDEF BONUSLEVEL}).
#[allow(dead_code)]
pub struct Level {
    pub numcryst: i32,
    pub nummine: i32,
    pub maxsmart: i32,
    pub smartprob: f32,
    pub newman: i32,
    pub maxenemies: i32,
    pub erelease: f32,
    pub gate_width: i32,
    pub gate_move: i32,
    pub gate_cdir_prob: f32,
    pub time: i32,
    pub bonus_level: bool,
}

#[allow(clippy::too_many_arguments)]
const fn lv(
    numcryst: i32,
    nummine: i32,
    maxsmart: i32,
    smartprob: f32,
    newman: i32,
    maxenemies: i32,
    erelease: f32,
    gate_width: i32,
    gate_move: i32,
    gate_cdir_prob: f32,
    time: i32,
    bonus_level: bool,
) -> Level {
    Level { numcryst, nummine, maxsmart, smartprob, newman, maxenemies, erelease, gate_width, gate_move, gate_cdir_prob, time, bonus_level }
}

pub struct Kind {
    pub speed: i32,
    pub speed2: i32,
    pub curve: i32,
    pub curve2: i32,
    pub hits: i32,
    pub firetype: i32,
    pub score: i32,
    pub fires: bool,
    pub follows: bool,
    pub curves: bool,
    pub explodes: bool,
    pub laysmines: bool,
    pub shootback: bool,
    pub zoom: bool,
    pub maxspeed: bool,
    pub rebounds: bool,
    pub tribbles: bool,
    pub repulses: bool,
    pub fireprob: f32,
    pub changedir: f32,
    pub changecurve: f32,
    pub follow: f32,
    pub numframes: i32,
    pub framespeed: i32,
    pub death_sound: u8,
}

#[allow(clippy::too_many_arguments)]
const fn kind(
    speed: i32,
    speed2: i32,
    curve: i32,
    curve2: i32,
    hits: i32,
    firetype: i32,
    score: i32,
    fires: bool,
    follows: bool,
    curves: bool,
    explodes: bool,
    laysmines: bool,
    shootback: bool,
    zoom: bool,
    maxspeed: bool,
    rebounds: bool,
    tribbles: bool,
    repulses: bool,
    fireprob: f32,
    changedir: f32,
    changecurve: f32,
    follow: f32,
    numframes: i32,
    framespeed: i32,
    death_sound: u8,
) -> Kind {
    Kind {
        speed,
        speed2,
        curve,
        curve2,
        hits,
        firetype,
        score,
        fires,
        follows,
        curves,
        explodes,
        laysmines,
        shootback,
        zoom,
        maxspeed,
        rebounds,
        tribbles,
        repulses,
        fireprob,
        changedir,
        changecurve,
        follow,
        numframes,
        framespeed,
        death_sound,
    }
}

/// Speed factor, enemy frequency, and whether the ship bounces off walls.
/// `rebound` is not read: xquest-sdl bounces off walls on Wimp alone.
#[allow(dead_code)]
pub struct Diff {
    pub speedfactor: f32,
    pub enemyfrequency: f32,
    pub rebound: bool,
}

pub const DIFFS: [Diff; 5] = [
    Diff { speedfactor: 0.7, enemyfrequency: 0.7, rebound: true },
    Diff { speedfactor: 1.0, enemyfrequency: 1.0, rebound: true },
    Diff { speedfactor: 1.0, enemyfrequency: 1.0, rebound: false },
    Diff { speedfactor: 1.5, enemyfrequency: 1.2, rebound: false },
    Diff { speedfactor: 2.0, enemyfrequency: 1.5, rebound: false },
];

// Enemy missiles by fire type (1 to 6; 0 unused).
const EM_SPEED: [i32; 7] = [0, 120, 150, 200, 150, 150, 170];
const EM_DIRECT: [bool; 7] = [false, false, false, true, false, true, false];
const EM_REBOUND: [bool; 7] = [false, false, false, false, true, false, false];

/// xorshift32, as in xquest-sdl.
#[derive(Clone, Copy, Debug)]
pub struct Rng(pub u32);

impl Rng {
    pub fn next(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        x
    }
    pub fn float(&mut self) -> f32 {
        (self.next() >> 8) as f32 / (1u32 << 24) as f32
    }
    pub fn range(&mut self, n: i32) -> i32 {
        (self.next() % n as u32) as i32
    }
}

#[derive(Clone, Copy, Default)]
pub struct Ship {
    pub sx: i32,
    pub sy: i32,
    pub x: i32,
    pub y: i32,
    pub xbr: i32,
    pub ybr: i32,
    pub delx: i32,
    pub dely: i32,
    pub dir: usize,
    pub w: i32,
    pub h: i32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ObjType {
    Crystal,
    Mine,
    Smart,
    EnemyMine,
}

#[derive(Clone, Copy)]
pub struct Object {
    pub x: i32,
    pub y: i32,
    pub xbr: i32,
    pub ybr: i32,
    pub kind: ObjType,
    pub active: bool,
}

#[derive(Clone, Copy, Default)]
pub struct Enemy {
    pub sx: i32,
    pub sy: i32,
    pub x: i32,
    pub y: i32,
    pub xbr: i32,
    pub ybr: i32,
    pub delx: i32,
    pub dely: i32,
    pub curvecos: i32,
    pub curvesin: i32,
    pub kind: usize,
    pub hit: i32,
    /// Animation counter; the sprite is frame >> 8.
    pub frame: i32,
    pub supertime: i32,
    pub active: bool,
}

#[derive(Clone, Copy, Default)]
pub struct Missile {
    pub sx: i32,
    pub sy: i32,
    pub x: i32,
    pub y: i32,
    pub xbr: i32,
    pub ybr: i32,
    pub delx: i32,
    pub dely: i32,
    pub time: i32,
    pub active: bool,
}

#[derive(Clone, Copy, Default)]
pub struct EnemyMissile {
    pub sx: i32,
    pub sy: i32,
    pub x: i32,
    pub y: i32,
    pub xbr: i32,
    pub ybr: i32,
    pub delx: i32,
    pub dely: i32,
    pub firetype: i32,
    pub active: bool,
}

/// One tick's controls.
#[derive(Clone, Copy, Default, Debug)]
pub struct Controls {
    pub dx: i32,
    pub dy: i32,
    pub brake: bool,
    /// The fire button went down this tick.
    pub fire: bool,
    /// The fire button is held (RapidFire).
    pub held: bool,
    pub bomb: bool,
    /// Demo playback: the recorded velocity, replacing the controls.
    pub demo: Option<(i32, i32)>,
}

/// What a tick ended in.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Outcome {
    Playing,
    /// Flew out through the gate. The game waits on this level until
    /// `finish_level` gives the bonus and moves on.
    LevelDone,
    /// The explosion finished and a life was lost; the ship is back at the
    /// start of the same level.
    LifeLost,
    /// No lives left.
    GameOver,
    /// Through the gate of level 50.
    Won,
}

#[derive(Clone)]
pub struct Game {
    pub ship: Ship,
    pub cam_x: i32,
    pub cam_y: i32,
    pub level: i32,
    pub score: i64,
    pub lives: i32,
    pub bombs: i32,
    pub num_crystals: i32,
    pub gate_open: bool,
    pub gate_left_x: i32,
    pub gate_right_x: i32,
    pub objects: Vec<Object>,
    pub missiles: Vec<Missile>,
    pub frame_count: i32,
    pub enemies: Vec<Enemy>,
    pub emissiles: Vec<EnemyMissile>,
    pub enemy_entering_left: i32,
    pub enemy_left_type: usize,
    pub enemy_entering_right: i32,
    pub enemy_right_type: usize,
    pub powerup_timer: [i32; PU_COUNT],
    pub diff_level: usize,
    pub gamespeed: i32,
    /// The score that brings the next extra ship, and the one that brought
    /// the last (NewManScore, LastNewManScore).
    pub new_man_score: i64,
    pub last_new_man_score: i64,
    /// Ticks on this level, against its par (TimeOnLevel). Keeps running
    /// through a lost life.
    pub time_on_level: i64,
    pub rng: Rng,
    pub ship_destroyed: bool,
    pub ship_explode_timer: i32,
    pub gate_inner_x: i32,
    pub level_complete: bool,
    gate_move_accum: i32,
    gate_move_dir: i32,
    /// The smart bomb flash, 11 down to 0.
    pub smart_bomb_flash: i32,
    demo_override: Option<(i32, i32)>,
    /// The velocity a recording stores for this tick.
    pub rec_delx: i32,
    pub rec_dely: i32,
    /// Sounds started this tick, drained by the caller.
    pub sounds: Vec<u8>,
}

fn kinds(i: usize) -> &'static Kind {
    &KINDS[i]
}

impl Game {
    /// A new game at difficulty `diff` (0 Wimp to 4 Inhuman).
    pub fn new(a: &Assets, seed: u32, diff: usize) -> Game {
        let mut g = Game::blank(a, seed);
        g.diff_level = diff;
        g.level_init(a);
        g
    }

    /// game_init in xquest-sdl.
    fn blank(a: &Assets, seed: u32) -> Game {
        let (w, h) = (a.ship[0].w, a.ship[0].h);
        let ship = Ship {
            w,
            h,
            x: SHIP_START_X,
            y: SHIP_START_Y,
            xbr: SHIP_START_X + w - 1,
            ybr: SHIP_START_Y + h - 1,
            sx: SHIP_START_X * 64,
            sy: SHIP_START_Y * 64,
            ..Ship::default()
        };
        let cam_x = (ship.x - VIEWPORT_W / 2).clamp(0, MAX_VISIBLE_X);
        let cam_y = (ship.y - VIEWPORT_H / 2).clamp(0, MAX_VISIBLE_Y);
        Game {
            ship,
            cam_x,
            cam_y,
            level: 1,
            score: 0,
            lives: START_LIVES,
            bombs: START_BOMBS,
            num_crystals: 0,
            gate_open: false,
            gate_left_x: 0,
            gate_right_x: 0,
            objects: Vec::new(),
            missiles: Vec::new(),
            frame_count: 0,
            enemies: Vec::new(),
            emissiles: Vec::new(),
            enemy_entering_left: 0,
            enemy_left_type: 0,
            enemy_entering_right: 0,
            enemy_right_type: 0,
            powerup_timer: [0; PU_COUNT],
            diff_level: 2,
            gamespeed: 64,
            new_man_score: 0,
            last_new_man_score: 0,
            time_on_level: 0,
            rng: Rng(if seed != 0 { seed } else { 0x9E37_79B9 }),
            ship_destroyed: false,
            ship_explode_timer: 0,
            gate_inner_x: 0,
            level_complete: false,
            gate_move_accum: 0,
            gate_move_dir: 1,
            smart_bomb_flash: 0,
            demo_override: None,
            rec_delx: 0,
            rec_dely: 0,
            sounds: Vec::new(),
        }
    }

    fn play(&mut self, s: u8) {
        self.sounds.push(s);
    }

    pub fn level_rec(&self) -> &'static Level {
        &LEVELS[(self.level - 1) as usize]
    }

    /// round(BaseGameSpeed * speedfactor): 45, 64, 64, 96, 128.
    pub fn base_speed(diff: usize) -> i32 {
        (64.0 * DIFFS[diff].speedfactor as f64).round() as i32
    }

    /// AddToScore: an extra ship for each `new_man_score` passed.
    pub fn add_score(&mut self, points: i64) {
        self.score += points;
        while self.score >= self.new_man_score {
            self.last_new_man_score = self.new_man_score;
            self.new_man_score = self.last_new_man_score + self.level_rec().newman as i64;
            self.lives += 1;
            self.play(SND_WOOHOO);
        }
    }

    /// The level clock, last thing in a tick; past par the game speeds up
    /// by one base speed for every 32 seconds over.
    fn clock_tick(&mut self) {
        self.time_on_level += 1;
        let par = self.level_rec().time as i64 * FRAME_RATE;
        if self.time_on_level > par {
            self.gamespeed = Game::base_speed(self.diff_level) + ((self.time_on_level - par) * 64 / (FRAME_RATE * 32)) as i32;
        }
    }

    /// Seconds taken on this level and the bonus they earn (GiveBonus):
    /// 500 for every whole second under par.
    pub fn level_bonus(&self) -> (i32, i64) {
        let taken = (self.time_on_level / FRAME_RATE) as i32;
        (taken, ((self.level_rec().time - taken) as i64 * 500).max(0))
    }

    /// After `LevelDone`: the bonus into the score, then the next level, or
    /// `Won` past level 50.
    pub fn finish_level(&mut self, a: &Assets) -> Outcome {
        let (_, bonus) = self.level_bonus();
        self.add_score(bonus);
        if self.level as usize >= MAX_LEVELS {
            return Outcome::Won;
        }
        let mut g = Game::blank(a, self.rng.0);
        g.score = self.score;
        g.level = self.level + 1;
        g.lives = self.lives;
        g.bombs = self.bombs;
        g.diff_level = self.diff_level;
        g.last_new_man_score = self.last_new_man_score;
        g.sounds = std::mem::take(&mut self.sounds);
        *self = g;
        self.level_init(a);
        Outcome::LevelDone
    }

    /// NewLife + StartNewLevel: the ship back at the start, enemies, shots,
    /// enemy mines and power-ups gone, speed back to base; the level carries
    /// on with its crystals, gate and clock.
    fn life_restart(&mut self) {
        let (w, h) = (self.ship.w, self.ship.h);
        self.ship = Ship { w, h, x: SHIP_START_X, y: SHIP_START_Y, xbr: SHIP_START_X + w - 1, ybr: SHIP_START_Y + h - 1, sx: SHIP_START_X * 64, sy: SHIP_START_Y * 64, ..Ship::default() };
        self.cam_x = (SHIP_START_X - VIEWPORT_W / 2).clamp(0, MAX_VISIBLE_X);
        self.cam_y = (SHIP_START_Y - VIEWPORT_H / 2).clamp(0, MAX_VISIBLE_Y);
        self.demo_override = None;
        self.ship_destroyed = false;
        self.ship_explode_timer = 0;
        self.level_complete = false;
        self.smart_bomb_flash = 0;
        self.frame_count = 0;
        self.gamespeed = Game::base_speed(self.diff_level);
        self.powerup_timer = [0; PU_COUNT];
        self.entities_level_init();
        compact(&mut self.objects, |o| o.active && o.kind != ObjType::EnemyMine);
    }

    fn entities_level_init(&mut self) {
        self.missiles.clear();
        self.enemies.clear();
        self.emissiles.clear();
        self.enemy_entering_left = 0;
        self.enemy_entering_right = 0;
        self.enemy_left_type = 0;
        self.enemy_right_type = 0;
    }

    fn obj_no_overlap(&self, count: usize, o: &Object) -> bool {
        let (sw, sh) = (self.ship.w, self.ship.h);
        if o.xbr + 8 >= SHIP_START_X && o.ybr + 8 >= SHIP_START_Y && o.x <= SHIP_START_X + sw + 8 && o.y <= SHIP_START_Y + sh + 8 {
            return false;
        }
        !self.objects[..count].iter().any(|p| (o.x - p.x).abs() < OBJ_W + 2 && (o.y - p.y).abs() < OBJ_H + 2)
    }

    fn level_init(&mut self, a: &Assets) {
        let rec = self.level_rec();
        self.gate_left_x = (WORLD_W - rec.gate_width) / 2 - a.gate_left.w;
        self.gate_right_x = (WORLD_W + rec.gate_width) / 2;

        let mut num_smarts = 0;
        while self.rng.float() < rec.smartprob && num_smarts < rec.maxsmart {
            num_smarts += 1;
        }
        let total = ((rec.numcryst + rec.nummine + num_smarts) as usize).min(MAX_OBJECTS);

        self.num_crystals = rec.numcryst;
        self.gate_open = false;
        self.level_complete = false;
        self.gate_inner_x = (WORLD_W - rec.gate_width) / 2;
        self.gate_move_accum = 0;
        self.gate_move_dir = 1;
        self.new_man_score = self.last_new_man_score + rec.newman as i64;
        self.time_on_level = 0;
        self.gamespeed = Game::base_speed(self.diff_level);
        self.entities_level_init();

        self.objects.clear();
        for i in 0..total {
            let mut o = Object { x: 0, y: 0, xbr: 0, ybr: 0, kind: ObjType::Crystal, active: true };
            let mut attempts = 0;
            loop {
                o.x = MINE_X_MIN + self.rng.range(MINE_X_MAX - MINE_X_MIN - OBJ_W + 1);
                o.y = MINE_Y_MIN + self.rng.range(MINE_Y_MAX - MINE_Y_MIN - OBJ_H + 1);
                o.xbr = o.x + OBJ_W - 1;
                o.ybr = o.y + OBJ_H - 1;
                attempts += 1;
                if attempts >= 1000 || self.obj_no_overlap(i, &o) {
                    break;
                }
            }
            let i = i as i32;
            o.kind = if i < rec.numcryst {
                ObjType::Crystal
            } else if i < rec.numcryst + rec.nummine {
                ObjType::Mine
            } else {
                ObjType::Smart
            };
            self.objects.push(o);
        }
    }

    /// One tick, as main.c's game loop runs it. Returns how it ended, the
    /// demo buttons for this tick, and whether the tick took a demo frame
    /// (none are taken while the ship is exploding).
    pub fn step(&mut self, a: &Assets, c: Controls) -> (Outcome, u8, bool) {
        let exploding = self.ship_explode_timer > 0;
        let (fire, held, bomb) = (c.fire, c.held, c.bomb);
        self.demo_override = c.demo;
        let mut but = 0;
        if !exploding {
            if fire {
                self.shoot(a);
                but |= BUT_FIRE;
            }
            if self.powerup_timer[PU_RAPID] > 0 && !fire && held && (self.frame_count & 3) == 0 {
                self.shoot(a);
                but |= BUT_FIRE_HELD;
            }
            if bomb {
                self.fire_smart_bomb(a);
                but |= BUT_BOMB;
            }
            let (dx, dy, brake) = if c.demo.is_some() { (0, 0, false) } else { (c.dx, c.dy, c.brake) };
            self.tick_ship(a, dx, dy, brake);
            self.check_pickups();
            self.powerups_tick();
        }
        self.missiles_tick(a);
        self.entities_tick(a);
        self.emissiles_tick(a);
        if !exploding {
            self.check_collisions(a);
        }
        self.clock_tick();

        if self.level_complete {
            return (Outcome::LevelDone, but, !exploding);
        }
        if self.ship_destroyed {
            self.ship_destroyed = false;
            self.ship_destroy(a);
        }
        if self.ship_explode_timer > 0 {
            self.ship_explode_timer -= 1;
            if self.ship_explode_timer == 0 {
                self.lives -= 1;
                if self.lives <= 0 {
                    return (Outcome::GameOver, but, !exploding);
                }
                self.life_restart();
                return (Outcome::LifeLost, but, !exploding);
            }
        }
        (Outcome::Playing, but, !exploding)
    }

    /// game_tick in xquest-sdl: the ship, its walls, the camera and the gate.
    fn tick_ship(&mut self, a: &Assets, inp_dx: i32, inp_dy: i32, brake: bool) {
        // the rotation frame from the previous tick, as in the original
        let smask = &a.ship[self.ship.dir].mask;
        let s = &mut self.ship;
        s.delx += inp_dx;
        s.dely += inp_dy;
        if brake {
            s.delx = (s.delx * 5) / 6;
            s.dely = (s.dely * 5) / 6;
        }
        if let Some((dx, dy)) = self.demo_override {
            s.delx = dx;
            s.dely = dy;
        }
        self.rec_delx = s.delx;
        self.rec_dely = s.dely;

        let speed = s.delx.abs() + s.dely.abs();
        if speed > MAX_SHIP_SPEED {
            s.delx = (s.delx * MAX_SHIP_SPEED) / speed;
            s.dely = (s.dely * MAX_SHIP_SPEED) / speed;
        }
        s.sx += s.delx;
        s.sy += s.dely;
        s.x = s.sx / 64;
        s.y = s.sy / 64;
        s.xbr = s.x + s.w - 1;
        s.ybr = s.y + s.h - 1;

        let wall_bounce = self.diff_level == 0 || self.powerup_timer[PU_SHIELD] > 0;
        let sh = s.h;
        let hit_ver = |from: i32, to: i32| {
            let (from, to) = (from.max(0), to.min(sh - 1));
            (from..=to).any(|i| smask[i as usize] != 0)
        };
        let hit_hor = |wm: u32| smask.iter().any(|&m| m & wm != 0);
        let mut destroyed = false;

        if s.x < SHIP_MIN_X {
            let m = SHIP_MIN_X - s.x - 1;
            let wm = if m >= 31 {
                u32::MAX
            } else if m < 0 {
                0
            } else {
                !((1u32 << (31 - m)) - 1)
            };
            if wm != 0 && hit_hor(wm) {
                s.x = SHIP_MIN_X;
                s.sx = s.x * 64;
                if wall_bounce {
                    s.delx = s.delx.abs();
                } else {
                    destroyed = true;
                }
            }
        }
        if s.xbr > SHIP_MAX_X {
            let k = SHIP_MAX_X - s.x;
            let wm = if k < 0 {
                u32::MAX
            } else if k >= 31 {
                0
            } else {
                (1u32 << (31 - k)) - 1
            };
            if wm != 0 && hit_hor(wm) {
                s.x = SHIP_MAX_X - s.w;
                s.sx = s.x * 64;
                if wall_bounce {
                    s.delx = -s.delx.abs();
                } else {
                    destroyed = true;
                }
            }
        }
        if s.y < SHIP_MIN_Y {
            let in_gate = self.gate_open && s.xbr >= self.gate_inner_x && s.x <= self.gate_right_x;
            if in_gate {
                if s.y < 0 {
                    self.level_complete = true;
                }
            } else if hit_ver(0, SHIP_MIN_Y - s.y - 1) {
                s.y = SHIP_MIN_Y;
                s.sy = s.y * 64;
                if wall_bounce {
                    s.dely = s.dely.abs();
                } else {
                    destroyed = true;
                }
            }
        }
        if s.ybr > SHIP_MAX_Y && hit_ver(SHIP_MAX_Y - s.y + 1, s.h - 1) {
            s.y = SHIP_MAX_Y - s.h;
            s.sy = s.y * 64;
            if wall_bounce {
                s.dely = -s.dely.abs();
            } else {
                destroyed = true;
            }
        }
        s.xbr = s.x + s.w - 1;
        s.ybr = s.y + s.h - 1;
        if destroyed {
            self.ship_destroyed = true;
        }

        if s.delx != 0 || s.dely != 0 {
            let mut theta = (s.delx as f64).atan2(s.dely as f64);
            if theta < 0.0 {
                theta += 2.0 * PI;
            }
            s.dir = ((theta / (2.0 * PI) * 24.0).round() as i32 % 24) as usize;
        }

        // the camera (CheckWallCollisions in xquest.pas)
        let sx = s.x;
        let sy = s.y;
        let temp = sx - (self.cam_x + VIEWPORT_W - SCREEN_H_BORDER);
        if temp > 0 && self.cam_x < MAX_VISIBLE_X {
            self.cam_x = (self.cam_x + temp / 20 + 1).min(MAX_VISIBLE_X);
        }
        let temp = (self.cam_x + SCREEN_H_BORDER) - sx;
        if temp > 0 && self.cam_x > 0 {
            self.cam_x = (self.cam_x - (temp / 20 + 1)).max(0);
        }
        let temp = sy - (self.cam_y + VIEWPORT_H - SCREEN_V_BORDER);
        if temp > 0 && self.cam_y < MAX_VISIBLE_Y {
            self.cam_y = (self.cam_y + temp / 20 + 1).min(MAX_VISIBLE_Y);
        }
        let temp = (self.cam_y + SCREEN_V_BORDER) - sy;
        if temp > 0 && self.cam_y > 0 {
            self.cam_y = (self.cam_y - (temp / 20 + 1)).max(0);
        }

        // the moving gate, levels 33 on
        let lrec = self.level_rec();
        if lrec.gate_move > 0 {
            self.gate_move_accum += lrec.gate_move;
            if self.rng.float() < lrec.gate_cdir_prob {
                self.gate_move_dir = -self.gate_move_dir;
            }
            while self.gate_move_accum >= 64 {
                self.gate_move_accum -= 64;
                self.gate_left_x += self.gate_move_dir;
                self.gate_right_x += self.gate_move_dir;
                if self.gate_right_x > 362 {
                    self.gate_move_dir = -1;
                }
                if self.gate_left_x < 20 {
                    self.gate_move_dir = 1;
                }
            }
            self.gate_inner_x = self.gate_left_x + self.ship.w;
        }
    }

    fn check_pickups(&mut self) {
        let s = self.ship;
        for i in 0..self.objects.len() {
            let o = self.objects[i];
            if !o.active || s.x > o.xbr || s.xbr < o.x || s.y > o.ybr || s.ybr < o.y {
                continue;
            }
            self.objects[i].active = false;
            match o.kind {
                ObjType::Crystal => {
                    self.play(SND_GETCRYSTAL);
                    self.add_score(200);
                    self.num_crystals -= 1;
                    if self.num_crystals <= 0 {
                        self.num_crystals = 0;
                        self.gate_open = true;
                        self.play(SND_GATESOUND);
                    }
                }
                ObjType::Smart => self.bombs += 1,
                ObjType::Mine | ObjType::EnemyMine => self.ship_destroyed = true,
            }
        }
        if self.ship_destroyed && self.powerup_timer[PU_SHIELD] > 0 {
            self.ship_destroyed = false;
        }
    }

    fn powerups_tick(&mut self) {
        for t in &mut self.powerup_timer {
            if *t > 0 {
                *t -= 1;
            }
        }
        if self.smart_bomb_flash > 0 && (self.frame_count & 1) == 0 {
            self.smart_bomb_flash -= 1;
        }
    }

    fn fire_smart_bomb(&mut self, a: &Assets) {
        if self.bombs <= 0 {
            return;
        }
        for i in (0..self.enemies.len()).rev() {
            if !self.enemies[i].active {
                continue;
            }
            self.enemies[i].hit = 1;
            self.enemy_hit(a, i, 0, 0);
        }
        self.emissiles.clear();
        self.bombs -= 1;
        self.smart_bomb_flash = 11;
        self.play(SND_EXPLOSN);
    }

    // ------------------------------------------------------------ entities

    fn add_enemy(&mut self, a: &Assets, xc: i32, yc: i32, kind: usize) {
        if self.enemies.len() >= MAX_ENEMIES {
            return;
        }
        let k = kinds(kind);
        let (w, h) = (a.enemy[kind][0].w, a.enemy[kind][0].h);
        let mut e = Enemy { kind, hit: k.hits, active: true, x: xc, xbr: xc + w - 1, y: yc, ybr: yc + h - 1, sx: xc * 64, sy: yc * 64, ..Enemy::default() };
        let gs = self.gamespeed;
        if !k.maxspeed {
            if k.speed > 0 {
                e.delx = (self.rng.range(k.speed) - k.speed2) * gs / 64;
                e.dely = (self.rng.range(k.speed) - k.speed2) * gs / 64;
            }
        } else {
            let t = (self.rng.float() as f64 * 2.0 * PI - PI).cos();
            e.delx = (k.speed as f64 * t).round() as i32 * gs / 64;
            e.dely = (k.speed as f64 * (1.0 - t * t).sqrt()).round() as i32 * gs / 64;
        }
        if k.curve > 0 {
            e.curvesin = -k.curve2 + self.rng.range(k.curve);
            e.curvecos = curvecos(e.curvesin);
        } else {
            e.curvesin = 0;
            e.curvecos = 32767;
        }
        e.supertime = match kind {
            1 => k.numframes * (256 / k.framespeed),
            0 => 335 + self.rng.range(335),
            _ => 0,
        };
        self.enemies.push(e);
    }

    fn fire_missile_raw(&mut self, a: &Assets, dx: i32, dy: i32) {
        if self.missiles.len() >= MAX_MISSILES {
            return;
        }
        let (mw, mh) = (a.player_missile.w, a.player_missile.h);
        let sx = self.ship.sx + (self.ship.w << 5) - (mw << 4);
        let sy = self.ship.sy + (self.ship.h << 5) - (mh << 4);
        let (x, y) = (sx / 64, sy / 64);
        self.missiles.push(Missile { sx, sy, x, y, xbr: x + mw - 1, ybr: y + mh - 1, delx: dx, dely: dy, time: 0, active: true });
    }

    fn shoot(&mut self, a: &Assets) {
        let mut dx = self.ship.delx * 2;
        let mut dy = self.ship.dely * 2;

        if self.powerup_timer[PU_AIMED] > 0 && !self.enemies.is_empty() {
            const AIMED_SPEED: i64 = 256;
            let mut mindist2: i64 = 0x7FFF_FFFF;
            let mut nearest = None;
            for (i, e) in self.enemies.iter().enumerate() {
                if !e.active || e.kind == 1 {
                    continue;
                }
                let ddx = (self.ship.x - e.x) as i64;
                let ddy = (self.ship.y - e.y) as i64;
                let d2 = ddx * ddx + ddy * ddy;
                if d2 < mindist2 {
                    mindist2 = d2;
                    nearest = Some(i);
                }
            }
            if let Some(n) = nearest
                && mindist2 > 0
            {
                let md = ((mindist2 as f64).sqrt() as i64).max(1);
                let fd = (md / (AIMED_SPEED / 64)) as i32;
                let e = self.enemies[n];
                let ex = e.x + (fd * e.delx) / 64;
                let ey = e.y + (fd * e.dely) / 64;
                dx = (AIMED_SPEED * (ex - self.ship.x) as i64 / md) as i32;
                dy = (AIMED_SPEED * (ey - self.ship.y) as i64 / md) as i32;
            }
        }

        self.play(if self.powerup_timer[PU_MULTI] > 0 { SND_FIRE4 } else { SND_FIRE });
        self.fire_missile_raw(a, dx, dy);
        if self.powerup_timer[PU_ASS] > 0 {
            self.fire_missile_raw(a, -dx, -dy);
        }
        if self.powerup_timer[PU_MULTI] > 0 {
            let r = ((dx as f64) * (dx as f64) + (dy as f64) * (dy as f64)).sqrt();
            let theta = (dy as f64).atan2(dx as f64);
            let delta = PI / 180.0 * 10.0;
            let (ax, ay) = ((r * (theta + delta).cos()) as i32, (r * (theta + delta).sin()) as i32);
            let (bx, by) = ((r * (theta - delta).cos()) as i32, (r * (theta - delta).sin()) as i32);
            self.fire_missile_raw(a, ax, ay);
            self.fire_missile_raw(a, bx, by);
            if self.powerup_timer[PU_ASS] > 0 {
                self.fire_missile_raw(a, -ax, -ay);
                self.fire_missile_raw(a, -bx, -by);
            }
        }
    }

    fn missiles_tick(&mut self, a: &Assets) {
        let (mw, mh) = (a.player_missile.w, a.player_missile.h);
        let bounce = self.powerup_timer[PU_BOUNCE] > 0;
        for m in &mut self.missiles {
            if !m.active {
                continue;
            }
            m.sx += m.delx;
            m.sy += m.dely;
            m.x = m.sx / 64;
            m.y = m.sy / 64;
            m.xbr = m.x + mw - 1;
            m.ybr = m.y + mh - 1;
            m.time += 1;
            if bounce {
                let mut hit = false;
                if m.x < 10 {
                    m.delx = m.delx.abs();
                    m.sx = 10 * 64;
                    m.x = 10;
                    hit = true;
                }
                if m.xbr > WORLD_W - 10 {
                    m.delx = -m.delx.abs();
                    m.x = WORLD_W - 10 - mw;
                    m.sx = m.x * 64;
                    m.xbr = m.x + mw - 1;
                    hit = true;
                }
                if m.y < 10 {
                    m.dely = m.dely.abs();
                    m.sy = 10 * 64;
                    m.y = 10;
                    hit = true;
                }
                if m.ybr > WORLD_H - 10 {
                    m.dely = -m.dely.abs();
                    m.y = WORLD_H - 10 - mh;
                    m.sy = m.y * 64;
                    m.ybr = m.y + mh - 1;
                    hit = true;
                }
                if hit {
                    self.sounds.push(SND_BOING);
                }
            } else if m.x < 10 || m.xbr > WORLD_W - 10 || m.y < 10 || m.ybr > WORLD_H - 10 {
                m.active = false;
            }
            if m.time > MISSILE_LIFE {
                m.active = false;
            }
        }
        compact(&mut self.missiles, |m| m.active);
    }

    fn fire_emissile(&mut self, a: &Assets, ei: usize) {
        if self.emissiles.len() >= MAX_ENEMY_MISSILES {
            return;
        }
        let e = self.enemies[ei];
        let ftype = kinds(e.kind).firetype;
        if !(1..=6).contains(&ftype) {
            return;
        }
        let ft = ftype as usize;
        let (ew, eh) = (a.enemy[e.kind][0].w, a.enemy[e.kind][0].h);
        let (mw, mh) = (a.enemy_missile[ft - 1].w, a.enemy_missile[ft - 1].h);
        let gs = self.gamespeed;
        let spd = EM_SPEED[ft];
        let sx = e.sx + (ew << 5) - (mw << 4);
        let sy = e.sy + (eh << 5) - (mh << 4);
        let (x, y) = (sx / 64, sy / 64);
        let mut em = EnemyMissile { firetype: ftype, active: true, sx, sy, x, y, xbr: x + mw - 1, ybr: y + mh - 1, ..EnemyMissile::default() };
        if EM_DIRECT[ft] {
            let dx = self.ship.x + self.ship.w / 2 - em.x;
            let dy = self.ship.y + self.ship.h / 2 - em.y;
            let mag = ((dx as f64) * (dx as f64) + (dy as f64) * (dy as f64)).sqrt().max(1.0);
            em.delx = (dx as f64 / mag * spd as f64) as i32 * gs / 64;
            em.dely = (dy as f64 / mag * spd as f64) as i32 * gs / 64;
        } else {
            em.delx = (self.rng.range(spd) - spd / 2) * gs / 64;
            em.dely = (self.rng.range(spd) - spd / 2) * gs / 64;
        }
        self.emissiles.push(em);
    }

    fn add_enemy_mine(&mut self, a: &Assets, idx: usize) {
        if self.objects.len() >= MAX_OBJECTS {
            return;
        }
        let e = self.enemies[idx];
        let ew = a.enemy[e.kind][0].w;
        let (mw, mh) = (a.enemy_mine.w, a.enemy_mine.h);
        // the C port centres both ways on the width, kept as it is
        let (x, y) = (e.x + ew / 2, e.y + ew / 2);
        self.objects.push(Object { x, y, xbr: x + mw - 1, ybr: y + mh - 1, kind: ObjType::EnemyMine, active: true });
    }

    fn emissiles_tick(&mut self, a: &Assets) {
        for em in &mut self.emissiles {
            if !em.active {
                continue;
            }
            let ft = em.firetype as usize;
            let (mw, mh) = (a.enemy_missile[ft - 1].w, a.enemy_missile[ft - 1].h);
            let rb = EM_REBOUND[ft];
            em.sx += em.delx;
            em.sy += em.dely;
            em.x = em.sx / 64;
            em.y = em.sy / 64;
            em.xbr = em.x + mw - 1;
            em.ybr = em.y + mh - 1;
            if em.x < ENEMY_X_MIN {
                if rb {
                    em.delx = em.delx.abs();
                    em.x = ENEMY_X_MIN;
                    em.sx = em.x * 64;
                } else {
                    em.active = false;
                    continue;
                }
            } else if em.xbr > ENEMY_X_MAX {
                if rb {
                    em.delx = -em.delx.abs();
                    em.x = ENEMY_X_MAX - mw;
                    em.sx = em.x * 64;
                } else {
                    em.active = false;
                    continue;
                }
            }
            if em.y < ENEMY_Y_MIN {
                if rb {
                    em.dely = em.dely.abs();
                    em.y = ENEMY_Y_MIN;
                    em.sy = em.y * 64;
                } else {
                    em.active = false;
                    continue;
                }
            } else if em.ybr > ENEMY_Y_MAX {
                if rb {
                    em.dely = -em.dely.abs();
                    em.y = ENEMY_Y_MAX - mh;
                    em.sy = em.y * 64;
                } else {
                    em.active = false;
                    continue;
                }
            }
        }
        compact(&mut self.emissiles, |m| m.active);
    }

    fn entities_tick(&mut self, a: &Assets) {
        self.frame_count += 1;
        let rec = self.level_rec();
        let diff = &DIFFS[self.diff_level];
        let gs = self.gamespeed;
        let li = (self.level - 1) as usize;

        for i in 0..ENEMY_KINDS {
            if self.enemies.len() as i32 >= rec.maxenemies {
                break;
            }
            if self.rng.float() >= rec.erelease * diff.enemyfrequency {
                continue;
            }
            if self.rng.range(100) >= PROBS[li][i] as i32 {
                continue;
            }
            if self.rng.range(2) == 1 && self.enemy_entering_left == 0 {
                self.enemy_entering_left = ENEMY_ENTER_TICKS;
                self.enemy_left_type = i;
            } else if self.enemy_entering_right == 0 {
                self.enemy_entering_right = ENEMY_ENTER_TICKS;
                self.enemy_right_type = i;
            }
        }

        if self.enemy_entering_left > 0 {
            self.enemy_entering_left -= 1;
            if self.enemy_entering_left == 0 {
                self.play(SND_ENEMYENT);
                self.add_enemy(a, 15, ENEMY_START_Y, self.enemy_left_type);
            }
        }
        if self.enemy_entering_right > 0 {
            self.enemy_entering_right -= 1;
            if self.enemy_entering_right == 0 {
                let kind = self.enemy_right_type;
                let w = a.enemy[kind][0].w;
                self.play(SND_ENEMYENT);
                self.add_enemy(a, WORLD_W - w - 16, ENEMY_START_Y, kind);
            }
        }

        let mut i = 0;
        while i < self.enemies.len() {
            if !self.enemies[i].active {
                i += 1;
                continue;
            }
            let k = kinds(self.enemies[i].kind);
            let (w, h) = (a.enemy[self.enemies[i].kind][0].w, a.enemy[self.enemies[i].kind][0].h);
            let ship = self.ship;
            let rng = &mut self.rng;
            let e = &mut self.enemies[i];

            e.frame += k.framespeed;
            if e.frame >= (k.numframes + 1) << 8 {
                e.frame = 0;
            }

            if k.follows && rng.float() < k.follow {
                let dx = ship.x - e.x;
                let dy = ship.y - e.y;
                let mag = dx.abs() + dy.abs();
                if mag > 0 {
                    e.delx = (dx as i64 * k.speed as i64 / mag as i64) as i32 * gs / 64;
                    e.dely = (dy as i64 * k.speed as i64 / mag as i64) as i32 * gs / 64;
                }
            } else if rng.float() < k.changedir {
                if k.zoom && (ship.delx.abs() + ship.dely.abs()) < 60 {
                    let dx = ship.x - e.x;
                    let dy = ship.y - e.y;
                    let mag = dx.abs() + dy.abs();
                    if mag > 0 {
                        e.delx = (dx as i64 * k.speed as i64 / mag as i64) as i32 * gs / 48;
                        e.dely = (dy as i64 * k.speed as i64 / mag as i64) as i32 * gs / 48;
                    }
                } else if !k.maxspeed {
                    e.delx = (rng.range(k.speed) - k.speed2) * gs / 64;
                    e.dely = (rng.range(k.speed) - k.speed2) * gs / 64;
                } else {
                    let t = (rng.float() as f64 * 2.0 * PI - PI).cos();
                    e.delx = (k.speed as f64 * t).round() as i32 * gs / 64;
                    e.dely = (k.speed as f64 * (1.0 - t * t).sqrt()).round() as i32 * gs / 64;
                }
            }

            if k.curves {
                if rng.float() < k.changecurve {
                    e.curvesin = -k.curve2 + rng.range(k.curve);
                    e.curvecos = curvecos(e.curvesin);
                }
                let old_dx = e.delx as i64;
                let lt = e.delx as i64 * e.curvecos as i64 - e.dely as i64 * e.curvesin as i64;
                e.delx = round_div(lt);
                let lt = e.dely as i64 * e.curvecos as i64 + old_dx * e.curvesin as i64;
                e.dely = round_div(lt);
            }

            e.sx += e.delx;
            e.sy += e.dely;
            e.x = e.sx / 64;
            e.y = e.sy / 64;
            e.xbr = e.x + w - 1;
            e.ybr = e.y + h - 1;

            if e.x < ENEMY_X_MIN {
                e.x = ENEMY_X_MIN;
                e.sx = e.x * 64;
                e.delx = e.delx.abs();
            }
            if e.xbr > ENEMY_X_MAX {
                e.x = ENEMY_X_MAX - w;
                e.sx = e.x * 64;
                e.delx = -e.delx.abs();
            }
            if e.y < ENEMY_Y_MIN {
                e.y = ENEMY_Y_MIN;
                e.sy = e.y * 64;
                e.dely = e.dely.abs();
            }
            if e.ybr > ENEMY_Y_MAX {
                e.y = ENEMY_Y_MAX - h;
                e.sy = e.y * 64;
                e.dely = -e.dely.abs();
            }
            e.xbr = e.x + w - 1;
            e.ybr = e.y + h - 1;

            // keep away from the enemy inlets in the side walls
            let mid = WORLD_H / 2;
            if e.y < mid + 7 && e.ybr > mid - 7 {
                if e.x < ENEMY_X_MIN + 7 {
                    e.delx = e.delx.abs();
                }
                if e.xbr > ENEMY_X_MAX - 7 {
                    e.delx = -e.delx.abs();
                }
            }

            if k.fires && self.rng.float() < k.fireprob {
                self.fire_emissile(a, i);
            }
            if k.laysmines && self.rng.float() < k.fireprob {
                self.add_enemy_mine(a, i);
            }

            if k.repulses {
                let e = self.enemies[i];
                let dx = (self.ship.x - e.x) as i64;
                let dy = (self.ship.y - e.y) as i64;
                if dx != 0 || dy != 0 {
                    let m = if dx.abs() > dy.abs() { (dx * dx + dy * dy) * (dx.abs() + dy.abs() / 2) } else { (dx * dx + dy * dy) * (dy.abs() + dx.abs() / 2) };
                    if m > 0 {
                        if m < 500_000 && (self.frame_count & 31) == 0 {
                            self.play(SND_REPULSE);
                        }
                        self.ship.delx += (8192 * dx / m) as i32;
                        self.ship.dely += (8192 * dy / m) as i32;
                    }
                }
            }

            let e = &mut self.enemies[i];
            if e.supertime > 0 {
                e.supertime -= 1;
                if e.supertime == 0 {
                    e.active = false;
                }
            }
            i += 1;
        }
        compact(&mut self.enemies, |e| e.active);
    }

    fn ship_destroy(&mut self, a: &Assets) {
        self.play(SND_EXPLOSN);
        let (ew, eh) = (a.enemy[1][0].w, a.enemy[1][0].h);
        let cx = self.ship.x + self.ship.w / 2 - ew / 2;
        let cy = self.ship.y + self.ship.h / 2 - eh / 2;
        const OX: [i32; 5] = [0, -8, 8, 0, 0];
        const OY: [i32; 5] = [0, 0, 0, -8, 8];
        for i in 0..5 {
            self.add_enemy(a, cx + OX[i], cy + OY[i], 1);
        }
        self.ship_explode_timer = SHIP_EXPLODE_TICKS;
    }

    /// A Cluster bursts into up to 15 missiles round the circle.
    fn explode_enemy(&mut self, a: &Assets, idx: usize) {
        let e = self.enemies[idx];
        let ftype = kinds(e.kind).firetype;
        if !(1..=6).contains(&ftype) {
            return;
        }
        let ft = ftype as usize;
        let (ew, eh) = (a.enemy[e.kind][0].w, a.enemy[e.kind][0].h);
        let (mw, mh) = (a.enemy_missile[ft - 1].w, a.enemy_missile[ft - 1].h);
        let spd = EM_SPEED[ft] * self.gamespeed / 64;
        let cx = e.sx + (ew << 5) - (mw << 4);
        let cy = e.sy + (eh << 5) - (mh << 4);
        let count = (MAX_ENEMY_MISSILES - self.emissiles.len()).min(15);
        for i in 0..count {
            if self.emissiles.len() >= MAX_ENEMY_MISSILES {
                break;
            }
            let theta = 2.0 * PI * i as f64 / count as f64;
            let (x, y) = (cx / 64, cy / 64);
            self.emissiles.push(EnemyMissile {
                firetype: ftype,
                active: true,
                sx: cx,
                sy: cy,
                x,
                y,
                xbr: x + mw - 1,
                ybr: y + mh - 1,
                delx: (theta.cos() * spd as f64) as i32,
                dely: (theta.sin() * spd as f64) as i32,
            });
        }
    }

    fn enemy_hit(&mut self, a: &Assets, idx: usize, dx: i32, dy: i32) {
        if self.powerup_timer[PU_HEAVY] > 0 {
            self.enemies[idx].hit = 0;
        }
        self.enemies[idx].hit -= 1;
        if self.enemies[idx].hit > 0 {
            return;
        }
        let e = self.enemies[idx];
        let (ex, ey, kind) = (e.x, e.y, e.kind);
        let k = kinds(kind);

        if k.rebounds && self.powerup_timer[PU_HEAVY] == 0 {
            let e = &mut self.enemies[idx];
            e.delx = (3 * e.delx + dx) / 4;
            e.dely = (3 * e.dely + dy) / 4;
            e.hit = 1;
            return;
        }

        self.add_score(k.score as i64);
        if k.death_sound == SND_EXPLOSN {
            let v = self.rng.range(3) as u8;
            self.play(SND_EXPLOSN + v);
        } else if k.death_sound > 0 {
            self.play(k.death_sound);
        }
        if k.shootback {
            self.fire_emissile(a, idx);
        }
        if k.explodes {
            self.explode_enemy(a, idx);
        }
        if k.tribbles {
            for _ in 0..5 {
                self.add_enemy(a, ex, ey, kind + 1);
            }
        }
        self.enemies[idx].active = false;
        if kind > 1 {
            self.add_enemy(a, ex, ey, 1);
        }
    }

    fn enemy_sprite<'a>(a: &'a Assets, e: &Enemy) -> &'a Sprite {
        &a.enemy[e.kind][(e.frame >> 8) as usize]
    }

    fn check_collisions(&mut self, a: &Assets) {
        let ms = &a.player_missile;
        for mi in 0..self.missiles.len() {
            let m = self.missiles[mi];
            if !m.active {
                continue;
            }
            for j in (0..self.enemies.len()).rev() {
                let e = self.enemies[j];
                if !e.active || e.kind == 1 {
                    continue;
                }
                let es = Self::enemy_sprite(a, &e);
                if m.xbr < e.x || m.x > e.xbr || m.ybr < e.y || m.y > e.ybr {
                    continue;
                }
                if sprites_collide(ms, m.x, m.y, es, e.x, e.y) {
                    self.enemy_hit(a, j, m.delx, m.dely);
                    self.missiles[mi].active = false;
                    break;
                }
            }
        }
        if self.ship_destroyed {
            return;
        }

        let ship_sp = &a.ship[self.ship.dir];
        for j in (0..self.enemies.len()).rev() {
            let e = self.enemies[j];
            if !e.active || e.kind == 1 {
                continue;
            }
            let es = Self::enemy_sprite(a, &e);
            let s = self.ship;
            if s.xbr < e.x || s.x > e.xbr || s.ybr < e.y || s.y > e.ybr {
                continue;
            }
            if !sprites_collide(ship_sp, s.x, s.y, es, e.x, e.y) {
                continue;
            }
            if e.kind == 0 {
                self.enemies[j].active = false;
                self.supercrystal();
            } else {
                self.enemy_hit(a, j, s.delx, s.dely);
                self.ship_destroyed = self.powerup_timer[PU_SHIELD] <= 0;
            }
            break;
        }
        if self.ship_destroyed {
            return;
        }

        for em in &mut self.emissiles {
            if !em.active {
                continue;
            }
            let ems = &a.enemy_missile[em.firetype as usize - 1];
            let s = self.ship;
            if s.xbr < em.x || s.x > em.xbr || s.ybr < em.y || s.y > em.ybr {
                continue;
            }
            if sprites_collide(ship_sp, s.x, s.y, ems, em.x, em.y) {
                em.active = false;
                if self.powerup_timer[PU_SHIELD] <= 0 {
                    self.ship_destroyed = true;
                }
                break;
            }
        }
    }

    /// A Supercrystal caught: a random power-up (repeat..until SpecialGiven).
    fn supercrystal(&mut self) {
        const TIME_MIN: [i32; PU_COUNT] = [PU_TIMEMIN_SHIELD, PU_TIMEMIN_AIMED, PU_TIMEMIN_LONG, PU_TIMEMIN_LONG, PU_TIMEMIN_LONG, PU_TIMEMIN_LONG, PU_TIMEMIN_BOUNCE];
        const TIME_RAN: [i32; PU_COUNT] = [PU_TIMRAN_SHIELD, PU_TIMRAN_AIMED, PU_TIMRAN_LONG, PU_TIMRAN_LONG, PU_TIMRAN_LONG, PU_TIMRAN_LONG, PU_TIMRAN_BOUNCE];
        // random(22) to a power-up; -1 clears the mines, -2 is the gate
        // freeze the C port leaves out
        #[rustfmt::skip]
        const ROLL: [i32; 22] = [
            PU_RAPID as i32, PU_RAPID as i32, PU_RAPID as i32,
            PU_MULTI as i32, PU_MULTI as i32, PU_MULTI as i32,
            PU_HEAVY as i32, PU_HEAVY as i32, PU_HEAVY as i32,
            PU_ASS as i32, PU_ASS as i32, PU_ASS as i32,
            PU_AIMED as i32, PU_AIMED as i32,
            PU_BOUNCE as i32, PU_BOUNCE as i32,
            -1, -1,
            PU_SHIELD as i32,
            -2, -2, -2,
        ];
        for _ in 0..100 {
            let pu = ROLL[(self.rng.next() % 22) as usize];
            if pu >= 0 {
                let pu = pu as usize;
                if pu == PU_SHIELD || self.powerup_timer[pu] <= 0 {
                    self.powerup_timer[pu] += (self.rng.next() % TIME_RAN[pu] as u32) as i32 + TIME_MIN[pu];
                    break;
                }
            } else if pu == -1 {
                if self.powerup_timer[PU_SHIELD] == 0 {
                    self.powerup_timer[PU_SHIELD] = 1;
                }
                for o in &mut self.objects {
                    if o.active && o.kind == ObjType::Mine {
                        o.active = false;
                    }
                }
                break;
            } else {
                break;
            }
        }
    }
}

fn curvecos(curvesin: i32) -> i32 {
    let s = curvesin as f64 / 32767.0;
    ((1.0 - s * s).sqrt() * 32767.0).round() as i32
}

fn round_div(lt: i64) -> i32 {
    (if lt > 0 { (lt + 16384) / 32767 } else { (lt - 16384) / 32767 }) as i32
}

/// The C port's compaction: walking down, each dead entry takes the last.
fn compact<T: Copy>(v: &mut Vec<T>, alive: impl Fn(&T) -> bool) {
    for i in (0..v.len()).rev() {
        if !alive(&v[i]) {
            v.swap_remove(i);
        }
    }
}

/// Pixel-accurate test of two masks, as the original's assembler does it.
fn collide_bitmaps(m1: &[u32], h1: i32, x1: i32, y1: i32, m2: &[u32], h2: i32, x2: i32, y2: i32) -> bool {
    if m1.is_empty() || m2.is_empty() {
        return false;
    }
    let shift = x2 - x1;
    if !(0..32).contains(&shift) {
        return false;
    }
    let ys = y1.max(y2);
    let ye = (y1 + h1).min(y2 + h2);
    (ys..ye).any(|y| (m1[(y - y1) as usize] << shift) & m2[(y - y2) as usize] != 0)
}

fn sprites_collide(s1: &Sprite, x1: i32, y1: i32, s2: &Sprite, x2: i32, y2: i32) -> bool {
    if x1 <= x2 { collide_bitmaps(&s1.mask, s1.h, x1, y1, &s2.mask, s2.h, x2, y2) } else { collide_bitmaps(&s2.mask, s2.h, x2, y2, &s1.mask, s1.h, x1, y1) }
}

// The tables below are generated from xquest-sdl's game.c and entities.c,
// which copy them from xqvars.pas and xqenter.pas.

#[rustfmt::skip]
pub const LEVELS: [Level; MAX_LEVELS] = [
    lv(15, 0, 1, 0.2, 15000, 20, 0.005, 27, 0, 0.0, 20, false),
    lv(16, 3, 1, 0.2, 15000, 4, 0.005, 27, 0, 0.0, 20, false),
    lv(17, 4, 1, 0.2, 15000, 5, 0.005, 26, 0, 0.0, 20, false),
    lv(18, 5, 1, 0.2, 15000, 6, 0.005, 26, 0, 0.0, 25, false),
    lv(19, 6, 1, 0.2, 15000, 7, 0.005, 25, 0, 0.0, 25, false),
    lv(20, 6, 1, 0.2, 15000, 8, 0.005, 25, 0, 0.0, 30, false),
    lv(21, 7, 1, 0.2, 15000, 9, 0.005, 24, 0, 0.0, 30, false),
    lv(22, 7, 1, 0.2, 20000, 10, 0.006, 24, 0, 0.0, 35, false),
    lv(23, 8, 1, 0.2, 20000, 10, 0.006, 23, 0, 0.0, 35, false),
    lv(24, 8, 1, 0.2, 20000, 10, 0.006, 23, 0, 0.0, 40, false),
    lv(24, 9, 2, 0.2, 20000, 10, 0.006, 22, 0, 0.0, 40, false),
    lv(25, 9, 2, 0.2, 20000, 10, 0.007, 22, 0, 0.0, 45, false),
    lv(25, 10, 2, 0.2, 40000, 10, 0.007, 21, 0, 0.0, 45, false),
    lv(26, 10, 2, 0.2, 40000, 10, 0.008, 21, 0, 0.0, 45, false),
    lv(26, 10, 2, 0.2, 40000, 10, 0.008, 20, 0, 0.0, 50, false),
    lv(27, 11, 2, 0.2, 40000, 10, 0.009, 20, 0, 0.0, 50, false),
    lv(27, 11, 2, 0.2, 40000, 10, 0.009, 19, 0, 0.0, 50, false),
    lv(28, 11, 2, 0.2, 40000, 10, 0.010, 19, 0, 0.0, 55, false),
    lv(28, 12, 2, 0.2, 40000, 10, 0.010, 18, 0, 0.0, 55, false),
    lv(29, 12, 2, 0.3, 40000, 10, 0.010, 18, 0, 0.0, 55, false),
    lv(29, 12, 2, 0.3, 70000, 10, 0.010, 17, 0, 0.0, 60, false),
    lv(30, 13, 2, 0.3, 70000, 10, 0.010, 17, 0, 0.0, 60, false),
    lv(30, 13, 2, 0.3, 70000, 10, 0.010, 17, 0, 0.0, 60, false),
    lv(31, 13, 2, 0.3, 70000, 10, 0.010, 17, 0, 0.0, 60, false),
    lv(31, 13, 2, 0.3, 70000, 10, 0.010, 17, 0, 0.0, 65, false),
    lv(32, 14, 2, 0.3, 70000, 10, 0.010, 17, 0, 0.0, 65, false),
    lv(32, 14, 2, 0.3, 70000, 11, 0.010, 17, 0, 0.0, 65, false),
    lv(33, 14, 2, 0.3, 70000, 11, 0.010, 17, 0, 0.0, 65, false),
    lv(33, 14, 2, 0.3, 70000, 12, 0.010, 17, 0, 0.0, 70, false),
    lv(34, 15, 2, 0.3, 70000, 12, 0.010, 17, 0, 0.0, 70, false),
    lv(34, 15, 2, 0.3, 70000, 13, 0.010, 17, 0, 0.0, 70, false),
    lv(35, 15, 2, 0.3, 70000, 13, 0.010, 17, 0, 0.0, 70, false),
    lv(35, 15, 2, 0.3, 100000, 14, 0.010, 17, 10, 0.000, 75, false),
    lv(36, 16, 2, 0.3, 100000, 14, 0.010, 17, 20, 0.000, 75, false),
    lv(36, 16, 2, 0.3, 100000, 15, 0.010, 17, 30, 0.000, 75, false),
    lv(37, 16, 2, 0.3, 100000, 15, 0.010, 17, 40, 0.000, 75, false),
    lv(37, 16, 2, 0.3, 100000, 16, 0.025, 17, 50, 0.000, 80, false),
    lv(38, 17, 2, 0.3, 100000, 16, 0.011, 17, 60, 0.000, 80, false),
    lv(38, 17, 2, 0.3, 100000, 17, 0.012, 17, 70, 0.000, 80, false),
    lv(39, 17, 2, 0.3, 100000, 17, 0.012, 17, 80, 0.000, 80, false),
    lv(39, 17, 2, 0.3, 100000, 18, 0.013, 17, 90, 0.002, 85, false),
    lv(40, 18, 2, 0.3, 100000, 18, 0.013, 17, 100, 0.004, 85, false),
    lv(40, 18, 2, 0.3, 100000, 19, 0.014, 17, 100, 0.006, 85, false),
    lv(40, 18, 2, 0.3, 100000, 19, 0.014, 17, 100, 0.008, 85, false),
    lv(40, 18, 2, 0.3, 100000, 20, 0.015, 17, 100, 0.010, 90, false),
    lv(40, 19, 2, 0.3, 100000, 20, 0.016, 17, 100, 0.012, 90, false),
    lv(40, 19, 2, 0.3, 100000, 20, 0.016, 17, 100, 0.014, 90, false),
    lv(40, 19, 2, 0.3, 100000, 20, 0.017, 17, 100, 0.016, 90, false),
    lv(40, 19, 2, 0.3, 100000, 20, 0.017, 17, 100, 0.018, 90, false),
    lv(40, 20, 2, 0.3, 100000, 20, 0.018, 17, 100, 0.020, 90, false),
];

#[rustfmt::skip]
pub const KINDS: [Kind; ENEMY_KINDS] = [
    kind(301, 150, 0, 0, 1, 0, 0, false, false, false, false, false, false, false, true, false, false, false, 0.0, 0.002, 0.0, 0.0, 5, 70, 19),
    kind(0, 0, 0, 0, 1, 0, 0, false, false, false, false, false, false, false, false, false, false, false, 0.0, 0.0, 0.0, 0.0, 5, 64, 0),
    kind(121, 60, 0, 0, 1, 0, 200, false, false, false, false, false, false, false, false, false, false, false, 0.0, 0.006, 0.0, 0.0, 3, 40, 12),
    kind(281, 140, 6000, 3000, 1, 0, 300, false, false, true, false, false, false, false, false, false, false, false, 0.0, 0.003, 0.02, 0.0, 3, 56, 12),
    kind(101, 50, 0, 0, 1, 1, 300, true, false, false, false, false, false, false, false, false, false, false, 0.01, 0.006, 0.0, 0.0, 3, 60, 12),
    kind(201, 100, 0, 0, 1, 0, 500, false, false, false, false, false, false, false, false, true, false, false, 0.0, 0.003, 0.0, 0.0, 3, 56, 12),
    kind(0, 0, 0, 0, 300, 0, 500, false, false, false, false, false, false, false, false, false, false, false, 0.0, 0.0, 0.0, 0.0, 0, 32767, 12),
    kind(121, 60, 4000, 2000, 1, 0, 600, false, false, true, false, true, false, false, false, false, false, false, 0.008, 0.006, 0.1, 0.0, 3, 32, 12),
    kind(81, 40, 0, 0, 5, 0, 2000, false, true, false, false, false, false, false, false, false, false, false, 0.0, 0.006, 0.0, 0.01, 5, 28, 12),
    kind(121, 60, 0, 0, 1, 3, 1000, false, false, false, false, false, true, false, false, false, false, false, 0.0, 0.006, 0.0, 0.0, 3, 64, 0),
    kind(121, 60, 0, 0, 1, 0, 1000, false, false, false, false, false, false, true, false, false, false, false, 0.0, 0.02, 0.0, 0.0, 3, 120, 12),
    kind(121, 60, 0, 0, 1, 4, 1000, true, false, false, false, false, false, false, false, false, false, false, 0.005, 0.003, 0.0, 0.0, 3, 128, 12),
    kind(131, 65, 0, 0, 1, 5, 1250, true, false, false, false, false, false, false, false, false, false, false, 0.004, 0.004, 0.0, 0.0, 3, 26, 12),
    kind(100, 50, 0, 0, 1, 0, 1500, false, false, false, false, false, false, false, false, false, true, false, 0.0, 0.01, 0.0, 0.0, 3, 32, 12),
    kind(220, 110, 1000, 500, 1, 0, 500, false, false, true, false, false, false, false, false, false, false, false, 0.0, 0.005, 0.1, 0.0, 3, 48, 12),
    kind(101, 50, 0, 0, 1, 2, 1500, true, false, false, false, false, false, false, false, false, false, false, 0.03, 0.006, 0.0, 0.0, 3, 36, 12),
    kind(81, 40, 0, 0, 1, 6, 5000, false, false, false, true, false, false, false, false, false, false, false, 0.0, 0.02, 0.0, 0.0, 3, 32, 15),
    kind(101, 50, 0, 0, 1, 0, 2000, false, true, false, false, false, false, false, false, false, false, false, 0.0, 0.0, 0.0, 1.0, 3, 40, 12),
    kind(141, 70, 0, 0, 1, 0, 7500, false, true, false, false, false, false, false, false, false, false, true, 0.0, 0.01, 0.0, 0.01, 5, 60, 12),
];

#[rustfmt::skip]
pub const PROBS: [[u8; ENEMY_KINDS]; MAX_LEVELS] = [
    [5, 0, 60, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [5, 0, 100, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [5, 0, 0, 100, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [5, 0, 15, 85, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [7, 0, 0, 0, 100, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [7, 0, 15, 15, 70, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [7, 0, 0, 0, 0, 100, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [7, 0, 15, 15, 15, 55, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [7, 0, 0, 0, 0, 0, 0, 100, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [7, 0, 15, 15, 15, 15, 0, 50, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [7, 0, 0, 0, 0, 0, 0, 0, 100, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [7, 0, 10, 10, 10, 10, 0, 10, 60, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [7, 0, 0, 0, 0, 0, 0, 0, 0, 100, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [7, 0, 10, 10, 10, 10, 0, 10, 3, 60, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [7, 0, 0, 0, 0, 0, 0, 0, 0, 0, 100, 0, 0, 0, 0, 0, 0, 0, 0],
    [7, 0, 10, 10, 10, 10, 0, 10, 3, 3, 60, 0, 0, 0, 0, 0, 0, 0, 0],
    [7, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 100, 0, 0, 0, 0, 0, 0, 0],
    [7, 0, 10, 10, 10, 10, 0, 10, 10, 3, 3, 60, 0, 0, 0, 0, 0, 0, 0],
    [7, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 100, 0, 0, 0, 0, 0, 0],
    [10, 0, 10, 10, 10, 10, 0, 10, 10, 5, 3, 3, 60, 0, 0, 0, 0, 0, 0],
    [10, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 100, 0, 0, 0, 0, 0],
    [10, 0, 10, 10, 10, 10, 0, 10, 10, 10, 5, 3, 3, 60, 0, 0, 0, 0, 0],
    [10, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 100, 0, 0, 0],
    [10, 0, 10, 10, 10, 10, 0, 10, 10, 10, 5, 5, 3, 3, 0, 60, 0, 0, 0],
    [10, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 100, 0, 0],
    [10, 0, 10, 10, 10, 10, 0, 10, 10, 10, 10, 5, 5, 3, 0, 3, 60, 0, 0],
    [10, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 100, 0],
    [10, 0, 10, 10, 10, 10, 0, 10, 10, 10, 10, 5, 5, 5, 0, 3, 3, 60, 0],
    [10, 0, 10, 10, 10, 10, 0, 10, 10, 10, 10, 10, 5, 5, 0, 3, 3, 50, 0],
    [10, 0, 10, 10, 10, 10, 0, 10, 10, 10, 10, 10, 10, 10, 0, 5, 5, 40, 0],
    [10, 0, 10, 10, 10, 10, 0, 10, 10, 10, 10, 10, 10, 10, 0, 10, 10, 30, 0],
    [10, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 100],
    [10, 0, 10, 10, 10, 10, 0, 10, 10, 10, 10, 5, 5, 5, 0, 3, 3, 3, 60],
    [10, 0, 10, 10, 10, 10, 0, 10, 10, 10, 10, 10, 10, 10, 0, 3, 3, 3, 40],
    [10, 0, 10, 10, 10, 10, 0, 10, 10, 10, 10, 10, 10, 10, 0, 10, 10, 3, 20],
    [10, 0, 10, 10, 10, 10, 0, 10, 10, 10, 10, 10, 10, 10, 0, 10, 10, 10, 10],
    [10, 0, 0, 100, 0, 100, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [10, 0, 10, 10, 10, 10, 0, 10, 10, 10, 10, 10, 10, 10, 0, 10, 10, 10, 10],
    [10, 0, 0, 0, 0, 0, 0, 50, 0, 0, 0, 0, 0, 0, 0, 0, 0, 50, 0],
    [10, 0, 10, 10, 10, 10, 0, 10, 10, 10, 10, 10, 10, 10, 0, 10, 10, 10, 10],
    [10, 0, 0, 0, 0, 0, 0, 0, 50, 0, 0, 0, 0, 50, 0, 0, 0, 0, 0],
    [10, 0, 10, 10, 10, 10, 0, 10, 10, 10, 10, 10, 10, 10, 0, 10, 10, 10, 10],
    [10, 0, 0, 0, 0, 0, 0, 0, 0, 50, 0, 50, 0, 0, 0, 0, 0, 0, 0],
    [10, 0, 10, 10, 10, 10, 0, 10, 10, 10, 10, 10, 10, 10, 0, 10, 10, 10, 10],
    [10, 0, 0, 0, 0, 0, 0, 0, 0, 0, 50, 0, 50, 0, 0, 0, 0, 0, 0],
    [10, 0, 10, 10, 10, 10, 0, 10, 10, 10, 10, 10, 10, 10, 0, 10, 10, 10, 10],
    [10, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 50, 50, 0, 0],
    [10, 0, 10, 10, 10, 10, 0, 10, 10, 10, 10, 10, 10, 10, 0, 10, 10, 10, 10],
    [10, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 50, 50],
    [10, 0, 10, 10, 10, 10, 0, 10, 10, 10, 10, 10, 10, 10, 0, 10, 10, 10, 10],
];
