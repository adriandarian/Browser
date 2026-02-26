use fontdue::{
    layout::{CoordinateSystem, Layout, LayoutSettings, TextStyle},
    Font, FontSettings,
};
use std::{
    collections::{HashMap, HashSet},
    env, fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pattern {
    Gradient,
    Solid,
    Rects,
}

impl Pattern {
    pub fn parse(input: &str) -> Option<Self> {
        match input {
            "gradient" => Some(Self::Gradient),
            "solid" => Some(Self::Solid),
            "rects" => Some(Self::Rects),
            _ => None,
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Gradient => Self::Solid,
            Self::Solid => Self::Rects,
            Self::Rects => Self::Gradient,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DrawRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub color: [u8; 4],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrawText {
    pub x: i32,
    pub y: i32,
    pub text: String,
    pub color: [u8; 4],
    pub scale: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OverlayInfo {
    pub frame_index: u64,
    pub fps: f32,
    pub width: u32,
    pub height: u32,
}

pub struct Renderer {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
    pattern: Pattern,
    fonts: Vec<FontChoice>,
    font_index: usize,
    loaded_fonts: HashMap<usize, Font>,
}

#[derive(Debug, Clone)]
struct FontChoice {
    name: String,
    path: Option<PathBuf>,
}

impl Renderer {
    pub fn new(width: u32, height: u32) -> Self {
        let fonts = discover_fonts();
        let font_index = default_font_index(&fonts);
        let mut renderer = Self {
            width: 0,
            height: 0,
            pixels: Vec::new(),
            pattern: Pattern::Gradient,
            fonts,
            font_index,
            loaded_fonts: HashMap::new(),
        };
        renderer.ensure_font_loaded(renderer.font_index);
        renderer.resize(width, height);
        renderer
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if self.width == width && self.height == height {
            return;
        }
        self.width = width;
        self.height = height;
        let new_len = pixel_len(width, height);
        if self.pixels.len() != new_len {
            self.pixels.resize(new_len, 0);
        }
    }

    pub fn set_pattern(&mut self, pattern: Pattern) {
        self.pattern = pattern;
    }

    pub fn pattern(&self) -> Pattern {
        self.pattern
    }

    pub fn render(&mut self, frame_index: u64, time_seconds: f32) -> &[u8] {
        self.render_pattern(frame_index, time_seconds, None)
    }

    pub fn render_pattern(
        &mut self,
        frame_index: u64,
        time_seconds: f32,
        overlay: Option<OverlayInfo>,
    ) -> &[u8] {
        match self.pattern {
            Pattern::Gradient => {
                render_gradient(&mut self.pixels, self.width, self.height, frame_index)
            }
            Pattern::Solid => {
                let pulse = pulse_u8(frame_index, time_seconds);
                clear_rgba(&mut self.pixels, pulse, 32, 120, 255);
            }
            Pattern::Rects => render_rects(&mut self.pixels, self.width, self.height, frame_index),
        }

        if let Some(overlay) = overlay {
            draw_overlay(&mut self.pixels, self.width, self.height, overlay);
        }

        &self.pixels
    }

    pub fn render_display_list(
        &mut self,
        frame_index: u64,
        time_seconds: f32,
        rects: &[DrawRect],
        texts: &[DrawText],
        overlay: Option<OverlayInfo>,
    ) -> &[u8] {
        let bg_pulse = pulse_u8(frame_index, time_seconds) >> 4;
        clear_rgba(
            &mut self.pixels,
            20_u8.saturating_add(bg_pulse),
            20_u8.saturating_add(bg_pulse),
            24_u8.saturating_add(bg_pulse),
            255,
        );

        for rect in rects {
            fill_rect(
                &mut self.pixels,
                self.width,
                self.height,
                rect.x,
                rect.y,
                rect.width,
                rect.height,
                rect.color,
            );
        }

        let use_system_font = self.ensure_font_loaded(self.font_index);
        for text in texts {
            if use_system_font {
                if let Some(font) = self.loaded_fonts.get(&self.font_index) {
                    let px = text_px(text.scale);
                    draw_text_fontdue(
                        &mut self.pixels,
                        self.width,
                        self.height,
                        text.x,
                        text.y,
                        &text.text,
                        text.color,
                        font,
                        px,
                    );
                } else {
                    draw_text_scaled(
                        &mut self.pixels,
                        self.width,
                        self.height,
                        text.x,
                        text.y,
                        &text.text,
                        text.color,
                        text.scale.max(1),
                    );
                }
            } else {
                draw_text_scaled(
                    &mut self.pixels,
                    self.width,
                    self.height,
                    text.x,
                    text.y,
                    &text.text,
                    text.color,
                    text.scale.max(1),
                );
            }
        }

        if let Some(overlay) = overlay {
            draw_overlay(&mut self.pixels, self.width, self.height, overlay);
        }

        &self.pixels
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn cycle_font(&mut self) -> String {
        if self.fonts.is_empty() {
            return "Pixel 5x7".to_string();
        }

        let start = self.font_index;
        loop {
            self.font_index = (self.font_index + 1) % self.fonts.len();
            if self.font_is_ready(self.font_index) || self.font_index == start {
                break;
            }
        }
        self.current_font_name().to_string()
    }

    pub fn current_font_name(&self) -> &str {
        self.fonts
            .get(self.font_index)
            .map(|entry| entry.name.as_str())
            .unwrap_or("Pixel 5x7")
    }

    pub fn current_font_index(&self) -> usize {
        self.font_index
    }

    pub fn font_name(&self, index: usize) -> Option<&str> {
        self.fonts.get(index).map(|f| f.name.as_str())
    }

    pub fn font_count(&self) -> usize {
        self.fonts.len()
    }

    pub fn set_font_index(&mut self, index: usize) -> bool {
        if index >= self.fonts.len() {
            return false;
        }
        if !self.font_is_ready(index) {
            return false;
        }
        self.font_index = index;
        true
    }

    fn font_is_ready(&mut self, index: usize) -> bool {
        match self.fonts.get(index) {
            Some(FontChoice { path: None, .. }) => true,
            Some(FontChoice { path: Some(_), .. }) => self.ensure_font_loaded(index),
            None => false,
        }
    }

    fn ensure_font_loaded(&mut self, index: usize) -> bool {
        if self.loaded_fonts.contains_key(&index) {
            return true;
        }
        let Some(choice) = self.fonts.get(index) else {
            return false;
        };
        let Some(path) = &choice.path else {
            return false;
        };

        let Ok(bytes) = fs::read(path) else {
            return false;
        };
        let Ok(font) = Font::from_bytes(bytes, FontSettings::default()) else {
            return false;
        };
        self.loaded_fonts.insert(index, font);
        true
    }
}

fn pixel_len(width: u32, height: u32) -> usize {
    (width as usize)
        .saturating_mul(height as usize)
        .saturating_mul(4)
}

fn text_px(scale: u32) -> f32 {
    12.0 + (scale.max(1) as f32 * 2.0)
}

fn discover_fonts() -> Vec<FontChoice> {
    let mut fonts = Vec::new();
    fonts.push(FontChoice {
        name: "Pixel 5x7".to_string(),
        path: None,
    });

    let mut roots = Vec::new();
    #[cfg(target_os = "macos")]
    {
        roots.push(PathBuf::from("/System/Library/Fonts"));
        roots.push(PathBuf::from("/Library/Fonts"));
        if let Ok(home) = env::var("HOME") {
            roots.push(PathBuf::from(home).join("Library/Fonts"));
        }
    }
    #[cfg(target_os = "windows")]
    {
        roots.push(PathBuf::from("C:/Windows/Fonts"));
    }
    #[cfg(target_os = "linux")]
    {
        roots.push(PathBuf::from("/usr/share/fonts"));
        roots.push(PathBuf::from("/usr/local/share/fonts"));
        if let Ok(home) = env::var("HOME") {
            roots.push(PathBuf::from(home).join(".fonts"));
            roots.push(PathBuf::from(home).join(".local/share/fonts"));
        }
    }

    let files = collect_font_files(&roots);
    let mut used_paths = HashSet::new();

    // Curated families first so the popup defaults to sane UI/text fonts.
    let preferred = [
        "SFNS",
        "SF Pro",
        "Helvetica",
        "Arial",
        "Avenir",
        "Inter",
        "Menlo",
        "Monaco",
        "Times",
        "Georgia",
        "Verdana",
        "Trebuchet",
        "Courier",
    ];
    for family in preferred {
        if let Some(path) = find_font_by_name(&files, family) {
            if used_paths.insert(path.clone()) {
                fonts.push(FontChoice {
                    name: font_display_name(&path),
                    path: Some(path),
                });
            }
        }
    }

    // Then add a filtered subset of remaining readable text fonts.
    for path in files {
        if used_paths.contains(&path) || !is_readable_text_font(&path) {
            continue;
        }
        used_paths.insert(path.clone());
        fonts.push(FontChoice {
            name: font_display_name(&path),
            path: Some(path),
        });
        if fonts.len() >= 80 {
            break;
        }
    }

    fonts
}

fn default_font_index(fonts: &[FontChoice]) -> usize {
    if fonts.len() <= 1 {
        return 0;
    }

    let preferred = ["sf", "helvetica", "arial", "georgia", "times", "menlo"];
    for (index, entry) in fonts.iter().enumerate().skip(1) {
        let lowered = entry.name.to_ascii_lowercase();
        if preferred.iter().any(|needle| lowered.contains(needle)) {
            return index;
        }
    }
    1
}

fn collect_font_files(roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack: Vec<PathBuf> = roots.to_vec();

    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if is_font_path(&path) && is_readable_text_font(&path) {
                out.push(path);
            }
        }
    }

    out.sort();
    out
}

fn is_font_path(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("ttf") | Some("otf") | Some("TTF") | Some("OTF")
    )
}

fn font_display_name(path: &Path) -> String {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .map(|name| name.replace(['_', '-'], " "))
        .unwrap_or_else(|| "Unknown Font".to_string())
}

fn find_font_by_name(files: &[PathBuf], token: &str) -> Option<PathBuf> {
    let token = token.to_ascii_lowercase();
    files.iter().find_map(|path| {
        let name = font_display_name(path).to_ascii_lowercase();
        if name.contains(&token) {
            Some(path.clone())
        } else {
            None
        }
    })
}

fn is_readable_text_font(path: &Path) -> bool {
    let name = font_display_name(path).to_ascii_lowercase();
    let excluded = [
        "emoji",
        "symbol",
        "dingbat",
        "wingdings",
        "webdings",
        "ornament",
        "lastresort",
        "math",
        "music",
        "icons",
        "materialicons",
    ];
    !excluded.iter().any(|term| name.contains(term))
}

fn pulse_u8(frame_index: u64, time_seconds: f32) -> u8 {
    // Quantize time first, then hash with frame index for deterministic animation.
    let time_ms = if time_seconds.is_finite() {
        (time_seconds.max(0.0) * 1000.0) as u64
    } else {
        0
    };
    mix_to_u8(frame_index.wrapping_mul(0x9e37_79b9_7f4a_7c15) ^ time_ms)
}

fn mix_to_u8(mut x: u64) -> u8 {
    x ^= x >> 33;
    x = x.wrapping_mul(0xff51afd7ed558ccd);
    x ^= x >> 33;
    x = x.wrapping_mul(0xc4ceb9fe1a85ec53);
    x ^= x >> 33;
    x as u8
}

fn clear_rgba(framebuffer: &mut [u8], r: u8, g: u8, b: u8, a: u8) {
    for px in framebuffer.chunks_exact_mut(4) {
        px[0] = r;
        px[1] = g;
        px[2] = b;
        px[3] = a;
    }
}

fn render_gradient(framebuffer: &mut [u8], width: u32, height: u32, frame_index: u64) {
    let w = width as usize;
    let h = height as usize;

    if framebuffer.len() < w * h * 4 || w == 0 || h == 0 {
        return;
    }

    let t = frame_index as u32;
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) * 4;
            let fx = x as u32;
            let fy = y as u32;

            framebuffer[i] = ((fx + t) & 0xFF) as u8;
            framebuffer[i + 1] = ((fy + (t / 2)) & 0xFF) as u8;
            framebuffer[i + 2] = (((fx ^ fy) + (t / 3)) & 0xFF) as u8;
            framebuffer[i + 3] = 0xFF;
        }
    }
}

