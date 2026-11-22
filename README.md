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

## Release builds (GitHub Actions)

Pushing a tag builds the game for Windows, Linux and Android and publishes the builds as a
GitHub release named after the tag (`.github/workflows/release.yml`):

```bash
git tag v0.1
```

```bash
git push origin v0.1
```

| File | Contents |
| --- | --- |
| `rusty-sudoku-<tag>-windows-x64.zip` | `rusty-sudoku.exe`, `assets/` and this README |
| `rusty-sudoku-<tag>-linux-x64.tar.gz` | the `rusty-sudoku` binary, `assets/` and this README (built on Ubuntu 22.04) |
| `rusty-sudoku-<tag>-android.apk` | the game for Android 5 and later (ARM phones and tablets, x86_64 Chromebooks and emulators), portrait or landscape as the phone is held |

Some things to know:

- Each platform is its own job, so if one fails the others still go into the release. You can see
  each job's log in the repository's Actions tab, and also start a build there by hand ("Run
  workflow"); a hand-started run makes the files as downloads on the run, without a release.
- The Linux binary needs ALSA, X11 and OpenGL, which desktop distributions have already.
- The Android build uses macroquad's packager, `cargo quad-apk`, built from source at a pinned
  commit (`QUAD_APK_REV` in the workflow), with the runner's Android SDK and NDK. Its settings
  (app name, package name, SDK versions, orientation) are under `[package.metadata.android]` in
  `Cargo.toml`. The app id is `io.github.savvazakharov.rusty_sudoku`. The package name must end in
  `rusty-sudoku`, the binary's name, or the app can't find its native library and closes on
  launch. Android asks you to allow installing it from an unknown source.
- Before the APK goes into a release, it's installed and launched on an Android 10 and an
  Android 15 emulator, and must still be running 25 seconds later
  (`.github/android-smoke-test.sh`). Each test keeps its logcat and a screenshot as downloads
  on the run. If a test fails, the release goes out without the APK.
- Each build's `versionCode` is the workflow run number and its `versionName` is the tag without
  the `v`, so a new release installs over the last one.
- The APK is signed in the workflow. Set two repository secrets so every build is signed with
  the same key and new versions install over old ones: `ANDROID_KEYSTORE` (a `.jks` keystore,
  base64-encoded) and `ANDROID_KEYSTORE_PASSWORD`. Without them, each run signs with a throwaway
  key: the APK installs, but moving to another build means uninstalling the old one first.
- On Android the colour schemes and sounds are packed inside the APK.
- The app icon is `assets/icon.png`: built into the desktop program, linked by the web page,
  and turned into the Android launcher icons (`android/res/mipmap-*/ic_launcher.png`) by
  `python android/icons.py` (needs Pillow). Run that again after changing the icon.

## Screen

The base screen is 315 x 560 in portrait (9:16). Taller screens add rows: the
extra height goes evenly above and below the board. In landscape the screen is
560 x 315 (wider screens add columns): the HUD on one line with the board's name
and stats, the board on the left, and on the right the number and notes keypads
at the top, the swipe pad under them and the command line; the control bar runs
in one row across the bottom.

- **Board**: 46 x 28 box drawing characters. Double lines (`╔═╦╬║`) round the edge
  and between the 3x3 boxes, single lines (`─│┼`) between cells. The glyphs fill
  their whole 6x10 text cell, so they join up (`src/font.rs`, `boxch`).
- **Number and notes rows**: square buttons the size of a board cell, with double
  lines round each row and single lines between, like the board. The number row
  shows big digits (and in each corner how many are left); the notes row under it
  shows each digit small, in that note's place in a cell. In landscape they're two
  3-wide keypads side by side. The row in use (notes mode or not) has bright double
  lines. The digit in focus (picked in fast mode, or the selected cell's) shows in
  its row as on the board: amber on dark amber, outlined in amber. Finished digits
  are dimmed.
- **Command line** (`>`): what happened, typed out with click sounds.
- **Control bar**: `[U]NDO`, `[E]RASE`, `[N]OTES`, `[F]AST`, `[H]INT`, `[T]HEME`, `NE[W]`,
  `DAIL[Y]`, `[M]UTE` (red while the sound is off). Two rows (five, then four) in
  portrait, one row in landscape.

## Playing

Tap a cell, then a number in the number row to place it, or in the notes row to
note it (each tap sets notes mode to match its row). Tapping the same digit again clears it. Tapping the
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
given cell just selects it. Tap the same digit in the other row to switch between placing and noting it, or
the same button again to put it down. On a keyboard, pressing the picked digit's
key again switches `[N]OTES` on or off for it, and a third time puts it down and
sets notes back. Once all nine of the picked digit are
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
| `T` | theme: colours, CRT scanlines and font |
| `W` | new board |
| `Y` | daily challenge |
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

## Theme

`[T]HEME` opens the theme dialog. `<` / `>` step through the colour schemes, `[C]RT`
switches the scanlines and `[F]ONT` steps through the text styles; the screen shows
each change at once, and `[A]CCEPT` keeps them (`[E]XIT` goes back).

| Font | Look |
| --- | --- |
| STANDARD | the 5x7 terminal font |
| LARGE | taller 5x9 letters and digits (the layout keeps its 6x10 character grid) |
| SERIF | the 5x7 shapes with serifs: feet on stems, ticks at the ends of bars |

The font is used for all text and the board's and buttons' digits; the small note
digits keep their own font.

## Daily challenge

`DAIL[Y]` (`Y`) opens the **DAILY CHALLENGE** dialog. Each day (UTC) has ten boards,
two sizes by five levels, generated from the date, so everyone playing 9x9 MEDIUM on
2026-11-22 gets the same board. `<` / `>` pick the level and `[S]IZE` the size; the dialog
shows that board's top ten online and how many have played. `[P]LAY` starts it. Today's
boards (also when started from `NE[W]` with today's seed) have no hints: `[H]INT` is dimmed.

Solving one shows your time to a tenth of a second and asks for a nickname, arcade style:
up to eight letters, digits, `-`, `.` or `_`. Type it, or use `^` / `⌄` to change the
letter under the cursor and `<` / `>` to move. `[ENTER] SEND` sends it, `[ESC] SKIP`
doesn't; the leaderboard then shows where you placed. Each name keeps only its best time
per board. The name is saved and filled in next time. If the time can't be sent
(`OFFLINE`), `[ENTER] RETRY` tries again. A board solved with hints (say, from before the
daily) isn't ranked.

The leaderboard lives on the website that hosts the web version of rusty-majong: a Vercel
function at `/api/sudoku` in the pano-gallery project (`api/sudoku.js`, beside mahjong's
`api/daily.js`), storing times in the same Upstash Redis. The web version asks its own site;
desktop and Android builds send to `https://mindbird.site/api/sudoku`
(`daily::DEFAULT_URL` in `src/daily.rs`), or to `--leaderboard URL`. The server:

- only takes today's and yesterday's boards (for anyone finishing just after midnight UTC);
- only takes a time sent with the board's solution: it generates the board again with
  `api/_sudoku.js`, a port of the game's generator (`src/sudoku.rs`);
- rejects times under 30 s (9x9) or 10 s (6x6), badly formed names and names on a small
  blocklist (`PICK ANOTHER NAME`);
- limits each address to 30 submissions and 300 leaderboard reads an hour.

The time is still the game's word, so a determined cheat can fake one. Remove an entry in
the Upstash console with `ZREM sudoku:<date>:<size>:<level> <NAME>`.

`api/_sudoku.js` must generate exactly as the game does. If the generator, the levels or
the random numbers change, update it too, or every genuine time is rejected. To check:
`node web/dev-server.js` serves the web build from `dist/` at http://localhost:8081 and runs
`api/sudoku.js` with an in-memory store instead of Redis, then
`DAILY_TEST_URL=http://localhost:8081/api/sudoku cargo test -- --ignored daily` solves all
ten of today's boards and sends them; the server must accept every one. A desktop build can
play against it with `cargo run -- --leaderboard http://localhost:8081/api/sudoku`.

To put the web version on the site next to mahjong (`mindbird.site/sudoku/`):

```bash
bash web/export.sh ../pano-gallery/sudoku
```

## Best times

Every board you start is recorded, and solving one keeps its best time with that
run's errors and hints. Under the seed, the `NE[W]` dialog lists the boards of the
seed shown (today's date by default, so these are the day's daily boards) in the
size picked: each level's best time, `STARTED` if it was begun but not solved, or
`-`. The level being picked is in amber; tap a row to pick that level. Beating a board's best time says
`NEW BEST TIME`. The last 300 boards are kept.

## Saved settings

The daily nickname, level, size, colour scheme, scanlines, font, mute, the board in progress and the
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
cargo run --release -- --shot serif.png --size 315x560 --demo --font serif --theme-dialog
cargo run --release -- --shot daily.png --size 315x560 --daily-dialog
cargo run --release -- --shot win.png --size 315x560 --daily-win 171.4 --name SAVVA
```
