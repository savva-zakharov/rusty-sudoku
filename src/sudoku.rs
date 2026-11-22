//! Sudoku boards: a seeded generator, a solver that counts solutions, and a
//! small logical solver (naked and hidden singles) that grades difficulty and
//! explains hints. Boards are 9x9 (3x3 boxes) or 6x6 (boxes 2 rows by 3 columns).
//!
//! A board comes from an 8-digit seed (today's date makes the daily board), a
//! shape and a difficulty level. Each level and shape mixes its own number into
//! the random stream, so the boards of one seed have completely different
//! solutions, not the same grid with more or fewer clues.

/// The cells, row by row; 0 is empty, 1..=n a digit.
pub type Grid = Vec<u8>;

/// A board shape: `n` x `n` cells in boxes of `br` rows by `bc` columns.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Shape {
    pub n: usize,
    pub br: usize,
    pub bc: usize,
}

pub const NINE: Shape = Shape { n: 9, br: 3, bc: 3 };
pub const SIX: Shape = Shape { n: 6, br: 2, bc: 3 };
/// The shapes on offer, in the order the new-board dialog cycles them.
pub const SHAPES: [Shape; 2] = [NINE, SIX];

impl Shape {
    pub fn cells(self) -> usize {
        self.n * self.n
    }
    pub fn row(self, i: usize) -> usize {
        i / self.n
    }
    pub fn col(self, i: usize) -> usize {
        i % self.n
    }
    pub fn bx(self, i: usize) -> usize {
        (self.row(i) / self.br) * (self.n / self.bc) + self.col(i) / self.bc
    }
    /// Index into per-shape tables (`Level`): 0 for 9x9, 1 for 6x6.
    pub fn idx(self) -> usize {
        SHAPES.iter().position(|&s| s == self).unwrap_or(0)
    }
    /// `9X9` / `6X6`.
    pub fn name(self) -> String {
        format!("{0}X{0}", self.n)
    }
    /// Digits 1..=n as bits.
    fn all(self) -> u16 {
        (1u16 << (self.n + 1)) - 2
    }

    /// The units: rows 0..n, columns n..2n, boxes 2n..3n.
    pub fn units(self) -> Vec<Vec<usize>> {
        let n = self.n;
        let mut u = vec![];
        for k in 0..n {
            u.push((0..n).map(|j| k * n + j).collect());
        }
        for k in 0..n {
            u.push((0..n).map(|j| j * n + k).collect());
        }
        let across = n / self.bc;
        for k in 0..n {
            let (top, left) = ((k / across) * self.br, (k % across) * self.bc);
            u.push((0..n).map(|j| (top + j / self.bc) * n + left + j % self.bc).collect());
        }
        u
    }

    /// Whether cells `a` and `b` share a row, column or box (a cell isn't its own peer).
    pub fn peers(self, a: usize, b: usize) -> bool {
        a != b && (self.row(a) == self.row(b) || self.col(a) == self.col(b) || self.bx(a) == self.bx(b))
    }
}

/// splitmix64: tiny, fast and the same on every platform, so a seed deals the
/// same board on desktop, the web and Android.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
    }

    pub fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    pub fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }

    pub fn shuffle<T>(&mut self, v: &mut [T]) {
        for i in (1..v.len()).rev() {
            let j = self.below(i + 1);
            v.swap(i, j);
        }
    }
}

fn bit(d: u8) -> u16 {
    1 << d
}

/// Digits used in each row, column and box.
#[derive(Clone)]
struct Used {
    s: Shape,
    row: [u16; 9],
    col: [u16; 9],
    bx: [u16; 9],
}

impl Used {
    fn of(s: Shape, g: &[u8]) -> Used {
        let mut u = Used { s, row: [0; 9], col: [0; 9], bx: [0; 9] };
        for (i, &d) in g.iter().enumerate() {
            if d != 0 {
                u.set(i, d);
            }
        }
        u
    }
    fn set(&mut self, i: usize, d: u8) {
        let s = self.s;
        self.row[s.row(i)] |= bit(d);
        self.col[s.col(i)] |= bit(d);
        self.bx[s.bx(i)] |= bit(d);
    }
    fn unset(&mut self, i: usize, d: u8) {
        let s = self.s;
        self.row[s.row(i)] &= !bit(d);
        self.col[s.col(i)] &= !bit(d);
        self.bx[s.bx(i)] &= !bit(d);
    }
    fn free(&self, i: usize) -> u16 {
        let s = self.s;
        s.all() & !(self.row[s.row(i)] | self.col[s.col(i)] | self.bx[s.bx(i)])
    }
}