fn render_rects(framebuffer: &mut [u8], width: u32, height: u32, frame_index: u64) {
    clear_rgba(framebuffer, 20, 20, 24, 255);

    let w = width as i32;
    let h = height as i32;
    if w <= 0 || h <= 0 {
        return;
    }

    let offset = (frame_index % 120) as i32;
    fill_rect(
        framebuffer,
        width,
        height,
        24 + offset / 2,
        20,
        120,
        90,
        [210, 70, 70, 255],
    );
    fill_rect(
        framebuffer,
        width,
        height,
        w / 2 - 80,
        h / 2 - 50,
        170,
        110,
        [70, 180, 240, 255],
    );
    fill_rect(
        framebuffer,
        width,
        height,
        w - 180 - offset,
        h - 110,
        140,
        70,
        [90, 220, 120, 255],
    );
}

fn draw_overlay(framebuffer: &mut [u8], width: u32, height: u32, overlay: OverlayInfo) {
    if width < 24 || height < 16 {
        return;
    }

    let panel_width = width.min(360) as i32;
    fill_rect(
        framebuffer,
        width,
        height,
        6,
        6,
        panel_width,
        22,
        [0, 0, 0, 180],
    );

    let text = format!(
        "F{} P{:.1} W{} H{}",
        overlay.frame_index, overlay.fps, overlay.width, overlay.height
    );
    draw_text(
        framebuffer,
        width,
        height,
        10,
        10,
        &text,
        [230, 230, 230, 255],
    );
}

