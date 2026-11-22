//! Hand-made 5x7 terminal font, a 3x5 micro font for pencil marks, and box
//! drawing characters that fill their whole 6x10 cell so lines join up.

pub fn glyph(c: char) -> Option<[&'static str; 7]> {
    Some(match c {
        'A' => [".###.", "#...#", "#...#", "#####", "#...#", "#...#", "#...#"],
        'B' => ["####.", "#...#", "#...#", "####.", "#...#", "#...#", "####."],
        'C' => [".###.", "#...#", "#....", "#....", "#....", "#...#", ".###."],
        'D' => ["####.", "#...#", "#...#", "#...#", "#...#", "#...#", "####."],
        'E' => ["#####", "#....", "#....", "####.", "#....", "#....", "#####"],
        'F' => ["#####", "#....", "#....", "####.", "#....", "#....", "#...."],
        'G' => [".###.", "#...#", "#....", "#.###", "#...#", "#...#", ".####"],
        'H' => ["#...#", "#...#", "#...#", "#####", "#...#", "#...#", "#...#"],
        'I' => [".###.", "..#..", "..#..", "..#..", "..#..", "..#..", ".###."],
        'J' => ["..###", "...#.", "...#.", "...#.", "...#.", "#..#.", ".##.."],
        'K' => ["#...#", "#..#.", "#.#..", "##...", "#.#..", "#..#.", "#...#"],
        'L' => ["#....", "#....", "#....", "#....", "#....", "#....", "#####"],
        'M' => ["#...#", "##.##", "#.#.#", "#.#.#", "#...#", "#...#", "#...#"],
        'N' => ["#...#", "#...#", "##..#", "#.#.#", "#..##", "#...#", "#...#"],
        'O' => [".###.", "#...#", "#...#", "#...#", "#...#", "#...#", ".###."],
        'P' => ["####.", "#...#", "#...#", "####.", "#....", "#....", "#...."],
        'Q' => [".###.", "#...#", "#...#", "#...#", "#.#.#", "#..#.", ".##.#"],
        'R' => ["####.", "#...#", "#...#", "####.", "#.#..", "#..#.", "#...#"],
        'S' => [".####", "#....", "#....", ".###.", "....#", "....#", "####."],
        'T' => ["#####", "..#..", "..#..", "..#..", "..#..", "..#..", "..#.."],
        'U' => ["#...#", "#...#", "#...#", "#...#", "#...#", "#...#", ".###."],
        'V' => ["#...#", "#...#", "#...#", "#...#", "#...#", ".#.#.", "..#.."],
        'W' => ["#...#", "#...#", "#...#", "#.#.#", "#.#.#", "#.#.#", ".#.#."],
        'X' => ["#...#", "#...#", ".#.#.", "..#..", ".#.#.", "#...#", "#...#"],
        'Y' => ["#...#", "#...#", ".#.#.", "..#..", "..#..", "..#..", "..#.."],
        'Z' => ["#####", "....#", "...#.", "..#..", ".#...", "#....", "#####"],
        '0' => [".###.", "#...#", "#..##", "#.#.#", "##..#", "#...#", ".###."],
        '1' => ["..#..", ".##..", "..#..", "..#..", "..#..", "..#..", ".###."],
        '2' => [".###.", "#...#", "....#", "...#.", "..#..", ".#...", "#####"],
        '3' => ["#####", "...#.", "..#..", "...#.", "....#", "#...#", ".###."],
        '4' => ["...#.", "..##.", ".#.#.", "#..#.", "#####", "...#.", "...#."],
        '5' => ["#####", "#....", "####.", "....#", "....#", "#...#", ".###."],
        '6' => ["..##.", ".#...", "#....", "####.", "#...#", "#...#", ".###."],
        '7' => ["#####", "....#", "...#.", "..#..", ".#...", ".#...", ".#..."],
        '8' => [".###.", "#...#", "#...#", ".###.", "#...#", "#...#", ".###."],
        '9' => [".###.", "#...#", "#...#", ".####", "....#", "...#.", ".##.."],
        ' ' => [".....", ".....", ".....", ".....", ".....", ".....", "....."],
        ':' => [".....", "..#..", "..#..", ".....", "..#..", "..#..", "....."],
        '-' => [".....", ".....", ".....", "#####", ".....", ".....", "....."],
        '[' => [".###.", ".#...", ".#...", ".#...", ".#...", ".#...", ".###."],
        ']' => [".###.", "...#.", "...#.", "...#.", "...#.", "...#.", ".###."],
        '>' => [".#...", "..#..", "...#.", "....#", "...#.", "..#..", ".#..."],
        '<' => ["...#.", "..#..", ".#...", "#....", ".#...", "..#..", "...#."],
        '^' => [".....", ".....", "..#..", ".#.#.", "#...#", ".....", "....."],
        '⌄' => [".....", ".....", "#...#", ".#.#.", "..#..", ".....", "....."],
        '_' => [".....", ".....", ".....", ".....", ".....", ".....", "#####"],
        '.' => [".....", ".....", ".....", ".....", ".....", ".##..", ".##.."],
        ',' => [".....", ".....", ".....", ".....", ".##..", "..#..", ".#..."],
        '/' => ["....#", "....#", "...#.", "..#..", ".#...", "#....", "#...."],
        '!' => ["..#..", "..#..", "..#..", "..#..", "..#..", ".....", "..#.."],
        '?' => [".###.", "#...#", "....#", "...#.", "..#..", ".....", "..#.."],
        '(' => ["...#.", "..#..", ".#...", ".#...", ".#...", "..#..", "...#."],
        ')' => [".#...", "..#..", "...#.", "...#.", "...#.", "..#..", ".#..."],
        '\'' => ["..#..", "..#..", ".#...", ".....", ".....", ".....", "....."],
        '=' => [".....", ".....", "#####", ".....", "#####", ".....", "....."],
        '+' => [".....", "..#..", "..#..", "#####", "..#..", "..#..", "....."],
        '|' => ["..#..", "..#..", "..#..", "..#..", "..#..", "..#..", "..#.."],
        '*' => [".....", "#.#.#", ".###.", "#####", ".###.", "#.#.#", "....."],
        '%' => ["##..#", "##.#.", "...#.", "..#..", ".#...", ".#.##", "#..##"],
        '#' => [".#.#.", "#####", ".#.#.", ".#.#.", ".#.#.", "#####", ".#.#."],
        '█' => ["#####", "#####", "#####", "#####", "#####", "#####", "#####"],
        _ => return None,
    })
}

