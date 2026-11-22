# rusty-sudoku

Sudoku in Rust, drawn like rusty-majong: a palette-indexed framebuffer, the same
5x7 terminal font and colour schemes, and a board built from box drawing characters.

```bash
cargo run --release
```

## Web version

From PowerShell:

```powershell
powershell -ExecutionPolicy Bypass -File web/build.ps1
```

From Git Bash: `bash web/build.sh`. (In PowerShell, `bash` is WSL's bash, whose Rust
needs its own `rustup target add wasm32-unknown-unknown`.)

```bash
npx --yes serve@14 dist --listen tcp://0.0.0.0:8081 --no-clipboard
```

Open `http://<this-machine's-LAN-IP>:8081` on a phone on the same network.

## Screen

The base screen is 315 x 560 in portrait (9:16). Taller screens add rows: the
extra height goes evenly above and below the board. In landscape the screen is
560 x 315 (wider screens add columns), with the board on the left and a panel
on the right holding the board's name, the digits, the command line and the controls.

- **Board**: 46 x 28 box drawing characters. Double lines (`╔═╦╬║`) round the edge
  and between the 3x3 boxes, single lines (`─│┼`) between cells. The glyphs fill
  their whole 6x10 text cell, so they join up (`src/font.rs`, `boxch`).
- **Digits**: square buttons the size of a board cell, drawn with shared single
  lines like the board: a row of 9 (or 6) under the board in portrait, a 3-wide
  keypad in landscape. Each previews what tapping it puts on the board: a big
  digit, or in notes mode (green lines) a small digit in that note's place in a
  cell. The digit in focus (picked in fast mode, or the selected cell's) shows as it
  does on the board, amber on dark amber, outlined in amber. Finished digits are
  dimmed. How many are left shows under each button in portrait, and in its corner
  in landscape (not in notes mode).
- **Command line** (`>`): what happened, typed out with click sounds.
- **Control bar**: `[U]NDO`, `[E]RASE`, `[N]OTES`, `[F]AST`, `[H]INT`, `[T]HEME`, `NE[W]`.

## Playing

Tap a cell, then a digit. Tapping the same digit again clears it. Tapping the
selected cell again lets it go. Tapping a digit with no cell selected switches to
`[F]AST` mode with that digit picked. A digit that
repeats in its row, column or box turns red (with the digit it clashes with) and
counts as an error. A digit that's wrong but clashes with nothing on the board
isn't marked: only `[H]INT` checks against the answer. With `[N]OTES` on, digits toggle pencil marks instead. A note can't be added for a
digit already in the cell's row, column or box (it says where); taking one off
always works.
Placing a right digit removes it from the notes in its row, column and box.

`[F]AST` switches to number-first input: pick a digit in the digit row (it stays
lit), then every tap on an open cell places it, or toggles it as a note while
`[N]OTES` is on. Cells that already hold another digit are left alone (erase it first); tapping a
cell that holds the picked digit clears it; tapping a
given cell just selects it. Tap the lit digit (or press its key) again to switch `[N]OTES` on or off for it;
a third time puts it down and sets notes back. Once all nine of the picked digit are
on the board with no clashes, fast mode moves on to the next digit that still has cells to fill. The picked
digit's cells and notes are highlighted across the board. On a keyboard, digit keys
pick the digit and `Enter` places it in the selected cell.

`[H]INT` first erases a wrong digit if there is one. Otherwise it places a digit
that logic forces (in the selected cell if it can) and says why: `NOTHING ELSE FITS`
or `ONLY SPOT IN BOX 5`.

On a touch screen, the empty area under the digits is a swipe pad (a mouse drag
there works too): swipe left / right for the next / previous digit still to place
(as `A` / `D`, switching fast mode on), up to switch notes on or off, down to undo. Once the screen has been touched, a legend shows there:
`SWIPE: < > DIGIT  ^ NOTES  ⌄ UNDO`.

| Key | Action |
| --- | --- |
| `1`–`9` | enter a digit |
| arrows | move the selection |
| `Backspace` / `Delete` / `0` / `E` | erase |
| `N` / `Space` | notes on or off |
| `F` | fast (number-first) mode on or off |
| `A` / `D` | fast mode: pick the previous / next number still to place (turns fast mode on) |
| `Enter` | fast mode: place the picked digit in the selected cell |
| `U` / `Z` | undo |
| `H` | hint |
| `T` | colours and CRT scanlines |
| `W` | new board |
| `M` | mute |
| `C` | scanlines on or off |
| `Esc` | quit (desktop) |

## Boards and seeds

Boards come in two sizes: 9x9 with 3x3 boxes, and 6x6 with boxes of 2 rows by 3
columns (digits 1-6). A 6x6 board has bigger cells and digits, and six digit
buttons. Pick the size with `[S]IZE` in the `NE[W]` dialog; the game starts on the
size you played last.

Every board comes from an 8-digit seed, a size and one of five levels. Today's date (UTC)
is the default seed, so everyone gets the same five daily boards. Each level and
size mixes its own number into the random stream, so the ten boards of a seed are
completely different grids, not the same grid with more or fewer clues.

| Level | Clues | Rule |
| --- | --- | --- |
| EASY | ~38 | naked singles solve it |
| MEDIUM | ~32 | naked and hidden singles solve it |
| HARD | ~27 | singles solve it, but naked singles alone don't |
| EXPERT | ~25 | unique, needs more than singles |
| MASTER | minimal (~23) | unique, needs more than singles, no symmetry |

On 6x6 boards singles almost always suffice, so the levels there go by clues and
hidden singles:

| Level | Clues | Rule |
| --- | --- | --- |
| EASY | ~20 | naked singles solve it |
| MEDIUM | ~16 | naked and hidden singles solve it |
| HARD | ~12 | singles solve it, but naked singles alone don't |
| EXPERT | ~10 | unique, naked singles alone don't solve it |
| MASTER | minimal (~9-11) | as EXPERT, no symmetry |

Clues are removed in symmetric pairs (except the last MASTER pass) while the
level's rule still solves the board (`src/sudoku.rs`). The `NE[W]` dialog picks the
level (`<` / `>`), the size (`[S]IZE`) and the seed: `[T]ODAY`, `[R]ANDOM`, or type digits (the on-screen
digit row types them too).

## Best times

Every board you start is recorded, and solving one keeps its best time with that
run's errors and hints. Under the seed, the `NE[W]` dialog lists the boards of the
seed shown (today's date by default, so these are the day's daily boards) in the
size picked: each level's best time, `STARTED` if it was begun but not solved, or
`-`. The level being picked is in amber; tap a row to pick that level. Beating a board's best time says
`NEW BEST TIME`. The last 300 boards are kept.

## Saved settings

The level, size, colour scheme, scanlines, mute, the board in progress and the
best times are saved as
`key=value` lines: `%APPDATA%\rusty-sudoku\settings.txt` on Windows, localStorage
(`rusty-sudoku-settings`) on the web.

## Screenshots

```bash
cargo run --release -- --shot out.png --size 315x560 --level 2 --seed 20261121 --demo
cargo run --release -- --shot six.png --size 315x560 --six --demo
cargo run --release -- --shot new.png --size 315x560 --demo --new-dialog
cargo run --release -- --shot notes.png --size 315x560 --demo --pick 4 --notes
cargo run --release -- --shot touch.png --size 315x560 --demo --touch
```