fn draw_text(
    framebuffer: &mut [u8],
    width: u32,
    height: u32,
    x: i32,
    y: i32,
    text: &str,
    color: [u8; 4],
) {
    draw_text_scaled(framebuffer, width, height, x, y, text, color, 1);
}

fn draw_text_fontdue(
    framebuffer: &mut [u8],
    width: u32,
    height: u32,
    x: i32,
    y: i32,
    text: &str,
    color: [u8; 4],
    font: &Font,
    px: f32,
) {
    let mut layout = Layout::new(CoordinateSystem::PositiveYDown);
    layout.reset(&LayoutSettings::default());
    layout.append(&[font], &TextStyle::new(text, px, 0));

    for glyph in layout.glyphs() {
        let (metrics, bitmap) = font.rasterize_config(glyph.key);
        if metrics.width == 0 || metrics.height == 0 {
            continue;
        }
        draw_alpha_bitmap(
            framebuffer,
            width,
            height,
            x + glyph.x.floor() as i32,
            y + glyph.y.floor() as i32,
            metrics.width,
            metrics.height,
            &bitmap,
            color,
        );
    }
}

fn draw_alpha_bitmap(
    framebuffer: &mut [u8],
    width: u32,
    height: u32,
    x: i32,
    y: i32,
    bmp_w: usize,
    bmp_h: usize,
    bitmap: &[u8],
    color: [u8; 4],
) {
    let stride = width as usize * 4;
    for row in 0..bmp_h {
        let py = y + row as i32;
        if py < 0 || py >= height as i32 {
            continue;
        }
        for col in 0..bmp_w {
            let px = x + col as i32;
            if px < 0 || px >= width as i32 {
                continue;
            }

            let src_row = bmp_h - 1 - row;
            let coverage = bitmap[src_row * bmp_w + col];
            if coverage == 0 {
                continue;
            }

            let index = py as usize * stride + px as usize * 4;
            blend_pixel(&mut framebuffer[index..index + 4], color, coverage);
        }
    }
}

