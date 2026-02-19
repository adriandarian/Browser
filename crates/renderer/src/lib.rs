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
