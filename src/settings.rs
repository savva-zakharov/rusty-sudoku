//! Choices and progress kept between sessions, saved as `key=value` lines:
//! the difficulty, the colour scheme, CRT scanlines, muted sound and the
//! board being played.
//!
//! Desktop: `settings.txt` in the user's config folder (`%APPDATA%\rusty-sudoku`
//! on Windows, `~/Library/Application Support/rusty-sudoku` on macOS,
//! `$XDG_CONFIG_HOME` or `~/.config/rusty-sudoku` elsewhere). Web: the
//! browser's localStorage (web/net.js).

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Settings {
    pub level: Option<usize>,
    /// Board size: 9 or 6.
    pub size: Option<usize>,
    pub colours: String,
    pub crt: Option<bool>,
    /// Text style name (font::Style::name).
    pub font: String,
    pub muted: Option<bool>,
    /// The board in progress (see `Game::save_board`).
    pub board: String,
    /// Boards played, one `game=` line each (see `Record`), oldest first.
    pub games: Vec<String>,
}

impl Settings {
    /// Read saved lines; unknown keys and bad lines are skipped.
    pub fn parse(text: &str) -> Settings {
        let mut s = Settings::default();
        for line in text.lines() {
            let Some((k, v)) = line.split_once('=') else { continue };
            let v = v.trim().to_string();
            match k.trim() {
                "level" => s.level = v.parse().ok(),
                "size" => s.size = v.parse().ok(),
                "colours" => s.colours = v,
                "crt" => s.crt = v.parse().ok(),
                "font" => s.font = v,
                "muted" => s.muted = v.parse().ok(),
                "board" => s.board = v,
                "game" => s.games.push(v),
                _ => {}
            }
        }
        s
    }

    pub fn text(&self) -> String {
        let mut t = format!("colours={}\n", self.colours);
        if !self.font.is_empty() {
            t += &format!("font={}\n", self.font);
        }
        if let Some(l) = self.level {
            t += &format!("level={l}\n");
        }
        if let Some(n) = self.size {
            t += &format!("size={n}\n");
        }
        for (key, v) in [("crt", self.crt), ("muted", self.muted)] {
            if let Some(v) = v {
                t += &format!("{key}={v}\n");
            }
        }
        if !self.board.is_empty() {
            t += &format!("board={}\n", self.board);
        }
        for g in &self.games {
            t += &format!("game={g}\n");
        }
        t
    }
}

/// The saved settings, or defaults when there are none.
pub fn load() -> Settings {
    read().map(|t| Settings::parse(&t)).unwrap_or_default()
}

/// Save the settings (failures are ignored: the game plays on without them).
pub fn save(s: &Settings) {
    write(&s.text());
}

#[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
fn path() -> Option<std::path::PathBuf> {
    use std::path::PathBuf;
    let env = |k: &str| std::env::var_os(k).filter(|v| !v.is_empty()).map(PathBuf::from);
    let dir = if cfg!(windows) {
        env("APPDATA")?
    } else if cfg!(target_os = "macos") {
        env("HOME")?.join("Library/Application Support")
    } else {
        env("XDG_CONFIG_HOME").or_else(|| env("HOME").map(|h| h.join(".config")))?
    };
    Some(dir.join("rusty-sudoku").join("settings.txt"))
}

#[cfg(target_os = "android")]
fn path() -> Option<std::path::PathBuf> {
    Some("/data/data/io.github.savvazakharov.rusty_sudoku/files/settings.txt".into())
}

#[cfg(not(target_arch = "wasm32"))]
fn read() -> Option<String> {
    std::fs::read_to_string(path()?).ok()
}

#[cfg(not(target_arch = "wasm32"))]
fn write(text: &str) {
    let Some(p) = path() else { return };
    if let Some(dir) = p.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Err(e) = std::fs::write(&p, text) {
        eprintln!("rusty-sudoku: couldn't save {}: {e}", p.display());
    }
}

#[cfg(target_arch = "wasm32")]
fn read() -> Option<String> {
    let len = unsafe { js::settings_len() };
    if len < 0 {
        return None;
    }
    let mut buf = vec![0u8; len as usize];
    unsafe { js::settings_read(buf.as_mut_ptr()) };
    String::from_utf8(buf).ok()
}

#[cfg(target_arch = "wasm32")]
fn write(text: &str) {
    unsafe { js::settings_write(text.as_ptr(), text.len() as u32) };
}

#[cfg(target_arch = "wasm32")]
mod js {
    extern "C" {
        /// Byte length of the saved settings, or -1 when there are none.
        pub fn settings_len() -> i32;
        /// Copy the saved settings to `dest`.
        pub fn settings_read(dest: *mut u8);
        /// Save `len` bytes of settings text.
        pub fn settings_write(text: *const u8, len: u32);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_survive_a_round_trip() {
        let s = Settings { level: Some(3), size: Some(6), colours: "AMBER".into(), crt: Some(false), font: "SERIF".into(), muted: Some(true), board: "20261121.1.abc".into(), games: vec!["a".into(), "b".into()] };
        assert_eq!(Settings::parse(&s.text()), s);
    }
}
