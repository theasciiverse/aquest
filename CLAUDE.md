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
  After the par-time and extra-ship changes (both games, independently
  written): 11,220 frames, 6 levels, 124 crystals, 39,400 points, 5
  deaths, game over, in both.
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
- Blocks and Braille, switchable with V (David's choice), and since
  2026-10-20 **Mixed**, the default, after David asked for a gradual move
  to Braille as the screen shrinks and then for Braille's fidelity to be
  used more. Mixed decides each cell by coverage of its 2x4 patch
  (`mixed_cell`): empty is space; partly covered (a sprite edge on space)
  is Braille dots in the brightest colour; fully covered is a half block,
  or two-colour dots where two clearly different colours meet in a
  pattern that is not a top/bottom split. From block scale 0.6 (about 200
  columns) it is all half blocks (`BLOCKS_FROM`). Tried and dropped the
  same day: choosing per cell by least colour error (averaged colours came
  out murky, and coloured backgrounds flooded small sprites).
- `--shot TICKS --size WxH` prints the built-in demo's frame at that tick
  with colour escapes, no terminal: for comparing views on the same frame
  (scratch `shot.py`-style tools turn it into a PNG).
- Downsampling keeps the brightest source pixel per terminal pixel, so
  stars and shots survive; the starfield shows fewer stars on small
  pictures (`set_stars`), or 400 stars crowd 5,000 pixels.
- At full size (`tw >= 320`) the original pixel status bar is drawn;
  below that, a text line with the power-ups in their icon colours.
- Menus and tables are terminal text: the comix font (`assets.comix`,
  loaded) is unreadable shrunk. Using it at full size is an open idea.

## The original Pascal

Mark's source is in xquest-sdl's GitHub release
`archive-original-distribution` (`xquest-1.3-original-distribution.zip`:
xquest.pas, xqvars.pas, xqinit.pas, xqenter.pas and the rest). Check
gameplay questions there, not in memory.

## Fixed in xquest-sdl and here (2026-10-20, xquest-sdl `e26270e`)

Read from xquest.pas, approved by David, done in both:

- Extra ships (`newman`): `add_score` gives a ship with the woohoo each
  time the score reaches the last extra ship's score plus the level's
  `newman` (AddToScore, BonusShip).
- Par time (`time`): the level clock runs through the level, lost lives
  included; the countdown (ShowCountDown, small digit font, top right);
  the bonus window, 500 a second under par (GiveBonus); past par the speed
  rises by one base speed every 32 seconds over.
- Difficulty speeds `round(64 * speedfactor)`: 45, 64, 64, 96, 128.
- A lost life restarts the level (NewLife + StartNewLevel) instead of
  dealing it afresh.
- `bonus_level` stays unread: the original compiled it out
  (`{$UNDEF BONUSLEVEL}`).

## Still different from the 1994 game (found 2026-10-20, not changed)

- Timid walls: the original kills on walls only when `DiffLevel>1`, so
  Timid bounces; xquest-sdl's `game_tick` bounces on Wimp alone.
- Power-ups: the original clears them only on a lost life (NewLife);
  xquest-sdl clears them at every level too (`game_init`).
- After level 50 the original goes round again, faster each time
  (GameClocked, the "rank" message); xquest-sdl ends the game.

## Catalog

- id `aquest`, binary `aquest`, kind game, genre arcade;
  `hub/catalog/aquest.toml` and `.md` (2026-10-20).
- Preview: `asciiverse/record-preview.sh` (100x30, the built-in demo,
  levels 1 and 2 with the countdown, Mixed with Blocks from 18 s to 34 s;
  still at 41 s, the level 2 bonus window). About 16 MB:
  the game redraws at 67 frames a second. The header's `term.version`
  ("tmux 3.6") was removed by hand after recording.
- Website mockup: NEW! banner in `dev/mock-titles.toml`, cast in
  `casts/aquest.cast`, local Linux x86_64 build in
  `dev-releases/aquest` (1.0.2-av.1; links libasound and libudev). The
  mockup's `local_binary` looks in `games/<id>`, which this folder's
  capitals miss, hence the packaged release.
- Release workflow `.github/workflows/release.yml`, on ASCII Wars!'
  pattern. Tag `v1.0.2-av.1` (2026-10-20; `v1.0.0-av.1` before Mixed) pushed; its run was refused by
  the organisation's Actions billing, so no GitHub release yet (re-run it,
  or `workflow_dispatch` with the tag, once Actions runs again).
- Repo: private at github.com/theasciiverse/aquest, branch `main`
  (pushed 2026-10-20).

## Commands

```sh
cargo run --release                     # play
cargo run --release -- --demo           # the built-in demo
cargo test --release                    # replay test and the rest
RUST_LOG=debug ./target/release/aquest 2>aquest.log
```
