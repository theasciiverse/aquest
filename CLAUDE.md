# AQuest

Mark Mackey's **XQuest** (DOS shareware, 1994 to 1996, Turbo Pascal) in the
terminal, for the Asciiverse catalog. Rust + Ratatui. Started 2026-10-20.

XQuest has been part of David's history since he played it as shareware in
the mid-90s. He ported it to C and SDL2 first, as **xquest-sdl**
(`/home/david/code/sandbranch/xquest-sdl`, github.com/sandbranch/xquest-sdl),
and AQuest is built on that port.

## Permission

David has Mark Mackey's blessing for his work on XQuest. In David's words
(2026-10-20), asked whether it covers an Asciiverse terminal version with
Mark's graphics, sounds and levels: "We was supper happy for me to work on
it. :-) Were cool". So AQuest uses the original data unchanged. Licence
terms are inherited from the original shareware licence through
xquest-sdl's LICENSE: free, credited to Mark Mackey, clearly unofficial,
never sold (catalog: `LicenseRef-Free-Noncommercial`).

## Source

- Game logic: a line-by-line Rust translation of xquest-sdl's `game.c`,
  `entities.c` and `collision.c` (`src/game.rs`). Same integer and float
  arithmetic, same xorshift32 random generator, same order of random draws,
  same entity compaction. **Do not change gameplay here**: change it in
  xquest-sdl first if it is wrong there, then carry it over, and keep the
  replay test passing (or re-record with the reason).
- Tables (levels, enemy kinds, spawn odds) at the end of `game.rs` are
  generated from the C source by a script, not typed.
- Data in `data/`, from xquest-sdl's `assets/` (Mark's originals) plus
  `startpic.raw` (converted by its `tools/convert_pbm.py`) and its
  `tests/data/reference.dmo`; all compiled in with `include_bytes!`.

sha256 of the copies (2026-10-20):

```
3cb3dbfdf37b5cb7ed2de56ff9a64791cb3baf764bb81b65e3e9527a3efe45ad  reference.dmo
1fa9154cc0d73af07c6ed091d9de36e5b3b0087f589bef127a157ea1cd0671ac  startpic.raw
a3c3dc52b481034a0d645b4054e1009df9cfc403563348cc67610cbb18a0965d  xquest2.fnt
da200840b1b030ad3693c8062069d1da87d1104b5ca8e160d551b3a934470a58  xquest.fnt
35e352b974067d9b8d9941ca3da3f6a79074480358472baec6962c5f1fbf9cfd  xquest.gfx
9d9cf095034a6b864bdc1c4ad7660e2954cac5eb49aa4636a5940289359d5f3a  xquest.snd
```

## Checked against xquest-sdl (2026-10-20)

- `cargo test`: xquest-sdl's `reference.dmo` gives 12,066 frames, 6 levels,
  162 crystals, 36,000 points, 2 deaths, as `test_replay.c` expects.
- Three games recorded in AQuest (Average, Tricky, Inhuman) replayed in a C
  harness built from xquest-sdl's own sources: identical frames, crystals,
  points and deaths (`aquest --verify FILE` prints the same summary line).
- `xquest.scr` written by AQuest reads correctly with xquest-sdl's
  `hiscore.c`.

## Files

| | |
|---|---|
| `src/game.rs` | the game: ship, walls, camera, gate, enemies, missiles, collisions, power-ups |
| `src/assets.rs` | xquest.gfx / .fnt / xquest2.fnt / startpic parsing, palette, collision masks |
| `src/frame.rs` | the original 320x240 screen, drawn as xquest-sdl's render.c draws it |
| `src/view.rs` | that screen fitted to the terminal: Blocks (half-block) or Braille, max-brightness downsampling |
| `src/audio.rs` | the original samples on xquest-sdl's 8-voice mixer, through rodio |
| `src/input.rs` | keyboard (kitty protocol or repeat-timing), mouse, gamepad to thrust/fire/brake/bomb |
| `src/demo.rs` | xquest.dmo read/write, `replay` (used by tests and `--verify`) |
| `src/save.rs` | data folder, xquest.scr Hall of Fame, aquest.cfg settings |
| `src/starfield.rs` | the 3-D starfield |
| `src/main.rs` | menu, play, pause/quit, game over, name entry, Hall of Fame, demos, attract mode |

## Rendering decisions

- The camera scrolls as the original's does (David, 2026-10-20: "Always
  scroll like XQuest"), the 320x217 window scaled to fit with square
  pixels using the terminal's cell aspect.
- Blocks and Braille, switchable with V (David's choice).
- Downsampling keeps the brightest source pixel per terminal pixel, so
  stars and shots survive; the starfield shows fewer stars on small
  pictures (`set_stars`), or 400 stars crowd 5,000 pixels.
- At full size (`tw >= 320`) the original pixel status bar is drawn;
  below that, a text line with the power-ups in their icon colours.
- Menus and tables are terminal text: the comix font (`assets.comix`,
  loaded) is unreadable shrunk. Using it at full size is an open idea.

## Things noticed in xquest-sdl (not changed here)

Reported to David 2026-10-20; AQuest follows xquest-sdl as it is:

- `LevelRecord.newman` (extra ship every N points), `.time` and
  `.bonus_level` are in the level table but nothing reads them, so there
  are no extra lives and no time bonus.
- `g_diff_levels` gives Timid `rebound = true` and the README says Timid
  bounces off walls, but `game_tick` bounces only on Wimp
  (`diff_level == 0`).
- `diff_speed {45, 54, 64, 77, 96}` in main.c does not match
  `round(64 * speedfactor)` from `g_diff_levels` (45, 64, 64, 96, 128),
  which game.h's comment says it should be.
- After a lost life, `game_init` + `level_init` deal the level afresh:
  new crystals and mines, the count back to full.

## Catalog

- id `aquest`, binary `aquest`, kind game, genre arcade;
  `hub/catalog/aquest.toml` and `.md` (2026-10-20).
- Preview: `asciiverse/record-preview.sh` (100x30, the built-in demo,
  levels 1 to 3, Braille from 18 s to 34 s; still at 40 s). About 16 MB:
  the game redraws at 67 frames a second. The header's `term.version`
  ("tmux 3.6") was removed by hand after recording.
- Website mockup: NEW! banner in `dev/mock-titles.toml`, cast in
  `casts/aquest.cast`, local Linux x86_64 build in
  `dev-releases/aquest` (1.0.0-av.1; links libasound and libudev). The
  mockup's `local_binary` looks in `games/<id>`, which this folder's
  capitals miss, hence the packaged release.
- Release workflow `.github/workflows/release.yml`, on ASCII Wars!'
  pattern; not run, no GitHub repo yet.

## Commands

```sh
cargo run --release                     # play
cargo run --release -- --demo           # the built-in demo
cargo test --release                    # replay test and the rest
RUST_LOG=debug ./target/release/aquest 2>aquest.log
```