fn blend_pixel(dst: &mut [u8], src: [u8; 4], coverage: u8) {
    let alpha = ((src[3] as u16 * coverage as u16) / 255) as u8;
    if alpha == 0 {
        return;
    }

    let inv_alpha = 255_u16.saturating_sub(alpha as u16);
    for channel in 0..3 {
        let d = dst[channel] as u16;
        let s = src[channel] as u16;
        dst[channel] = ((d * inv_alpha + s * alpha as u16) / 255) as u8;
    }
    dst[3] = 255;
}

fn draw_text_scaled(
    framebuffer: &mut [u8],
    width: u32,
    height: u32,
    mut x: i32,
    y: i32,
    text: &str,
    color: [u8; 4],
    scale: u32,
) {
    let advance = (6 * scale as i32).max(1);
    for ch in text.chars() {
        draw_char_scaled(framebuffer, width, height, x, y, ch, color, scale);
        x += advance;
    }
}

fn draw_char_scaled(
    framebuffer: &mut [u8],
    width: u32,
    height: u32,
    x: i32,
    y: i32,
    ch: char,
    color: [u8; 4],
    scale: u32,
) {
    let rows = glyph_rows(ch.to_ascii_uppercase());
    let pixel = scale.max(1) as i32;

    for (row_index, row_bits) in rows.iter().enumerate() {
        for col in 0..5 {
            let bit = 1 << (4 - col);
            if row_bits & bit == 0 {
                continue;
            }

            fill_rect(
                framebuffer,
                width,
                height,
                x + (col * pixel),
                y + (row_index as i32 * pixel),
                pixel,
                pixel,
                color,
            );
        }
    }
}