/// Candidates for each cell of `g` (0 for filled cells).
pub fn candidates(s: Shape, g: &[u8]) -> Vec<u16> {
    let u = Used::of(s, g);
    (0..g.len()).map(|i| if g[i] == 0 { u.free(i) } else { 0 }).collect()
}

/// The empty cell with the fewest candidates, or None if the grid is full.
fn most_constrained(g: &[u8], u: &Used) -> Option<(usize, u16)> {
    let mut best: Option<(usize, u16)> = None;
    for i in 0..g.len() {
        if g[i] == 0 {
            let f = u.free(i);
            if best.map_or(true, |(_, b)| f.count_ones() < b.count_ones()) {
                best = Some((i, f));
                if f.count_ones() <= 1 {
                    break;
                }
            }
        }
    }
    best
}

/// Count solutions of `g`, stopping at `limit`.
pub fn count_solutions(s: Shape, g: &[u8], limit: usize) -> usize {
    fn go(g: &mut [u8], u: &mut Used, limit: usize, n: &mut usize) {
        let Some((i, mut f)) = most_constrained(g, u) else {
            *n += 1;
            return;
        };
        while f != 0 && *n < limit {
            let d = f.trailing_zeros() as u8;
            f &= f - 1;
            g[i] = d;
            u.set(i, d);
            go(g, u, limit, n);
            u.unset(i, d);
            g[i] = 0;
        }
    }
    let mut g = g.to_vec();
    let mut u = Used::of(s, &g);
    // A grid that already breaks the rules has no solutions.
    for i in 0..g.len() {
        if g[i] != 0 {
            let d = g[i];
            u.unset(i, d);
            let ok = u.free(i) & bit(d) != 0;
            u.set(i, d);
            if !ok {
                return 0;
            }
        }
    }
    let mut n = 0;
    go(&mut g, &mut u, limit, &mut n);
    n
}

/// A random complete, valid grid.
fn full_grid(s: Shape, rng: &mut Rng) -> Grid {
    fn go(s: Shape, g: &mut [u8], u: &mut Used, rng: &mut Rng) -> bool {
        let Some((i, f)) = most_constrained(g, u) else { return true };
        let mut ds: Vec<u8> = (1..=s.n as u8).filter(|&d| f & bit(d) != 0).collect();
        rng.shuffle(&mut ds);
        for d in ds {
            g[i] = d;
            u.set(i, d);
            if go(s, g, u, rng) {
                return true;
            }
            u.unset(i, d);
            g[i] = 0;
        }
        false
    }
    let mut g = vec![0; s.cells()];
    let mut u = Used::of(s, &g);
    go(s, &mut g, &mut u, rng);
    g
}

/// Why a single logical step works.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Reason {
    /// No other digit fits the cell.
    Naked,
    /// The only place for the digit in unit `n` (see `Shape::units`).
    Hidden(usize),
}

/// One forced placement: cell, digit and why.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Step {
    pub cell: usize,
    pub digit: u8,
    pub reason: Reason,
}

/// The next single in `g`: a naked single first (lowest cell), then a hidden
/// single (rows, then columns, then boxes). None if neither exists.
pub fn find_single(s: Shape, g: &[u8], hidden: bool) -> Option<Step> {
    let c = candidates(s, g);
    for i in 0..g.len() {
        if g[i] == 0 && c[i].count_ones() == 1 {
            return Some(Step { cell: i, digit: c[i].trailing_zeros() as u8, reason: Reason::Naked });
        }
    }
    if hidden {
        for (n, unit) in s.units().iter().enumerate() {
            for d in 1..=s.n as u8 {
                if unit.iter().any(|&i| g[i] == d) {
                    continue;
                }
                let mut spots = unit.iter().filter(|&&i| g[i] == 0 && c[i] & bit(d) != 0);
                if let (Some(&i), None) = (spots.next(), spots.next()) {
                    return Some(Step { cell: i, digit: d, reason: Reason::Hidden(n) });
                }
            }
        }
    }
    None
}

/// Whether singles alone solve `g` (naked only, or naked and hidden). A grid
/// that singles solve has exactly one solution.
pub fn singles_solve(s: Shape, g: &[u8], hidden: bool) -> bool {
    let mut g = g.to_vec();
    loop {
        // Place every naked single found in one sweep, then look again.
        let c = candidates(s, &g);
        let mut progress = false;
        for i in 0..g.len() {
            if g[i] == 0 {
                match c[i].count_ones() {
                    0 => return false,
                    1 => {
                        let d = c[i].trailing_zeros() as u8;
                        // Two naked singles in a sweep could clash; re-check.
                        if Used::of(s, &g).free(i) & bit(d) == 0 {
                            return false;
                        }
                        g[i] = d;
                        progress = true;
                    }
                    _ => {}
                }
            }
        }
        if !progress && hidden {
            if let Some(st) = find_single(s, &g, true) {
                g[st.cell] = st.digit;
                progress = true;
            }
        }
        if !progress {
            return g.iter().all(|&d| d != 0);
        }
    }
}

