#[derive(Debug, Clone, Copy)]
pub enum Pattern {
    Plasma,
    Checker,
}

impl Pattern {
    pub fn toggle(self) -> Self {
        match self {
            Self::Plasma => Self::Checker,
            Self::Checker => Self::Plasma,
        }
    }
}

pub struct OverlayStats {
    pub frame_number: u64,
    pub fps: f32,
    pub width: u32,
    pub height: u32,
}

pub fn render_pattern(
    framebuffer: &mut [u8],
    width: u32,
    height: u32,
    frame_index: u64,
    pattern: Pattern,
) {
    match pattern {
        Pattern::Plasma => render_test_pattern(framebuffer, width, height, frame_index),
        Pattern::Checker => render_checker_pattern(framebuffer, width, height, frame_index),
    }
}

pub fn render_test_pattern(framebuffer: &mut [u8], width: u32, height: u32, frame_index: u64) {
    let w = width as usize;
    let h = height as usize;
    if framebuffer.len() < w * h * 4 {
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

fn render_checker_pattern(framebuffer: &mut [u8], width: u32, height: u32, frame_index: u64) {
    let w = width as usize;
    let h = height as usize;
    if framebuffer.len() < w * h * 4 {
        return;
    }

    let t = (frame_index as usize / 12) % 32;
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) * 4;
            let checker = ((x / 32 + y / 32 + t) & 1) as u8;
            let value = if checker == 0 { 30 } else { 220 };
            framebuffer[i] = value;
            framebuffer[i + 1] = (value / 2).saturating_add(20);
            framebuffer[i + 2] = 255_u8.saturating_sub(value / 3);
            framebuffer[i + 3] = 0xFF;
        }
    }
}

pub fn draw_debug_overlay(framebuffer: &mut [u8], width: u32, height: u32, stats: &OverlayStats) {
    let text = format!(
        "FRAME:{} FPS:{:.1} SIZE:{}X{}",
        stats.frame_number, stats.fps, stats.width, stats.height
    );
    fill_rect(framebuffer, width, height, 6, 6, 420, 20, [0, 0, 0, 190]);
    draw_text(
        framebuffer,
        width,
        height,
        10,
        12,
        &text,
        [240, 255, 240, 255],
    );
}

fn fill_rect(
    framebuffer: &mut [u8],
    width: u32,
    height: u32,
    x: u32,
    y: u32,
    rect_w: u32,
    rect_h: u32,
    color: [u8; 4],
) {
    let w = width as usize;
    for py in y..y.saturating_add(rect_h).min(height) {
        for px in x..x.saturating_add(rect_w).min(width) {
            let i = ((py as usize) * w + px as usize) * 4;
            if i + 3 < framebuffer.len() {
                framebuffer[i] = color[0];
                framebuffer[i + 1] = color[1];
                framebuffer[i + 2] = color[2];
                framebuffer[i + 3] = color[3];
            }
        }
    }
}

fn draw_text(
    framebuffer: &mut [u8],
    width: u32,
    height: u32,
    x: u32,
    y: u32,
    text: &str,
    color: [u8; 4],
) {
    let mut pen_x = x;
    for ch in text.chars() {
        draw_char(framebuffer, width, height, pen_x, y, ch, color);
        pen_x = pen_x.saturating_add(4);
    }
}

fn draw_char(
    framebuffer: &mut [u8],
    width: u32,
    height: u32,
    x: u32,
    y: u32,
    ch: char,
    color: [u8; 4],
) {
    let glyph = glyph_3x5(ch);
    let w = width as usize;
    for (row, bits) in glyph.iter().enumerate() {
        for col in 0..3 {
            if bits & (1 << (2 - col)) == 0 {
                continue;
            }
            let px = x + col;
            let py = y + row as u32;
            if px >= width || py >= height {
                continue;
            }
            let i = (py as usize * w + px as usize) * 4;
            if i + 3 < framebuffer.len() {
                framebuffer[i] = color[0];
                framebuffer[i + 1] = color[1];
                framebuffer[i + 2] = color[2];
                framebuffer[i + 3] = color[3];
            }
        }
    }
}

fn glyph_3x5(ch: char) -> [u8; 5] {
    match ch {
        'A' => [0b010, 0b101, 0b111, 0b101, 0b101],
        'E' => [0b111, 0b110, 0b111, 0b110, 0b111],
        'F' => [0b111, 0b110, 0b111, 0b100, 0b100],
        'I' => [0b111, 0b010, 0b010, 0b010, 0b111],
        'M' => [0b101, 0b111, 0b111, 0b101, 0b101],
        'P' => [0b110, 0b101, 0b110, 0b100, 0b100],
        'R' => [0b110, 0b101, 0b110, 0b101, 0b101],
        'S' => [0b011, 0b100, 0b010, 0b001, 0b110],
        'X' => [0b101, 0b101, 0b010, 0b101, 0b101],
        'Z' => [0b111, 0b001, 0b010, 0b100, 0b111],
        '0' => [0b111, 0b101, 0b101, 0b101, 0b111],
        '1' => [0b010, 0b110, 0b010, 0b010, 0b111],
        '2' => [0b110, 0b001, 0b010, 0b100, 0b111],
        '3' => [0b110, 0b001, 0b010, 0b001, 0b110],
        '4' => [0b101, 0b101, 0b111, 0b001, 0b001],
        '5' => [0b111, 0b100, 0b110, 0b001, 0b110],
        '6' => [0b011, 0b100, 0b110, 0b101, 0b010],
        '7' => [0b111, 0b001, 0b010, 0b010, 0b010],
        '8' => [0b010, 0b101, 0b010, 0b101, 0b010],
        '9' => [0b010, 0b101, 0b011, 0b001, 0b110],
        ':' => [0b000, 0b010, 0b000, 0b010, 0b000],
        '.' => [0b000, 0b000, 0b000, 0b010, 0b000],
        ' ' => [0, 0, 0, 0, 0],
        _ => [0b111, 0b001, 0b010, 0b000, 0b010],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_expected_alpha_channel() {
        let mut buf = vec![0; 4 * 4 * 4];
        render_test_pattern(&mut buf, 4, 4, 1);
        assert!(buf.chunks_exact(4).all(|px| px[3] == 0xFF));
    }
}
