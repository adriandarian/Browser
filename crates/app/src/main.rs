mod ffi;

use platform_abi::{
    PlatformConfig, PlatformEvent, PlatformFrame, PLATFORM_ABI_VERSION, PLATFORM_EVENT_KEY_DOWN,
    PLATFORM_EVENT_QUIT, PLATFORM_EVENT_RESIZE, PLATFORM_KEY_ESCAPE,
};
use renderer::render_test_pattern;
use std::{ffi::CString, mem::MaybeUninit, thread, time::Duration};

fn main() {
    if let Err(err) = run() {
        eprintln!("tessera failed: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
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

    let mut framebuffer = vec![0_u8; (width as usize) * (height as usize) * 4];
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
                        framebuffer.resize((width as usize) * (height as usize) * 4, 0);
                    }
                }
                _ => {}
            }
        }

        if !running {
            break;
        }

        render_test_pattern(&mut framebuffer, width, height, frame_index);
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
