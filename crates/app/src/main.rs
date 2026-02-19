mod ffi;

use engine_loop::Scheduler;
use platform_abi::{
    PlatformConfig, PlatformEvent, PlatformFrame, PLATFORM_ABI_VERSION, PLATFORM_EVENT_KEY_DOWN,
    PLATFORM_EVENT_QUIT, PLATFORM_EVENT_RESIZE, PLATFORM_KEY_ESCAPE, PLATFORM_KEY_H,
    PLATFORM_KEY_SPACE,
};
use renderer::{draw_debug_overlay, render_pattern, OverlayStats, Pattern};
use std::{ffi::CString, mem::MaybeUninit, thread, time::Duration};
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

const TICK_HZ: f32 = 60.0;

fn main() {
    init_tracing();
    if let Err(err) = run() {
        eprintln!("tessera failed: {err}");
        std::process::exit(1);
    }
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();
}

struct SoftwareRenderer {
    width: u32,
    height: u32,
    framebuffer: Vec<u8>,
}

impl SoftwareRenderer {
    fn new(width: u32, height: u32) -> Self {
        let framebuffer = vec![0; (width as usize) * (height as usize) * 4];
        Self {
            width,
            height,
            framebuffer,
        }
    }

    fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        self.framebuffer
            .resize((width as usize) * (height as usize) * 4, 0);
        info!("resized renderer framebuffer to {}x{}", width, height);
    }
}

fn run() -> Result<(), String> {
    let title = CString::new("Tessera")
        .map_err(|_| "window title contains interior null byte".to_string())?;

    let width = 960_u32;
    let height = 540_u32;

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

    let mut renderer = SoftwareRenderer::new(width, height);
    let mut scheduler = Scheduler::with_hz(TICK_HZ);
    let mut frame_index = 0_u64;
    let mut running = true;
    let mut pattern = Pattern::Plasma;
    let mut show_help_overlay = true;
    info!("help overlay state: {}", show_help_overlay);

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
                PLATFORM_EVENT_KEY_DOWN if event.key_code == PLATFORM_KEY_SPACE => {
                    pattern = pattern.toggle();
                    info!("toggled render pattern: {:?}", pattern);
                }
                PLATFORM_EVENT_KEY_DOWN if event.key_code == PLATFORM_KEY_H => {
                    show_help_overlay = !show_help_overlay;
                    info!("help overlay state: {}", show_help_overlay);
                }
                PLATFORM_EVENT_KEY_DOWN => warn!("unhandled key code {}", event.key_code),
                PLATFORM_EVENT_RESIZE => {
                    if event.width > 0 && event.height > 0 {
                        renderer.resize(event.width, event.height);
                    }
                }
                _ => {}
            }
        }

        if !running {
            break;
        }

        let tick = scheduler.begin_frame();
        for _ in 0..tick.tick_count.max(1) {
            frame_index = frame_index.wrapping_add(1);
        }

        render_pattern(
            &mut renderer.framebuffer,
            renderer.width,
            renderer.height,
            frame_index,
            pattern,
        );

        let stats = OverlayStats {
            frame_number: tick.metrics.frame_number,
            fps: tick.metrics.fps,
            width: renderer.width,
            height: renderer.height,
        };
        draw_debug_overlay(
            &mut renderer.framebuffer,
            renderer.width,
            renderer.height,
            &stats,
        );

        let frame = PlatformFrame {
            width: renderer.width,
            height: renderer.height,
            stride_bytes: renderer.width * 4,
            pixels_rgba8: renderer.framebuffer.as_ptr(),
        };

        let presented = unsafe { ffi::platform_present_frame(&frame as *const PlatformFrame) };
        if !presented {
            running = false;
        }

        // Keep rendering responsive; platform may still throttle via vsync/compositor.
        thread::sleep(Duration::from_millis(1));
    }

    unsafe { ffi::platform_shutdown() };
    Ok(())
}
