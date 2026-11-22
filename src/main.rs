#![allow(dead_code)]

mod fb;
mod font;
mod game;
mod palette;
mod settings;
mod sudoku;

use macroquad::prelude::*;

fn arg(flag: &str) -> Option<String> {
    let args: Vec<String> = std::env::args().collect();
    args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1)).cloned()
}

fn flag(flag: &str) -> bool {
    std::env::args().any(|a| a == flag)
}

fn conf() -> Conf {
    Conf {
        window_title: "RUSTY-SUDOKU".to_owned(),
        // Portrait first: the base screen is 315 x 560 (9:16). The window can
        // be resized to any shape; landscape switches to the side-by-side layout.
        window_width: 450,
        window_height: 800,
        window_resizable: true,
        // In the browser, render at the device's real pixel density so phones get
        // crisp pixels instead of an upscaled blur.
        high_dpi: cfg!(target_arch = "wasm32"),
        ..Default::default()
    }
}

#[macroquad::main(conf)]
async fn main() {
    macroquad::rand::srand((macroquad::miniquad::date::now() * 1000.0) as u64);
    // `--shot out.png [--size WxH] [--level N] [--seed YYYYMMDD] [--six] [--demo]`
    // renders one frame of that screen size to a PNG and exits.
    let shot = arg("--shot");
    let shot_size = arg("--size").and_then(|s| {
        let (w, h) = s.split_once('x')?;
        Some((w.parse::<i32>().ok()?, h.parse::<i32>().ok()?))
    });

    let saved = if shot.is_some() { settings::Settings::default() } else { settings::load() };
    let root = assets_dir();
    let schemes = palette::load(&root, false).await;
    let scheme = schemes.iter().position(|s| s.name.eq_ignore_ascii_case(&saved.colours)).unwrap_or(0);
    let mut game = game::Game::new(schemes, scheme);
    game.crt = saved.crt.unwrap_or(true);
    game.muted = saved.muted.unwrap_or(false);
    let level = arg("--level").and_then(|l| l.parse().ok()).or(saved.level).unwrap_or(0);
    let seed = arg("--seed").and_then(|s| s.chars().filter(|c| c.is_ascii_digit()).collect::<String>().parse().ok());
    game.set_records(&saved.games);
    if seed.is_some() || shot.is_some() || !game.load_board(&saved.board) {
        // `--six` starts on a 6x6 board; otherwise the size played last.
        let n = if flag("--six") { Some(6) } else if shot.is_some() { None } else { saved.size };
        let shape = sudoku::SHAPES.iter().copied().find(|s| Some(s.n) == n).unwrap_or(sudoku::NINE);
        game.new_board(shape, level.min(4), seed.unwrap_or_else(game::today_seed));
    }
    if flag("--demo") {
        game.demo();
    }
    game.set_input(flag("--notes"), arg("--pick").and_then(|d| d.parse().ok()));
    // `--touch` shows the swipe pad's legend, as after a touch (for screenshots).
    game.touch_seen = flag("--touch");
    if flag("--new-dialog") {
        game.open_new_dialog();
    }

    let mut sound_loader = shot.is_none().then(|| {
        let root = root.clone();
        macroquad::experimental::coroutines::start_coroutine(async move { load_sounds(&root).await })
    });
    let mut kept = game.settings();

    let mut size = (0, 0);
    let mut fb = fb::Fb::new(1, 1);
    let mut image = Image::gen_image_color(1, 1, BLACK);
    let mut tex = Texture2D::from_image(&image);

    loop {
        if sound_loader.as_ref().is_some_and(|c| c.is_done()) {
            if let Some(s) = sound_loader.take().and_then(|c| c.retrieve()) {
                game.sounds = s;
            }
        }
        // The framebuffer is 315 x 560 in portrait (560 x 315 in landscape),
        // grown along the long side to match the window, then scaled up.
        let want = shot_size.unwrap_or_else(|| game::screen_size(screen_width(), screen_height()));
        if want != size {
            size = want;
            fb = fb::Fb::new(size.0, size.1);
            image = Image::gen_image_color(size.0 as u16, size.1 as u16, BLACK);
            tex = Texture2D::from_image(&image);
            tex.set_filter(FilterMode::Nearest);
            game.resize(size.0, size.1);
        }
        let (w, h) = size;
        let s = (screen_width() / w as f32).min(screen_height() / h as f32);
        let ox = ((screen_width() - w as f32 * s) / 2.0).floor();
        let oy = ((screen_height() - h as f32 * s) / 2.0).floor();
        let (mx, my) = mouse_position();
        let dt = get_frame_time().min(0.05);

        if !game.update(dt, ((mx - ox) / s, (my - oy) / s)) {
            break;
        }
        if shot.is_none() {
            let now = game.settings();
            if now != kept {
                settings::save(&now);
                kept = now;
            }
        }
        game.render(&mut fb);
        fb.write_rgba(&mut image.bytes, game.palette());
        tex.update(&image);

        let bg = game.palette()[palette::BG as usize];
        clear_background(Color::from_rgba(bg[0], bg[1], bg[2], 255));
        draw_texture_ex(&tex, ox, oy, WHITE, DrawTextureParams { dest_size: Some(vec2(w as f32 * s, h as f32 * s)), ..Default::default() });
        if game.crt && s >= 2.0 {
            let lh = (s / 3.0).max(1.0).floor();
            let col = Color::new(0.0, 0.0, 0.0, 0.38);
            for r in 0..h {
                draw_rectangle(ox, (oy + (r + 1) as f32 * s).round() - lh, w as f32 * s, lh, col);
            }
        }
        if let Some(path) = &shot {
            save_shot(&fb, game.palette(), path, 3);
            break;
        }
        next_frame().await
    }
}

