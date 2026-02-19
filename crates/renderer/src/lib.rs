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
}

pub struct Renderer {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
    pattern: Pattern,
}

impl Renderer {
    pub fn new(width: u32, height: u32) -> Self {
        let mut renderer = Self {
            width,
            height,
            pixels: Vec::new(),
            pattern: Pattern::Gradient,
        };
        renderer.resize(width, height);
        renderer
    }

    pub fn resize(&mut self, width: u32, height: u32) {
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

    pub fn render(&mut self, frame_index: u64, time_seconds: f32) -> &[u8] {
        match self.pattern {
            Pattern::Gradient => {
                render_gradient(&mut self.pixels, self.width, self.height, frame_index)
            }
            Pattern::Solid => {
                let pulse = ((time_seconds * 2.0).sin() * 0.5 + 0.5) * 255.0;
                clear_rgba(&mut self.pixels, pulse as u8, 32, 120, 255);
            }
            Pattern::Rects => render_rects(&mut self.pixels, self.width, self.height, frame_index),
        }
        &self.pixels
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }
}

fn pixel_len(width: u32, height: u32) -> usize {
    (width as usize)
        .saturating_mul(height as usize)
        .saturating_mul(4)
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

    fn fnv1a64(bytes: &[u8]) -> u64 {
        let mut hash = 0xcbf29ce484222325_u64;
        for b in bytes {
            hash ^= u64::from(*b);
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash
    }
}
