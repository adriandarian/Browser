mod ffi;

use platform_abi::{
    PlatformConfig, PlatformEvent, PlatformFrame, PLATFORM_ABI_VERSION, PLATFORM_EVENT_KEY_DOWN,
    PLATFORM_EVENT_QUIT, PLATFORM_EVENT_RESIZE, PLATFORM_KEY_ESCAPE,
};
use renderer::{Pattern, Renderer};
use std::{ffi::CString, mem::MaybeUninit, thread, time::Duration};

fn main() {
    if let Err(err) = run() {
        eprintln!("tessera failed: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let pattern = parse_pattern_from_args(std::env::args().skip(1))?;
    let title = CString::new("Tessera")
        .map_err(|_| "window title contains interior null byte".to_string())?;

    let mut width = 960_u32;
    let mut height = 540_u32;

    let config = PlatformConfig {
        abi_version: PLATFORM_ABI_VERSION,
        width,
        height,
        title_utf8: title.as_ptr(),
    };

    let initialized = unsafe { ffi::platform_init_window(&config as *const PlatformConfig) };
    if !initialized {
        return Err("platform_init_window returned false".to_string());
    }

    let mut renderer = Renderer::new(width, height);
    renderer.set_pattern(pattern);

    let mut frame_index = 0_u64;
    let mut running = true;

    while running {
        loop {
            let mut event = MaybeUninit::<PlatformEvent>::zeroed();
            let has_event = unsafe { ffi::platform_poll_event(event.as_mut_ptr()) };
            if !has_event {
                break;
            }

            let event = unsafe { event.assume_init() };
            match event.kind {
                PLATFORM_EVENT_QUIT => running = false,
                PLATFORM_EVENT_KEY_DOWN if event.key_code == PLATFORM_KEY_ESCAPE => running = false,
                PLATFORM_EVENT_RESIZE => {
                    if event.width > 0 && event.height > 0 {
                        width = event.width;
                        height = event.height;
                        renderer.resize(width, height);
                    }
                }
                _ => {}
            }
        }

        if !running {
            break;
        }

        let time_seconds = frame_index as f32 / 60.0;
        let framebuffer = renderer.render(frame_index, time_seconds);
        frame_index = frame_index.wrapping_add(1);

        let frame = PlatformFrame {
            width,
            height,
            stride_bytes: width * 4,
            pixels_rgba8: framebuffer.as_ptr(),
        };

        let presented = unsafe { ffi::platform_present_frame(&frame as *const PlatformFrame) };
        if !presented {
            running = false;
        }

        thread::sleep(Duration::from_millis(16));
    }

    unsafe { ffi::platform_shutdown() };
    Ok(())
}

fn parse_pattern_from_args(args: impl Iterator<Item = String>) -> Result<Pattern, String> {
    let mut pattern = Pattern::Gradient;
    let mut args = args.peekable();

    while let Some(arg) = args.next() {
        if arg == "--pattern" {
            let Some(value) = args.next() else {
                return Err(
                    "missing value for --pattern (expected: gradient|solid|rects)".to_string(),
                );
            };

            pattern = Pattern::parse(&value).ok_or_else(|| {
                format!("unknown pattern '{value}' (expected: gradient|solid|rects)",)
            })?;
        }
    }

    Ok(pattern)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_pattern_flag() {
        let args = vec!["--pattern".to_string(), "rects".to_string()];
        let parsed = parse_pattern_from_args(args.into_iter()).unwrap();
        assert_eq!(parsed, Pattern::Rects);
    }
}