pub fn tiny(c: char) -> Option<[&'static str; 5]> {
    Some(match c {
        '1' => [".#.", "##.", ".#.", ".#.", "###"],
        '2' => ["##.", "..#", ".#.", "#..", "###"],
        '3' => ["##.", "..#", ".#.", "..#", "##."],
        '4' => ["#.#", "#.#", "###", "..#", "..#"],
        '5' => ["###", "#..", "##.", "..#", "##."],
        '6' => [".##", "#..", "###", "#.#", "###"],
        '7' => ["###", "..#", ".#.", ".#.", ".#."],
        '8' => ["###", "#.#", "###", "#.#", "###"],
        '9' => ["###", "#.#", "###", "..#", "##."],
        'E' => ["###", "#..", "##.", "#..", "###"],
        'S' => [".##", "#..", ".#.", "..#", "##."],
        'W' => ["#.#", "#.#", "#.#", "###", "#.#"],
        'N' => ["#.#", "###", "###", "#.#", "#.#"],
        _ => return None,
    })
}

/// Box drawing characters, 6 wide and 10 tall: the whole text cell, so
/// neighbours join. `#` is part of a double line, `+` part of a single one,
/// so a crossing of both can be drawn in two colours. Single lines run at
/// x = 2 / y = 4; double lines at x = 1 and 3 / y = 3 and 5.
pub fn boxch(c: char) -> Option<[&'static str; 10]> {
    const E: &str = "......";
    const V: &str = "..+...";
    const VV: &str = ".#.#..";
    const H: &str = "++++++";
    const HH: &str = "######";
    Some(match c {
        '─' => [E, E, E, E, H, E, E, E, E, E],
        '│' => [V; 10],
        '┼' => [V, V, V, V, H, V, V, V, V, V],
        '┌' => [E, E, E, E, "..++++", V, V, V, V, V],
        '┐' => [E, E, E, E, "+++...", V, V, V, V, V],
        '└' => [V, V, V, V, "..++++", E, E, E, E, E],
        '┘' => [V, V, V, V, "+++...", E, E, E, E, E],
        '┬' => [E, E, E, E, H, V, V, V, V, V],
        '┴' => [V, V, V, V, H, E, E, E, E, E],
        '├' => [V, V, V, V, "..++++", V, V, V, V, V],
        '┤' => [V, V, V, V, "+++...", V, V, V, V, V],
        '═' => [E, E, E, HH, E, HH, E, E, E, E],
        '║' => [VV; 10],
        '╔' => [E, E, E, ".#####", ".#....", ".#.###", VV, VV, VV, VV],
        '╗' => [E, E, E, "####..", "...#..", "##.#..", VV, VV, VV, VV],
        '╚' => [VV, VV, VV, ".#.###", ".#....", ".#####", E, E, E, E],
        '╝' => [VV, VV, VV, "##.#..", "...#..", "####..", E, E, E, E],
        '╦' => [E, E, E, HH, E, "##.###", VV, VV, VV, VV],
        '╩' => [VV, VV, VV, "##.###", E, HH, E, E, E, E],
        '╠' => [VV, VV, VV, ".#.###", ".#....", ".#.###", VV, VV, VV, VV],
        '╣' => [VV, VV, VV, "##.#..", "...#..", "##.#..", VV, VV, VV, VV],
        '╬' => [VV, VV, VV, "##.###", E, "##.###", VV, VV, VV, VV],
        '╤' => [E, E, E, HH, E, HH, V, V, V, V],
        '╧' => [V, V, V, HH, E, HH, E, E, E, E],
        '╟' => [VV, VV, VV, VV, ".#.#++", VV, VV, VV, VV, VV],
        '╢' => [VV, VV, VV, VV, "+#.#..", VV, VV, VV, VV, VV],
        '╪' => [V, V, V, HH, V, HH, V, V, V, V],
        '╫' => [VV, VV, VV, VV, "+#+#++", VV, VV, VV, VV, VV],
        _ => return None,
    })
}

