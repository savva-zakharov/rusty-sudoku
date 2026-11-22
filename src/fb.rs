//! Palette-indexed framebuffer with a few pixel primitives and text.

use crate::font;
use crate::palette::*;

pub struct Fb {
    pub w: i32,
    pub h: i32,
    pub px: Vec<u8>,
}

/// Text cell: 6 pixels wide (5x7 glyphs plus a gap), lines 10 apart.
pub const CW: i32 = 6;
pub const LH: i32 = 10;

impl Fb {
    pub fn new(w: i32, h: i32) -> Self {
        Fb { w, h, px: vec![BG; (w * h) as usize] }
    }

    pub fn clear(&mut self, c: u8) {
        self.px.fill(c);
    }

    #[inline]
    pub fn pset(&mut self, x: i32, y: i32, c: u8) {
        if x >= 0 && y >= 0 && x < self.w && y < self.h {
            self.px[(y * self.w + x) as usize] = c;
        }
    }

    #[inline]
    pub fn get(&self, x: i32, y: i32) -> u8 {
        if x >= 0 && y >= 0 && x < self.w && y < self.h {
            self.px[(y * self.w + x) as usize]
        } else {
            BG
        }
    }

    pub fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, c: u8) {
        let x0 = x.max(0);
        let y0 = y.max(0);
        let x1 = (x + w).min(self.w);
        let y1 = (y + h).min(self.h);
        for yy in y0..y1 {
            let row = (yy * self.w) as usize;
            self.px[row + x0 as usize..row + x1.max(x0) as usize].fill(c);
        }
    }

    pub fn hline(&mut self, x: i32, y: i32, w: i32, c: u8) {
        self.rect(x, y, w, 1, c);
    }

    pub fn outline(&mut self, x: i32, y: i32, w: i32, h: i32, c: u8) {
        self.rect(x, y, w, 1, c);
        self.rect(x, y + h - 1, w, 1, c);
        self.rect(x, y, 1, h, c);
        self.rect(x + w - 1, y, 1, h, c);
    }

    /// 50% checkerboard dither over a rectangle.
    pub fn dither(&mut self, x: i32, y: i32, w: i32, h: i32, c: u8) {
        for yy in y..y + h {
            for xx in x..x + w {
                if (xx + yy) & 1 == 0 {
                    self.pset(xx, yy, c);
                }
            }
        }
    }

    /// Draw `rows` at (x, y): `#` in `ink`, `+` in `alt`, anything else clear.
    /// Each pixel is a `k` x `k` block.
    pub fn sprite_k(&mut self, x: i32, y: i32, rows: &[&str], ink: u8, alt: u8, k: i32) {
        for (j, row) in rows.iter().enumerate() {
            for (i, ch) in row.chars().enumerate() {
                let c = match ch {
                    '#' => ink,
                    '+' => alt,
                    _ => continue,
                };
                self.rect(x + i as i32 * k, y + j as i32 * k, k, k, c);
            }
        }
    }

    pub fn text_w(s: &str) -> i32 {
        s.chars().count() as i32 * CW
    }

    pub fn text(&mut self, x: i32, y: i32, s: &str, c: u8) -> i32 {
        let mut cx = x;
        for ch in s.chars() {
            if let Some(g) = font::glyph(ch.to_ascii_uppercase()) {
                self.sprite_k(cx, y, &g, c, c, 1);
            }
            cx += CW;
        }
        cx
    }

    /// Text at double size (each font pixel 2x2), 12 pixels per character.
    pub fn text2x(&mut self, x: i32, y: i32, s: &str, c: u8) {
        self.text_k(x, y, s, c, 2);
    }

    /// Text with each font pixel a `k` x `k` block.
    pub fn text_k(&mut self, x: i32, y: i32, s: &str, c: u8, k: i32) {
        for (n, ch) in s.chars().enumerate() {
            if let Some(g) = font::glyph(ch) {
                self.sprite_k(x + n as i32 * k * CW, y, &g, c, c, k);
            }
        }
    }

    /// A line of box drawing characters (spaces are skipped), each in a full
    /// 6x10 cell: double lines in `major`, single lines in `minor`.
    pub fn boxes(&mut self, x: i32, y: i32, s: &str, major: u8, minor: u8) {
        for (k, ch) in s.chars().enumerate() {
            if let Some(g) = font::boxch(ch) {
                self.sprite_k(x + k as i32 * CW, y, &g, major, minor, 1);
            }
        }
    }

    /// Text where anything inside [brackets] is drawn in the key colour.
    pub fn text_keys(&mut self, x: i32, y: i32, s: &str, c: u8, kc: u8) -> i32 {
        self.text_keys_br(x, y, s, c, kc, UI_DK)
    }

    /// Like `text_keys`, with the bracket colour `bc` chosen by the caller
    /// (e.g. bright on a highlighted button, where dark brackets would vanish).
    pub fn text_keys_br(&mut self, x: i32, y: i32, s: &str, c: u8, kc: u8, bc: u8) -> i32 {
        let mut cx = x;
        let mut inside = false;
        for ch in s.chars() {
            let col = match ch {
                '[' => {
                    inside = true;
                    bc
                }
                ']' => {
                    inside = false;
                    bc
                }
                _ if inside => kc,
                _ => c,
            };
            if let Some(g) = font::glyph(ch.to_ascii_uppercase()) {
                self.sprite_k(cx, y, &g, col, col, 1);
            }
            cx += CW;
        }
        cx
    }

    pub fn text_center(&mut self, cx: i32, y: i32, s: &str, c: u8, kc: u8) {
        let w = Self::text_w(s) - 1;
        self.text_keys(cx - w / 2, y, s, c, kc);
    }

    pub fn tiny(&mut self, x: i32, y: i32, s: &str, c: u8) {
        let mut cx = x;
        for ch in s.chars() {
            if let Some(g) = font::tiny(ch) {
                self.sprite_k(cx, y, &g, c, c, 1);
            }
            cx += 4;
        }
    }

    /// Expand palette indices to RGBA using the active colour scheme.
    pub fn write_rgba(&self, out: &mut [u8], pal: &[Rgb; SLOTS]) {
        for (i, &p) in self.px.iter().enumerate() {
            let rgb = pal[p as usize];
            let o = i * 4;
            out[o] = rgb[0];
            out[o + 1] = rgb[1];
            out[o + 2] = rgb[2];
            out[o + 3] = 255;
        }
    }
}
