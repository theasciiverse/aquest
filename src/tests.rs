//! The engine against xquest-sdl's own expectations.

use crate::assets::Assets;
use crate::demo::{self, Demo, replay};

/// xquest-sdl's tests/test_replay.c: the reference demo plays through six
/// levels exactly as it did when it was recorded there.
#[test]
fn reference_demo_replays_as_in_xquest_sdl() {
    let a = Assets::load();
    let d = Demo::parse(demo::BUILT_IN).unwrap();
    let (frames, levels, crystals, score, deaths, over) = replay(&a, &d);
    println!("{frames} frames, {levels} levels, {crystals} crystals, {score} points, {deaths} deaths");
    assert_eq!(frames, 12066);
    assert!(!over);
    assert_eq!(levels, 6);
    assert_eq!(crystals, 162);
    assert_eq!(score, 36000);
    assert_eq!(deaths, 2);
}

#[test]
fn demo_round_trips_byte_for_byte() {
    let d = Demo::parse(demo::BUILT_IN).unwrap();
    assert_eq!(d.bytes(), demo::BUILT_IN);
}