/// The text styles on offer (theme dialog, `[F]ONT`).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Style {
    /// The 5x7 terminal font.
    #[default]
    Standard,
    /// Taller: 5x9, the 5x7 shapes with two rows repeated, one pixel above
    /// and below (see `stretch_rows`).
    Large,
    /// The 5x7 shapes with serifs: where a stem ends at the top or bottom, a
    /// pixel either side of it; where a top or bottom bar ends at the edge, a
    /// tick turning inward.
    Serif,
}

pub const STYLES: [Style; 3] = [Style::Standard, Style::Large, Style::Serif];

impl Style {
    pub fn name(self) -> &'static str {
        match self {
            Style::Standard => "STANDARD",
            Style::Large => "LARGE",
            Style::Serif => "SERIF",
        }
    }

    pub fn by_name(name: &str) -> Option<Style> {
        STYLES.iter().copied().find(|s| s.name().eq_ignore_ascii_case(name))
    }
}

/// A glyph in a style: `h` rows of 5 pixels (bit 4 is the left column),
/// drawn from `top` rows below the text's y (Large starts a row higher).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Glyph {
    pub rows: [u8; 9],
    pub h: usize,
    pub top: i32,
}

impl Glyph {
    pub fn on(&self, x: usize, y: usize) -> bool {
        self.rows[y] & (0b10000 >> x) != 0
    }
}

/// Which 5x7 row each of the 9 Large rows copies. Two rows are repeated,
/// ideally the third and fifth (so a bar in the middle row stays in the
/// middle), but never one holding a horizontal stroke if a nearby row has
/// none: repeating a stroke would make it twice as thick (the bars of 4 and
/// 5). Repeating a row of only vertical strokes just makes them longer.
fn stretch_rows(base: &[u8; 7]) -> [usize; 9] {
    let stroke = |r: usize| base[r] & (base[r] >> 1) != 0;
    let pick = |want: usize| (1..=5usize).min_by_key(|&r| (r.abs_diff(want) + if stroke(r) { 10 } else { 0 }, r)).unwrap();
    let (a, b) = (pick(2), pick(4));
    let mut out = [0; 9];
    let mut o = 0;
    for r in 0..7 {
        let times = 1 + (r == a) as usize + (r == b) as usize;
        for _ in 0..times {
            out[o] = r;
            o += 1;
        }
    }
    out
}