fn glyph_rows(ch: char) -> [u8; 7] {
    match ch {
        'A' => [0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        'B' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110],
        'C' => [0b01111, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b01111],
        'D' => [0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110],
        'E' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111],
        'F' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000],
        'G' => [0b01111, 0b10000, 0b10000, 0b10111, 0b10001, 0b10001, 0b01111],
        'H' => [0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        'I' => [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b11111],
        'J' => [0b00111, 0b00010, 0b00010, 0b00010, 0b00010, 0b10010, 0b01100],
        'K' => [0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001],
        'L' => [0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111],
        'M' => [0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001],
        'N' => [0b10001, 0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001],
        'O' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        'P' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000],
        'Q' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101],
        'R' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001],
        'S' => [0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110],
        'T' => [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100],
        'U' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        'V' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100],
        'W' => [0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b11011, 0b10001],
        'X' => [0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001],
        'Y' => [0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100],
        'Z' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111],
        '0' => [0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110],
        '1' => [0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
        '2' => [0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111],
        '3' => [0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110],
        '4' => [0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010],
        '5' => [0b11111, 0b10000, 0b10000, 0b11110, 0b00001, 0b00001, 0b11110],
        '6' => [0b01110, 0b10000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110],
        '7' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000],
        '8' => [0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110],
        '9' => [0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00001, 0b01110],
        '<' => [0b00001, 0b00010, 0b00100, 0b01000, 0b00100, 0b00010, 0b00001],
        '>' => [0b10000, 0b01000, 0b00100, 0b00010, 0b00100, 0b01000, 0b10000],
        '/' => [0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b00000, 0b00000],
        ':' => [0b00000, 0b00100, 0b00100, 0b00000, 0b00100, 0b00100, 0b00000],
        ';' => [0b00000, 0b00100, 0b00100, 0b00000, 0b00100, 0b00100, 0b01000],
        ',' => [0b00000, 0b00000, 0b00000, 0b00000, 0b00110, 0b00100, 0b01000],
        '.' => [0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b01100, 0b01100],
        '-' => [0b00000, 0b00000, 0b00000, 0b11111, 0b00000, 0b00000, 0b00000],
        '_' => [0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b11111],
        '=' => [0b00000, 0b11111, 0b00000, 0b11111, 0b00000, 0b00000, 0b00000],
        '\'' => [0b00100, 0b00100, 0b01000, 0b00000, 0b00000, 0b00000, 0b00000],
        '"' => [0b01010, 0b01010, 0b10001, 0b00000, 0b00000, 0b00000, 0b00000],
        '(' => [0b00010, 0b00100, 0b01000, 0b01000, 0b01000, 0b00100, 0b00010],
        ')' => [0b01000, 0b00100, 0b00010, 0b00010, 0b00010, 0b00100, 0b01000],
        '[' => [0b01110, 0b01000, 0b01000, 0b01000, 0b01000, 0b01000, 0b01110],
        ']' => [0b01110, 0b00010, 0b00010, 0b00010, 0b00010, 0b00010, 0b01110],
        '+' => [0b00000, 0b00100, 0b00100, 0b11111, 0b00100, 0b00100, 0b00000],
        '!' => [0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00000, 0b00100],
        '?' => [0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b00000, 0b00100],
        '&' => [0b01100, 0b10010, 0b10100, 0b01000, 0b10101, 0b10010, 0b01101],
        ' ' => [0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000],
        _ => [0b00000, 0b00000, 0b01110, 0b00010, 0b00100, 0b00000, 0b00100],
    }
}