/// Where to load the palettes and sounds from: `assets/` next to index.html
/// on the web, in the working directory or next to the executable on desktop.
fn assets_dir() -> String {
    if cfg!(target_arch = "wasm32") || cfg!(target_os = "android") || std::path::Path::new("assets").is_dir() {
        return "assets".into();
    }
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("assets")))
        .filter(|d| d.is_dir())
        .map(|d| d.to_string_lossy().into_owned())
        .unwrap_or_else(|| "assets".into())
}

async fn load_sounds(root: &str) -> game::Sounds {
    game::Sounds {
        clicks: load_numbered(root, "click", 6).await,
        spaces: load_numbered(root, "space", 3).await,
        enters: load_numbered(root, "enter", 3).await,
        escapes: load_numbered(root, "escape", 3).await,
    }
}

/// `sounds/<stem>1.wav` .. `<stem><count>.wav`, each also tried capitalised
/// (web servers are case-sensitive). Missing files are skipped.
async fn load_numbered(root: &str, stem: &str, count: usize) -> Vec<macroquad::audio::Sound> {
    let cap = stem[..1].to_ascii_uppercase() + &stem[1..];
    let mut out = vec![];
    for n in 1..=count {
        for name in [format!("{stem}{n}.wav"), format!("{cap}{n}.wav")] {
            if let Ok(s) = macroquad::audio::load_sound(&format!("{root}/sounds/{name}")).await {
                out.push(s);
                break;
            }
        }
    }
    out
}

fn save_shot(fb: &fb::Fb, pal: &[palette::Rgb; palette::SLOTS], path: &str, k: i32) {
    let (w, h) = (fb.w * k, fb.h * k);
    let mut img = Image::gen_image_color(w as u16, h as u16, BLACK);
    for y in 0..h {
        let scan = (h - 1 - y) % k == k - 1;
        for x in 0..w {
            // export_png writes rows bottom-up, so sample the framebuffer flipped.
            let rgb = pal[fb.get(x / k, (h - 1 - y) / k) as usize];
            let f = if scan { 0.62 } else { 1.0 };
            let o = ((y * w + x) * 4) as usize;
            img.bytes[o] = (rgb[0] as f32 * f) as u8;
            img.bytes[o + 1] = (rgb[1] as f32 * f) as u8;
            img.bytes[o + 2] = (rgb[2] as f32 * f) as u8;
            img.bytes[o + 3] = 255;
        }
    }
    img.export_png(path);
}
