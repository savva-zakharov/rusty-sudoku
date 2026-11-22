//! The game: board state, input, the portrait and landscape layouts, drawing.

use crate::fb::{Fb, CW, LH};
use crate::font::{Style, STYLES};
use crate::palette::*;
use crate::daily::{self, NameEdit};
use crate::net::{Reply, Request};
use crate::sudoku::{self, Grid, Reason, Shape, LEVELS, NINE, SHAPES};
use macroquad::audio::{play_sound, PlaySoundParams, Sound};
use macroquad::input::{is_key_pressed, is_mouse_button_pressed, is_mouse_button_released, KeyCode, MouseButton};
use std::collections::VecDeque;

/// Base screen: 315 x 560 in portrait, 560 x 315 in landscape (16:9).
pub const LONG: i32 = 560;
pub const SHORT: i32 = 315;

/// How a board shape is drawn: each cell is `cw` characters by `ch` lines
/// inside its lines, and its digit is drawn at `scale` times the font size.
/// 9x9: 4 x 2 (cells 30 x 30 pixels apart, the board 46 x 28 characters,
/// 276 x 280 pixels). 6x6: 6 x 3 (cells 42 x 40 apart, the board 43 x 25
/// characters, 258 x 250 pixels), with bigger digits.
#[derive(Clone, Copy)]
struct Geom {
    s: Shape,
    cw: usize,
    ch: usize,
    scale: i32,
}

impl Geom {
    fn of(s: Shape) -> Geom {
        if s.n == 9 {
            Geom { s, cw: 4, ch: 2, scale: 2 }
        } else {
            Geom { s, cw: 6, ch: 3, scale: 3 }
        }
    }
    fn cols(self) -> usize {
        self.s.n * (self.cw + 1) + 1
    }
    fn rows(self) -> usize {
        self.s.n * (self.ch + 1) + 1
    }
    fn w(self) -> i32 {
        self.cols() as i32 * CW
    }
    fn h(self) -> i32 {
        self.rows() as i32 * LH
    }
    /// Distance between cells in pixels.
    fn px(self) -> i32 {
        (self.cw as i32 + 1) * CW
    }
    fn py(self) -> i32 {
        (self.ch as i32 + 1) * LH
    }
}

/// Typing speed of the command line, in steps per second.
const TYPE_CPS: f32 = 70.0;
/// Steps the letter after a space takes (other characters take 1).
const AFTER_SPACE_STEPS: f32 = 2.0;
/// Click on every Nth letter.
const CLICK_EVERY: usize = 2;
const VOLUME: f32 = 0.3;
/// Messages waiting to type; beyond this the oldest waiting one is dropped.
const MSG_QUEUE_MAX: usize = 2;

const SEED_DIGITS: usize = 8;

/// The sounds: letter clicks and spaces while typing, enter for a right
/// digit, escape for a wrong one.
#[derive(Default)]
pub struct Sounds {
    pub clicks: Vec<Sound>,
    pub spaces: Vec<Sound>,
    pub enters: Vec<Sound>,
    pub escapes: Vec<Sound>,
}

#[derive(PartialEq, Debug, Clone, Copy)]
enum TypeSound {
    Click,
    Space,
}

/// How many characters of `msg` show after `steps` typing steps.
fn chars_shown(msg: &str, steps: f32) -> usize {
    let mut t = 0.0;
    let mut prev_space = false;
    for (n, ch) in msg.chars().enumerate() {
        t += if prev_space { AFTER_SPACE_STEPS } else { 1.0 };
        if t > steps {
            return n;
        }
        prev_space = ch.is_whitespace();
    }
    msg.chars().count()
}

/// A line typed out one character at a time.
#[derive(Default, Clone)]
struct Typer {
    t: f32,
    typed: usize,
    letters: usize,
}

impl Typer {
    fn done(&self, text: &str) -> bool {
        self.typed >= text.chars().count()
    }

    fn step(&mut self, text: &str, dt: f32) -> Option<TypeSound> {
        self.t += dt;
        let shown = chars_shown(text, self.t * TYPE_CPS);
        if shown <= self.typed {
            return None;
        }
        let (mut space, mut click) = (false, false);
        for ch in text.chars().skip(self.typed).take(shown - self.typed) {
            if ch.is_whitespace() {
                space = true;
            } else {
                click |= self.letters % CLICK_EVERY == 0;
                self.letters += 1;
            }
        }
        self.typed = shown;
        if space {
            Some(TypeSound::Space)
        } else if click {
            Some(TypeSound::Click)
        } else {
            None
        }
    }

    fn finish(&mut self, text: &str) {
        self.typed = text.chars().count();
        self.t = 1e6;
    }
}

/// Everything a key, a button or a tap can do.
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
enum Action {
    Select(usize),
    Move(i32, i32),
    Digit(u8),
    Erase,
    Notes,
    Fast,
    /// Fast mode: put the picked digit in the selected cell (Enter).
    Place,
    Undo,
    Hint,
    Theme,
    New,
    Mute,
    Crt,
    Quit,
    LevelPrev,
    LevelNext,
    NewYes,
    NewNo,
    SeedToday,
    SeedRandom,
    SeedBack,
    ThemePrev,
    ThemeNext,
    ThemeAccept,
    ThemeExit,
    SolvedClose,
    /// Theme dialog: the next text style.
    Font,
    /// New-board dialog: switch between 9x9 and 6x6.
    Size,
    /// The daily challenge dialog: open it, play its board, close it.
    Daily,
    DailyPlay,
    DailyExit,
    /// Nickname entry after solving a daily board, arcade style.
    NameUp,
    NameDown,
    NameLeft,
    NameRight,
    NameBack,
    NameSubmit,
    NameSkip,
    /// A tap on the number row (place the digit) or the notes row (note it).
    Num(u8),
    Note(u8),
    /// Fast mode: pick the next (1) or previous (-1) digit still to place.
    CycleDigit(i32),
    /// New-board dialog: pick a level by its row in the best times table.
    PickLevel(usize),
}

/// The control bar along the bottom: two rows (five, then four) in
/// portrait, one row across the whole screen in landscape.
const BAR: [(&str, Action); 9] = [
    ("[U]NDO", Action::Undo),
    ("[E]RASE", Action::Erase),
    ("[N]OTES", Action::Notes),
    ("[F]AST", Action::Fast),
    ("[H]INT", Action::Hint),
    ("[T]HEME", Action::Theme),
    ("NE[W]", Action::New),
    ("DAIL[Y]", Action::Daily),
    ("[M]UTE", Action::Mute),
];

type Rect = (i32, i32, i32, i32);

fn in_rect(mx: i32, my: i32, (x, y, w, h): Rect) -> bool {
    mx >= x && mx < x + w && my >= y && my < y + h
}

/// A text button: where its label is drawn and its (generous) hit area.
#[derive(Clone)]
struct Button {
    label: &'static str,
    action: Action,
    text_x: i32,
    text_y: i32,
    hit: Rect,
}

