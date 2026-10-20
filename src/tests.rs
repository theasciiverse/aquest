//! The engine against xquest-sdl's own expectations.

use crate::assets::Assets;
use crate::demo::{self, Demo, replay};

/// xquest-sdl's tests/test_replay.c: the reference demo replays to exactly
/// the numbers it pins. Since 2026-10-20 (the level clock, time bonus,
/// extra ships, original speeds and lost-life restart from xquest.pas) the
/// old recording meets a faster game and ends in game over 846 frames early.
#[test]
fn reference_demo_replays_as_in_xquest_sdl() {
    let a = Assets::load();
    let d = Demo::parse(demo::BUILT_IN).unwrap();
    let (frames, levels, crystals, score, deaths, over) = replay(&a, &d);
    println!("{frames} frames, {levels} levels, {crystals} crystals, {score} points, {deaths} deaths");
    assert_eq!(frames, 11220);
    assert!(over);
    assert_eq!(levels, 6);
    assert_eq!(crystals, 124);
    assert_eq!(score, 39400);
    assert_eq!(deaths, 5);
}

#[test]
fn demo_round_trips_byte_for_byte() {
    let d = Demo::parse(demo::BUILT_IN).unwrap();
    assert_eq!(d.bytes(), demo::BUILT_IN);
}

#[test]
fn speeds_and_extra_ships_follow_xquest_pas() {
    use crate::game::Game;
    let speeds: Vec<i32> = (0..5).map(Game::base_speed).collect();
    assert_eq!(speeds, [45, 64, 64, 96, 128]);
    let a = Assets::load();
    let mut g = Game::new(&a, 1, 2);
    assert_eq!(g.new_man_score, 15000);
    let lives = g.lives;
    g.add_score(31000);
    // past 15,000 and 30,000: two ships
    assert_eq!(g.lives, lives + 2);
    assert_eq!(g.new_man_score, 45000);
    // under par: 500 a second left
    g.time_on_level = 5 * 67 + 10;
    assert_eq!(g.level_bonus(), (5, (20 - 5) * 500));
    g.time_on_level = 30 * 67;
    assert_eq!(g.level_bonus(), (30, 0));
}
