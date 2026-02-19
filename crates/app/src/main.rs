mod ffi;

use platform_abi::{
    PlatformConfig, PlatformEvent, PlatformFrame, PLATFORM_ABI_VERSION, PLATFORM_EVENT_KEY_DOWN,
    PLATFORM_EVENT_QUIT, PLATFORM_EVENT_RESIZE, PLATFORM_KEY_ESCAPE,
};
use renderer::render_test_pattern;
use std::{
    env,
    ffi::CString,
    fs,
    mem::MaybeUninit,
    path::{Path, PathBuf},
    thread,
    time::Duration,
};

#[derive(Clone)]
struct CliArgs {
    pattern: String,
    width: u32,
    height: u32,
    frame_index: u64,
    headless_output: Option<PathBuf>,
}

fn main() {
    if let Err(err) = run() {
        eprintln!("tessera failed: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args = parse_args()?;
    if args.pattern != "test-pattern" {
        return Err(format!(
            "unsupported --pattern '{}'; only 'test-pattern' is available",
            args.pattern
        ));
    }

    if let Some(output_path) = args.headless_output.as_ref() {
        return run_headless(&args, output_path);
    }

    run_windowed(&args)
}

fn parse_args() -> Result<CliArgs, String> {
    let mut args = CliArgs {
        pattern: "test-pattern".to_string(),
        width: 960,
        height: 540,
        frame_index: 0,
        headless_output: None,
    };

    let mut iter = env::args().skip(1);
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--pattern" => {
                let value = iter
                    .next()
                    .ok_or_else(|| "--pattern requires a value".to_string())?;
                args.pattern = value;
            }
            "--width" => {
                let value = iter
                    .next()
                    .ok_or_else(|| "--width requires a value".to_string())?;
                args.width = value
                    .parse::<u32>()
                    .map_err(|_| format!("invalid --width value: {value}"))?;
            }
            "--height" => {
                let value = iter
                    .next()
                    .ok_or_else(|| "--height requires a value".to_string())?;
                args.height = value
                    .parse::<u32>()
                    .map_err(|_| format!("invalid --height value: {value}"))?;
            }
            "--frame-index" => {
                let value = iter
                    .next()
                    .ok_or_else(|| "--frame-index requires a value".to_string())?;
                args.frame_index = value
                    .parse::<u64>()
                    .map_err(|_| format!("invalid --frame-index value: {value}"))?;
            }
            "--headless-output" => {
                let value = iter
                    .next()
                    .ok_or_else(|| "--headless-output requires a value".to_string())?;
                args.headless_output = Some(PathBuf::from(value));
            }
            _ => return Err(format!("unknown argument: {arg}")),
        }
    }

    if args.width == 0 || args.height == 0 {
        return Err("--width and --height must be greater than zero".to_string());
    }

    Ok(args)
}

fn run_headless(args: &CliArgs, output_path: &Path) -> Result<(), String> {
    let mut framebuffer = vec![0_u8; (args.width as usize) * (args.height as usize) * 4];
    render_test_pattern(&mut framebuffer, args.width, args.height, args.frame_index);

    if let Some(parent) = output_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("failed to create output directory: {e}"))?;
        }
    }

    fs::write(output_path, &framebuffer).map_err(|e| {
        format!(
            "failed writing RGBA output '{}': {e}",
            output_path.display()
        )
    })?;

    let metadata_path = output_path.with_extension("json");
    let metadata = format!(
        "{{\n  \"format\": \"rgba8\",\n  \"width\": {},\n  \"height\": {},\n  \"stride_bytes\": {},\n  \"frame_index\": {},\n  \"pattern\": \"{}\"\n}}\n",
        args.width,
        args.height,
        args.width * 4,
        args.frame_index,
        args.pattern
    );
    fs::write(&metadata_path, metadata)
        .map_err(|e| format!("failed writing metadata '{}': {e}", metadata_path.display()))?;

    println!(
        "headless frame exported: rgba={} metadata={}",
        output_path.display(),
        metadata_path.display()
    );
    Ok(())
}

fn run_windowed(args: &CliArgs) -> Result<(), String> {
    let title = CString::new("Tessera")
        .map_err(|_| "window title contains interior null byte".to_string())?;

    let mut width = args.width;
    let mut height = args.height;

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
    let mut frame_index = args.frame_index;
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