/// Character `c` in `style`.
pub fn styled(c: char, style: Style) -> Option<Glyph> {
    let g = glyph(c)?;
    let mut base = [0u8; 7];
    for (r, row) in g.iter().enumerate() {
        for (x, ch) in row.chars().enumerate() {
            if ch == '#' {
                base[r] |= 0b10000 >> x;
            }
        }
    }
    let mut rows = [0u8; 9];
    Some(match style {
        Style::Standard => {
            rows[..7].copy_from_slice(&base);
            Glyph { rows, h: 7, top: 0 }
        }
        Style::Large => {
            for (o, &src) in stretch_rows(&base).iter().enumerate() {
                rows[o] = base[src];
            }
            Glyph { rows, h: 9, top: -1 }
        }
        // Serifs only on letters and digits: brackets and other marks keep
        // their shapes (a bracket with feet would look like an I).
        Style::Serif if !c.is_ascii_alphanumeric() => {
            rows[..7].copy_from_slice(&base);
            Glyph { rows, h: 7, top: 0 }
        }
        Style::Serif => {
            rows[..7].copy_from_slice(&base);
            let on = |r: usize, x: i32| (0..5).contains(&x) && base[r] & (0b10000 >> x) != 0;
            // (end row, the row next to it inside the glyph)
            for (end, inner) in [(6, 5), (0, 1)] {
                for x in 0..5 {
                    if on(end, x) && on(inner, x) {
                        for nx in [x - 1, x + 1] {
                            if (0..5).contains(&nx) && !on(end, nx) && !on(inner, nx) {
                                rows[end] |= 0b10000 >> nx;
                            }
                        }
                    }
                }
            }
            // Bars along the top or bottom that end at the glyph's edge get a
            // tick turning inward there (as on T, E and L).
            for (end, inner) in [(6, 5), (0, 1)] {
                for (x, next) in [(0, 1), (4, 3)] {
                    if on(end, x) && on(end, next) && !on(inner, x) {
                        rows[inner] |= 0b10000 >> x;
                    }
                }
            }
            Glyph { rows, h: 7, top: 0 }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn show(g: &Glyph) -> Vec<String> {
        (0..g.h).map(|y| (0..5).map(|x| if g.on(x, y) { '#' } else { '.' }).collect()).collect()
    }

    #[test]
    fn styles_change_the_shapes() {
        let t = styled('T', Style::Serif).unwrap();
        assert_eq!(show(&t)[6], ".###.", "T gets a foot");
        assert_eq!(show(&t)[1], "#.#.#", "and ticks on its bar");
        assert_eq!(show(&styled('L', Style::Serif).unwrap())[5], "#...#");
        let h = styled('H', Style::Serif).unwrap();
        assert_eq!(show(&h)[0], "##.##");
        assert_eq!(show(&styled('O', Style::Serif).unwrap()), show(&styled('O', Style::Standard).unwrap()), "round letters stay");
        let e = styled('E', Style::Large).unwrap();
        assert_eq!(e.h, 9);
        assert_eq!(show(&e)[4], "####.", "the middle bar stays in the middle");
        // Bars are never doubled: no two rows in a row both hold a stroke
        // that the 5x7 glyph has once.
        for c in ['4', '5', '=', 'A', 'E', 'F', 'H', 'P', 'R', '2', '3', '7', 'Z'] {
            let g = styled(c, Style::Large).unwrap();
            let base = styled(c, Style::Standard).unwrap();
            let strokes = |g: &Glyph| (0..g.h).filter(|&r| g.rows[r] & (g.rows[r] >> 1) != 0).count();
            assert_eq!(strokes(&g), strokes(&base), "{c}: a bar got doubled");
        }
        assert!(styled('█', Style::Large).is_some());
        assert_eq!(styled('[', Style::Serif), styled('[', Style::Standard), "marks keep their shapes");
    }
}