fn fill_rect(
    framebuffer: &mut [u8],
    width: u32,
    height: u32,
    x: i32,
    y: i32,
    rect_width: i32,
    rect_height: i32,
    color: [u8; 4],
) {
    if rect_width <= 0 || rect_height <= 0 {
        return;
    }

    let x0 = x.max(0).min(width as i32);
    let y0 = y.max(0).min(height as i32);
    let x1 = (x + rect_width).max(0).min(width as i32);
    let y1 = (y + rect_height).max(0).min(height as i32);

    if x0 >= x1 || y0 >= y1 {
        return;
    }

    let stride = width as usize * 4;
    for py in y0 as usize..y1 as usize {
        let row = py * stride;
        for px in x0 as usize..x1 as usize {
            let i = row + px * 4;
            framebuffer[i..i + 4].copy_from_slice(&color);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pixels_are_rgba8() {
        let mut renderer = Renderer::new(8, 4);
        renderer.set_pattern(Pattern::Rects);
        let frame = renderer.render(5, 0.25);

        assert_eq!(frame.len(), 8 * 4 * 4);
        assert!(frame.chunks_exact(4).all(|px| px[3] == 0xFF));

        // Ensure channel ordering by checking a known rasterized pixel.
        assert_eq!(&frame[0..4], &[70, 180, 240, 255]);
    }

    #[test]
    fn deterministic_frame_hash() {
        let mut renderer = Renderer::new(64, 32);
        renderer.set_pattern(Pattern::Gradient);
        let frame = renderer.render(42, 1.25);

        assert_eq!(fnv1a64(frame), 0xaa3e6ff366d761a5);
    }

    #[test]
    fn deterministic_frame_hash_with_time_input() {
        let mut renderer = Renderer::new(64, 32);
        renderer.set_pattern(Pattern::Solid);
        let frame = renderer.render(77, 1.5);

        assert_eq!(fnv1a64(frame), 0xb10375b873063325);
    }

    #[test]
    fn resize_reuses_capacity_when_shrinking() {
        let mut renderer = Renderer::new(64, 64);
        let initial_capacity = renderer.pixels.capacity();
        renderer.resize(32, 32);
        let shrunk_capacity = renderer.pixels.capacity();

        assert_eq!(renderer.pixels.len(), 32 * 32 * 4);
        assert_eq!(shrunk_capacity, initial_capacity);
    }

    #[test]
    fn display_list_renders_rects() {
        let mut renderer = Renderer::new(32, 16);
        let rects = [DrawRect {
            x: 2,
            y: 2,
            width: 6,
            height: 4,
            color: [255, 10, 10, 255],
        }];

        let frame = renderer.render_display_list(0, 0.0, &rects, &[], None);
        let stride = 32 * 4;
        let idx = (2 * stride) + (2 * 4);
        assert_eq!(&frame[idx..idx + 4], &[255, 10, 10, 255]);
    }

    fn fnv1a64(bytes: &[u8]) -> u64 {
        let mut hash = 0xcbf29ce484222325_u64;
        for b in bytes {
            hash ^= u64::from(*b);
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash
    }
}