/// Buttons in a grid over `area`, `cols` to a row, each label centred in
/// its cell and the whole cell its hit area, so there are no dead gaps.
fn grid_buttons(items: &[(&'static str, Action)], (x, y, w, h): Rect, cols: usize) -> Vec<Button> {
    let rows = items.len().div_ceil(cols);
    let (cw, ch) = (w / cols as i32, h / rows as i32);
    items
        .iter()
        .enumerate()
        .map(|(k, &(label, action))| {
            let (c, r) = ((k % cols) as i32, (k / cols) as i32);
            let (cx, cy) = (x + c * cw, y + r * ch);
            let lw = Fb::text_w(label) - 1;
            Button { label, action, text_x: cx + (cw - lw) / 2, text_y: cy + (ch - 7) / 2, hit: (cx, cy, cw, ch) }
        })
        .collect()
}

/// Buttons side by side, centred on `cx`, as in rusty-majong's dialogs.
fn row_buttons(items: &[(&'static str, Action)], cx: i32, y: i32) -> Vec<Button> {
    const GAP: i32 = 18;
    let widths: Vec<i32> = items.iter().map(|(l, _)| Fb::text_w(l) - 1).collect();
    let total: i32 = widths.iter().sum::<i32>() + GAP * (items.len() as i32 - 1);
    let mut x = cx - total / 2;
    items
        .iter()
        .zip(widths)
        .map(|(&(label, action), w)| {
            let b = Button { label, action, text_x: x, text_y: y, hit: (x - GAP / 2, y - 6, w + GAP, 19) };
            x += w + GAP;
            b
        })
        .collect()
}

/// Where everything goes on a screen of a given size.
struct Layout {
    w: i32,
    h: i32,
    portrait: bool,
    geom: Geom,
    /// The board as lines of box drawing characters.
    grid_text: Vec<String>,
    /// Top-left of the board text.
    gx: i32,
    gy: i32,
    /// The number row and, under it, the notes row: square cells the size of
    /// a 9x9 board cell, double lines round the edge and single ones between,
    /// like the board. Rows under the board in portrait; keypads (3 across)
    /// side by side in landscape.
    pads: [Pad; 2],
    hud_rule: i32,
    prompt: (i32, i32),
    /// The dashed rule over the control bar: y, and the x range it spans.
    rule: (i32, i32, i32),
    bar: Vec<Button>,
    /// Rows behind which the dot lattice is drawn: (x0, y0, x1, y1).
    lattice: Rect,
    /// The swipe pad: the empty area under the digits, where swipes pick the
    /// digit (left / right), switch notes (up) and undo (down).
    swipe: Rect,
}

/// The framebuffer size for a window: the base screen for its orientation,
/// grown along whichever side the window has extra room on.
pub fn screen_size(sw: f32, sh: f32) -> (i32, i32) {
    let (bw, bh) = if sw >= sh { (LONG, SHORT) } else { (SHORT, LONG) };
    if sw <= 0.0 || sh <= 0.0 {
        return (bw, bh);
    }
    if sw / sh >= bw as f32 / bh as f32 {
        (((bh as f32 * sw / sh).round() as i32).clamp(bw, 3 * bw), bh)
    } else {
        (bw, ((bw as f32 * sh / sw).round() as i32).clamp(bh, 3 * bh))
    }
}

impl Layout {
    fn new(w: i32, h: i32, s: Shape) -> Layout {
        let g = Geom::of(s);
        if h > w {
            Self::portrait(w, h, g)
        } else {
            Self::landscape(w, h, g)
        }
    }

    /// HUD on two lines at the top, then the board, the digits under it,
    /// the command line and the control bar along the bottom (in thumb reach).
    /// Extra height goes evenly above and below the board.
    fn portrait(w: i32, h: i32, g: Geom) -> Layout {
        let (gw, gh, n) = (g.w(), g.h(), g.s.n as i32);
        let hud_rule = 25;
        let rule_y = h - 36;
        let prompt_y = rule_y - 14;
        let block = gh + 6 + 2 * PAD_H + 4;
        let top = hud_rule + 5;
        let free = (prompt_y - 6 - top - block).max(0);
        let gy = top + free / 2;
        let gx = (w - gw) / 2;
        // The number row under the board, the notes row under that.
        let cols = n as usize;
        let x = (w - pad_w(cols)) / 2;
        let nums = keypad((x, gy + gh + 6), cols, 1, cols);
        let notes = keypad((x, nums.bottom()), cols, 1, cols);
        let swipe_top = notes.bottom() + 4;
        Layout {
            w,
            h,
            portrait: true,
            geom: g,
            grid_text: grid_lines(g),
            gx,
            gy,
            pads: [nums, notes],
            hud_rule,
            prompt: (4, prompt_y),
            rule: (rule_y, 0, w),
            bar: grid_buttons(&BAR, (0, rule_y + 1, w, h - rule_y - 1), 5),
            lattice: (0, hud_rule + 3, w, rule_y - 2),
            swipe: (0, swipe_top, w, rule_y - swipe_top),
        }
    }

    /// HUD on one line, the board on the left, and a panel on the right with
    /// the board's name and stats, the digits, the command line and the
    /// controls in two rows.
    fn landscape(w: i32, h: i32, g: Geom) -> Layout {
        let (gw, gh, n) = (g.w(), g.h(), g.s.n as i32);
        // The HUD on one line (the board's name and stats in it), the control
        // bar in one row across the bottom.
        let hud_rule = 13;
        let rule_y = h - 18;
        let gy = hud_rule + (rule_y - hud_rule - gh) / 2;
        // The panel starts where it would beside a 9x9 board, so it doesn't
        // jump about between sizes.
        let pw = 256;
        let x0 = (w - (Geom::of(NINE).w() + 16 + pw)) / 2;
        let gx = x0.max(4) + (Geom::of(NINE).w() - gw) / 2;
        let px = x0.max(4) + Geom::of(NINE).w() + 16;
        let prompt_y = rule_y - 13;
        // Number and notes keypads, three across, side by side at the top of
        // the panel, level with the board; below them the swipe pad, then the
        // command line.
        let rows = n as usize / 3;
        let gap = 12;
        let x = px + (pw - (2 * pad_w(3) + gap)) / 2;
        let y = hud_rule + (rule_y - hud_rule - Geom::of(NINE).h()) / 2;
        let nums = keypad((x, y), 3, rows, n as usize);
        let notes = keypad((x + pad_w(3) + gap, y), 3, rows, n as usize);
        let swipe_top = nums.bottom() + 2;
        Layout {
            w,
            h,
            portrait: false,
            geom: g,
            grid_text: grid_lines(g),
            gx,
            gy,
            pads: [nums, notes],
            hud_rule,
            prompt: (px, prompt_y),
            rule: (rule_y, 0, w),
            bar: grid_buttons(&BAR, (0, rule_y + 1, w, h - rule_y - 1), BAR.len()),
            lattice: (0, hud_rule + 3, w, rule_y - 2),
            swipe: (px - 4, swipe_top, pw + 8, prompt_y - 4 - swipe_top),
        }
    }

    /// The pixel rectangle inside cell `i` (between its lines).
    fn cell_rect(&self, i: usize) -> Rect {
        let g = self.geom;
        let (r, c) = ((i / g.s.n) as i32, (i % g.s.n) as i32);
        (self.gx + c * g.px() + 4, self.gy + r * g.py() + 6, g.px() - 3, g.py() - 3)
    }

    fn cell_at(&self, mx: i32, my: i32) -> Option<usize> {
        let g = self.geom;
        let (dx, dy) = (mx - self.gx - 2, my - self.gy - 4);
        if dx < 0 || dy < 0 {
            return None;
        }
        let (c, r) = ((dx / g.px()) as usize, (dy / g.py()) as usize);
        (c < g.s.n && r < g.s.n).then_some(r * g.s.n + c)
    }
}

/// A swipe must travel this far (in game pixels, about 50 on a phone screen)
/// within SWIPE_TIME seconds.
const SWIPE_MIN: f32 = 16.0;
const SWIPE_TIME: f32 = 1.0;

/// What a swipe by (dx, dy) does, going by its main direction: left / right
/// pick the next / previous digit (the row moves with the finger, like a
/// carousel), up switches notes, down undoes.
fn swipe_action(dx: f32, dy: f32) -> Option<Action> {
    if dx.abs().max(dy.abs()) < SWIPE_MIN {
        return None;
    }
    Some(if dx.abs() >= dy.abs() {
        Action::CycleDigit(if dx > 0.0 { -1 } else { 1 })
    } else if dy < 0.0 {
        Action::Notes
    } else {
        Action::Undo
    })
}

/// Height of a one-row digit strip: a line, two text lines, a line.
const PAD_H: i32 = 4 * LH;

/// A keypad of square digit buttons: the top-left of its text, the text, each
/// cell's top-left (laid out like a board cell, 30 pixels apart) and each
/// cell's hit area.
struct Pad {
    at: (i32, i32),
    text: Vec<String>,
    cells: Vec<(i32, i32)>,
    hits: Vec<Rect>,
}

impl Pad {
    fn bottom(&self) -> i32 {
        self.at.1 + self.text.len() as i32 * LH
    }
}

/// Width of a digit keypad `cols` cells across (5 characters a cell, plus
/// the closing line).
fn pad_w(cols: usize) -> i32 {
    (cols * 5 + 1) as i32 * CW
}

/// A keypad of `count` square cells, `cols` across and `rows` down, with its
/// top-left at `at`: double lines round the edge, single lines between cells.
fn keypad(at: (i32, i32), cols: usize, rows: usize, count: usize) -> Pad {
    let mut text = vec![];
    for line in 0..rows * 3 + 1 {
        let border = line % 3 == 0;
        let (first, last) = (line == 0, line == rows * 3);
        let mut s = String::new();
        for col in 0..cols * 5 + 1 {
            let on_v = col % 5 == 0;
            let (left, right) = (col == 0, col == cols * 5);
            s.push(match (border, on_v) {
                (false, false) => ' ',
                (false, true) => if left || right { '║' } else { '│' },
                (true, false) => if first || last { '═' } else { '─' },
                (true, true) => match (first, last, left, right) {
                    (true, _, true, _) => '╔',
                    (true, _, _, true) => '╗',
                    (_, true, true, _) => '╚',
                    (_, true, _, true) => '╝',
                    (true, _, _, _) => '╤',
                    (_, true, _, _) => '╧',
                    (_, _, true, _) => '╟',
                    (_, _, _, true) => '╢',
                    _ => '┼',
                },
            });
        }
        text.push(s);
    }
    let cells: Vec<(i32, i32)> = (0..count).map(|k| (at.0 + (k % cols) as i32 * 30, at.1 + (k / cols) as i32 * 30)).collect();
    let hits = cells.iter().map(|&(x, y)| (x + 2, y + 4, 31, 31)).collect();
    Pad { at, text, cells, hits }
}

/// The board as lines of box drawing characters: double lines round the
/// edge and between the boxes, single lines between cells.
fn grid_lines(g: Geom) -> Vec<String> {
    let n = g.s.n;
    // Horizontal line k is double between box rows, vertical line j between box columns.
    let dh = |k: usize| k % g.s.br == 0;
    let dv = |j: usize| j % g.s.bc == 0;
    let mut out = Vec::with_capacity(g.rows());
    for line in 0..g.rows() {
        let mut s = String::with_capacity(g.cols() * 3);
        let border = line % (g.ch + 1) == 0;
        let k = line / (g.ch + 1); // horizontal line index when on a border
        for col in 0..g.cols() {
            let on_v = col % (g.cw + 1) == 0;
            let j = col / (g.cw + 1);
            let ch = match (border, on_v) {
                (false, false) => ' ',
                (false, true) => if dv(j) { '║' } else { '│' },
                (true, false) => if dh(k) { '═' } else { '─' },
                (true, true) => match (k, j) {
                    (0, 0) => '╔',
                    (0, j) if j == n => '╗',
                    (k, j) if k == n && j == n => '╝',
                    (k, 0) if k == n => '╚',
                    (0, _) => if dv(j) { '╦' } else { '╤' },
                    (k, _) if k == n => if dv(j) { '╩' } else { '╧' },
                    (_, 0) => if dh(k) { '╠' } else { '╟' },
                    (_, j) if j == n => if dh(k) { '╣' } else { '╢' },
                    _ => match (dh(k), dv(j)) {
                        (true, true) => '╬',
                        (true, false) => '╪',
                        (false, true) => '╫',
                        (false, false) => '┼',
                    },
                },
            };
            s.push(ch);
        }
        out.push(s);
    }
    out
}

/// A box of `w` characters by 3 lines drawn with single lines.
fn frame(w: usize) -> [String; 3] {
    let mid = w - 2;
    [format!("┌{}┐", "─".repeat(mid)), format!("│{}│", " ".repeat(mid)), format!("└{}┘", "─".repeat(mid))]
}

/// Draw a bar or dialog button: plain, hovered (dark box) or pressed / on
/// (inverse video). `normal` is the label colour when not hovered.
fn draw_button(fb: &mut Fb, label: &str, x: i32, y: i32, normal: u8, hover: bool, pressed: bool) {
    let w = Fb::text_w(label) - 1;
    if pressed {
        fb.rect(x - 2, y - 2, w + 4, 11, AMBER);
        fb.text_keys_br(x, y, label, BG, BG, BG);
    } else if hover {
        fb.rect(x - 2, y - 2, w + 4, 11, UI_DK);
        fb.text_keys_br(x, y, label, UI, AMBER, UI);
    } else {
        fb.text_keys(x, y, label, normal, AMBER);
    }
}

fn dashed(fb: &mut Fb, y: i32, x0: i32, x1: i32) {
    let mut x = x0;
    while x < x1 {
        fb.hline(x, y, 4.min(x1 - x), UI_DK);
        x += 6;
    }
}

/// `R4C7`: row and column, counted from 1.
fn cell_name(s: Shape, i: usize) -> String {
    format!("R{}C{}", s.row(i) + 1, s.col(i) + 1)
}

/// `ROW 4`, `COLUMN 7` or `BOX 5` for unit `u` (see `Shape::units`).
fn unit_name(s: Shape, u: usize) -> String {
    let n = s.n;
    if u < n {
        format!("ROW {}", u + 1)
    } else if u < 2 * n {
        format!("COLUMN {}", u - n + 1)
    } else {
        format!("BOX {}", u - 2 * n + 1)
    }
}

/// Seed digits grouped like a date: `20261121` -> `2026-11-21`.
fn fmt_seed(digits: &str) -> String {
    let mut out = String::new();
    for (k, c) in digits.chars().enumerate() {
        if k == 4 || k == 6 {
            out.push('-');
        }
        out.push(c);
    }
    out
}

fn seed_str(seed: u32) -> String {
    format!("{seed:0w$}", w = SEED_DIGITS)
}

/// The seed for a date, given as days since 1970-01-01: its YYYYMMDD digits.
fn date_seed(days: i64) -> u32 {
    // Howard Hinnant's days-to-civil conversion.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    (y * 10_000 + m * 100 + d) as u32
}

/// What the server made of a submitted time.
fn result_line(b: &daily::Board) -> String {
    let (rank, best) = b.you.unwrap_or((0, 0));
    match b.result.as_deref() {
        Some("BETTER") => format!("NEW PERSONAL BEST! RANK {rank} OF {}", b.count),
        Some("KEPT") => format!("YOUR BEST STAYS {}, RANK {rank}", daily::fmt_ms(best)),
        _ => format!("YOU'RE RANK {rank} OF {}", b.count),
    }
}

/// Today's date (UTC) as YYYY-MM-DD: the daily boards' seed.
pub fn today_date() -> String {
    fmt_seed(&seed_str(today_seed()))
}

/// Today's date (UTC, so everyone gets the same daily boards) as a seed.
pub fn today_seed() -> u32 {
    date_seed((macroquad::miniquad::date::now() / 86_400.0).floor() as i64)
}

fn fmt_time(t: f32) -> String {
    let s = t as u32;
    if s >= 3600 {
        format!("{}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60)
    } else {
        format!("{:02}:{:02}", s / 60, s % 60)
    }
}

fn version() -> String {
    let v = env!("CARGO_PKG_VERSION");
    format!("V{}", v.strip_suffix(".0").unwrap_or(v))
}

enum Dialog {
    /// New board: the level being picked, the seed digits typed so far.
    New { level: usize, shape: Shape, seed: String, typer: Typer },
    /// Colour scheme and scanlines being tried (shown live).
    Theme { scheme: usize, crt: bool, font: Style, typer: Typer },
    /// The daily challenge: today's board for a level and size, and its
    /// leaderboard.
    Daily { level: usize, shape: Shape, typer: Typer },
    Solved,
}

/// A board's leaderboard being fetched, fetched, or failed (with the reason).
enum Scores {
    Loading(Request),
    Ready(daily::Board),
    Failed(String),
}

/// A solved daily board: its time and sending it to the leaderboard.
struct Finish {
    ms: u32,
    submit: Submit,
}

enum Submit {
    Editing(NameEdit),
    Sending(Request),
    Done(daily::Board),
    Failed(String),
    Skipped,
    /// Solved, but not eligible (why).
    Unranked(&'static str),
}

/// Rows in a leaderboard table: the top ten, or nine and the player's own.
const SCORE_ROWS: usize = 10;

enum El {
    /// Text at (x, y) in a colour.
    Text(i32, i32, String, u8),
    /// Text centred on the screen at y, with [key] letters highlighted.
    Center(i32, String, u8),
    /// A filled rectangle (x, y, w, h) in a colour, drawn before the text.
    Rect(i32, i32, i32, i32, u8),
}

/// A dialog as drawn: its frame, title, contents and buttons.
struct View {
    rect: Rect,
    title: &'static str,
    count: Option<String>,
    els: Vec<El>,
    buttons: Vec<Button>,
}

const DIALOG_W: i32 = 252;

/// Boards kept in the record; beyond this the oldest are dropped.
const RECORDS_MAX: usize = 300;

/// One board played: which board, and the best time it was solved in (with
/// that run's errors and hints), or none if it was started but not solved.
/// Saved as `SEED.SIZE.LEVEL.SECONDS.ERRORS.HINTS`, seconds `-` if unsolved.
#[derive(Clone, Debug, PartialEq)]
struct Record {
    seed: u32,
    n: usize,
    level: usize,
    best: Option<(u32, u32, u32)>,
}

impl Record {
    fn parse(s: &str) -> Option<Record> {
        let f: Vec<&str> = s.split('.').collect();
        let [seed, n, level, secs, errors, hints] = f.as_slice() else { return None };
        let best = match *secs {
            "-" => None,
            t => Some((t.parse().ok()?, errors.parse().ok()?, hints.parse().ok()?)),
        };
        Some(Record { seed: seed.parse().ok()?, n: n.parse().ok()?, level: level.parse().ok()?, best })
    }

    fn text(&self) -> String {
        let (t, e, h) = match self.best {
            Some((t, e, h)) => (t.to_string(), e, h),
            None => ("-".into(), 0, 0),
        };
        format!("{}.{}.{}.{t}.{e}.{h}", seed_str(self.seed), self.n, self.level)
    }
}

#[derive(Clone)]
struct Snapshot {
    vals: Grid,
    notes: Vec<u16>,
}

pub struct Game {
    shape: Shape,
    level: usize,
    seed: u32,
    puzzle: Grid,
    solution: Grid,
    /// The board as it stands: givens and the player's digits.
    vals: Grid,
    /// Pencil marks: bit d set for digit d.
    notes: Vec<u16>,
    undo: Vec<Snapshot>,
    sel: Option<usize>,
    notes_mode: bool,
    /// Number-first input: pick a digit, then every tapped cell gets it.
    fast: bool,
    /// The digit picked in fast mode.
    fast_digit: Option<u8>,
    /// Fast mode: the picked digit was picked again once, toggling notes
    /// (picking it a third time puts it down).
    fast_toggled: bool,
    errors: u32,
    hints: u32,
    elapsed: f32,
    solved: bool,
    t: f32,
    msg: String,
    queue: VecDeque<String>,
    typer: Typer,
    lay: Layout,
    hover: Option<Action>,
    /// A button pressed a moment ago, shown in inverse video.
    flash: Option<(Action, f32)>,
    /// Rows, columns and boxes just completed, flashing.
    fx: Vec<(Vec<usize>, f32)>,
    /// A wrong digit just entered, blinking.
    bad: Option<(usize, f32)>,
    dialog: Option<Dialog>,
    schemes: Vec<Scheme>,
    scheme: usize,
    pub crt: bool,
    pub font: Style,
    pub muted: bool,
    pub sounds: Sounds,
    /// A sound already played this frame (at most one per frame).
    sounded: bool,
    /// The daily leaderboard: its address (None: offline), the nickname to
    /// send times under, the boards' leaderboards fetched so far (by date,
    /// size and level) and the solved daily's time being sent.
    pub leaderboard: Option<String>,
    pub nickname: String,
    scores: Vec<((String, usize, usize), Scores)>,
    finish: Option<Finish>,
    /// Boards played (see `Record`), oldest first.
    records: Vec<Record>,
    /// Where and when a press in the swipe pad started.
    swipe_start: Option<(f32, f32, f32)>,
    /// A touch has been seen: show the swipe pad's legend.
    pub touch_seen: bool,
}

impl Game {
    pub fn new(schemes: Vec<Scheme>, scheme: usize) -> Game {
        let (w, h) = (SHORT, LONG);
        Game {
            shape: NINE,
            level: 0,
            seed: 0,
            puzzle: vec![0; 81],
            solution: vec![0; 81],
            vals: vec![0; 81],
            notes: vec![0; 81],
            undo: vec![],
            sel: None,
            notes_mode: false,
            fast: false,
            fast_digit: None,
            fast_toggled: false,
            errors: 0,
            hints: 0,
            elapsed: 0.0,
            solved: false,
            t: 0.0,
            msg: String::new(),
            queue: VecDeque::new(),
            typer: Typer::default(),
            lay: Layout::new(w, h, NINE),
            hover: None,
            flash: None,
            fx: vec![],
            bad: None,
            dialog: None,
            schemes,
            scheme,
            crt: true,
            font: Style::Standard,
            muted: false,
            sounds: Sounds::default(),
            sounded: false,
            leaderboard: None,
            nickname: String::new(),
            scores: vec![],
            finish: None,
            records: vec![],
            swipe_start: None,
            touch_seen: false,
        }
    }

    pub fn resize(&mut self, w: i32, h: i32) {
        self.lay = Layout::new(w, h, self.shape);
    }

    pub fn palette(&self) -> &[Rgb; SLOTS] {
        match &self.dialog {
            Some(Dialog::Theme { scheme, .. }) => &self.schemes[*scheme].rgb,
            _ => &self.schemes[self.scheme].rgb,
        }
    }

    pub fn scanlines(&self) -> bool {
        match &self.dialog {
            Some(Dialog::Theme { crt, .. }) => *crt,
            _ => self.crt,
        }
    }

    /// The text style: the one being tried while the theme dialog is open.
    fn font_now(&self) -> Style {
        match &self.dialog {
            Some(Dialog::Theme { font, .. }) => *font,
            _ => self.font,
        }
    }

    pub fn new_board(&mut self, shape: Shape, level: usize, seed: u32) {
        let (p, s) = sudoku::generate(shape, seed, level);
        if shape != self.shape {
            self.shape = shape;
            self.lay = Layout::new(self.lay.w, self.lay.h, shape);
        }
        self.level = level;
        self.seed = seed;
        self.record();
        let givens = p.iter().filter(|&&d| d != 0).count();
        self.vals = p.clone();
        self.notes = vec![0; shape.cells()];
        self.puzzle = p;
        self.solution = s;
        self.undo.clear();
        self.sel = None;
        self.notes_mode = false;
        self.errors = 0;
        self.hints = 0;
        self.elapsed = 0.0;
        self.solved = false;
        self.fx.clear();
        self.bad = None;
        // Fast mode keeps going, but a picked digit may not exist any more.
        self.fast_digit = self.fast_digit.filter(|&d| d as usize <= shape.n);
        let m = format!("{}: {givens} GIVENS. GOOD LUCK", self.board_name());
        self.say(&m);
    }

    fn is_daily(&self) -> bool {
        self.seed == today_seed()
    }

    /// `DAILY MEDIUM 2026-11-21` for today's board, else `MEDIUM #1234-56-78`;
    /// 6x6 boards add their size: `DAILY MEDIUM 6X6 2026-11-21`.
    fn board_name(&self) -> String {
        let s = fmt_seed(&seed_str(self.seed));
        let mut name = LEVELS[self.level].name.to_string();
        if self.shape != NINE {
            name += &format!(" {}", self.shape.name());
        }
        if self.is_daily() {
            format!("DAILY {name} {s}")
        } else {
            format!("{name} #{s}")
        }
    }

    /// The board in progress as one line:
    /// seed.level.size.digits.notes.seconds.errors.hints
    fn save_board(&self) -> String {
        let vals: String = self.vals.iter().map(|d| (b'0' + d) as char).collect();
        let notes: String = self.notes.iter().map(|n| format!("{n:03x}")).collect();
        // Seconds in steps of 5, so the file isn't rewritten every second.
        let secs = (self.elapsed as u32) / 5 * 5;
        format!("{}.{}.{}.{vals}.{notes}.{secs}.{}.{}", seed_str(self.seed), self.level, self.shape.n, self.errors, self.hints)
    }

    /// Restore a saved board; false (and nothing changed) if it doesn't fit.
    pub fn load_board(&mut self, text: &str) -> bool {
        let mut f: Vec<&str> = text.split('.').collect();
        // Saves from before 6x6 boards have no size: they are 9x9.
        if f.len() == 7 {
            f.insert(2, "9");
        }
        let [seed, level, size, vals, notes, secs, errors, hints] = f.as_slice() else { return false };
        let (Ok(seed), Ok(level)) = (seed.parse::<u32>(), level.parse::<usize>()) else { return false };
        let Some(shape) = SHAPES.iter().copied().find(|s| size.parse() == Ok(s.n)) else { return false };
        let cells = shape.cells();
        if level >= LEVELS.len() || vals.len() != cells || notes.len() != 3 * cells {
            return false;
        }
        let (p, s) = sudoku::generate(shape, seed, level);
        let mut v = vec![0u8; cells];
        let mut n = vec![0u16; cells];
        for i in 0..cells {
            let d = vals.as_bytes()[i].wrapping_sub(b'0');
            if d as usize > shape.n || (p[i] != 0 && d != p[i]) {
                return false;
            }
            v[i] = d;
            let Ok(m) = u16::from_str_radix(&notes[i * 3..i * 3 + 3], 16) else { return false };
            n[i] = m;
        }
        self.new_board(shape, level, seed);
        self.queue.clear();
        self.solved = v == s;
        self.vals = v;
        self.notes = n;
        self.elapsed = secs.parse().unwrap_or(0) as f32;
        self.errors = errors.parse().unwrap_or(0);
        self.hints = hints.parse().unwrap_or(0);
        let m = format!("{}: WELCOME BACK", self.board_name());
        self.msg.clear();
        self.say(&m);
        true
    }

    pub fn settings(&self) -> crate::settings::Settings {
        crate::settings::Settings {
            name: self.nickname.clone(),
            level: Some(self.level),
            size: Some(self.shape.n),
            colours: self.schemes[self.scheme].name.clone(),
            crt: Some(self.crt),
            font: self.font.name().to_string(),
            muted: Some(self.muted),
            board: self.save_board(),
            games: self.records.iter().map(Record::text).collect(),
        }
    }

    /// Notes mode, and fast mode with a digit picked, for screenshots
    /// (`--notes`, `--pick N`).
    pub fn set_input(&mut self, notes: bool, pick: Option<u8>) {
        self.notes_mode = notes;
        if let Some(d) = pick.filter(|&d| d >= 1 && d as usize <= self.shape.n) {
            self.fast = true;
            self.fast_digit = Some(d);
        }
    }

    /// A board part-way through, for screenshots (`--demo`).
    pub fn demo(&mut self) {
        let n = self.shape.n as u8;
        let empty: Vec<usize> = (0..self.shape.cells()).filter(|&i| self.puzzle[i] == 0).collect();
        for &i in empty.iter().step_by(3) {
            self.vals[i] = self.solution[i];
        }
        let c = sudoku::candidates(self.shape, &self.vals);
        for &i in empty.iter().skip(1).step_by(5) {
            if self.vals[i] == 0 {
                self.notes[i] = c[i];
            }
        }
        if let Some(&i) = empty.iter().find(|&&i| self.vals[i] == 0 && self.notes[i] == 0) {
            self.vals[i] = self.solution[i] % n + 1;
            self.errors = 1;
        }
        self.sel = empty.iter().copied().find(|&i| self.vals[i] != 0 && self.vals[i] == self.solution[i]);
        self.elapsed = 222.0;
        // A solved board in the record, for the new-board dialog's table.
        let (seed, n) = (self.seed, self.shape.n);
        self.records.push(Record { seed, n, level: (self.level + 1) % LEVELS.len(), best: Some((252, 1, 0)) });
        self.msg = format!("{} = {}", cell_name(self.shape, self.sel.unwrap_or(0)), self.vals[self.sel.unwrap_or(0)]);
        self.typer.finish(&self.msg);
    }

    // ---- command line

    fn say(&mut self, s: &str) {
        if self.msg.is_empty() || self.typer.done(&self.msg) && self.queue.is_empty() {
            self.start_msg(s.to_string());
        } else {
            self.queue.push_back(s.to_string());
            while self.queue.len() > MSG_QUEUE_MAX {
                self.queue.pop_front();
            }
        }
    }

    fn start_msg(&mut self, s: String) {
        self.msg = s;
        self.typer = Typer::default();
    }

    fn step_messages(&mut self, dt: f32) {
        let msg = self.msg.clone();
        let sound = self.typer.step(&msg, dt);
        self.play_typing(sound);
        if self.typer.done(&self.msg) {
            if let Some(next) = self.queue.pop_front() {
                self.start_msg(next);
            }
        }
    }

    fn play_pool(&mut self, which: fn(&Sounds) -> &Vec<Sound>) {
        if self.muted || self.sounded {
            return;
        }
        let pool = which(&self.sounds);
        if !pool.is_empty() {
            let k = macroquad::rand::gen_range(0, pool.len() as u32) as usize;
            play_sound(&pool[k], PlaySoundParams { looped: false, volume: VOLUME });
            self.sounded = true;
        }
    }

    fn play_typing(&mut self, s: Option<TypeSound>) {
        match s {
            Some(TypeSound::Click) => self.play_pool(|s| &s.clicks),
            Some(TypeSound::Space) => self.play_pool(|s| &s.spaces),
            None => {}
        }
    }

    // ---- input

    pub fn update(&mut self, dt: f32, mouse: (f32, f32)) -> bool {
        self.t += dt;
        self.sounded = false;
        // A right or wrong digit sounds over the typing.
        let names: Option<String> = match &self.dialog {
            Some(Dialog::New { level, .. } | Dialog::Daily { level, .. }) => Some(LEVELS[*level].name.to_string()),
            Some(Dialog::Theme { scheme, .. }) => Some(self.schemes[*scheme].name.clone()),
            _ => None,
        };
        if let (Some(name), Some(Dialog::New { typer, .. } | Dialog::Theme { typer, .. } | Dialog::Daily { typer, .. })) =
            (names, &mut self.dialog)
        {
            let s = typer.step(&name, dt);
            self.play_typing(s);
        }
        self.poll_requests();
        // Typed characters go to the nickname while it's being entered (and
        // are dropped otherwise, so none pile up).
        while let Some(c) = macroquad::input::get_char_pressed() {
            if let Some(Finish { submit: Submit::Editing(n), .. }) = &mut self.finish {
                if self.dialog.is_some() {
                    if let Some(c) = daily::name_char(c) {
                        n.type_char(c);
                        self.play_typing(Some(TypeSound::Click));
                    }
                }
            }
        }
        self.step_messages(dt);
        if self.dialog.is_none() && !self.solved {
            self.elapsed += dt;
        }
        for f in &mut self.fx {
            f.1 += dt;
        }
        self.fx.retain(|f| f.1 < 0.6);
        if let Some(b) = &mut self.bad {
            b.1 -= dt;
        }
        self.bad = self.bad.filter(|b| b.1 > 0.0);
        if let Some(f) = &mut self.flash {
            f.1 -= dt;
        }
        self.flash = self.flash.filter(|f| f.1 > 0.0);

        let (mx, my) = (mouse.0.floor() as i32, mouse.1.floor() as i32);
        self.hover = self.target_at(mx, my);
        let mut actions = vec![];
        if !macroquad::input::touches().is_empty() {
            self.touch_seen = true;
        }
        if is_mouse_button_pressed(MouseButton::Left) {
            if let Some(a) = self.hover {
                actions.push(a);
                if !matches!(a, Action::Select(_)) {
                    self.flash = Some((a, 0.15));
                }
            } else if self.dialog.is_none() && in_rect(mx, my, self.lay.swipe) {
                // Touches simulate the mouse, so a finger and a mouse drag both swipe.
                self.swipe_start = Some((mouse.0, mouse.1, self.t));
            }
        }
        if is_mouse_button_released(MouseButton::Left) {
            if let Some((x0, y0, t0)) = self.swipe_start.take() {
                let quick = self.t - t0 < SWIPE_TIME;
                if let Some(a) = swipe_action(mouse.0 - x0, mouse.1 - y0).filter(|_| quick && self.dialog.is_none()) {
                    actions.push(a);
                }
            }
        }
        actions.extend(self.keys());
        for a in actions {
            if !self.run(a) {
                return false;
            }
        }
        true
    }

    /// What a tap or click at (mx, my) would do.
    fn target_at(&self, mx: i32, my: i32) -> Option<Action> {
        let at = |p: usize| self.lay.pads[p].hits.iter().position(|&r| in_rect(mx, my, r)).map(|k| k as u8 + 1);
        if let Some(v) = self.view() {
            if let Some(b) = v.buttons.iter().find(|b| in_rect(mx, my, b.hit)) {
                return Some(b.action);
            }
            // The number buttons type the seed in the new-board dialog.
            if matches!(self.dialog, Some(Dialog::New { .. })) {
                return at(0).map(Action::Digit);
            }
            return None;
        }
        if let Some(b) = self.lay.bar.iter().find(|b| in_rect(mx, my, b.hit)) {
            return Some(b.action);
        }
        at(0).map(Action::Num).or_else(|| at(1).map(Action::Note)).or_else(|| self.lay.cell_at(mx, my).map(Action::Select))
    }

    fn keys(&self) -> Vec<Action> {
        use KeyCode::*;
        let digits = [Key1, Key2, Key3, Key4, Key5, Key6, Key7, Key8, Key9];
        let pads = [Kp1, Kp2, Kp3, Kp4, Kp5, Kp6, Kp7, Kp8, Kp9];
        let mut out = vec![];
        let mut map = |keys: &[KeyCode], a: Action| {
            if keys.iter().any(|&k| is_key_pressed(k)) {
                out.push(a);
            }
        };
        match &self.dialog {
            Some(Dialog::New { .. }) => {
                map(&[S], Action::Size);
                map(&[Left, A], Action::LevelPrev);
                map(&[Right, D], Action::LevelNext);
                map(&[Y, Enter, KpEnter, Space], Action::NewYes);
                map(&[N, Escape], Action::NewNo);
                map(&[T], Action::SeedToday);
                map(&[R], Action::SeedRandom);
                map(&[Backspace, Delete], Action::SeedBack);
                map(&[Key0, Kp0], Action::Digit(0));
                for d in 0..9 {
                    map(&[digits[d], pads[d]], Action::Digit(d as u8 + 1));
                }
            }
            Some(Dialog::Theme { .. }) => {
                map(&[Left, Up], Action::ThemePrev);
                map(&[Right, Down], Action::ThemeNext);
                map(&[C], Action::Crt);
                map(&[F], Action::Font);
                map(&[A, Y, Enter, KpEnter, Space], Action::ThemeAccept);
                map(&[E, N, Escape, Backspace], Action::ThemeExit);
            }
            Some(Dialog::Solved) => match self.finish.as_ref().map(|f| &f.submit) {
                // Typing a nickname: letters are the name, so no letter hotkeys.
                Some(Submit::Editing(_)) => {
                    map(&[Up], Action::NameUp);
                    map(&[Down], Action::NameDown);
                    map(&[Left], Action::NameLeft);
                    map(&[Right], Action::NameRight);
                    map(&[Backspace], Action::NameBack);
                    map(&[Enter, KpEnter], Action::NameSubmit);
                    map(&[Escape], Action::NameSkip);
                }
                Some(Submit::Sending(_) | Submit::Failed(_)) => {
                    map(&[Enter, KpEnter], Action::NameSubmit);
                    map(&[Escape], Action::NameSkip);
                }
                _ => {
                    map(&[N, W, Enter, KpEnter], Action::New);
                    map(&[Y], Action::Daily);
                    map(&[C, Escape, Space], Action::SolvedClose);
                }
            },
            Some(Dialog::Daily { .. }) => {
                map(&[Left, A], Action::LevelPrev);
                map(&[Right, D], Action::LevelNext);
                map(&[S], Action::Size);
                map(&[P, Y, Enter, KpEnter, Space], Action::DailyPlay);
                map(&[E, N, Escape, Backspace], Action::DailyExit);
            }
            None => {
                for d in 0..self.shape.n {
                    map(&[digits[d], pads[d]], Action::Digit(d as u8 + 1));
                }
                map(&[Backspace, Delete, Key0, Kp0, E], Action::Erase);
                map(&[N, Space], Action::Notes);
                map(&[F], Action::Fast);
                map(&[A], Action::CycleDigit(-1));
                map(&[D], Action::CycleDigit(1));
                map(&[Enter, KpEnter], Action::Place);
                map(&[U, Z], Action::Undo);
                map(&[H], Action::Hint);
                map(&[T], Action::Theme);
                map(&[W], Action::New);
                map(&[Y], Action::Daily);
                map(&[M], Action::Mute);
                map(&[C], Action::Crt);
                map(&[Left], Action::Move(-1, 0));
                map(&[Right], Action::Move(1, 0));
                map(&[Up], Action::Move(0, -1));
                map(&[Down], Action::Move(0, 1));
                if !cfg!(target_arch = "wasm32") {
                    map(&[Escape], Action::Quit);
                }
            }
        }
        out
    }

    /// Do an action; false to quit.
    fn run(&mut self, a: Action) -> bool {
        match a {
            Action::Quit => return false,
            Action::Mute => {
                self.muted = !self.muted;
                self.say(if self.muted { "SOUND OFF" } else { "SOUND ON" });
            }
            Action::Crt => match &mut self.dialog {
                Some(Dialog::Theme { crt, .. }) => *crt = !*crt,
                _ => {
                    self.crt = !self.crt;
                    self.say(if self.crt { "SCANLINES ON" } else { "SCANLINES OFF" });
                }
            },
            Action::Select(i) => {
                if let (true, Some(d)) = (self.fast, self.fast_digit) {
                    // Fast mode: a tap on an open cell writes the picked digit
                    // (or toggles it as a note). Given cells are just selected.
                    self.sel = Some(i);
                    self.fast_place(i, d);
                } else if self.sel == Some(i) {
                    // Tapping the selected cell again lets it go.
                    self.sel = None;
                } else {
                    self.sel = Some(i);
                }
            }
            Action::Place => {
                if let (true, Some(d), Some(i)) = (self.fast, self.fast_digit, self.sel) {
                    self.fast_place(i, d);
                }
            }
            Action::Fast => {
                self.fast = !self.fast;
                if !self.fast {
                    self.fast_digit = None;
                }
                self.say(if self.fast { "FAST ON: PICK A DIGIT, TAP CELLS" } else { "FAST OFF" });
            }
            Action::Move(dx, dy) => {
                let n = self.shape.n as i32;
                let i = self.sel.unwrap_or(self.shape.cells() / 2) as i32;
                let (r, c) = ((i / n + dy).rem_euclid(n), (i % n + dx).rem_euclid(n));
                self.sel = Some((r * n + c) as usize);
            }
            Action::Digit(d) => {
                if let Some(Dialog::New { seed, .. }) = &mut self.dialog {
                    if seed.len() >= SEED_DIGITS {
                        seed.clear();
                    }
                    seed.push((b'0' + d) as char);
                    self.play_typing(Some(TypeSound::Click));
                } else if d as usize > self.shape.n {
                    // No such digit on a 6x6 board.
                } else if self.fast {
                    // Pick the digit to place. Picking it again switches notes
                    // on or off; a third time puts it down (and the notes back).
                    if self.fast_digit == Some(d) && !self.fast_toggled {
                        self.notes_mode = !self.notes_mode;
                        self.fast_toggled = true;
                        let m = if self.notes_mode { format!("NOTING {d}S") } else { format!("PLACING {d}S") };
                        self.say(&m);
                    } else if self.fast_digit == Some(d) {
                        self.notes_mode = !self.notes_mode;
                        self.fast_toggled = false;
                        self.fast_digit = None;
                        self.say("NO DIGIT PICKED");
                    } else {
                        self.fast_digit = Some(d);
                        self.fast_toggled = false;
                        let m = if self.notes_mode { format!("NOTING {d}S") } else { format!("PLACING {d}S") };
                        self.say(&m);
                    }
                } else if self.sel.is_none() && !self.solved {
                    // A digit with no cell selected: number first, so fast mode.
                    self.fast = true;
                    self.fast_digit = Some(d);
                    self.fast_toggled = false;
                    let m = format!("FAST ON: PLACING {d}S");
                    self.say(&m);
                } else {
                    self.enter(d);
                }
            }
            Action::Erase => self.erase(),
            Action::Notes => {
                self.notes_mode = !self.notes_mode;
                self.say(if self.notes_mode { "NOTES ON" } else { "NOTES OFF" });
            }
            Action::Undo => match self.undo.pop() {
                Some(s) if !self.solved => {
                    self.vals = s.vals;
                    self.notes = s.notes;
                    self.say("UNDONE");
                }
                _ => self.say("NOTHING TO UNDO"),
            },
            Action::Hint => self.hint(),
            Action::Theme => {
                let mut typer = Typer::default();
                typer.t = 0.0;
                self.dialog = Some(Dialog::Theme { scheme: self.scheme, crt: self.crt, font: self.font, typer });
            }
            Action::New => {
                let level = if self.solved { (self.level + 1).min(LEVELS.len() - 1) } else { self.level };
                self.dialog = Some(Dialog::New { level, shape: self.shape, seed: seed_str(today_seed()), typer: Typer::default() });
            }
            Action::LevelPrev | Action::LevelNext => {
                if let Some(Dialog::New { level, typer, .. } | Dialog::Daily { level, typer, .. }) = &mut self.dialog {
                    let n = LEVELS.len();
                    *level = if a == Action::LevelNext { (*level + 1) % n } else { (*level + n - 1) % n };
                    *typer = Typer::default();
                }
                self.fetch_daily_shown();
            }
            Action::CycleDigit(step) => self.cycle_digit(step),
            Action::Num(d) => self.tap_digit(d, false),
            Action::Note(d) => self.tap_digit(d, true),
            Action::PickLevel(k) => {
                if let Some(Dialog::New { level, typer, .. }) = &mut self.dialog {
                    if *level != k {
                        *level = k;
                        *typer = Typer::default();
                    }
                }
            }
            Action::Size => {
                if let Some(Dialog::New { shape, .. } | Dialog::Daily { shape, .. }) = &mut self.dialog {
                    *shape = SHAPES[(shape.idx() + 1) % SHAPES.len()];
                }
                self.fetch_daily_shown();
            }
            Action::Daily => self.open_daily(),
            Action::DailyPlay => {
                if let Some(Dialog::Daily { level, shape, .. }) = self.dialog.take() {
                    self.new_board(shape, level, today_seed());
                }
            }
            Action::DailyExit => self.dialog = None,
            Action::NameUp | Action::NameDown | Action::NameLeft | Action::NameRight | Action::NameBack => {
                if let Some(Finish { submit: Submit::Editing(n), .. }) = &mut self.finish {
                    match a {
                        Action::NameUp => n.cycle(1),
                        Action::NameDown => n.cycle(-1),
                        Action::NameLeft => n.move_cursor(-1),
                        Action::NameRight => n.move_cursor(1),
                        _ => n.backspace(),
                    }
                    self.play_typing(Some(TypeSound::Click));
                }
            }
            Action::NameSubmit => self.submit_time(),
            Action::NameSkip => {
                if let Some(f) = &mut self.finish {
                    if matches!(f.submit, Submit::Editing(_) | Submit::Sending(_) | Submit::Failed(_)) {
                        f.submit = Submit::Skipped;
                    }
                }
            }
            Action::SeedToday => {
                if let Some(Dialog::New { seed, .. }) = &mut self.dialog {
                    *seed = seed_str(today_seed());
                }
            }
            Action::SeedRandom => {
                if let Some(Dialog::New { seed, .. }) = &mut self.dialog {
                    *seed = seed_str(macroquad::rand::gen_range(0, 100_000_000u32));
                }
            }
            Action::SeedBack => {
                if let Some(Dialog::New { seed, .. }) = &mut self.dialog {
                    seed.pop();
                }
            }
            Action::NewYes => {
                if let Some(Dialog::New { level, shape, seed, .. }) = self.dialog.take() {
                    let seed = if seed.is_empty() { today_seed() } else { seed.parse().unwrap_or(0) };
                    self.new_board(shape, level, seed);
                }
            }
            Action::NewNo => self.dialog = None,
            Action::ThemePrev | Action::ThemeNext => {
                let n = self.schemes.len();
                if let Some(Dialog::Theme { scheme, typer, .. }) = &mut self.dialog {
                    *scheme = if a == Action::ThemeNext { (*scheme + 1) % n } else { (*scheme + n - 1) % n };
                    *typer = Typer::default();
                }
            }
            Action::Font => {
                if let Some(Dialog::Theme { font, .. }) = &mut self.dialog {
                    let k = STYLES.iter().position(|s| s == font).unwrap_or(0);
                    *font = STYLES[(k + 1) % STYLES.len()];
                }
            }
            Action::ThemeAccept => {
                if let Some(Dialog::Theme { scheme, crt, font, .. }) = self.dialog.take() {
                    self.scheme = scheme;
                    self.crt = crt;
                    self.font = font;
                    let m = format!("COLOURS {}", self.schemes[scheme].name);
                    self.say(&m);
                }
            }
            Action::ThemeExit | Action::SolvedClose => self.dialog = None,
        }
        true
    }

    /// A tap on the number row (`notes` false) or the notes row: notes mode
    /// follows the row. In fast mode it picks the digit, or switches between
    /// placing and noting it, or (the same button again) puts it down; with no
    /// cell selected it starts fast mode; otherwise it writes in the cell.
    fn tap_digit(&mut self, d: u8, notes: bool) {
        if d as usize > self.shape.n {
            return;
        }
        let switched = self.notes_mode != notes;
        self.notes_mode = notes;
        let verb = if notes { "NOTING" } else { "PLACING" };
        if self.fast {
            self.fast_toggled = false;
            if self.fast_digit == Some(d) && !switched {
                self.fast_digit = None;
                self.say("NO DIGIT PICKED");
            } else {
                self.fast_digit = Some(d);
                let m = format!("{verb} {d}S");
                self.say(&m);
            }
        } else if self.sel.is_none() && !self.solved {
            self.fast = true;
            self.fast_digit = Some(d);
            self.fast_toggled = false;
            let m = format!("FAST ON: {verb} {d}S");
            self.say(&m);
        } else {
            self.enter(d);
        }
    }

    /// Pick the digit `step` along from the current one (wrapping), skipping
    /// digits already all placed, switching fast mode on if it's off. Starts
    /// from the picked digit, else the selected cell's, else the ends.
    fn cycle_digit(&mut self, step: i32) {
        if self.solved {
            return;
        }
        let n = self.shape.n as i32;
        let cur = self.fast_digit.filter(|_| self.fast).unwrap_or(self.sel.map_or(0, |i| self.vals[i])) as i32;
        let base = if cur != 0 { cur } else if step > 0 { 0 } else { n + 1 };
        let Some(e) = (1..=n).map(|k| ((base - 1 + step * k).rem_euclid(n) + 1) as u8).find(|&e| !self.all_placed(e)) else {
            return;
        };
        self.fast = true;
        self.fast_digit = Some(e);
        self.fast_toggled = false;
        let m = if self.notes_mode { format!("NOTING {e}S") } else { format!("PLACING {e}S") };
        self.say(&m);
    }

    /// Fast mode: put the picked digit `d` in cell `i`. Given cells and cells
    /// holding another digit are left alone (erase that digit first); a cell
    /// holding `d` itself is cleared, as tapping a digit twice does.
    fn fast_place(&mut self, i: usize, d: u8) {
        let v = self.vals[i];
        if self.puzzle[i] != 0 {
            return;
        }
        if v != 0 && v != d {
            let m = format!("{} HAS A {v}", cell_name(self.shape, i));
            self.say(&m);
            return;
        }
        self.enter(d);
    }

    fn snapshot(&mut self) {
        self.undo.push(Snapshot { vals: self.vals.clone(), notes: self.notes.clone() });
    }

    /// The selected cell if the player may write in it (or why not).
    fn writable(&mut self) -> Option<usize> {
        if self.solved {
            self.say("SOLVED! TAP NE[W] FOR ANOTHER");
            return None;
        }
        let Some(i) = self.sel else {
            self.say("PICK A CELL FIRST");
            return None;
        };
        if self.puzzle[i] != 0 {
            let m = format!("{} IS GIVEN", cell_name(self.shape, i));
            self.say(&m);
            return None;
        }
        Some(i)
    }

    fn enter(&mut self, d: u8) {
        let Some(i) = self.writable() else { return };
        if self.notes_mode {
            if self.vals[i] != 0 {
                let m = format!("{} HAS A DIGIT", cell_name(self.shape, i));
                self.say(&m);
                return;
            }
            // A new note can't go where the digit visibly can't: already in
            // the cell's row, column or box. (Taking a note off always works.)
            if self.notes[i] & (1 << d) == 0 {
                if let Some(n) = self.clash_unit(i, d) {
                    let m = format!("{d} IS ALREADY IN {}", unit_name(self.shape, n));
                    self.say(&m);
                    self.play_pool(|s| &s.escapes);
                    return;
                }
            }
            self.snapshot();
            self.notes[i] ^= 1 << d;
            self.play_typing(Some(TypeSound::Click));
            return;
        }
        self.snapshot();
        if self.vals[i] == d {
            self.vals[i] = 0;
            let m = format!("{} ERASED", cell_name(self.shape, i));
            self.say(&m);
            return;
        }
        self.vals[i] = d;
        self.notes[i] = 0;
        // Only a clash the player can see counts as an error: the board never
        // gives away that a digit differs from the hidden solution.
        if let Some(n) = self.clash_unit(i, d) {
            self.errors += 1;
            self.bad = Some((i, 0.6));
            let m = format!("{d} IS ALREADY IN {}", unit_name(self.shape, n));
            self.say(&m);
            self.play_pool(|s| &s.escapes);
            return;
        }
        self.placed(i);
        let m = format!("{} = {d}", cell_name(self.shape, i));
        self.say(&m);
    }

    // ---- daily challenge

    /// Open the daily dialog on the current board's level and size, and
    /// fetch that board's leaderboard.
    pub fn open_daily(&mut self) {
        self.dialog = Some(Dialog::Daily { level: self.level, shape: self.shape, typer: Typer::default() });
        self.fetch_daily_shown();
        let m = format!("DAILY CHALLENGE {}: NO HINTS. PICK A BOARD, THEN [P]LAY", today_date());
        self.say(&m);
    }

    /// Show the open dialog's typed name in full (screenshots).
    pub fn finish_dialog_typing(&mut self) {
        let name = match &self.dialog {
            Some(Dialog::New { level, .. } | Dialog::Daily { level, .. }) => LEVELS[*level].name.to_string(),
            Some(Dialog::Theme { scheme, .. }) => self.schemes[*scheme].name.clone(),
            _ => return,
        };
        if let Some(Dialog::New { typer, .. } | Dialog::Daily { typer, .. } | Dialog::Theme { typer, .. }) = &mut self.dialog {
            typer.finish(&name);
        }
    }

    /// Today's board (current size and level) as just solved in `secs`, at the
    /// nickname entry (`--daily-win`, for screenshots only, so no made-up time
    /// can be sent).
    pub fn demo_daily_win(&mut self, secs: f32) {
        self.new_board(self.shape, self.level, today_seed());
        let open: Vec<usize> = (0..self.shape.cells()).filter(|&i| self.vals[i] == 0).collect();
        for &i in &open {
            self.vals[i] = self.solution[i];
        }
        self.elapsed = secs;
        self.placed(open[open.len() - 1]);
        self.fx.clear();
    }

    /// Fetch the leaderboard of the board shown in the daily dialog.
    fn fetch_daily_shown(&mut self) {
        if let Some(Dialog::Daily { level, shape, .. }) = &self.dialog {
            let key = (today_date(), shape.n, *level);
            self.fetch_scores(key);
        }
    }

    /// Start fetching a board's leaderboard (date, size, level).
    fn fetch_scores(&mut self, key: (String, usize, usize)) {
        let s = match &self.leaderboard {
            Some(url) => Scores::Loading(Request::get(&daily::board_url(url, &key.0, key.1, LEVELS[key.2].name, &self.nickname))),
            None => Scores::Failed("OFFLINE".into()),
        };
        self.scores.retain(|(k, _)| *k != key);
        self.scores.push((key, s));
    }

    /// Hand finished leaderboard requests their replies.
    fn poll_requests(&mut self) {
        for (_, s) in &mut self.scores {
            if let Scores::Loading(r) = s {
                if let Some(reply) = r.poll() {
                    *s = match reply.and_then(|t| daily::parse(&t)) {
                        Ok(b) => Scores::Ready(b),
                        Err(m) => Scores::Failed(m),
                    };
                }
            }
        }
        let reply = match &mut self.finish {
            Some(Finish { submit: Submit::Sending(r), .. }) => r.poll(),
            _ => None,
        };
        if let Some(reply) = reply {
            self.submitted(reply);
        }
    }

    /// Wait (up to a few seconds) for requests to finish (screenshots and
    /// tests, which can't wait frame by frame).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn settle_requests(&mut self) {
        for _ in 0..200 {
            self.poll_requests();
            let busy = self.scores.iter().any(|(_, s)| matches!(s, Scores::Loading(_)))
                || matches!(&self.finish, Some(Finish { submit: Submit::Sending(_), .. }));
            if !busy {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    }

    /// Send the demo win's time and wait for the answer (screenshots).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn submit_demo(&mut self) {
        self.submit_time();
        self.settle_requests();
    }
    #[cfg(target_arch = "wasm32")]
    pub fn submit_demo(&mut self) {}

    /// The date of the board being played, as YYYY-MM-DD (its seed).
    fn board_date(&self) -> String {
        fmt_seed(&seed_str(self.seed))
    }

    /// Send the solved daily's time under the typed nickname (or retry).
    fn submit_time(&mut self) {
        let name = match self.finish.as_ref().map(|f| &f.submit) {
            Some(Submit::Editing(n)) => n.text(),
            Some(Submit::Failed(_)) => self.nickname.clone(),
            _ => return,
        };
        if name.is_empty() {
            self.say("TYPE A NAME FIRST");
            return;
        }
        self.nickname = name.clone();
        let body = daily::submit_body(&self.board_date(), self.shape.n, LEVELS[self.level].name, &name, self.finish.as_ref().unwrap().ms, &self.vals);
        let submit = match &self.leaderboard {
            Some(url) => Submit::Sending(Request::post(url, &body)),
            None => Submit::Failed("OFFLINE".into()),
        };
        if let Some(f) = &mut self.finish {
            f.submit = submit;
        }
    }

    /// The server answered a submitted time.
    fn submitted(&mut self, reply: Reply) {
        match reply.and_then(|t| daily::parse(&t)) {
            Ok(b) => {
                let line = result_line(&b);
                // The answer is the latest leaderboard for this board.
                let key = (self.board_date(), self.shape.n, self.level);
                self.scores.retain(|(k, _)| *k != key);
                self.scores.push((key, Scores::Ready(b.clone())));
                if let Some(f) = &mut self.finish {
                    f.submit = Submit::Done(b);
                }
                self.say(&line);
                self.play_pool(|s| &s.enters);
            }
            Err(m) => {
                let line = format!("TIME NOT SENT: {m}");
                if let Some(f) = &mut self.finish {
                    f.submit = Submit::Failed(m);
                }
                self.say(&line);
            }
        }
    }

    /// Leaderboard rows (and a message or footer) for a board, from `y`:
    /// rank, name and time per row, the player's own row in amber.
    fn scores_els(&self, key: &(String, usize, usize), (x, w): (i32, i32), y: i32) -> Vec<El> {
        let cx = self.lay.w / 2;
        let board = self.scores.iter().find(|(k, _)| k == key).map(|(_, s)| s);
        let b = match board {
            Some(Scores::Ready(b)) => b,
            Some(Scores::Failed(m)) => return vec![El::Center(y + 40, m.clone(), UI_DK)],
            _ => return vec![El::Center(y + 40, "LOADING...".into(), UI_DK)],
        };
        let you = self.nickname.as_str();
        let mut rows: Vec<(u32, String, u32)> = b.top.iter().take(SCORE_ROWS - 1).map(|e| (e.rank, e.name.clone(), e.ms)).collect();
        if let Some((rank, ms)) = b.you {
            if !rows.iter().any(|r| r.1 == you) {
                rows.push((rank, you.to_string(), ms));
            } else if b.top.len() >= SCORE_ROWS {
                // The player is in the top nine: the tenth fits too.
                let e = &b.top[SCORE_ROWS - 1];
                rows.push((e.rank, e.name.clone(), e.ms));
            }
        } else if let Some(e) = b.top.get(SCORE_ROWS - 1) {
            rows.push((e.rank, e.name.clone(), e.ms));
        }
        let mut els = vec![];
        if rows.is_empty() {
            els.push(El::Center(y + 40, "NO TIMES YET. BE THE FIRST!".into(), UI_DK));
        }
        let (rank_r, name_x, time_r) = (x + 50, x + 58, x + w - 50);
        for (k, (rank, name, ms)) in rows.iter().enumerate() {
            let ry = y + k as i32 * 10;
            let mine = !you.is_empty() && name == you;
            let (c, dim) = if mine { (AMBER, AMBER) } else { (UI, UI_DK) };
            let rank = format!("{rank}.");
            let time = daily::fmt_ms(*ms);
            els.push(El::Text(rank_r - Fb::text_w(&rank) + 1, ry, rank, dim));
            els.push(El::Text(name_x, ry, name.clone(), c));
            els.push(El::Text(time_r - Fb::text_w(&time) + 1, ry, time, c));
        }
        let footer = match b.count {
            0 => String::new(),
            1 => "1 PLAYER SO FAR".into(),
            n => format!("{n} PLAYERS SO FAR"),
        };
        let _ = cx;
        els.push(El::Center(y + SCORE_ROWS as i32 * 10 + 2, footer, UI_DK));
        els
    }

    /// Open the theme dialog with its scheme name typed out (`--theme-dialog`).
    pub fn open_theme_dialog(&mut self) {
        self.run(Action::Theme);
        let name = self.schemes[self.scheme].name.clone();
        if let Some(Dialog::Theme { typer, .. }) = &mut self.dialog {
            typer.finish(&name);
        }
    }

    /// Open the new-board dialog with its level name typed out (`--new-dialog`).
    pub fn open_new_dialog(&mut self) {
        self.run(Action::New);
        if let Some(Dialog::New { level, typer, .. }) = &mut self.dialog {
            typer.finish(LEVELS[*level].name);
        }
    }

    /// The saved record lines (settings `game=`), oldest first.
    pub fn set_records(&mut self, lines: &[String]) {
        self.records = lines.iter().filter_map(|l| Record::parse(l)).collect();
    }

    /// The current board's record, added (as started) if it has none.
    fn record(&mut self) -> &mut Record {
        let (seed, n, level) = (self.seed, self.shape.n, self.level);
        match self.records.iter().position(|r| (r.seed, r.n, r.level) == (seed, n, level)) {
            Some(k) => &mut self.records[k],
            None => {
                if self.records.len() >= RECORDS_MAX {
                    self.records.remove(0);
                }
                self.records.push(Record { seed, n, level, best: None });
                self.records.last_mut().unwrap()
            }
        }
    }

    /// The table of a seed's boards for the new-board dialog: one row per
    /// level, with its best time, or STARTED, or `-` if never played.
    fn day_rows(&self, seed: u32, n: usize) -> Vec<(String, String, String)> {
        LEVELS
            .iter()
            .enumerate()
            .map(|(k, lv)| {
                let r = self.records.iter().find(|r| (r.seed, r.n, r.level) == (seed, n, k));
                let (time, stats) = match r.map(|r| r.best) {
                    Some(Some((t, e, h))) => (fmt_time(t as f32), format!("ERR {e} HINT {h}")),
                    Some(None) => ("STARTED".into(), String::new()),
                    None => ("-".into(), String::new()),
                };
                (lv.name.to_string(), time, stats)
            })
            .collect()
    }

    /// The first unit (row, column, box) of cell `i` where another cell
    /// already shows digit `d`, if any.
    fn clash_unit(&self, i: usize, d: u8) -> Option<usize> {
        self.shape.units().iter().position(|u| u.contains(&i) && u.iter().any(|&j| j != i && self.vals[j] == d))
    }

    /// Whether the digit in cell `i` repeats in its row, column or box.
    fn conflict(&self, i: usize) -> bool {
        let v = self.vals[i];
        v != 0 && (0..self.shape.cells()).any(|j| self.vals[j] == v && self.shape.peers(i, j))
    }

    /// Whether all n of digit `d` are on the board with no clashes (judged on
    /// what's visible, not the hidden solution).
    fn all_placed(&self, d: u8) -> bool {
        let cells: Vec<usize> = (0..self.shape.cells()).filter(|&j| self.vals[j] == d).collect();
        cells.len() >= self.shape.n && !cells.iter().any(|&j| self.conflict(j))
    }

    /// After a right digit at `i`: clear it from the notes it rules out,
    /// flash finished rows, columns and boxes, and check for the win.
    fn placed(&mut self, i: usize) {
        let d = self.vals[i];
        let n = self.shape.n as u8;
        for j in 0..self.shape.cells() {
            if self.shape.peers(i, j) {
                self.notes[j] &= !(1 << d);
            }
        }
        self.play_pool(|s| &s.enters);
        for (n, u) in self.shape.units().iter().enumerate() {
            // A unit is done when it's full with no repeats (not when it
            // matches the solution, which would give answers away).
            if u.contains(&i) && u.iter().all(|&j| self.vals[j] != 0 && !self.conflict(j)) {
                self.fx.push((u.to_vec(), 0.0));
                let m = format!("{} DONE", unit_name(self.shape, n));
                self.say(&m);
            }
        }
        // Fast mode: once all of the picked digit are right, move on to the
        // next digit (wrapping) that still has cells to fill.
        if self.fast && self.fast_digit == Some(d) && self.all_placed(d) {
            let next = (1..n).map(|k| (d - 1 + k) % n + 1).find(|&e| !self.all_placed(e));
            self.fast_digit = next;
            if let Some(e) = next {
                let m = format!("ALL {d}S PLACED. PLACING {e}S");
                self.say(&m);
            }
        }
        if self.vals == self.solution {
            self.solved = true;
            self.notes_mode = false;
            self.dialog = Some(Dialog::Solved);
            // Today's boards go on the daily leaderboard (if no hints were used).
            self.finish = self.is_daily().then(|| Finish {
                ms: (self.elapsed * 1000.0) as u32,
                submit: if self.hints == 0 { Submit::Editing(NameEdit::new(&self.nickname)) } else { Submit::Unranked("HINTS USED: NOT RANKED") },
            });
            self.fx.push(((0..self.shape.cells()).collect(), 0.0));
            let m = format!("SOLVED IN {}", fmt_time(self.elapsed));
            self.say(&m);
            // Keep the time if it beats this board's best.
            let run = (self.elapsed as u32, self.errors, self.hints);
            let r = self.record();
            let before = r.best;
            if before.map_or(true, |(t, _, _)| run.0 < t) {
                r.best = Some(run);
                if before.is_some() {
                    self.say("NEW BEST TIME");
                }
            }
        }
    }

    fn erase(&mut self) {
        let Some(i) = self.writable() else { return };
        if self.vals[i] == 0 && self.notes[i] == 0 {
            return;
        }
        self.snapshot();
        self.vals[i] = 0;
        self.notes[i] = 0;
        let m = format!("{} ERASED", cell_name(self.shape, i));
        self.say(&m);
    }

    /// Point out a wrong digit, else place one digit that logic forces
    /// (in the selected cell if it can), saying why; else reveal one.
    fn hint(&mut self) {
        if self.solved {
            return;
        }
        if self.is_daily() {
            self.say("NO HINTS IN THE DAILY CHALLENGE");
            return;
        }
        let cells = self.shape.cells();
        if let Some(i) = (0..cells).find(|&i| self.vals[i] != 0 && self.vals[i] != self.solution[i]) {
            self.snapshot();
            self.vals[i] = 0;
            self.sel = Some(i);
            self.hints += 1;
            let m = format!("HINT: {} WAS WRONG. ERASED", cell_name(self.shape, i));
            self.say(&m);
            return;
        }
        let cand = sudoku::candidates(self.shape, &self.vals);
        let step = self
            .sel
            .filter(|&i| self.vals[i] == 0 && cand[i].count_ones() == 1)
            .map(|i| sudoku::Step { cell: i, digit: self.solution[i], reason: Reason::Naked })
            .or_else(|| sudoku::find_single(self.shape, &self.vals, true));
        let (i, why) = match step {
            Some(s) => (
                s.cell,
                match s.reason {
                    Reason::Naked => ": NOTHING ELSE FITS".to_string(),
                    Reason::Hidden(n) => format!(": ONLY SPOT IN {}", unit_name(self.shape, n)),
                },
            ),
            None => {
                // No single: reveal the selected cell, or the tightest one.
                let i = self.sel.filter(|&i| self.vals[i] == 0).unwrap_or_else(|| {
                    (0..cells).filter(|&i| self.vals[i] == 0).min_by_key(|&i| cand[i].count_ones()).unwrap_or(0)
                });
                (i, String::new())
            }
        };
        self.snapshot();
        self.vals[i] = self.solution[i];
        self.notes[i] = 0;
        self.sel = Some(i);
        self.hints += 1;
        let m = format!("HINT {} = {}{why}", cell_name(self.shape, i), self.vals[i]);
        self.say(&m);
        self.placed(i);
    }

    // ---- dialogs

    fn view(&self) -> Option<View> {
        let d = self.dialog.as_ref()?;
        let (w, h) = (self.lay.w, self.lay.h);
        let cx = w / 2;
        let make = |dh: i32| {
            let x = (w - DIALOG_W) / 2;
            let y = if self.lay.portrait { (self.lay.gy + (self.lay.geom.h() - dh) / 2).max(self.lay.hud_rule + 4) } else { (h - dh) / 2 };
            (x, y, DIALOG_W, dh)
        };
        let typed = |y: i32, name: &str, typer: &Typer| {
            let shown: String = name.chars().take(typer.typed).collect();
            let x = cx - (Fb::text_w(name) - 1) / 2;
            let mut v = vec![El::Text(x, y, shown.clone(), AMBER)];
            if !typer.done(name) {
                v.push(El::Text(x + Fb::text_w(&shown), y, "█".into(), AMBER));
            }
            v
        };
        let arrows = |y: i32, prev: Action, next: Action| {
            vec![
                // Wide hit areas: from the dialog's edge to near the name.
                Button { label: "<", action: prev, text_x: cx - 72, text_y: y, hit: (cx - DIALOG_W / 2, y - 9, DIALOG_W / 2 - 40, 25) },
                Button { label: ">", action: next, text_x: cx + 68, text_y: y, hit: (cx + 40, y - 9, DIALOG_W / 2 - 40, 25) },
            ]
        };
        Some(match d {
            Dialog::New { level, shape, seed, typer } => {
                let r @ (x, y, _, _) = make(210);
                let lv = &LEVELS[*level];
                let mut els = typed(y + 30, lv.name, typer);
                els.push(El::Center(y + 42, lv.blurb[shape.idx()].to_string(), UI_DK));
                // SIZE row: the [S]IZE label is a button; its value here.
                let lx = cx - 30;
                els.push(El::Text(lx + 42, y + 58, shape.name(), AMBER));
                let padded = format!("{seed}{}", "_".repeat(SEED_DIGITS - seed.len().min(SEED_DIGITS)));
                let shown = fmt_seed(&padded);
                let sx = cx - (Fb::text_w(&format!("SEED {shown}")) - 1) / 2;
                els.push(El::Text(sx, y + 74, "SEED".into(), UI_DK));
                els.push(El::Text(sx + 30, y + 74, shown, AMBER));
                if *seed == seed_str(today_seed()) {
                    els.push(El::Text(x + DIALOG_W - 12 - Fb::text_w("DAILY") + 1, y + 74, "DAILY".into(), UI_DK));
                }
                let mut buttons = arrows(y + 30, Action::LevelPrev, Action::LevelNext);
                buttons.push(Button { label: "[S]IZE", action: Action::Size, text_x: lx, text_y: y + 58, hit: (lx - 40, y + 52, 120, 17) });
                buttons.extend(row_buttons(&[("[T]ODAY", Action::SeedToday), ("[R]ANDOM", Action::SeedRandom)], cx, y + 92));
                // This seed's boards in this size: best times, by level.
                let ty = y + 110;
                let (nx, tx, stx) = (x + 18, x + 84, x + 144);
                els.push(El::Text(x + 12, ty, format!("{0} BEST TIMES {0}", "-".repeat(13)), UI_DK));
                match seed.parse::<u32>().ok().filter(|_| seed.len() == SEED_DIGITS) {
                    Some(sd) => {
                        for (k, (name, time, stats)) in self.day_rows(sd, shape.n).into_iter().enumerate() {
                            let ry = ty + 10 + k as i32 * 10;
                            let picked = k == *level;
                            let played = time != "-";
                            let c = if picked { AMBER } else if played { UI } else { UI_DK };
                            els.push(El::Text(nx, ry, name, if picked { AMBER } else { UI_DK }));
                            els.push(El::Text(tx, ry, time, c));
                            els.push(El::Text(stx, ry, stats, UI_DK));
                            // The whole row picks its level (an unlabelled button).
                            let action = Action::PickLevel(k);
                            buttons.push(Button { label: "", action, text_x: nx, text_y: ry, hit: (x + 8, ry - 2, DIALOG_W - 16, 10) });
                        }
                    }
                    None => els.push(El::Center(ty + 30, "TYPE ALL 8 SEED DIGITS".into(), UI_DK)),
                }
                buttons.extend(row_buttons(&[("[Y]ES", Action::NewYes), ("[N]O", Action::NewNo)], cx, y + 186));
                View { rect: r, title: "NEW BOARD?", count: Some(format!("{}/{}", level + 1, LEVELS.len())), els, buttons }
            }
            Dialog::Theme { scheme, crt, font, typer } => {
                let r @ (_, y, _, _) = make(120);
                let mut els = typed(y + 32, &self.schemes[*scheme].name, typer);
                // [C]RT and [F]ONT rows: the labels are buttons; their values here.
                let lx = cx - 42;
                els.push(El::Text(lx + 42, y + 50, if *crt { "ON".into() } else { "OFF".into() }, AMBER));
                els.push(El::Text(lx + 42, y + 66, font.name().into(), AMBER));
                let mut buttons = arrows(y + 32, Action::ThemePrev, Action::ThemeNext);
                buttons.push(Button { label: "[C]RT", action: Action::Crt, text_x: lx, text_y: y + 50, hit: (lx - 30, y + 44, 130, 16) });
                buttons.push(Button { label: "[F]ONT", action: Action::Font, text_x: lx, text_y: y + 66, hit: (lx - 30, y + 60, 130, 16) });
                buttons.extend(row_buttons(&[("[A]CCEPT", Action::ThemeAccept), ("[E]XIT", Action::ThemeExit)], cx, y + 96));
                View { rect: r, title: "THEME", count: Some(format!("{}/{}", scheme + 1, self.schemes.len())), els, buttons }
            }
            Dialog::Daily { level, shape, typer } => {
                let r @ (x, y, w, _) = make(230);
                let lv = &LEVELS[*level];
                let mut els = typed(y + 30, lv.name, typer);
                let lx = cx - 30;
                els.push(El::Text(lx + 42, y + 46, shape.name(), AMBER));
                els.push(El::Center(y + 62, format!("{}: NO HINTS", today_date()), UI_DK));
                els.push(El::Text(x + 12, y + 74, format!("{0} TOP TEN {0}", "-".repeat(14)), UI_DK));
                els.extend(self.scores_els(&(today_date(), shape.n, *level), (x, w), y + 86));
                let mut buttons = arrows(y + 30, Action::LevelPrev, Action::LevelNext);
                buttons.push(Button { label: "[S]IZE", action: Action::Size, text_x: lx, text_y: y + 46, hit: (lx - 40, y + 40, 120, 16) });
                buttons.extend(row_buttons(&[("[P]LAY", Action::DailyPlay), ("[E]XIT", Action::DailyExit)], cx, y + 206));
                View { rect: r, title: "DAILY CHALLENGE", count: Some(format!("{}/{}", level + 1, LEVELS.len())), els, buttons }
            }
            Dialog::Solved => {
                let done_buttons = || {
                    let mut b = vec![("[N]EW", Action::New)];
                    if self.finish.is_some() {
                        b.push(("DAIL[Y]", Action::Daily));
                    }
                    b.push(("[C]LOSE", Action::SolvedClose));
                    b
                };
                let Some(f) = &self.finish else {
                    let r @ (_, y, _, _) = make(100);
                    let els = vec![
                        El::Center(y + 30, self.board_name(), UI),
                        El::Center(y + 42, format!("TIME {}", fmt_time(self.elapsed)), AMBER),
                        El::Center(y + 54, format!("ERRORS {} | HINTS {}", self.errors, self.hints), UI),
                    ];
                    let buttons = row_buttons(&done_buttons(), cx, y + 78);
                    return Some(View { rect: r, title: "SOLVED!", count: None, els, buttons });
                };
                let h = match &f.submit {
                    Submit::Editing(_) => 150,
                    Submit::Done(_) => 240,
                    _ => 120,
                };
                let r @ (x, y, w, _) = make(h);
                let mut els = vec![
                    El::Center(y + 30, self.board_name(), UI),
                    El::Center(y + 42, format!("TIME {}", daily::fmt_ms(f.ms)), AMBER),
                    El::Center(y + 54, format!("ERRORS {} | HINTS {}", self.errors, self.hints), UI_DK),
                ];
                let mut buttons = vec![];
                match &f.submit {
                    Submit::Editing(n) => {
                        els.push(El::Center(y + 70, "ENTER A NAME FOR THE LEADERBOARD".into(), UI));
                        // Eight slots, the cursor's blinking in amber.
                        let (nx, ny) = (cx - 40, y + 86);
                        let blink = ((self.t * 2.5) as i32) % 2 == 0;
                        for k in 0..daily::NAME_MAX {
                            let sx = nx + k as i32 * 10;
                            let c = n.chars.get(k).copied();
                            if k == n.cursor && blink {
                                els.push(El::Rect(sx - 1, ny - 2, 9, 11, AMBER));
                                els.push(El::Text(sx + 1, ny, c.unwrap_or(' ').to_string(), BG));
                            } else {
                                match c {
                                    Some(c) => els.push(El::Text(sx + 1, ny, c.to_string(), AMBER)),
                                    None => els.push(El::Text(sx + 1, ny, "_".into(), UI_DK)),
                                }
                            }
                        }
                        // ^ / ⌄ change the letter, < / > move: for touch screens.
                        for (k, (label, action)) in
                            [("^", Action::NameUp), ("⌄", Action::NameDown), ("<", Action::NameLeft), (">", Action::NameRight)].into_iter().enumerate()
                        {
                            let bx = cx - 66 + k as i32 * 36;
                            buttons.push(Button { label, action, text_x: bx + 14, text_y: y + 104, hit: (bx, y + 98, 34, 19) });
                        }
                        buttons.extend(row_buttons(&[("[ENTER] SEND", Action::NameSubmit), ("[ESC] SKIP", Action::NameSkip)], cx, y + 126));
                    }
                    Submit::Sending(_) => {
                        els.push(El::Center(y + 70, "SENDING YOUR TIME...".into(), UI));
                        buttons = row_buttons(&[("[ESC] SKIP", Action::NameSkip)], cx, y + 96);
                    }
                    Submit::Failed(m) => {
                        els.push(El::Center(y + 70, format!("NOT SENT: {m}"), RED));
                        buttons = row_buttons(&[("[ENTER] RETRY", Action::NameSubmit), ("[ESC] SKIP", Action::NameSkip)], cx, y + 96);
                    }
                    Submit::Skipped => {
                        els.push(El::Center(y + 70, "TIME NOT SENT".into(), UI_DK));
                        buttons = row_buttons(&done_buttons(), cx, y + 96);
                    }
                    Submit::Unranked(m) => {
                        els.push(El::Center(y + 70, m.to_string(), UI_DK));
                        buttons = row_buttons(&done_buttons(), cx, y + 96);
                    }
                    Submit::Done(b) => {
                        els.push(El::Center(y + 70, result_line(b), AMBER));
                        els.extend(self.scores_els(&(self.board_date(), self.shape.n, self.level), (x, w), y + 86));
                        buttons = row_buttons(&done_buttons(), cx, y + 214);
                    }
                }
                View { rect: r, title: "SOLVED!", count: None, els, buttons }
            }
        })
    }

    // ---- drawing

    fn pressed(&self, a: Action) -> bool {
        self.flash.is_some_and(|(f, _)| f == a)
    }

    pub fn render(&self, fb: &mut Fb) {
        let l = &self.lay;
        fb.font = self.font_now();
        fb.clear(BG);

        // Dot lattice on the "desk", as in rusty-majong.
        let (lx0, ly0, lx1, ly1) = l.lattice;
        for (r, y) in (ly0..ly1).step_by(5).enumerate() {
            let off = if r % 2 == 1 { 5 } else { 0 };
            for x in (off..lx1).step_by(10).filter(|&x| x >= lx0) {
                fb.pset(x, y, DIM);
            }
        }
        fb.rect(l.gx, l.gy, l.geom.w(), l.geom.h(), BG);
        self.render_board(fb);
        self.render_pads(fb);
        self.render_hud(fb);
        // On touch screens, a legend for the swipe pad.
        if self.touch_seen && self.dialog.is_none() {
            let (sx, sy, sw, sh) = l.swipe;
            let legend = if l.portrait { "SWIPE: < > DIGIT  ^ NOTES  ⌄ UNDO" } else { "< > DIGIT  ^ NOTES  ⌄ UNDO" };
            let ly = if l.portrait { sy + (l.prompt.1 - 8 - sy) / 2 } else { sy + (sh - 7) / 2 - 6 };
            fb.text_center(sx + sw / 2, ly, legend, UI_DK, UI_DK);
        }
        if let Some(v) = self.view() {
            draw_view(fb, &v, self.hover, l.w);
        }
    }

    fn render_board(&self, fb: &mut Fb) {
        let l = &self.lay;
        // Fast mode doesn't mark the selected cell or its row, column and
        // box: the picked digit is what matters there.
        let sel = if self.fast { None } else { self.sel };
        let sel_digit = self.focus_digit();
        let blink = ((self.t * 10.0) as i32) % 2 == 0;
        let g = l.geom;
        let note_rows = (g.s.n / 3) as i32;
        for i in 0..g.s.cells() {
            let (x, y, w, h) = l.cell_rect(i);
            let v = self.vals[i];
            // Red only for digits that visibly clash with another in their
            // row, column or box.
            let wrong = self.conflict(i);
            let is_sel = sel == Some(i);
            // Every other cell holding the focused digit gets a dark amber fill.
            let same = !is_sel && v != 0 && v == sel_digit;
            let fx = self.fx.iter().filter(|f| f.0.contains(&i)).map(|f| f.1).fold(None, |a: Option<f32>, t| Some(a.map_or(t, |a| a.min(t))));
            // Background: selection, same digit, finished units, the
            // selection's row / column / box.
            if is_sel {
                fb.rect(x, y, w, h, if wrong { RED } else { AMBER });
            } else if let Some(t) = fx {
                if t < 0.2 {
                    fb.rect(x, y, w, h, AMBER);
                } else {
                    fb.dither(x, y, w, h, DKAMBER);
                }
            } else if same {
                fb.rect(x, y, w, h, DKAMBER);
            } else if sel.is_some_and(|s| g.s.peers(s, i)) {
                fb.dither(x, y, w, h, DIM);
            }
            if !is_sel && self.hover == Some(Action::Select(i)) {
                fb.outline(x, y, w, h, UI_DK);
            }
            let flashing = fx.is_some_and(|t| t < 0.2);
            if v != 0 {
                let c = if is_sel || flashing {
                    BG
                } else if wrong {
                    if self.bad.is_some_and(|b| b.0 == i) && !blink { DKRED } else { RED }
                } else if self.puzzle[i] != 0 {
                    // Givens stay green, highlighted or not.
                    UI
                } else if v == sel_digit {
                    // The player's digits turn amber when highlighted.
                    AMBER
                } else {
                    BLUE
                };
                let k = g.scale;
                fb.text_k(x + (w - 5 * k + 1) / 2, y + (h - 7 * k) / 2, &v.to_string(), c, k);
            } else if self.notes[i] != 0 {
                // Notes sit three to a row, n / 3 rows, in digit order.
                let (sw, sh) = (w / 3, h / note_rows);
                for d in 1..=g.s.n as u8 {
                    if self.notes[i] & (1 << d) != 0 {
                        let k = (d - 1) as i32;
                        let (nx, ny) = (x + (k % 3) * sw, y + (k / 3) * sh);
                        // A note of the focused digit gets the same dark amber
                        // fill as its placed digits, over its own 1/9 (or 1/6)
                        // of the cell.
                        let focus = !is_sel && d == sel_digit;
                        if focus {
                            fb.rect(nx, ny, sw, sh, DKAMBER);
                        }
                        let c = if is_sel { BG } else if focus { AMBER } else { UI };
                        fb.tiny(nx + (sw - 3) / 2, ny + (sh - 5) / 2, &d.to_string(), c);
                    }
                }
            }
        }
        for (k, line) in l.grid_text.iter().enumerate() {
            fb.boxes(l.gx, l.gy + k as i32 * LH, line, UI, UI_DK);
        }
    }

    /// The digit highlighted across the board: the picked one in fast mode,
    /// else the selected cell's.
    fn focus_digit(&self) -> u8 {
        if self.fast {
            self.fast_digit.unwrap_or(0)
        } else {
            self.sel.map_or(0, |i| self.vals[i])
        }
    }

    /// The number and notes rows. Each button previews what tapping it puts on
    /// the board: a big digit, or a small one where that note goes in a cell.
    /// The row in use (notes mode or not) has bright double lines. The focused
    /// digit (picked in fast mode, else the selected cell's) shows in its row
    /// as on the board: amber on dark amber, outlined in amber. Finished digits
    /// are dimmed; how many of each are left shows in the number's corner.
    /// While the new-board dialog is open the number row types seed digits.
    fn render_pads(&self, fb: &mut Fb) {
        let l = &self.lay;
        let focus = self.focus_digit();
        let typing_seed = matches!(self.dialog, Some(Dialog::New { .. }));
        let note_rows = (self.shape.n / 3) as i32;
        // The row the focused digit shows in: the notes row only for a digit
        // picked for noting in fast mode.
        let focus_row = if self.fast && self.notes_mode { 1 } else { 0 };
        for (p, pad) in l.pads.iter().enumerate() {
            let is_notes = p == 1;
            let active = if typing_seed { !is_notes } else { self.notes_mode == is_notes };
            let rows = pad.text.len() as i32;
            fb.rect(pad.at.0, pad.at.1, pad.text[0].chars().count() as i32 * CW, rows * LH, BG);
            for (k, &(x, y)) in pad.cells.iter().enumerate() {
                let d = k as u8 + 1;
                let a = if typing_seed { Action::Digit(d) } else if is_notes { Action::Note(d) } else { Action::Num(d) };
                let placed = self.vals.iter().filter(|&&v| v == d).count();
                let left = self.shape.n.saturating_sub(placed);
                let (ix, iy, iw, ih) = (x + 4, y + 6, 27, 27);
                let is_focus = !typing_seed && p == focus_row && d == focus;
                let done = !typing_seed && left == 0;
                let flash = typing_seed && !is_notes && self.pressed(a);
                if flash {
                    fb.rect(ix, iy, iw, ih, AMBER);
                } else if is_focus && !is_notes {
                    fb.rect(ix, iy, iw, ih, DKAMBER);
                } else if !is_focus && self.hover == Some(a) && !(typing_seed && is_notes) {
                    fb.dither(ix, iy, iw, ih, UI_DK);
                }
                if is_notes {
                    // The note's own slot, as in a board cell.
                    let (sw, sh) = (iw / 3, ih / note_rows);
                    let kk = k as i32;
                    let (nx, ny) = (ix + (kk % 3) * sw, iy + (kk / 3) * sh);
                    if is_focus {
                        fb.rect(nx, ny, sw, sh, DKAMBER);
                    }
                    let c = if is_focus { AMBER } else if done || typing_seed { DIM } else { UI };
                    fb.tiny(nx + (sw - 3) / 2, ny + (sh - 5) / 2, &d.to_string(), c);
                } else {
                    let c = if flash {
                        BG
                    } else if is_focus {
                        AMBER
                    } else if done {
                        DIM
                    } else {
                        UI
                    };
                    fb.text2x(ix + (iw - 9) / 2, iy + (ih - 14) / 2, &d.to_string(), c);
                    if !done && !typing_seed {
                        fb.tiny(ix + 2, iy + 2, &left.to_string(), UI_DK);
                    }
                }
            }
            let major = if active { UI } else { UI_DK };
            for (j, line) in pad.text.iter().enumerate() {
                fb.boxes(pad.at.0, pad.at.1 + j as i32 * LH, line, major, UI_DK);
            }
            // The focused button's outline, over the lines.
            if let Some(&(x, y)) = (focus as usize).checked_sub(1).and_then(|k| pad.cells.get(k)) {
                if !typing_seed && p == focus_row {
                    fb.outline(x + 3, y + 5, 29, 29, AMBER);
                }
            }
        }
    }

    fn render_hud(&self, fb: &mut Fb) {
        let l = &self.lay;
        let w = l.w;
        let x = fb.text(4, 4, "RUSTY-SUDOKU", AMBER);
        let x = fb.text(x + 6, 4, &version(), UI_DK);
        let time = format!("TIME {}", fmt_time(self.elapsed));
        let time_x = w - 4 - Fb::text_w(&time) + 1;
        let left = self.vals.iter().filter(|&&d| d == 0).count();
        let scheme = format!("/ {}", self.schemes[self.scheme].name);
        let name = self.board_name();
        let short = name.strip_prefix("DAILY ").unwrap_or(&name).to_string();
        if l.portrait {
            if x + 6 + Fb::text_w(&scheme) + 8 <= time_x {
                fb.text(x + 6, 4, &scheme, UI_DK);
            }
            fb.text(time_x, 4, &time, UI);
            let mut nx = 4;
            if self.is_daily() {
                nx = fb.text(4, 15, "DAILY", AMBER) + 6;
            }
            fb.text(nx, 15, &short, UI_DK);
            let stats = format!("ERR {} | LEFT {left}", self.errors);
            fb.text(w - 4 - Fb::text_w(&stats) + 1, 15, &stats, UI);
        } else {
            // One line: title, the board's name (and the colours, if they
            // fit), then the stats and time on the right.
            let stats = format!("ERR {} | LEFT {left} | {time}", self.errors);
            let stats_x = w - 4 - Fb::text_w(&stats) + 1;
            let mut nx = x + 12;
            if self.is_daily() {
                nx = fb.text(nx, 4, "DAILY", AMBER) + 6;
            }
            let nx = fb.text(nx, 4, &short, UI_DK);
            if nx + 6 + Fb::text_w(&scheme) + 12 <= stats_x {
                fb.text(nx + 6, 4, &scheme, UI_DK);
            }
            fb.text(stats_x, 4, &stats, UI);
        }
        dashed(fb, l.hud_rule, 0, w);

        // Command line with typewriter reveal and a blinking block cursor.
        let (px, py) = l.prompt;
        let line: String = self.msg.chars().take(self.typer.typed).collect();
        let x = fb.text(px, py, ">", AMBER);
        let x = fb.text_keys(x + 6, py, &line, UI, AMBER);
        if ((self.t * 2.5) as i32) % 2 == 0 {
            fb.text(x, py, "█", UI);
        }

        let (ry, rx0, rx1) = l.rule;
        dashed(fb, ry, rx0, rx1);
        let modal = self.dialog.is_some();
        for b in &l.bar {
            let on = (b.action == Action::Notes && self.notes_mode) || (b.action == Action::Fast && self.fast);
            // MUTE shows in red while the sound is off; HINT is dimmed on the
            // daily boards, which have none.
            let off = b.action == Action::Hint && self.is_daily();
            let normal = if modal || off { DIM } else if b.action == Action::Mute && self.muted { RED } else { UI_DK };
            let hover = !modal && self.hover == Some(b.action);
            draw_button(fb, b.label, b.text_x, b.text_y, normal, hover, on || self.pressed(b.action));
        }
    }
}

/// A dialog: drop shadow, a double-line box drawing frame, the title, the
/// contents and the buttons.
fn draw_view(fb: &mut Fb, v: &View, hover: Option<Action>, screen_w: i32) {
    let (x, y, w, h) = v.rect;
    fb.rect(x + 6, y + 4, w, h, UI_DK);
    fb.rect(x, y, w, h, BG);
    let cols = (w / CW) as usize;
    let rows = (h / LH) as usize;
    for r in 0..rows {
        let s = if r == 0 {
            format!("╔{}╗", "═".repeat(cols - 2))
        } else if r == rows - 1 {
            format!("╚{}╝", "═".repeat(cols - 2))
        } else {
            format!("║{}║", " ".repeat(cols - 2))
        };
        fb.boxes(x, y + r as i32 * LH, &s, UI, UI);
    }
    let cx = screen_w / 2;
    // Unlabelled buttons (whole rows) show hover as shading under their text.
    for b in v.buttons.iter().filter(|b| b.label.is_empty() && hover == Some(b.action)) {
        let (hx, hy, hw, hh) = b.hit;
        fb.dither(hx, hy, hw, hh, UI_DK);
    }
    fb.text_center(cx, y + 13, v.title, AMBER, AMBER);
    if let Some(c) = &v.count {
        fb.text(x + w - 12 - Fb::text_w(c) + 1, y + 13, c, UI_DK);
    }
    for e in &v.els {
        if let El::Rect(rx, ry, rw, rh, c) = e {
            fb.rect(*rx, *ry, *rw, *rh, *c);
        }
    }
    for e in &v.els {
        match e {
            El::Text(tx, ty, s, c) => {
                fb.text(*tx, *ty, s, *c);
            }
            El::Center(ty, s, c) => fb.text_center(cx, *ty, s, *c, AMBER),
            El::Rect(..) => {}
        }
    }
    for b in v.buttons.iter().filter(|b| !b.label.is_empty()) {
        let normal = if matches!(b.action, Action::Crt | Action::Font | Action::Size | Action::SeedToday | Action::SeedRandom) { UI_DK } else { UI };
        draw_button(fb, b.label, b.text_x, b.text_y, normal, hover == Some(b.action), false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_text_fits_and_uses_known_characters() {
        for (s, cols, rows) in [(NINE, 46, 28), (sudoku::SIX, 43, 25)] {
            let g = grid_lines(Geom::of(s));
            assert_eq!(g.len(), rows);
            for line in &g {
                assert_eq!(line.chars().count(), cols);
                for ch in line.chars() {
                    assert!(ch == ' ' || crate::font::boxch(ch).is_some(), "no glyph for {ch:?}");
                }
            }
        }
        let g = grid_lines(Geom::of(NINE));
        assert!(g[0].starts_with("╔════╤════╤════╦"));
        assert!(g[3].starts_with("╟────┼────┼────╫"));
        assert!(g[9].starts_with("╠════╪════╪════╬"));
        assert!(g[27].ends_with("╧════╝"));
        // 6x6: boxes two rows by three columns.
        let g = grid_lines(Geom::of(sudoku::SIX));
        assert_eq!(g[0], "╔══════╤══════╤══════╦══════╤══════╤══════╗");
        assert!(g[4].starts_with("╟──────┼──────┼──────╫"));
        assert!(g[8].starts_with("╠══════╪══════╪══════╬"));
        assert!(g[24].starts_with("╚══════╧"));
    }

    #[test]
    fn layouts_fit_their_screens() {
        for (w, h, s) in [(315, 560), (315, 700), (400, 560), (560, 315), (700, 315), (560, 420)]
            .into_iter()
            .flat_map(|(w, h)| SHAPES.map(|s| (w, h, s)))
        {
            let l = Layout::new(w, h, s);
            let (gw, gh) = (l.geom.w(), l.geom.h());
            assert_eq!(l.pads[0].cells.len(), s.n);
            assert_eq!(l.pads[1].cells.len(), s.n);
            assert!(l.gx >= 0 && l.gx + gw <= w, "{w}x{h} board x");
            assert!(l.gy > l.hud_rule && l.gy + gh <= h, "{w}x{h} board y");
            for pad in &l.pads {
                let (dx, dy) = *pad.cells.last().unwrap();
                assert!(dx + 36 <= w && dy + 40 < l.prompt.1, "{w}x{h} digits");
                // Buttons are square, and a tap in the middle hits the right one
                // (and nothing in the other row).
                for (k, &(x, y, hw, hh)) in pad.hits.iter().enumerate() {
                    assert_eq!(hw, hh);
                    let mid = (x + hw / 2, y + hh / 2);
                    let hits: Vec<usize> = l.pads.iter().flat_map(|p| p.hits.iter().enumerate().filter(|(_, &r)| in_rect(mid.0, mid.1, r)).map(|(k, _)| k)).collect();
                    assert_eq!(hits, vec![k]);
                }
            }
            let dy = l.pads[0].cells[0].1;
            assert!(l.prompt.1 + 10 < l.rule.0, "{w}x{h} prompt");
            if l.portrait {
                assert!(dy > l.gy + gh);
            } else {
                assert!(l.pads[0].cells[0].0 > l.gx + gw);
            }
            for i in [0, s.cells() / 2, s.cells() - 1] {
                let (x, y, cw, ch) = l.cell_rect(i);
                assert_eq!(l.cell_at(x + cw / 2, y + ch / 2), Some(i));
            }
        }
    }

    #[test]
    fn screen_sizes_keep_the_base_and_grow() {
        assert_eq!(screen_size(1080.0, 1920.0), (315, 560));
        assert_eq!(screen_size(1080.0, 2400.0), (315, 700));
        assert_eq!(screen_size(1920.0, 1080.0), (560, 315));
        assert_eq!(screen_size(1024.0, 768.0), (560, 420));
    }

    #[test]
    fn fast_mode_moves_on_when_a_digit_is_done() {
        let mut g = Game::new(vec![crate::palette::builtin()], 0);
        g.new_board(NINE, 0, 20261121);
        g.fast = true;
        // Pick a digit with an open cell and fill all but its last one.
        let d = (1..=9u8).find(|&d| !g.all_placed(d)).unwrap();
        let open: Vec<usize> = (0..g.shape.cells()).filter(|&i| g.solution[i] == d && g.vals[i] == 0).collect();
        let (&last, rest) = open.split_last().unwrap();
        for &i in rest {
            g.vals[i] = d;
        }
        g.fast_digit = Some(d);
        g.run(Action::Select(last));
        assert_eq!(g.vals[last], d);
        let next = g.fast_digit.expect("moved to another digit");
        assert_ne!(next, d);
        assert!(!g.all_placed(next));
        // Every digit between d and next (wrapping) was already done.
        let mut e = d % 9 + 1;
        while e != next {
            assert!(g.all_placed(e));
            e = e % 9 + 1;
        }
    }

    #[test]
    fn six_by_six_plays() {
        let mut g = Game::new(vec![crate::palette::builtin()], 0);
        g.new_board(sudoku::SIX, 0, 20261122);
        assert_eq!(g.lay.pads[0].cells.len(), 6);
        let open: Vec<usize> = (0..36).filter(|&i| g.puzzle[i] == 0).collect();
        // Tapping a cell's middle selects it.
        let (x, y, w, h) = g.lay.cell_rect(open[0]);
        assert_eq!(g.target_at(x + w / 2, y + h / 2), Some(Action::Select(open[0])));
        g.run(Action::Select(open[0]));
        // There is no 7 on a 6x6 board.
        g.run(Action::Digit(7));
        assert_eq!(g.vals[open[0]], 0);
        // Let it go (tapping it again would deselect it below).
        g.run(Action::Select(open[0]));
        assert_eq!(g.sel, None);
        // Fill every open cell: the board is solved.
        for &i in &open {
            g.run(Action::Select(i));
            g.run(Action::Digit(g.solution[i]));
        }
        assert!(g.solved);
        assert_eq!(g.errors, 0);
    }

    #[test]
    fn boards_save_and_load_in_both_sizes() {
        for s in SHAPES {
            let mut g = Game::new(vec![crate::palette::builtin()], 0);
            g.new_board(s, 3, 12345678);
            g.demo();
            let saved = g.save_board();
            let mut h = Game::new(vec![crate::palette::builtin()], 0);
            assert!(h.load_board(&saved));
            assert_eq!((h.shape, h.level, h.seed), (s, 3, 12345678));
            assert_eq!(h.vals, g.vals);
            assert_eq!(h.notes, g.notes);
        }
        // A save from before 6x6 boards (no size field) loads as 9x9.
        let mut g = Game::new(vec![crate::palette::builtin()], 0);
        g.new_board(NINE, 1, 20261121);
        let old: Vec<String> = g.save_board().split('.').map(String::from).collect();
        let old = [&old[..2], &old[3..]].concat().join(".");
        let mut h = Game::new(vec![crate::palette::builtin()], 0);
        assert!(h.load_board(&old));
        assert_eq!(h.shape, NINE);
    }

    #[test]
    fn solved_boards_keep_their_best_time() {
        let mut g = Game::new(vec![crate::palette::builtin()], 0);
        let solve = |g: &mut Game, secs: f32| {
            g.new_board(sudoku::SIX, 2, 20261122);
            g.elapsed = secs;
            for i in 0..36 {
                if g.vals[i] == 0 {
                    g.run(Action::Select(i));
                    g.run(Action::Digit(g.solution[i]));
                }
            }
            assert!(g.solved);
        };
        solve(&mut g, 200.0);
        solve(&mut g, 300.0);
        g.new_board(NINE, 0, 20261122);
        let rows = g.day_rows(20261122, 6);
        assert_eq!(rows[2].1, "03:20", "the faster time stays");
        assert_eq!(rows[0].1, "-");
        assert_eq!(g.day_rows(20261122, 9)[0].1, "STARTED");
        // Records survive a save and load.
        let saved = g.settings().games;
        let mut h = Game::new(vec![crate::palette::builtin()], 0);
        h.set_records(&saved);
        assert_eq!(h.records, g.records);
    }

    #[test]
    fn a_digit_with_no_cell_starts_fast_mode_and_cells_toggle() {
        let mut g = Game::new(vec![crate::palette::builtin()], 0);
        g.new_board(NINE, 0, 20261122);
        let i = (0..81).find(|&i| g.puzzle[i] == 0).unwrap();
        // Tapping a cell twice selects it, then lets it go.
        g.run(Action::Select(i));
        assert_eq!(g.sel, Some(i));
        g.run(Action::Select(i));
        assert_eq!(g.sel, None);
        // A digit with nothing selected switches to fast mode with it picked...
        g.run(Action::Digit(g.solution[i]));
        assert!(g.fast);
        assert_eq!(g.fast_digit, Some(g.solution[i]));
        assert_eq!(g.vals[i], 0);
        // ...so the next tap places it.
        g.run(Action::Select(i));
        assert_eq!(g.vals[i], g.solution[i]);
    }

    #[test]
    fn best_times_rows_pick_their_level() {
        let mut g = Game::new(vec![crate::palette::builtin()], 0);
        g.new_board(NINE, 0, 20261122);
        g.run(Action::New);
        let row = g.view().unwrap().buttons.into_iter().find(|b| b.action == Action::PickLevel(3)).unwrap();
        let (x, y, w, h) = row.hit;
        let target = g.target_at(x + w / 2, y + h / 2);
        assert_eq!(target, Some(Action::PickLevel(3)));
        g.run(target.unwrap());
        g.run(Action::NewYes);
        assert_eq!(g.level, 3);
    }

    #[test]
    fn picking_the_fast_digit_again_toggles_notes_then_drops_it() {
        let mut g = Game::new(vec![crate::palette::builtin()], 0);
        g.new_board(NINE, 0, 20261122);
        g.run(Action::Fast);
        g.run(Action::Digit(5));
        assert_eq!((g.fast_digit, g.notes_mode), (Some(5), false));
        g.run(Action::Digit(5));
        assert_eq!((g.fast_digit, g.notes_mode), (Some(5), true), "second press: notes");
        g.run(Action::Digit(5));
        assert_eq!((g.fast_digit, g.notes_mode), (None, false), "third press: put down, notes back");
        // Picking another digit starts the cycle again, keeping the notes setting.
        g.run(Action::Digit(5));
        g.run(Action::Digit(5));
        g.run(Action::Digit(3));
        assert_eq!((g.fast_digit, g.notes_mode), (Some(3), true));
        g.run(Action::Digit(3));
        assert_eq!((g.fast_digit, g.notes_mode), (Some(3), false));
    }

    #[test]
    fn fast_mode_leaves_filled_cells_alone() {
        let mut g = Game::new(vec![crate::palette::builtin()], 0);
        g.new_board(NINE, 0, 20261122);
        let i = (0..81).find(|&i| g.puzzle[i] == 0).unwrap();
        let other = g.solution[i] % 9 + 1;
        g.run(Action::Select(i));
        g.run(Action::Digit(g.solution[i]));
        g.run(Action::Select(i)); // let it go
        g.run(Action::Digit(other)); // no cell selected: fast mode, `other` picked
        assert!(g.fast);
        g.run(Action::Select(i));
        assert_eq!(g.vals[i], g.solution[i], "another digit isn't placed over it");
        assert_eq!(g.errors, 0);
        // The same digit clears it, as before.
        g.run(Action::Digit(g.solution[i]));
        g.run(Action::Select(i));
        assert_eq!(g.vals[i], 0);
    }

    #[test]
    fn a_and_d_step_through_digits_to_place() {
        let mut g = Game::new(vec![crate::palette::builtin()], 0);
        g.new_board(sudoku::SIX, 0, 20261122);
        g.run(Action::CycleDigit(1));
        assert!(g.fast);
        let first = g.fast_digit.unwrap();
        assert!(!g.all_placed(first));
        g.run(Action::CycleDigit(1));
        g.run(Action::CycleDigit(-1));
        assert_eq!(g.fast_digit, Some(first), "D then A comes back");
        // Finished digits are skipped.
        let d = first % 6 + 1;
        for i in 0..36 {
            if g.solution[i] == d {
                g.vals[i] = d;
            }
        }
        g.run(Action::CycleDigit(1));
        assert_ne!(g.fast_digit, Some(d));
    }

    #[test]
    fn only_visible_clashes_are_errors() {
        let mut g = Game::new(vec![crate::palette::builtin()], 0);
        g.new_board(NINE, 0, 20261122);
        let cand = sudoku::candidates(NINE, &g.vals);
        // A wrong digit that clashes with nothing on the board: not an error.
        let (i, d) = (0..81)
            .filter(|&i| g.vals[i] == 0)
            .find_map(|i| (1..=9u8).find(|&d| d != g.solution[i] && cand[i] & (1 << d) != 0).map(|d| (i, d)))
            .unwrap();
        g.run(Action::Select(i));
        g.run(Action::Digit(d));
        assert_eq!(g.vals[i], d);
        assert!(!g.conflict(i));
        assert_eq!(g.errors, 0);
        // A digit already in its row: an error, and both cells clash.
        let j = (0..81).find(|&j| g.vals[j] == 0 && j / 9 == i / 9 && j != i).unwrap();
        g.run(Action::Select(j));
        g.run(Action::Digit(d));
        assert!(g.conflict(j) && g.conflict(i));
        assert_eq!(g.errors, 1);
    }

    #[test]
    fn notes_cant_go_where_the_digit_is_already_seen() {
        let mut g = Game::new(vec![crate::palette::builtin()], 0);
        g.new_board(NINE, 0, 20261122);
        let i = (0..81).find(|&i| g.vals[i] == 0).unwrap();
        let cand = sudoku::candidates(NINE, &g.vals)[i];
        let seen = (1..=9u8).find(|&d| cand & (1 << d) == 0).unwrap();
        let free = (1..=9u8).find(|&d| cand & (1 << d) != 0).unwrap();
        g.run(Action::Notes);
        g.run(Action::Select(i));
        g.run(Action::Digit(seen));
        assert_eq!(g.notes[i] & (1 << seen), 0, "blocked: already in the row, column or box");
        g.run(Action::Digit(free));
        assert_ne!(g.notes[i] & (1 << free), 0);
        // An old note that has since become impossible can still be taken off.
        g.notes[i] |= 1 << seen;
        g.run(Action::Digit(seen));
        assert_eq!(g.notes[i] & (1 << seen), 0);
        assert_eq!(g.errors, 0, "a blocked note isn't an error");
    }

    #[test]
    fn swipes_go_by_their_main_direction() {
        assert_eq!(swipe_action(40.0, 5.0), Some(Action::CycleDigit(-1)), "right: previous");
        assert_eq!(swipe_action(-40.0, 12.0), Some(Action::CycleDigit(1)), "left: next");
        assert_eq!(swipe_action(6.0, -30.0), Some(Action::Notes));
        assert_eq!(swipe_action(-3.0, 30.0), Some(Action::Undo));
        assert_eq!(swipe_action(5.0, 4.0), None, "a tap isn't a swipe");
        for (w, h) in [(315, 560), (315, 700), (560, 315)] {
            for s in SHAPES {
                let l = Layout::new(w, h, s);
                let (_, sy, _, sh) = l.swipe;
                let digits_bottom = l.pads.iter().flat_map(|p| p.hits.iter()).map(|r| r.1 + r.3).max().unwrap();
                assert!(sy >= digits_bottom && sh >= 30, "{w}x{h} {} swipe pad below the digits", s.name());
                assert!(sy + sh <= l.rule.0);
            }
        }
    }

    #[test]
    fn the_number_and_notes_rows() {
        let mut g = Game::new(vec![crate::palette::builtin()], 0);
        g.new_board(NINE, 0, 20261122);
        let i = (0..81).find(|&i| g.vals[i] == 0).unwrap();
        let cand = sudoku::candidates(NINE, &g.vals)[i];
        let free = (1..=9u8).find(|&d| cand & (1 << d) != 0).unwrap();
        // With a cell selected: the notes row notes, the number row places.
        g.run(Action::Select(i));
        g.run(Action::Note(free));
        assert!(g.notes_mode);
        assert_ne!(g.notes[i] & (1 << free), 0);
        g.run(Action::Num(g.solution[i]));
        assert!(!g.notes_mode);
        assert_eq!(g.vals[i], g.solution[i]);
        // With none: fast mode. Tapping the other row switches placing and
        // noting; the same button again puts the digit down.
        g.run(Action::Select(i));
        assert_eq!(g.sel, None);
        g.run(Action::Num(4));
        assert_eq!((g.fast, g.fast_digit, g.notes_mode), (true, Some(4), false));
        g.run(Action::Note(4));
        assert_eq!((g.fast_digit, g.notes_mode), (Some(4), true));
        g.run(Action::Note(4));
        assert_eq!(g.fast_digit, None);
        // Tapping a row's button is the screen's way; the keyboard's digit keys
        // keep the press-again-for-notes cycle.
        g.run(Action::Digit(6));
        g.run(Action::Digit(6));
        assert_eq!((g.fast_digit, g.notes_mode), (Some(6), false), "notes were on, so the second press turns them off");
    }

    fn solve(g: &mut Game) {
        for i in 0..g.shape.cells() {
            if g.vals[i] == 0 {
                g.sel = None;
                g.run(Action::Select(i));
                g.run(Action::Num(g.solution[i]));
            }
        }
        assert!(g.solved);
    }

    #[test]
    fn solving_a_daily_board_asks_for_a_name() {
        let mut g = Game::new(vec![crate::palette::builtin()], 0);
        g.nickname = "SAVVA".into();
        g.new_board(sudoku::SIX, 0, today_seed());
        g.run(Action::Hint);
        assert_eq!(g.hints, 0, "no hints in the daily");
        solve(&mut g);
        assert!(matches!(&g.finish, Some(Finish { submit: Submit::Editing(n), .. }) if n.text() == "SAVVA"));
        // Offline: sending fails, and can be retried or skipped.
        g.run(Action::NameSubmit);
        assert!(matches!(&g.finish, Some(Finish { submit: Submit::Failed(_), .. })));
        g.run(Action::NameSkip);
        assert!(matches!(&g.finish, Some(Finish { submit: Submit::Skipped, .. })));
        // Another day's board isn't on the leaderboard.
        g.new_board(sudoku::SIX, 0, 20200101);
        solve(&mut g);
        assert!(g.finish.is_none());
    }

    /// Against the local test server: `node web/dev-server.js`, then
    /// `cargo test -- --ignored daily`. Every one of today's ten boards,
    /// solved, is accepted: the server's generator matches the game's.
    #[test]
    #[ignore]
    fn daily_grids_are_checked_on_the_server() {
        for s in SHAPES {
            for level in 0..LEVELS.len() {
                let mut g = Game::new(vec![crate::palette::builtin()], 0);
                g.leaderboard = Some(crate::net::tests::test_url());
                g.new_board(s, level, today_seed());
                g.elapsed = 600.0;
                solve(&mut g);
                if let Some(Finish { submit: Submit::Editing(n), .. }) = &mut g.finish {
                    *n = NameEdit::new("TESTRUN");
                }
                g.run(Action::NameSubmit);
                g.settle_requests();
                assert!(
                    matches!(&g.finish, Some(Finish { submit: Submit::Done(_), .. })),
                    "{} {} not accepted: {:?}",
                    s.name(),
                    LEVELS[level].name,
                    g.finish.as_ref().map(|f| match &f.submit {
                        Submit::Failed(m) => m.clone(),
                        _ => "?".into(),
                    })
                );
            }
        }
    }

    #[test]
    fn dates_make_seeds() {
        assert_eq!(date_seed(0), 19700101);
        assert_eq!(date_seed(20_778), 20261121);
    }
}
