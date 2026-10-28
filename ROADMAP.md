# Roadmap

AQuest is at 1.0.8 (2026-10-28): the whole game, from Mark Mackey's
original data, frame for frame with xquest-sdl.

## 1.0.8: built

- [x] xquest-sdl's game logic translated line by line; replays give the
  same frames, crystals, points and deaths in both
- [x] Mark's graphics, sounds, fonts and levels, unchanged
- [x] Braille field with line-character walls; Mark's pixel status bar on
  big terminals
- [x] Keyboard, mouse and gamepad

## 1.1.0: as the 1994 game played

The three differences from the original found on 2026-10-20 (see
`CLAUDE.md`). Each is fixed in xquest-sdl first, then carried over here,
with the replay test passing or re-recorded with the reason.

- [ ] Timid: walls bounce the ship, as the original does on Wimp and
  Timid (xquest-sdl only does it on Wimp)
- [ ] Power-ups kept from level to level, cleared only on a lost life
- [ ] After level 50 the game goes round again, faster each time, with the
  "rank" message (it ends there now)