/// What a level allows while taking clues away.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Rule {
    /// Naked singles must solve it.
    Naked,
    /// Naked and hidden singles must solve it.
    Singles,
    /// Anything, as long as the solution stays unique.
    Unique,
}

/// A difficulty level. The tables hold one entry per shape (see `Shape::idx`).
pub struct Level {
    pub name: &'static str,
    /// Stop taking clues away at this many (0: as few as possible).
    pub clues: [usize; 2],
    pub keep: [Rule; 2],
    /// The finished board must not be solvable by this rule alone.
    pub not: [Option<Rule>; 2],
    /// A few words for the new-board dialog.
    pub blurb: [&'static str; 2],
}

/// On 6x6 boards singles nearly always suffice, so the top levels there ask
/// for hidden singles (not naked ones alone) and fewer clues instead.
pub const LEVELS: [Level; 5] = [
    Level { name: "EASY", clues: [38, 20], keep: [Rule::Naked; 2], not: [None; 2], blurb: ["ONE DIGIT FITS"; 2] },
    Level { name: "MEDIUM", clues: [32, 16], keep: [Rule::Singles; 2], not: [None; 2], blurb: ["ONE PLACE LEFT"; 2] },
    Level {
        name: "HARD",
        clues: [27, 13],
        keep: [Rule::Singles; 2],
        not: [Some(Rule::Naked); 2],
        blurb: ["HIDDEN SINGLES"; 2],
    },
    Level {
        name: "EXPERT",
        clues: [25, 11],
        keep: [Rule::Unique; 2],
        not: [Some(Rule::Singles), Some(Rule::Naked)],
        blurb: ["BEYOND SINGLES", "FEWER CLUES"],
    },
    Level {
        name: "MASTER",
        clues: [0, 0],
        keep: [Rule::Unique; 2],
        not: [Some(Rule::Singles), Some(Rule::Naked)],
        blurb: ["FEWEST CLUES"; 2],
    },
];

fn solvable_by(s: Shape, g: &[u8], rule: Rule) -> bool {
    match rule {
        Rule::Naked => singles_solve(s, g, false),
        Rule::Singles => singles_solve(s, g, true),
        Rule::Unique => count_solutions(s, g, 2) == 1,
    }
}

/// The random stream for a seed, level and shape. Each gets its own stream,
/// so the boards of a day have nothing in common. (9x9 streams are the same
/// as before 6x6 boards existed, so saved 9x9 boards still load.)
fn stream(s: Shape, seed: u32, level: usize) -> Rng {
    let shape_mix = if s == NINE { 0 } else { 0x6A09_E667_F3BC_C908 ^ s.n as u64 };
    let mut r = Rng::new(((seed as u64) << 8) ^ (level as u64 + 1).wrapping_mul(0xA24B_AED4_963E_E407) ^ 0x5D0C_0000 ^ shape_mix);
    r.next();
    r
}

/// The puzzle and its solution for a shape, seed (0..=99_999_999) and level
/// (0..5). Clues are taken away in symmetric pairs (the cell and its mirror
/// through the centre), as long as the level's rule still solves the board.
pub fn generate(s: Shape, seed: u32, level: usize) -> (Grid, Grid) {
    let lv = &LEVELS[level.min(LEVELS.len() - 1)];
    let k = s.idx();
    let (target, keep, not) = (lv.clues[k], lv.keep[k], lv.not[k]);
    let cells = s.cells();
    let mut rng = stream(s, seed, level);
    let mut last = (vec![], vec![]);
    for _attempt in 0..40 {
        let solution = full_grid(s, &mut rng);
        let mut puzzle = solution.clone();
        let mut order: Vec<usize> = (0..cells.div_ceil(2)).collect();
        rng.shuffle(&mut order);
        let mut clues = cells;
        for &i in &order {
            if clues <= target {
                break;
            }
            let j = cells - 1 - i;
            let (a, b) = (puzzle[i], puzzle[j]);
            puzzle[i] = 0;
            puzzle[j] = 0;
            if solvable_by(s, &puzzle, keep) {
                clues -= if i == j { 1 } else { 2 };
            } else {
                puzzle[i] = a;
                puzzle[j] = b;
            }
        }
        // The hardest level then tries every remaining clue on its own too,
        // giving up the symmetry for a minimal board.
        if target == 0 {
            let mut rest: Vec<usize> = (0..cells).filter(|&i| puzzle[i] != 0).collect();
            rng.shuffle(&mut rest);
            for i in rest {
                let a = puzzle[i];
                puzzle[i] = 0;
                if !solvable_by(s, &puzzle, keep) {
                    puzzle[i] = a;
                }
            }
        }
        let hard_enough = not.map_or(true, |r| !solvable_by(s, &puzzle, r));
        last = (puzzle, solution);
        if hard_enough {
            break;
        }
    }
    last
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clues(g: &[u8]) -> usize {
        g.iter().filter(|&&d| d != 0).count()
    }

    #[test]
    fn boards_are_valid_unique_and_graded() {
        for s in SHAPES {
            for seed in [20261121, 20261122, 12345678, 20261225, 31415926] {
                for (k, lv) in LEVELS.iter().enumerate() {
                    let (p, sol) = generate(s, seed, k);
                    assert_eq!(sol.len(), s.cells());
                    assert!(sol.iter().all(|&d| d != 0));
                    assert_eq!(count_solutions(s, &sol, 2), 1, "solution is valid");
                    for i in 0..s.cells() {
                        assert!(p[i] == 0 || p[i] == sol[i]);
                    }
                    assert_eq!(count_solutions(s, &p, 2), 1, "{} {} {seed} unique", s.name(), lv.name);
                    let keep = lv.keep[s.idx()];
                    if keep != Rule::Unique {
                        assert!(solvable_by(s, &p, keep), "{} {} {seed} keeps its rule", s.name(), lv.name);
                    }
                    if let Some(r) = lv.not[s.idx()] {
                        assert!(!solvable_by(s, &p, r), "{} {} {seed} is hard enough", s.name(), lv.name);
                    }
                    eprintln!("{} {seed} {:6} {} clues", s.name(), lv.name, clues(&p));
                }
            }
        }
    }

    #[test]
    fn levels_and_shapes_of_a_seed_are_different_boards() {
        for s in SHAPES {
            let sols: Vec<Grid> = (0..5).map(|k| generate(s, 20261121, k).1).collect();
            for a in 0..5 {
                for b in a + 1..5 {
                    // Unrelated grids match in about 1 cell in n by chance
                    // (more on 6x6, which has far fewer possible grids).
                    let same = (0..s.cells()).filter(|&i| sols[a][i] == sols[b][i]).count();
                    assert!(same < s.cells() / 2, "{} levels {a} and {b} share {same} cells", s.name());
                    assert_ne!(generate(s, 20261121, a).0, generate(s, 20261121, b).0);
                }
            }
        }
    }

    #[test]
    fn same_seed_same_board() {
        for s in SHAPES {
            assert_eq!(generate(s, 20261121, 2), generate(s, 20261121, 2));
            assert_ne!(generate(s, 20261121, 2).0, generate(s, 20261122, 2).0);
        }
    }

    /// Boards for the server's port (pano-gallery's api/_sudoku.js) to check
    /// itself against: `SUDOKU_FIXTURE=path cargo test fixture` writes
    /// `seed size level puzzle solution` lines.
    #[test]
    fn fixture() {
        let Ok(path) = std::env::var("SUDOKU_FIXTURE") else { return };
        let mut out = String::new();
        for seed in [20261121, 20261122, 20261231, 12345678] {
            for s in SHAPES {
                for k in 0..LEVELS.len() {
                    let (p, sol) = generate(s, seed, k);
                    let digits = |g: &Grid| g.iter().map(|d| d.to_string()).collect::<String>();
                    out += &format!("{seed} {} {} {} {}
", s.n, LEVELS[k].name, digits(&p), digits(&sol));
                }
            }
        }
        std::fs::write(path, out).unwrap();
    }

    #[test]
    fn units_cover_the_grid() {
        for s in SHAPES {
            let units = s.units();
            assert_eq!(units.len(), 3 * s.n);
            let mut count = vec![0; s.cells()];
            for u in &units {
                let mut seen = vec![false; s.cells()];
                for &i in u {
                    seen[i] = true;
                    count[i] += 1;
                }
                assert_eq!(seen.iter().filter(|&&b| b).count(), s.n);
            }
            assert!(count.iter().all(|&c| c == 3), "each cell is in one row, column and box");
        }
        assert_eq!(NINE.units()[18 + 4][4], 40);
        // 6x6 box 3 (second row of boxes, right): rows 2-3, columns 3-5.
        assert_eq!(SIX.units()[12 + 3], vec![15, 16, 17, 21, 22, 23]);
    }
}
