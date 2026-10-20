# AQuest

**XQuest in the terminal.** Mark Mackey's 1994 arcade game, flown with the
same physics, the same 50 levels, the same 18 enemy kinds and the same
sounds, drawn in a text terminal.

> You're a jolly little ship dingus which shoots around a rather abstract
> landscape collecting little gem thingies, while avoiding mines. Boring,
> you say? Well, we kept the Super Kill-o-Zapper, and added LOTS of things
> to blow up. Happy?
>
> *the original XQuest manual, Mark Mackey, 1994*

Pick up every crystal on the level and the gate at the top opens; fly out
through it to the next level. Mines blow you up, and so does the wall
(unless you play Wimp). Enemies pour in through the side gates: Grungers,
Zippos, Zingers, Vinces, Miners, Meebies, Retaliators, Terriers, Doingers,
Snipes, Tribblers, Buckshots, Clusters, Sticktights and Repulsors. Catch a
Supercrystal for a power-up: Shield, AimedFire, RapidFire, MultiFire,
AssFire, HeavyFire or Bounce. Smart bombs clear the screen.

AQuest is built on [xquest-sdl](https://github.com/sandbranch/xquest-sdl),
David's faithful port of XQuest v1.3 to C and SDL2, made with Mark Mackey's
blessing. The game logic is translated line by line, and a demo recorded
in either one replays in the other frame for frame.

## Playing

```sh
cargo run --release
```

The ship steers by thrust: a held direction speeds it up that way, and it
drifts on when you let go. Brake to stop. Your shots go the way you fly,
as fast as you fly.

| Action | Keyboard | Mouse | Gamepad |
|---|---|---|---|
| Thrust | arrows, WASD, numpad 8 4 6 2 | move it | stick or d-pad |
| Fire (hold with RapidFire) | Space or Enter | left button | A |
| Brake | Shift, Z or numpad 5 | middle button | X or a shoulder |
| Smart bomb | B or X | right button | B |
| Pause | P | | Start |
| Quit the game | Esc, then Y | | |
| Blocks or Braille | V | | |
| Sound on or off | M | | |

Best in a terminal that reports key releases (the kitty keyboard
protocol: kitty, foot, WezTerm, Ghostty, Alacritty, iTerm2 and others),
with the mouse, or with a gamepad. Elsewhere the terminal sends only key
repeats, so a held key stutters until they start and thrust goes on a
moment after you let go.

### Difficulty

| | Speed | Enemies | Walls |
|---|---|---|---|
| Wimp | 0.7x | 0.7x | bounce |
| Timid | 0.84x | 1.0x | deadly |
| Average | 1.0x | 1.0x | deadly |
| Tricky | 1.2x | 1.2x | deadly |
| Inhuman | 1.5x | 1.5x | deadly |

(As in xquest-sdl: the speeds are its `DIFF_SPEED`, 45, 54, 64, 77 and 96
against Average's 64.)

### How it looks

The picture is the original 320x240 screen, a 320x217 window scrolling
over the 392x320 arena, fitted to the terminal with square pixels:

- **Blocks** (the default): half-block cells, two pixels a cell, each in
  its own colour. Mark's palette and sprites as they were.
- **Braille**: 2x4 dots a cell in one colour, four times the detail.

A shrunk picture keeps the brightest pixel of each patch it covers, so
one-pixel stars and shots never vanish. Once the terminal is big enough to
show the screen at full size (320 columns in Blocks, 160 in Braille) the
original status bar appears in Mark's own font; smaller, it is a line of
text.

## Options

```
--braille, --blocks   how to draw (V switches while playing)
--demo           play the built-in demo, then exit
--play FILE      play back a demo (xquest.dmo format), then exit
--record FILE    record the next game to FILE
--verify FILE    replay a demo without drawing it and print the result
--difficulty N   0 Wimp, 1 Timid, 2 Average, 3 Tricky, 4 Inhuman
--mute           no sound
```

The menu also records demos (to `xquest.dmo` in the data folder) and plays
them back; after 30 seconds idle it plays one by itself, the last one you
recorded or else the built-in one.

## Files

In `$XDG_DATA_HOME/aquest` (`~/.local/share/aquest`; `%APPDATA%\aquest` on
Windows, `~/Library/Application Support/aquest` on macOS; `AQUEST_DATA_DIR`
overrides it):

- `xquest.scr`: the Hall of Fame, in the original binary format
- `xquest.dmo`: the last demo recorded, in the original format
- `aquest.cfg`: difficulty, graphics and sound

Both `.scr` and `.dmo` files move freely between AQuest and xquest-sdl.

## Testing

```sh
cargo test --release
```

`reference_demo_replays_as_in_xquest_sdl` replays xquest-sdl's own test
recording, `tests/data/reference.dmo` there, through AQuest's engine and
checks the result xquest-sdl's `tests/test_replay.c` checks: 12,066
frames, 6 levels, 162 crystals, 36,000 points, 2 deaths. Any change to the
physics, the collisions or the order of random draws breaks it.

`RUST_LOG=debug aquest 2>aquest.log` logs every key and click, each state
change, and a snapshot of the game twice a second.

## What AQuest changes

Everything that happens in the game is xquest-sdl's. What is new is the
terminal around it:

- The picture is fitted to the terminal in Blocks or Braille (`view.rs`);
  the menus, Game Over box and Hall of Fame are text over the starfield
  instead of Mark's comix font, which is too fine to read shrunk.
- A text status line under smaller pictures.
- The mouse thrusts by how far the pointer moves between cells (a
  terminal has no relative mouse), at about xquest-sdl's sensitivity.
- Keys for the smart bomb (B, X) and brake (Z), which xquest-sdl had on
  the mouse and Shift only.
- The game data and the reference demo are compiled into the binary, so
  AQuest is a single file. The demo plays on the title screen.
- Graphics and Sound entries in the menu; settings in `aquest.cfg`
  instead of the original `xquest.cfg`.

## Credits

- **XQuest** (1994 to 1996): Mark Mackey. Game, graphics, sounds, levels.
- **xquest-sdl**: David, the port of XQuest v1.3 to C and SDL2 this one is
  built on, with Mark Mackey's blessing.
- **AQuest**: the terminal edition, for the
  [Asciiverse](https://asciiverse.sh) catalog.

Free to play and share, never sold. See [LICENSE](LICENSE).
